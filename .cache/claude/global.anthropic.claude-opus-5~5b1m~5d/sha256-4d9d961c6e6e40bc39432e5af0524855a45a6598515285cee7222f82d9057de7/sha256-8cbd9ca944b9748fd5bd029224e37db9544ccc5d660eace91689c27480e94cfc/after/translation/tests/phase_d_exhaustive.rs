//! Phase D — wide brute-force sweeps, on top of the per-row tests.
//!
//! Mode 3 is the only arm of the shipped `.so` that actually computes, and its
//! result depends on the *decimal rendering* of two `int`s (the Rust side
//! reimplements `sprintf`/`strlen`). That makes it the highest-risk surface, so
//! it gets a dense sweep rather than sampling.

mod common;

use common::*;

/// Dense sweep of mode 3 over a contiguous block of node_id x depth, which
/// covers every digit-count transition and both signs.
#[test]
fn exhaustive_mode3_dense_grid() {
    let _g = common::lock_state();
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for id in -600i32..=600 {
        for depth in -600i32..=600 {
            let gc = unsafe { c(3, id, depth, 0) };
            let gr = unsafe { rs(3, id, depth, 0) };
            assert_eq!(gc, gr, "jumpnode(3, {id}, {depth}, 0)");
        }
    }
}

/// Mode 3 across every power-of-ten neighbourhood up to i32 range, both signs,
/// with the flag mask varied — this is where a `%d` emulation bug would hide.
#[test]
fn exhaustive_mode3_decade_neighbourhoods() {
    let _g = common::lock_state();
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");

    let mut points: Vec<i32> = Vec::new();
    let mut pow: i64 = 1;
    while pow <= 10_000_000_000i64 {
        for delta in -3i64..=3 {
            for sign in [1i64, -1] {
                let v = sign * (pow + delta);
                if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                    points.push(v as i32);
                }
            }
        }
        pow *= 10;
    }
    points.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0]);
    points.sort_unstable();
    points.dedup();

    for &id in &points {
        for &depth in &points {
            for flags in [0i32, 1, 127, 128, -1, i32::MIN, i32::MAX] {
                let gc = unsafe { c(3, id, depth, flags) };
                let gr = unsafe { rs(3, id, depth, flags) };
                assert_eq!(gc, gr, "jumpnode(3, {id}, {depth}, {flags})");
            }
        }
    }
}

/// Dense sweep of every operation_mode over a wide window, on the shipped `.so`.
#[test]
fn exhaustive_all_modes_dense() {
    let _g = common::lock_state();
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for mode in -50i32..=50 {
        for id in -50i32..=50 {
            for depth in [-17i32, -1, 0, 1, 15, 16, 17, 100] {
                for flags in [0i32, 1, -1, 127, 128, 65535] {
                    let gc = unsafe { c(mode, id, depth, flags) };
                    let gr = unsafe { rs(mode, id, depth, flags) };
                    assert_eq!(gc, gr, "jumpnode({mode}, {id}, {depth}, {flags})");
                }
            }
        }
    }
}

/// The populated surface, swept densely over the whole live id range and every
/// in-bounds depth, for all four modes.
#[test]
fn exhaustive_populated_all_modes() {
    let _g = common::lock_state();
    let Some(p) = Pair::instrumented() else {
        eprintln!("skipped: Rust .so built without `expose_init_test_data`");
        return;
    };
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    unsafe {
        ic();
        irs();
    }
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for mode in 1i32..=4 {
        for id in -10i32..=20 {
            // mode 2 with depth < 0 walks off the array in the C (UB), so the
            // sweep starts at 0 for that arm only.
            let lo = if mode == 2 { 0 } else { -40 };
            for depth in lo..=40 {
                for flags in [0i32, 1, -1, 127, 1000, -1000] {
                    let gc = unsafe { c(mode, id, depth, flags) };
                    let gr = unsafe { rs(mode, id, depth, flags) };
                    assert_eq!(gc, gr, "jumpnode({mode}, {id}, {depth}, {flags})");
                }
            }
        }
    }
}

