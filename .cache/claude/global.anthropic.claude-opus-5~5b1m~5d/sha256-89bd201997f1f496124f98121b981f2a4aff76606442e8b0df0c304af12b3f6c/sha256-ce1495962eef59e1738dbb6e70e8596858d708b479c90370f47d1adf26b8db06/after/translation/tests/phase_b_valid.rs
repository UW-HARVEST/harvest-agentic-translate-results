// Phase B — valid-path differential tests, one test per CONFIGS.md row.
//
// Every call goes through `libloading` into either the C `.so` or the Rust `.so`;
// the Rust crate is never called directly.

mod common;

use common::*;
use std::os::raw::{c_double, c_int};

// ===========================================================================
// Rows 1-5: the four leaf operations
// ===========================================================================

fn diff_op(
    name: &str,
    pick: fn(&Impl) -> unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    gen: fn(&mut Rng) -> (c_int, c_int),
) {
    let p = pair();
    let mut rng = Rng::seeded();
    let (cf, rf) = (pick(&p.c), pick(&p.rs));
    for i in 0..200_000 {
        let (a, b) = gen(&mut rng);
        // random junk in the "unused" slots: proves they really are ignored and
        // that the ABI passes 4 args identically.
        let (u1, u2) = (rng.i32(), rng.i32());
        let cv = unsafe { cf(a, b, u1, u2) };
        let rv = unsafe { rf(a, b, u1, u2) };
        assert_eq!(
            cv, rv,
            "{name} #{i}: a={a} b={b} u1={u1} u2={u2} -> C={cv} Rust={rv}"
        );
    }
}

#[test] // row 1
fn row01_add_operation() {
    diff_op("add", |i| i.add_operation, |r| (r.spicy_i32(), r.spicy_i32()));
}

#[test] // row 2
fn row02_multiply_operation() {
    diff_op(
        "multiply",
        |i| i.multiply_operation,
        |r| (r.spicy_i32(), r.spicy_i32()),
    );
}

#[test] // row 3
fn row03_subtract_operation() {
    diff_op(
        "subtract",
        |i| i.subtract_operation,
        |r| (r.spicy_i32(), r.spicy_i32()),
    );
}

#[test] // row 4
fn row04_modulo_operation_nonzero_divisor() {
    diff_op(
        "modulo",
        |i| i.modulo_operation,
        |r| {
            let a = r.spicy_i32();
            let mut b = r.spicy_i32();
            if b == 0 {
                b = 1;
            }
            // `INT_MIN % -1` SIGFPEs inside the C library (ERRORS.md row 2), so
            // that single pair is steered away from rather than compared.
            if is_idiv_trap(a, b) {
                b = -2;
            }
            (a, b)
        },
    );
}

#[test] // row 5
fn row05_modulo_operation_zero_divisor() {
    let p = pair();
    let mut rng = Rng::seeded();
    for _ in 0..20_000 {
        let a = rng.spicy_i32();
        assert_eq!(
            unsafe { (p.c.modulo_operation)(a, 0, rng.i32(), rng.i32()) },
            unsafe { (p.rs.modulo_operation)(a, 0, 0, 0) },
            "modulo({a}, 0)"
        );
    }
    // `(INT_MIN, -1)` is NOT called here: the C library SIGFPEs on it
    // (ERRORS.md row 2). Instead assert the neighbours are identical, which pins
    // down the guard's exact extent.
    for (a, b) in [
        (i32::MIN, -2),
        (i32::MIN + 1, -1),
        (i32::MIN, 1),
        (i32::MAX, -1),
        (-1, -1),
        (0, -1),
    ] {
        assert_eq!(
            unsafe { (p.c.modulo_operation)(a, b, 0, 0) },
            unsafe { (p.rs.modulo_operation)(a, b, 0, 0) },
            "modulo({a}, {b})"
        );
    }
}

// ===========================================================================
// Rows 6-9: the double entry points
// ===========================================================================

#[test] // rows 6 + 7
fn row06_07_safe_double_to_int() {
    let p = pair();
    let mut rng = Rng::seeded();

    // fixed shapes first
    let fixed: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1), // subnormal
        f64::from_bits(0x8000_0000_0000_0001),
        0.25,
        -0.25,
        0.5,
        -0.5,
        0.75,
        -0.75,
        1.0,
        -1.0,
        1.9999,
        -1.9999,
        2147483646.0,
        2147483646.5,
        2147483646.9999,
        2147483647.0,
        2147483647.5,
        2147483648.0,
        2147483649.0,
        -2147483646.0,
        -2147483647.0,
        -2147483647.5,
        -2147483648.0,
        -2147483648.5,
        -2147483649.0,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF8_0000_0000_1234), // NaN with payload
        f64::EPSILON,
    ];
    for d in &fixed {
        let (cv, rv) = unsafe {
            (
                (p.c.safe_double_to_int)(*d),
                (p.rs.safe_double_to_int)(*d),
            )
        };
        assert_eq!(
            cv, rv,
            "safe_double_to_int({d:?} bits=0x{:016x}) -> C={cv} Rust={rv}",
            d.to_bits()
        );
    }

    // then 400k randomized / boundary-band values
    for i in 0..400_000 {
        let d = rng.spicy_f64();
        let (cv, rv) = unsafe {
            ((p.c.safe_double_to_int)(d), (p.rs.safe_double_to_int)(d))
        };
        assert_eq!(
            cv, rv,
            "#{i} safe_double_to_int(bits=0x{:016x} = {d:?}) -> C={cv} Rust={rv}",
            d.to_bits()
        );
    }
}

