// Verifies that every public function symbol exported by the C
// `libsphincs_core.so` is also exported by the Rust `libsphincs_plus.so`.

mod common;

use std::process::Command;

fn t_funcs(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("nm failed");
    assert!(out.status.success(), "nm failed: {}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<String> = s
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let _addr = parts.next()?;
            let kind = parts.next()?;
            let name = parts.next()?;
            // Only T (text/code) symbols are functions. Skip linker-generated
            // _init/_fini.
            if kind != "T" || name == "_init" || name == "_fini" {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect();
    v.sort();
    v
}

#[test]
fn rust_so_exports_superset_of_c() {
    let c = t_funcs(&common::c_so_path());
    let r = t_funcs(&common::rust_so_path());

    let r_set: std::collections::BTreeSet<&str> = r.iter().map(|s| s.as_str()).collect();
    let mut missing = Vec::new();
    for sym in &c {
        if !r_set.contains(sym.as_str()) {
            missing.push(sym.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "Rust .so missing {} symbols exported by C: {:?}",
        missing.len(),
        missing
    );
}
