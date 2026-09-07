//! Phase C — error/rejection-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`. The C library has NO error return, no
//! `errno`, no sentinel and no validation, so "same rejection" is asserted as
//! "bit-identical observable effect" (raw `u32` bit patterns of the 3 written
//! floats, plus canaries, plus process-level fault parity for NULL).

mod common;

use common::*;
use std::process::Command;

/// The value `cvttss2si` (and therefore C's `(int)`) produces for NaN and for
/// any float outside `[-2^31, 2^31)`: the "integer indefinite" value.
const INT_INDEFINITE: i32 = i32::MIN;

/// Compute the `i` that C's `(int)floorf(h/60.0f)` yields, mirroring the
/// hardware conversion, so tests can assert WHICH switch arm was selected.
fn c_switch_index(h: f32) -> i32 {
    let x = (h / 60.0f32).floor();
    if x.is_nan() || x >= 2_147_483_648.0f32 || x <= -2_147_483_648.0f32 {
        INT_INDEFINITE
    } else {
        x as i32
    }
}

/// Assert the C output equals the `default:` arm (`r=v, g=p, b=q`).
#[track_caller]
fn assert_default_arm(row: &str, h: f32, s: f32, v: f32) {
    assert_eq!(
        c_switch_index(h) as i64 >= 5 || c_switch_index(h) < 0,
        true,
        "[{row}] expected h={} to select the default arm, got i={}",
        fmt(h),
        c_switch_index(h)
    );
    let out = check(row, [h, s, v]);
    let f = h / 60.0f32 - c_switch_index(h) as f32;
    let p = v * (1.0f32 - s);
    let q = v * (1.0f32 - s * f);
    assert_eq!(
        out.map(f32::to_bits),
        [v.to_bits(), p.to_bits(), q.to_bits()],
        "[{row}] default arm shape for h={} s={} v={}",
        fmt(h),
        fmt(s),
        fmt(v)
    );
}

// --- Row 1: s == +0.0f exactly -> early return -----------------------------
#[test]
fn err01_s_plus_zero_returns_v_verbatim() {
    let mut rng = Rng::new(SEED ^ 0x101);
    for &v in SPECIAL {
        for &h in SPECIAL {
            let out = check("err01", [h, 0.0, v]);
            assert_eq!(
                out.map(f32::to_bits),
                [v.to_bits(); 3],
                "err01 must copy v bit-verbatim (v={})",
                fmt(v)
            );
        }
    }
    for _ in 0..50_000 {
        let v = rng.any_f32();
        let out = check("err01/rand", [rng.any_f32(), 0.0, v]);
        assert_eq!(out.map(f32::to_bits), [v.to_bits(); 3]);
    }
}

// --- Row 2: s == -0.0f  (C: -0.0f == 0 is TRUE) ---------------------------
#[test]
fn err02_s_negative_zero_takes_early_return() {
    for &v in SPECIAL {
        for &h in SPECIAL {
            let out = check("err02", [h, -0.0, v]);
            assert_eq!(
                out.map(f32::to_bits),
                [v.to_bits(); 3],
                "err02: -0.0 saturation MUST take the early return (v={})",
                fmt(v)
            );
        }
    }
    // -0.0 and +0.0 must be indistinguishable here.
    let mut rng = Rng::new(SEED ^ 0x102);
    for _ in 0..20_000 {
        let h = rng.any_f32();
        let v = rng.any_f32();
        let a = check("err02/pz", [h, 0.0, v]);
        let b = check("err02/nz", [h, -0.0, v]);
        assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
    }
}

// --- Row 3: s = NaN -> `s == 0` is FALSE, full path taken -----------------
#[test]
fn err03_s_nan_skips_early_return() {
    let mut rng = Rng::new(SEED ^ 0x103);
    for &nan in &[f32::NAN, -f32::NAN, f32::from_bits(0x7fc0_0001), f32::from_bits(0xffff_ffff)] {
        for &h in SPECIAL {
            for &v in SPECIAL {
                let out = check("err03", [h, nan, v]);
                // NaN s must NOT have produced the early-return `[v,v,v]`
                // unless the arithmetic legitimately lands there.
                let _ = out;
            }
        }
        for _ in 0..20_000 {
            check("err03/rand", [rng.any_f32(), nan, rng.any_f32()]);
        }
    }
    // Explicit shape check: s=NaN, v=1.0, arm i=0 -> r=v=1.0, g=t=NaN, b=p=NaN
    let out = check("err03/shape", [30.0, f32::NAN, 1.0]);
    assert_eq!(out[0].to_bits(), 1.0f32.to_bits());
    assert!(out[1].is_nan() && out[2].is_nan(), "err03 expected NaN g/b");
}