/// `safe_double_to_int` over a very dense grid around both clamp thresholds,
/// stepping by single ULPs.
#[test]
fn exhaustive_clamp_ulps() {
    let _g = common::lock_state();
    let Some(p) = Pair::instrumented() else {
        eprintln!("skipped: Rust .so built without `expose_init_test_data`");
        return;
    };
    let (cd, rd) = p.sym::<D2IFn>(b"jumpnode_test_safe_double_to_int\0");
    for center in [2147483647.0f64, -2147483648.0, 0.0, 1.0, -1.0] {
        let mut v = center;
        for _ in 0..200 {
            v = next_up(v);
        }
        for _ in 0..400 {
            assert_eq!(unsafe { cd(v) }, unsafe { rd(v) }, "ulp v={v:?}");
            v = next_down(v);
        }
    }
    // Every integer-valued double in a window around the high clamp.
    for k in -300i64..=300 {
        let v = 2147483647.0f64 + k as f64;
        assert_eq!(unsafe { cd(v) }, unsafe { rd(v) }, "high window v={v}");
        let v = -2147483648.0f64 + k as f64;
        assert_eq!(unsafe { cd(v) }, unsafe { rd(v) }, "low window v={v}");
    }
}

fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    let bits = x.to_bits();
    if x == 0.0 {
        return f64::from_bits(1);
    }
    if x > 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

fn next_down(x: f64) -> f64 {
    if x.is_nan() || x == f64::NEG_INFINITY {
        return x;
    }
    let bits = x.to_bits();
    if x == 0.0 {
        return -f64::from_bits(1);
    }
    if x > 0.0 {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

/// DOCUMENTATION test for ERRORS.md row U3.
///
/// `(int)NaN` is undefined behaviour in C, so `safe_double_to_int(NaN)` is the
/// one input where C and Rust are permitted to disagree. This test does not
/// assert equality there — it records what each side actually does, and then
/// asserts the property that matters: NaN is **unreachable** from every public
/// entry point, so a caller of `jumpnode` can never observe the difference.
#[test]
fn nan_divergence_is_unreachable_from_public_api() {
    let _g = common::lock_state();
    let Some(p) = Pair::instrumented() else {
        eprintln!("skipped: Rust .so built without `expose_init_test_data`");
        return;
    };
    let (cd, rd) = p.sym::<D2IFn>(b"jumpnode_test_safe_double_to_int\0");
    for v in [f64::NAN, -f64::NAN, f64::from_bits(0x7ff8_0000_0000_0001)] {
        let (gc, gr) = (unsafe { cd(v) }, unsafe { rd(v) });
        eprintln!(
            "NaN probe bits={:#x}: C={gc} Rust={gr} (the C is UB here)",
            v.to_bits()
        );
    }

    // Unreachability. `safe_double_to_int` is only ever fed:
    //   * mode 1: a sum of stored `value`s scaled by 1.5;
    //   * mode 4: sqrt(data[i]) * 2.718... scaled by (1 + depth*0.1);
    // and `add_node` hardcodes data[] = {0100,0200,0300,0400}, so sqrt never
    // sees a negative argument and no inf-inf can arise from finite values.
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    unsafe {
        ic();
        irs();
    }
    let (cn, rn) = p.sym::<GetNodeFn>(b"jumpnode_test_get_node\0");
    for i in 0..7 {
        let mut a = (0i32, 0i32, 0f64, [0i32; 4]);
        let mut b = (0i32, 0i32, 0f64, [0i32; 4]);
        unsafe {
            cn(i, &mut a.0, &mut a.1, &mut a.2, a.3.as_mut_ptr());
            rn(i, &mut b.0, &mut b.1, &mut b.2, b.3.as_mut_ptr());
        }
        assert!(a.2.is_finite(), "stored value {i} must be finite");
        assert_eq!(a.3, [0o100, 0o200, 0o300, 0o400], "data[] is constant");
        assert_eq!(a.2.to_bits(), b.2.to_bits());
        assert_eq!(a.3, b.3);
    }

    // And the whole public surface still agrees.
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 0xA5A5);
    for _ in 0..20000 {
        let mode = r.range(1, 4);
        let id = r.range(-3, 12);
        let depth = if mode == 2 {
            r.range(0, 40)
        } else {
            r.interesting_i32()
        };
        let flags = r.range(-100_000_000, 100_000_000);
        let gc = unsafe { c(mode, id, depth, flags) };
        assert_eq!(
            gc,
            unsafe { rs(mode, id, depth, flags) },
            "({mode},{id},{depth},{flags})"
        );
    }
}
