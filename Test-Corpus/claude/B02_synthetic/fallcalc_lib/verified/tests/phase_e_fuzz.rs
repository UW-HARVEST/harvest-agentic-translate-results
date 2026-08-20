// Phase E (extra) — long-running randomised sweeps over the whole input space.
//
// Not part of the mandatory phases; run with
//   cargo test --test phase_e_fuzz -- --ignored --nocapture
// These are `#[ignore]`d so the default `cargo test` stays fast.

mod common;

use common::{both, eq_i32, Buf, Rng};

#[test]
#[ignore]
fn fuzz_fallcalc_millions() {
    let (c, r) = both();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    let mut n = 0u64;
    for _ in 0..1_000_000 {
        let (p1, p2, p3, p4) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        let cv = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
        let rv = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
        eq_i32("fuzz/fallcalc", (p1, p2, p3, p4), cv, rv);
        n += 1;
    }
    // biased sampling around every boundary
    for _ in 0..1_000_000 {
        let (p1, p2, p3, p4) = (
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
        let cv = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
        let rv = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
        eq_i32("fuzz/fallcalc-spicy", (p1, p2, p3, p4), cv, rv);
        n += 1;
    }
    println!("fuzz_fallcalc_millions: {n} cases compared");
}

#[test]
#[ignore]
fn fuzz_safe_double_to_int_millions() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0BAD_F00D_1234_5678);
    for _ in 0..3_000_000 {
        let d = rng.next_f64_bits();
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("fuzz/sdti-bits", (d, d.to_bits()), cv, rv);
    }
    for _ in 0..1_000_000 {
        // concentrate near the clamp boundaries
        let d = rng.f64_in(-2_200_000_000.0, 2_200_000_000.0);
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("fuzz/sdti-near-clamp", (d, d.to_bits()), cv, rv);
    }
    println!("fuzz_safe_double_to_int_millions: 4,000,000 cases compared");
}

#[test]
#[ignore]
fn fuzz_switch_exhaustive_operations() {
    // Exhaustive over `operation` in a wide window, randomised `value`.
    let (c, r) = both();
    let mut rng = Rng::new(0x5EED_5EED_5EED_5EED);
    for op in -2000..=2000 {
        for _ in 0..200 {
            let v = if rng.below(2) == 0 {
                rng.next_i32()
            } else {
                rng.spicy_i32()
            };
            let cv = unsafe { (c.switch_fallthrough_calculator)(v, op) };
            let rv = unsafe { (r.switch_fallthrough_calculator)(v, op) };
            eq_i32("fuzz/switch", (v, op), cv, rv);
        }
    }
    println!("fuzz_switch_exhaustive_operations: 800,200 cases compared");
}

#[test]
#[ignore]
fn fuzz_array_apis() {
    let (c, r) = both();
    let mut rng = Rng::new(0xA5A5_5A5A_A5A5_5A5A);
    for _ in 0..20_000 {
        let n = 1 + rng.below(64) as usize;
        let mut b = Buf::new(
            (0..n)
                .map(|_| if rng.below(2) == 0 { rng.next_i32() } else { rng.spicy_i32() })
                .collect(),
        );
        let count = rng.below(n as u64 + 1) as i32;
        let head = b.ptr();
        eq_i32(
            "fuzz/foreach",
            (n, count),
            unsafe { (c.foreach_sum)(head, count) },
            unsafe { (r.foreach_sum)(head, count) },
        );
        let k = rng.below(n as u64) as usize;
        let rcount = rng.below(k as u64 + 2) as i32; // 0..=k+1, in bounds
        let p = b.ptr_at(k);
        eq_i32(
            "fuzz/reverse",
            (n, k, rcount),
            unsafe { (c.process_array_reverse)(p, rcount) },
            unsafe { (r.process_array_reverse)(p, rcount) },
        );
    }
    println!("fuzz_array_apis: 40,000 cases compared");
}

#[test]
#[ignore]
fn fuzz_allocate_and_compute() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1357_9BDF_2468_ACE0);
    for _ in 0..40_000 {
        let size = match rng.below(5) {
            0 => rng.range_i32(-32, 32),
            1 => rng.range_i32(0, 1024),
            2 => rng.range_i32(-1024, 0),
            3 => rng.next_i32() >> 20, // moderate magnitudes, both signs
            _ => rng.range_i32(0, 10),
        };
        let mult = match rng.below(3) {
            0 => rng.next_f64_bits(),
            1 => rng.f64_in(-1e6, 1e6),
            _ => rng.f64_in(-2.0, 2.0),
        };
        eq_i32(
            "fuzz/alloc",
            (size, mult, mult.to_bits()),
            unsafe { (c.allocate_and_compute)(size, mult) },
            unsafe { (r.allocate_and_compute)(size, mult) },
        );
    }
    println!("fuzz_allocate_and_compute: 40,000 cases compared");
}