// --- Row 4: h = NaN -> (int)NaN = INT_MIN -> default arm ------------------
#[test]
fn err04_h_nan_selects_default_arm() {
    assert_eq!(c_switch_index(f32::NAN), INT_INDEFINITE);
    let mut rng = Rng::new(SEED ^ 0x104);
    for &nan in &[
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::from_bits(0x7fff_ffff),
        f32::from_bits(0xffc0_0000),
    ] {
        for &s in SPECIAL {
            for &v in SPECIAL {
                check("err04", [nan, s, v]);
            }
        }
        for _ in 0..20_000 {
            let s = rng.range(f32::MIN_POSITIVE, 1.0);
            check("err04/rand", [nan, s, rng.range(-2.0, 2.0)]);
        }
        // must be the default arm, NOT case 0
        assert_default_arm("err04/shape", nan, 0.5, 1.0);
    }
}

// --- Row 5/6: h = ±Inf -> INT_MIN -> default arm -------------------------
#[test]
fn err05_06_h_infinite_selects_default_arm() {
    for &h in &[f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(c_switch_index(h), INT_INDEFINITE);
        for &s in SPECIAL {
            for &v in SPECIAL {
                check("err05_06", [h, s, v]);
            }
        }
        assert_default_arm("err05_06/shape", h, 0.5, 1.0);
        let mut rng = Rng::new(SEED ^ 0x105);
        for _ in 0..20_000 {
            check("err05_06/rand", [h, rng.any_f32(), rng.any_f32()]);
        }
    }
}

// --- Row 7: h/60 overflows int ------------------------------------------
#[test]
fn err07_h_overflows_int_range() {
    let mut rng = Rng::new(SEED ^ 0x107);
    for &h in &[
        1e30f32,
        -1e30f32,
        f32::MAX,
        f32::MIN,
        3.4e38,
        -3.4e38,
        1e20,
        -1e20,
        128_849_018_880.0,  // 2^31 * 60 exactly
        -128_849_018_880.0,
    ] {
        assert_eq!(
            c_switch_index(h),
            INT_INDEFINITE,
            "err07: h={} should overflow the int conversion",
            fmt(h)
        );
        for &s in SPECIAL {
            for &v in SPECIAL {
                check("err07", [h, s, v]);
            }
        }
        for _ in 0..5_000 {
            check("err07/rand", [h, rng.any_f32(), rng.any_f32()]);
        }
    }
}

// --- Rows 8/9/10: exact int-conversion boundaries on h/60 ----------------
#[test]
fn err08_09_10_int_conversion_boundaries() {
    // Construct h such that h/60 hits the exact boundary values.
    let cases: &[(&str, f32)] = &[
        ("2^31", 2_147_483_648.0f32 * 60.0),
        ("-2^31", -2_147_483_648.0f32 * 60.0),
        ("largest<2^31", 2_147_483_520.0f32 * 60.0),
        ("one below -2^31", -2_147_483_904.0f32 * 60.0),
    ];
    let mut rng = Rng::new(SEED ^ 0x108);
    for (name, h) in cases {
        for &s in SPECIAL {
            for &v in SPECIAL {
                check(&format!("err08_09_10/{name}"), [*h, s, v]);
            }
        }
        for _ in 0..5_000 {
            check(
                &format!("err08_09_10/{name}/rand"),
                [*h, rng.any_f32(), rng.any_f32()],
            );
        }
    }
    // Also feed h/60 boundaries directly as ULP walks around 2^31 and -2^31.
    for base in [2_147_483_648.0f32, -2_147_483_648.0f32] {
        let mut b = base.to_bits();
        for _ in 0..64 {
            let x = f32::from_bits(b);
            check("err08_09_10/ulp", [x * 60.0, 0.5, 1.0]);
            check("err08_09_10/ulp-direct", [x, 0.5, 1.0]);
            b = b.wrapping_add(1);
        }
    }
}

