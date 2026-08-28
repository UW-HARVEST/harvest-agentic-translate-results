//! Phase D -- symbol parity between the C and Rust shared objects.
//!
//! Asserts the `SYMBOLS.md` claim programmatically: every dynamic symbol the C
//! `.so` defines must also be defined by the Rust `.so` under the exact same
//! name, and the Rust `.so` must have no unresolvable (non-libc) symbol.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::*;

/// Defined (exported) dynamic symbol names, via `nm -D --defined-only`.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm (is binutils installed?)");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed:\n{}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>"  or  "        <type> <name>"
            let mut it = line.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            let (ty, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Skip the local/absolute noise; keep real exported definitions.
            if ty.len() != 1 {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Undefined dynamic symbol names, via `nm -D -u`.
fn undefined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm -D -u failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = c_so_path();
    let r = rust_so_path();

    let c_syms = defined_dynamic_symbols(&c);
    let r_syms = defined_dynamic_symbols(&r);

    println!("C   .so: {} ({} defined symbols)", c.display(), c_syms.len());
    for s in &c_syms {
        println!("    {s}");
    }
    println!("Rust .so: {} ({} defined symbols)", r.display(), r_syms.len());

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or \
         translate the missing C source.",
        missing.len(),
        missing
    );

    // The C library is a single translation unit with a single public function;
    // pin that so a future C change cannot silently widen the surface without
    // this test noticing.
    assert!(
        c_syms.contains("hdr_bitrate"),
        "expected hdr_bitrate among the C exports, got {c_syms:?}"
    );
    assert_eq!(
        c_syms.len(),
        1,
        "SYMBOLS.md records exactly 1 exported C symbol; nm now reports {c_syms:?}"
    );
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let r = rust_so_path();

    // `RTLD_NOW` forces every relocation to be resolved at load time, so a
    // successful open proves there is no unresolvable symbol.
    let lib = unsafe {
        libloading::os::unix::Library::open(
            Some(&r),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
        )
    }
    .unwrap_or_else(|e| panic!("dlopen(RTLD_NOW) of {} failed: {e}", r.display()));

    // And the exported entry point is reachable through dlsym.
    let f: libloading::os::unix::Symbol<HdrBitrateFn> =
        unsafe { lib.get(b"hdr_bitrate\0") }.expect("dlsym hdr_bitrate");
    let buf = [0xFFu8, 0xFB, 0x90];
    let v = unsafe { (*f)(buf.as_ptr()) };
    println!("Rust .so hdr_bitrate({buf:?}) = {v}");

    // Report the undefined-symbol list for the record: all entries must be
    // libc / toolchain-provided, which RTLD_NOW above has just proven.
    let undef = undefined_dynamic_symbols(&r);
    println!("Rust .so undefined (all satisfied by libc/toolchain): {undef:?}");
}

#[test]
fn phase_d_both_so_agree_through_dlsym_only() {
    // Belt and braces: resolve through dlsym on both and compare, so this file
    // also proves the exported wrapper (not just an internal Rust fn) is what
    // the differential tests exercised.
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xD);
    for _ in 0..5000 {
        let buf = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        p.assert_same(&buf, "Phase D dlsym round trip");
    }
}
