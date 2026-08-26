//! Rust translation of c_src/src/lib.c
//!
//! The C translation unit contains a family of `static` sample-prediction
//! helpers, a `static` dispatcher that hands back a `void *` to one of them,
//! and exactly one public (exported) function: `call_predict`.
//!
//! Only `call_predict` appears in the C shared library's dynamic symbol table
//! (`nm -D`), so it is the only `#[unsafe(no_mangle)] extern "C"` item here.
//! Everything else is translated faithfully but kept internal, exactly like the
//! C `static` functions.
//!
//! Behaviour is reproduced verbatim, including the quirks of the original code
//! (e.g. `*_Pfn10` / `*_Pfn11` using different shift amounts than the
//! corresponding `case 10` / `case 11` arms of `BTAC1C2_PredictSample`). Those
//! discrepancies are *not* fixed.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use core::ffi::{c_int, c_void};

// typedef unsigned short btac1c_u16;
type btac1c_u16 = u16;
// typedef signed short btac1c_s16;
type btac1c_s16 = i16;
// typedef unsigned char btac1c_byte;
type btac1c_byte = u8;

/// struct btac1c_idxstate_s
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

/// Signature shared by every predictor in the C file:
/// `int (*)(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx)`
type PredictFn =
    unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

/// `psamp[(i - n) & 7]` — the mask keeps the index inside 0..=7 for every
/// possible `int` value, mirroring C's two's-complement `&`.
#[inline(always)]
unsafe fn s(psamp: *const c_int, i: c_int, n: c_int) -> i32 {
    let off = (i.wrapping_sub(n) & 7) as usize;
    unsafe { *psamp.add(off) }
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
    let pred: i32;
    let p0: i32;
    let p1: i32;
    let i: c_int = idx;

    unsafe {
        match pfcn {
            0 => {
                pred = s(psamp, i, 1);
            }
            1 => {
                pred = 2i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(s(psamp, i, 2));
            }
            2 => {
                pred = 3i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(s(psamp, i, 2))
                    >> 1;
            }
            3 => {
                pred = 5i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(s(psamp, i, 2))
                    >> 2;
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
                pred = 18i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(4i32.wrapping_mul(s(psamp, i, 2)))
                    .wrapping_add(3i32.wrapping_mul(s(psamp, i, 3)))
                    .wrapping_sub(2i32.wrapping_mul(s(psamp, i, 4)))
                    .wrapping_add(1i32.wrapping_mul(s(psamp, i, 5)))
                    .wrapping_div(16);
            }
            8 => {
                pred = 72i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(16i32.wrapping_mul(s(psamp, i, 2)))
                    .wrapping_add(12i32.wrapping_mul(s(psamp, i, 3)))
                    .wrapping_sub(8i32.wrapping_mul(s(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(s(psamp, i, 5)))
                    .wrapping_sub(3i32.wrapping_mul(s(psamp, i, 6)))
                    .wrapping_add(3i32.wrapping_mul(s(psamp, i, 7)))
                    .wrapping_sub(1i32.wrapping_mul(s(psamp, i, 8)))
                    .wrapping_div(64);
            }
            9 => {
                pred = 76i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(17i32.wrapping_mul(s(psamp, i, 2)))
                    .wrapping_add(10i32.wrapping_mul(s(psamp, i, 3)))
                    .wrapping_sub(7i32.wrapping_mul(s(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(s(psamp, i, 5)))
                    .wrapping_sub(4i32.wrapping_mul(s(psamp, i, 6)))
                    .wrapping_add(4i32.wrapping_mul(s(psamp, i, 7)))
                    .wrapping_sub(3i32.wrapping_mul(s(psamp, i, 8)))
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
                let mut acc: i32 = 0;
                acc = acc.wrapping_add((fx[0] as i32).wrapping_mul(s(psamp, i, 1)));
                acc = acc.wrapping_add((fx[1] as i32).wrapping_mul(s(psamp, i, 2)));
                acc = acc.wrapping_add((fx[2] as i32).wrapping_mul(s(psamp, i, 3)));
                acc = acc.wrapping_add((fx[3] as i32).wrapping_mul(s(psamp, i, 4)));
                acc = acc.wrapping_add((fx[4] as i32).wrapping_mul(s(psamp, i, 5)));
                acc = acc.wrapping_add((fx[5] as i32).wrapping_mul(s(psamp, i, 6)));
                acc = acc.wrapping_add((fx[6] as i32).wrapping_mul(s(psamp, i, 7)));
                acc = acc.wrapping_add((fx[7] as i32).wrapping_mul(s(psamp, i, 8)));
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
// The individual, specialised predictors (all `static` in C).
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
        18i32
            .wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(4i32.wrapping_mul(s(psamp, idx, 2)))
            .wrapping_add(3i32.wrapping_mul(s(psamp, idx, 3)))
            .wrapping_sub(2i32.wrapping_mul(s(psamp, idx, 4)))
            .wrapping_add(1i32.wrapping_mul(s(psamp, idx, 5)))
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
            .wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(16i32.wrapping_mul(s(psamp, idx, 2)))
            .wrapping_add(12i32.wrapping_mul(s(psamp, idx, 3)))
            .wrapping_sub(8i32.wrapping_mul(s(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(s(psamp, idx, 5)))
            .wrapping_sub(3i32.wrapping_mul(s(psamp, idx, 6)))
            .wrapping_add(3i32.wrapping_mul(s(psamp, idx, 7)))
            .wrapping_sub(1i32.wrapping_mul(s(psamp, idx, 8)))
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
            .wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(17i32.wrapping_mul(s(psamp, idx, 2)))
            .wrapping_add(10i32.wrapping_mul(s(psamp, idx, 3)))
            .wrapping_sub(7i32.wrapping_mul(s(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(s(psamp, idx, 5)))
            .wrapping_sub(4i32.wrapping_mul(s(psamp, idx, 6)))
            .wrapping_add(4i32.wrapping_mul(s(psamp, idx, 7)))
            .wrapping_sub(3i32.wrapping_mul(s(psamp, idx, 8)))
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
        // NOTE: the C source shifts by 3 here (unlike `case 10` above, which
        // shifts by 4). Reproduced as-is.
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
        // NOTE: the C source shifts by 1 here (unlike `case 11` above, which
        // shifts by 3). Reproduced as-is.
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

#[inline(always)]
fn fptr(f: PredictFn) -> *mut c_void {
    f as *mut c_void
}

// ---------------------------------------------------------------------------
// int call_predict(int pfcn)   <-- the only exported symbol
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn: *mut c_void = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn0)) as c_int,
        1 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn1)) as c_int,
        2 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn2)) as c_int,
        3 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn3)) as c_int,
        4 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn4)) as c_int,
        5 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn5)) as c_int,
        6 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn6)) as c_int,
        7 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn7)) as c_int,
        8 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn8)) as c_int,
        9 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn9)) as c_int,
        10 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn10)) as c_int,
        11 => result = (fcn == fptr(BTAC1C2_PredictSample_Pfn11)) as c_int,
        _ => {}
    }
    result
}
