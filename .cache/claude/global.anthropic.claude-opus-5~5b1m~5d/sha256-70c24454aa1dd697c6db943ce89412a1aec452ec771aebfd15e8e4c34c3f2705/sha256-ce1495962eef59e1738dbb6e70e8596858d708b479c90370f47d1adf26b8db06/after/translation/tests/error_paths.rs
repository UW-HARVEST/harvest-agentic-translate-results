//! Phase C -- error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library performs no validation at all
//! (no error return, no `assert`, no null check, no range check), so its
//! "rejection surface" consists entirely of implicit/undefined-behaviour
//! conditions. For the two crashing rows the assertion is the *same fatal
//! signal*, not merely "both failed somehow".

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Row 1 -- static_alias(NULL): `*outer` is read with no null check
// ---------------------------------------------------------------------------

#[test]
fn err_01_static_alias_null_pointer_segv() {
    let _g = lock();
    let l = libs();
    let c_exit = run_in_child(|| unsafe {
        let r = (l.c.static_alias)(std::ptr::null_mut());
        // Not reached; keep the call from being elided.
        std::hint::black_box(r);
    });
    let rust_exit = run_in_child(|| unsafe {
        let r = (l.rust.static_alias)(std::ptr::null_mut());
        std::hint::black_box(r);
    });
    assert_eq!(
        c_exit, rust_exit,
        "static_alias(NULL) must fail identically: C={c_exit:?} Rust={rust_exit:?}"
    );
    assert_eq!(
        c_exit,
        Exit::Signal(libc::SIGSEGV),
        "expected the C null dereference to raise SIGSEGV"
    );
}

// ---------------------------------------------------------------------------
// Row 2 -- driver with iterations <= 0: zero-trip loop, no output, no state change
// ---------------------------------------------------------------------------

