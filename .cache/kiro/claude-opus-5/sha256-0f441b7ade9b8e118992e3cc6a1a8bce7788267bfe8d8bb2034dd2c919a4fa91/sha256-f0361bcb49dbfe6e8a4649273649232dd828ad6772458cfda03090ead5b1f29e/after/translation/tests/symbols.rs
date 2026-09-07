//! Phase A / Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        // Weak/compiler-emitted ABI plumbing present in every ELF shared object.
        .filter(|s| {
            !s.starts_with("_ITM_")
                && !s.starts_with("__cxa_")
                && !s.starts_with("__gmon_")
                && s != "_init"
                && s != "_fini"
                && s != "_edata"
                && s != "_end"
                && s != "__bss_start"
        })
        .collect()
}

fn symbol_sizes(so: &std::path::Path) -> Vec<(String, String, String)> {
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(so)
        .output()
        .expect("nm not available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() == 4 {
                Some((f[3].to_string(), f[2].to_string(), f[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn c_symbols_are_all_exported_by_rust() {
    let c = common::c_so_path();
    let r = common::rust_so_path();
    let cs = defined_dynamic_symbols(&c);
    let rs = defined_dynamic_symbols(&r);

    // Sanity: the three symbols the C source actually defines.
    for want in ["array", "long_exec", "perform_expensive_operations"] {
        assert!(cs.contains(want), "C .so unexpectedly lacks `{want}`");
    }

    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {cs:?}\nRust = {rs:?}"
    );
}

#[test]
fn array_object_has_identical_type_and_size() {
    let cs = symbol_sizes(&common::c_so_path());
    let rs = symbol_sizes(&common::rust_so_path());
    let find = |v: &[(String, String, String)], n: &str| {
        v.iter()
            .find(|(name, _, _)| name == n)
            .map(|(_, t, sz)| (t.clone(), sz.clone()))
    };
    let (ct, csz) = find(&cs, "array").expect("C array symbol");
    let (rt, rsz) = find(&rs, "array").expect("Rust array symbol");
    assert_eq!(ct, rt, "array symbol type differs (C={ct} Rust={rt})");
    assert_eq!(
        u64::from_str_radix(&csz, 16).unwrap(),
        u64::from_str_radix(&rsz, 16).unwrap(),
        "array symbol size differs (C=0x{csz} Rust=0x{rsz})"
    );
    assert_eq!(
        u64::from_str_radix(&csz, 16).unwrap(),
        (common::ARRAY_SIZE * 4) as u64,
        "array is not 1 MiB"
    );
}

#[test]
fn rust_so_has_no_missing_non_libc_imports() {
    // `nm -D -u` lists undefined symbols; every one must be satisfiable from
    // libc / libgcc, i.e. none may be an untranslated module of the C library.
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(common::rust_so_path())
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let cs = defined_dynamic_symbols(&common::c_so_path());
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let base = name.split('@').next().unwrap();
        assert!(
            !cs.contains(base),
            "Rust .so imports `{base}`, which the C library *defines* -- \
             that means the Rust translation is calling into the C instead of \
             implementing it"
        );
    }
}

#[test]
fn both_libraries_resolve_all_three_symbols_via_dlsym() {
    let p = common::load_pair();
    let (c, r) = p.split();
    // `Impl::open` panics if any of the three symbols is unresolvable.
    assert_eq!(c.array().len(), common::ARRAY_SIZE);
    assert_eq!(r.array().len(), common::ARRAY_SIZE);
    let _ = c.lib();
    let _ = r.lib();
}
