//! Phase C — error/rejection-path differential tests, one per row of
//! `ERRORS.md`, plus the generic FFI boundary cases.
//!
//! The C library has no error codes, no sentinels and no asserts (see
//! `ERRORS.md` for the grep that establishes this), so "same rejection" means:
//! the same fatal signal for the trapping input, and the same
//! returned-pointer-identity + same bytes for the degenerate and extreme
//! inputs. Both are asserted against the C `.so` as ground truth.

mod common;

use common::{Outcome, Pair, RetClass, Rng};

// ===========================================================================
// Row 1 — static_alias(NULL): the unguarded `*outer` read must trap the same
// way in both libraries. Probed in a forked child so the harness survives.
// ===========================================================================
#[test]
fn err_01_static_alias_null_ptr_same_signal() {
    let p = Pair::fresh();

    let c_out = common::outcome_in_child(|| {
        // SAFETY: intentionally invalid -- this is the behaviour under test.
        unsafe { (p.c.static_alias)(core::ptr::null_mut()) };
    });
    let r_out = common::outcome_in_child(|| {
        // SAFETY: intentionally invalid -- this is the behaviour under test.
        unsafe { (p.rust.static_alias)(core::ptr::null_mut()) };
    });

    assert_eq!(
        c_out, r_out,
        "static_alias(NULL) must terminate identically; C = {c_out:?}, Rust = {r_out:?}"
    );
    assert_eq!(
        c_out,
        Outcome::Signal(11),
        "the C reference dereferences NULL, so SIGSEGV (11) is expected; got {c_out:?}"
    );
}

// ===========================================================================
// Row 2 — driver(_, 0): degenerate count, no output, no state change.
// ===========================================================================
#[test]
fn err_02_driver_zero_iterations() {
    let mut rng = Rng::new(0xE770_0002);
    for i in 0..64 {
        let p = Pair::fresh();
        let v = rng.next_i32();
        let out = p.assert_driver(v, 0, &format!("err2 #{i}"));
        assert!(out.is_empty(), "err2 #{i}: driver(_, 0) must print nothing");
        // `inner` must be untouched: a following call with *outer == 1 must
        // still see inner == 1 and take the then-branch to 2.
        let obs = p.assert_static_alias(1, &format!("err2 #{i} state probe"));
        assert_eq!(obs.class, RetClass::LibStatic, "err2 #{i}: probe branch");
        assert_eq!(obs.ret_val, 2, "err2 #{i}: driver(_, 0) mutated `inner`");
    }
}

// ===========================================================================
// Row 3 — driver(_, negative): one step (and far) past the valid count range.
// ===========================================================================
#[test]
fn err_03_driver_negative_iterations() {
    let mut rng = Rng::new(0xE770_0003);
    let mut cases: Vec<i32> = vec![-1, -2, -1000, i32::MIN, i32::MIN + 1];
    for _ in 0..32 {
        cases.push(rng.range_i32(i32::MIN, -1));
    }
    for n in cases {
        let p = Pair::fresh();
        let v = rng.next_i32();
        let out = p.assert_driver(v, n, &format!("err3 n={n}"));
        assert!(
            out.is_empty(),
            "err3 n={n}: `i < iterations` is false on entry, so no output"
        );
        let obs = p.assert_static_alias(1, &format!("err3 n={n} state probe"));
        assert_eq!(obs.ret_val, 2, "err3 n={n}: negative count mutated `inner`");
    }
}

// ===========================================================================
// Row 4 — self-alias: feed the returned `&inner` straight back in.
// ===========================================================================
#[test]
fn err_04_static_alias_self_alias() {
    let mut rng = Rng::new(0xE770_0004);
    for i in 0..64 {
        let p = Pair::fresh();
        // Get onto `&inner` with a value >= inner (so the then-branch returns
        // the static's address rather than these locals').
        let seed = rng.range_i32(1, i32::MAX);
        let mut c_cell = seed;
        let mut r_cell = seed;
        let c_first = p.c.call_raw(&mut c_cell as *mut i32);
        let r_first = p.rust.call_raw(&mut r_cell as *mut i32);
        assert_eq!(
            (c_first.0, c_first.1),
            (r_first.0, r_first.1),
            "err4 #{i}: seeding call diverged (seed = {seed})"
        );

        let mut c_ptr = c_first.2;
        let mut r_ptr = r_first.2;
        for k in 0..20 {
            let c = p.c.call_raw(c_ptr);
            let r = p.rust.call_raw(r_ptr);
            // `*outer >= inner` is trivially true when they are the same
            // object, so the pointer must come back unchanged (it is both the
            // argument and `&inner`) and the value must double.
            assert_eq!(c.0, RetClass::Arg, "err4 #{i}.{k}: C self-alias identity");
            assert_eq!(r.0, RetClass::Arg, "err4 #{i}.{k}: Rust self-alias identity");
            assert_eq!(
                c.1, r.1,
                "err4 #{i}.{k}: self-alias value diverged (seed = {seed})"
            );
            c_ptr = c.2;
            r_ptr = r.2;
        }
    }
}

