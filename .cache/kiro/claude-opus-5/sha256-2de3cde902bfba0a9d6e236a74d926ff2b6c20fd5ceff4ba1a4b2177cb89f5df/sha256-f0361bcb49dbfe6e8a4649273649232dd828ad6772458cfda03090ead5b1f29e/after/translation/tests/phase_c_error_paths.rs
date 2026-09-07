//! Phase C — error-path differential tests. One test per row of `ERRORS.md`.
//!
//! This library has no error codes; its rejection surface is the `default:`
//! arm of `switch (pfcn)`, which yields the sentinel return value `0`. Each
//! test asserts C and Rust return the *same* sentinel, and (as ground truth)
//! that the sentinel is `0`.

mod common;

use common::{Lcg, Pair};

/// Assert C and Rust agree AND that the shared answer is the rejection
/// sentinel `0`.
#[track_caller]
fn assert_rejected(p: &Pair, pfcn: i32) {
    let c = p.c(pfcn);
    let r = p.rust(pfcn);
    assert_eq!(
        c, r,
        "divergence for get_predict_func({pfcn}) [0x{pfcn:08x}]: C={c} Rust={r}"
    );
    assert_eq!(
        c, 0,
        "C ground truth: pfcn={pfcn} is outside 0..=11 so it must reject with 0, got {c}"
    );
}

/// Row 1: `pfcn == 12` — first value past `BTAC1C2_GetPredictFunc`'s domain,
/// yet a valid `case` label inside `BTAC1C2_PredictSample`. The fallback
/// pointer matches no comparison, so `get_predict_func` returns 0.
#[test]
fn err_row01_pfcn_12() {
    let p = Pair::load();
    assert_rejected(&p, 12);
}

/// Row 2: `pfcn` 13, 14, 15 — the remaining `BTAC1C2_PredictSample` labels.
#[test]
fn err_row02_pfcn_13_14_15() {
    let p = Pair::load();
    for v in 13..=15 {
        assert_rejected(&p, v);
    }
}

/// Row 3: `pfcn == 16` — first value past every case label in the file.
#[test]
fn err_row03_pfcn_16() {
    let p = Pair::load();
    assert_rejected(&p, 16);
}

/// Row 4: `pfcn == -1` — one step below the valid domain.
#[test]
fn err_row04_pfcn_minus_1() {
    let p = Pair::load();
    assert_rejected(&p, -1);
}

/// Row 5: arbitrary negatives.
#[test]
fn err_row05_pfcn_negative() {
    let p = Pair::load();
    for v in [-2, -3, -8, -11, -12, -13, -15, -16, -17, -100, -1000, -65536] {
        assert_rejected(&p, v);
    }
    for v in -2048..0 {
        assert_rejected(&p, v);
    }
}

/// Row 6: `INT_MAX`.
#[test]
fn err_row06_pfcn_int_max() {
    let p = Pair::load();
    assert_rejected(&p, i32::MAX);
}

/// Row 7: `INT_MIN`. `pfcn - 12` overflows here, so any jump-table offset
/// lowering must not mis-dispatch into a valid arm.
#[test]
fn err_row07_pfcn_int_min() {
    let p = Pair::load();
    assert_rejected(&p, i32::MIN);
    // Neighbours of the extremes too.
    for v in [i32::MIN + 1, i32::MIN + 12, i32::MAX - 1, i32::MAX - 12] {
        assert_rejected(&p, v);
    }
}

/// Row 8: out-of-range "enum" values crossing the FFI boundary. `pfcn` is an
/// `int`, so C accepts any 32-bit value; a value with no valid variant is a
/// real input and must be handled identically.
#[test]
fn err_row08_out_of_range_enum_values() {
    let p = Pair::load();
    for v in [
        0x10, 0x1F, 0x20, 0x7F, 0xFF, 0x100, 0x1000, 0xFFFF, 0x1_0000, 0x7FFF_FFFE, 0x7FFF_FFFF,
    ] {
        assert_rejected(&p, v);
    }
    // Same bit patterns reinterpreted as negatives.
    for v in [
        -0x10i32, -0x20, -0xFF, -0x100, -0xFFFF, -0x1_0000, -0x7FFF_FFFF,
    ] {
        assert_rejected(&p, v);
    }
}

/// Row 9: bit-pattern aliasing. A translation that masked (`pfcn & 15`,
/// `pfcn & 7`) instead of comparing would wrongly accept these.
#[test]
fn err_row09_masking_aliases() {
    let p = Pair::load();
    for v in [16, 32, 64, 128, 256, 4096, 65536, -16, -32, -256, -4096] {
        assert_rejected(&p, v);
    }
    // k + 2^n for every valid k and several powers of two.
    for k in 0..=11i32 {
        for shift in [4u32, 5, 8, 12, 16, 24, 30] {
            let v = k + (1i32 << shift);
            assert_rejected(&p, v);
            let v2 = k - (1i32 << shift);
            assert_rejected(&p, v2);
        }
    }
}

/// Row 10: exhaustive near-domain sweep plus a large fixed-seed random sweep,
/// asserting the exact C classification (1 iff 0..=11) on every value.
#[test]
fn err_row10_exhaustive_and_random() {
    let p = Pair::load();

    for v in -2048..=2048i32 {
        let c = p.c(v);
        let r = p.rust(v);
        assert_eq!(c, r, "divergence at pfcn={v}: C={c} Rust={r}");
        let expect = if (0..=11).contains(&v) { 1 } else { 0 };
        assert_eq!(c, expect, "C ground truth changed for pfcn={v}");
    }

    let mut rng = Lcg::new(0xDEAD_BEEF_0000_0007);
    for _ in 0..200_000 {
        let v = rng.next_i32();
        let c = p.c(v);
        let r = p.rust(v);
        assert_eq!(c, r, "divergence at pfcn={v} [0x{v:08x}]: C={c} Rust={r}");
        let expect = if (0..=11).contains(&v) { 1 } else { 0 };
        assert_eq!(c, expect, "C ground truth changed for pfcn={v}");
    }
}

/// Rows 11 & 12 concern `static` C functions with local linkage: they appear in
/// neither library's `nm -D`, so no FFI call can reach them. Assert that fact
/// (rather than stubbing a fake export), which also documents that the Rust
/// translation kept the same private linkage as the C.
#[test]
fn err_rows11_12_static_helpers_are_not_exported() {
    let c = unsafe { libloading::Library::new(common::c_lib_path()).unwrap() };
    let r = unsafe { libloading::Library::new(common::rust_lib_path()).unwrap() };

    let mut names: Vec<String> = vec![
        "BTAC1C2_GetPredictFunc\0".to_string(),
        "BTAC1C2_PredictSample\0".to_string(),
    ];
    for i in 0..12 {
        names.push(format!("BTAC1C2_PredictSample_Pfn{i}\0"));
    }

    for n in &names {
        let in_c = unsafe {
            r#unsafe_get(&c, n)
        };
        let in_r = unsafe { r#unsafe_get(&r, n) };
        assert!(!in_c, "C unexpectedly exports {n:?}");
        assert_eq!(
            in_c, in_r,
            "export-visibility mismatch for {n:?}: C={in_c} Rust={in_r}"
        );
    }
}

unsafe fn r#unsafe_get(lib: &libloading::Library, name: &str) -> bool {
    unsafe {
        lib.get::<*const ()>(name.as_bytes()).is_ok()
    }
}
