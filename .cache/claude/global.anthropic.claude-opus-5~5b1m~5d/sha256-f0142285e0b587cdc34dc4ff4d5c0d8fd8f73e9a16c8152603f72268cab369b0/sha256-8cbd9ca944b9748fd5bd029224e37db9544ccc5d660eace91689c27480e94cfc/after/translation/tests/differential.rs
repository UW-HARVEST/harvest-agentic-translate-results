//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! * `phase_b_*` — one test per row of `CONFIGS.md` (valid paths).
//! * `phase_c_*` — one test per row of `ERRORS.md` (error / boundary paths).
//! * `phase_d_*` — symbol parity.

mod common;

use common::{
    assert_same, c_ldexp_q2, c_so_path, representative_exps, representative_ys, rust_ldexp_q2,
    rust_so_path, Rng,
};

const SEED: u64 = 0x5eed_1dea;

/// Number of randomized inputs per configuration row.
const N: usize = 2000;

// ===========================================================================
// Phase B — valid-path rows from CONFIGS.md
// ===========================================================================

/// Rows C01..C04: pass-through clamp, single iteration, one test per `e & 3`
/// phase, randomized `y`.
fn phase_b_single_iteration_phase(phase: i32, row: &str) {
    let mut rng = Rng::new(SEED ^ (phase as u64) << 8);
    for i in 0..N {
        let mut e = phase;
        while e >= 120 {
            e -= 4;
        }
        // walk all `exp_q2 = phase + 4k` values below the clamp
        let steps = (120 - phase + 3) / 4;
        let exp_q2 = phase + 4 * ((i as i32) % steps.max(1));
        assert!(exp_q2 < 120 && exp_q2 & 3 == phase);
        assert_same(row, rng.finite_normal_f32(), exp_q2);
        assert_same(row, rng.any_f32(), exp_q2);
    }
}

#[test]
fn phase_b_c01_phase0_single_iteration() {
    phase_b_single_iteration_phase(0, "C01");
}

#[test]
fn phase_b_c02_phase1_single_iteration() {
    phase_b_single_iteration_phase(1, "C02");
}

#[test]
fn phase_b_c03_phase2_single_iteration() {
    phase_b_single_iteration_phase(2, "C03");
}

#[test]
fn phase_b_c04_phase3_single_iteration() {
    phase_b_single_iteration_phase(3, "C04");
}

/// Row C05: `exp_q2 == 120` — the clamp boundary, `e >> 2 == 30`, scale == 1.
#[test]
fn phase_b_c05_clamp_boundary_scale_one() {
    let mut rng = Rng::new(SEED ^ 0x05);
    for _ in 0..N {
        assert_same("C05", rng.finite_normal_f32(), 120);
        assert_same("C05", rng.any_f32(), 120);
    }
    for y in representative_ys() {
        assert_same("C05", y, 120);
    }
}

/// Row C06: `exp_q2` in `121..=240` — exactly two loop iterations, covering all
/// four residual phases on the second iteration.
#[test]
fn phase_b_c06_two_iterations() {
    let mut rng = Rng::new(SEED ^ 0x06);
    for exp_q2 in 121..=240 {
        assert_same("C06", rng.finite_normal_f32(), exp_q2);
        assert_same("C06", rng.any_f32(), exp_q2);
        assert_same("C06", 1.0, exp_q2);
    }
    for _ in 0..N {
        let exp_q2 = rng.range_i32(121, 240);
        assert_same("C06", rng.finite_normal_f32(), exp_q2);
    }
}

/// Row C07: `exp_q2` in `241..=2000` — more than two iterations.
#[test]
fn phase_b_c07_many_iterations() {
    let mut rng = Rng::new(SEED ^ 0x07);
    for _ in 0..N {
        let exp_q2 = rng.range_i32(241, 2000);
        assert_same("C07", rng.finite_normal_f32(), exp_q2);
        assert_same("C07", rng.any_f32(), exp_q2);
    }
    for exp_q2 in 241..=600 {
        assert_same("C07", 1.0, exp_q2);
        assert_same("C07", -3.25e17, exp_q2);
    }
}

