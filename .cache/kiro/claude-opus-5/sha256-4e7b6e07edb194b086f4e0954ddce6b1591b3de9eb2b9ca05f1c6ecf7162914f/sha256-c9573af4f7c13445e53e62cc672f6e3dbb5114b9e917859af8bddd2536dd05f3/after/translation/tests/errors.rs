//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `gaussian_kernel` returns `void` and has no error channel, so "same error"
//! means *the same observable rejection*: the same set of stores performed (or
//! not performed) and the same `f32` bit patterns, including which special
//! value (`+0.0`, `-0.0`, `NaN`, `±inf`) is produced and which bytes are left
//! untouched. Every assertion is on exact bits, never on "both failed somehow".
//!
//! Row E3 (unchecked null dereference) lives in `tests/null_deref.rs` because it
//! must be observed out-of-process.

mod common;

use common::{Rng, SEED, branches, buffer_len, expect_match, expect_match_fill, garbage_fill, pair, s2, touched_len};

fn seeded(row: u64) -> Rng {
    Rng::new(SEED ^ 0xE770_0000 ^ (row.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

// ---------------------------------------------------------------------------
// E1 — size <= -2: loop guard false on entry, zero stores
// ---------------------------------------------------------------------------
#[test]
fn e01_size_le_minus_two_performs_zero_stores() {
    let mut rng = seeded(1);
    for _ in 0..500 {
        let size = rng.range_i32(-100_000, -2);
        let radius = rng.any_f32();
        let len = buffer_len(size);
        let fill = garbage_fill(&mut rng, len.max(16));
        let out = expect_match_fill(size, radius, &fill);
        assert_eq!(bits(&out), bits(&fill), "size={size} must not store anything");
        let b = branches(size, radius);
        assert_eq!(b.iterations, 0, "size={size}: loop must not run");
        assert!(!b.normalised, "size={size}: normalisation must be skipped");
    }
}

// ---------------------------------------------------------------------------
// E2 — dest == NULL with size <= -2 must be survivable and identical
// ---------------------------------------------------------------------------
#[test]
fn e02_null_dest_with_negative_size_is_survivable_in_both() {
    let p = pair();
    let mut rng = seeded(2);
    for _ in 0..500 {
        let size = rng.range_i32(-1_000_000, -2);
        let radius = rng.any_f32();
        // Neither implementation may dereference; a defensive early return in
        // Rust would be indistinguishable here, but an eager null *check* that
        // changed behaviour for size >= -1 is caught by E3/null_deref.rs.
        unsafe {
            (p.c.gaussian_kernel)(std::ptr::null_mut(), size, radius);
            (p.rs.gaussian_kernel)(std::ptr::null_mut(), size, radius);
        }
    }
    // Extremes too.
    for size in [i32::MIN, i32::MIN + 1, -2, -3, -1_000_000_000] {
        unsafe {
            (p.c.gaussian_kernel)(std::ptr::null_mut(), size, 1.0);
            (p.rs.gaussian_kernel)(std::ptr::null_mut(), size, 1.0);
            (p.c.gaussian_kernel)(std::ptr::null_mut(), size, f32::NAN);
            (p.rs.gaussian_kernel)(std::ptr::null_mut(), size, f32::NAN);
        }
    }
}

// ---------------------------------------------------------------------------
// E4 — size == 0 still stores one element, un-normalised
// ---------------------------------------------------------------------------
#[test]
fn e04_size_zero_stores_one_unnormalised_element() {
    let mut rng = seeded(4);
    let expected_centre = 1.0f32 - s2();
    for _ in 0..500 {
        let radius = rng.any_f32();
        let len = buffer_len(0);
        let fill = garbage_fill(&mut rng, len);
        let out = expect_match_fill(0, radius, &fill);
        // Exactly one element written; the rest untouched.
        assert_eq!(touched_len(0), 1);
        for i in 1..len {
            assert_eq!(out[i].to_bits(), fill[i].to_bits(), "size=0 wrote past dest[0] at {i}");
        }
    }
    // With a finite non-zero radius the single tap is 1.0 - s2, NOT 1.0,
    // because the normalisation loop `for (r=0; r<0)` never runs.
    for radius in [1.0f32, 3.0, 1e3, -2.5, f32::INFINITY] {
        let out = expect_match(0, radius);
        assert_eq!(
            out[0].to_bits(),
            expected_centre.to_bits(),
            "size=0, radius={radius:e}: dest[0] must stay un-normalised at 1.0-s2"
        );
    }
    assert!(branches(0, 1.0).normalised, "sum>0 is true for size=0, but the loop body is empty");
}

// ---------------------------------------------------------------------------
// E5 — size == -1: truncating (not flooring) division => hsize == 0
// ---------------------------------------------------------------------------
#[test]
fn e05_size_minus_one_truncates_toward_zero() {
    assert_eq!(-1i32 / 2, 0, "C/Rust integer division must truncate toward zero");
    let mut rng = seeded(5);
    let expected_centre = 1.0f32 - s2();
    for _ in 0..500 {
        let radius = rng.any_f32();
        let len = buffer_len(-1).max(8);
        let fill = garbage_fill(&mut rng, len);
        let out = expect_match_fill(-1, radius, &fill);
        for i in 1..len {
            assert_eq!(out[i].to_bits(), fill[i].to_bits(), "size=-1 wrote past dest[0] at {i}");
        }
    }
    for radius in [1.0f32, 7.5, -0.25] {
        let out = expect_match(-1, radius);
        assert_eq!(out[0].to_bits(), expected_centre.to_bits());
    }
    // A floor-division bug would give hsize == -1 and zero stores instead.
    assert_eq!(branches(-1, 1.0).stores, 1);
}

// ---------------------------------------------------------------------------
// E6 — even size: one-past-the-end store, left un-normalised
// ---------------------------------------------------------------------------
#[test]
fn e06_even_size_overruns_by_one_element() {
    let mut rng = seeded(6);
    for _ in 0..500 {
        let size = rng.range_i32(1, 64) * 2; // even, 2..=128
        let radius = rng.log_uniform(1e-2, 1e2);
        let len = buffer_len(size);
        let fill = garbage_fill(&mut rng, len);
        let out = expect_match_fill(size, radius, &fill);
        assert_eq!(touched_len(size), size as usize + 1, "even size must touch size+1 elements");
        // The overrun element WAS written (it differs from a guard NaN pattern
        // that the kernel cannot produce) ...
        assert_ne!(
            out[size as usize].to_bits(),
            fill[size as usize].to_bits(),
            "size={size}: dest[size] should have been overwritten"
        );
        // ... and nothing beyond it was.
        for i in (size as usize + 1)..len {
            assert_eq!(out[i].to_bits(), fill[i].to_bits(), "size={size} wrote at {i}");
        }
    }
    // Concretely: the overrun element is un-normalised while dest[0..size) is
    // scaled, so for a wide kernel dest[size] > dest[size-1].
    let out = expect_match(8, f32::INFINITY);
    assert_eq!(out[8].to_bits(), (1.0f32 - s2()).to_bits(), "dest[size] is un-normalised");
    assert_ne!(out[7].to_bits(), out[8].to_bits(), "dest[size-1] is normalised, dest[size] is not");
}

// ---------------------------------------------------------------------------
// E7 / E8 — radius == ±0.0: division by zero, all taps clamp, no normalisation
// ---------------------------------------------------------------------------
fn zero_radius_row(radius: f32) {
    let mut rng = seeded(radius.to_bits() as u64);
    for _ in 0..300 {
        let size = rng.range_i32(-4, 129);
        let len = buffer_len(size);
        let fill = garbage_fill(&mut rng, len.max(8));
        let out = expect_match_fill(size, radius, &fill);
        let b = branches(size, radius);
        assert!(!b.normalised, "radius={radius:e}, size={size}: sum must be 0");
        assert_eq!(b.kept_positive, 0, "radius={radius:e}: no tap may survive the clamp");
        for i in 0..touched_len(size) {
            assert_eq!(
                out[i].to_bits(),
                0u32,
                "radius={radius:e}, size={size}: tap {i} must be exactly +0.0 (not -0.0, not NaN)"
            );
        }
    }
}

#[test]
fn e07_radius_positive_zero_yields_all_positive_zero() {
    assert!((1.6f32 / 0.0f32).is_infinite() && (1.6f32 / 0.0f32) > 0.0);
    // r == 0 gives x = 0 * inf = NaN, and `NaN > 0` is false, so the clamp
    // stores the literal 0 rather than propagating the NaN.
    let rs = 1.6f32 / 0.0f32;
    let x = 0.0f32 * rs;
    assert!(x.is_nan(), "0 * inf must be NaN");
    assert!(!(((1.0f32 / (x * x).exp()) - s2()) > 0.0), "NaN > 0 must be false");
    zero_radius_row(0.0);
}

#[test]
fn e08_radius_negative_zero_yields_all_positive_zero() {
    assert!((1.6f32 / -0.0f32).is_infinite() && (1.6f32 / -0.0f32) < 0.0);
    zero_radius_row(-0.0);
}

// ---------------------------------------------------------------------------
// E9 — radius == NaN
// ---------------------------------------------------------------------------
#[test]
fn e09_radius_nan_clamps_every_tap() {
    let mut rng = seeded(9);
    let nans = [
        f32::NAN.to_bits(),
        0x7FC0_0000,
        0xFFC0_0000,
        0x7F80_0001, // signalling
        0xFF80_0001,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
    ];
    for &nb in &nans {
        let radius = f32::from_bits(nb);
        assert!(radius.is_nan());
        for _ in 0..80 {
            let size = rng.range_i32(-4, 129);
            let len = buffer_len(size);
            let fill = garbage_fill(&mut rng, len.max(8));
            let out = expect_match_fill(size, radius, &fill);
            let b = branches(size, radius);
            assert_eq!(b.clamped_from_nan, b.stores, "NaN radius: every v must be NaN");
            assert!(!b.normalised);
            for i in 0..touched_len(size) {
                assert_eq!(
                    out[i].to_bits(),
                    0u32,
                    "NaN radius 0x{nb:08X}, size={size}: tap {i} must be +0.0, not a NaN"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E10 — radius == ±inf: rs == ±0.0, flat kernel, normalised by 1/(2*hsize+1)
// ---------------------------------------------------------------------------
#[test]
fn e10_radius_infinite_gives_flat_kernel_normalised_by_2hsize_plus_1() {
    let mut rng = seeded(10);
    for radius in [f32::INFINITY, f32::NEG_INFINITY] {
        for _ in 0..200 {
            let size = rng.range_i32(-4, 129);
            let len = buffer_len(size);
            let fill = garbage_fill(&mut rng, len.max(8));
            expect_match_fill(size, radius, &fill);
        }
        // For even size the divisor is size+1, not size.
        for size in [1i32, 2, 3, 4, 7, 8, 16, 17] {
            let out = expect_match(size, radius);
            let n = 2 * (size / 2) + 1;
            let b = branches(size, radius);
            assert_eq!(b.kept_positive, n as u64, "size={size}: flat kernel keeps all {n} taps");
            assert!(b.normalised);
            // All normalised taps are identical.
            let first = out[0].to_bits();
            for i in 0..(size.max(0) as usize) {
                assert_eq!(out[i].to_bits(), first, "size={size}: flat kernel tap {i} differs");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — subnormal radius: sigma/radius overflows to +inf silently
// ---------------------------------------------------------------------------
#[test]
fn e11_subnormal_radius_overflows_rs_to_infinity() {
    // Note: `sigma / radius` only overflows for *small* subnormals. The largest
    // subnormal (~1.175e-38) gives rs ~= 1.36e38, which is still finite, so
    // that case falls into the Dirac-spike regime instead. Both sub-cases are
    // asserted, and at least one true overflow must occur.
    let edges = [1u32, 2, 0x0000_FFFF, 0x007F_FFFF, 0x8000_0001, 0x807F_FFFF];
    let mut rng = seeded(11);
    let mut saw_overflow = false;
    let mut saw_finite_rs = false;
    for &b in &edges {
        let radius = f32::from_bits(b);
        assert!(radius != 0.0 && radius.is_subnormal(), "0x{b:08X} must be subnormal");
        let rs = 1.6f32 / radius;
        if rs.is_infinite() {
            saw_overflow = true;
        } else {
            saw_finite_rs = true;
        }
        for _ in 0..40 {
            let size = rng.range_i32(-4, 129);
            let len = buffer_len(size);
            let fill = garbage_fill(&mut rng, len.max(8));
            let out = expect_match_fill(size, radius, &fill);
            let br = branches(size, radius);
            if rs.is_infinite() {
                // Every tap clamps: r == 0 via NaN, r != 0 via v == -s2 < 0.
                assert!(!br.normalised, "0x{b:08X}, size={size}: sum must be 0");
                for i in 0..touched_len(size) {
                    assert_eq!(out[i].to_bits(), 0u32, "subnormal radius 0x{b:08X}: tap {i}");
                }
            } else if size >= -1 {
                // rs finite but astronomically large: only the r == 0 tap lives.
                assert_eq!(br.kept_positive, 1, "0x{b:08X}, size={size}: expected a spike");
            }
        }
    }
    assert!(saw_overflow, "E11 never produced an infinite rs from a subnormal radius");
    assert!(saw_finite_rs, "E11 never produced a finite rs from a subnormal radius");
    for _ in 0..400 {
        let size = rng.range_i32(-4, 129);
        let radius = rng.subnormal_f32();
        expect_match(size, radius);
    }
}

// ---------------------------------------------------------------------------
// E12 — Dirac-spike regime: only the centre tap survives, normalises to 1.0
// ---------------------------------------------------------------------------
#[test]
fn e12_dirac_spike_regime() {
    let mut rng = seeded(12);
    let mut saw_spike = false;
    for _ in 0..400 {
        let size = rng.range_i32(3, 129) | 1; // odd
        // |r|*1.6/radius >= 2.4 for all |r| >= 1  <=>  radius <= 2/3
        let radius = rng.log_uniform(1e-30, 0.6);
        let out = expect_match(size, radius);
        let b = branches(size, radius);
        if b.kept_positive == 1 {
            saw_spike = true;
            assert!(b.normalised);
            let c = (size / 2) as usize;
            assert_eq!(out[c].to_bits(), 1.0f32.to_bits(), "spike must normalise to exactly 1.0");
            for i in 0..(size as usize) {
                if i != c {
                    assert_eq!(out[i].to_bits(), 0u32, "non-spike tap {i} must be +0.0");
                }
            }
        }
    }
    assert!(saw_spike, "E12 never reached the single-surviving-tap regime");
}

// ---------------------------------------------------------------------------
// E13 — huge finite radius: rs underflows
// ---------------------------------------------------------------------------
#[test]
fn e13_huge_finite_radius_underflows_rs() {
    let mut rng = seeded(13);
    for radius in [f32::MAX, 1e38f32, 1e30, -f32::MAX, -1e38] {
        assert!(radius.is_finite());
        for size in [1i32, 2, 5, 8, 33, 64] {
            let out = expect_match(size, radius);
            let first = out[0].to_bits();
            for i in 0..(size.max(1) as usize) {
                assert_eq!(out[i].to_bits(), first, "radius={radius:e}, size={size}: not flat at {i}");
            }
        }
    }
    for _ in 0..400 {
        let size = rng.range_i32(-4, 129);
        let radius = rng.log_uniform(1e20, 3.4e38);
        expect_match(size, radius);
        expect_match(size, -radius);
    }
}

// ---------------------------------------------------------------------------
// E14 — v == 0.0 exactly takes the else arm of the strict `>` clamp
// ---------------------------------------------------------------------------
#[test]
fn e14_exactly_zero_v_takes_the_else_arm() {
    // |x| == 2.4 exactly  <=>  radius == |r| * (2/3)
    let mut exact = 0usize;
    for r in 1..=48i32 {
        let radius = (r as f32) * (2.0f32 / 3.0f32);
        let rs = 1.6f32 / radius;
        let x = (r as f32) * rs;
        let v = (1.0f32 / (x * x).exp()) - s2();
        let size = 2 * r + 1;
        let out = expect_match(size, radius);
        let idx = (size / 2 - r) as usize;
        if v == 0.0 {
            exact += 1;
            assert_eq!(
                out[idx].to_bits(),
                0u32,
                "r={r}: v is exactly 0, the strict `>` must store +0.0 at tap {idx}"
            );
        }
        // In no case may a clamped tap come out negative or as -0.0.
        assert_eq!(out[idx].to_bits() & 0x8000_0000, 0, "r={r}: tap {idx} sign bit set");
    }
    assert!(exact >= 8, "E14 only found {exact} exact-zero v cases; path under-exercised");
}

// ---------------------------------------------------------------------------
// E15 — negative radius is never rejected and gives an identical kernel
// ---------------------------------------------------------------------------
#[test]
fn e15_negative_radius_is_not_rejected() {
    let mut rng = seeded(15);
    for _ in 0..600 {
        let size = rng.range_i32(-4, 129);
        let mag = rng.log_uniform(1e-30, 1e30);
        let pos = expect_match(size, mag);
        let neg = expect_match(size, -mag);
        assert_eq!(
            bits(&pos),
            bits(&neg),
            "size={size}, |radius|={mag:e}: sign of radius must not change the result"
        );
    }
}

// ---------------------------------------------------------------------------
// E16 — size == INT_MIN: no overflow trap on -hsize, zero stores
// ---------------------------------------------------------------------------
#[test]
fn e16_size_int_min_does_not_overflow() {
    assert_eq!(i32::MIN / 2, -1_073_741_824);
    assert_eq!((i32::MIN / 2).wrapping_neg(), 1_073_741_824);
    let mut rng = seeded(16);
    for size in [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 3] {
        for _ in 0..64 {
            let radius = rng.any_f32();
            let fill = garbage_fill(&mut rng, 16);
            let out = expect_match_fill(size, radius, &fill);
            assert_eq!(bits(&out), bits(&fill), "size={size} must store nothing");
        }
        // Also safe with a null destination.
        let p = pair();
        unsafe {
            (p.c.gaussian_kernel)(std::ptr::null_mut(), size, 1.0);
            (p.rs.gaussian_kernel)(std::ptr::null_mut(), size, 1.0);
        }
    }
}

// ---------------------------------------------------------------------------
// E17 — size == 1 always normalises to exactly 1.0
// ---------------------------------------------------------------------------
#[test]
fn e17_size_one_normalises_to_exactly_one() {
    let mut rng = seeded(17);
    for _ in 0..800 {
        let mag = rng.log_uniform(1e-30, 1e30);
        let radius = if rng.next_u32() & 1 == 0 { mag } else { -mag };
        let out = expect_match(1, radius);
        assert_eq!(
            out[0].to_bits(),
            1.0f32.to_bits(),
            "size=1, radius={radius:e}: must normalise to exactly 1.0"
        );
    }
    // ...except in the degenerate classes where sum == 0 and no normalisation
    // happens at all.
    for radius in [0.0f32, -0.0, f32::NAN] {
        let out = expect_match(1, radius);
        assert_eq!(out[0].to_bits(), 0u32, "radius={radius:e}: size=1 tap must be +0.0");
    }
    for radius in [f32::INFINITY, f32::NEG_INFINITY] {
        let out = expect_match(1, radius);
        assert_eq!(out[0].to_bits(), 1.0f32.to_bits());
    }
}

// ---------------------------------------------------------------------------
// E18 — size == 2: smallest overrun
// ---------------------------------------------------------------------------
#[test]
fn e18_size_two_smallest_overrun() {
    assert_eq!(touched_len(2), 3, "size=2 must touch dest[0..3)");
    let mut rng = seeded(18);
    for _ in 0..600 {
        let radius = rng.any_f32();
        let fill = garbage_fill(&mut rng, buffer_len(2));
        let out = expect_match_fill(2, radius, &fill);
        for i in 3..out.len() {
            assert_eq!(out[i].to_bits(), fill[i].to_bits(), "size=2 wrote at {i}");
        }
    }
    // dest[0] and dest[1] normalised, dest[2] not.
    let out = expect_match(2, f32::INFINITY);
    assert_eq!(out[0].to_bits(), out[1].to_bits());
    assert_eq!(out[2].to_bits(), (1.0f32 - s2()).to_bits());
}

// ---------------------------------------------------------------------------
// E19 — "out-of-range enum" equivalent: every int is a legal `size`
// ---------------------------------------------------------------------------
#[test]
fn e19_every_int_size_is_accepted_identically() {
    // The FFI signature carries no enum, so the analogous "value with no valid
    // variant" input is a nonsensical `int` size. C accepts all of them; the
    // Rust must not panic, abort, or diverge on any.
    let mut rng = seeded(19);
    let sentinels: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -2_147_483_000,
        -1_000_000_000,
        -65_537,
        -4097,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
    ];
    for &size in &sentinels {
        for radius in [1.0f32, 0.0, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e-30, 1e30] {
            let fill = garbage_fill(&mut rng, buffer_len(size).max(16));
            expect_match_fill(size, radius, &fill);
        }
    }
    // Plus a randomized sweep over arbitrary int bit patterns, restricted to
    // the range whose buffer we can actually allocate; larger positives are
    // covered by the CONFIGS.md rows.
    for _ in 0..2000 {
        let size = rng.range_i32(-1_000_000, 512);
        let radius = rng.any_f32();
        let fill = garbage_fill(&mut rng, buffer_len(size).max(16));
        expect_match_fill(size, radius, &fill);
    }
}

// ---------------------------------------------------------------------------
// E20 — isum = 1/sum overflow
// ---------------------------------------------------------------------------
#[test]
fn e20_reciprocal_of_sum_overflow_is_unreachable_but_parity_holds() {
    // `sum` is a sum of clamped values, each either 0 or `1/expf(x*x) - s2`.
    // Because s2 ~= 3.15e-3, the smallest representable *positive* v is on the
    // order of one ulp at that magnitude (~2.3e-10), so a positive `sum` can
    // never be small enough for `1.0f/sum` to overflow to +inf. Rather than
    // fake this row, search hard for the smallest positive sum reachable and
    // assert (a) C/Rust parity throughout and (b) that the search confirms
    // non-reachability.
    let mut rng = seeded(20);
    let mut min_positive_sum = f32::INFINITY;
    let mut saw_inf_or_nan_output = false;

    // Sweep radii densely just below the all-clamped threshold, where exactly
    // one tap survives with the smallest possible value.
    let probe = |size: i32, radius: f32, min: &mut f32, saw: &mut bool| {
        let out = expect_match(size, radius);
        let b = branches(size, radius);
        if b.normalised {
            let s2v = s2();
            let mut sum = 0.0f32;
            let hsize = size / 2;
            let rs = 1.6f32 / radius;
            let mut r = -hsize;
            while r <= hsize {
                let x = (r as f32) * rs;
                let v = (1.0f32 / (x * x).exp()) - s2v;
                sum += if v > 0.0 { v } else { 0.0 };
                r += 1;
            }
            if sum > 0.0 && sum < *min {
                *min = sum;
            }
        }
        if out.iter().take(touched_len(size)).any(|v| !v.is_finite()) {
            *saw = true;
        }
    };

    // Dense ULP walk around the boundary radius for several r.
    for r in 1..=6i32 {
        let base = ((r as f32) * (2.0f32 / 3.0f32)).to_bits();
        for d in 0..4000u32 {
            for cand in [base.wrapping_sub(d), base.wrapping_add(d)] {
                let radius = f32::from_bits(cand);
                if !radius.is_finite() || radius == 0.0 {
                    continue;
                }
                probe(2 * r + 1, radius, &mut min_positive_sum, &mut saw_inf_or_nan_output);
            }
        }
    }
    // Broad randomized search.
    for _ in 0..4000 {
        let size = rng.range_i32(1, 65);
        let radius = rng.log_uniform(1e-8, 1e2);
        probe(size, radius, &mut min_positive_sum, &mut saw_inf_or_nan_output);
    }

    println!("E20: smallest positive sum found = {min_positive_sum:e}");
    assert!(
        min_positive_sum > f32::MIN_POSITIVE * 1e10,
        "a positive sum small enough to overflow 1/sum was found ({min_positive_sum:e}); \
         E20 is reachable after all and needs a dedicated assertion"
    );
    assert!(
        !saw_inf_or_nan_output,
        "an inf/NaN escaped into the output buffer; 1/sum overflow may be reachable"
    );
    // Parity was asserted by every `expect_match` above.
}