// --- Row 11: i == 5, first value past the last named case ----------------
#[test]
fn err11_i_five_uses_default_not_case_zero() {
    let mut rng = Rng::new(SEED ^ 0x10b);
    for _ in 0..20_000 {
        let h = rng.range(300.0, 360.0);
        assert_eq!(c_switch_index(h), 5);
        let s = rng.range(f32::MIN_POSITIVE, 1.0);
        let v = rng.range(0.0, 1.0);
        let out = check("err11", [h, s, v]);
        // default arm: r=v, g=p, b=q  (NOT case 0's r=v, g=t, b=p)
        let f = h / 60.0f32 - 5.0f32;
        let p = v * (1.0f32 - s);
        let q = v * (1.0f32 - s * f);
        assert_eq!(
            out.map(f32::to_bits),
            [v.to_bits(), p.to_bits(), q.to_bits()],
            "err11 must take default, no wrap to case 0 (h={})",
            fmt(h)
        );
    }
    assert_default_arm("err11/exact300", 300.0, 0.5, 1.0);
}

// --- Row 12: i == -1 (negative hue) -------------------------------------
#[test]
fn err12_i_negative_one() {
    let mut rng = Rng::new(SEED ^ 0x10c);
    for _ in 0..20_000 {
        let h = rng.range(-60.0, -f32::MIN_POSITIVE);
        assert_eq!(c_switch_index(h), -1, "h={}", fmt(h));
        let s = rng.range(f32::MIN_POSITIVE, 1.0);
        let v = rng.range(0.0, 1.0);
        check("err12", [h, s, v]);
    }
    // floorf(-0.5) = -1, so f = h/60 - (-1) is POSITIVE
    let h = -30.0f32;
    assert_eq!(c_switch_index(h), -1);
    let f = h / 60.0f32 - (-1.0f32);
    assert!(f > 0.0, "err12 fraction must be positive, got {f}");
    assert_default_arm("err12/shape", h, 0.5, 1.0);
}

// --- Row 13: large positive i ------------------------------------------
#[test]
fn err13_i_large_positive() {
    let mut rng = Rng::new(SEED ^ 0x10d);
    for &h in &[1e9f32, 1e6, 360.0, 3600.0, 1e7, 12_345_678.0] {
        for &s in SPECIAL {
            for &v in SPECIAL {
                check("err13", [h, s, v]);
            }
        }
        for _ in 0..5_000 {
            let s = rng.range(f32::MIN_POSITIVE, 1.0);
            check("err13/rand", [h, s, rng.range(-2.0, 2.0)]);
        }
    }
}

// --- Row 14: v special values with s != 0 -------------------------------
#[test]
fn err14_v_special_values() {
    let mut rng = Rng::new(SEED ^ 0x10e);
    let hues: &[f32] = &[30.0, 90.0, 150.0, 210.0, 270.0, 330.0, -30.0, f32::NAN];
    for &v in &[
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1e-45,
        -1e-45,
        f32::MAX,
        f32::MIN,
    ] {
        for &h in hues {
            for _ in 0..500 {
                let s = rng.range(f32::MIN_POSITIVE, 2.0);
                check("err14", [h, s, v]);
            }
            check("err14/s1", [h, 1.0, v]);
        }
    }
    // 0 * Inf must yield NaN identically: v=0, s=Inf => p = 0*(1-Inf) = -0*Inf
    check("err14/zero-times-inf", [30.0, f32::INFINITY, 0.0]);
    check("err14/inf-times-zero", [30.0, 0.0f32.next_up(), f32::INFINITY]);
}

// --- Row 15: s = ±Inf --------------------------------------------------
#[test]
fn err15_s_infinite() {
    let mut rng = Rng::new(SEED ^ 0x10f);
    for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &h in SPECIAL {
            for &v in SPECIAL {
                check("err15", [h, s, v]);
            }
        }
        for _ in 0..20_000 {
            check("err15/rand", [rng.any_f32(), s, rng.any_f32()]);
        }
    }
}

