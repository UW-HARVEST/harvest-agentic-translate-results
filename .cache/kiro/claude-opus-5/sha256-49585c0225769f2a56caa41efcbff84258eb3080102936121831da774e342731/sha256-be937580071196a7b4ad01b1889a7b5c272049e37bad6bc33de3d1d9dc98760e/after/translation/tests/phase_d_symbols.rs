//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs for every feature combination (see `run_all_features.sh`), because the
//! export set is a property of each build, not just of the default one.

mod common;

use common::*;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], path: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("nm {args:?} {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        // strip glibc version suffixes: `printf@GLIBC_2.2.5` -> `printf`
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// The eight symbols `mdmacros.h` declares as the public surface of `mdcore.c`.
const EXPECTED: [&str; 8] = [
    "G_OP",
    "G_OP_NAME",
    "helper_call",
    "helper_ptr",
    "op_add",
    "op_mul",
    "op_sub",
    "use_generated",
];

#[test]
fn d1_exported_symbol_diff_is_empty() {
    let a = artifacts();
    let c = nm(&["-D", "--defined-only"], &a.c_lib);
    let r = nm(&["-D", "--defined-only"], &a.r_lib);

    let mut expected = EXPECTED.to_vec();
    expected.sort();
    assert_eq!(
        c,
        expected,
        "[{}] the C .so must export exactly the header's public surface",
        tag()
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "[{}] symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}",
        tag()
    );
}

/// Symbols that are `static` in the C (or live in `mdmain.c`) must not leak into
/// the Rust `.so` either — an extra export is a surface mismatch too.
#[test]
fn d2_rust_exports_nothing_extra() {
    let a = artifacts();
    let c = nm(&["-D", "--defined-only"], &a.c_lib);
    let r = nm(&["-D", "--defined-only"], &a.r_lib);
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    assert!(
        extra.is_empty(),
        "[{}] the Rust .so exports symbols the C .so does not: {extra:?}",
        tag()
    );
}

/// `accum_<OP>` comes from `DEFINE_ACCUM(op)`, which declares it `static int`, so
/// it must be absent from both. `main` lives in `mdmain.c`, also absent.
#[test]
fn d3_static_and_main_are_not_exported() {
    let a = artifacts();
    for lib in [&a.c_lib, &a.r_lib] {
        let syms = nm(&["-D", "--defined-only"], lib);
        for forbidden in ["accum_add", "accum_sub", "accum_mul", "main", "atoi"] {
            assert!(
                !syms.iter().any(|s| s == forbidden),
                "[{}] {} must not export {forbidden}",
                tag(),
                lib.display()
            );
        }
    }
}

/// Every undefined dynamic symbol in the Rust `.so` must be satisfied by libc /
/// libgcc_s / the loader. `ldd` reporting no "not found" entry, plus a successful
/// `dlopen(RTLD_NOW)` (which `both()` already performs), proves it.
#[test]
fn d4_no_unresolved_non_libc_symbols() {
    let a = artifacts();
    for lib in [&a.c_lib, &a.r_lib] {
        let out = Command::new("ldd").arg(lib).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.contains("not found"),
            "[{}] unresolved shared-object dependency in {}:\n{text}",
            tag(),
            lib.display()
        );
        let allowed = ["libc.so", "libgcc_s.so", "ld-linux", "linux-vdso", "libm.so"];
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || !line.contains("=>") && !line.contains('(') {
                continue;
            }
            let name = line.split_whitespace().next().unwrap_or("");
            assert!(
                allowed.iter().any(|a| name.contains(a)),
                "[{}] {} depends on an unexpected library: {line}",
                tag(),
                lib.display()
            );
        }
    }
    // RTLD_NOW resolution of every symbol actually used.
    let _ = both();
}

/// The C `.so`'s export set must be identical in every configuration; if a
/// future `OP`/`REPEAT` value changed it, the parity check above would silently
/// compare a different surface.
#[test]
fn d5_export_set_is_configuration_independent() {
    let a = artifacts();
    let c = nm(&["-D", "--defined-only"], &a.c_lib);
    let mut expected = EXPECTED.to_vec();
    expected.sort();
    assert_eq!(
        c,
        expected,
        "[{}] C export set changed with configuration",
        tag()
    );
}
