//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both libraries are driven exclusively through `libloading` symbols.

mod common;

use common::*;

const BOUNDARY: [i32; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -65536,
    -1,
    0,
    1,
    65536,
    i32::MAX - 1,
    i32::MAX,
];

// ---------------------------------------------------------------------------
// C1..C4 — the four low-level `operation_func` implementations
// ---------------------------------------------------------------------------

/// `modulo_operation(INT32_MIN, -1, _, _)` compiles to `idivl`, whose quotient
/// overflows, so the C library raises `SIGFPE` and returns no value at all
/// (verified: a C driver linked against the C `.so` exits with status 136).
/// There is no value to compare, so this one UB input is skipped — see the
/// "deliberately NOT tested" section of `ERRORS.md` (row E2).
fn is_idiv_trap(is_modulo: bool, a: i32, b: i32) -> bool {
    is_modulo && a == i32::MIN && b == -1
}

fn binop_row(
    row: &str,
    call: impl Fn(&Lib, i32, i32, i32, i32) -> i32,
    is_modulo: bool,
) {
    let p = libs();
    let mut rng = Rng::new(SEED ^ row.len() as u64);

    // boundary cross-product
    for &a in BOUNDARY.iter() {
        for &b in BOUNDARY.iter() {
            if is_idiv_trap(is_modulo, a, b) {
                continue;
            }
            let (u1, u2) = (rng.next_i32(), rng.next_i32());
            eq_i32(
                &format!("{row} boundary a={a} b={b}"),
                call(&p.c, a, b, u1, u2),
                call(&p.rs, a, b, u1, u2),
            );
        }
    }
    // randomized
    for it in 0..20_000u32 {
        let a = rng.mixed_i32();
        let mut b = rng.mixed_i32();
        if is_modulo && rng.below(8) == 0 {
            b = 0;
        }
        if is_idiv_trap(is_modulo, a, b) {
            continue;
        }
        // unused params are randomized to prove they really are ignored
        let (u1, u2) = (rng.next_i32(), rng.next_i32());
        eq_i32(
            &format!("{row} it={it} a={a} b={b} u1={u1} u2={u2}"),
            call(&p.c, a, b, u1, u2),
            call(&p.rs, a, b, u1, u2),
        );
    }
}

#[test]
fn cfg_c1_add_operation() {
    binop_row("C1 add", |l, a, b, u1, u2| l.add(a, b, u1, u2), false);
}

#[test]
fn cfg_c2_multiply_operation() {
    binop_row("C2 mul", |l, a, b, u1, u2| l.mul(a, b, u1, u2), false);
    // overflow-heavy: large x large
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xC2);
    for it in 0..20_000u32 {
        let a = rng.next_i32() | 0x4000_0000;
        let b = rng.next_i32() | 0x4000_0000;
        eq_i32(
            &format!("C2 mul overflow it={it} a={a} b={b}"),
            p.c.mul(a, b, 0, 0),
            p.rs.mul(a, b, 0, 0),
        );
    }
}

#[test]
fn cfg_c3_subtract_operation() {
    binop_row("C3 sub", |l, a, b, u1, u2| l.sub(a, b, u1, u2), false);
}

