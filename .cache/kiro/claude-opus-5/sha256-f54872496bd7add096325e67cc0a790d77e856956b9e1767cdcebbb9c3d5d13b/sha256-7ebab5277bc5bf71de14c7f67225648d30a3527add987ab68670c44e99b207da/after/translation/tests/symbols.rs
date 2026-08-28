//! Step 8: every symbol exported by the C `.so` must also be exported, under
//! the exact same name, by the Rust `.so`.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().expect("no C .so built")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let mut cands = vec![profile.join("libhelxo_lib.so")];
    if let Some(t) = profile.parent() {
        cands.push(t.join("debug").join("libhelxo_lib.so"));
        cands.push(t.join("release").join("libhelxo_lib.so"));
    }
    cands
        .into_iter()
        .find(|p| p.exists())
        .expect("Rust cdylib not built - run `cargo build` first")
}

/// Dynamic symbols defined (exported) by `path`.
fn exported_symbols(path: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());
    assert!(!c.is_empty(), "no symbols found in the C .so");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\nC exports: {c:?}"
    );

    // Guard against silently losing an export in the future.
    for expected in [
        "helxo",
        "strkey",
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_rand_seed",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_hmdel_key",
        "stbds_shmode_func",
        "stbds_stralloc",
        "stbds_strreset",
    ] {
        assert!(
            c.iter().any(|s| s == expected),
            "expected {expected} in the C .so exports"
        );
        assert!(
            r.iter().any(|s| s == expected),
            "expected {expected} in the Rust .so exports"
        );
    }
}
