// Phase D -- symbol parity, enforced from inside the test suite as well as by
// the `nm -D` diff in run_all.sh.

mod common;
use common::*;

/// Every non-`static` function in c_src/src/lib.c, i.e. every symbol the C `.so`
/// exports (verified against `nm -D --defined-only` in run_all.sh).
const C_EXPORTS: &[&[u8]] = &[
    b"classify_mode\0",
    b"apply_multiplier\0",
    b"convert_time_factor\0",
    b"convert_negative_overflow\0",
    b"get_modified_time\0",
    b"hash_time_value\0",
    b"modeselect\0",
];

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let mut missing = Vec::new();
    for sym in C_EXPORTS {
        let name = String::from_utf8_lossy(&sym[..sym.len() - 1]).to_string();
        assert!(l.c.has(sym), "C .so unexpectedly lacks {name}");
        if !l.rust.has(sym) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "Rust .so is missing exported symbols: {missing:?}"
    );
}

#[test]
fn phase_d_nm_diff_is_empty() {
    // Cross-check with nm if it is available; skip silently if not.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let c_dir = root.join("c_src/build");
    let Ok(entries) = std::fs::read_dir(&c_dir) else {
        eprintln!("skipping: {} not built", c_dir.display());
        return;
    };
    let Some(c_so) = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
    else {
        eprintln!("skipping: no .so in {}", c_dir.display());
        return;
    };

    let nm = |p: &std::path::Path| -> Option<Vec<String>> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(p)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        Some(v)
    };

    // The Rust .so sits next to (or one level up from) the test executable.
    let mut dir = std::env::current_exe().unwrap();
    dir.pop();
    let mut rust_so = dir.join("libmodeselect_lib.so");
    if !rust_so.exists() {
        dir.pop();
        rust_so = dir.join("libmodeselect_lib.so");
    }
    if !rust_so.exists() {
        eprintln!("skipping: Rust .so not found");
        return;
    }

    let (Some(cs), Some(rs)) = (nm(&c_so), nm(&rust_so)) else {
        eprintln!("skipping: nm unavailable");
        return;
    };
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but not the Rust .so: {missing:?}"
    );
}
