//! Phase D — high-volume randomized stress sweeps over the whole surface.
//!
//! Phases B and C cover every `CONFIGS.md` / `ERRORS.md` row; this file exists
//! to push far more values through each entry point (millions of calls) so that
//! value-dependent divergences that a few thousand samples could miss get hit.
//! Everything is still driven through both `.so` files via `libloading`.

mod common;

use common::*;

const BIG: usize = 2_000_000;

#[test]
fn d01_stress_safe_double_to_int_bit_patterns() {
    let (c, r) = safe_double_to_int();
    let mut rng = Rng::new(0xD001);
    let mut d = Diff::new("D1");
    for _ in 0..BIG {
        let v = f64::from_bits(rng.next_u64());
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check(format!("bits={:#018x} ({v:?})", v.to_bits()), cv, rv);
    }
    d.finish();
}

#[test]
fn d02_stress_safe_double_to_int_near_int_range() {
    let (c, r) = safe_double_to_int();
    let mut rng = Rng::new(0xD002);
    let mut d = Diff::new("D2");
    for _ in 0..BIG {
        // Doubles clustered around the int range so both clamp edges and the
        // truncating conversion get hammered.
        let base = rng.next_i32() as f64;
        let jitter = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let v = base + jitter * 4.0 - 2.0;
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check(format!("{v:?}"), cv, rv);
    }
    d.finish();
}

#[test]
fn d03_stress_switch_all_operations() {
    let (c, r) = switch_fallthrough_calculator();
    let mut rng = Rng::new(0xD003);
    let mut d = Diff::new("D3");
    // Structured sweep: every operation from -3..=8 against a wide value spread.
    for op in -3i32..=8 {
        for _ in 0..40_000 {
            let v = rng.next_i32();
            let cv = unsafe { c(v, op) };
            let rv = unsafe { r(v, op) };
            d.check(format!("value={v} op={op}"), cv, rv);
        }
        for &v in &[
            i32::MIN,
            i32::MIN + 1,
            i32::MIN / 2,
            i32::MIN / 3,
            i32::MIN / 8,
            -512,
            -511,
            -129,
            -128,
            -1,
            0,
            1,
            63,
            64,
            127,
            128,
            255,
            256,
            511,
            512,
            i32::MAX / 8,
            i32::MAX / 3,
            i32::MAX / 2,
            i32::MAX - 1,
            i32::MAX,
        ] {
            let cv = unsafe { c(v, op) };
            let rv = unsafe { r(v, op) };
            d.check(format!("value={v} op={op}"), cv, rv);
        }
    }
    // Fully random operations, including out-of-range enum values.
    for _ in 0..400_000 {
        let v = rng.next_i32();
        let op = if rng.next_u64() % 2 == 0 {
            rng.range_i32(-8, 12)
        } else {
            rng.next_i32()
        };
        let cv = unsafe { c(v, op) };
        let rv = unsafe { r(v, op) };
        d.check(format!("value={v} op={op}"), cv, rv);
    }
    d.finish();
}

#[test]
fn d04_stress_fallcalc_uniform() {
    let (c, r) = fallcalc();
    let mut rng = Rng::new(0xD004);
    let mut d = Diff::new("D4");
    for _ in 0..BIG {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let cc = rng.next_i32();
        let dd = rng.next_i32();
        let cv = unsafe { c(a, b, cc, dd) };
        let rv = unsafe { r(a, b, cc, dd) };
        d.check(format!("({a}, {b}, {cc}, {dd})"), cv, rv);
    }
    d.finish();
}

