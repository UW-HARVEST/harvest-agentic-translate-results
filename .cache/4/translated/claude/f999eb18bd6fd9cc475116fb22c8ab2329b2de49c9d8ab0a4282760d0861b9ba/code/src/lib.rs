//! Rust translation of `c_src/src/lib.c`.
//!
//! The C library is compiled as one shared object whose only exported (public)
//! symbol is `get_predict_func` (see `c_src/include/lib.h`). Everything else in
//! the translation unit is `static` (internal linkage), so it must NOT be
//! exported here either.
//!
//! The whole translation unit is reproduced faithfully — including the
//! internal-linkage predictor routines and the function-pointer dispatch table
//! that `get_predict_func` inspects — because `get_predict_func`'s result is
//! defined in terms of the *identity* of those function pointers.
//!
//! Bug-for-bug fidelity notes (do NOT "fix" these):
//!   * `BTAC1C2_PredictSample_Pfn10` shifts by 3 whereas `case 10:` of
//!     `BTAC1C2_PredictSample` shifts by 4.
//!   * `BTAC1C2_PredictSample_Pfn11` shifts by 1 whereas `case 11:` of
//!     `BTAC1C2_PredictSample` shifts by 3.
//!   * `pfcn`/`ridx` are unused by most of the `Pfn*` helpers.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// typedefs / struct layout from lib.c
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub type btac1c_u16 = u16;
#[allow(dead_code)]
pub type btac1c_s16 = i16;
#[allow(dead_code)]
pub type btac1c_byte = u8;

/// `struct btac1c_idxstate_s` — same layout/ordering as the C definition.
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

/// The common signature shared by every predictor routine in the C file.
type PredictFn =
    unsafe extern "C" fn(psamp: *mut c_int, idx: c_int, pfcn: c_int, ridx: *mut btac1c_idxstate) -> c_int;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `psamp[(i - n) & 7]` exactly as C computes it: the subtraction is performed
/// in `int` (wrapping on overflow, as gcc does) and the mask of a possibly
/// negative value yields a two's-complement result in `0..=7`.
#[inline(always)]
unsafe fn s(psamp: *const c_int, i: c_int, n: c_int) -> c_int {
    let k = i.wrapping_sub(n) & 7;
    unsafe { *psamp.offset(k as isize) }
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
                pred = 3i32.wrapping_mul(s(psamp, i, 1)).wrapping_sub(s(psamp, i, 2)) >> 1;
            }
            3 => {
                pred = 5i32.wrapping_mul(s(psamp, i, 1)).wrapping_sub(s(psamp, i, 2)) >> 2;
            }
            4 => {
                p0 = s(psamp, i, 1).wrapping_add(s(psamp, i, 2));
                p1 = s(psamp, i, 2).wrapping_add(s(psamp, i, 3));
                pred = p0.wrapping_sub(p1 >> 1);
            }
            5 => {
                p0 = s(psamp, i, 1).wrapping_add(s(psamp, i, 2));
                p1 = s(psamp, i, 2).wrapping_add(s(psamp, i, 3));
                pred = 3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2;
            }
            6 => {
                p0 = s(psamp, i, 1).wrapping_add(s(psamp, i, 2));
                p1 = s(psamp, i, 2).wrapping_add(s(psamp, i, 3));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3;
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
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 4;
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
                pred = p0.wrapping_add(p1) >> 3;
            }
            12 | 13 | 14 | 15 => {
                let fx = &(*ridx).firfx[(pfcn - 12) as usize];
                pred = ((fx[0] as c_int).wrapping_mul(s(psamp, i, 1)))
                    .wrapping_add((fx[1] as c_int).wrapping_mul(s(psamp, i, 2)))
                    .wrapping_add((fx[2] as c_int).wrapping_mul(s(psamp, i, 3)))
                    .wrapping_add((fx[3] as c_int).wrapping_mul(s(psamp, i, 4)))
                    .wrapping_add((fx[4] as c_int).wrapping_mul(s(psamp, i, 5)))
                    .wrapping_add((fx[5] as c_int).wrapping_mul(s(psamp, i, 6)))
                    .wrapping_add((fx[6] as c_int).wrapping_mul(s(psamp, i, 7)))
                    .wrapping_add((fx[7] as c_int).wrapping_mul(s(psamp, i, 8)))
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
// static int BTAC1C2_PredictSample_Pfn0 .. _Pfn11
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
    unsafe {
        2i32.wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(s(psamp, idx, 2))
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        3i32.wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(s(psamp, idx, 2))
            >> 1
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        5i32.wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(s(psamp, idx, 2))
            >> 2
    }
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
        3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2
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
        5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
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
        // NOTE: shift of 3 here (the big switch's case 10 uses 4) — kept as-is.
        5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
    }
}

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
        // NOTE: shift of 1 here (the big switch's case 11 uses 3) — kept as-is.
        p0.wrapping_add(p1) >> 1
    }
}

// ---------------------------------------------------------------------------
// static void *BTAC1C2_GetPredictFunc(int pfcn)
// ---------------------------------------------------------------------------

#[inline(never)]
fn BTAC1C2_GetPredictFunc(pfcn: c_int) -> *mut c_void {
    let fcn: PredictFn = match pfcn {
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
    fcn as *mut c_void
}

// ---------------------------------------------------------------------------
// int get_predict_func(int pfcn)  -- the library's sole public symbol
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn: *mut c_void = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => result = (fcn == BTAC1C2_PredictSample_Pfn0 as *mut c_void) as c_int,
        1 => result = (fcn == BTAC1C2_PredictSample_Pfn1 as *mut c_void) as c_int,
        2 => result = (fcn == BTAC1C2_PredictSample_Pfn2 as *mut c_void) as c_int,
        3 => result = (fcn == BTAC1C2_PredictSample_Pfn3 as *mut c_void) as c_int,
        4 => result = (fcn == BTAC1C2_PredictSample_Pfn4 as *mut c_void) as c_int,
        5 => result = (fcn == BTAC1C2_PredictSample_Pfn5 as *mut c_void) as c_int,
        6 => result = (fcn == BTAC1C2_PredictSample_Pfn6 as *mut c_void) as c_int,
        7 => result = (fcn == BTAC1C2_PredictSample_Pfn7 as *mut c_void) as c_int,
        8 => result = (fcn == BTAC1C2_PredictSample_Pfn8 as *mut c_void) as c_int,
        9 => result = (fcn == BTAC1C2_PredictSample_Pfn9 as *mut c_void) as c_int,
        10 => result = (fcn == BTAC1C2_PredictSample_Pfn10 as *mut c_void) as c_int,
        11 => result = (fcn == BTAC1C2_PredictSample_Pfn11 as *mut c_void) as c_int,
        _ => {}
    }
    result
}
