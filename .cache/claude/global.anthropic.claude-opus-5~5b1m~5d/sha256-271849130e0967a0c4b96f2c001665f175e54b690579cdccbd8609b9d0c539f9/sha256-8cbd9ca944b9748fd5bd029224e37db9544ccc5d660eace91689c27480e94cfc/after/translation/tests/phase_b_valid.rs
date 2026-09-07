// Phase B — valid-path differential tests. One test per row of CONFIGS.md.
// Every call goes through `dlsym` on both the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Rows 1-5: safe_double_to_int
// ---------------------------------------------------------------------------

fn check_d(ctx: &str, d: f64) {
    let p = pair();
    let c = unsafe { (p.c.safe_double_to_int)(d) };
    let r = unsafe { (p.rust.safe_double_to_int)(d) };
    eq(&format!("{ctx} d={d:?} bits=0x{:016x}", d.to_bits()), c, r);
}

#[test]
fn cfg_01_sdti_in_range_positive_integral() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..20_000 {
        let d = rng.range_i32(0, i32::MAX - 1) as f64;
        check_d("row1", d);
    }
}

#[test]
fn cfg_02_sdti_in_range_negative_integral() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..20_000 {
        let d = rng.range_i32(i32::MIN + 1, -1) as f64;
        check_d("row2", d);
    }
}

#[test]
fn cfg_03_sdti_in_range_fractional() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..20_000 {
        let base = rng.range_i32(i32::MIN / 2, i32::MAX / 2) as f64;
        let frac = rng.range_f64(-0.999_999, 0.999_999);
        check_d("row3", base + frac);
    }
    for d in [
        0.5, -0.5, 0.999, -0.999, 1.0 - f64::EPSILON, -(1.0 - f64::EPSILON), 1e-9, -1e-9,
    ] {
        check_d("row3-fixed", d);
    }
}

#[test]
fn cfg_04_sdti_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..50_000 {
        check_d("row4", rng.next_f64_bits());
    }
}

#[test]
fn cfg_05_sdti_boundary_ladder() {
    for d in interesting_doubles() {
        check_d("row5", d);
    }
    // Dense ULP ladder around both bounds, plus a scaled sweep.
    for base in [i32::MAX as f64, i32::MIN as f64] {
        let mut x = base;
        for _ in 0..64 {
            x = next_after(x, 0.0);
            check_d("row5-inward", x);
        }
        let mut y = base;
        for _ in 0..64 {
            y = next_after(y, if base > 0.0 { f64::INFINITY } else { f64::NEG_INFINITY });
            check_d("row5-outward", y);
        }
    }
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..5_000 {
        let m = rng.range_f64(0.999_999_99, 1.000_000_01);
        check_d("row5-scaled", i32::MAX as f64 * m);
        check_d("row5-scaled", i32::MIN as f64 * m);
    }
}

// ---------------------------------------------------------------------------
// Rows 6-11: switch_fallthrough_calculator
// ---------------------------------------------------------------------------

fn check_switch(ctx: &str, value: c_int, op: c_int) {
    let p = pair();
    let c = unsafe { (p.c.switch_fallthrough_calculator)(value, op) };
    let r = unsafe { (p.rust.switch_fallthrough_calculator)(value, op) };
    eq(&format!("{ctx} value={value} op={op}"), c, r);
}

fn switch_arm_row(row: &str, op: c_int, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..20_000 {
        check_switch(row, rng.next_i32(), op);
    }
    for &v in INTERESTING {
        check_switch(row, v, op);
    }
}

#[test]
fn cfg_06_switch_op0() {
    switch_arm_row("row6", 0, SEED ^ 6);
}

#[test]
fn cfg_07_switch_op1() {
    switch_arm_row("row7", 1, SEED ^ 7);
}

#[test]
fn cfg_08_switch_op2() {
    switch_arm_row("row8", 2, SEED ^ 8);
}

#[test]
fn cfg_09_switch_op3() {
    switch_arm_row("row9", 3, SEED ^ 9);
}

#[test]
fn cfg_10_switch_op4() {
    switch_arm_row("row10", 4, SEED ^ 10);
}