#[test] // rows 8 + 9
fn row08_09_compute_scaled_value() {
    let p = pair();
    let mut rng = Rng::seeded();

    let scales: [c_double; 14] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.333,
        0.75,
        0.8,
        1.5,
        1e10,
        -1e10,
        1e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    let bases: [c_int; 9] = [
        0,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX / 2,
        i32::MIN / 2,
    ];
    for &b in &bases {
        for &s in &scales {
            let (cv, rv) = unsafe {
                (
                    (p.c.compute_scaled_value)(b, s),
                    (p.rs.compute_scaled_value)(b, s),
                )
            };
            assert_eq!(cv, rv, "compute_scaled_value({b}, {s:?})");
        }
    }
    for i in 0..300_000 {
        let b = rng.spicy_i32();
        let s = rng.spicy_f64();
        let (cv, rv) = unsafe {
            (
                (p.c.compute_scaled_value)(b, s),
                (p.rs.compute_scaled_value)(b, s),
            )
        };
        assert_eq!(
            cv, rv,
            "#{i} compute_scaled_value({b}, bits=0x{:016x})",
            s.to_bits()
        );
    }
}

// ===========================================================================
// Rows 10-17: init_result_array
// ===========================================================================

/// Runs `init_result_array` on two identical buffers (one per implementation) and
/// asserts the resulting 248+slack bytes are identical.
fn diff_init(ctx: &str, start: &ArrBuf, values: &[c_int], count: c_int) {
    let p = pair();
    let mut cb = start.clone();
    let mut rb = start.clone();
    let mut cv = values.to_vec();
    let mut rv = values.to_vec();
    unsafe {
        (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), count);
        (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), count);
    }
    assert_bufs_eq(ctx, &cb, &rb);
    assert_eq!(cv, rv, "values[] mutated differently [{ctx}]");
}

#[test] // row 10
fn row10_init_count_zero() {
    let mut rng = Rng::seeded();
    for _ in 0..500 {
        let z = ArrBuf::zeroed();
        diff_init("init count=0 zeroed", &z, &[rng.i32(); 16], 0);
        let poison = ArrBuf::poisoned(&mut rng);
        diff_init("init count=0 poisoned", &poison, &[rng.i32(); 16], 0);
    }
}

#[test] // rows 11 + 12
fn row11_12_init_count_one_and_two() {
    let mut rng = Rng::seeded();
    for count in [1, 2] {
        for _ in 0..2_000 {
            let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
            let poison = ArrBuf::poisoned(&mut rng);
            diff_init(&format!("init count={count}"), &poison, &vals, count);
        }
    }
}

#[test] // rows 13 + 14
fn row13_14_init_count_three_to_ten() {
    let mut rng = Rng::seeded();
    for count in 3..=10 {
        for _ in 0..3_000 {
            let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
            let poison = ArrBuf::poisoned(&mut rng);
            diff_init(&format!("init count={count} poisoned"), &poison, &vals, count);
            let z = ArrBuf::zeroed();
            diff_init(&format!("init count={count} zeroed"), &z, &vals, count);
        }
    }
}

#[test] // row 15
fn row15_init_count_over_capacity_clamps() {
    let mut rng = Rng::seeded();
    // The `values` buffer is 128 wide, so even if a broken clamp read past 10 the
    // access would stay in memory we own and produce a *visible* difference
    // instead of a crash.
    for count in 11..=64 {
        let vals: Vec<c_int> = (0..128).map(|_| rng.spicy_i32()).collect();
        let poison = ArrBuf::poisoned(&mut rng);
        diff_init(&format!("init count={count} clamp"), &poison, &vals, count);
    }
    for count in [100, 1000, i32::MAX - 1, i32::MAX] {
        let vals: Vec<c_int> = (0..128).map(|_| rng.spicy_i32()).collect();
        let poison = ArrBuf::poisoned(&mut rng);
        diff_init(&format!("init count={count} clamp"), &poison, &vals, count);
    }
}

