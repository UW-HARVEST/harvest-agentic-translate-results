//! Rust translation of c_src/src/lib.c
//!
//! Public ABI surface (matches `nm -D` on the C shared library):
//!   - `get_predict_func`
//!
//! Everything else in the C source has internal (`static`) linkage and is kept
//! private here as well. The behaviour of the original C — including the places
//! where the standalone `_PfnNN` helpers disagree with the corresponding `case`
//! arms of `BTAC1C2_PredictSample` (cases 10 and 11) — is reproduced verbatim.
//! Those discrepancies are NOT fixed.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_int;
use std::os::raw::{c_uchar, c_ushort, c_void};

type btac1c_u16 = c_ushort;
type btac1c_s16 = i16;
type btac1c_byte = c_uchar;

/// Mirrors `struct btac1c_idxstate_s`. Layout must match the C struct because
/// callers may hand us a pointer to one.
#[repr(C)]
pub struct btac1c_idxstate {
    pub idx: btac1c_u16,
    pub lpred: btac1c_s16,
    pub rpred: btac1c_s16,
    pub tag: btac1c_byte,
    pub bcfcn: btac1c_byte,
    pub bsfcn: btac1c_byte,
    pub usefx: btac1c_byte,
    pub firfx: [[btac1c_s16; 8]; 4],
}

/// Signature shared by every predictor entry point.
type PredictFunc =
    unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

/// `psamp[(i - n) & 7]` in the original C. `i - n` is computed with wrapping
/// `int` arithmetic and the mask makes the result a valid 0..=7 index, exactly
/// as in C (where `& 7` on a negative two's-complement value yields the low
/// three bits).
#[inline(always)]
unsafe fn s(psamp: *const c_int, i: c_int, n: c_int) -> c_int {
    let idx = (i.wrapping_sub(n) & 7) as usize;
    unsafe { *psamp.add(idx) }
}

