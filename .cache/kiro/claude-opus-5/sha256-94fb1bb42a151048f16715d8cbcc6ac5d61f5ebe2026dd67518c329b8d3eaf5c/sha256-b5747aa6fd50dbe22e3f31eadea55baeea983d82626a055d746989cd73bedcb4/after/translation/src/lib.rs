//! Rust translation of `c_src/src/lib.c`.
//!
//! The C translation unit exposes exactly one external symbol, `call_predict`
//! (declared in `include/lib.h` is `get_predict_func`, which the C never
//! defines -- so it is intentionally not provided here either).
//!
//! Everything else in the C file is `static`; the corresponding Rust items are
//! private, but they are kept as real, address-taken functions because
//! `call_predict` observes their *addresses* rather than their results.
//!
//! Behavioural quirks of the original C are reproduced verbatim, notably:
//!   * `..._Pfn10` shifts by 3 while `case 10:` of the switch shifts by 4,
//!   * `..._Pfn11` shifts by 1 while `case 11:` of the switch shifts by 3.
//! These are not corrected.

#![allow(non_snake_case, non_camel_case_types, unused_variables)]

use std::ffi::{c_int, c_void};

type btac1c_u16 = u16;
type btac1c_s16 = i16;
type btac1c_byte = u8;

/// `struct btac1c_idxstate_s`
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

/// Common signature shared by every predictor entry point.
type PredictFn =
    unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

