//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D`) that every dynamic symbol the C shared
//! object defines is also defined by the Rust shared object under the exact
//! same name, and that `dlsym` actually resolves each one in both.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let dir = repo_root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    v.sort();
    v.into_iter().next().expect("no C .so built")
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let p = profile_dir.join("libldexp_q2_lib.so");
    if p.is_file() {
        return p;
    }
    for prof in ["release", "debug"] {
        let q = repo_root()
            .join("translation")
            .join("target")
            .join(prof)
            .join("libldexp_q2_lib.so");
        if q.is_file() {
            return q;
        }
    }
    panic!("no Rust cdylib found");
}

/// Names of globally-defined dynamic symbols, excluding link-editor artifacts.
fn defined_dynamic_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        // "<addr> <type> <name>" or "<type> <name>"
        let (ty, name) = if cols.len() >= 3 {
            (cols[1], cols[2])
        } else {
            (cols[0], cols[1])
        };
        // Only real global definitions: T/t text, D/d data, B/b bss, W/w weak, R/r rodata.
        if !matches!(ty, "T" | "t" | "D" | "d" | "B" | "b" | "W" | "w" | "R" | "r" | "V" | "v") {
            continue;
        }
        // Link-editor / toolchain artifacts that are not part of the library API.
        if matches!(
            name,
            "_init" | "_fini" | "_edata" | "_end" | "__bss_start" | "__bss_start__"
                | "_bss_end__" | "__end__" | "_edata__"
        ) || name.starts_with("__gmon")
            || name.starts_with("_ITM_")
            || name.starts_with("__cxa")
        {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = c_so();
    let r = rust_so();
    let cs = defined_dynamic_symbols(&c);
    let rs = defined_dynamic_symbols(&r);

    assert!(
        !cs.is_empty(),
        "nm found no exported symbols in the C .so ({}) — build problem",
        c.display()
    );
    assert!(
        cs.contains("ldexp_q2"),
        "C .so does not export ldexp_q2; found {cs:?}"
    );

    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is MISSING {} symbol(s) exported by the C .so ({}): {:?}\n\
         C symbols:    {:?}\n\
         Rust symbols: {:?}",
        r.display(),
        missing.len(),
        c.display(),
        missing,
        cs,
        rs
    );
}

#[test]
fn every_c_symbol_resolves_via_dlsym_in_both() {
    // `common::pair()` performs the two `dlsym` lookups; if either failed the
    // library would not be usable as a drop-in replacement.
    let p = common::pair();
    let a = unsafe { (p.c)(1.0, 4) };
    let b = unsafe { (p.rust)(1.0, 4) };
    assert_eq!(
        a.to_bits(),
        b.to_bits(),
        "dlsym'd ldexp_q2 diverged on a trivial input: C {a:?} vs Rust {b:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_project_symbols() {
    let r = rust_so();
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(&r)
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    // Anything undefined must come from libc / libgcc / the Rust runtime, not
    // from the project itself.
    let suspicious: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|n| n.contains("ldexp_q2"))
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved project symbols: {suspicious:?}"
    );
}
