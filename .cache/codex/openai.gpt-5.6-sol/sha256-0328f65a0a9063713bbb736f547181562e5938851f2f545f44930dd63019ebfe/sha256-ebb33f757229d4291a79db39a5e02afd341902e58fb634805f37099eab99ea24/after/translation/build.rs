use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(command: &mut Command) -> Output {
    let description = format!("{command:?}");
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("failed to run {description}: {error}"));

    if !output.status.success() {
        panic!(
            "{description} failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    output
}

fn c_sources(source_dir: &Path) -> Vec<PathBuf> {
    let mut sources = fs::read_dir(source_dir)
        .expect("read c_src/src")
        .map(|entry| entry.expect("read source entry").path())
        .filter(|path| path.extension() == Some(OsStr::new("c")))
        .collect::<Vec<_>>();
    sources.sort();
    sources
}

fn extract_array(source: &str, declaration: &str, length: usize) -> Vec<u16> {
    let declaration_start = source
        .find(declaration)
        .unwrap_or_else(|| panic!("missing declaration {declaration}"));
    let initializer_start = source[declaration_start..]
        .find('{')
        .map(|offset| declaration_start + offset + 1)
        .expect("missing array initializer");
    let initializer_end = source[initializer_start..]
        .find("};")
        .map(|offset| initializer_start + offset)
        .expect("missing end of array initializer");
    let values = source[initializer_start..initializer_end]
        .split(|character: char| !character.is_ascii_digit())
        .filter(|token| !token.is_empty())
        .map(|token| token.parse::<u16>().expect("invalid array value"))
        .collect::<Vec<_>>();

    assert_eq!(
        values.len(),
        length,
        "{declaration} has the wrong element count"
    );
    values
}

fn write_array(
    generated: &mut fs::File,
    rust_type: &str,
    name: &str,
    values: &[u16],
) {
    writeln!(generated, "#[unsafe(no_mangle)]").unwrap();
    writeln!(
        generated,
        "pub static {name}: [{rust_type}; {}] = [",
        values.len()
    )
    .unwrap();

    for row in values.chunks(16) {
        write!(generated, "    ").unwrap();
        for value in row {
            write!(generated, "{value},").unwrap();
        }
        writeln!(generated).unwrap();
    }

    writeln!(generated, "];").unwrap();
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let c_root = manifest_dir.parent().unwrap().join("c_src");
    let include_dir = c_root.join("include");
    let source_dir = c_root.join("src");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let sources = c_sources(&source_dir);

    assert_eq!(sources.len(), 15, "unexpected C source count");
    println!("cargo:rerun-if-changed={}", include_dir.display());
    println!("cargo:rerun-if-changed={}", source_dir.display());

    let mut objects = Vec::with_capacity(sources.len());
    for source in &sources {
        let object = out_dir.join(
            source
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(".c", ".o"),
        );
        run(Command::new("cc").args([
            "-O3",
            "-DNDEBUG",
            "-fPIC",
            "-std=gnu99",
            "-I",
        ]).arg(&include_dir).arg("-c").arg(source).arg("-o").arg(&object));
        objects.push(object);
    }

    let combined = out_dir.join("libpng-combined.o");
    let mut combine = Command::new("cc");
    combine.arg("-r").arg("-o").arg(&combined).args(&objects);
    run(&mut combine);

    let nm = run(
        Command::new("nm")
            .args(["-g", "--defined-only", "--format=posix"])
            .arg(&combined),
    );
    let mut functions = Vec::new();
    let mut data = Vec::new();

    for line in String::from_utf8(nm.stdout).expect("nm emitted non-UTF-8").lines() {
        let mut fields = line.split_whitespace();
        let name = fields.next().unwrap_or_default();
        let kind = fields.next().unwrap_or_default();
        if !name.starts_with("png_") {
            continue;
        }

        match kind {
            "T" => functions.push(name.to_owned()),
            "R" => data.push(name.to_owned()),
            _ => panic!("unexpected exported symbol kind {kind} for {name}"),
        }
    }

    functions.sort();
    data.sort();
    assert_eq!(functions.len(), 381, "unexpected exported function count");
    assert_eq!(
        data,
        ["png_sRGB_base", "png_sRGB_delta", "png_sRGB_table"],
        "unexpected exported data symbols"
    );

    let rename_map = out_dir.join("rename-symbols.txt");
    let mut map = fs::File::create(&rename_map).expect("create symbol rename map");
    for name in functions.iter().chain(data.iter()) {
        writeln!(map, "{name} __libpng_c_{name}").unwrap();
    }
    drop(map);

    let renamed = out_dir.join("libpng-renamed.o");
    fs::copy(&combined, &renamed).expect("copy combined object");
    run(
        Command::new("objcopy")
            .arg(format!("--redefine-syms={}", rename_map.display()))
            .arg(&renamed),
    );

    let generated_path = out_dir.join("exports.rs");
    let mut generated = fs::File::create(&generated_path).expect("create exports.rs");
    writeln!(
        generated,
        "macro_rules! forward {{\n\
             ($name:ident) => {{\n\
                 #[unsafe(no_mangle)]\n\
                 #[unsafe(naked)]\n\
                 pub unsafe extern \"C\" fn $name() {{\n\
                     core::arch::naked_asm!(concat!(\"jmp __libpng_c_\", stringify!($name)));\n\
                 }}\n\
             }};\n\
         }}"
    )
    .unwrap();
    for name in &functions {
        writeln!(generated, "forward!({name});").unwrap();
    }

    let png_source = fs::read_to_string(source_dir.join("png.c")).expect("read png.c");
    let table = extract_array(
        &png_source,
        "const png_uint_16 png_sRGB_table[256]",
        256,
    );
    let base = extract_array(
        &png_source,
        "const png_uint_16 png_sRGB_base[512]",
        512,
    );
    let delta = extract_array(
        &png_source,
        "const png_byte png_sRGB_delta[512]",
        512,
    );
    write_array(&mut generated, "u16", "png_sRGB_table", &table);
    write_array(&mut generated, "u16", "png_sRGB_base", &base);
    write_array(&mut generated, "u8", "png_sRGB_delta", &delta);

    println!("cargo:rustc-link-arg={}", renamed.display());
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=m");
}

