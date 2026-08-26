//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every dynamic symbol the C shared object defines must also be defined by the
//! Rust shared object under the exact same name, and must be resolvable with
//! `dlsym` (which is what an external C consumer actually does).

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Skip local/debug entries; keep global/weak text & data symbols.
            if kind.chars().next()?.is_uppercase() {
                Some(name.to_string())
            } else {
                None
            }
        })
        // Toolchain-injected symbols that are not part of the library API.
        .filter(|n| {
            !matches!(
                n.as_str(),
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
        .collect()
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let h = common::harness();
    let c_syms = defined_dynamic_symbols(&h.c.path);
    assert!(
        c_syms.contains("ldexp_q2"),
        "sanity: C .so must export ldexp_q2, got {c_syms:?}"
    );

    for r in &h.rust {
        let r_syms = defined_dynamic_symbols(&r.path);
        let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
        assert!(
            missing.is_empty(),
            "{} is missing {} symbol(s) exported by the C .so: {:?}\n\
             C symbols:    {:?}\n\
             Rust symbols: {:?}",
            r.name,
            missing.len(),
            missing,
            c_syms,
            r_syms
        );
    }
}

#[test]
fn d02_every_c_symbol_is_dlsym_resolvable_in_rust() {
    let h = common::harness();
    let c_syms = defined_dynamic_symbols(&h.c.path);
    for r in &h.rust {
        let lib = unsafe { libloading::Library::new(&r.path).unwrap() };
        for name in &c_syms {
            let mut key = name.clone().into_bytes();
            key.push(0);
            let res: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&key) };
            assert!(
                res.is_ok(),
                "dlsym({name}) failed in {}: {:?}",
                r.path.display(),
                res.err()
            );
        }
    }
}

#[test]
fn d03_rust_so_has_no_unresolved_symbols() {
    let h = common::harness();
    for r in &h.rust {
        let out = Command::new("ldd")
            .args(["-r", r.path.to_str().unwrap()])
            .output()
            .expect("failed to run ldd");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(
            bad.is_empty(),
            "{} has unresolved symbols:\n{}",
            r.path.display(),
            bad.join("\n")
        );
    }
}
