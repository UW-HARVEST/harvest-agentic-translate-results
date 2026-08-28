//! Rust translation of c_src/src/lib.c (BTAC1C2 sample predictors).
//!
//! The only symbol exported by the C shared library is `call_predict`; all of
//! the predictor helpers are `static` in C and therefore internal here as well.
//! Their addresses are still observable through `call_predict`, so they are all
//! translated faithfully (including the discrepancies between the `switch`-based
//! `BTAC1C2_PredictSample` and the specialised `..._PfnN` variants, which are
//! reproduced exactly as written in the C source).

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_int;
use std::os::raw::c_void;

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

/// `psamp[(i - k) & 7]`
#[inline(always)]
unsafe fn ps(psamp: *const c_int, i: c_int, k: c_int) -> c_int {
    let idx = i.wrapping_sub(k) & 7;
    unsafe { *psamp.offset(idx as isize) }
}

/// Signature of the C predictor functions.
type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

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
                p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = p0.wrapping_sub(p1 >> 1);
            }
            5 => {
                p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = 3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2;
            }
            6 => {
                p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3;
            }
            7 => {
                pred = 18i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(4i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(3i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(2i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(1i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_div(16);
            }
            8 => {
                pred = 72i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(16i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(12i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(8i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_sub(3i32.wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add(3i32.wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_sub(1i32.wrapping_mul(ps(psamp, i, 8)))
                    .wrapping_div(64);
            }
            9 => {
                pred = 76i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(17i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(10i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(7i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_sub(4i32.wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add(4i32.wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_sub(3i32.wrapping_mul(ps(psamp, i, 8)))
                    .wrapping_div(64);
            }
            10 => {
                p0 = ps(psamp, i, 1)
                    .wrapping_add(ps(psamp, i, 2))
                    .wrapping_add(ps(psamp, i, 3))
                    .wrapping_add(ps(psamp, i, 4));
                p1 = ps(psamp, i, 5)
                    .wrapping_add(ps(psamp, i, 6))
                    .wrapping_add(ps(psamp, i, 7))
                    .wrapping_add(ps(psamp, i, 8));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 4;
            }
            11 => {
                p0 = ps(psamp, i, 1)
                    .wrapping_add(ps(psamp, i, 2))
                    .wrapping_add(ps(psamp, i, 3))
                    .wrapping_add(ps(psamp, i, 4));
                p1 = ps(psamp, i, 5)
                    .wrapping_add(ps(psamp, i, 6))
                    .wrapping_add(ps(psamp, i, 7))
                    .wrapping_add(ps(psamp, i, 8));
                pred = p0.wrapping_add(p1) >> 3;
            }
            12 | 13 | 14 | 15 => {
                let row = &(*ridx).firfx[(pfcn - 12) as usize];
                pred = (row[0] as c_int)
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_add((row[1] as c_int).wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add((row[2] as c_int).wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_add((row[3] as c_int).wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add((row[4] as c_int).wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_add((row[5] as c_int).wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add((row[6] as c_int).wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_add((row[7] as c_int).wrapping_mul(ps(psamp, i, 8)))
                    .wrapping_div(256);
            }
            _ => {
                pred = 0;
            }
        }
    }
    pred
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { ps(psamp, idx, 1) }
}

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
            .wrapping_add(1i32.wrapping_mul(ps(psamp, idx, 5)))
            .wrapping_div(16)
    }
}

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
            .wrapping_sub(1i32.wrapping_mul(ps(psamp, idx, 8)))
            .wrapping_div(64)
    }
}

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
        // NOTE: the C source shifts by 3 here (whereas case 10 of the big
        // switch shifts by 4); reproduced verbatim.
        5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
    }
}

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
        // NOTE: the C source shifts by 1 here (whereas case 11 of the big
        // switch shifts by 3); reproduced verbatim.
        p0.wrapping_add(p1) >> 1
    }
}

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

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
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