// ===========================================================================
// Row 5 — signed overflow of `inner += *outer`.
// ===========================================================================
#[test]
fn err_05_static_alias_overflow_inner() {
    // Drive `inner` to a large value, then add more so `inner += *outer`
    // overflows INT_MAX. The self-alias doubling reaches overflow on its own.
    for seed in [i32::MAX, i32::MAX - 1, i32::MAX / 2, 1 << 30] {
        let p = Pair::fresh();
        let mut c_cell = seed;
        let mut r_cell = seed;
        let mut c_ptr = p.c.call_raw(&mut c_cell as *mut i32).2;
        let mut r_ptr = p.rust.call_raw(&mut r_cell as *mut i32).2;
        assert_ne!(c_ptr, &mut c_cell as *mut i32, "seed must land on &inner");
        assert_ne!(r_ptr, &mut r_cell as *mut i32, "seed must land on &inner");
        for k in 0..40 {
            let c = p.c.call_raw(c_ptr);
            let r = p.rust.call_raw(r_ptr);
            assert_eq!(
                c.1, r.1,
                "err5 seed={seed} step {k}: `inner += *outer` overflow diverged"
            );
            c_ptr = c.2;
            r_ptr = r.2;
        }
    }

    // Also the direct shape: inner already == INT_MAX-ish, add a positive.
    let mut rng = Rng::new(0xE770_0005);
    for i in 0..64 {
        let p = Pair::fresh();
        p.assert_static_alias(i32::MAX, &format!("err5b #{i} push inner high"));
        // inner is now 1 + INT_MAX (wrapped) == INT_MIN. Anything >= that
        // re-enters the then-branch and overflows again.
        for k in 0..8 {
            let v = rng.next_i32();
            p.assert_static_alias(v, &format!("err5b #{i} call {k}"));
        }
    }
}

// ===========================================================================
// Row 6 — signed overflow of `*outer += inner`.
// ===========================================================================
#[test]
fn err_06_static_alias_overflow_outer() {
    let mut rng = Rng::new(0xE770_0006);
    for i in 0..64 {
        let p = Pair::fresh();
        // Make `inner` large and positive so that adding it to a very negative
        // `*outer` is the *underflow* direction, and adding to a large positive
        // `*outer` overflows.
        p.assert_static_alias(rng.range_i32(1 << 28, i32::MAX), &format!("err6 #{i} seed"));
        for &v in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            p.assert_static_alias(v, &format!("err6 #{i} v={v}"));
        }
    }

    // The specific documented trigger: *outer == INT_MIN with inner > *outer.
    let p = Pair::fresh();
    let obs = p.assert_static_alias(i32::MIN, "err6 direct INT_MIN");
    assert_eq!(obs.class, RetClass::Arg, "INT_MIN < inner so `outer` returned");
    assert_eq!(
        obs.outer_after,
        i32::MIN.wrapping_add(1),
        "INT_MIN + 1 (no overflow here, but the arithmetic must match the C)"
    );
}

// ===========================================================================
// Row 7 — *outer == INT_MIN (extreme low end).
// ===========================================================================
#[test]
fn err_07_static_alias_int_min() {
    let p = Pair::fresh();
    let obs = p.assert_static_alias(i32::MIN, "err7");
    assert_eq!(obs.class, RetClass::Arg);
    assert_eq!(obs.ret_val, i32::MIN + 1);
    assert_eq!(obs.outer_after, i32::MIN + 1);
}

// ===========================================================================
// Row 8 — *outer == INT_MAX (extreme high end), inner wraps.
// ===========================================================================
#[test]
fn err_08_static_alias_int_max() {
    let p = Pair::fresh();
    let obs = p.assert_static_alias(i32::MAX, "err8");
    assert_eq!(obs.class, RetClass::LibStatic);
    assert_eq!(
        obs.ret_val,
        1i32.wrapping_add(i32::MAX),
        "inner = 1 + INT_MAX must wrap identically in both libraries"
    );
    assert_eq!(obs.outer_after, i32::MAX, "caller cell untouched");
}

// ===========================================================================
// Row 9 — driver with extreme initial_value and large iteration counts, so
// overflow recurs through the alias chain.
// ===========================================================================
#[test]
fn err_09_driver_extreme_values() {
    for v in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for n in [1, 2, 5, 33, 64, 200] {
            let p = Pair::fresh();
            p.assert_driver(v, n, &format!("err9 v={v} n={n}"));
        }
    }
}

