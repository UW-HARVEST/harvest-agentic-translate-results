//! Phase C — error-path / boundary differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error channel at all
//! (0 error returns, 0 asserts, 0 range checks, 0 null checks — see
//! `ERRORS.md`), so "the same error/rejection" here means: both libraries must
//! agree on the *absence* of rejection and on the exact bytes emitted, for every
//! boundary and out-of-range input a caller can construct.

use crate::common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Rows 1 & 2 — the internal `print_hex` guards (`len <= 0`, `p == NULL`).
//
// `print_hex` is `static` in C, so it is not a dynamic symbol in EITHER library
// (verified below). It is therefore unreachable across the FFI boundary and
// there is no way for a caller to supply len<=0 or a null pointer. This test
// asserts that unreachability holds identically for both libraries — i.e. the
// Rust translation did not accidentally widen the API surface.
// ---------------------------------------------------------------------------
pub fn err_rows1_2_internal_print_hex_not_reachable() {
    let l = libs();
    let c_has: Option<libloading::Symbol<unsafe extern "C" fn(*const u8, c_int)>> =
        unsafe { l.c.get(b"print_hex\0") }.ok();
    let r_has: Option<libloading::Symbol<unsafe extern "C" fn(*const u8, c_int)>> =
        unsafe { l.rust.get(b"print_hex\0") }.ok();
    assert!(
        c_has.is_none(),
        "print_hex is `static` in C; it must not be exported"
    );
    assert!(
        r_has.is_none(),
        "the Rust translation must not export print_hex either \
         (that would be a wider API surface than the C library)"
    );
    // Same rejection, from the same cause, in both libraries: dlsym fails.
    assert_eq!(c_has.is_none(), r_has.is_none());
}

// ---------------------------------------------------------------------------
// Row 3 — INT_MIN
// ---------------------------------------------------------------------------
pub fn err_row3_int_min() {
    let out = assert_same("err3", i32::MIN);
    assert_eq!(out, b"00000080\n".to_vec());
}

// ---------------------------------------------------------------------------
// Row 4 — INT_MAX
// ---------------------------------------------------------------------------
pub fn err_row4_int_max() {
    let out = assert_same("err4", i32::MAX);
    assert_eq!(out, b"ffffff7f\n".to_vec());
}

// ---------------------------------------------------------------------------
// Row 5 — the classic error sentinel -1
// ---------------------------------------------------------------------------
pub fn err_row5_minus_one() {
    let out = assert_same("err5", -1);
    assert_eq!(out, b"ffffffff\n".to_vec());
}

// ---------------------------------------------------------------------------
// Row 6 — zero (the "zero length / empty" analogue)
// ---------------------------------------------------------------------------
pub fn err_row6_zero() {
    let out = assert_same("err6", 0);
    assert_eq!(out, b"00000000\n".to_vec());
}

// ---------------------------------------------------------------------------
// Rows 7 & 8 — OUT-OF-RANGE VALUES ACROSS THE FFI BOUNDARY.
//
// A C `int` parameter, exactly like a C enum, accepts any bit pattern the ABI
// register happens to hold; a value with no valid "variant" is a real input the
// C handles, and the Rust must handle it identically. We call the very same
// exported `driver` symbol through the wider signature `extern "C" fn(i64)`.
// ---------------------------------------------------------------------------
pub fn err_row7_oversized_arg_truncation() {
    let cases: &[i64] = &[
        0,
        -1,
        1,
        i64::MIN,
        i64::MAX,
        i32::MIN as i64,
        i32::MAX as i64,
        // One step past the end of the unsigned 32-bit range (row 8).
        u32::MAX as i64 + 1,
        u32::MAX as i64,
        u32::MAX as i64 - 1,
        // Garbage in the upper 32 bits, meaningful low half.
        0x7FFF_FFFF_DEAD_BEEFu64 as i64,
        0xFFFF_FFFF_0000_0000u64 as i64,
        0xDEAD_BEEF_FFFF_FFFFu64 as i64,
        0x0000_0001_8000_0000u64 as i64,
        0xAAAA_AAAA_5555_5555u64 as i64,
        // One step past several documented-range edges.
        (i32::MAX as i64) + 1,
        (i32::MIN as i64) - 1,
    ];
    for &v in cases {
        assert_same_wide("err7", v);
    }

    // Randomized: full 64-bit garbage, fixed seed.
    let mut rng = Rng::new(7007);
    for _ in 0..2000 {
        let v = ((rng.next_u64() as u128) as u64) as i64;
        assert_same_wide("err7-rand", v);
    }

    // Row 8 explicitly: 0x1_0000_0000 must print as all-zero low half.
    let out = assert_same_wide("err8", u32::MAX as i64 + 1);
    assert_eq!(
        out,
        b"00000000\n".to_vec(),
        "only the low 32 bits are read by the callee"
    );
}