#[test] // row 16
fn row16_init_boundary_values_scaled_bits() {
    let mut rng = Rng::seeded();
    let edges: [c_int; 10] = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
    ];
    // every boundary value at every slot
    for &e in &edges {
        let vals = vec![e; 16];
        let poison = ArrBuf::poisoned(&mut rng);
        diff_init(&format!("init all={e}"), &poison, &vals, 10);
    }
    for _ in 0..5_000 {
        let vals: Vec<c_int> = (0..16).map(|_| edges[rng.below(edges.len())]).collect();
        let poison = ArrBuf::poisoned(&mut rng);
        diff_init("init edge mix", &poison, &vals, 10);
    }
}

#[test] // row 17
fn row17_init_called_twice_stale_tail() {
    let p = pair();
    let mut rng = Rng::seeded();
    for _ in 0..3_000 {
        let big = (rng.below(10) + 1) as c_int;
        let small = rng.below(big as usize + 1) as c_int;
        let v1: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
        let v2: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
        let start = ArrBuf::poisoned(&mut rng);
        let mut cb = start.clone();
        let mut rb = start.clone();
        let (mut c1, mut c2) = (v1.clone(), v2.clone());
        let (mut r1, mut r2) = (v1.clone(), v2.clone());
        unsafe {
            (p.c.init_result_array)(cb.as_ptr(), c1.as_mut_ptr(), big);
            (p.c.init_result_array)(cb.as_ptr(), c2.as_mut_ptr(), small);
            (p.rs.init_result_array)(rb.as_ptr(), r1.as_mut_ptr(), big);
            (p.rs.init_result_array)(rb.as_ptr(), r2.as_mut_ptr(), small);
        }
        assert_bufs_eq(&format!("init twice {big}->{small}"), &cb, &rb);
    }
}

// ===========================================================================
// Rows 18-28: process_with_foreach
// ===========================================================================

/// Applies a sequence of ops to a freshly initialised array in both
/// implementations and compares the return values *and* the mutated memory.
///
/// `op_from_c` selects whether the function pointer handed to *both* drivers comes
/// from the C `.so` or the Rust `.so` — the cross-provider check of row 23.
fn diff_process_seq(
    ctx: &str,
    start: &ArrBuf,
    values: &[c_int],
    init_count: Option<c_int>,
    direct_count: Option<c_int>,
    op_idx: &[usize],
    op_from_c: bool,
) {
    let p = pair();
    let mut cb = start.clone();
    let mut rb = start.clone();
    let mut cv = values.to_vec();
    let mut rv = values.to_vec();

    unsafe {
        if let Some(n) = init_count {
            (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), n);
            (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), n);
        }
    }
    if let Some(n) = direct_count {
        cb.set_count(n);
        rb.set_count(n);
    }

    let provider: &Impl = if op_from_c { &p.c } else { &p.rs };
    let ops = provider.ops();

    let mut ctot: Vec<c_int> = Vec::new();
    let mut rtot: Vec<c_int> = Vec::new();
    for &oi in op_idx {
        unsafe {
            ctot.push((p.c.process_with_foreach)(cb.as_ptr(), Some(ops[oi])));
            rtot.push((p.rs.process_with_foreach)(rb.as_ptr(), Some(ops[oi])));
        }
    }
    let names: Vec<&str> = op_idx.iter().map(|&i| OP_NAMES[i]).collect();
    let ctx = format!(
        "{ctx} ops={names:?} op_provider={}",
        if op_from_c { "C.so" } else { "Rust.so" }
    );
    assert_eq!(ctot, rtot, "process_with_foreach totals differ [{ctx}]");
    assert_bufs_eq(&ctx, &cb, &rb);
}

#[test] // rows 18-21
fn row18_21_process_each_op() {
    let mut rng = Rng::seeded();
    for oi in 0..4 {
        for count in [1, 2, 5, 9, 10] {
            for _ in 0..2_000 {
                let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
                let poison = ArrBuf::poisoned(&mut rng);
                diff_process_seq(
                    &format!("process op={} count={count}", OP_NAMES[oi]),
                    &poison,
                    &vals,
                    Some(count),
                    None,
                    &[oi],
                    true,
                );
            }
        }
    }
}

#[test] // row 22
fn row22_process_count_zero() {
    let mut rng = Rng::seeded();
    for oi in 0..4 {
        for _ in 0..500 {
            let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
            let poison = ArrBuf::poisoned(&mut rng);
            diff_process_seq(
                &format!("process count=0 op={}", OP_NAMES[oi]),
                &poison,
                &vals,
                Some(0),
                None,
                &[oi],
                true,
            );
        }
    }
}

#[test] // row 23
fn row23_cross_provider_function_pointers() {
    let mut rng = Rng::seeded();
    // Same work driven with the op pointer taken from the C .so and from the
    // Rust .so: both drivers must agree in both directions.
    for from_c in [true, false] {
        for oi in 0..4 {
            for count in [1, 3, 10] {
                for _ in 0..1_000 {
                    let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
                    let poison = ArrBuf::poisoned(&mut rng);
                    diff_process_seq(
                        "cross-provider",
                        &poison,
                        &vals,
                        Some(count),
                        None,
                        &[oi],
                        from_c,
                    );
                }
            }
        }
    }
}

