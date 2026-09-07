//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every symbol *defined* by the C shared object must also be defined (exact
//! same name) by the Rust shared object.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn work_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let build = work_root().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("c_src/build must exist (build the C library first)")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    v.sort();
    v.pop().expect("no lib*.so in c_src/build")
}

fn rust_so() -> PathBuf {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for p in [
        target.join("release/libhdr_bitrate_lib.so"),
        target.join("debug/libhdr_bitrate_lib.so"),
    ] {
        if p.is_file() {
            return p;
        }
    }
    panic!("Rust cdylib not built");
}

/// Names of symbols with a *definition* (not `U`ndefined) in the given `.so`.
fn defined_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(out.status.success(), "nm -D failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut names: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let mut cols: Vec<&str> = line.split_whitespace().collect();
            if cols.is_empty() {
                return None;
            }
            let name = cols.pop().unwrap().to_string();
            // The type letter is the column before the name.
            let kind = cols.last().copied().unwrap_or("");
            // Skip undefined (non-weak) imports; keep everything with a real
            // definition or a weak binding, since `nm -D` reports both.
            if kind == "U" {
                return None;
            }
            Some(name)
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());

    assert!(
        c.contains(&"hdr_bitrate".to_string()),
        "sanity: C .so must define hdr_bitrate; got {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols defined by the C .so but missing from the Rust .so: {missing:?}\n\
         C:    {c:?}\nRust: {r:?}"
    );
}

/// The exported symbol must actually be callable via `dlsym` (this is what
/// exercises the `#[no_mangle] extern \"C\"` wrapper).
#[test]
fn phase_d_exported_symbol_is_callable() {
    let l = common::libs();
    let buf = [0u8, 0b0000_1010, 0x50];
    let (cv, rv) = unsafe { ((l.c)(buf.as_ptr()), (l.rust)(buf.as_ptr())) };
    assert_eq!(cv, rv);
}