/// `psamp[(i - n) & 7]`, with C's two's-complement masking semantics.
#[inline(always)]
unsafe fn sample(psamp: *mut c_int, i: c_int, n: c_int) -> c_int {
    let k = (i.wrapping_sub(n)) & 7;
    unsafe { *psamp.offset(k as isize) }
}

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
                pred = sample(psamp, i, 1);
            }
            1 => {
                pred = 2i32
                    .wrapping_mul(sample(psamp, i, 1))
                    .wrapping_sub(sample(psamp, i, 2));
            }
            2 => {
                pred = 3i32
                    .wrapping_mul(sample(psamp, i, 1))
                    .wrapping_sub(sample(psamp, i, 2))
                    >> 1;
            }
            3 => {
                pred = 5i32
                    .wrapping_mul(sample(psamp, i, 1))
                    .wrapping_sub(sample(psamp, i, 2))
                    >> 2;
            }
            4 => {
                p0 = sample(psamp, i, 1).wrapping_add(sample(psamp, i, 2));
                p1 = sample(psamp, i, 2).wrapping_add(sample(psamp, i, 3));
                pred = p0.wrapping_sub(p1 >> 1);
            }
            5 => {
                p0 = sample(psamp, i, 1).wrapping_add(sample(psamp, i, 2));
                p1 = sample(psamp, i, 2).wrapping_add(sample(psamp, i, 3));
                pred = 3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2;
            }
            6 => {
                p0 = sample(psamp, i, 1).wrapping_add(sample(psamp, i, 2));
                p1 = sample(psamp, i, 2).wrapping_add(sample(psamp, i, 3));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3;
            }
            7 => {
                pred = 18i32
                    .wrapping_mul(sample(psamp, i, 1))
                    .wrapping_sub(4i32.wrapping_mul(sample(psamp, i, 2)))
                    .wrapping_add(3i32.wrapping_mul(sample(psamp, i, 3)))
                    .wrapping_sub(2i32.wrapping_mul(sample(psamp, i, 4)))
                    .wrapping_add(1i32.wrapping_mul(sample(psamp, i, 5)))
                    .wrapping_div(16);
            }
            8 => {
                pred = 72i32
                    .wrapping_mul(sample(psamp, i, 1))
                    .wrapping_sub(16i32.wrapping_mul(sample(psamp, i, 2)))
                    .wrapping_add(12i32.wrapping_mul(sample(psamp, i, 3)))
                    .wrapping_sub(8i32.wrapping_mul(sample(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(sample(psamp, i, 5)))
                    .wrapping_sub(3i32.wrapping_mul(sample(psamp, i, 6)))
                    .wrapping_add(3i32.wrapping_mul(sample(psamp, i, 7)))
                    .wrapping_sub(1i32.wrapping_mul(sample(psamp, i, 8)))
                    .wrapping_div(64);
            }
            9 => {
                pred = 76i32
                    .wrapping_mul(sample(psamp, i, 1))
                    .wrapping_sub(17i32.wrapping_mul(sample(psamp, i, 2)))
                    .wrapping_add(10i32.wrapping_mul(sample(psamp, i, 3)))
                    .wrapping_sub(7i32.wrapping_mul(sample(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(sample(psamp, i, 5)))
                    .wrapping_sub(4i32.wrapping_mul(sample(psamp, i, 6)))
                    .wrapping_add(4i32.wrapping_mul(sample(psamp, i, 7)))
                    .wrapping_sub(3i32.wrapping_mul(sample(psamp, i, 8)))
                    .wrapping_div(64);
            }
            10 => {
                p0 = sample(psamp, i, 1)
                    .wrapping_add(sample(psamp, i, 2))
                    .wrapping_add(sample(psamp, i, 3))
                    .wrapping_add(sample(psamp, i, 4));
                p1 = sample(psamp, i, 5)
                    .wrapping_add(sample(psamp, i, 6))
                    .wrapping_add(sample(psamp, i, 7))
                    .wrapping_add(sample(psamp, i, 8));
                pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 4;
            }
            11 => {
                p0 = sample(psamp, i, 1)
                    .wrapping_add(sample(psamp, i, 2))
                    .wrapping_add(sample(psamp, i, 3))
                    .wrapping_add(sample(psamp, i, 4));
                p1 = sample(psamp, i, 5)
                    .wrapping_add(sample(psamp, i, 6))
                    .wrapping_add(sample(psamp, i, 7))
                    .wrapping_add(sample(psamp, i, 8));
                pred = p0.wrapping_add(p1) >> 3;
            }
            12 | 13 | 14 | 15 => {
                let fx = &(*ridx).firfx[(pfcn - 12) as usize];
                let mut acc: c_int = 0;
                for n in 0..8usize {
                    acc = acc.wrapping_add(
                        (fx[n] as c_int).wrapping_mul(sample(psamp, i, (n + 1) as c_int)),
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

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn0(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { sample(psamp, idx, 1) }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn1(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        2i32.wrapping_mul(sample(psamp, idx, 1))
            .wrapping_sub(sample(psamp, idx, 2))
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn2(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        3i32.wrapping_mul(sample(psamp, idx, 1))
            .wrapping_sub(sample(psamp, idx, 2))
            >> 1
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn3(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        5i32.wrapping_mul(sample(psamp, idx, 1))
            .wrapping_sub(sample(psamp, idx, 2))
            >> 2
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn4(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1).wrapping_add(sample(psamp, idx, 2));
        let p1 = sample(psamp, idx, 2).wrapping_add(sample(psamp, idx, 3));
        p0.wrapping_sub(p1 >> 1)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn5(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1).wrapping_add(sample(psamp, idx, 2));
        let p1 = sample(psamp, idx, 2).wrapping_add(sample(psamp, idx, 3));
        3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn6(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1).wrapping_add(sample(psamp, idx, 2));
        let p1 = sample(psamp, idx, 2).wrapping_add(sample(psamp, idx, 3));
        5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn7(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        18i32
            .wrapping_mul(sample(psamp, idx, 1))
            .wrapping_sub(4i32.wrapping_mul(sample(psamp, idx, 2)))
            .wrapping_add(3i32.wrapping_mul(sample(psamp, idx, 3)))
            .wrapping_sub(2i32.wrapping_mul(sample(psamp, idx, 4)))
            .wrapping_add(1i32.wrapping_mul(sample(psamp, idx, 5)))
            .wrapping_div(16)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn8(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        72i32
            .wrapping_mul(sample(psamp, idx, 1))
            .wrapping_sub(16i32.wrapping_mul(sample(psamp, idx, 2)))
            .wrapping_add(12i32.wrapping_mul(sample(psamp, idx, 3)))
            .wrapping_sub(8i32.wrapping_mul(sample(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(sample(psamp, idx, 5)))
            .wrapping_sub(3i32.wrapping_mul(sample(psamp, idx, 6)))
            .wrapping_add(3i32.wrapping_mul(sample(psamp, idx, 7)))
            .wrapping_sub(1i32.wrapping_mul(sample(psamp, idx, 8)))
            .wrapping_div(64)
    }
}

#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn9(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        76i32
            .wrapping_mul(sample(psamp, idx, 1))
            .wrapping_sub(17i32.wrapping_mul(sample(psamp, idx, 2)))
            .wrapping_add(10i32.wrapping_mul(sample(psamp, idx, 3)))
            .wrapping_sub(7i32.wrapping_mul(sample(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(sample(psamp, idx, 5)))
            .wrapping_sub(4i32.wrapping_mul(sample(psamp, idx, 6)))
            .wrapping_add(4i32.wrapping_mul(sample(psamp, idx, 7)))
            .wrapping_sub(3i32.wrapping_mul(sample(psamp, idx, 8)))
            .wrapping_div(64)
    }
}

// NOTE: the original shifts by 3 here, unlike `case 10:` which shifts by 4.
#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn10(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1)
            .wrapping_add(sample(psamp, idx, 2))
            .wrapping_add(sample(psamp, idx, 3))
            .wrapping_add(sample(psamp, idx, 4));
        let p1 = sample(psamp, idx, 5)
            .wrapping_add(sample(psamp, idx, 6))
            .wrapping_add(sample(psamp, idx, 7))
            .wrapping_add(sample(psamp, idx, 8));
        5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
    }
}

// NOTE: the original shifts by 1 here, unlike `case 11:` which shifts by 3.
#[inline(never)]
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn11(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1)
            .wrapping_add(sample(psamp, idx, 2))
            .wrapping_add(sample(psamp, idx, 3))
            .wrapping_add(sample(psamp, idx, 4));
        let p1 = sample(psamp, idx, 5)
            .wrapping_add(sample(psamp, idx, 6))
            .wrapping_add(sample(psamp, idx, 7))
            .wrapping_add(sample(psamp, idx, 8));
        p0.wrapping_add(p1) >> 1
    }
}

#[inline(always)]
fn as_void(f: PredictFn) -> *mut c_void {
    f as *mut c_void
}

/// `static void *BTAC1C2_GetPredictFunc(int pfcn)`
fn BTAC1C2_GetPredictFunc(pfcn: c_int) -> *mut c_void {
    let fcn: *mut c_void = match pfcn {
        0 => as_void(BTAC1C2_PredictSample_Pfn0),
        1 => as_void(BTAC1C2_PredictSample_Pfn1),
        2 => as_void(BTAC1C2_PredictSample_Pfn2),
        3 => as_void(BTAC1C2_PredictSample_Pfn3),
        4 => as_void(BTAC1C2_PredictSample_Pfn4),
        5 => as_void(BTAC1C2_PredictSample_Pfn5),
        6 => as_void(BTAC1C2_PredictSample_Pfn6),
        7 => as_void(BTAC1C2_PredictSample_Pfn7),
        8 => as_void(BTAC1C2_PredictSample_Pfn8),
        9 => as_void(BTAC1C2_PredictSample_Pfn9),
        10 => as_void(BTAC1C2_PredictSample_Pfn10),
        11 => as_void(BTAC1C2_PredictSample_Pfn11),
        _ => as_void(BTAC1C2_PredictSample),
    };
    fcn
}

/// `int call_predict(int pfcn)`
#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn0)) as c_int,
        1 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn1)) as c_int,
        2 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn2)) as c_int,
        3 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn3)) as c_int,
        4 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn4)) as c_int,
        5 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn5)) as c_int,
        6 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn6)) as c_int,
        7 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn7)) as c_int,
        8 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn8)) as c_int,
        9 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn9)) as c_int,
        10 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn10)) as c_int,
        11 => result = (fcn == as_void(BTAC1C2_PredictSample_Pfn11)) as c_int,
        _ => {}
    }
    result
}