/// Row C08: very large trip counts.
#[test]
fn phase_b_c08_large_trip_counts() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for exp_q2 in [100_000i32, 1_000_000, 999_999, 100_001] {
        for _ in 0..64 {
            assert_same("C08", rng.finite_normal_f32(), exp_q2);
            assert_same("C08", rng.any_f32(), exp_q2);
        }
        for y in representative_ys() {
            assert_same("C08", y, exp_q2);
        }
    }
}

/// Row C09: negative `exp_q2` in `-1..=-256` — the negative-shift-count path,
/// all four `e & 3` phases via the two's-complement low bits.
#[test]
fn phase_b_c09_negative_small() {
    let mut rng = Rng::new(SEED ^ 0x09);
    for exp_q2 in -256..=-1 {
        assert_same("C09", rng.finite_normal_f32(), exp_q2);
        assert_same("C09", rng.any_f32(), exp_q2);
        assert_same("C09", 1.0, exp_q2);
        assert_same("C09", -1.0, exp_q2);
    }
}

/// Row C10: negative `exp_q2` spanning the whole negative `int` range, so the
/// masked shift count `(e >> 2) & 31` takes many different values.
#[test]
fn phase_b_c10_negative_full_range() {
    let mut rng = Rng::new(SEED ^ 0x0a);
    for _ in 0..N * 4 {
        let exp_q2 = rng.range_i32(i32::MIN, -1);
        assert_same("C10", rng.finite_normal_f32(), exp_q2);
        assert_same("C10", rng.any_f32(), exp_q2);
    }
    // Deliberately hit every possible masked shift count 0..=31 on the UB path:
    // choose exp_q2 = -(4 * k) so that e >> 2 == -k, masked count == (-k) & 31.
    for k in 1..=64i32 {
        let exp_q2 = -4 * k;
        for y in representative_ys() {
            assert_same("C10", y, exp_q2);
        }
    }
    // ...and the same with each of the four low-bit phases.
    for k in 1..=64i32 {
        for phase in 0..4i32 {
            let exp_q2 = -4 * k + phase;
            assert_same("C10", 1.0, exp_q2);
            assert_same("C10", 12345.678, exp_q2);
        }
    }
}

/// Row C11: non-finite `y` against a representative `exp_q2` sweep.
#[test]
fn phase_b_c11_non_finite_y() {
    let ys = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFC0_0001),
        f32::from_bits(0x7FA0_0000),
        f32::from_bits(0xFFA0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
    ];
    for y in ys {
        for exp_q2 in representative_exps() {
            assert_same("C11", y, exp_q2);
        }
        for exp_q2 in -300..=300 {
            assert_same("C11", y, exp_q2);
        }
    }
}

/// Row C12: zero and subnormal `y`.
#[test]
fn phase_b_c12_zero_and_subnormal_y() {
    let mut ys = vec![
        0.0f32,
        -0.0f32,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0040_0000),
        f32::from_bits(0x8040_0000),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
    ];
    let mut rng = Rng::new(SEED ^ 0x0c);
    for _ in 0..64 {
        // random subnormals
        ys.push(f32::from_bits(rng.next_u32() & 0x807F_FFFF));
    }
    for y in ys {
        for exp_q2 in representative_exps() {
            assert_same("C12", y, exp_q2);
        }
        for exp_q2 in -200..=260 {
            assert_same("C12", y, exp_q2);
        }
    }
}

/// Row C13: overflow / underflow saturation of the `float` product.
#[test]
fn phase_b_c13_saturation() {
    let ys = [
        f32::MAX,
        f32::MIN,
        -f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e38,
        -1e38,
        1e-38,
        -1e-38,
        3.4e38,
        -3.4e38,
    ];
    for y in ys {
        for exp_q2 in representative_exps() {
            assert_same("C13", y, exp_q2);
        }
        for exp_q2 in -400..=400 {
            assert_same("C13", y, exp_q2);
        }
    }
}

