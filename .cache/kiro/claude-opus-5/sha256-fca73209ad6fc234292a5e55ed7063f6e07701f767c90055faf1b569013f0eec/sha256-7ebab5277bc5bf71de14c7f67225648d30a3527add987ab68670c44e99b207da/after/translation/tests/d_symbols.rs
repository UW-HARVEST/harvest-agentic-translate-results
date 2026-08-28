//! Exported-symbol parity: every dynamic symbol the C `.so` defines must also
//! be defined by the Rust `.so` under the exact same name.
mod harness;

use std::path::{Path, PathBuf};
use std::process::Command;

fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // exported code/data, ignoring linker-internal entries
            matches!(kind, "T" | "t" | "D" | "B" | "R" | "W" | "G" | "S")
                .then(|| name.to_string())
        })
        .filter(|n| !n.starts_with('_') && !n.contains('@'))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let dir = root().join("c_src").join("build");
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().map(|e| e == "so").unwrap_or(false))
        .collect();
    hits.sort();
    hits.pop().expect("no C .so built")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = defined_dynamic_symbols(&c_so());
    assert!(!c_syms.is_empty(), "nm found no C symbols");

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for profile in ["debug", "release"] {
        let p = manifest.join("target").join(profile).join("libmaxnmin_lib.so");
        if !p.is_file() {
            continue;
        }
        checked += 1;
        let r_syms = defined_dynamic_symbols(&p);
        let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "{} is missing C-exported symbols: {missing:?}\nrust exports: {r_syms:?}",
            p.display()
        );
        // Sanity: the whole documented API is present.
        for want in [
            "maxnmin",
            "add_node",
            "find_node_by_id",
            "get_children_count",
            "calculate_subtree_sum",
            "process_string",
            "safe_double_to_int",
        ] {
            assert!(
                c_syms.iter().any(|s| s == want),
                "C .so unexpectedly lacks {want}"
            );
            assert!(
                r_syms.iter().any(|s| s == want),
                "{} lacks {want}",
                p.display()
            );
        }
    }
    assert!(checked > 0, "no Rust .so found to compare");
}

/// Both libraries must be loadable and all symbols resolvable via dlsym.
#[test]
fn all_symbols_resolve_through_dlopen() {
    let i = harness::impls();
    assert!(!i.rust.is_empty());
    // Api::load already dlsym'd every function; touching them proves it.
    unsafe {
        assert_eq!((i.c.safe_double_to_int)(1.0), 1);
        for r in &i.rust {
            assert_eq!((r.safe_double_to_int)(1.0), 1, "{}", r.label);
        }
    }
}