#[test]
fn cfg_c4_modulo_operation() {
    binop_row("C4 mod", |l, a, b, u1, u2| l.modulo(a, b, u1, u2), true);
    let p = libs();
    // sign matrix + the INT_MIN % -1 case
    for &a in BOUNDARY.iter() {
        for &b in [-7i32, -2, -1, 0, 1, 2, 7, i32::MIN, i32::MAX].iter() {
            if is_idiv_trap(true, a, b) {
                continue;
            }
            eq_i32(
                &format!("C4 mod signs a={a} b={b}"),
                p.c.modulo(a, b, 0, 0),
                p.rs.modulo(a, b, 0, 0),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C5, C6 — safe_double_to_int
// ---------------------------------------------------------------------------

#[test]
fn cfg_c5_sdti_random_bits() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xC5);
    for it in 0..30_000u32 {
        let d = if it % 2 == 0 { rng.any_f64() } else { rng.mixed_f64() };
        eq_i32(
            &format!("C5 sdti it={it} bits={:#018x} d={d:?}", d.to_bits()),
            p.c.sdti(d),
            p.rs.sdti(d),
        );
    }
}

fn sdti_boundary_values() -> Vec<f64> {
    let imax = i32::MAX as f64; //  2147483647.0 (exact)
    let imin = i32::MIN as f64; // -2147483648.0 (exact)
    let mut v = vec![
        imax,
        imin,
        imin - 1.0,
        imin + 1.0,
        imax - 1.0,
        imax + 1.0,
        f64::from_bits(imax.to_bits() - 1), // nextafter(IMAX, 0)
        f64::from_bits(imax.to_bits() + 1), // nextafter(IMAX, +inf)
        f64::from_bits(imin.to_bits() - 1), // nextafter(IMIN, 0)
        f64::from_bits(imin.to_bits() + 1), // nextafter(IMIN, -inf)
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),  // smallest subnormal
        f64::from_bits(0x8000_0000_0000_0001),
        0.5,
        -0.5,
        1.5,
        -1.5,
        -0.9999999999,
        0.9999999999,
        2147483646.5,
        -2147483647.5,
        f64::MAX,
        f64::MIN,
        1e308,
        -1e308,
        1e-308,
    ];
    for k in 0..32 {
        v.push((1u64 << k) as f64);
        v.push(-((1u64 << k) as f64));
        v.push((1u64 << k) as f64 + 0.5);
        v.push(-((1u64 << k) as f64) - 0.5);
    }
    v
}

#[test]
fn cfg_c6_sdti_boundaries() {
    let p = libs();
    for d in sdti_boundary_values() {
        eq_i32(
            &format!("C6 sdti boundary bits={:#018x} d={d:?}", d.to_bits()),
            p.c.sdti(d),
            p.rs.sdti(d),
        );
    }
}

// ---------------------------------------------------------------------------
// C7 — compute_scaled_value
// ---------------------------------------------------------------------------

#[test]
fn cfg_c7_compute_scaled_value() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xC7);
    for &base in BOUNDARY.iter() {
        for s in [0.0f64, -0.0, 1.0, -1.0, 1.5, 0.75, 0.333, 1e10, -1e10, f64::NAN,
                  f64::INFINITY, f64::NEG_INFINITY, f64::MIN_POSITIVE]
        {
            eq_i32(
                &format!("C7 csv base={base} scale={s:?}"),
                p.c.compute_scaled_value(base, s),
                p.rs.compute_scaled_value(base, s),
            );
        }
    }
    for it in 0..20_000u32 {
        let base = rng.mixed_i32();
        let s = rng.mixed_f64();
        eq_i32(
            &format!("C7 csv it={it} base={base} scale bits={:#018x}", s.to_bits()),
            p.c.compute_scaled_value(base, s),
            p.rs.compute_scaled_value(base, s),
        );
    }
}

// ---------------------------------------------------------------------------
// C8..C10 — init_result_array
// ---------------------------------------------------------------------------

fn diff_init(ctx: &str, values: &[i32], count: i32) {
    let p = libs();
    let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
    let (mut vc, mut vrs) = (values.to_vec(), values.to_vec());
    p.c.init(&mut ac, &mut vc, count);
    p.rs.init(&mut ars, &mut vrs, count);
    eq_state(ctx, &ac, &ars);
    assert_eq!(vc, vrs, "values[] was modified differently [{ctx}]");
}

#[test]
fn cfg_c8_init_count_sweep() {
    let mut rng = Rng::new(SEED ^ 0xC8);
    for count in 0..=12i32 {
        for it in 0..500u32 {
            // always supply 16 values so an unclamped read would still be in-bounds
            let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
            diff_init(&format!("C8 count={count} it={it}"), &values, count);
        }
    }
}

#[test]
fn cfg_c9_init_clamp_boundary() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xC9);
    for count in [9i32, 10, 11, 12, 100, i32::MAX] {
        for it in 0..200u32 {
            let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
            let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
            let (mut vc, mut vrs) = (values.clone(), values.clone());
            p.c.init(&mut ac, &mut vc, count);
            p.rs.init(&mut ars, &mut vrs, count);
            eq_state(&format!("C9 count={count} it={it}"), &ac, &ars);
            let expected = if count < 10 { count } else { 10 };
            assert_eq!(ac.count, expected, "C clamp changed? count={count}");
            assert_eq!(ars.count, expected, "Rust clamp wrong; count={count}");
        }
    }
}

