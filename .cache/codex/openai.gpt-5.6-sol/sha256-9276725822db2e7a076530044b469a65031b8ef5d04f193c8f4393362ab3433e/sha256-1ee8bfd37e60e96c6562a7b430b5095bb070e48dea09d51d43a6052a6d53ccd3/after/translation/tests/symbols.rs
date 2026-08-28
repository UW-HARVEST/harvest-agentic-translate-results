use libloading::Library;
use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn exported_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to execute nm");
    assert!(output.status.success(), "nm failed for {}", path.display());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .map(str::to_owned)
        .collect()
}

#[test]
fn c_and_rust_defined_dynamic_symbols_match_exactly() {
    let c_path = manifest_dir().join("../c_src/build/libharvest-work-IPImlt.so");
    let rust_path = manifest_dir().join("target/release/libmatrixsum_lib.so");
    assert_eq!(exported_symbols(&c_path), exported_symbols(&rust_path));

    // Also force every exported symbol through the dynamic loader.
    let c = unsafe { Library::new(&c_path) }.unwrap();
    let rust = unsafe { Library::new(&rust_path) }.unwrap();
    for symbol in [
        b"add_element\0".as_slice(),
        b"calculate_matrix_checksum\0",
        b"expand_array\0",
        b"free_array\0",
        b"init_array\0",
        b"matrix\0",
        b"matrixsum\0",
        b"process_flags\0",
    ] {
        unsafe {
            c.get::<*mut ()>(symbol).unwrap();
            rust.get::<*mut ()>(symbol).unwrap();
        }
    }
}
