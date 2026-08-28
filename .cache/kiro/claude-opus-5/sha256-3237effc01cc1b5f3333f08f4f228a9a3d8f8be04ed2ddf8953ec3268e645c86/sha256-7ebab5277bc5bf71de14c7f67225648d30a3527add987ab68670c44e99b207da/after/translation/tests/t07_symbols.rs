//! Export parity: every dynamic symbol the C `.so` defines must also be defined
//! by the Rust `.so` under exactly the same name.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Dynamic *defined* symbols, as reported by `nm -D --defined-only`.
fn dynamic_defined_symbols(so: &Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>"
        if cols.len() >= 3 {
            let ty = cols[cols.len() - 2];
            let name = cols[cols.len() - 1];
            // skip the toolchain-generated bookkeeping symbols
            if name.starts_with("_") || name.contains('.') {
                continue;
            }
            // T/t: text, D/d/B/b: data, W: weak
            if matches!(ty, "T" | "t" | "D" | "d" | "B" | "b" | "W" | "R" | "r") {
                set.insert(name.to_string());
            }
        }
    }
    Some(set)
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_so = c_so_path();
    let r_so = rust_so_path();

    let Some(c_syms) = dynamic_defined_symbols(&c_so) else {
        eprintln!("`nm` unavailable -- falling back to dlsym checks only");
        return;
    };
    let r_syms = dynamic_defined_symbols(&r_so).expect("nm on the Rust .so");

    assert!(
        !c_syms.is_empty(),
        "no symbols found in {} -- is it built?",
        c_so.display()
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C   ({}): {:?}\nRust({}): {:?}",
        missing.len(),
        missing,
        c_so.display(),
        c_syms,
        r_so.display(),
        r_syms
    );
}

/// Belt-and-braces: resolve each C symbol name through `dlsym` on the Rust
/// library, which is what an external caller actually does.
#[test]
fn rust_symbols_resolve_via_dlsym() {
    let c_so = c_so_path();
    let r_so = rust_so_path();
    let names: Vec<String> = match dynamic_defined_symbols(&c_so) {
        Some(s) => s.into_iter().collect(),
        None => [
            "hm_geti",
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
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
    };

    let lib = unsafe { libloading::Library::new(&r_so) }.expect("load rust .so");
    for n in &names {
        let mut bytes = n.clone().into_bytes();
        bytes.push(0);
        let found = unsafe { lib.get::<*const ()>(&bytes) }.is_ok();
        assert!(found, "dlsym({n}) failed on {}", r_so.display());
    }
}
