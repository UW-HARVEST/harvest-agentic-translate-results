use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SOURCE_DIRS: &[&str] = &[
    "common",
    "compress",
    "decompress",
    "dictBuilder",
    "deprecated",
    "legacy",
];

const DATA_EXPORTS: &[&str] = &["g_ZSTD_threading_useless_symbol", "g_debuglevel"];

fn run(command: &mut Command) {
    eprintln!("running: {command:?}");
    let status = command.status().expect("failed to start build command");
    assert!(status.success(), "build command failed with {status}");
}

fn collect_sources(source_root: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    for directory in SOURCE_DIRS {
        let path = source_root.join(directory);
        for entry in fs::read_dir(&path).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", path.display());
        }) {
            let path = entry.expect("failed to read source entry").path();
            if path.extension() == Some(OsStr::new("c")) {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}

fn defined_exports(object: &Path) -> Vec<(char, String)> {
    let mut command = Command::new("nm");
    command
        .arg("-g")
        .arg("--defined-only")
        .arg(object)
        .stdout(Stdio::piped());
    let mut child = command.spawn().expect("failed to run nm");
    let stdout = child.stdout.take().expect("nm stdout was not piped");
    let mut symbols = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.expect("failed to read nm output");
        let mut fields = line.split_whitespace();
        let _address = fields.next();
        let kind = fields
            .next()
            .and_then(|field| field.chars().next())
            .expect("missing nm symbol type");
        let name = fields.next().expect("missing nm symbol name").to_owned();
        if matches!(kind, 'T' | 'D' | 'B' | 'R') {
            symbols.push((kind, name));
        }
    }
    let status = child.wait().expect("failed to wait for nm");
    assert!(status.success(), "nm failed with {status}");
    symbols.sort_by(|left, right| left.1.cmp(&right.1));
    symbols
}

fn write_generated_rust(path: &Path, exports: &[(char, String)]) {
    let mut output = File::create(path).expect("failed to create generated Rust exports");
    writeln!(
        output,
        "// Generated from the global definitions in the complete C source build."
    )
    .unwrap();
    for (kind, name) in exports {
        if *kind == 'T' {
            writeln!(output, "trampoline!({name}, \"c_{name}\");").unwrap();
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let source_root = manifest_dir.join("../c_src/src");
    let sources = collect_sources(&source_root);
    assert_eq!(sources.len(), 40, "unexpected C source count");

    for source in &sources {
        println!("cargo:rerun-if-changed={}", source.display());
    }
    for header in ["zstd.h", "zdict.h", "zstd_errors.h"] {
        println!(
            "cargo:rerun-if-changed={}",
            source_root.join("include").join(header).display()
        );
    }

    let include_dirs = [
        source_root.clone(),
        source_root.join("include"),
        source_root.join("common"),
        source_root.join("compress"),
        source_root.join("decompress"),
        source_root.join("dictBuilder"),
        source_root.join("deprecated"),
        source_root.join("legacy"),
    ];

    let mut objects = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let object = out_dir.join(format!("zstd-{index}.o"));
        let mut command = Command::new("cc");
        command.arg("-c").arg(source).arg("-o").arg(&object).args([
            "-O3",
            "-fPIC",
            "-std=c99",
            "-w",
            "-DZSTD_LEGACY_SUPPORT=5",
            "-DXXH_NAMESPACE=ZSTD_",
            "-DDYNAMIC_BMI2=0",
        ]);
        for include in &include_dirs {
            command.arg("-I").arg(include);
        }
        run(&mut command);
        objects.push(object);
    }

    let combined = out_dir.join("zstd-combined.o");
    let mut combine = Command::new("cc");
    combine.arg("-r").arg("-o").arg(&combined).args(&objects);
    run(&mut combine);

    let exports = defined_exports(&combined);
    let function_count = exports.iter().filter(|(kind, _)| *kind == 'T').count();
    let data_exports: Vec<_> = exports
        .iter()
        .filter(|(kind, _)| matches!(kind, 'D' | 'B' | 'R'))
        .map(|(_, name)| name.as_str())
        .collect();
    assert_eq!(function_count, 613, "unexpected exported function count");
    assert_eq!(
        data_exports, DATA_EXPORTS,
        "unexpected exported data symbols"
    );

    let rename_map = out_dir.join("rename-symbols.txt");
    let mut map = File::create(&rename_map).expect("failed to create objcopy rename map");
    for (_, name) in &exports {
        writeln!(map, "{name} c_{name}").unwrap();
    }
    drop(map);

    run(Command::new("objcopy")
        .arg("--redefine-syms")
        .arg(&rename_map)
        .arg(&combined));

    let generated = out_dir.join("exports.rs");
    write_generated_rust(&generated, &exports);

    println!("cargo:rustc-link-arg={}", combined.display());
    println!("cargo:rustc-link-arg=-Wl,-z,noexecstack");
    println!("cargo:rustc-link-lib=c");
}
