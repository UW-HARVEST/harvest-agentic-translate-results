use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const IMPL_PREFIX: &str = "__sodium_rust_impl_";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SymbolKind {
    Function,
    Data,
}

#[derive(Clone, Debug)]
struct Symbol {
    kind: SymbolKind,
    size: usize,
    object_index: usize,
}

fn collect_files(root: &Path, extension: &str, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| entry.expect("failed to read directory entry").path())
        .collect();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_files(&path, extension, files);
        } else if path.extension() == Some(OsStr::new(extension)) {
            files.push(path);
        }
    }
}

fn run(command: &mut Command, description: &str) -> Output {
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

fn parse_hex(value: &str, context: &str) -> usize {
    usize::from_str_radix(value, 16)
        .unwrap_or_else(|error| panic!("invalid hexadecimal {context} {value:?}: {error}"))
}

fn symbols_in_object(object: &Path, object_index: usize) -> BTreeMap<String, Symbol> {
    let output = run(
        Command::new("nm")
            .arg("-g")
            .arg("--defined-only")
            .arg("-P")
            .arg(object),
        "nm",
    );
    let text = String::from_utf8(output.stdout).expect("nm output was not UTF-8");
    let mut symbols = BTreeMap::new();

    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 4 {
            continue;
        }
        let kind = match fields[1] {
            "T" | "W" => SymbolKind::Function,
            "D" => SymbolKind::Data,
            _ => continue,
        };
        let symbol = Symbol {
            kind,
            size: parse_hex(fields[3], "symbol size"),
            object_index,
        };
        if let Some(previous) = symbols.insert(fields[0].to_owned(), symbol.clone()) {
            panic!(
                "duplicate symbol {} in {}: {:?} and {:?}",
                fields[0],
                object.display(),
                previous,
                symbol
            );
        }
    }
    symbols
}

