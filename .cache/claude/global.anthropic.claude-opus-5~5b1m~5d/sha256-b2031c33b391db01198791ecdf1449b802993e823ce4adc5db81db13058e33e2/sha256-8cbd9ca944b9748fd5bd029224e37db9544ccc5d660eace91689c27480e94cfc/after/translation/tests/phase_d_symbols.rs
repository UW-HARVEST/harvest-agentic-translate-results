//! Phase D — symbol parity enforced as a test.
//!
//! Every symbol the C `.so` exports must be exported by the Rust `.so` under
//! the exact same name, and must be callable through `dlsym`.

mod common;
use common::pair;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_so(dir: &Path, exact: Option<&str>) -> PathBuf {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            match exact {
                Some(x) => n == x,
                None => n.starts_with("lib") && n.ends_with(".so"),
            }
        })
        .collect();
    assert!(!found.is_empty(), "no .so in {}", dir.display());
    found.sort();
    found.remove(0)
}

/// `nm -D --defined-only` -> sorted unique symbol names.
fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("`nm` must be available to run the symbol-parity test");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c_so = find_so(&root().join("c_src").join("build"), None);
    let rs_so = common::find_rust_so();

    let c_syms = defined_dynamic_symbols(&c_so);
    let rs_syms = defined_dynamic_symbols(&rs_so);

    assert!(
        c_syms.contains(&"memchra2".to_string()),
        "C .so must export memchra2, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rs_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} C symbol(s): {missing:?}\nC exports:   {c_syms:?}",
        missing.len()
    );
}

#[test]
fn both_symbols_resolve_and_are_callable() {
    let p = pair();
    // A trivial call through each dlsym'd pointer proves the export wrappers
    // are real code with the right ABI, not just names in the symbol table.
    let a = unsafe { (p.c)(1, 2, 3, 4) };
    let b = unsafe { (p.rs)(1, 2, 3, 4) };
    assert_eq!(a, b, "memchra2(1,2,3,4): C={a} RS={b}");
}