// --- row 24: a harness callback that records every argument ----------------

use std::sync::Mutex;

static CALLS: Mutex<Vec<[c_int; 4]>> = Mutex::new(Vec::new());

unsafe extern "C" fn recorder(a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
    CALLS.lock().unwrap().push([a, b, u1, u2]);
    // Deterministic, value-dependent, and exercises the saturating path.
    a.wrapping_mul(3).wrapping_sub(b).wrapping_add(0x2000_0000)
}

#[test] // row 24
fn row24_harness_callback_argument_sequence() {
    let p = pair();
    let mut rng = Rng::seeded();
    for count in 0..=10 {
        for _ in 0..500 {
            let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
            let start = ArrBuf::poisoned(&mut rng);
            let mut cb = start.clone();
            let mut rb = start.clone();
            let mut cv = vals.clone();
            let mut rv = vals.clone();

            let (ctot, c_calls) = unsafe {
                (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), count);
                CALLS.lock().unwrap().clear();
                let t = (p.c.process_with_foreach)(cb.as_ptr(), Some(recorder));
                (t, CALLS.lock().unwrap().clone())
            };
            let (rtot, r_calls) = unsafe {
                (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), count);
                CALLS.lock().unwrap().clear();
                let t = (p.rs.process_with_foreach)(rb.as_ptr(), Some(recorder));
                (t, CALLS.lock().unwrap().clone())
            };

            assert_eq!(
                c_calls, r_calls,
                "callback argument sequence differs (count={count})"
            );
            assert_eq!(c_calls.len(), count.max(0) as usize, "call count");
            for c in &c_calls {
                assert_eq!((c[2], c[3]), (0, 0), "unused args must be literal 0,0");
            }
            assert_eq!(ctot, rtot, "recorder total (count={count})");
            assert_bufs_eq(&format!("recorder count={count}"), &cb, &rb);
        }
    }
}

#[test] // row 25
fn row25_process_repeated_same_op() {
    let mut rng = Rng::seeded();
    for oi in 0..4 {
        for count in [1, 4, 10] {
            for _ in 0..1_000 {
                let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
                let poison = ArrBuf::poisoned(&mut rng);
                diff_process_seq(
                    "process x4 same op",
                    &poison,
                    &vals,
                    Some(count),
                    None,
                    &[oi, oi, oi, oi],
                    true,
                );
            }
        }
    }
}

#[test] // row 26
fn row26_process_arrayfunc_op_sequence() {
    let mut rng = Rng::seeded();
    for count in 0..=10 {
        for _ in 0..2_000 {
            let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
            let poison = ArrBuf::poisoned(&mut rng);
            diff_process_seq(
                &format!("arrayfunc op pipeline count={count}"),
                &poison,
                &vals,
                Some(count),
                None,
                &[0, 1, 2, 3],
                true,
            );
        }
    }
    // random-length random-order op sequences too
    for _ in 0..5_000 {
        let count = rng.below(11) as c_int;
        let n = rng.below(6) + 1;
        let seq: Vec<usize> = (0..n).map(|_| rng.below(4)).collect();
        let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
        let poison = ArrBuf::poisoned(&mut rng);
        diff_process_seq("random op sequence", &poison, &vals, Some(count), None, &seq, true);
    }
}

#[test] // row 27
fn row27_process_saturating_values() {
    let mut rng = Rng::seeded();
    let edges: [c_int; 8] = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        0x4000_0000,
        -0x4000_0000,
    ];
    for _ in 0..10_000 {
        let vals: Vec<c_int> = (0..16).map(|_| edges[rng.below(edges.len())]).collect();
        let count = (rng.below(10) + 1) as c_int;
        let poison = ArrBuf::poisoned(&mut rng);
        let n = rng.below(5) + 1;
        let seq: Vec<usize> = (0..n).map(|_| rng.below(4)).collect();
        diff_process_seq("saturating", &poison, &vals, Some(count), None, &seq, true);
    }
}