fn write_exports(path: &Path, symbols: &BTreeMap<String, Symbol>) {
    let functions: Vec<_> = symbols
        .iter()
        .filter(|(_, symbol)| symbol.kind == SymbolKind::Function)
        .collect();
    let data: Vec<_> = symbols
        .iter()
        .filter(|(_, symbol)| symbol.kind == SymbolKind::Data)
        .collect();

    let mut rust = String::from(
        "// Generated from the global symbols in every C translation unit.\n\
         // Naked tail jumps preserve the complete platform C ABI.\n\n",
    );

    for (name, _) in &functions {
        assert!(
            name.bytes().enumerate().all(|(index, byte)| byte == b'_'
                || byte.is_ascii_alphanumeric() && (index != 0 || !byte.is_ascii_digit())),
            "symbol cannot be represented as a Rust identifier: {name}"
        );
        rust.push_str(&format!(
            "#[unsafe(naked)]\n\
             #[unsafe(no_mangle)]\n\
             pub unsafe extern \"C\" fn {name}() {{\n\
             \tcore::arch::naked_asm!(\"jmp {IMPL_PREFIX}{name}\");\n\
             }}\n\n"
        ));
    }

    for (index, (name, symbol)) in data.iter().enumerate() {
        assert_eq!(
            symbol.size % std::mem::size_of::<usize>(),
            0,
            "exported data symbol {name} is not pointer-aligned"
        );
        let words = symbol.size / std::mem::size_of::<usize>();
        rust.push_str(&format!(
            "#[unsafe(no_mangle)]\n\
             pub static mut {name}: [usize; {words}] = [1; {words}];\n\
             unsafe extern \"C\" {{\n\
             \t#[link_name = \"{IMPL_PREFIX}{name}\"]\n\
             \tstatic IMPL_DATA_{index}: [usize; {words}];\n\
             }}\n\n"
        ));
    }

    rust.push_str(
        "unsafe extern \"C\" fn initialize_exported_data() {\n\
         \tunsafe {\n",
    );
    for (index, (name, symbol)) in data.iter().enumerate() {
        rust.push_str(&format!(
            "\t\tcore::ptr::copy_nonoverlapping(\n\
             \t\t\tcore::ptr::addr_of!(IMPL_DATA_{index}).cast::<u8>(),\n\
             \t\t\tcore::ptr::addr_of_mut!({name}).cast::<u8>(),\n\
             \t\t\t{},\n\
             \t\t);\n",
            symbol.size
        ));
    }
    rust.push_str(
        "\t}\n\
         }\n\n\
         #[used]\n\
         #[unsafe(link_section = \".init_array\")]\n\
         static INITIALIZE_EXPORTED_DATA: unsafe extern \"C\" fn() = initialize_exported_data;\n",
    );

    fs::write(path, rust)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source_root = manifest_dir
        .parent()
        .expect("translation directory has no parent")
        .join("c_src/libsodium");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let object_dir = out_dir.join("objects");

    println!("cargo:rerun-if-changed={}", source_root.display());
    println!("cargo:rerun-if-changed=build.rs");

    fs::create_dir_all(&object_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", object_dir.display()));

    let mut sources = Vec::new();
    collect_files(&source_root, "c", &mut sources);
    assert_eq!(sources.len(), 145, "unexpected C source count");

    let mut include_dirs = vec![
        source_root.join("include"),
        source_root.join("include/sodium"),
    ];
    let mut top_level_dirs: Vec<_> = fs::read_dir(&source_root)
        .expect("failed to read libsodium source root")
        .map(|entry| entry.expect("failed to read source entry").path())
        .filter(|path| path.is_dir())
        .collect();
    top_level_dirs.sort();
    include_dirs.extend(top_level_dirs);

    let mut objects = Vec::with_capacity(sources.len());
    let mut all_symbols = BTreeMap::new();
    for (index, source) in sources.iter().enumerate() {
        let object = object_dir.join(format!("{index:03}.o"));
        let mut compiler = Command::new("cc");
        compiler
            .arg("-c")
            .arg("-Dsodium_EXPORTS")
            .arg("-std=gnu99")
            .arg("-fPIC")
            .arg("-w");
        for include_dir in &include_dirs {
            compiler.arg("-I").arg(include_dir);
        }
        compiler.arg(source).arg("-o").arg(&object);
        run(
            &mut compiler,
            &format!("C compilation of {}", source.display()),
        );

        let object_symbols = symbols_in_object(&object, index);
        for (name, symbol) in object_symbols {
            if let Some(previous) = all_symbols.insert(name.clone(), symbol.clone()) {
                panic!("duplicate global symbol {name}: {previous:?} and {symbol:?}");
            }
        }
        objects.push(object);
    }
    assert_eq!(all_symbols.len(), 890, "unexpected exported symbol count");

    for (index, object) in objects.iter().enumerate() {
        let map_path = object_dir.join(format!("{index:03}.redefine"));
        let mut map = String::new();
        for (name, symbol) in &all_symbols {
            if symbol.kind == SymbolKind::Function || symbol.object_index == index {
                map.push_str(name);
                map.push(' ');
                map.push_str(IMPL_PREFIX);
                map.push_str(name);
                map.push('\n');
            }
        }
        fs::write(&map_path, map)
            .unwrap_or_else(|error| panic!("failed to write {}: {error}", map_path.display()));
        run(
            Command::new("objcopy")
                .arg(format!("--redefine-syms={}", map_path.display()))
                .arg(object),
            "objcopy symbol namespacing",
        );
    }

    let archive = out_dir.join("libsodium_impl.a");
    let mut archiver = Command::new("ar");
    archiver.arg("crs").arg(&archive);
    for object in &objects {
        archiver.arg(object);
    }
    run(&mut archiver, "static archive creation");

    write_exports(&out_dir.join("exports.rs"), &all_symbols);

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static:+whole-archive=sodium_impl");
}
