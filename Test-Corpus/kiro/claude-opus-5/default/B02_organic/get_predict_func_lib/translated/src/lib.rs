//! Rust translation of `c_src/src/lib.c`.
//!
//! The C file defines a family of sample-prediction routines plus a
//! selector (`BTAC1C2_GetPredictFunc`) that hands back a function pointer,
//! and one exported entry point (`get_predict_func`) that checks whether the
//! selector returned the pointer expected for the given predictor number.
//!
//! The translation keeps the original structure -- real function pointers are
//! produced and compared by address -- so any quirk of the C control flow is
//! reproduced rather than folded into a constant. Arithmetic quirks/bugs in the
//! individual predictors (e.g. the `Pfn10`/`Pfn11` shift counts differing from
//! the corresponding `switch` arms) are preserved verbatim.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};

type btac1c_u16 = u16;
type btac1c_s16 = i16;
type btac1c_byte = u8;

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

/// Signature shared by every predictor function in the C source.
type PredictFn =
    unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

/// `psamp[(base - off) & 7]`, matching C's two's-complement `&` on a possibly
/// negative index (which always lands in `0..=7`).
#[inline(always)]
unsafe fn ps(psamp: *const c_int, base: c_int, off: c_int) -> c_int {
    let i = base.wrapping_sub(off) & 7;
    unsafe { *psamp.offset(i as isize) }
}