// ===========================================================================
// Row 10 — the `>=` boundary: *outer exactly equal to inner.
// ===========================================================================
#[test]
fn err_10_static_alias_equal_boundary() {
    let mut rng = Rng::new(0xE770_000A);
    for i in 0..64 {
        let p = Pair::fresh();
        // Establish a known `inner` by observing the then-branch result.
        let seed = rng.range_i32(1, 1 << 20);
        let obs = p.assert_static_alias(seed, &format!("err10 #{i} seed"));
        assert_eq!(obs.class, RetClass::LibStatic);
        let inner = obs.ret_val;

        // Exactly equal -> then-branch (a `>` would flip to the else-branch).
        let eq = p.assert_static_alias(inner, &format!("err10 #{i} == inner ({inner})"));
        assert_eq!(
            eq.class,
            RetClass::LibStatic,
            "err10 #{i}: `>=` must take the then-branch at equality"
        );
        assert_eq!(eq.ret_val, inner.wrapping_add(inner), "err10 #{i}: doubling");
    }

    // And one below -> else-branch, proving the boundary is exact.
    let p = Pair::fresh();
    let obs = p.assert_static_alias(5, "err10 seed2");
    let inner = obs.ret_val; // 6
    let below = p.assert_static_alias(inner - 1, "err10 inner-1");
    assert_eq!(below.class, RetClass::Arg, "one below inner -> else-branch");
}

// ===========================================================================
// Row 11 — driver with iterations == 1, the minimum non-degenerate count.
// ===========================================================================
#[test]
fn err_12_driver_one_iteration() {
    let mut rng = Rng::new(0xE770_000C);
    for i in 0..64 {
        let p = Pair::fresh();
        let v = rng.next_i32();
        let out = p.assert_driver(v, 1, &format!("err12 #{i}"));
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            1,
            "err12 #{i}: exactly one line"
        );
    }
}

// ===========================================================================
// Generic FFI-boundary cases the completion gate requires even though the C
// declares no enums and no lengths.
// ===========================================================================

/// `static_alias` has no enum parameter, and `driver`'s two parameters are plain
/// `int`s that accept *any* 32-bit value. The "out-of-range enum variant" class
/// therefore degenerates to "arbitrary bit pattern in an `int` parameter": we
/// feed values that no sane caller would produce and require identical results.
#[test]
fn boundary_arbitrary_int_bit_patterns() {
    let mut rng = Rng::new(0xE770_00FF);
    let mut cases: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -0x7FFF_FFFF,
        -0x8000,
        -1,
        0,
        1,
        0x7FFF,
        0x5555_5555,
        -0x5555_5556, // 0xAAAAAAAA
        i32::MAX - 1,
        i32::MAX,
    ];
    for _ in 0..128 {
        cases.push(rng.next_i32());
    }

    for &v in &cases {
        let p = Pair::fresh();
        p.assert_static_alias(v, &format!("boundary static_alias v={v:#010x}"));
    }

    // Same patterns as driver's `initial_value`, with a small positive count so
    // the loop actually runs, and as driver's `iterations`.
    for &v in &cases {
        let p = Pair::fresh();
        p.assert_driver(v, 3, &format!("boundary driver initial={v:#010x}"));
    }
    for &n in &cases {
        // Clamp the *count* so the test terminates; huge positive counts are
        // covered separately below.
        let n = if n > 300 { n % 300 } else { n };
        let p = Pair::fresh();
        p.assert_driver(7, n, &format!("boundary driver iterations={n}"));
    }
}

/// Oversized-but-finite iteration count: the closest analogue of an "oversized
/// length" for this API.
#[test]
fn boundary_oversized_iteration_count() {
    let p = Pair::fresh();
    let out = p.assert_driver(-3, 100_000, "boundary huge iterations");
    assert_eq!(
        out.iter().filter(|&&b| b == b'\n').count(),
        100_000,
        "one line per iteration"
    );
}

/// `driver` never dereferences a caller pointer, so it has no NULL case; assert
/// that explicitly (it returns normally for every count, including 0) so the
/// null-pointer boundary is covered for *both* entry points.
#[test]
fn boundary_driver_has_no_pointer_parameter() {
    let p = Pair::fresh();
    for n in [0, 1, 2] {
        let c = common::outcome_in_child(|| {
            // SAFETY: scalar arguments only.
            unsafe { (p.c.driver)(0, n) };
        });
        let r = common::outcome_in_child(|| {
            // SAFETY: scalar arguments only.
            unsafe { (p.rust.driver)(0, n) };
        });
        assert_eq!(c, Outcome::Returned, "C driver(0, {n}) should return");
        assert_eq!(r, c, "driver(0, {n}) termination diverged");
    }
}