/// Row C14: unconstrained fuzz over both parameters.
#[test]
fn phase_b_c14_unconstrained_fuzz() {
    let mut rng = Rng::new(SEED ^ 0x0e);
    for _ in 0..N * 10 {
        let y = rng.any_f32();
        // Bound the positive side so the trip count stays reasonable; the
        // negative side is unbounded (it always terminates in one iteration).
        let exp_q2 = if rng.next_u32() & 1 == 0 {
            rng.range_i32(i32::MIN, 0)
        } else {
            rng.range_i32(0, 100_000)
        };
        assert_same("C14", y, exp_q2);
    }
}

/// Row C15: exhaustive `exp_q2` sweep `-4096..=4096` against every
/// representative `y`.
#[test]
fn phase_b_c15_exhaustive_exp_sweep() {
    let ys = representative_ys();
    for exp_q2 in -4096..=4096 {
        for &y in &ys {
            assert_same("C15", y, exp_q2);
        }
    }
}

/// Row C16: `int` extremes for `exp_q2`, including the maximal trip count.
#[test]
fn phase_b_c16_int_extremes() {
    let ys = representative_ys();
    let mut exps = vec![i32::MIN, i32::MAX];
    for k in 1..=8 {
        exps.push(i32::MIN + k);
        exps.push(i32::MAX - k);
    }
    for exp_q2 in exps {
        for &y in &ys {
            assert_same("C16", y, exp_q2);
        }
    }
}

// ===========================================================================
// Phase C — error / boundary rows from ERRORS.md
// ===========================================================================

/// E1: `exp_q2 == 120` takes the clamp branch (ternary false).
#[test]
fn phase_c_e1_clamp_taken_at_120() {
    let mut rng = Rng::new(SEED ^ 0xe1);
    for _ in 0..N {
        assert_same("E1", rng.any_f32(), 120);
    }
    for y in representative_ys() {
        assert_same("E1", y, 120);
    }
}

/// E2: `exp_q2 == 119`, one step below the clamp (ternary true).
#[test]
fn phase_c_e2_one_below_clamp() {
    let mut rng = Rng::new(SEED ^ 0xe2);
    for _ in 0..N {
        assert_same("E2", rng.any_f32(), 119);
    }
    for y in representative_ys() {
        assert_same("E2", y, 119);
        assert_same("E2", y, 118);
    }
}

/// E3: `exp_q2 == 121`, one step past the clamp — forces a second iteration.
#[test]
fn phase_c_e3_one_past_clamp() {
    let mut rng = Rng::new(SEED ^ 0xe3);
    for _ in 0..N {
        assert_same("E3", rng.any_f32(), 121);
    }
    for y in representative_ys() {
        assert_same("E3", y, 121);
        assert_same("E3", y, 122);
        assert_same("E3", y, 123);
        assert_same("E3", y, 124);
    }
}