#[test] // row 28
fn row28_process_hand_built_state() {
    let p = pair();
    let mut rng = Rng::seeded();
    // `count` written directly, `rank` decoupled from the index, arbitrary
    // `scaled` bits — nothing goes through `init_result_array`.
    for _ in 0..10_000 {
        let count = rng.below(11) as c_int;
        let mut start = ArrBuf::poisoned(&mut rng);
        for i in 0..CAP {
            start.set_value(i, rng.spicy_i32());
            start.set_scaled(i, rng.spicy_f64());
            // `rank` is the `b` argument of every op. A rank of -1 would SIGFPE the
            // C `modulo_operation` as soon as some op leaves `INT32_MIN` in `value`
            // (ERRORS.md row 2), and `value` is rewritten each pass so it cannot be
            // predicted — so -1 is excluded from the rank domain. Every other
            // arbitrary rank (decoupled from the index) is exercised.
            let mut rk = rng.spicy_i32();
            if rk == -1 {
                rk = -3;
            }
            start.set_rank(i, rk);
        }
        start.set_count(count);
        let n = rng.below(4) + 1;
        let seq: Vec<usize> = (0..n).map(|_| rng.below(4)).collect();

        let mut cb = start.clone();
        let mut rb = start.clone();
        let ops_c = p.c.ops();
        let ops_r = p.rs.ops();
        for &oi in &seq {
            let (ct, rt) = unsafe {
                (
                    (p.c.process_with_foreach)(cb.as_ptr(), Some(ops_c[oi])),
                    (p.rs.process_with_foreach)(rb.as_ptr(), Some(ops_r[oi])),
                )
            };
            assert_eq!(ct, rt, "hand-built process total (count={count}, op={oi})");
        }
        assert_bufs_eq(&format!("hand-built count={count}"), &cb, &rb);
    }
}

// ===========================================================================
// Rows 29-34: compute_weighted_sum
// ===========================================================================

#[test] // rows 29 + 30 + 31
fn row29_31_weighted_sum_counts() {
    let p = pair();
    let mut rng = Rng::seeded();
    for count in 0..=10 {
        for _ in 0..5_000 {
            let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
            let start = ArrBuf::poisoned(&mut rng);
            let mut cb = start.clone();
            let mut rb = start.clone();
            let mut cv = vals.clone();
            let mut rv = vals.clone();
            let (cs, rs) = unsafe {
                (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), count);
                (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), count);
                (
                    (p.c.compute_weighted_sum)(cb.as_ptr()),
                    (p.rs.compute_weighted_sum)(rb.as_ptr()),
                )
            };
            assert_eq!(cs, rs, "compute_weighted_sum count={count} vals={vals:?}");
            // must be read-only
            assert_bufs_eq(&format!("weighted_sum count={count} (read-only)"), &cb, &rb);
        }
    }
}

#[test] // row 32
fn row32_weighted_sum_boundary_values() {
    let p = pair();
    let mut rng = Rng::seeded();
    let edges: [c_int; 9] = [
        0,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
    ];
    // an extreme value at each individual index (so it meets each weight)
    for &e in &edges {
        for idx in 0..CAP {
            for count in (idx as c_int + 1)..=10 {
                let mut start = ArrBuf::zeroed();
                for i in 0..CAP {
                    start.set_value(i, if i == idx { e } else { 1 });
                    start.set_scaled(i, 0.0);
                    start.set_rank(i, i as c_int);
                }
                start.set_count(count);
                let mut cb = start.clone();
                let mut rb = start.clone();
                let (cs, rs) = unsafe {
                    (
                        (p.c.compute_weighted_sum)(cb.as_ptr()),
                        (p.rs.compute_weighted_sum)(rb.as_ptr()),
                    )
                };
                assert_eq!(cs, rs, "weighted_sum e={e} idx={idx} count={count}");
                assert_bufs_eq("weighted_sum boundary", &cb, &rb);
            }
        }
    }
    for _ in 0..20_000 {
        let count = rng.below(11) as c_int;
        let mut start = ArrBuf::poisoned(&mut rng);
        for i in 0..CAP {
            start.set_value(i, edges[rng.below(edges.len())]);
        }
        start.set_count(count);
        let mut cb = start.clone();
        let mut rb = start.clone();
        let (cs, rs) = unsafe {
            (
                (p.c.compute_weighted_sum)(cb.as_ptr()),
                (p.rs.compute_weighted_sum)(rb.as_ptr()),
            )
        };
        assert_eq!(cs, rs, "weighted_sum random edges count={count}");
    }
}

#[test] // row 33
fn row33_weighted_sum_hand_built_reads_only_value() {
    let p = pair();
    let mut rng = Rng::seeded();
    for _ in 0..20_000 {
        let count = rng.below(11) as c_int;
        let mut start = ArrBuf::poisoned(&mut rng);
        for i in 0..CAP {
            start.set_value(i, rng.spicy_i32());
            start.set_scaled(i, rng.spicy_f64());
            start.set_rank(i, rng.spicy_i32());
        }
        start.set_count(count);
        let mut cb = start.clone();
        let mut rb = start.clone();
        let (cs, rs) = unsafe {
            (
                (p.c.compute_weighted_sum)(cb.as_ptr()),
                (p.rs.compute_weighted_sum)(rb.as_ptr()),
            )
        };
        assert_eq!(cs, rs, "weighted_sum hand-built count={count}");
        assert_bufs_eq("weighted_sum hand-built", &cb, &rb);
    }
}

