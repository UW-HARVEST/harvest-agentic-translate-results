//! Phase C -- error-path differential tests, one per row of `ERRORS.md`.
//!
//! The C library contains zero conditional branches, zero `return` statements
//! and no pointer/length parameters, so it has no in-band error surface (see
//! `ERRORS.md` for the mechanical grep). What remains is the ABI boundary: the
//! extremes of the 8-bit domain, one-step-past-range values, and over-wide
//! integers pushed across the FFI boundary (the analogue of an out-of-range
//! enum value). Each row asserts the two implementations agree *exactly*, not
//! merely that "both did something".

mod common;

use common::*;
use std::ffi::{c_char, c_int};

const PHCL: &str = "printHexCharLine";
const DRV: &str = "driver";

/// Helper: capture one call and return the produced text.
fn out_of(imp: Impl, name: &str, arg: c_char) -> String {
    let f = sym_char(imp, name);
    String::from_utf8_lossy(&capture(|| unsafe { f(arg) })).into_owned()
}

fn out_of_int(imp: Impl, name: &str, arg: c_int) -> String {
    let f = sym_int(imp, name);
    String::from_utf8_lossy(&capture(|| unsafe { f(arg) })).into_owned()
}

// ---------------------------------------------------------------------------
// Row 1: printHexCharLine(0x00) -- lower bound of the domain.
// ---------------------------------------------------------------------------
#[test]
fn err_row01_phcl_lower_bound() {
    let c = out_of(Impl::C, PHCL, 0x00u8 as c_char);
    let r = out_of(Impl::Rust, PHCL, 0x00u8 as c_char);
    assert_eq!(c, r, "printHexCharLine(0x00) diverged");
    assert_eq!(c, "00\n", "C behaviour changed from the recorded baseline");
}

// ---------------------------------------------------------------------------
// Row 2: printHexCharLine(CHAR_MAX).
// ---------------------------------------------------------------------------
#[test]
fn err_row02_phcl_char_max() {
    let c = out_of(Impl::C, PHCL, 0x7fu8 as c_char);
    let r = out_of(Impl::Rust, PHCL, 0x7fu8 as c_char);
    assert_eq!(c, r, "printHexCharLine(0x7f) diverged");
    assert_eq!(c, "7f\n");
}

// ---------------------------------------------------------------------------
// Row 3: printHexCharLine(0x80) -- one step past CHAR_MAX; sign extension.
// ---------------------------------------------------------------------------
#[test]
fn err_row03_phcl_one_past_char_max() {
    let c = out_of(Impl::C, PHCL, 0x80u8 as c_char);
    let r = out_of(Impl::Rust, PHCL, 0x80u8 as c_char);
    assert_eq!(c, r, "printHexCharLine(0x80) diverged");
    // Not "80": the sign extension of `char` -> `int` is observable.
    assert_eq!(c, "ffffff80\n", "C sign-extension baseline changed");
}

// ---------------------------------------------------------------------------
// Row 4: printHexCharLine(0xff) -- upper bound of the domain.
// ---------------------------------------------------------------------------
#[test]
fn err_row04_phcl_upper_bound() {
    let c = out_of(Impl::C, PHCL, 0xffu8 as c_char);
    let r = out_of(Impl::Rust, PHCL, 0xffu8 as c_char);
    assert_eq!(c, r, "printHexCharLine(0xff) diverged");
    assert_eq!(c, "ffffffff\n");
}