// ---------------------------------------------------------------------------
// static int BTAC1C2_PredictSample(...)
// ---------------------------------------------------------------------------

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let pred: c_int;
    let p0: c_int;
    let p1: c_int;
    let i: c_int = idx;
    unsafe {
        match pfcn {
            0 => {
                pred = s(psamp, i, 1);
            }
            1 => {
                pred = 2i32.wrapping_mul(s(psamp, i, 1)).wrapping_sub(s(psamp, i, 2));
            }
            2 => {
                pred = (3i32.wrapping_mul(s(psamp, i, 1)).wrapping_sub(s(psamp, i, 2))) >> 1;
            }
            3 => {
                pred = (5i32.wrapping_mul(s(psamp, i, 1)).wrapping_sub(s(psamp, i, 2))) >> 2;
            }
            4 => {
                p0 = s(psamp, i, 1).wrapping_add(s(psamp, i, 2));
                p1 = s(psamp, i, 2).wrapping_add(s(psamp, i, 3));
                pred = p0.wrapping_sub(p1 >> 1);
            }
            5 => {
                p0 = s(psamp, i, 1).wrapping_add(s(psamp, i, 2));
                p1 = s(psamp, i, 2).wrapping_add(s(psamp, i, 3));
                pred = (3i32.wrapping_mul(p0).wrapping_sub(p1)) >> 2;
            }
            6 => {
                p0 = s(psamp, i, 1).wrapping_add(s(psamp, i, 2));
                p1 = s(psamp, i, 2).wrapping_add(s(psamp, i, 3));
                pred = (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3;
            }
            7 => {
                pred = (18i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(4i32.wrapping_mul(s(psamp, i, 2)))
                    .wrapping_add(3i32.wrapping_mul(s(psamp, i, 3)))
                    .wrapping_sub(2i32.wrapping_mul(s(psamp, i, 4)))
                    .wrapping_add(1i32.wrapping_mul(s(psamp, i, 5))))
                    .wrapping_div(16);
            }
            8 => {
                pred = (72i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(16i32.wrapping_mul(s(psamp, i, 2)))
                    .wrapping_add(12i32.wrapping_mul(s(psamp, i, 3)))
                    .wrapping_sub(8i32.wrapping_mul(s(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(s(psamp, i, 5)))
                    .wrapping_sub(3i32.wrapping_mul(s(psamp, i, 6)))
                    .wrapping_add(3i32.wrapping_mul(s(psamp, i, 7)))
                    .wrapping_sub(1i32.wrapping_mul(s(psamp, i, 8))))
                    .wrapping_div(64);
            }
            9 => {
                pred = (76i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(17i32.wrapping_mul(s(psamp, i, 2)))
                    .wrapping_add(10i32.wrapping_mul(s(psamp, i, 3)))
                    .wrapping_sub(7i32.wrapping_mul(s(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(s(psamp, i, 5)))
                    .wrapping_sub(4i32.wrapping_mul(s(psamp, i, 6)))
                    .wrapping_add(4i32.wrapping_mul(s(psamp, i, 7)))
                    .wrapping_sub(3i32.wrapping_mul(s(psamp, i, 8))))
                    .wrapping_div(64);
            }
            10 => {
                p0 = s(psamp, i, 1)
                    .wrapping_add(s(psamp, i, 2))
                    .wrapping_add(s(psamp, i, 3))
                    .wrapping_add(s(psamp, i, 4));
                p1 = s(psamp, i, 5)
                    .wrapping_add(s(psamp, i, 6))
                    .wrapping_add(s(psamp, i, 7))
                    .wrapping_add(s(psamp, i, 8));
                pred = (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 4;
            }
            11 => {
                p0 = s(psamp, i, 1)
                    .wrapping_add(s(psamp, i, 2))
                    .wrapping_add(s(psamp, i, 3))
                    .wrapping_add(s(psamp, i, 4));
                p1 = s(psamp, i, 5)
                    .wrapping_add(s(psamp, i, 6))
                    .wrapping_add(s(psamp, i, 7))
                    .wrapping_add(s(psamp, i, 8));
                pred = (p0.wrapping_add(p1)) >> 3;
            }
            12 | 13 | 14 | 15 => {
                let fir = &(*ridx).firfx[(pfcn - 12) as usize];
                pred = ((fir[0] as c_int).wrapping_mul(s(psamp, i, 1)))
                    .wrapping_add((fir[1] as c_int).wrapping_mul(s(psamp, i, 2)))
                    .wrapping_add((fir[2] as c_int).wrapping_mul(s(psamp, i, 3)))
                    .wrapping_add((fir[3] as c_int).wrapping_mul(s(psamp, i, 4)))
                    .wrapping_add((fir[4] as c_int).wrapping_mul(s(psamp, i, 5)))
                    .wrapping_add((fir[5] as c_int).wrapping_mul(s(psamp, i, 6)))
                    .wrapping_add((fir[6] as c_int).wrapping_mul(s(psamp, i, 7)))
                    .wrapping_add((fir[7] as c_int).wrapping_mul(s(psamp, i, 8)))
                    .wrapping_div(256);
            }
            _ => {
                pred = 0;
            }
        }
    }
    pred
}

// ---------------------------------------------------------------------------
// The specialised per-function-number predictors.
//
// `pfcn` and `ridx` are unused in each of these, exactly as in the C.
// ---------------------------------------------------------------------------

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { s(psamp, idx, 1) }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { 2i32.wrapping_mul(s(psamp, idx, 1)).wrapping_sub(s(psamp, idx, 2)) }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { (3i32.wrapping_mul(s(psamp, idx, 1)).wrapping_sub(s(psamp, idx, 2))) >> 1 }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { (5i32.wrapping_mul(s(psamp, idx, 1)).wrapping_sub(s(psamp, idx, 2))) >> 2 }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = s(psamp, idx, 1).wrapping_add(s(psamp, idx, 2));
        let p1 = s(psamp, idx, 2).wrapping_add(s(psamp, idx, 3));
        p0.wrapping_sub(p1 >> 1)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = s(psamp, idx, 1).wrapping_add(s(psamp, idx, 2));
        let p1 = s(psamp, idx, 2).wrapping_add(s(psamp, idx, 3));
        (3i32.wrapping_mul(p0).wrapping_sub(p1)) >> 2
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = s(psamp, idx, 1).wrapping_add(s(psamp, idx, 2));
        let p1 = s(psamp, idx, 2).wrapping_add(s(psamp, idx, 3));
        (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (18i32
            .wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(4i32.wrapping_mul(s(psamp, idx, 2)))
            .wrapping_add(3i32.wrapping_mul(s(psamp, idx, 3)))
            .wrapping_sub(2i32.wrapping_mul(s(psamp, idx, 4)))
            .wrapping_add(1i32.wrapping_mul(s(psamp, idx, 5))))
        .wrapping_div(16)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (72i32
            .wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(16i32.wrapping_mul(s(psamp, idx, 2)))
            .wrapping_add(12i32.wrapping_mul(s(psamp, idx, 3)))
            .wrapping_sub(8i32.wrapping_mul(s(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(s(psamp, idx, 5)))
            .wrapping_sub(3i32.wrapping_mul(s(psamp, idx, 6)))
            .wrapping_add(3i32.wrapping_mul(s(psamp, idx, 7)))
            .wrapping_sub(1i32.wrapping_mul(s(psamp, idx, 8))))
        .wrapping_div(64)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (76i32
            .wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(17i32.wrapping_mul(s(psamp, idx, 2)))
            .wrapping_add(10i32.wrapping_mul(s(psamp, idx, 3)))
            .wrapping_sub(7i32.wrapping_mul(s(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(s(psamp, idx, 5)))
            .wrapping_sub(4i32.wrapping_mul(s(psamp, idx, 6)))
            .wrapping_add(4i32.wrapping_mul(s(psamp, idx, 7)))
            .wrapping_sub(3i32.wrapping_mul(s(psamp, idx, 8))))
        .wrapping_div(64)
    }
}

/// NOTE: the C uses `>> 3` here while `case 10` of `BTAC1C2_PredictSample`
/// uses `>> 4`. Reproduced as-is.
#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = s(psamp, idx, 1)
            .wrapping_add(s(psamp, idx, 2))
            .wrapping_add(s(psamp, idx, 3))
            .wrapping_add(s(psamp, idx, 4));
        let p1 = s(psamp, idx, 5)
            .wrapping_add(s(psamp, idx, 6))
            .wrapping_add(s(psamp, idx, 7))
            .wrapping_add(s(psamp, idx, 8));
        (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3
    }
}

/// NOTE: the C uses `>> 1` here while `case 11` of `BTAC1C2_PredictSample`
/// uses `>> 3`. Reproduced as-is.
#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = s(psamp, idx, 1)
            .wrapping_add(s(psamp, idx, 2))
            .wrapping_add(s(psamp, idx, 3))
            .wrapping_add(s(psamp, idx, 4));
        let p1 = s(psamp, idx, 5)
            .wrapping_add(s(psamp, idx, 6))
            .wrapping_add(s(psamp, idx, 7))
            .wrapping_add(s(psamp, idx, 8));
        (p0.wrapping_add(p1)) >> 1
    }
}

// ---------------------------------------------------------------------------
// static void *BTAC1C2_GetPredictFunc(int pfcn)
// ---------------------------------------------------------------------------

#[inline(never)]
fn BTAC1C2_GetPredictFunc(pfcn: c_int) -> *const c_void {
    let fcn: PredictFunc = match pfcn {
        0 => BTAC1C2_PredictSample_Pfn0,
        1 => BTAC1C2_PredictSample_Pfn1,
        2 => BTAC1C2_PredictSample_Pfn2,
        3 => BTAC1C2_PredictSample_Pfn3,
        4 => BTAC1C2_PredictSample_Pfn4,
        5 => BTAC1C2_PredictSample_Pfn5,
        6 => BTAC1C2_PredictSample_Pfn6,
        7 => BTAC1C2_PredictSample_Pfn7,
        8 => BTAC1C2_PredictSample_Pfn8,
        9 => BTAC1C2_PredictSample_Pfn9,
        10 => BTAC1C2_PredictSample_Pfn10,
        11 => BTAC1C2_PredictSample_Pfn11,
        _ => BTAC1C2_PredictSample,
    };
    fcn as *const c_void
}

/// Address of a predictor, for the pointer-identity comparisons below.
#[inline(always)]
fn addr(f: PredictFunc) -> *const c_void {
    f as *const c_void
}

// ---------------------------------------------------------------------------
// int get_predict_func(int pfcn)   <-- the only exported symbol
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn0)) as c_int,
        1 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn1)) as c_int,
        2 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn2)) as c_int,
        3 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn3)) as c_int,
        4 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn4)) as c_int,
        5 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn5)) as c_int,
        6 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn6)) as c_int,
        7 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn7)) as c_int,
        8 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn8)) as c_int,
        9 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn9)) as c_int,
        10 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn10)) as c_int,
        11 => result = (fcn == addr(BTAC1C2_PredictSample_Pfn11)) as c_int,
        _ => {}
    }
    result
}