// --- Row 16: subnormal inputs (no flush-to-zero) ------------------------
#[test]
fn err16_subnormals() {
    let subs: &[f32] = &[
        1e-45,
        -1e-45,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007f_ffff), // largest subnormal
        f32::from_bits(0x807f_ffff),
        f32::MIN_POSITIVE,
    ];
    for &h in subs {
        for &s in subs {
            for &v in subs {
                // a subnormal s is NOT == 0, so the full path must be taken
                assert!(s != 0.0, "subnormal must compare != 0");
                check("err16", [h, s, v]);
            }
        }
    }
    // subnormal s combined with normal h/v across every arm
    for &h in &[30.0f32, 90.0, 150.0, 210.0, 270.0, 330.0, -30.0] {
        for &s in subs {
            for &v in &[1.0f32, 0.0, -1.0, f32::MAX, f32::INFINITY] {
                check("err16/arms", [h, s, v]);
            }
        }
    }
}

// --- Rows 17/18/19: aliasing & overlap ---------------------------------
#[track_caller]
fn check_aliased(row: &str, arena: &[f32], dest_off: usize, src_off: usize) {
    let l = libs();
    let mut c_arena: Vec<f32> = arena.to_vec();
    let mut r_arena: Vec<f32> = arena.to_vec();
    unsafe {
        (l.c)(
            c_arena.as_mut_ptr().add(dest_off),
            c_arena.as_ptr().add(src_off),
        );
        (l.rust)(
            r_arena.as_mut_ptr().add(dest_off),
            r_arena.as_ptr().add(src_off),
        );
    }
    assert_eq!(
        c_arena.iter().map(|f| f.to_bits()).collect::<Vec<_>>(),
        r_arena.iter().map(|f| f.to_bits()).collect::<Vec<_>>(),
        "[{row}] alias mismatch dest_off={dest_off} src_off={src_off}"
    );
}

#[test]
fn err17_full_alias_full_path() {
    let mut rng = Rng::new(SEED ^ 0x111);
    for _ in 0..20_000 {
        let arena = vec![
            rng.range(-720.0, 720.0),
            rng.range(f32::MIN_POSITIVE, 1.0),
            rng.range(-2.0, 2.0),
        ];
        check_aliased("err17", &arena, 0, 0);
    }
    for &h in SPECIAL {
        for &s in SPECIAL {
            for &v in SPECIAL {
                check_aliased("err17/special", &[h, s, v], 0, 0);
            }
        }
    }
}

#[test]
fn err18_full_alias_early_path() {
    let mut rng = Rng::new(SEED ^ 0x112);
    for &z in &[0.0f32, -0.0f32] {
        for _ in 0..20_000 {
            let arena = vec![rng.any_f32(), z, rng.any_f32()];
            let v = arena[2];
            let l = libs();
            let mut c_arena = arena.clone();
            let mut r_arena = arena.clone();
            unsafe {
                (l.c)(c_arena.as_mut_ptr(), c_arena.as_ptr());
                (l.rust)(r_arena.as_mut_ptr(), r_arena.as_ptr());
            }
            assert_eq!(
                c_arena.iter().map(|f| f.to_bits()).collect::<Vec<_>>(),
                r_arena.iter().map(|f| f.to_bits()).collect::<Vec<_>>(),
                "err18 alias/early mismatch arena={arena:?}"
            );
            assert_eq!(
                c_arena.iter().map(|f| f.to_bits()).collect::<Vec<_>>(),
                vec![v.to_bits(); 3],
                "err18 expected all three slots = v"
            );
        }
    }
}

