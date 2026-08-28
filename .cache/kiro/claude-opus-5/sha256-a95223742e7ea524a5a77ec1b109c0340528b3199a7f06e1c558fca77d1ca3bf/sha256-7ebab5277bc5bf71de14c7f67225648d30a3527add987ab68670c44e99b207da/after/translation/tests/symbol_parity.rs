//! Verifies that the Rust cdylib exports every dynamic symbol that the C
//! shared library exports, with byte-identical names.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find(candidates: &[PathBuf]) -> PathBuf {
    for c in candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("shared library not found in {candidates:?}");
}

/// Defined (exported) dynamic symbol names of `path`, via `nm -D`.
fn exported_symbols(path: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm -D failed for {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "         U <name>"
            let mut parts = line.split_whitespace();
            let a = parts.next()?;
            let b = parts.next()?;
            let (ty, name) = match parts.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Keep global code/data symbols; drop link-editor bookkeeping.
            if ty.len() == 1 && "TWDBRVGtd".contains(ty) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .filter(|s| {
            !matches!(
                s.as_str(),
                "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_edata"
                    | "_end"
                    | "__gmon_start__"
                    | "_ITM_registerTMCloneTable"
                    | "_ITM_deregisterTMCloneTable"
                    | "__cxa_finalize"
            )
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_exports_superset_of_c_exports() {
    common::ensure_cdylib_fresh();
    let root = workspace_root();

    let c_so = find(&[
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ]);

    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let rust_so = find(&[
        profile_dir.join("libdriver.so"),
        root.join("translation/target/debug/libdriver.so"),
        root.join("translation/target/release/libdriver.so"),
    ]);

    let c_syms = exported_symbols(&c_so);
    let rust_syms = exported_symbols(&rust_so);

    assert!(
        c_syms.contains(&"decode_base64".to_string()),
        "sanity: C .so must export decode_base64, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C   : {c_syms:?}\n  \
         rust: {rust_syms:?}"
    );
}