// ---------------------------------------------------------------------------
// BTAC1C2_PredictSample -- the generic switch-based predictor.
// ---------------------------------------------------------------------------

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let i = idx;
    let pred: c_int;
    unsafe {
        match pfcn {
            0 => {
                pred = ps(psamp, i, 1);
            }
            1 => {
                pred = 2i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(ps(psamp, i, 2));
            }
            2 => {
                pred = 3i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(ps(psamp, i, 2))
                    >> 1;
            }
            3 => {
                pred = 5i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(ps(psamp, i, 2))
                    >> 2;
            }
            4 => {
                let p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                let p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = p0.wrapping_sub(p1 >> 1);
            }
            5 => {
                let p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                let p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = 3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2;
            }
            6 => {
                let p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                let p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3;
            }
            7 => {
                let acc = 18i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(4i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(3i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(2i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(ps(psamp, i, 5));
                pred = acc.wrapping_div(16);
            }
            8 => {
                let acc = 72i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(16i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(12i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(8i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_sub(3i32.wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add(3i32.wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_sub(ps(psamp, i, 8));
                pred = acc.wrapping_div(64);
            }
            9 => {
                let acc = 76i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(17i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(10i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(7i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_sub(4i32.wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add(4i32.wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_sub(3i32.wrapping_mul(ps(psamp, i, 8)));
                pred = acc.wrapping_div(64);
            }
            10 => {
                let p0 = ps(psamp, i, 1)
                    .wrapping_add(ps(psamp, i, 2))
                    .wrapping_add(ps(psamp, i, 3))
                    .wrapping_add(ps(psamp, i, 4));
                let p1 = ps(psamp, i, 5)
                    .wrapping_add(ps(psamp, i, 6))
                    .wrapping_add(ps(psamp, i, 7))
                    .wrapping_add(ps(psamp, i, 8));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 4;
            }
            11 => {
                let p0 = ps(psamp, i, 1)
                    .wrapping_add(ps(psamp, i, 2))
                    .wrapping_add(ps(psamp, i, 3))
                    .wrapping_add(ps(psamp, i, 4));
                let p1 = ps(psamp, i, 5)
                    .wrapping_add(ps(psamp, i, 6))
                    .wrapping_add(ps(psamp, i, 7))
                    .wrapping_add(ps(psamp, i, 8));
                pred = p0.wrapping_add(p1) >> 3;
            }
            12 | 13 | 14 | 15 => {
                let row = &(*ridx).firfx[(pfcn - 12) as usize];
                let mut acc: c_int = 0;
                for k in 0..8usize {
                    acc = acc.wrapping_add(
                        (row[k] as c_int).wrapping_mul(ps(psamp, i, (k + 1) as c_int)),
                    );
                }
                pred = acc.wrapping_div(256);
            }
            _ => {
                pred = 0;
            }
        }
    }
    pred
}

// ---------------------------------------------------------------------------
// Specialized predictors. Each must stay a distinct symbol: `get_predict_func`
// compares addresses, so folding two of these together would change behavior.
// ---------------------------------------------------------------------------

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { ps(psamp, idx, 1) }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        2i32.wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(ps(psamp, idx, 2))
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
        3i32.wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(ps(psamp, idx, 2))
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
        5i32.wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(ps(psamp, idx, 2))
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
        let p0 = ps(psamp, idx, 1).wrapping_add(ps(psamp, idx, 2));
        let p1 = ps(psamp, idx, 2).wrapping_add(ps(psamp, idx, 3));
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
        let p0 = ps(psamp, idx, 1).wrapping_add(ps(psamp, idx, 2));
        let p1 = ps(psamp, idx, 2).wrapping_add(ps(psamp, idx, 3));
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
        let p0 = ps(psamp, idx, 1).wrapping_add(ps(psamp, idx, 2));
        let p1 = ps(psamp, idx, 2).wrapping_add(ps(psamp, idx, 3));
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
        18i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(4i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(3i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(2i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(ps(psamp, idx, 5))
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
        72i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(16i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(12i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(8i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(ps(psamp, idx, 5)))
            .wrapping_sub(3i32.wrapping_mul(ps(psamp, idx, 6)))
            .wrapping_add(3i32.wrapping_mul(ps(psamp, idx, 7)))
            .wrapping_sub(ps(psamp, idx, 8))
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
        76i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(17i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(10i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(7i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(ps(psamp, idx, 5)))
            .wrapping_sub(4i32.wrapping_mul(ps(psamp, idx, 6)))
            .wrapping_add(4i32.wrapping_mul(ps(psamp, idx, 7)))
            .wrapping_sub(3i32.wrapping_mul(ps(psamp, idx, 8)))
            .wrapping_div(64)
    }
}

// NOTE: the C `Pfn10` shifts by 3 while the `switch` arm for pfcn == 10 shifts
// by 4. Reproduced as-is.
#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1)
            .wrapping_add(ps(psamp, idx, 2))
            .wrapping_add(ps(psamp, idx, 3))
            .wrapping_add(ps(psamp, idx, 4));
        let p1 = ps(psamp, idx, 5)
            .wrapping_add(ps(psamp, idx, 6))
            .wrapping_add(ps(psamp, idx, 7))
            .wrapping_add(ps(psamp, idx, 8));
        5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
    }
}

// NOTE: the C `Pfn11` shifts by 1 while the `switch` arm for pfcn == 11 shifts
// by 3. Reproduced as-is.
#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1)
            .wrapping_add(ps(psamp, idx, 2))
            .wrapping_add(ps(psamp, idx, 3))
            .wrapping_add(ps(psamp, idx, 4));
        let p1 = ps(psamp, idx, 5)
            .wrapping_add(ps(psamp, idx, 6))
            .wrapping_add(ps(psamp, idx, 7))
            .wrapping_add(ps(psamp, idx, 8));
        p0.wrapping_add(p1) >> 1
    }
}

// ---------------------------------------------------------------------------
// Selector
// ---------------------------------------------------------------------------

#[inline(never)]
fn BTAC1C2_GetPredictFunc(pfcn: c_int) -> *const c_void {
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
    fcn as *const c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => result = (fcn == BTAC1C2_PredictSample_Pfn0 as *const c_void) as c_int,
        1 => result = (fcn == BTAC1C2_PredictSample_Pfn1 as *const c_void) as c_int,
        2 => result = (fcn == BTAC1C2_PredictSample_Pfn2 as *const c_void) as c_int,
        3 => result = (fcn == BTAC1C2_PredictSample_Pfn3 as *const c_void) as c_int,
        4 => result = (fcn == BTAC1C2_PredictSample_Pfn4 as *const c_void) as c_int,
        5 => result = (fcn == BTAC1C2_PredictSample_Pfn5 as *const c_void) as c_int,
        6 => result = (fcn == BTAC1C2_PredictSample_Pfn6 as *const c_void) as c_int,
        7 => result = (fcn == BTAC1C2_PredictSample_Pfn7 as *const c_void) as c_int,
        8 => result = (fcn == BTAC1C2_PredictSample_Pfn8 as *const c_void) as c_int,
        9 => result = (fcn == BTAC1C2_PredictSample_Pfn9 as *const c_void) as c_int,
        10 => result = (fcn == BTAC1C2_PredictSample_Pfn10 as *const c_void) as c_int,
        11 => result = (fcn == BTAC1C2_PredictSample_Pfn11 as *const c_void) as c_int,
        _ => {}
    }
    result
}