#[test]
fn err19_offset_overlap() {
    let mut rng = Rng::new(SEED ^ 0x113);
    for _ in 0..10_000 {
        let arena: Vec<f32> = (0..6).map(|_| rng.any_f32()).collect();
        for (d, s) in [(1, 0), (0, 1), (2, 0), (0, 2), (3, 0), (0, 3), (2, 1), (1, 2)] {
            check_aliased("err19", &arena, d, s);
        }
        let canon = vec![
            rng.range(-720.0, 720.0),
            rng.range(f32::MIN_POSITIVE, 1.0),
            rng.range(-2.0, 2.0),
            rng.range(-720.0, 720.0),
            rng.range(f32::MIN_POSITIVE, 1.0),
            rng.range(-2.0, 2.0),
        ];
        for (d, s) in [(1, 0), (0, 1), (2, 0), (0, 2), (2, 1), (1, 2)] {
            check_aliased("err19/canon", &canon, d, s);
        }
    }
}

// --- Row 20: unaligned pointers ----------------------------------------
#[test]
fn err20_unaligned_pointers() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x114);
    for _ in 0..5_000 {
        let vals = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        let canon = [
            rng.range(-720.0, 720.0),
            rng.range(f32::MIN_POSITIVE, 1.0),
            rng.range(-2.0, 2.0),
        ];
        for set in [vals, canon] {
            for src_off in 0..8usize {
                for dst_off in 0..8usize {
                    let mut src = [0u8; 48];
                    for (i, v) in set.iter().enumerate() {
                        src[src_off + i * 4..src_off + i * 4 + 4]
                            .copy_from_slice(&v.to_le_bytes());
                    }
                    let mut cd = [0x5Au8; 48];
                    let mut rd = [0x5Au8; 48];
                    unsafe {
                        (l.c)(
                            cd.as_mut_ptr().add(dst_off) as *mut f32,
                            src.as_ptr().add(src_off) as *const f32,
                        );
                        (l.rust)(
                            rd.as_mut_ptr().add(dst_off) as *mut f32,
                            src.as_ptr().add(src_off) as *const f32,
                        );
                    }
                    assert_eq!(
                        cd, rd,
                        "err20 mismatch src_off={src_off} dst_off={dst_off} set={set:?}"
                    );
                }
            }
        }
    }
}

// --- Row 21: NULL pointers -> both must fault identically ---------------
//
// Dereferencing NULL is UB in C and there is no NULL check anywhere in
// `lib.c`, so the correct Rust behaviour is to fault too (a Rust-side NULL
// check that silently returned would be a DIVERGENCE). Asserted
// out-of-process: this test re-executes itself so the SIGSEGV does not take
// down the harness.

const NULL_PROBE_ENV: &str = "HARNESS_NULL_PROBE";

#[test]
#[ignore = "child-process helper for err21; not a standalone test"]
fn err21_null_probe_child() {
    let which = std::env::var(NULL_PROBE_ENV).expect("probe env must be set");
    let l = libs();
    let src = [30.0f32, 0.5, 1.0];
    let mut dst = [0.0f32; 3];
    unsafe {
        match which.as_str() {
            "c_dest" => (l.c)(std::ptr::null_mut(), src.as_ptr()),
            "rust_dest" => (l.rust)(std::ptr::null_mut(), src.as_ptr()),
            "c_src" => (l.c)(dst.as_mut_ptr(), std::ptr::null()),
            "rust_src" => (l.rust)(dst.as_mut_ptr(), std::ptr::null()),
            "c_both" => (l.c)(std::ptr::null_mut(), std::ptr::null()),
            "rust_both" => (l.rust)(std::ptr::null_mut(), std::ptr::null()),
            other => panic!("unknown probe {other}"),
        }
    }
    // If we get here, no fault happened.
    println!("NOFAULT");
    std::process::exit(7);
}

