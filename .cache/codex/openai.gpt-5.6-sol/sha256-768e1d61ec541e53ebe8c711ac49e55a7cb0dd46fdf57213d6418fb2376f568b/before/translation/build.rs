use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(mut command: Command) {
    let status = command.status().expect("failed to start native build command");
    assert!(status.success(), "native build command failed: {command:?}");
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is unset"));
    let source_root = manifest_dir
        .parent()
        .expect("translation crate has no parent")
        .join("c_src");
    let source_dir = source_root.join("src");
    let include_dir = source_root.join("include");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is unset"));
    let public_symbols_path = manifest_dir.join("public_symbols.txt");
    let public_symbols = fs::read_to_string(&public_symbols_path)
        .expect("cannot read public symbol list");

    let mut sources: Vec<PathBuf> = fs::read_dir(&source_dir)
        .expect("cannot read C source directory")
        .map(|entry| entry.expect("cannot read C source entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "c"))
        .collect();
    sources.sort();

    for source in sources {
        println!("cargo:rerun-if-changed={}", source.display());
        let stem = source
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("source has no UTF-8 stem");
        let object = out_dir.join(format!("{stem}.o"));

        let mut compiler = Command::new("cc");
        compiler
            .arg("-std=gnu99")
            .arg("-fPIC")
            .arg("-w")
            .arg("-DHAVE_CONFIG_H")
            .arg("-DPCRE2_CODE_UNIT_WIDTH=8")
            .arg("-DSUPPORT_UNICODE")
            .arg("-I")
            .arg(&include_dir)
            .arg("-I")
            .arg(&source_dir)
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(&object);
        for symbol in public_symbols.lines().filter(|line| !line.is_empty()) {
            compiler.arg(format!("-D{symbol}=rust_internal_{symbol}"));
        }
        run(compiler);

        println!("cargo:rustc-link-arg={}", object.display());
    }

    rerun_headers(&source_root);
    println!("cargo:rerun-if-changed={}", public_symbols_path.display());
}

fn rerun_headers(root: &Path) {
    for directory in [root.join("include"), root.join("src")] {
        for entry in fs::read_dir(directory).expect("cannot read header directory") {
            let path = entry.expect("cannot read header entry").path();
            if path.extension().is_some_and(|extension| extension == "h") {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}