/// E4: negative `exp_q2` — the negative shift count (UB in C, `sar %cl` in the
/// compiled object). Must NOT panic and must match bit-for-bit.
#[test]
fn phase_c_e4_negative_shift_count() {
    let mut rng = Rng::new(SEED ^ 0xe4);
    for exp_q2 in -1024..=-1 {
        assert_same("E4", rng.any_f32(), exp_q2);
    }
    for y in representative_ys() {
        for exp_q2 in [-1i32, -2, -3, -4, -5, -100, -124, -128, -1 << 20, i32::MIN] {
            assert_same("E4", y, exp_q2);
        }
    }
    // Sanity pin on the masked-shift semantics, so this row provably exercises
    // the UB path rather than silently testing nothing. With count masking
    // (`sar %cl`, count & 31):
    //   exp_q2 = -1..=-4  => e >> 2 == -1, masked count 31 => scale 0  => 0.0
    //   exp_q2 = -128     => e >> 2 == -32, masked count 0  => scale 2^30
    //                        and e & 3 == 0 => frac == 2^-30 => result 1.0
    // i.e. the shift count WRAPS; it does not saturate. Both must agree.
    let c = c_ldexp_q2();
    let r = rust_ldexp_q2();
    for exp_q2 in [-1i32, -2, -3, -4] {
        let cv = unsafe { c(1.0, exp_q2) };
        let rv = unsafe { r(1.0, exp_q2) };
        assert_eq!(cv.to_bits(), rv.to_bits());
        assert_eq!(cv.to_bits(), 0.0f32.to_bits(), "masked count 31 => zero scale");
    }
    let cv = unsafe { c(1.0, -128) };
    let rv = unsafe { r(1.0, -128) };
    assert_eq!(cv.to_bits(), rv.to_bits());
    assert_eq!(
        cv.to_bits(),
        1.0f32.to_bits(),
        "masked count wraps to 0 at exp_q2 == -128 => scale 2^30 => 1.0"
    );
}

/// E5: `e & 3` on a negative `e` must use two's-complement low bits and stay
/// inside `g_expfrac[4]`. Exercise every negative residue class heavily.
#[test]
fn phase_c_e5_negative_index_masking() {
    let mut rng = Rng::new(SEED ^ 0xe5);
    for phase in 0..4i32 {
        for k in 1..=512i32 {
            let exp_q2 = -4 * k + phase;
            assert!(exp_q2 & 3 == phase);
            assert_same("E5", rng.any_f32(), exp_q2);
            assert_same("E5", 1.0, exp_q2);
        }
    }
}

/// E6: `exp_q2 == INT_MIN` — extreme of the UB path, and the value where
/// `exp_q2 -= e` would overflow if the clamp misbehaved.
#[test]
fn phase_c_e6_int_min() {
    let mut rng = Rng::new(SEED ^ 0xe6);
    for _ in 0..N {
        assert_same("E6", rng.any_f32(), i32::MIN);
    }
    for y in representative_ys() {
        assert_same("E6", y, i32::MIN);
        assert_same("E6", y, i32::MIN + 1);
        assert_same("E6", y, i32::MIN + 2);
        assert_same("E6", y, i32::MIN + 3);
    }
}

/// E7: `exp_q2 == INT_MAX` — maximal trip count (~17.9 M iterations).
#[test]
fn phase_c_e7_int_max_max_trip_count() {
    for y in [1.0f32, -1.0, f32::MAX, f32::INFINITY, f32::NAN, 0.0, -0.0] {
        assert_same("E7", y, i32::MAX);
    }
    for k in 1..=4 {
        assert_same("E7", 1.0, i32::MAX - k);
    }
}

/// E8: `exp_q2 == 0` still runs the body once (do/while), so the result is
/// `y * g_expfrac[0] * (1 << 30)`, which is NOT exactly `y`.
#[test]
fn phase_c_e8_zero_runs_body_once() {
    let mut rng = Rng::new(SEED ^ 0xe8);
    for _ in 0..N * 5 {
        assert_same("E8", rng.any_f32(), 0);
    }
    for y in representative_ys() {
        assert_same("E8", y, 0);
    }
    // Sanity pin: the body DOES run once at exp_q2 == 0, and the scale it
    // applies is `g_expfrac[0] * (1 << 30)`, which the f32 literal
    // 9.31322575e-10f makes EXACTLY 2^-30 * 2^30 == 1.0. So the observable
    // result happens to be the identity — pin that on both sides so a
    // translation that changed the literal's rounding would be caught.
    let c = c_ldexp_q2();
    let r = rust_ldexp_q2();
    for y in representative_ys() {
        let cv = unsafe { c(y, 0) };
        let rv = unsafe { r(y, 0) };
        assert_eq!(cv.to_bits(), rv.to_bits());
        if y.is_finite() {
            assert_eq!(
                cv.to_bits(),
                y.to_bits(),
                "g_expfrac[0] * (1 << 30) must be exactly 1.0"
            );
        }
    }
}

