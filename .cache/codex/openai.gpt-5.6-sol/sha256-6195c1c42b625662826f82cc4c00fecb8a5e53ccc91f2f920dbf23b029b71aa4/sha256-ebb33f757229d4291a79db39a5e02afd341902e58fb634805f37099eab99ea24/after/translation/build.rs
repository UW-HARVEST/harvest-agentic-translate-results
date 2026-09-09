use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(command: &mut Command) {
    let display = format!("{command:?}");
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to run {display}: {error}"));
    assert!(status.success(), "command failed: {display}");
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let c_root = manifest_dir.join("../c_src");
    let symbols_path = manifest_dir.join("symbols.txt");
    let symbols_text = fs::read_to_string(&symbols_path).expect("read symbols.txt");
    let symbols: Vec<&str> = symbols_text
        .lines()
        .filter(|line| !line.is_empty())
        .collect();
    let abi_path = manifest_dir.join("src/abi.rs");
    let abi_text = fs::read_to_string(&abi_path).expect("read src/abi.rs");
    let exact_symbols: HashSet<&str> = abi_text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("fn js_"))
        .filter_map(|line| line.strip_prefix("fn "))
        .filter_map(|line| line.split_once('(').map(|(name, _)| name))
        .collect();

    assert_eq!(
        symbols.len(),
        237,
        "the ABI manifest must contain all 237 exports"
    );
    assert!(
        symbols.windows(2).all(|pair| pair[0] < pair[1]),
        "symbols.txt must remain sorted and duplicate-free"
    );
    assert_eq!(
        exact_symbols.len(),
        127,
        "src/abi.rs must type every non-variadic public-header function"
    );
    assert!(
        exact_symbols.iter().all(|symbol| symbols.contains(symbol)),
        "every typed ABI entry must exist in symbols.txt"
    );

    println!("cargo:rerun-if-changed={}", symbols_path.display());
    println!("cargo:rerun-if-changed={}", abi_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        c_root.join("include/mujs.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        c_root.join("src/jsi.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        c_root.join("src/regexp.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        c_root.join("src/utf.h").display()
    );

    let mut sources: Vec<PathBuf> = fs::read_dir(c_root.join("src"))
        .expect("read C source directory")
        .map(|entry| entry.expect("read source entry").path())
        .filter(|path| path.extension() == Some(OsStr::new("c")))
        .collect();
    sources.sort();
    assert_eq!(sources.len(), 25, "expected every MuJS C translation unit");

    let cc = env::var_os("CC").unwrap_or_else(|| "cc".into());
    let mut objects = Vec::with_capacity(sources.len());
    for source in &sources {
        println!("cargo:rerun-if-changed={}", source.display());
        let object = out_dir.join(
            source
                .file_name()
                .and_then(OsStr::to_str)
                .expect("UTF-8 source name")
                .replace(".c", ".o"),
        );
        let mut command = Command::new(&cc);
        command
            .arg("-std=c99")
            .arg("-O3")
            .arg("-DNDEBUG")
            .arg("-fPIC")
            .arg("-fvisibility=hidden")
            .arg("-w")
            .arg("-I")
            .arg(c_root.join("include"))
            .arg("-I")
            .arg(c_root.join("src"));
        for symbol in &symbols {
            command.arg(format!("-D{symbol}=__mujs_impl_{symbol}"));
        }
        command.arg("-c").arg(source).arg("-o").arg(&object);
        run(&mut command);
        objects.push(object);
    }

    let archive = out_dir.join("libmujs_impl.a");
    let ar = env::var_os("AR").unwrap_or_else(|| "ar".into());
    let mut command = Command::new(ar);
    command.arg("crs").arg(&archive).args(&objects);
    run(&mut command);

    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("target architecture");
    let jump = match arch.as_str() {
        "x86_64" | "x86" => "jmp",
        "aarch64" | "arm" => "b",
        _ => panic!("unsupported target architecture for ABI trampolines: {arch}"),
    };
    let mut exports = String::from(
        "// Generated from symbols.txt. Each naked function preserves the caller's C ABI.\n",
    );
    for symbol in &symbols {
        if exact_symbols.contains(symbol) {
            continue;
        }
        exports.push_str(&format!(
            "#[unsafe(naked)]\n\
             #[unsafe(no_mangle)]\n\
             pub unsafe extern \"C\" fn {symbol}() {{\n\
             \tcore::arch::naked_asm!(\"{jump} __mujs_impl_{symbol}\");\n\
             }}\n\n"
        ));
    }
    fs::write(out_dir.join("exports.rs"), exports).expect("write generated Rust exports");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=mujs_impl");
    println!("cargo:rustc-link-lib=m");
}

#[allow(dead_code)]
fn _assert_path(_: &Path) {}
