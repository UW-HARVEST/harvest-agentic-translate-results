//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("`nm` must be on PATH");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

fn undefined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", so.to_str().unwrap()])
        .output()
        .expect("`nm` must be on PATH");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_dynamic_symbols(&common::c_so_asserts());
    let r = defined_dynamic_symbols(&common::rust_so());

    // Sanity: we really did read a symbol table.
    assert!(
        c.contains("unfilter") && c.contains("cp_inflate"),
        "unexpected C symbol table: {c:?}"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   : {c:?}\nRust: {r:?}"
    );

    // Also check the release C build, in case the two configurations differ.
    let c2 = defined_dynamic_symbols(&common::c_so_release());
    let missing2: Vec<_> = c2.difference(&r).cloned().collect();
    assert!(
        missing2.is_empty(),
        "symbols exported by the release C .so but MISSING from the Rust .so: {missing2:?}"
    );

    // And report (without failing) anything extra on the Rust side.
    let extra: Vec<_> = r.difference(&c).cloned().collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );

    eprintln!("symbol parity OK: {} symbols\n{c:#?}", c.len());
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let u = undefined_dynamic_symbols(&common::rust_so());
    // Everything the Rust cdylib imports must come from glibc / libgcc, i.e.
    // carry a `@GLIBC_x.y` / `@GCC_x.y` version tag, or be one of the three
    // weak link-editor markers that every ELF shared object has.
    let bad: Vec<_> = u
        .iter()
        .filter(|s| {
            !(s.contains("@GLIBC_")
                || s.contains("@GCC_")
                || matches!(
                    s.as_str(),
                    "_ITM_deregisterTMCloneTable"
                        | "_ITM_registerTMCloneTable"
                        | "__gmon_start__"
                ))
        })
        .cloned()
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols: {bad:?}\nall: {u:?}"
    );

    // The same check for the C .so, as a control.
    let uc = undefined_dynamic_symbols(&common::c_so_release());
    let badc: Vec<_> = uc
        .iter()
        .filter(|s| {
            !(s.contains("@GLIBC_")
                || s.contains("@GCC_")
                || matches!(
                    s.as_str(),
                    "_ITM_deregisterTMCloneTable"
                        | "_ITM_registerTMCloneTable"
                        | "__gmon_start__"
                ))
        })
        .cloned()
        .collect();
    assert!(badc.is_empty(), "C .so has unexpected imports: {badc:?}");
}

#[test]
fn exported_data_tables_are_byte_identical() {
    let p = common::pair_release();
    unsafe {
        let cmp_u8 = |name: &str, a: *const u8, b: *const u8, n: usize| {
            let x = std::slice::from_raw_parts(a, n);
            let y = std::slice::from_raw_parts(b, n);
            assert_eq!(x, y, "{name} differs");
        };
        let cmp_u32 = |name: &str, a: *const u32, b: *const u32, n: usize| {
            let x = std::slice::from_raw_parts(a, n);
            let y = std::slice::from_raw_parts(b, n);
            assert_eq!(x, y, "{name} differs");
        };
        cmp_u8(
            "cp_fixed_table",
            p.c.fixed_table,
            p.rust.fixed_table,
            288 + 32,
        );
        cmp_u8(
            "cp_permutation_order",
            p.c.permutation_order,
            p.rust.permutation_order,
            19,
        );
        cmp_u8(
            "cp_len_extra_bits",
            p.c.len_extra_bits,
            p.rust.len_extra_bits,
            29 + 2,
        );
        cmp_u32("cp_len_base", p.c.len_base, p.rust.len_base, 29 + 2);
        cmp_u8(
            "cp_dist_extra_bits",
            p.c.dist_extra_bits,
            p.rust.dist_extra_bits,
            30 + 2,
        );
        cmp_u32("cp_dist_base", p.c.dist_base, p.rust.dist_base, 30 + 2);
        // cp_error_reason starts out NULL in both.
        assert!((*p.c.error_reason).is_null());
        assert!((*p.rust.error_reason).is_null());
    }
}