#[test]
fn cfg_c10_init_extreme_values() {
    let mut rng = Rng::new(SEED ^ 0xCA);
    for it in 0..2_000u32 {
        let count = 1 + (rng.below(10) as i32);
        let values: Vec<i32> = (0..16)
            .map(|_| match rng.below(3) {
                0 => i32::MIN,
                1 => i32::MAX,
                _ => rng.mixed_i32(),
            })
            .collect();
        diff_init(&format!("C10 count={count} it={it}"), &values, count);
    }
}

// ---------------------------------------------------------------------------
// C11 — compare_results_in_array
// ---------------------------------------------------------------------------

#[test]
fn cfg_c11_compare_index_matrix() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xCB);
    for it in 0..200u32 {
        let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
        for count in 0..=10i32 {
            let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
            let (mut vc, mut vrs) = (values.clone(), values.clone());
            p.c.init(&mut ac, &mut vc, count);
            p.rs.init(&mut ars, &mut vrs, count);
            for i1 in -2..=12i32 {
                for i2 in -2..=12i32 {
                    let ctx = format!("C11 it={it} count={count} i1={i1} i2={i2}");
                    eq_i32(
                        &ctx,
                        p.c.compare(&mut ac, i1, i2),
                        p.rs.compare(&mut ars, i1, i2),
                    );
                }
            }
            eq_state(&format!("C11 it={it} count={count} (state)"), &ac, &ars);
        }
    }
}

// ---------------------------------------------------------------------------
// C12..C15 — process_with_foreach with each built-in op (each library uses its
// OWN op function pointer, exactly as `arrayfunc` does).
// ---------------------------------------------------------------------------

fn foreach_row(row: &str, which: usize) {
    let p = libs();
    let mut rng = Rng::new(SEED ^ (0xD0 + which as u64));
    for count in 0..=10i32 {
        for it in 0..400u32 {
            let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
            let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
            let (mut vc, mut vrs) = (values.clone(), values.clone());
            p.c.init(&mut ac, &mut vc, count);
            p.rs.init(&mut ars, &mut vrs, count);
            let ctx = format!("{row} count={count} it={it} values={values:?}");
            eq_i32(
                &ctx,
                p.c.foreach(&mut ac, p.c.own_op(which)),
                p.rs.foreach(&mut ars, p.rs.own_op(which)),
            );
            eq_state(&format!("{ctx} (state)"), &ac, &ars);
        }
    }
}

#[test]
fn cfg_c12_foreach_add() {
    foreach_row("C12 foreach/add", 0);
}

#[test]
fn cfg_c13_foreach_multiply() {
    foreach_row("C13 foreach/mul", 1);
}

#[test]
fn cfg_c14_foreach_subtract() {
    foreach_row("C14 foreach/sub", 2);
}

#[test]
fn cfg_c15_foreach_modulo() {
    foreach_row("C15 foreach/mod", 3);
}

// ---------------------------------------------------------------------------
// C16 — caller-supplied op: the SAME Rust fn pointer is handed to both
// libraries, and the per-call argument traces are compared.
// ---------------------------------------------------------------------------

static mut TRACE: Vec<[i32; 4]> = Vec::new();
static mut TRACE_RET: i32 = 0;

unsafe extern "C" fn tracing_op(a: i32, b: i32, u1: i32, u2: i32) -> i32 {
    #[allow(static_mut_refs)]
    TRACE.push([a, b, u1, u2]);
    // value-dependent, overflow-prone return so the op is not a constant
    TRACE_RET = a
        .wrapping_mul(3)
        .wrapping_add(b.wrapping_mul(-7))
        .wrapping_add(TRACE_RET ^ 0x1234_5678);
    TRACE_RET
}

fn run_traced(lib: &Lib, arr: &mut ResultArray) -> (i32, Vec<[i32; 4]>) {
    unsafe {
        #[allow(static_mut_refs)]
        TRACE.clear();
        TRACE_RET = 0;
        let r = lib.foreach(arr, Some(tracing_op));
        #[allow(static_mut_refs)]
        (r, TRACE.clone())
    }
}

