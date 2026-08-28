// High-volume randomized soak. `#[ignore]`d so the normal suite stays fast;
// run explicitly with:
//     cargo test --offline --release --test soak -- --ignored --nocapture
//
// Purpose: the `fallcalc` float expression
//     (double)param1 * 3.7 + (double)param2 * 2.3 - (double)param3 * 0.5
// and the `allocate_and_compute` accumulator are the only places where a
// rounding difference (e.g. FMA contraction, x87 excess precision, reassociation)
// could hide. Those need volume, not cleverness, to rule out.

mod common;

use common::{diff_eq, Bits, Rng};

const SOAK: usize = 2_000_000;

#[test]
#[ignore = "long-running soak; run with --ignored"]
fn soak_fallcalc_random() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x50AC_0000_0001);
    for i in 0..SOAK {
        let p = (rng.i32_any(), rng.i32_any(), rng.i32_any(), rng.i32_any());
        let cv = unsafe { (c.fallcalc)(p.0, p.1, p.2, p.3) };
        let rv = unsafe { (r.fallcalc)(p.0, p.1, p.2, p.3) };
        diff_eq("soak/fallcalc", format!("iter={i} {p:?}"), cv, rv);
    }
    println!("fallcalc: {SOAK} random quadruples matched");
}

#[test]
#[ignore = "long-running soak; run with --ignored"]
fn soak_fallcalc_small_and_mixed() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x50AC_0000_0002);
    for i in 0..SOAK {
        // Small magnitudes keep the float expression in the exactly-rounded
        // regime where a contraction difference would show up as +/-1.
        let p = (
            rng.i32_in(-100_000, 100_000),
            rng.i32_in(-100_000, 100_000),
            rng.i32_in(-100_000, 100_000),
            rng.i32_in(-100, 100),
        );
        let cv = unsafe { (c.fallcalc)(p.0, p.1, p.2, p.3) };
        let rv = unsafe { (r.fallcalc)(p.0, p.1, p.2, p.3) };
        diff_eq("soak/fallcalc-small", format!("iter={i} {p:?}"), cv, rv);
    }
    println!("fallcalc: {SOAK} small-magnitude quadruples matched");
}

#[test]
#[ignore = "long-running soak; run with --ignored"]
fn soak_safe_double_to_int_random_bits() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x50AC_0000_0003);
    for i in 0..(SOAK * 4) {
        let d = rng.f64_bits();
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("soak/d2i", format!("iter={i} {}", Bits(d)), cv, rv);
    }
    println!("safe_double_to_int: {} random bit patterns matched", SOAK * 4);
}

#[test]
#[ignore = "long-running soak; run with --ignored"]
fn soak_allocate_and_compute() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x50AC_0000_0004);
    for i in 0..(SOAK / 4) {
        let size = rng.i32_in(-4, 64);
        let m = if i % 3 == 0 {
            rng.f64_bits()
        } else {
            rng.f64_moderate()
        };
        let cv = unsafe { (c.allocate_and_compute)(size, m) };
        let rv = unsafe { (r.allocate_and_compute)(size, m) };
        diff_eq(
            "soak/alloc",
            format!("iter={i} size={size} mult={}", Bits(m)),
            cv,
            rv,
        );
    }
    println!("allocate_and_compute: {} cases matched", SOAK / 4);
}

#[test]
#[ignore = "long-running soak; run with --ignored"]
fn soak_switch_exhaustive_ops_random_values() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x50AC_0000_0005);
    for i in 0..(SOAK / 2) {
        let v = rng.i32_any();
        // Cover the five real arms and the default arm uniformly.
        let op = rng.i32_in(-3, 8);
        let cv = unsafe { (c.switch_fallthrough_calculator)(v, op) };
        let rv = unsafe { (r.switch_fallthrough_calculator)(v, op) };
        diff_eq("soak/switch", format!("iter={i} v={v} op={op}"), cv, rv);
    }
    println!("switch_fallthrough_calculator: {} cases matched", SOAK / 2);
}

#[test]
#[ignore = "long-running soak; run with --ignored"]
fn soak_array_functions() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x50AC_0000_0006);
    for i in 0..(SOAK / 8) {
        let len = rng.usize_in(1, 128);
        let mut buf: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        let count = rng.i32_in(1, len as i32);

        let (cf, rf) = common::foreach_both(&c, &r, &mut buf, count);
        diff_eq("soak/foreach", format!("iter={i} len={len} n={count}"), cf, rf);

        let (cr, rr) = common::reverse_both(&c, &r, &mut buf, count);
        diff_eq("soak/reverse", format!("iter={i} len={len} n={count}"), cr, rr);
    }
    println!("foreach_sum / process_array_reverse: {} cases matched", SOAK / 8);
}
