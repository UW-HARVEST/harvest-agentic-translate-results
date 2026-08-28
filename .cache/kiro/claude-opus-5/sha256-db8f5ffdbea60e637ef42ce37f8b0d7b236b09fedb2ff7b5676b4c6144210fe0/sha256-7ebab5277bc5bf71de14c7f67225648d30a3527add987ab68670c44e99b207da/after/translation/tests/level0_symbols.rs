//! Level 0: ABI surface. Every dynamic symbol the C `.so` defines must also be
//! defined by the Rust `.so` under the exact same name, and every one of them
//! must be resolvable through `dlsym`.

mod harness;

use harness::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only` -> {name: type letter}
fn defined_symbols(path: &Path) -> BTreeMap<String, char> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {:?}: {}",
        path,
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        let ty = cols[cols.len() - 2].chars().next().unwrap_or('?');
        let name = cols[cols.len() - 1].to_string();
        map.insert(name, ty);
    }
    map
}

/// Symbols glibc/rustc inject into every shared object; they are not part of the
/// translated API.
fn is_toolchain_symbol(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__")
        || name.starts_with("_fini")
        || name.starts_with("_init")
        || name == "_edata"
        || name == "_end"
        || name.starts_with("rust_")
        || name.starts_with("_R")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_symbols(&c_lib_path());
    let r = defined_symbols(&rust_lib_path());

    let c_api: Vec<&String> = c.keys().filter(|n| !is_toolchain_symbol(n)).collect();
    assert!(
        !c_api.is_empty(),
        "no API symbols found in the C .so - nm parsing is broken"
    );

    let missing: Vec<&&String> = c_api.iter().filter(|n| !r.contains_key(**n)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\nC API symbols:   {:?}\nRust symbols:    {:?}",
        missing.len(),
        missing,
        c_api,
        r.keys().filter(|n| !is_toolchain_symbol(n)).collect::<Vec<_>>()
    );

    // the C source has exactly these non-static definitions
    let expected = [
        "sh_puts",
        "strkey",
        "stbds_arrfreef",
        "stbds_arrgrowf",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_hmdel_key",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_rand_seed",
        "stbds_shmode_func",
        "stbds_stralloc",
        "stbds_strreset",
    ];
    for name in expected {
        assert!(c.contains_key(name), "C .so lost {}", name);
        assert!(r.contains_key(name), "Rust .so lost {}", name);
    }
}

/// Functions must be exported as *code* symbols on both sides, not as data.
#[test]
fn symbol_kinds_agree() {
    let c = defined_symbols(&c_lib_path());
    let r = defined_symbols(&rust_lib_path());
    for (name, cty) in c.iter().filter(|(n, _)| !is_toolchain_symbol(n)) {
        let rty = r.get(name).unwrap_or_else(|| panic!("missing {}", name));
        assert_eq!(
            cty.to_ascii_uppercase(),
            rty.to_ascii_uppercase(),
            "symbol {} is {} in the C .so but {} in the Rust .so",
            name,
            cty,
            rty
        );
    }
}

/// The whole test suite already dlsym's every symbol; this makes the
/// requirement explicit and independent of `nm`.
#[test]
fn every_symbol_is_dlsym_resolvable() {
    // Api::open panics if any symbol is absent from either library.
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
}