#[test]
fn cfg_c16_foreach_custom_op_trace() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xD6);
    for count in 0..=10i32 {
        for it in 0..200u32 {
            let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
            let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
            let (mut vc, mut vrs) = (values.clone(), values.clone());
            p.c.init(&mut ac, &mut vc, count);
            p.rs.init(&mut ars, &mut vrs, count);
            let ctx = format!("C16 count={count} it={it}");
            let (rc, tc) = run_traced(&p.c, &mut ac);
            let (rr, tr) = run_traced(&p.rs, &mut ars);
            assert_eq!(tc, tr, "op call trace mismatch [{ctx}]");
            eq_i32(&ctx, rc, rr);
            eq_state(&format!("{ctx} (state)"), &ac, &ars);
        }
    }
}

// ---------------------------------------------------------------------------
// C17 — repeated application of process_with_foreach (state carries over)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c17_foreach_repeated_passes() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xD7);
    for which in 0..4usize {
        for count in 0..=10i32 {
            for it in 0..100u32 {
                let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
                let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
                let (mut vc, mut vrs) = (values.clone(), values.clone());
                p.c.init(&mut ac, &mut vc, count);
                p.rs.init(&mut ars, &mut vrs, count);
                for pass in 1..=4u32 {
                    let ctx =
                        format!("C17 op={which} count={count} it={it} pass={pass}");
                    eq_i32(
                        &ctx,
                        p.c.foreach(&mut ac, p.c.own_op(which)),
                        p.rs.foreach(&mut ars, p.rs.own_op(which)),
                    );
                    eq_state(&format!("{ctx} (state)"), &ac, &ars);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — compute_weighted_sum
// ---------------------------------------------------------------------------

#[test]
fn cfg_c18_weighted_sum_sweep() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xD8);
    for count in 0..=10i32 {
        for it in 0..800u32 {
            let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
            let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
            let (mut vc, mut vrs) = (values.clone(), values.clone());
            p.c.init(&mut ac, &mut vc, count);
            p.rs.init(&mut ars, &mut vrs, count);
            let ctx = format!("C18 count={count} it={it} values={values:?}");
            eq_i32(&ctx, p.c.weighted(&mut ac), p.rs.weighted(&mut ars));
            eq_state(&format!("{ctx} (state)"), &ac, &ars);
        }
    }
}

// ---------------------------------------------------------------------------
// C19, C20 — the composed pipeline driven through the LOW-LEVEL entry points
// ---------------------------------------------------------------------------

fn pipeline(ctx: &str, values: &[i32], count: i32, ops: &[usize]) {
    let p = libs();
    let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
    let (mut vc, mut vrs) = (values.to_vec(), values.to_vec());
    p.c.init(&mut ac, &mut vc, count);
    p.rs.init(&mut ars, &mut vrs, count);
    eq_state(&format!("{ctx} after init"), &ac, &ars);

    let mut rc: i32 = 0;
    let mut rr: i32 = 0;
    for (k, &w) in ops.iter().enumerate() {
        rc = rc.wrapping_add(p.c.foreach(&mut ac, p.c.own_op(w)));
        rr = rr.wrapping_add(p.rs.foreach(&mut ars, p.rs.own_op(w)));
        eq_i32(&format!("{ctx} after foreach#{k} op={w}"), rc, rr);
        eq_state(&format!("{ctx} after foreach#{k} op={w} (state)"), &ac, &ars);
    }

    rc = rc.wrapping_add(p.c.weighted(&mut ac));
    rr = rr.wrapping_add(p.rs.weighted(&mut ars));
    eq_i32(&format!("{ctx} after weighted"), rc, rr);
    eq_state(&format!("{ctx} after weighted (state)"), &ac, &ars);

    let mut i = 0i32;
    while i < ac.count.wrapping_sub(1) {
        let a = p.c.compare(&mut ac, i, i.wrapping_add(1));
        let b = p.rs.compare(&mut ars, i, i.wrapping_add(1));
        eq_i32(&format!("{ctx} compare i={i}"), a, b);
        rc = rc.wrapping_add(a);
        rr = rr.wrapping_add(b);
        i = i.wrapping_add(1);
    }

    eq_i32(
        &format!("{ctx} final sdti"),
        p.c.sdti(rc as f64 * 0.333),
        p.rs.sdti(rr as f64 * 0.333),
    );
}

#[test]
fn cfg_c19_manual_pipeline() {
    let mut rng = Rng::new(SEED ^ 0xD9);
    for count in 0..=10i32 {
        for it in 0..150u32 {
            let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
            pipeline(
                &format!("C19 count={count} it={it}"),
                &values,
                count,
                &[0, 1, 2, 3],
            );
        }
    }
}

#[test]
fn cfg_c20_pipeline_op_permutations() {
    let mut rng = Rng::new(SEED ^ 0xDA);
    for it in 0..300u32 {
        let count = rng.below(11) as i32;
        let n = 1 + rng.below(6) as usize;
        let ops: Vec<usize> = (0..n).map(|_| rng.below(4) as usize).collect();
        let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
        pipeline(
            &format!("C20 it={it} count={count} ops={ops:?}"),
            &values,
            count,
            &ops,
        );
    }
}

// ---------------------------------------------------------------------------
// C21..C23 — arrayfunc (the one-shot wrapper)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c21_arrayfunc_random() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xDB);
    for it in 0..100_000u32 {
        let (a, b, c, d) = (
            rng.mixed_i32(),
            rng.mixed_i32(),
            rng.mixed_i32(),
            rng.mixed_i32(),
        );
        eq_i32(
            &format!("C21 it={it} ({a},{b},{c},{d})"),
            p.c.arrayfunc(a, b, c, d),
            p.rs.arrayfunc(a, b, c, d),
        );
    }
}

#[test]
fn cfg_c22_arrayfunc_small_grid() {
    let p = libs();
    for a in -4..=4i32 {
        for b in -4..=4i32 {
            for c in -4..=4i32 {
                for d in -4..=4i32 {
                    eq_i32(
                        &format!("C22 ({a},{b},{c},{d})"),
                        p.c.arrayfunc(a, b, c, d),
                        p.rs.arrayfunc(a, b, c, d),
                    );
                }
            }
        }
    }
}

#[test]
fn cfg_c23_arrayfunc_boundary_grid() {
    let p = libs();
    const V: [i32; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in V.iter() {
        for &b in V.iter() {
            for &c in V.iter() {
                for &d in V.iter() {
                    eq_i32(
                        &format!("C23 ({a},{b},{c},{d})"),
                        p.c.arrayfunc(a, b, c, d),
                        p.rs.arrayfunc(a, b, c, d),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C24 — struct layout / ABI parity observed through the FFI
// ---------------------------------------------------------------------------

#[test]
fn cfg_c24_struct_layout_crosswrite() {
    assert_eq!(std::mem::size_of::<Result_>(), 24, "sizeof(Result)");
    assert_eq!(std::mem::align_of::<Result_>(), 8, "alignof(Result)");
    assert_eq!(std::mem::size_of::<ResultArray>(), 248, "sizeof(ResultArray)");
    assert_eq!(
        std::mem::offset_of!(ResultArray, count),
        240,
        "offsetof(ResultArray, count)"
    );
    assert_eq!(std::mem::offset_of!(Result_, value), 0);
    assert_eq!(std::mem::offset_of!(Result_, scaled), 8);
    assert_eq!(std::mem::offset_of!(Result_, rank), 16);

    let p = libs();
    // C writes the struct, Rust reads/continues it, and vice versa: if either
    // side disagreed about the layout the cross-driven pipeline would diverge.
    let mut rng = Rng::new(SEED ^ 0xDC);
    for it in 0..500u32 {
        let mut values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();

        let mut a = ResultArray::poisoned();
        p.c.init(&mut a, &mut values.clone(), 10);
        let after_c_init = a;
        // Rust continues on the buffer C initialised.
        let r_rust = p.rs.foreach(&mut a, p.rs.own_op(1));

        let mut b = after_c_init;
        let r_c = p.c.foreach(&mut b, p.c.own_op(1));

        eq_i32(&format!("C24 crosswrite it={it}"), r_c, r_rust);
        eq_state(&format!("C24 crosswrite it={it} (state)"), &b, &a);

        // and the mirror direction
        let mut a2 = ResultArray::poisoned();
        p.rs.init(&mut a2, &mut values, 10);
        eq_state(&format!("C24 init parity it={it}"), &after_c_init, &a2);
    }
}