#[test]
fn cfg_11_switch_random_operation() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..50_000 {
        check_switch("row11-wide", rng.next_i32(), rng.next_i32());
    }
    for _ in 0..20_000 {
        // Bias towards the small neighbourhood of the real arms.
        check_switch("row11-near", rng.next_i32(), rng.range_i32(-8, 12));
    }
    for &v in INTERESTING {
        for &o in INTERESTING {
            check_switch("row11-cross", v, o);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 12-16: foreach_sum
// ---------------------------------------------------------------------------

fn check_foreach(ctx: &str, buf: &mut [c_int], off: usize, count: c_int) {
    let p = pair();
    let base = unsafe { buf.as_mut_ptr().add(off) };
    let c = unsafe { (p.c.foreach_sum)(base, count) };
    let r = unsafe { (p.rust.foreach_sum)(base, count) };
    eq(&format!("{ctx} off={off} count={count} buf={buf:?}"), c, r);
}

#[test]
fn cfg_12_foreach_count_one() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..20_000 {
        let mut buf = [rng.next_i32()];
        check_foreach("row12", &mut buf, 0, 1);
    }
}

#[test]
fn cfg_13_foreach_count_five() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..20_000 {
        let mut buf: Vec<c_int> = (0..5).map(|_| rng.next_i32()).collect();
        check_foreach("row13", &mut buf, 0, 5);
    }
}

#[test]
fn cfg_14_foreach_random_count() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..10_000 {
        let n = rng.range_i32(2, 64);
        let mut buf: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        check_foreach("row14", &mut buf, 0, n);
    }
}

#[test]
fn cfg_15_foreach_overflowing_total() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..10_000 {
        let n = rng.range_i32(2, 64);
        let mut buf: Vec<c_int> = (0..n)
            .map(|_| {
                if rng.next_u64() & 1 == 0 {
                    i32::MAX - rng.range_i32(0, 8)
                } else {
                    i32::MIN + rng.range_i32(0, 8)
                }
            })
            .collect();
        check_foreach("row15", &mut buf, 0, n);
    }
}

#[test]
fn cfg_16_foreach_offset_pointer() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..10_000 {
        let n = rng.range_i32(2, 32) as usize;
        let mut buf: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let off = rng.range_i32(0, n as i32 - 1) as usize;
        let count = rng.range_i32(1, (n - off) as i32);
        check_foreach("row16", &mut buf, off, count);
    }
}

// ---------------------------------------------------------------------------
// Rows 17-21: process_array_reverse
// ---------------------------------------------------------------------------

fn check_reverse(ctx: &str, buf: &mut [c_int], end_idx: usize, count: c_int) {
    let p = pair();
    let end = unsafe { buf.as_mut_ptr().add(end_idx) };
    let c = unsafe { (p.c.process_array_reverse)(end, count) };
    let r = unsafe { (p.rust.process_array_reverse)(end, count) };
    eq(
        &format!("{ctx} end_idx={end_idx} count={count} buf={buf:?}"),
        c,
        r,
    );
}

#[test]
fn cfg_17_reverse_count_one() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..20_000 {
        let mut buf = [rng.next_i32()];
        check_reverse("row17", &mut buf, 0, 1);
    }
}

#[test]
fn cfg_18_reverse_fallcalc_shape() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..20_000 {
        let mut buf: Vec<c_int> = (0..5).map(|_| rng.next_i32()).collect();
        check_reverse("row18", &mut buf, 4, 5);
    }
}

#[test]
fn cfg_19_reverse_random_count() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..10_000 {
        let n = rng.range_i32(2, 64);
        let mut buf: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        check_reverse("row19", &mut buf, (n - 1) as usize, n);
    }
}

#[test]
fn cfg_20_reverse_interior_end() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..10_000 {
        let n = rng.range_i32(2, 64) as usize;
        let mut buf: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let end_idx = rng.range_i32(0, n as i32 - 1) as usize;
        // Backward walk of `count` elements must stay in bounds: count <= end_idx+1
        let count = rng.range_i32(1, end_idx as i32 + 1);
        check_reverse("row20", &mut buf, end_idx, count);
    }
}

#[test]
fn cfg_21_reverse_overflowing_sum() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..10_000 {
        let n = rng.range_i32(2, 64);
        let mut buf: Vec<c_int> = (0..n)
            .map(|_| {
                if rng.next_u64() & 1 == 0 {
                    i32::MAX - rng.range_i32(0, 8)
                } else {
                    i32::MIN + rng.range_i32(0, 8)
                }
            })
            .collect();
        check_reverse("row21", &mut buf, (n - 1) as usize, n);
    }
}