#[test]
fn err_02_driver_non_positive_iterations() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 102);
    let mut counts = [0, -1, -2, -1000, c_int::MIN, c_int::MIN + 1].to_vec();
    for _ in 0..200 {
        counts.push(rng.int_in(c_int::MIN, 0));
    }
    for &n in counts.iter() {
        for &v in BOUNDARIES.iter() {
            for &inner in &[INNER_INITIAL, 0, -1, c_int::MAX, c_int::MIN] {
                let obs = assert_driver_eq("err-02", inner, v, n);
                assert!(
                    obs.stdout.is_empty(),
                    "iterations={n} must print nothing, got {:?}",
                    preview(&obs.stdout)
                );
                assert_eq!(
                    obs.inner_after, inner,
                    "iterations={n} must leave inner unchanged"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 -- driver's `initial_value` is a by-value parameter whose address escapes
// ---------------------------------------------------------------------------

#[test]
fn err_03_driver_writes_only_its_own_parameter_copy() {
    let _g = lock();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 103);
    for _ in 0..300 {
        // A configuration that guarantees the else arm (`*outer += inner`) runs,
        // i.e. the callee really does write through the escaped address.
        let inner = rng.int_in(1, 1_000_000);
        let v = rng.int_in(-1_000_000, 0);
        let n = rng.int_in(1, 8);
        let obs = assert_driver_eq("err-03", inner, v, n);
        assert_eq!(
            obs.caller_arg_after, v,
            "the caller's variable must never be modified"
        );
        assert!(!obs.stdout.is_empty());
    }
    // Same check done directly on the low-level entry point: the else arm writes
    // through the caller's pointer and nowhere else.
    for _ in 0..300 {
        let inner = rng.int_in(1, 1_000_000);
        let v = rng.int_in(-1_000_000, 0);
        let mut guard_before: c_int = 0x5A5A_5A5A;
        let mut outer: c_int = v;
        let mut guard_after: c_int = 0x3C3C_3C3C;
        for lib in [&l.c, &l.rust] {
            set_inner(lib, inner);
            guard_before = 0x5A5A_5A5A;
            outer = v;
            guard_after = 0x3C3C_3C3C;
            let ret = unsafe { (lib.static_alias)(&mut outer) };
            assert_eq!(ret, &mut outer as *mut c_int, "{}: else arm", lib.name);
            assert_eq!(guard_before, 0x5A5A_5A5A, "{}: wrote out of bounds", lib.name);
            assert_eq!(guard_after, 0x3C3C_3C3C, "{}: wrote out of bounds", lib.name);
        }
        std::hint::black_box((guard_before, outer, guard_after));
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- self-aliasing: `outer == &inner` can never take the else arm
// ---------------------------------------------------------------------------

#[test]
fn err_04_static_alias_self_alias() {
    let _g = lock();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 104);
    let mut presets: Vec<c_int> = BOUNDARIES.to_vec();
    for _ in 0..300 {
        presets.push(rng.int_biased());
    }
    for &inner in presets.iter() {
        let run = |lib: &Lib| -> (bool, bool, c_int, c_int) {
            set_inner(lib, inner);
            let p = lib.inner_addr;
            let ret = unsafe { (lib.static_alias)(p) };
            (ret == p, ret == lib.inner_addr, unsafe { *ret }, get_inner(lib))
        };
        let got_c = run(&l.c);
        let got_rust = run(&l.rust);
        assert_eq!(
            got_c, got_rust,
            "[err-04] self-alias divergence for inner={inner}"
        );
        assert!(got_c.0 && got_c.1, "self-alias must return &inner");
        assert_eq!(
            got_c.3,
            inner.wrapping_add(inner),
            "self-alias must double inner"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 5 -- signed overflow in the `if` arm (`inner += *outer`)
// ---------------------------------------------------------------------------

#[test]
fn err_05_overflow_then_branch() {
    let _g = lock();
    // Every case here satisfies `*outer >= inner` (so the if arm runs) and
    // overflows `int`.
    let cases: [(c_int, c_int); 8] = [
        (c_int::MAX, c_int::MAX),
        (1, c_int::MAX),
        (2, c_int::MAX),
        (c_int::MAX - 1, c_int::MAX),
        (c_int::MAX, c_int::MAX - 1), // MAX-1 < MAX -> else arm, still overflow-free
        (c_int::MIN, c_int::MIN),
        (c_int::MIN, c_int::MIN + 1),
        (c_int::MIN + 1, c_int::MIN + 1),
    ];
    for &(inner, outer) in cases.iter() {
        let obs = assert_alias_eq("err-05", inner, outer);
        if obs.ret_is_inner {
            assert_eq!(
                obs.inner_after,
                inner.wrapping_add(outer),
                "overflow must wrap (inner={inner}, *outer={outer})"
            );
        }
    }
    // Randomized overflow-guaranteed inputs for the if arm.
    let mut rng = Rng::new(SEED ^ 105);
    for _ in 0..2000 {
        let inner = rng.int_in(1, c_int::MAX);
        // Need BOTH `*outer >= inner` (to take the if arm) and
        // `inner + *outer > INT_MAX` (to overflow).
        let lo = inner.max(c_int::MAX - inner + 1);
        let outer = rng.int_in(lo, c_int::MAX);
        let obs = assert_alias_eq("err-05", inner, outer);
        assert!(obs.ret_is_inner, "expected the if arm (inner={inner}, *outer={outer})");
        assert_eq!(obs.inner_after, inner.wrapping_add(outer));
    }
}

// ---------------------------------------------------------------------------
// Row 6 -- signed overflow in the `else` arm (`*outer += inner`)
// ---------------------------------------------------------------------------

#[test]
fn err_06_overflow_else_branch() {
    let _g = lock();
    // inner = -1, *outer = INT_MIN: INT_MIN < -1 so the else arm runs and
    // INT_MIN + (-1) underflows.
    let obs = assert_alias_eq("err-06", -1, c_int::MIN);
    assert!(obs.ret_is_outer);
    assert_eq!(obs.outer_after, c_int::MIN.wrapping_sub(1));
    assert_eq!(obs.inner_after, -1);

    let mut rng = Rng::new(SEED ^ 106);
    for _ in 0..2000 {
        // inner negative, *outer < inner, and inner + *outer < INT_MIN.
        let inner = rng.int_in(c_int::MIN / 2, -1);
        let lo = c_int::MIN;
        let hi = (c_int::MIN as i64 - inner as i64 - 1) as c_int; // *outer < MIN-inner
        if lo > hi {
            continue;
        }
        let outer = rng.int_in(lo, hi);
        if outer >= inner {
            continue; // need the else arm
        }
        let obs = assert_alias_eq("err-06", inner, outer);
        assert!(obs.ret_is_outer, "expected the else arm");
        assert_eq!(obs.outer_after, outer.wrapping_add(inner));
        assert_eq!(obs.inner_after, inner);
    }
}

// ---------------------------------------------------------------------------
// Row 7 -- boundary cross-product; every int bit pattern is an accepted input
// (this also covers the "out-of-range enum value" class: the API has no enum,
// and no int value is rejected)
// ---------------------------------------------------------------------------

#[test]
fn err_07_boundary_value_cross_product() {
    let _g = lock();
    for &inner in BOUNDARIES.iter() {
        for &outer in BOUNDARIES.iter() {
            let obs = assert_alias_eq("err-07", inner, outer);
            assert!(
                obs.ret_is_inner || obs.ret_is_outer,
                "the return value is always one of the two known pointers"
            );
        }
    }
    // Nonsensical / "out of range" bit patterns handed across the FFI boundary
    // for both parameters of `driver` as well.
    let odd: [c_int; 10] = [
        c_int::MIN,
        c_int::MIN + 1,
        -0x7FFF_FFFF,
        -0x0100_0000,
        -1,
        0,
        1,
        0x0100_0000,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for &v in odd.iter() {
        for &n in &[0, 1, 2, 5] {
            assert_driver_eq("err-07", INNER_INITIAL, v, n);
        }
        // and `iterations` itself given an absurd value
        assert_driver_eq("err-07", INNER_INITIAL, 3, v.clamp(-8, 8));
    }
}

// ---------------------------------------------------------------------------
// Row 8 -- overflow reached transitively through driver's iteration count
// ---------------------------------------------------------------------------

#[test]
fn err_08_driver_overflow_by_iteration() {
    let _g = lock();
    // inner = 1, initial_value = 1 -> if arm at once, then self-alias doubling:
    // 2, 4, 8, ... which overflows after ~31 iterations and keeps wrapping.
    let obs = assert_driver_eq("err-08", 1, 1, 80);
    let text = String::from_utf8(obs.stdout.clone()).unwrap();
    let nums: Vec<i64> = text.lines().map(|s| s.parse().unwrap()).collect();
    assert_eq!(nums.len(), 80);
    assert!(
        nums.iter().any(|&x| x < 0) || nums.iter().any(|&x| x == 0),
        "80 doublings must wrap into negative/zero territory: {nums:?}"
    );
    // Randomized: any state/input, enough iterations to wrap repeatedly.
    let mut rng = Rng::new(SEED ^ 108);
    for _ in 0..150 {
        assert_driver_eq("err-08", rng.int_biased(), rng.int_biased(), rng.int_in(40, 200));
    }
}

// ---------------------------------------------------------------------------
// Row 9 -- driver with initial_value == INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn err_09_driver_int_min_initial_value() {
    let _g = lock();
    for &inner in &[1, 2, 7, 1000, c_int::MAX, c_int::MIN, 0, -1] {
        for &n in &[1, 2, 3, 10, 64] {
            assert_driver_eq("err-09", inner, c_int::MIN, n);
            assert_driver_eq("err-09", inner, c_int::MIN + 1, n);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 -- the largest accepted iteration count
// ---------------------------------------------------------------------------

#[test]
fn err_10_driver_huge_iteration_count_prefix() {
    let _g = lock();
    // `iterations == INT_MAX` differs from a large count only in how long the
    // loop runs, so it is verified through its steady state: with `inner == 0`
    // and `initial_value == 0` the if arm runs, `inner` stays 0, and the loop
    // prints "0\n" forever. Both libraries must produce exactly that.
    for &n in &[1, 2, 1000, 20000] {
        let obs = assert_driver_eq("err-10", 0, 0, n);
        assert_eq!(obs.stdout, "0\n".repeat(n as usize).into_bytes());
        assert_eq!(obs.inner_after, 0);
    }
    // A second saturating steady state: inner = INT_MIN doubles to 0, then stays.
    let obs = assert_driver_eq("err-10", c_int::MIN, c_int::MAX, 5000);
    assert_eq!(obs.inner_after, 0);
    assert!(obs.stdout.ends_with(b"0\n"));
    // And a long run from the fresh state.
    assert_driver_eq("err-10", INNER_INITIAL, 1, 20000);
}

// ---------------------------------------------------------------------------
// Row 11 -- else arm storing through a pointer into read-only memory
// ---------------------------------------------------------------------------

#[test]
fn err_11_static_alias_readonly_outer_segv() {
    let _g = lock();
    let l = libs();
    unsafe {
        let page = libc::mmap(
            std::ptr::null_mut(),
            4096,
            libc::PROT_READ,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(page, libc::MAP_FAILED, "mmap failed");
        let ro = page as *mut c_int; // reads as 0
        assert_eq!(*ro, 0, "anonymous mapping must read as zero");

        // inner = INT_MAX makes `0 >= INT_MAX` false, so the else arm runs and
        // stores through `ro` -> SIGSEGV.
        set_inner(&l.c, c_int::MAX);
        set_inner(&l.rust, c_int::MAX);
        let c_exit = run_in_child(|| {
            let r = (l.c.static_alias)(ro);
            std::hint::black_box(r);
        });
        let rust_exit = run_in_child(|| {
            let r = (l.rust.static_alias)(ro);
            std::hint::black_box(r);
        });
        libc::munmap(page, 4096);

        assert_eq!(
            c_exit, rust_exit,
            "a store through a read-only `outer` must fail identically: C={c_exit:?} Rust={rust_exit:?}"
        );
        assert_eq!(
            c_exit,
            Exit::Signal(libc::SIGSEGV),
            "expected SIGSEGV from the read-only store"
        );
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep (beyond the table): every 32-bit pattern class is
// a legal argument, so sweep them densely for both entry points.
// ---------------------------------------------------------------------------

#[test]
fn err_generic_full_range_sweep() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xBADC0DE);
    for _ in 0..20000 {
        assert_alias_eq("err-generic", rng.int(), rng.int());
    }
    for _ in 0..400 {
        assert_driver_eq("err-generic", rng.int(), rng.int(), rng.int_in(0, 16));
    }
}