// ---------------------------------------------------------------------------
// Row 8 (dedicated) — one step past the unsigned range boundary
// ---------------------------------------------------------------------------
pub fn err_row8_one_past_unsigned_range() {
    for v in [
        u32::MAX as i64 + 1,
        u32::MAX as i64 + 2,
        (u32::MAX as i64 + 1) * 2,
        -(u32::MAX as i64 + 1),
    ] {
        assert_same_wide("err8", v);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — no hidden state: repeated / interleaved calls cannot corrupt anything
// ---------------------------------------------------------------------------
pub fn err_row9_no_hidden_state() {
    let cf = c_driver();
    let rf = rust_driver();
    let mut rng = Rng::new(9);

    // The same value called many times must produce the identical record each
    // time, in both libraries.
    for _ in 0..50 {
        let x: c_int = rng.next_i32();
        let one = assert_same("err9", x);
        let repeated = assert_same_batch("err9", &[x; 7]);
        let mut expected = Vec::new();
        for _ in 0..7 {
            expected.extend_from_slice(&one);
        }
        assert_eq!(repeated, expected, "driver must be a pure function of x");
    }

    // Interleaving extreme values with random ones must not perturb later
    // results in either library.
    let poison: Vec<c_int> = vec![i32::MIN, i32::MAX, -1, 0, 1];
    for &p in &poison {
        let before = assert_same("err9", 0x0123_4567);
        let _ = capture_stdout(|| unsafe {
            cf(p);
            rf(p);
        });
        let after = assert_same("err9", 0x0123_4567);
        assert_eq!(before, after, "state leaked after driver({p})");
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary: the exported symbol must have the SAME shape in both.
// Calling `driver` through several ABI-compatible-ish signatures a sloppy caller
// might use must behave identically in both libraries.
// ---------------------------------------------------------------------------
pub fn err_generic_alternate_call_signatures_agree() {
    let l = libs();
    let mut rng = Rng::new(4242);

    // As unsigned int.
    let cu: libloading::Symbol<unsafe extern "C" fn(u32)> =
        unsafe { l.c.get(b"driver\0") }.unwrap();
    let ru: libloading::Symbol<unsafe extern "C" fn(u32)> =
        unsafe { l.rust.get(b"driver\0") }.unwrap();
    for _ in 0..500 {
        let v = rng.next_u32();
        let c_out = capture_stdout(|| unsafe { cu(v) });
        let r_out = capture_stdout(|| unsafe { ru(v) });
        assert_eq!(c_out, r_out, "driver(u32 {v:#010x}) diverged");
    }

    // As unsigned long long (upper bits ignored by the SysV callee).
    let cw: libloading::Symbol<unsafe extern "C" fn(u64)> =
        unsafe { l.c.get(b"driver\0") }.unwrap();
    let rw: libloading::Symbol<unsafe extern "C" fn(u64)> =
        unsafe { l.rust.get(b"driver\0") }.unwrap();
    for _ in 0..500 {
        let v = rng.next_u64();
        let c_out = capture_stdout(|| unsafe { cw(v) });
        let r_out = capture_stdout(|| unsafe { rw(v) });
        assert_eq!(c_out, r_out, "driver(u64 {v:#018x}) diverged");
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary: nonexistent / mistyped symbol lookups must fail the
// same way against both libraries (no accidental extra exports in Rust).
// ---------------------------------------------------------------------------
pub fn err_generic_no_extra_exports() {
    let l = libs();
    for name in [
        b"print_hex\0".as_ref(),
        b"Driver\0".as_ref(),
        b"driver_\0".as_ref(),
        b"_driver\0".as_ref(),
        b"driver2\0".as_ref(),
        b"rust_driver\0".as_ref(),
    ] {
        let c_ok = unsafe { l.c.get::<*const ()>(name) }.is_ok();
        let r_ok = unsafe { l.rust.get::<*const ()>(name) }.is_ok();
        assert_eq!(
            c_ok,
            r_ok,
            "symbol {:?} present in C = {c_ok} but in Rust = {r_ok}",
            String::from_utf8_lossy(&name[..name.len() - 1])
        );
        assert!(!c_ok, "unexpected export");
    }

    // And the one symbol that MUST exist, exists in both.
    assert!(unsafe { l.c.get::<*const ()>(b"driver\0") }.is_ok());
    assert!(unsafe { l.rust.get::<*const ()>(b"driver\0") }.is_ok());
}
