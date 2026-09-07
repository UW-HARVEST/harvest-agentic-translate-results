//! Phase C — error/rejection-path differential tests. One test per row of
//! `ERRORS.md`, plus the generic FFI boundaries.
//!
//! `driver.c` contains no `if`, no `return`, no `assert`, no allocation and no
//! pointer/enum parameter, so it has no error channel: both public functions are
//! `void f(int)` and accept every one of the 2^32 argument values. These tests
//! therefore pin down that BOTH libraries accept the same inputs and produce the
//! same bytes for them -- in particular at the signed-overflow boundaries, where
//! the C relies on implementation-defined two's-complement wrapping and the Rust
//! must use `wrapping_add` to agree.

#[path = "harness/mod.rs"]
mod harness;

use harness::{assert_each_driver, assert_each_run, assert_seq_matches, capture, fresh_pair, Op, Rng};
use libloading::Symbol;
use std::ffi::{c_int, c_void};

/// Boundary arguments that a C API of this shape can be probed with.
fn boundary_args() -> Vec<i32> {
    vec![
        0,
        1,
        -1,
        2,
        -2,
        5,
        -5,
        -6,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 5,     // bedrooms lands exactly on INT_MAX
        i32::MAX - 4,     // one past
        i32::MIN + 5,     // bedrooms lands exactly on INT_MIN
        i32::MIN + 4,     // one past
        i32::MAX / 2,
        i32::MIN / 2,
        0x7FFF_FFFF,
        -0x8000_0000,
        0x0000_FFFF,
        -0x0001_0000,
    ]
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — `run` has no rejecting input.
// ---------------------------------------------------------------------------

#[test]
fn e1_run_accepts_every_int() {
    // All boundary values...
    assert_each_run("e1_bound", boundary_args());
    // ...plus a randomised sweep of the full 32-bit domain.
    let mut rng = Rng::new(0xE1_5EED);
    let args: Vec<i32> = (0..400).map(|_| rng.next_i32()).collect();
    assert_each_run("e1_rand", args);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — `driver` has no rejecting input.
// ---------------------------------------------------------------------------

#[test]
fn e2_driver_accepts_every_int() {
    assert_each_driver("e2_bound", boundary_args());
    let mut rng = Rng::new(0xE2_5EED);
    let args: Vec<i32> = (0..400).map(|_| rng.next_i32()).collect();
    assert_each_driver("e2_rand", args);

    // `driver(x)` must be exactly `run(x); run(x);` on both sides, including at
    // the wrapping boundaries.
    for x in boundary_args() {
        let p = fresh_pair("e2_equiv_a");
        let via_driver = capture(|| unsafe { (p.c_fn("driver"))(x) });
        let q = fresh_pair("e2_equiv_b");
        let via_two_runs = capture(|| unsafe {
            let f = q.c_fn("run");
            f(x);
            f(x);
        });
        assert_eq!(
            String::from_utf8_lossy(&via_driver),
            String::from_utf8_lossy(&via_two_runs),
            "C: driver({x}) != run({x}) twice"
        );
        let r = fresh_pair("e2_equiv_c");
        let rust_driver = capture(|| unsafe { (r.r_fn("driver"))(x) });
        assert_eq!(
            String::from_utf8_lossy(&via_driver),
            String::from_utf8_lossy(&rust_driver),
            "Rust driver({x}) diverges from C"
        );
    }
}

// ---------------------------------------------------------------------------
// G2 — "zero length" analogue: the zero value.
// ---------------------------------------------------------------------------

#[test]
fn e3_zero_value() {
    assert_seq_matches("e3_run", &[Op::Run(0)]);
    assert_seq_matches("e3_driver", &[Op::Driver(0)]);
    // Zero repeated: isolates floors/bathrooms accumulation with no bedroom change.
    let ops: Vec<Op> = (0..25).map(|_| Op::Run(0)).collect();
    assert_seq_matches("e3_repeat", &ops);
}

// ---------------------------------------------------------------------------
// G3 — "oversized length" analogue: INT_MAX, which overflows `bedrooms`.
// ---------------------------------------------------------------------------

#[test]
fn e4_int_max_overflow() {
    assert_seq_matches("e4_run", &[Op::Run(i32::MAX)]);
    assert_seq_matches("e4_driver", &[Op::Driver(i32::MAX)]);
    // Overflow repeatedly, so `bedrooms` wraps many times in a row.
    let ops: Vec<Op> = (0..40).map(|_| Op::Run(i32::MAX)).collect();
    assert_seq_matches("e4_repeat", &ops);
    let ops: Vec<Op> = (0..40).map(|_| Op::Driver(i32::MAX)).collect();
    assert_seq_matches("e4_repeat_drv", &ops);
}

// ---------------------------------------------------------------------------
// G4 — one step past every boundary of the valid range.
// ---------------------------------------------------------------------------

#[test]
fn e5_one_past_range() {
    assert_each_run("e5", boundary_args());
    assert_each_driver("e5", boundary_args());
    // Mixed sequences that step across a boundary and back.
    assert_seq_matches(
        "e5_seq_up",
        &[Op::Run(i32::MAX - 5), Op::Run(1), Op::Run(1), Op::Run(-2)],
    );
    assert_seq_matches(
        "e5_seq_down",
        &[Op::Run(i32::MIN + 5), Op::Run(-1), Op::Run(-1), Op::Run(2)],
    );
    assert_seq_matches(
        "e5_seq_mixed",
        &[
            Op::Driver(i32::MAX),
            Op::Run(i32::MIN),
            Op::Driver(-1),
            Op::Run(i32::MAX - 5),
            Op::Driver(i32::MIN + 4),
        ],
    );
}

// ---------------------------------------------------------------------------
// G6 — state after extreme arguments keeps tracking identically.
// ---------------------------------------------------------------------------

#[test]
fn e6_state_after_extremes() {
    let mut rng = Rng::new(0xE6_5EED);
    for trial in 0..10 {
        let mut ops = vec![Op::Run(i32::MAX), Op::Driver(i32::MIN), Op::Run(i32::MAX)];
        ops.extend((0..40).map(|_| {
            if rng.next_u64() % 2 == 0 {
                Op::Run(rng.next_i32())
            } else {
                Op::Driver(rng.in_range(-3, 3))
            }
        }));
        assert_seq_matches(&format!("e6_t{trial}"), &ops);
    }
}

// ---------------------------------------------------------------------------
// G1 / G5 — argument bit patterns with no "valid variant".
//
// The C API has no pointer and no enum parameter, but an external caller can
// still push any 64-bit register value through the `void f(int)` ABI. These
// tests call the exported symbols through deliberately mis-typed function
// pointers so both sides see identical raw register contents, and assert they
// agree. (SysV AMD64: the callee reads only the low 32 bits of `edi`, so the
// high garbage bits must be ignored identically by C and Rust.)
// ---------------------------------------------------------------------------

#[test]
fn e7_null_pointer_shaped_argument() {
    let p = fresh_pair("e7");
    type PtrFn<'a> = Symbol<'a, unsafe extern "C" fn(*const c_void)>;
    let c: PtrFn = unsafe { p.c.get(b"run") }.unwrap();
    let r: PtrFn = unsafe { p.r.get(b"run") }.unwrap();
    let c_out = capture(|| unsafe { c(std::ptr::null()) });
    let r_out = capture(|| unsafe { r(std::ptr::null()) });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "NULL-shaped argument diverges"
    );

    // A NULL argument is just the integer 0 to this ABI; confirm against run(0).
    let q = fresh_pair("e7_ref");
    let ref_out = capture(|| unsafe { (q.c_fn("run"))(0) });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&ref_out),
        "C: run(NULL) != run(0)"
    );
}