#[test]
fn d05_stress_fallcalc_structured() {
    let (c, r) = fallcalc();
    let mut rng = Rng::new(0xD005);
    let mut d = Diff::new("D5");
    // Cross-product of the branch selectors: param3 % 5 (switch case),
    // param4 % 10 + 1 (inner alloc regime), param3 > 128 (flag).
    for p3r in -4i32..=4 {
        for p4r in -9i32..=9 {
            for _ in 0..1200 {
                // Build param3 with the desired residue and a controlled sign.
                let mag = rng.range_i32(0, 400_000_000);
                let p3 = if p3r <= 0 { -(mag * 5 - p3r) } else { mag * 5 + p3r };
                let p4 = if p4r <= 0 { -(rng.range_i32(0, 200_000_000) * 10 - p4r) } else { rng.range_i32(0, 200_000_000) * 10 + p4r };
                let a = rng.next_i32();
                let b = rng.next_i32();
                let cv = unsafe { c(a, b, p3, p4) };
                let rv = unsafe { r(a, b, p3, p4) };
                d.check(
                    format!("({a}, {b}, {p3}, {p4}) p3%5={} p4%10={}", p3 % 5, p4 % 10),
                    cv,
                    rv,
                );
            }
        }
    }
    // Dense small-value grid around the flag boundary 0200 = 128.
    for p3 in 120..=136 {
        for a in -20..=20 {
            for b in -20..=20 {
                for p4 in [-11, -2, -1, 0, 1, 9, 10] {
                    let cv = unsafe { c(a, b, p3, p4) };
                    let rv = unsafe { r(a, b, p3, p4) };
                    d.check(format!("({a}, {b}, {p3}, {p4})"), cv, rv);
                }
            }
        }
    }
    d.finish();
}

#[test]
fn d06_stress_array_functions() {
    let (fc, fr) = foreach_sum();
    let (pc, pr) = process_array_reverse();
    let mut rng = Rng::new(0xD006);
    let mut d = Diff::new("D6");
    let mut buf: Vec<i32> = vec![0; 256];
    for _ in 0..200_000 {
        for slot in buf.iter_mut() {
            *slot = rng.next_i32();
        }
        let n = rng.range_i32(0, 256);
        let f_c = unsafe { fc(buf.as_mut_ptr(), n) };
        let f_r = unsafe { fr(buf.as_mut_ptr(), n) };
        d.check(format!("foreach n={n}"), f_c, f_r);
        if n > 0 {
            let end = unsafe { buf.as_mut_ptr().add(n as usize - 1) };
            let p_c = unsafe { pc(end, n) };
            let p_r = unsafe { pr(end, n) };
            d.check(format!("reverse n={n}"), p_c, p_r);
            // interior window
            let endidx = rng.range_i32(0, n - 1);
            let count = rng.range_i32(0, endidx + 1);
            let endp = unsafe { buf.as_mut_ptr().add(endidx as usize) };
            let w_c = unsafe { pc(endp, count) };
            let w_r = unsafe { pr(endp, count) };
            d.check(format!("window endidx={endidx} count={count}"), w_c, w_r);
        }
    }
    d.finish();
}

#[test]
fn d07_stress_allocate_and_compute() {
    let (c, r) = allocate_and_compute();
    let mut rng = Rng::new(0xD007);
    let mut d = Diff::new("D7");
    for _ in 0..200_000 {
        // Mostly small sizes (cheap) with a mix of negative and zero.
        let size = match rng.next_u64() % 8 {
            0 => 0,
            1 => -(rng.range_i32(1, i32::MAX - 1)),
            2 => rng.range_i32(1, 4),
            _ => rng.range_i32(0, 200),
        };
        let mult = match rng.next_u64() % 6 {
            0 => f64::from_bits(rng.next_u64()),
            1 => 0.0,
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => f64::NAN,
            _ => rng.next_finite_f64(),
        };
        let cv = unsafe { c(size, mult) };
        let rv = unsafe { r(size, mult) };
        d.check(format!("size={size} mult={mult:?} bits={:#018x}", mult.to_bits()), cv, rv);
    }
    // Sizes that push the accumulated sum through every saturation regime.
    for size in [1, 2, 3, 5, 8, 13, 100, 1000, 5000, 20_000] {
        for mult in [
            0.0, -0.0, 1.0, -1.0, 1.5, -1.5, 1e8, -1e8, 1e150, -1e150, 1e300, -1e300,
            f64::MAX, f64::MIN, f64::MIN_POSITIVE, f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
        ] {
            let cv = unsafe { c(size, mult) };
            let rv = unsafe { r(size, mult) };
            d.check(format!("size={size} mult={mult:?}"), cv, rv);
        }
    }
    d.finish();
}
