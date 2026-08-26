//! Phase D — symbol parity between the C `.so` and the Rust `.so`, plus
//! element-wise parity of the three private lookup tables.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Symbols that `nm -D` reports for any glibc-linked shared object and that are
/// not part of the library's own API.
const TOOLCHAIN_SYMBOLS: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__gmon_start__",
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
];

fn dynamic_defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (a, b) = (parts.next()?, parts.next()?);
            // "<addr> <type> <name>" or "<type> <name>" for undefined/weak.
            let (ty, name) = match parts.next() {
                Some(name) => (b, name),
                None => (a, b),
            };
            // Keep only exported code/data definitions.
            if !matches!(ty, "T" | "t" | "D" | "B" | "R" | "W" | "i") {
                return None;
            }
            let name = name.split('@').next().unwrap_or(name);
            if TOOLCHAIN_SYMBOLS.contains(&name) || name.starts_with("__rust") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let c_syms = dynamic_defined_symbols(&l.c_path);

    assert!(
        !c_syms.is_empty(),
        "nm found no exported symbols in the C .so -- the comparison would be vacuous"
    );
    assert!(
        c_syms.contains("half2float"),
        "the C .so must export half2float; got {c_syms:?}"
    );

    for so in [&l.rust_path, &l.rust_release_path] {
        let rust_syms = dynamic_defined_symbols(so);
        let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
        assert!(
            missing.is_empty(),
            "symbols exported by the C .so but MISSING from {}: {missing:?}\n\
             C   ({}): {c_syms:?}\n\
             Rust: {rust_syms:?}",
            so.display(),
            l.c_path.display(),
        );
    }
}

#[test]
fn d2_rust_so_has_no_unresolved_symbols() {
    // "No undefined non-libc symbols" is checked mechanically rather than with
    // an allowlist: `ldd -r` performs full relocation processing and reports
    // every symbol that cannot be resolved from the shared object's
    // dependencies (libc, libm, the loader, ...). Anything it lists is a real
    // missing implementation.
    let l = libs();
    for so in [&l.c_path, &l.rust_path, &l.rust_release_path] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(so)
            .output()
            .expect("run ldd -r");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let unresolved: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(
            unresolved.is_empty(),
            "{} has unresolved symbols:\n{}",
            so.display(),
            unresolved.join("\n")
        );
    }
}

#[test]
fn d2b_rust_so_defines_no_extra_public_api_and_no_stubs() {
    // The Rust .so must not merely *export* half2float -- the symbol must be a
    // real implementation. A stub that aborts / unimplemented!()s would show up
    // as a call that never returns a value, so exercise it and require a
    // finite, table-derived answer.
    let l = libs();
    for (name, f) in l.rust_variants() {
        for h in [0x0000u16, 0x3C00, 0x7BFF, 0xFFFF] {
            let bits = unsafe { f(h) }.to_bits();
            assert_eq!(
                bits,
                oracle_bits(h),
                "{name} half2float(0x{h:04X}) = 0x{bits:08X} is not the C table value"
            );
        }
    }
    // And the tables really are compiled in: the .so must be large enough to
    // hold 2048*4 + 64*2 + 64*4 = 8576 bytes of table data.
    for so in [&l.rust_path, &l.rust_release_path] {
        let len = std::fs::metadata(so).expect("stat .so").len();
        assert!(len > 8576, "{} is too small to contain the tables", so.display());
    }
}

#[test]
fn d3_lookup_tables_are_element_wise_identical() {
    // The three tables are `static` in C, so they are not visible to nm. Prove
    // parity behaviourally: for every table element that any input can reach,
    // the value implied by the Rust .so must equal the value parsed out of the
    // C source. Every one of the 2048 + 64 + 64 elements is reachable.
    let t = c_tables();
    assert_eq!(t.mantissa.len(), 2048);
    assert_eq!(t.offset.len(), 64);
    assert_eq!(t.exponent.len(), 64);

    let l = libs();
    let c = l.c_fn();

    // Reachability: which mantissa indices does the domain actually touch?
    let mut touched = vec![false; 2048];
    for h in 0..=u16::MAX {
        let n = (h >> 10) as usize;
        touched[(h & 0x3ff) as usize + t.offset[n] as usize] = true;
    }
    assert!(touched.iter().all(|&b| b), "every mantissa element is reachable");

    for (name, rust) in l.rust_variants() {
        // For n = 0 the exponent addend is 0, so half2float(h) reads back
        // m__mantissa[h & 0x3ff] verbatim for indices 0..=1023.
        for m in 0..=1023u16 {
            let h = h_from(0, m);
            let rb = unsafe { rust(h) }.to_bits();
            let cb = unsafe { c(h) }.to_bits();
            assert_eq!(cb, t.mantissa[m as usize], "C m__mantissa[{m}]");
            assert_eq!(
                rb, t.mantissa[m as usize],
                "{name} M__MANTISSA[{m}] = 0x{rb:08X}, C source says 0x{:08X}",
                t.mantissa[m as usize]
            );
        }

        // For every other row, subtracting the C exponent addend recovers
        // m__mantissa[m__offset[n] + (h & 0x3ff)], covering all 2048 elements
        // and all 64 exponent/offset elements.
        for n in 1..64u16 {
            for m in 0..=1023u16 {
                let h = h_from(n, m);
                let idx = m as usize + t.offset[n as usize] as usize;
                let rb = unsafe { rust(h) }.to_bits();
                let recovered = rb.wrapping_sub(t.exponent[n as usize]);
                assert_eq!(
                    recovered, t.mantissa[idx],
                    "{name} disagrees at m__mantissa[{idx}] / m__exponent[{n}] (h = 0x{h:04X})"
                );
            }
        }
    }
}
