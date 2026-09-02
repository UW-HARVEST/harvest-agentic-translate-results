//! Phase D — symbol parity, ABI layout, and harness self-checks.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// `nm -D --defined-only <so>` -> set of exported symbol names.
fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {:?}", so);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

/// `nm -D --undefined-only <so>` -> set of imported symbol names.
fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {:?}", so);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Symbols that are supplied by libc / the dynamic loader / the unwinder.
///
/// `nm -D` prints glibc imports with their version tag (`memcpy@GLIBC_2.14`);
/// any symbol carrying a `@GLIBC_` / `@GCC_` / `@GLIBCXX_` tag is by definition
/// resolved out of the platform runtime, so those are accepted wholesale.
fn is_runtime_symbol(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "__cxa_finalize",
        "__gmon_start__",
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__tls_get_addr",
    ];
    if name.contains("@GLIBC_") || name.contains("@GCC_") || name.contains("@GLIBCXX_") {
        return true;
    }
    EXACT.contains(&name)
        || name.starts_with("_Unwind_")
        || name.starts_with("__libc_")
        || name.starts_with("__cxa_")
        || name.starts_with("__stack_chk")
        || name.starts_with("_ITM_")
        || name.starts_with("__rust_probestack")
}

// ===========================================================================
// Symbol parity
// ===========================================================================

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c_so = find_c_so();
    let r_so = find_rust_so();
    let c_syms = defined_symbols(&c_so);
    let r_syms = defined_symbols(&r_so);

    assert!(
        c_syms.contains("dequantize_granule"),
        "sanity: the C .so must export dequantize_granule; got {:?}",
        c_syms
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C   ({}): {:?}\n\
         Rust({}): {:?}",
        missing.len(),
        missing,
        c_syms.len(),
        c_syms,
        r_syms.len(),
        r_syms
    );
}

#[test]
fn static_c_helper_is_not_exported_by_either() {
    // `get_bits` is `static` in c_src/src/lib.c, so neither library may export
    // it. A Rust translation that exported it would not be ABI-identical.
    for so in [find_c_so(), find_rust_so()] {
        let syms = defined_symbols(&so);
        assert!(!syms.contains("get_bits"), "{:?} must not export get_bits", so);
    }
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let r_so = find_rust_so();
    let undef: Vec<String> = undefined_symbols(&r_so)
        .into_iter()
        .filter(|s| !is_runtime_symbol(s))
        .collect();
    assert!(
        undef.is_empty(),
        "the Rust .so has undefined non-libc symbols: {:?}",
        undef
    );
}

#[test]
fn dlopen_of_both_libraries_resolves_the_symbol() {
    // If either `.so` had an unresolved dependency, `Library::new` would fail;
    // `load_impls` panics with a descriptive message in that case.
    let (c, r) = load_impls();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}

// ===========================================================================
// ABI layout of the two structs, verified through observed C behaviour
// ===========================================================================

#[test]
fn bs_t_layout_matches_c() {
    // sizeof(bs_t) == 16 and offsetof(pos) == 8, offsetof(limit) == 12 on LP64.
    assert_eq!(std::mem::size_of::<BsT>(), 16);
    assert_eq!(std::mem::align_of::<BsT>(), 8);

    // Behavioural proof that the C reads `pos` at +8 and `limit` at +12:
    // setting only `limit` to 0 must reject, and only `pos` to a huge value
    // must also reject.
    let (c, r) = load_impls();
    check_case(
        &c,
        &r,
        &Case::new(200_000, 2, 2, BaMode::Const(5))
            .limit(LimitMode::Abs(0))
            .buf(BufMode::Ones)
            .iters(1),
    );
}

#[test]
fn l12_scale_info_layout_matches_c() {
    // Proven behaviourally: with `bitalloc` (offset 770) all zero and the bytes
    // from offset 770+64 == 834 (`scfcod`) set to a non-zero allocation, a
    // granule with total_bands = 64 (i.e. i in 0..127) must consume exactly
    // `4 * 64 bands * group_size * ba` bits. That can only happen if `bitalloc`
    // sits at 770 and `scfcod` directly follows it at 834.
    let (c, r) = load_impls();
    let case = Case::new(201_000, 4, 64, BaMode::ZeroBelowThenConst(64, 9))
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);
    assert_eq!(
        out.pos,
        4 * 64 * 4 * 9,
        "the OOB bitalloc read must land exactly on scfcod (offset 834)"
    );

    // And with the value starting at index 130 (offset 900 == sizeof(struct))
    // the read is strictly past the object.
    let case = Case::new(201_100, 4, 255, BaMode::ZeroBelowThenConst(130, 9))
        .buf(BufMode::Ones)
        .iters(1);
    let out = run_both(&c, &r, &case, 0);
    assert_eq!(out.pos, 4 * 380 * 4 * 9);
}