/// E9: non-finite and degenerate `y` combined with a zero scale
/// (`inf * 0 == NaN`), including signalling-NaN quieting and NaN payloads.
#[test]
fn phase_c_e9_non_finite_and_nan_payloads() {
    let mut ys: Vec<f32> = vec![
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
    ];
    // A spread of NaN payloads, both signs, both quiet and signalling.
    for payload in [
        0x0000_0001u32,
        0x0000_0002,
        0x0020_0000,
        0x003F_FFFF,
        0x0040_0000,
        0x0040_0001,
        0x007F_FFFF,
    ] {
        ys.push(f32::from_bits(0x7F80_0000 | payload));
        ys.push(f32::from_bits(0xFF80_0000 | payload));
    }
    for y in ys {
        for exp_q2 in representative_exps() {
            assert_same("E9", y, exp_q2);
        }
        for exp_q2 in -160..=260 {
            assert_same("E9", y, exp_q2);
        }
    }
}

/// E10: overflow/underflow of the product itself.
#[test]
fn phase_c_e10_float_overflow_underflow() {
    let ys = [
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x7F7F_FFFF),
        f32::from_bits(0xFF7F_FFFF),
    ];
    for y in ys {
        for exp_q2 in -500..=500 {
            assert_same("E10", y, exp_q2);
        }
        for exp_q2 in representative_exps() {
            assert_same("E10", y, exp_q2);
        }
    }
}

/// Generic C-API boundaries that every FFI surface has, even though this
/// function takes no pointers, no lengths and no enums: values one step past
/// every documented/implicit range, and "out-of-range enum"-analogous ints.
#[test]
fn phase_c_generic_ffi_boundaries() {
    // One step past every interesting threshold in the C source.
    let mut exps: Vec<i32> = Vec::new();
    for centre in [
        0i32, 1, 3, 4, 119, 120, 121, 124, 128, 239, 240, 241, 480, 481, -1, -4, -31, -32, -33,
        -124, -128, i32::MIN, i32::MIN + 4, i32::MAX, i32::MAX - 120,
    ] {
        for d in -2i32..=2 {
            exps.push(centre.saturating_add(d));
        }
    }
    exps.sort();
    exps.dedup();
    // INT_MAX-ish values have ~18 M-iteration loops; keep only a few.
    exps.retain(|&e| e < i32::MAX - 1_000_000 || e >= i32::MAX - 5);

    let ys = representative_ys();
    for exp_q2 in exps {
        if exp_q2 >= i32::MAX - 5 {
            // expensive: only a couple of y values
            assert_same("generic", 1.0, exp_q2);
            assert_same("generic", f32::NAN, exp_q2);
            continue;
        }
        for &y in &ys {
            assert_same("generic", y, exp_q2);
        }
    }
}

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

fn defined_symbols(so: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .filter(|s| {
            // Ignore the toolchain/runtime bookkeeping symbols that are not
            // part of either library's API surface.
            !s.starts_with("_ITM_")
                && !s.starts_with("__")
                && s != "_init"
                && s != "_fini"
                && s != "rust_eh_personality"
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn phase_d_symbol_parity() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
    assert!(
        c.contains(&"ldexp_q2".to_string()),
        "expected `ldexp_q2` in the C .so, got {c:?}"
    );
}

/// Both `.so`s must be loadable and expose the symbol through `dlsym`.
#[test]
fn phase_d_both_libraries_export_the_symbol() {
    let c = c_ldexp_q2();
    let r = rust_ldexp_q2();
    let cv = unsafe { c(1.0, 4) };
    let rv = unsafe { r(1.0, 4) };
    assert_eq!(cv.to_bits(), rv.to_bits());
}