#[test] // row 34
fn row34_weighted_sum_after_process() {
    let p = pair();
    let mut rng = Rng::seeded();
    for _ in 0..10_000 {
        let count = rng.below(11) as c_int;
        let vals: Vec<c_int> = (0..16).map(|_| rng.spicy_i32()).collect();
        let start = ArrBuf::poisoned(&mut rng);
        let mut cb = start.clone();
        let mut rb = start.clone();
        let mut cv = vals.clone();
        let mut rv = vals.clone();
        let ops_c = p.c.ops();
        let ops_r = p.rs.ops();
        let n = rng.below(5);
        let seq: Vec<usize> = (0..n).map(|_| rng.below(4)).collect();
        unsafe {
            (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), count);
            (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), count);
            for &oi in &seq {
                let ct = (p.c.process_with_foreach)(cb.as_ptr(), Some(ops_c[oi]));
                let rt = (p.rs.process_with_foreach)(rb.as_ptr(), Some(ops_r[oi]));
                assert_eq!(ct, rt, "pipeline process total");
            }
            let cs = (p.c.compute_weighted_sum)(cb.as_ptr());
            let rs = (p.rs.compute_weighted_sum)(rb.as_ptr());
            assert_eq!(cs, rs, "pipeline weighted_sum count={count} seq={seq:?}");
        }
        assert_bufs_eq("pipeline", &cb, &rb);
    }
}

// ===========================================================================
// Rows 35-36: compare_results_in_array (valid indices)
// ===========================================================================

#[test] // row 35
fn row35_compare_all_in_range_pairs() {
    let p = pair();
    let mut rng = Rng::seeded();
    for count in 1..=10i32 {
        let mut start = ArrBuf::poisoned(&mut rng);
        start.set_count(count);
        let mut cb = start.clone();
        let mut rb = start.clone();
        for i1 in 0..count {
            for i2 in 0..count {
                let (cv, rv) = unsafe {
                    (
                        (p.c.compare_results_in_array)(cb.as_ptr(), i1, i2),
                        (p.rs.compare_results_in_array)(rb.as_ptr(), i1, i2),
                    )
                };
                assert_eq!(cv, rv, "compare(count={count}, {i1}, {i2})");
                let want = (i1 as i64).cmp(&(i2 as i64)) as i32;
                assert_eq!(cv, want, "compare sanity (count={count}, {i1}, {i2})");
            }
        }
        assert_bufs_eq("compare is read-only", &cb, &rb);
    }
}

#[test] // row 36
fn row36_compare_arrayfunc_sweep() {
    let p = pair();
    let mut rng = Rng::seeded();
    for _ in 0..2_000 {
        let mut start = ArrBuf::poisoned(&mut rng);
        start.set_count(8);
        let mut cb = start.clone();
        let mut rb = start.clone();
        let mut csum = 0i32;
        let mut rsum = 0i32;
        for i in 0..7 {
            unsafe {
                csum += (p.c.compare_results_in_array)(cb.as_ptr(), i, i + 1);
                rsum += (p.rs.compare_results_in_array)(rb.as_ptr(), i, i + 1);
            }
        }
        assert_eq!(csum, rsum);
        assert_eq!(csum, -7, "arrayfunc's fixed comparison contribution");
    }
}

// ===========================================================================
// Rows 37-40: arrayfunc (the public header entry point)
// ===========================================================================

fn diff_arrayfunc(ctx: &str, a: c_int, b: c_int, c: c_int, d: c_int) {
    let p = pair();
    let (cv, rv) = unsafe { ((p.c.arrayfunc)(a, b, c, d), (p.rs.arrayfunc)(a, b, c, d)) };
    assert_eq!(cv, rv, "[{ctx}] arrayfunc({a}, {b}, {c}, {d}) C={cv} Rust={rv}");
}

#[test] // row 37
fn row37_arrayfunc_random() {
    let mut rng = Rng::seeded();
    for _ in 0..200_000 {
        diff_arrayfunc(
            "random",
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
    }
    let mut rng2 = Rng::new(0xDEAD_BEEF_1234_5678);
    for _ in 0..200_000 {
        diff_arrayfunc("uniform", rng2.i32(), rng2.i32(), rng2.i32(), rng2.i32());
    }
}

#[test] // row 38
fn row38_arrayfunc_boundary_cross_product() {
    const E: [c_int; 9] = [
        0,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX / 2,
        i32::MIN / 2,
    ];
    for &a in &E {
        for &b in &E {
            for &c in &E {
                for &d in &E {
                    diff_arrayfunc("boundary", a, b, c, d);
                }
            }
        }
    }
}

#[test] // row 39
fn row39_arrayfunc_small_magnitude_exhaustive() {
    for a in -8..=8 {
        for b in -8..=8 {
            for c in -8..=8 {
                for d in -8..=8 {
                    diff_arrayfunc("small", a, b, c, d);
                }
            }
        }
    }
}

#[test] // row 40
fn row40_arrayfunc_param4_division_truncation() {
    let mut rng = Rng::seeded();
    for d in -64..=64 {
        for _ in 0..40 {
            diff_arrayfunc("p4 trunc", rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), d);
        }
    }
    for d in [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -1,
        -3,
        1,
        3,
        -0x4000_0000,
        0x4000_0000,
    ] {
        for _ in 0..500 {
            diff_arrayfunc("p4 edge", rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), d);
        }
    }
}

