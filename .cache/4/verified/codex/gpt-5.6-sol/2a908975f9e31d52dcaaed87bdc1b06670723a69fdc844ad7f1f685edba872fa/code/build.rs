use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(command: &mut Command) -> Output {
    let description = format!("{command:?}");
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("failed to run {description}: {error}"));
    if !output.status.success() {
        panic!(
            "{description} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output
}

fn collect_files(root: &Path, extension: &str, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
    {
        let path = entry.expect("invalid directory entry").path();
        if path.is_dir() {
            collect_files(&path, extension, files);
        } else if path.extension() == Some(OsStr::new(extension)) {
            files.push(path);
        }
    }
}

fn defined_symbols(object: &Path) -> Vec<String> {
    let output = run(Command::new("nm")
        .arg("-g")
        .arg("--defined-only")
        .arg("--format=posix")
        .arg(object));
    String::from_utf8(output.stdout)
        .expect("nm emitted non-UTF-8 output")
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

fn main() {
    println!("cargo:rerun-if-changed=c_src");
    println!("cargo:rerun-if-changed=src/abi_symbols.txt");

    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is unset"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is unset"));
    let c_source = manifest_dir.join("c_src");
    let cmake_build = out_dir.join("c_backend");
    let renamed_dir = out_dir.join("renamed_objects");
    fs::create_dir_all(&renamed_dir).expect("failed to create renamed object directory");

    run(Command::new("cmake")
        .arg("-S")
        .arg(&c_source)
        .arg("-B")
        .arg(&cmake_build)
        .arg("-DCMAKE_BUILD_TYPE=Release"));
    run(Command::new("cmake")
        .arg("--build")
        .arg(&cmake_build)
        .arg("--parallel"));

    let abi = fs::read_to_string(manifest_dir.join("src/abi_symbols.txt"))
        .expect("failed to read ABI symbol inventory");
    let mut functions = Vec::new();
    let mut data = Vec::new();
    for line in abi.lines().filter(|line| !line.is_empty()) {
        let (kind, name) = line
            .split_once(' ')
            .unwrap_or_else(|| panic!("invalid ABI inventory line: {line}"));
        match kind {
            "F" => functions.push(name.to_owned()),
            "D" => data.push(name.to_owned()),
            _ => panic!("invalid ABI symbol kind in: {line}"),
        }
    }

    let function_map = out_dir.join("function-symbol-map.txt");
    let mappings = functions
        .iter()
        .map(|name| format!("{name} rust_backend_{name}\n"))
        .collect::<String>();
    fs::write(&function_map, mappings).expect("failed to write function symbol map");

    let mut objects = Vec::new();
    collect_files(
        &cmake_build.join("CMakeFiles/sodium.dir"),
        "o",
        &mut objects,
    );
    objects.sort();
    assert!(!objects.is_empty(), "CMake produced no backend objects");

    let mut renamed_objects = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        let output = renamed_dir.join(format!("backend_{index:04}.o"));
        let definitions = defined_symbols(object);
        let mut objcopy = Command::new("objcopy");
        objcopy.arg(format!("--redefine-syms={}", function_map.display()));
        for name in &data {
            if definitions.contains(name) {
                objcopy.arg(format!("--redefine-sym={name}=rust_backend_{name}"));
            }
        }
        objcopy.arg(object).arg(&output);
        run(&mut objcopy);
        renamed_objects.push(output);
    }

    let archive = out_dir.join("libsodium_backend.a");
    let mut ar = Command::new("ar");
    ar.arg("crs").arg(&archive).args(&renamed_objects);
    run(&mut ar);

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=sodium_backend");
}