/// Run the child probe and classify the outcome.
fn run_null_probe(which: &str) -> String {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--ignored", "--exact", "err21_null_probe_child"])
        .env(NULL_PROBE_ENV, which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn null probe child");
    if let Some(sig) = out.status.signal() {
        format!("signal:{sig}")
    } else {
        let code = out.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("NOFAULT") {
            "nofault".to_string()
        } else {
            format!("exit:{code}")
        }
    }
}

#[test]
fn err21_null_pointers_fault_identically() {
    for kind in ["dest", "src", "both"] {
        let c = run_null_probe(&format!("c_{kind}"));
        let r = run_null_probe(&format!("rust_{kind}"));
        assert_eq!(
            c, r,
            "err21 NULL {kind}: C outcome {c} but Rust outcome {r} — the Rust \
             translation must not add a NULL check the C does not have, and \
             must fault the same way"
        );
        assert_eq!(
            c, "signal:11",
            "err21 NULL {kind}: expected SIGSEGV from C, got {c}"
        );
    }
}

// --- Row 22: no length parameter; exactly 3 reads and 3 writes ----------
#[test]
fn err22_exactly_three_elements_touched() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x116);
    for _ in 0..20_000 {
        // 4th src element differs between the two runs; output must not change.
        let base = [
            rng.range(-720.0, 720.0),
            rng.range(f32::MIN_POSITIVE, 1.0),
            rng.range(-2.0, 2.0),
        ];
        let mut prev: Option<[u32; 8]> = None;
        for &tail in &[0.0f32, f32::NAN, 1e30, -1e30, f32::INFINITY] {
            let src = [base[0], base[1], base[2], tail, tail];
            let mut c_buf = [f32::from_bits(CANARY); 8];
            let mut r_buf = [f32::from_bits(CANARY); 8];
            unsafe {
                (l.c)(c_buf.as_mut_ptr().add(2), src.as_ptr());
                (l.rust)(r_buf.as_mut_ptr().add(2), src.as_ptr());
            }
            let cb = c_buf.map(f32::to_bits);
            let rb = r_buf.map(f32::to_bits);
            assert_eq!(cb, rb, "err22 C/Rust mismatch tail={}", fmt(tail));
            // canaries intact => exactly 3 slots written
            for i in [0usize, 1, 5, 6, 7] {
                assert_eq!(cb[i], CANARY, "err22 C wrote outside window at {i}");
                assert_eq!(rb[i], CANARY, "err22 Rust wrote outside window at {i}");
            }
            // output independent of src[3..] => exactly 3 slots read
            if let Some(p) = prev {
                assert_eq!(p, cb, "err22 output depended on src[3] (tail={})", fmt(tail));
            }
            prev = Some(cb);
        }
    }
    // Also with the early-return path.
    for _ in 0..5_000 {
        let v = rng.any_f32();
        let src = [rng.any_f32(), 0.0, v, f32::NAN, f32::NAN];
        let mut c_buf = [f32::from_bits(CANARY); 8];
        let mut r_buf = [f32::from_bits(CANARY); 8];
        unsafe {
            (l.c)(c_buf.as_mut_ptr().add(2), src.as_ptr());
            (l.rust)(r_buf.as_mut_ptr().add(2), src.as_ptr());
        }
        assert_eq!(c_buf.map(f32::to_bits), r_buf.map(f32::to_bits));
        for i in [0usize, 1, 5, 6, 7] {
            assert_eq!(c_buf[i].to_bits(), CANARY);
            assert_eq!(r_buf[i].to_bits(), CANARY);
        }
    }
}

// --- Extra: "out-of-range enum value" analogue --------------------------
//
// This API takes no enum parameter, but `switch (i)` in `lib.c` is its
// discriminant: 5 named arms (0..4) plus `default`. Every `i` with no named
// arm is exercised here, including the two INT_MIN provenances (NaN and
// overflow), which is the class of value that has no valid variant.
#[test]
fn err_extra_switch_discriminant_full_range() {
    let mut rng = Rng::new(SEED ^ 0x117);
    // sweep i over a wide signed range by constructing h = 60*i + frac
    for i in -600i32..=600 {
        for frac in [0.0f32, 0.5, 30.0, 59.9] {
            let h = 60.0f32 * i as f32 + frac;
            check("err_extra/sweep", [h, 0.5, 1.0]);
            check("err_extra/sweep2", [h, rng.range(f32::MIN_POSITIVE, 2.0), rng.range(-2.0, 2.0)]);
        }
    }
    // both INT_MIN provenances
    for &h in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e30, -1e30, f32::MAX, f32::MIN] {
        assert_eq!(c_switch_index(h), INT_INDEFINITE);
        for &s in SPECIAL {
            for &v in SPECIAL {
                check("err_extra/indefinite", [h, s, v]);
            }
        }
    }
}