// ===========================================================================
// Row 41: struct layout agreement
// ===========================================================================

#[test] // row 41
fn row41_struct_layout_agreement() {
    assert_eq!(std::mem::size_of::<CResult>(), RESULT_SIZE);
    assert_eq!(std::mem::size_of::<CResultArray>(), RESULT_ARRAY_SIZE);

    let p = pair();
    let mut rng = Rng::seeded();
    // Write a distinct marker into every element via init_result_array, then check
    // both libraries agree on where each field lives by reading the bytes back.
    let vals: Vec<c_int> = (0..16).map(|i| 0x0100_0000 * (i as c_int + 1)).collect();
    let start = ArrBuf::poisoned(&mut rng);
    let mut cb = start.clone();
    let mut rb = start.clone();
    let mut cv = vals.clone();
    let mut rv = vals.clone();
    unsafe {
        (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), 10);
        (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), 10);
    }
    assert_bufs_eq("layout markers", &cb, &rb);
    for i in 0..CAP {
        assert_eq!(cb.value(i), vals[i], "value @ offset 0");
        assert_eq!(cb.rank(i), i as c_int, "rank @ offset 16");
        assert_eq!(
            f64::from_bits(cb.scaled_bits(i)),
            vals[i] as f64 * 1.5,
            "scaled @ offset 8"
        );
    }
    assert_eq!(cb.get_count(), 10, "count @ offset 240");
    // padding must be byte-identical too (both must leave the holes untouched)
    for i in 0..CAP {
        for off in [4, 5, 6, 7, 20, 21, 22, 23] {
            let b = i * RESULT_SIZE + off;
            assert_eq!(
                cb.bytes[b], start.bytes[b],
                "C wrote padding byte {off} of data[{i}]"
            );
            assert_eq!(
                rb.bytes[b], start.bytes[b],
                "Rust wrote padding byte {off} of data[{i}]"
            );
        }
    }
}

// ===========================================================================
// Rows 42-43: `count` beyond the 10-element capacity.
//
// `init_result_array` clamps to 10, but NOTHING stops a caller from writing
// `arr->count` directly — and `process_with_foreach` / `compute_weighted_sum` then
// walk straight past `data[10]`. The C does this happily; these rows pin the
// behaviour down instead of leaving the deepest path untested.
//
// `ArrBuf` over-allocates SLACK_ELEMS elements past the struct, so every access
// the C makes stays inside memory the test owns: the comparison is meaningful
// rather than a race with the allocator. This also drives `compute_weighted_sum`'s
// `weight` above 9, which no `init_result_array`-built array can reach.
// ===========================================================================

const OVER_MAX: c_int = (CAP + SLACK_ELEMS - 1) as c_int;

#[test] // row 42
fn row42_weighted_sum_count_past_capacity() {
    let p = pair();
    let mut rng = Rng::seeded();
    for count in 11..=OVER_MAX {
        for _ in 0..40 {
            let mut start = ArrBuf::poisoned(&mut rng);
            // give every in-range element a well-defined value
            for i in 0..count as usize {
                start.set_value(i, rng.spicy_i32());
                start.set_scaled(i, rng.spicy_f64());
                start.set_rank(i, i as c_int);
            }
            start.set_count(count);
            let mut cb = start.clone();
            let mut rb = start.clone();
            let (cs, rs) = unsafe {
                (
                    (p.c.compute_weighted_sum)(cb.as_ptr()),
                    (p.rs.compute_weighted_sum)(rb.as_ptr()),
                )
            };
            assert_eq!(cs, rs, "weighted_sum count={count} (past capacity)");
            assert_bufs_eq(&format!("weighted_sum count={count} read-only"), &cb, &rb);
        }
    }
    // extreme values at large weights: the association of
    // `value * weight * 0.8` now matters for weights well past 9
    for count in [11i32, 20, 33, 50, OVER_MAX] {
        for v in [i32::MAX, i32::MIN, i32::MAX / 3, i32::MIN / 3, 5, -5, 12345] {
            let mut start = ArrBuf::zeroed();
            for i in 0..count as usize {
                start.set_value(i, v);
                start.set_rank(i, i as c_int);
            }
            start.set_count(count);
            let mut cb = start.clone();
            let mut rb = start.clone();
            let (cs, rs) = unsafe {
                (
                    (p.c.compute_weighted_sum)(cb.as_ptr()),
                    (p.rs.compute_weighted_sum)(rb.as_ptr()),
                )
            };
            assert_eq!(cs, rs, "weighted_sum count={count} v={v}");
        }
    }
}

