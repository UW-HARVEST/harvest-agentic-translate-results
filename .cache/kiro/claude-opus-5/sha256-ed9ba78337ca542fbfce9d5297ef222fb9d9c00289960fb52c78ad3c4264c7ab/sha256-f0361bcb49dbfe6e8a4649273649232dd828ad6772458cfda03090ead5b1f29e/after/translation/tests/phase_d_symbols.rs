//! Phase D — symbol parity, checked programmatically so it cannot drift.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn defined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn find_c_so() -> PathBuf {
    let dir = root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} unreadable: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().expect("C .so must be built")
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("WCSCAT_RUST_SO") {
        return PathBuf::from(p);
    }
    for p in [
        root().join("translation/target/release/libwcscat_lib.so"),
        root().join("translation/target/debug/libwcscat_lib.so"),
    ] {
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib must be built");
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&find_c_so());
    let r = defined_symbols(&find_rust_so());
    assert!(!c.is_empty(), "C .so exported no symbols — build problem");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_project_symbols() {
    // `Library::new` (RTLD_NOW is not used by default, so force resolution by
    // actually looking the symbol up and calling it) proves the .so links.
    let f = common::rust_wcscat();
    let mut buf = [0i32; 4];
    let src = [b'a' as i32, 0];
    let rc = unsafe { f(buf.as_mut_ptr(), 4, src.as_ptr()) };
    assert_eq!(rc, 0);
    assert_eq!(buf, [b'a' as i32, 0, 0, 0]);
}