// ---------------------------------------------------------------------------
// Row 22: composed low-level pipeline (as fallcalc does it)
// ---------------------------------------------------------------------------

#[test]
fn cfg_22_foreach_then_reverse_pipeline() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..10_000 {
        let n = rng.range_i32(1, 32);
        let mut buf: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let ptr = buf.as_mut_ptr();
        let last = unsafe { ptr.add((n - 1) as usize) };

        let c_total = unsafe { (p.c.foreach_sum)(ptr, n) }
            .wrapping_add(unsafe { (p.c.process_array_reverse)(last, n) });
        let r_total = unsafe { (p.rust.foreach_sum)(ptr, n) }
            .wrapping_add(unsafe { (p.rust.process_array_reverse)(last, n) });
        eq(&format!("row22 n={n} buf={buf:?}"), c_total, r_total);

        // Also confirm neither implementation mutated the buffer.
        let snapshot = buf.clone();
        unsafe { (p.c.foreach_sum)(buf.as_mut_ptr(), n) };
        unsafe { (p.rust.foreach_sum)(buf.as_mut_ptr(), n) };
        assert_eq!(snapshot, buf, "row22: buffer was mutated");
    }
}

// ---------------------------------------------------------------------------
// Rows 23-30: allocate_and_compute
// ---------------------------------------------------------------------------

fn check_alloc(ctx: &str, size: c_int, mult: f64) {
    let p = pair();
    let c = unsafe { (p.c.allocate_and_compute)(size, mult) };
    let r = unsafe { (p.rust.allocate_and_compute)(size, mult) };
    eq(
        &format!("{ctx} size={size} mult={mult:?} bits=0x{:016x}", mult.to_bits()),
        c,
        r,
    );
}

#[test]
fn cfg_23_alloc_size_one() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..20_000 {
        check_alloc("row23", 1, rng.range_f64(-1e6, 1e6));
    }
}

#[test]
fn cfg_24_alloc_size_two() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..20_000 {
        check_alloc("row24", 2, rng.range_f64(-1e6, 1e6));
    }
}

#[test]
fn cfg_25_alloc_fallcalc_shape() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..20_000 {
        check_alloc("row25", rng.range_i32(1, 10), 1.5);
    }
    for size in 1..=10 {
        check_alloc("row25-sweep", size, 1.5);
    }
}

#[test]
fn cfg_26_alloc_zero_and_negative_multiplier() {
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..10_000 {
        let size = rng.range_i32(1, 10);
        check_alloc("row26-zero", size, 0.0);
        check_alloc("row26-negzero", size, -0.0);
        check_alloc("row26-neg", size, rng.range_f64(-1e9, -1e-9));
    }
}

#[test]
fn cfg_27_alloc_large_sizes() {
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..2_000 {
        let size = rng.range_i32(1, 4096);
        check_alloc("row27", size, rng.range_f64(-1e4, 1e4));
    }
}

#[test]
fn cfg_28_alloc_huge_multiplier_saturates() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..5_000 {
        let size = rng.range_i32(2, 64);
        let mag = rng.range_f64(1e250, 1e308);
        check_alloc("row28-pos", size, mag);
        check_alloc("row28-neg", size, -mag);
    }
}

#[test]
fn cfg_29_alloc_tiny_multiplier() {
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..5_000 {
        let size = rng.range_i32(2, 64);
        let mag = rng.range_f64(0.0, 1e-6);
        check_alloc("row29-pos", size, mag);
        check_alloc("row29-neg", size, -mag);
        check_alloc("row29-subnormal", size, 5e-324);
    }
}

