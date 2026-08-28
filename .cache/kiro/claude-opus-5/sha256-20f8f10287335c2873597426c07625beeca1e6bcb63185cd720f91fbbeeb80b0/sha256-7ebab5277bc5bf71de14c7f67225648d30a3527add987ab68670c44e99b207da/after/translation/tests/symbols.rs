//! Every dynamic symbol the C shared object defines must also be defined by the
//! Rust shared object, under exactly the same name.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn dyn_defined(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    let dir = root.join("c_src/build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("c_src/build missing")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    cands.sort();
    cands.pop().expect("no C .so")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    let profile = deps.parent().unwrap();
    for d in [profile, deps] {
        let p = d.join("libarr_ins_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libarr_ins_lib.so not found");
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = dyn_defined(&c_so());
    let r = dyn_defined(&rust_so());
    assert!(!c.is_empty(), "C .so exported nothing -- bad build?");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // The full documented API surface of lib.c, spelled out so a silently
    // dropped `#[no_mangle]` cannot slip past even if the C build changes.
    for expected in [
        "arr_ins",
        "strkey",
        "stbds_rand_seed",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_stralloc",
        "stbds_strreset",
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_hmdel_key",
        "stbds_shmode_func",
    ] {
        assert!(
            c.iter().any(|s| s == expected),
            "C .so unexpectedly lacks {expected}"
        );
        assert!(
            r.iter().any(|s| s == expected),
            "Rust .so lacks {expected}"
        );
    }
}