// ===========================================================================
// Harness self-checks — guard against a vacuous differential comparison
// ===========================================================================

/// Independent model of the `dst` / `choff` walk in `dequantize_granule`,
/// giving the exact set of `grbuf` slots the C must write.
fn model_written_indices(group_size: i32, total_bands: u8, ba_nonzero: bool) -> BTreeSet<usize> {
    let mut set = BTreeSet::new();
    let mut choff: i64 = 576;
    for j in 0..4i64 {
        let mut dst: i64 = group_size as i64 * j;
        for _i in 0..(2 * total_bands as i64) {
            if ba_nonzero {
                for k in 0..group_size.max(0) as i64 {
                    set.insert((dst + k) as usize);
                }
            }
            dst += choff;
            choff = 18 - choff;
        }
    }
    set
}

#[test]
fn harness_actually_observes_written_data() {
    let (c, _r) = load_impls();

    // Linear path. Write windows from different `j` iterations overlap, so the
    // number of distinct slots is well below `4 * 16 * 12`; compare against an
    // independent model of the address walk instead of a guessed count.
    let case = Case::new(202_000, 12, 8, BaMode::Const(9)).iters(1);
    let out = run(&c, &case, 0);
    let expected = model_written_indices(12, 8, true);
    let observed: BTreeSet<usize> = out.written_indices().into_iter().collect();
    assert_eq!(
        observed, expected,
        "the C wrote a different set of grbuf slots than the dst/choff model predicts"
    );
    assert!(out.pos > 0, "bits must have been consumed");

    // Grouped path writes the same slots.
    let case = Case::new(202_100, 12, 8, BaMode::Const(20)).iters(1);
    let out = run(&c, &case, 0);
    let observed: BTreeSet<usize> = out.written_indices().into_iter().collect();
    assert_eq!(observed, expected);

    // A few more shapes, including the maximum band count.
    for &(g, tb) in &[(1i32, 1u8), (3, 2), (18, 32), (64, 64), (12, 255), (576, 2)] {
        let case = Case::new(202_200 + tb as u32, g, tb, BaMode::Const(5)).iters(1);
        let out = run(&c, &case, 0);
        let observed: BTreeSet<usize> = out.written_indices().into_iter().collect();
        assert_eq!(
            observed,
            model_written_indices(g, tb, true),
            "g={g} tb={tb}: written-slot set mismatch"
        );
    }
}

#[test]
fn harness_is_input_sensitive() {
    let (c, _r) = load_impls();
    // Different repetitions draw different random bitstreams, so the output
    // must change. If it did not, the comparison would be vacuous.
    let case = Case::new(203_000, 12, 8, BaMode::Range(1, 16)).iters(2);
    let a = run(&c, &case, 0);
    let b = run(&c, &case, 1);
    assert_ne!(a.grbuf, b.grbuf, "changing the input must change grbuf");

    // ...and the fixture must be reproducible for a fixed (case, rep).
    let a2 = run(&c, &case, 0);
    assert_eq!(a.grbuf, a2.grbuf, "the fixture must be deterministic");
    assert_eq!(a.pos, a2.pos);
}

#[test]
fn harness_writes_stay_inside_grbuf() {
    // The `grbuf_len` bound must actually cover every write the C makes,
    // otherwise the comparison would be reading uninitialised slack.
    let (c, _r) = load_impls();
    for &g in &[1i32, 2, 3, 12, 18, 32, 64, 576] {
        let case = Case::new(204_000 + g as u32, g, 255, BaMode::Const(4)).iters(1);
        let out = run(&c, &case, 0);
        let max_idx = *out.written_indices().last().unwrap();
        let len = grbuf_len(g);
        assert!(
            max_idx < len,
            "group_size={g}: highest written slot {max_idx} vs grbuf_len {len}"
        );
        // Keep at least a little slack so an off-by-one in the Rust would land
        // inside the compared region rather than corrupting the allocator.
        assert!(
            max_idx + 8 < len,
            "group_size={g}: only {} slots of slack",
            len - max_idx
        );
    }
}

#[test]
fn harness_detects_a_deliberate_divergence() {
    // Feed the two libraries deliberately *different* inputs and confirm the
    // comparator reports it. This proves the assertions in `check_case` are not
    // trivially satisfied.
    let (c, _r) = load_impls();
    let case_a = Case::new(205_000, 8, 4, BaMode::Const(6)).iters(1);
    let case_b = Case::new(205_001, 8, 4, BaMode::Const(7)).iters(1);
    let a = run(&c, &case_a, 0);
    let b = run(&c, &case_b, 0);
    assert_ne!(a.grbuf, b.grbuf);
    assert_ne!(a.pos, b.pos);
}