// ---------------------------------------------------------------------------
// Row 5: printHexCharLine with an over-wide `int` (out-of-range "enum" value).
// ---------------------------------------------------------------------------
#[test]
fn err_row05_phcl_out_of_range_int() {
    let cases: [c_int; 16] = [
        0x100,
        0x1ff,
        0x180,
        0x7fff,
        0x1_0000,
        0x1234_5678,
        -1000,
        -256,
        -257,
        -129,
        128,
        129,
        255,
        256,
        c_int::MIN,
        c_int::MAX,
    ];
    for v in cases {
        let c = out_of_int(Impl::C, PHCL, v);
        let r = out_of_int(Impl::Rust, PHCL, v);
        assert_eq!(c, r, "printHexCharLine({v}) diverged: C={c:?} Rust={r:?}");
    }
    // Exhaustive-ish randomized sweep of the full i32 domain.
    let mut rng = Rng::new(0xC0FFEE_05);
    for _ in 0..512 {
        let v = rng.next_i32();
        let c = out_of_int(Impl::C, PHCL, v);
        let r = out_of_int(Impl::Rust, PHCL, v);
        assert_eq!(c, r, "printHexCharLine({v}) diverged: C={c:?} Rust={r:?}");
    }
}

// ---------------------------------------------------------------------------
// Row 6: driver(0x7f) -- `data + 1` overflows the char range.
// ---------------------------------------------------------------------------
#[test]
fn err_row06_driver_overflow() {
    let c = out_of(Impl::C, DRV, 0x7fu8 as c_char);
    let r = out_of(Impl::Rust, DRV, 0x7fu8 as c_char);
    assert_eq!(c, r, "driver(0x7f) diverged");
    assert_eq!(c, "ffffff80\n", "C wrap-on-overflow baseline changed");
}

// ---------------------------------------------------------------------------
// Row 7: driver with an over-wide `int`.
// ---------------------------------------------------------------------------
#[test]
fn err_row07_driver_out_of_range_int() {
    let cases: [c_int; 16] = [
        0x100,
        0x17f,
        0x1ff,
        0x180,
        0x7fff,
        0x1_0000,
        0x1234_5678,
        -1000,
        -256,
        -257,
        -129,
        128,
        129,
        255,
        c_int::MIN,
        c_int::MAX,
    ];
    for v in cases {
        let c = out_of_int(Impl::C, DRV, v);
        let r = out_of_int(Impl::Rust, DRV, v);
        assert_eq!(c, r, "driver({v}) diverged: C={c:?} Rust={r:?}");
    }
    let mut rng = Rng::new(0xC0FFEE_07);
    for _ in 0..512 {
        let v = rng.next_i32();
        let c = out_of_int(Impl::C, DRV, v);
        let r = out_of_int(Impl::Rust, DRV, v);
        assert_eq!(c, r, "driver({v}) diverged: C={c:?} Rust={r:?}");
    }
}

// ---------------------------------------------------------------------------
// Row 8: driver(0xff) -- result wraps down to 0.
// ---------------------------------------------------------------------------
#[test]
fn err_row08_driver_wraps_to_zero() {
    let c = out_of(Impl::C, DRV, 0xffu8 as c_char);
    let r = out_of(Impl::Rust, DRV, 0xffu8 as c_char);
    assert_eq!(c, r, "driver(0xff) diverged");
    assert_eq!(c, "00\n");
}

// ---------------------------------------------------------------------------
// Row 9: "null pointer" -- unreachable by construction. What *is* observable
// is that neither export takes a pointer, i.e. an all-zero 64-bit argument is
// interpreted as the integer 0 by both, with no crash and no error path.
// ---------------------------------------------------------------------------
#[test]
fn err_row09_no_pointer_parameter() {
    type FnPtrArg = unsafe extern "C" fn(*const std::ffi::c_void);
    for &name in SYMBOLS.iter() {
        // A "null pointer" is just the zero bit pattern in the argument register.
        let zero_c = out_of(Impl::C, name, 0);
        let zero_r = out_of(Impl::Rust, name, 0);
        assert_eq!(zero_c, zero_r, "{name}(0) diverged");

        // Calling through a pointer-typed view with NULL must behave the same
        // as passing 0 -- proving there is no pointer dereference / null check.
        let fc: FnPtrArg = unsafe { std::mem::transmute(sym_char(Impl::C, name)) };
        let fr: FnPtrArg = unsafe { std::mem::transmute(sym_char(Impl::Rust, name)) };
        let oc = String::from_utf8_lossy(&capture(|| unsafe { fc(std::ptr::null()) })).into_owned();
        let or = String::from_utf8_lossy(&capture(|| unsafe { fr(std::ptr::null()) })).into_owned();
        assert_eq!(oc, or, "{name}(NULL) diverged: C={oc:?} Rust={or:?}");
        assert_eq!(oc, zero_c, "{name}(NULL) differs from {name}(0) in C");
    }
}