#[test] // row 43
fn row43_process_single_pass_count_past_capacity() {
    let p = pair();
    let mut rng = Rng::seeded();
    // IMPORTANT: exactly ONE pass. `Result` is 24 bytes and `count` sits at offset
    // 240 == 10 * 24, so `data[10].value` ALIASES the `count` field. With
    // count > 10 the loop's 11th iteration overwrites `count` with
    // `safe_double_to_int(result * 0.75)` — often INT32_MAX. The loop itself
    // survives (the FOREACH macro snapshots `size` once), but any SUBSEQUENT call
    // re-reads the corrupted `count` and walks off the end of the buffer: verified
    // experimentally, a second pass SIGSEGVs. That is real C behaviour, and it is
    // exactly what `init_result_array`'s clamp to 10 exists to prevent, so a
    // single pass is the deepest well-defined probe here.
    for count in 11..=OVER_MAX {
        for _ in 0..25 {
            let mut start = ArrBuf::poisoned(&mut rng);
            for i in 0..count as usize {
                start.set_value(i, rng.spicy_i32());
                start.set_scaled(i, rng.spicy_f64());
                // rank is the op's `b`; -1 would SIGFPE the C modulo once some
                // pass leaves INT32_MIN in `value` (ERRORS.md row 2).
                let mut rk = rng.spicy_i32();
                if rk == -1 {
                    rk = -3;
                }
                start.set_rank(i, rk);
            }
            start.set_count(count);
            let ops_c = p.c.ops();
            let ops_r = p.rs.ops();
            for oi in 0..4 {
                let mut cb = start.clone();
                let mut rb = start.clone();
                let (ct, rt) = unsafe {
                    (
                        (p.c.process_with_foreach)(cb.as_ptr(), Some(ops_c[oi])),
                        (p.rs.process_with_foreach)(rb.as_ptr(), Some(ops_r[oi])),
                    )
                };
                assert_eq!(ct, rt, "process count={count} op={}", OP_NAMES[oi]);
                // Includes the aliased `count` field, so the two libraries must
                // corrupt it identically too.
                assert_bufs_eq(
                    &format!("process count={count} op={} past capacity", OP_NAMES[oi]),
                    &cb,
                    &rb,
                );
            }
        }
    }
}

#[test] // row 44 — the data[10]/count aliasing, pinned down explicitly
fn row44_element_ten_aliases_the_count_field() {
    let p = pair();
    // Offset arithmetic that makes the aliasing inevitable.
    assert_eq!(RESULT_SIZE * CAP, COUNT_OFFSET);

    for count in [11i32, 12, 15, 20] {
        let mut start = ArrBuf::zeroed();
        for i in 0..count as usize {
            start.set_value(i, 100 + i as c_int);
            start.set_rank(i, i as c_int);
        }
        start.set_count(count);
        // `set_count` and `set_value(10, ..)` write the SAME four bytes; the last
        // writer wins, so `data[10].value` reads back as `count`.
        assert_eq!(start.value(CAP), count, "data[10].value aliases count");

        let mut cb = start.clone();
        let mut rb = start.clone();
        let (ct, rt) = unsafe {
            (
                (p.c.process_with_foreach)(cb.as_ptr(), Some(p.c.add_operation)),
                (p.rs.process_with_foreach)(rb.as_ptr(), Some(p.rs.add_operation)),
            )
        };
        assert_eq!(ct, rt, "aliasing total count={count}");
        assert_bufs_eq(&format!("aliasing count={count}"), &cb, &rb);
        // Both must have corrupted `count` to the same value.
        assert_eq!(
            cb.get_count(),
            rb.get_count(),
            "the clobbered count must match"
        );
        assert_ne!(
            cb.get_count(),
            count,
            "iteration 10 should have overwritten count"
        );
    }
}

#[test] // row 45 — read-only entry points at count > 10
fn row45_compare_count_past_capacity_readonly() {
    let p = pair();
    let mut rng = Rng::seeded();
    // `compare_results_in_array` never writes, so a large `count` is stable.
    for _ in 0..2_000 {
        let count = (11 + rng.below((SLACK_ELEMS - 2) as usize)) as c_int;
        let mut start = ArrBuf::poisoned(&mut rng);
        start.set_count(count);
        let mut cb = start.clone();
        let mut rb = start.clone();
        for _ in 0..20 {
            let i1 = rng.below(count as usize + 4) as c_int - 2;
            let i2 = rng.below(count as usize + 4) as c_int - 2;
            let (cv, rv) = unsafe {
                (
                    (p.c.compare_results_in_array)(cb.as_ptr(), i1, i2),
                    (p.rs.compare_results_in_array)(rb.as_ptr(), i1, i2),
                )
            };
            assert_eq!(cv, rv, "compare(count={count}, {i1}, {i2})");
        }
        assert_bufs_eq("compare past capacity is read-only", &cb, &rb);
    }
}