#[test]
fn e8_out_of_range_unsigned_and_wide_arguments() {
    type U32Fn<'a> = Symbol<'a, unsafe extern "C" fn(u32)>;
    type I64Fn<'a> = Symbol<'a, unsafe extern "C" fn(i64)>;

    // Unsigned bit patterns with no corresponding non-negative `int` value
    // (the closest analogue of an out-of-range enum for this API).
    for v in [
        0xFFFF_FFFFu32,
        0x8000_0000,
        0xDEAD_BEEF,
        0xFFFF_FFFE,
        0x7FFF_FFFF,
        0xCCCC_CCCC,
    ] {
        for sym in [b"run".as_slice(), b"driver".as_slice()] {
            let p = fresh_pair("e8u");
            let c: U32Fn = unsafe { p.c.get(sym) }.unwrap();
            let r: U32Fn = unsafe { p.r.get(sym) }.unwrap();
            let c_out = capture(|| unsafe { c(v) });
            let r_out = capture(|| unsafe { r(v) });
            assert_eq!(
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out),
                "u32 {v:#010x} through {} diverges",
                String::from_utf8_lossy(sym)
            );
        }
    }

    // 64-bit values whose high half is garbage: both callees must ignore it.
    for hi in [0x0000_0001u64, 0xFFFF_FFFF, 0xDEAD_BEEF] {
        for lo in [0u32, 1, 7, 0xFFFF_FFFF, 0x8000_0000] {
            let wide = ((hi << 32) | lo as u64) as i64;
            let p = fresh_pair("e8w");
            let c: I64Fn = unsafe { p.c.get(b"run") }.unwrap();
            let r: I64Fn = unsafe { p.r.get(b"run") }.unwrap();
            let c_out = capture(|| unsafe { c(wide) });
            let r_out = capture(|| unsafe { r(wide) });
            assert_eq!(
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out),
                "wide arg {wide:#018x} diverges"
            );
            // And it must equal the low-32-bit `int` call.
            let q = fresh_pair("e8w_ref");
            let expect = capture(|| unsafe { (q.c_fn("run"))(lo as c_int) });
            assert_eq!(
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&expect),
                "C: wide arg {wide:#018x} != run({})",
                lo as c_int
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Symbol-resolution failures must be identical: neither library may export any
// extra public entry point (a stubbed/faked symbol would show up here).
// ---------------------------------------------------------------------------

#[test]
fn e9_no_extra_or_missing_symbols() {
    let p = fresh_pair("e9");
    for name in [b"run".as_slice(), b"driver".as_slice()] {
        assert!(
            unsafe { p.c.get::<unsafe extern "C" fn(c_int)>(name) }.is_ok(),
            "C lib missing {}",
            String::from_utf8_lossy(name)
        );
        assert!(
            unsafe { p.r.get::<unsafe extern "C" fn(c_int)>(name) }.is_ok(),
            "Rust lib missing {}",
            String::from_utf8_lossy(name)
        );
    }
    // The `static` helpers must NOT be exported by either side.
    for name in [
        b"add_floor".as_slice(),
        b"add_bedrooms".as_slice(),
        b"add_floor_to_the_house".as_slice(),
        b"print_the_house".as_slice(),
        b"the_house".as_slice(),
    ] {
        let c_has = unsafe { p.c.get::<*const c_void>(name) }.is_ok();
        let r_has = unsafe { p.r.get::<*const c_void>(name) }.is_ok();
        assert_eq!(
            c_has,
            r_has,
            "visibility mismatch for internal symbol {}: C={c_has} Rust={r_has}",
            String::from_utf8_lossy(name)
        );
        assert!(
            !c_has,
            "{} should be static/internal in C",
            String::from_utf8_lossy(name)
        );
    }
}