// ---------------------------------------------------------------------------
// Row 10: "zero / oversized length" -- unreachable: there is no length or
// buffer parameter. Observable equivalent: extra arguments beyond the single
// declared one are ignored identically by both (varargs-style overcall), and
// zero-length input has no representation.
// ---------------------------------------------------------------------------
#[test]
fn err_row10_no_length_parameter() {
    type FnManyArgs = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int);
    let mut rng = Rng::new(0xC0FFEE_10);
    for &name in SYMBOLS.iter() {
        for _ in 0..32 {
            let a = rng.range_u8(0, 0xff) as c_char;
            let junk = [
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            ];
            let base_c = out_of(Impl::C, name, a);
            let base_r = out_of(Impl::Rust, name, a);
            assert_eq!(base_c, base_r, "{name}({a}) diverged");

            let fc: FnManyArgs = unsafe { std::mem::transmute(sym_char(Impl::C, name)) };
            let fr: FnManyArgs = unsafe { std::mem::transmute(sym_char(Impl::Rust, name)) };
            let x = a as c_int;
            let oc = String::from_utf8_lossy(&capture(|| unsafe {
                fc(x, junk[0], junk[1], junk[2], junk[3], junk[4])
            }))
            .into_owned();
            let or = String::from_utf8_lossy(&capture(|| unsafe {
                fr(x, junk[0], junk[1], junk[2], junk[3], junk[4])
            }))
            .into_owned();
            assert_eq!(oc, or, "{name} overcall diverged: C={oc:?} Rust={or:?}");
            assert_eq!(oc, base_c, "{name}: extra arguments changed C's behaviour");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11: both functions are `void`; a caller that reads a return value must
// see the same thing from both. Exercised by calling through an
// `fn(c_char) -> c_int` view: whatever garbage is in the return register, the
// call must not error, and the printed side effect must still match.
// ---------------------------------------------------------------------------
#[test]
fn err_row11_void_return() {
    type FnRet = unsafe extern "C" fn(c_char) -> c_int;
    let mut rng = Rng::new(0xC0FFEE_11);
    for &name in SYMBOLS.iter() {
        for _ in 0..32 {
            let a = rng.range_u8(0, 0xff) as c_char;
            let fc: FnRet = unsafe { std::mem::transmute(sym_char(Impl::C, name)) };
            let fr: FnRet = unsafe { std::mem::transmute(sym_char(Impl::Rust, name)) };
            let mut rc = 0;
            let mut rr = 0;
            let oc = capture(|| rc = unsafe { fc(a) });
            let or = capture(|| rr = unsafe { fr(a) });
            assert_eq!(
                oc,
                or,
                "{name}(0x{:02x}) side effect diverged",
                a as u8
            );
            // Both are `void`, so no error code can be returned. The printed
            // byte count that `printf` returns is not propagated by either.
            let _ = (rc, rr);
        }
    }
}

// ---------------------------------------------------------------------------
// Generic boundary sweep: every value one step past every documented range
// edge, for both entry points, in both ABI views.
// ---------------------------------------------------------------------------
#[test]
fn err_generic_boundaries() {
    const EDGES: [i32; 18] = [
        i32::MIN,
        i32::MIN + 1,
        -65537,
        -65536,
        -257,
        -256,
        -255,
        -129,
        -128,
        -1,
        0,
        1,
        127,
        128,
        129,
        255,
        256,
        i32::MAX,
    ];
    for &name in SYMBOLS.iter() {
        for v in EDGES {
            assert_same_int(name, v);
            assert_same_char(name, v as u8 as c_char);
        }
    }
}
