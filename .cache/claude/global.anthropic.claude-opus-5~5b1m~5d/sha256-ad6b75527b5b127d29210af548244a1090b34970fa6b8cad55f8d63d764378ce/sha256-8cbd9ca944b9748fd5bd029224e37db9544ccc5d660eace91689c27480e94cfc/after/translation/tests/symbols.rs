//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every symbol the C library exports must be exported by the Rust library
//! under the *exact* same name, and must be reachable through `dlopen`/`dlsym`
//! (which is what the rest of the suite uses).

mod common;
use common::*;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("`nm` not available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn c_and_rust_export_the_same_symbols() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         Rust({}): {r:?}",
        c.len(),
        r.len()
    );
    // SYMBOLS.md documents exactly these eight.
    let expected = [
        "cp_dist_base",
        "cp_dist_extra_bits",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_len_base",
        "cp_len_extra_bits",
        "cp_permutation_order",
        "pinflate",
    ];
    assert_eq!(c, expected, "the C .so's export list changed; update SYMBOLS.md");
    eprintln!("symbol diff (C -> Rust) is empty; {} symbols", c.len());
}

#[test]
fn every_symbol_resolves_through_dlsym_in_both() {
    // `nm` looking right is not enough: the tests actually call through
    // `dlsym`, so check that too, for both libraries.
    let p = pair();
    let _g = lock();
    // both `pinflate` symbols resolved (dlsym would have panicked otherwise)
    assert_ne!(p.c.pinflate as usize, p.rs.pinflate as usize);
    for (tag, l) in [("C", &p.c), ("Rust", &p.rs)] {
        assert!(!l.error_reason.is_null(), "{tag}: cp_error_reason");
        assert!(!l.fixed_table.is_null(), "{tag}: cp_fixed_table");
        assert!(!l.permutation_order.is_null(), "{tag}: cp_permutation_order");
        assert!(!l.len_extra_bits.is_null(), "{tag}: cp_len_extra_bits");
        assert!(!l.len_base.is_null(), "{tag}: cp_len_base");
        assert!(!l.dist_extra_bits.is_null(), "{tag}: cp_dist_extra_bits");
        assert!(!l.dist_base.is_null(), "{tag}: cp_dist_base");
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // `dlopen` with RTLD_NOW (what `libloading::Library::new` uses) already
    // fails if anything is unresolvable; make that explicit and also check the
    // undefined list contains nothing outside libc / the Rust runtime.
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(rust_so_path())
        .output()
        .expect("`nm` not available");
    let mut unexpected = Vec::new();
    for l in String::from_utf8_lossy(&out.stdout).lines() {
        let Some(name) = l.split_whitespace().last() else { continue };
        let base = name.split('@').next().unwrap();
        let known = base.starts_with("__")
            || base.starts_with('_')
            || base.starts_with("pthread_")
            || matches!(
                base,
                "abort" | "bcmp" | "calloc" | "close" | "free" | "getcwd" | "getenv" | "gettid"
                    | "lseek64" | "malloc" | "memcpy" | "memmove" | "memset" | "mmap64"
                    | "munmap" | "open64" | "posix_memalign" | "read" | "readlink" | "realloc"
                    | "realpath" | "stat64" | "statx" | "strlen" | "syscall" | "write"
                    | "writev" | "fstat64" | "dl_iterate_phdr"
            );
        if !known {
            unexpected.push(name.to_string());
        }
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unexpected undefined symbols: {unexpected:?}"
    );
}

#[test]
fn exported_data_symbols_have_the_c_sizes() {
    // `nm -S` reports the symbol sizes; the arrays must match the C
    // declarations, because callers may legitimately write to them (and the
    // tests do).
    let sizes = |p: &std::path::Path| -> std::collections::BTreeMap<String, u64> {
        let out = std::process::Command::new("nm")
            .arg("-D")
            .arg("-S")
            .arg("--defined-only")
            .arg(p)
            .output()
            .unwrap();
        let mut m = std::collections::BTreeMap::new();
        for l in String::from_utf8_lossy(&out.stdout).lines() {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() == 4 {
                if let Ok(sz) = u64::from_str_radix(f[1], 16) {
                    m.insert(f[3].to_string(), sz);
                }
            }
        }
        m
    };
    let c = sizes(&c_so_path());
    let r = sizes(&rust_so_path());
    let want: [(&str, u64); 7] = [
        ("cp_fixed_table", 320),
        ("cp_permutation_order", 19),
        ("cp_len_extra_bits", 31),
        ("cp_len_base", 124),
        ("cp_dist_extra_bits", 32),
        ("cp_dist_base", 128),
        ("cp_error_reason", 8),
    ];
    for (name, sz) in want {
        assert_eq!(c.get(name), Some(&sz), "C size of {name}");
        assert_eq!(r.get(name), Some(&sz), "Rust size of {name}");
    }
}