#[test]
fn cfg_30_alloc_random_multiplier_bits() {
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..20_000 {
        check_alloc("row30", rng.range_i32(1, 10), rng.next_f64_bits());
    }
    for d in interesting_doubles() {
        for size in 1..=6 {
            check_alloc("row30-fixed", size, d);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 31-36: fallcalc
// ---------------------------------------------------------------------------

fn check_fallcalc(ctx: &str, a: c_int, b: c_int, c_: c_int, d: c_int) {
    let p = pair();
    let c = unsafe { (p.c.fallcalc)(a, b, c_, d) };
    let r = unsafe { (p.rust.fallcalc)(a, b, c_, d) };
    eq(&format!("{ctx} fallcalc({a},{b},{c_},{d})"), c, r);
}

#[test]
fn cfg_31_fallcalc_small_nonnegative() {
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..20_000 {
        check_fallcalc(
            "row31",
            rng.range_i32(0, 100),
            rng.range_i32(0, 100),
            rng.range_i32(0, 100),
            rng.range_i32(0, 100),
        );
    }
}

#[test]
fn cfg_32_fallcalc_param3_sweep() {
    let mut rng = Rng::new(SEED ^ 32);
    for p3 in -20..=260 {
        for _ in 0..64 {
            check_fallcalc("row32", rng.next_i32(), rng.next_i32(), p3, rng.next_i32());
        }
    }
}

#[test]
fn cfg_33_fallcalc_param4_sweep() {
    let mut rng = Rng::new(SEED ^ 33);
    for p4 in -30..=30 {
        for _ in 0..128 {
            check_fallcalc(
                "row33",
                rng.range_i32(-1000, 1000),
                rng.range_i32(-1000, 1000),
                rng.range_i32(-1000, 1000),
                p4,
            );
        }
    }
}

#[test]
fn cfg_34_fallcalc_wide_param1_param2() {
    let mut rng = Rng::new(SEED ^ 34);
    for _ in 0..20_000 {
        check_fallcalc(
            "row34",
            rng.next_i32(),
            rng.next_i32(),
            rng.range_i32(0, 300),
            rng.range_i32(0, 30),
        );
    }
}

#[test]
fn cfg_35_fallcalc_full_random() {
    let mut rng = Rng::new(SEED ^ 35);
    for _ in 0..50_000 {
        check_fallcalc(
            "row35",
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

#[test]
fn cfg_36_fallcalc_interesting_cross_product() {
    // Full 4-way cross product of INTERESTING would be ~4.5M calls; sample it
    // deterministically plus do an exhaustive 2-way sweep on the two params
    // that select code paths (param3, param4).
    let mut rng = Rng::new(SEED ^ 36);
    for &p3 in INTERESTING {
        for &p4 in INTERESTING {
            for &p1 in &[0i32, 1, -1, i32::MAX, i32::MIN] {
                for &p2 in &[0i32, 1, -1, i32::MAX, i32::MIN] {
                    check_fallcalc("row36-exhaustive34", p1, p2, p3, p4);
                }
            }
        }
    }
    for _ in 0..50_000 {
        check_fallcalc(
            "row36-sampled",
            rng.pick(INTERESTING),
            rng.pick(INTERESTING),
            rng.pick(INTERESTING),
            rng.pick(INTERESTING),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 37: all six symbols exercised in fallcalc's internal call order
// ---------------------------------------------------------------------------

#[test]
fn cfg_37_combined_driver_all_symbols() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 37);

    // Re-implements fallcalc's body *using only dlsym'd symbols*, so the two
    // libraries' low-level pieces are composed exactly as the C wrapper does.
    fn drive(i: &common::Impl, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> Vec<c_int> {
        let mut out = Vec::new();
        let mut buf: Vec<c_int> = (0..5)
            .map(|k| ((k + 1) * 8i32).wrapping_add(p1))
            .collect();
        let ptr = buf.as_mut_ptr();
        out.push(unsafe { (i.foreach_sum)(ptr, 5) });
        out.push(unsafe { (i.process_array_reverse)(ptr.add(4), 5) });
        out.push(unsafe { (i.switch_fallthrough_calculator)(p2, p3.wrapping_rem(5)) });
        let fc = (p1 as f64) * 3.7 + (p2 as f64) * 2.3 - (p3 as f64) * 0.5;
        out.push(unsafe { (i.safe_double_to_int)(fc) });
        out.push(unsafe { (i.allocate_and_compute)(p4.wrapping_rem(10).wrapping_add(1), 1.5) });
        out.push(unsafe { (i.fallcalc)(p1, p2, p3, p4) });
        out
    }

    for _ in 0..20_000 {
        let (p1, p2, p3, p4) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let cv = drive(&p.c, p1, p2, p3, p4);
        let rv = drive(&p.rust, p1, p2, p3, p4);
        assert_eq!(
            cv, rv,
            "DIVERGENCE [row37] params=({p1},{p2},{p3},{p4}): C={cv:?} Rust={rv:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 38: symbol parity re-checked from inside the test process
// ---------------------------------------------------------------------------

#[test]
fn cfg_38_all_symbols_resolvable_in_both() {
    // `pair()` panics if any of the six symbols is missing from either .so.
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
}
