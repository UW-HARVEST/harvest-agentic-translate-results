//! Rust translation of `c_src/src/lib.c`.
//!
//! The C translation unit defines a family of `static` sample-prediction
//! helpers, a `static` dispatcher that hands back a `void *` to one of them,
//! and a single exported entry point, `call_predict`, which checks that the
//! dispatcher returned the helper matching the requested prediction function
//! number.
//!
//! Only `call_predict` is exported by the C shared library. The public header
//! `include/lib.h` declares `int get_predict_func(int pfcn);`, but no such
//! function is ever defined in the C sources, so it is *not* part of the
//! exported ABI (confirmed with `nm -D` on the C `.so`) and is therefore not
//! defined here either.
//!
//! Semantics are reproduced exactly, including the discrepancies between the
//! big `switch`-based `BTAC1C2_PredictSample` and the specialised
//! `BTAC1C2_PredictSample_Pfn*` variants (`Pfn10` shifts by 3 where the switch
//! shifts by 4; `Pfn11` shifts by 1 where the switch shifts by 3). Those are
//! bugs in the original C, and they are preserved verbatim.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_int;

/// `typedef unsigned short btac1c_u16;`
type btac1c_u16 = u16;
/// `typedef signed short btac1c_s16;`
type btac1c_s16 = i16;
/// `typedef unsigned char btac1c_byte;`
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

/// The C prediction-helper signature:
/// `int (*)(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx)`.
type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

/// `psamp[(i - k) & 7]`
///
/// The subtraction is performed with wrapping semantics to match what the C
/// compiler emits for `int` arithmetic, and `& 7` keeps the result inside the
/// eight-entry ring buffer for any input (C and Rust agree on `&` for negative
/// operands under two's complement).
#[inline(always)]
unsafe fn ps(psamp: *mut c_int, i: c_int, k: c_int) -> c_int {
    let off = (i.wrapping_sub(k)) & 7;
    unsafe { *psamp.offset(off as isize) }
}

/// `static int BTAC1C2_PredictSample(...)`
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
                pred = (2i32.wrapping_mul(ps(psamp, i, 1))).wrapping_sub(ps(psamp, i, 2));
            }
            2 => {
                pred = ((3i32.wrapping_mul(ps(psamp, i, 1))).wrapping_sub(ps(psamp, i, 2))) >> 1;
            }
            3 => {
                pred = ((5i32.wrapping_mul(ps(psamp, i, 1))).wrapping_sub(ps(psamp, i, 2))) >> 2;
            }
            4 => {
                p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = p0.wrapping_sub(p1 >> 1);
            }
            5 => {
                p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = (3i32.wrapping_mul(p0).wrapping_sub(p1)) >> 2;
            }
            6 => {
                p0 = ps(psamp, i, 1).wrapping_add(ps(psamp, i, 2));
                p1 = ps(psamp, i, 2).wrapping_add(ps(psamp, i, 3));
                pred = (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3;
            }
            7 => {
                pred = (18i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(4i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(3i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(2i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(1i32.wrapping_mul(ps(psamp, i, 5))))
                .wrapping_div(16);
            }
            8 => {
                pred = (72i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(16i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(12i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(8i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_sub(3i32.wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add(3i32.wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_sub(1i32.wrapping_mul(ps(psamp, i, 8))))
                .wrapping_div(64);
            }
            9 => {
                pred = (76i32
                    .wrapping_mul(ps(psamp, i, 1))
                    .wrapping_sub(17i32.wrapping_mul(ps(psamp, i, 2)))
                    .wrapping_add(10i32.wrapping_mul(ps(psamp, i, 3)))
                    .wrapping_sub(7i32.wrapping_mul(ps(psamp, i, 4)))
                    .wrapping_add(5i32.wrapping_mul(ps(psamp, i, 5)))
                    .wrapping_sub(4i32.wrapping_mul(ps(psamp, i, 6)))
                    .wrapping_add(4i32.wrapping_mul(ps(psamp, i, 7)))
                    .wrapping_sub(3i32.wrapping_mul(ps(psamp, i, 8))))
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
                pred = (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 4;
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
                pred = (p0.wrapping_add(p1)) >> 3;
            }
            12..=15 => {
                let fx = &(*ridx).firfx[(pfcn - 12) as usize];
                let mut acc: c_int = (fx[0] as c_int).wrapping_mul(ps(psamp, i, 1));
                acc = acc.wrapping_add((fx[1] as c_int).wrapping_mul(ps(psamp, i, 2)));
                acc = acc.wrapping_add((fx[2] as c_int).wrapping_mul(ps(psamp, i, 3)));
                acc = acc.wrapping_add((fx[3] as c_int).wrapping_mul(ps(psamp, i, 4)));
                acc = acc.wrapping_add((fx[4] as c_int).wrapping_mul(ps(psamp, i, 5)));
                acc = acc.wrapping_add((fx[5] as c_int).wrapping_mul(ps(psamp, i, 6)));
                acc = acc.wrapping_add((fx[6] as c_int).wrapping_mul(ps(psamp, i, 7)));
                acc = acc.wrapping_add((fx[7] as c_int).wrapping_mul(ps(psamp, i, 8)));
                pred = acc.wrapping_div(256);
            }
            _ => {
                pred = 0;
            }
        }
    }
    pred
}

/// `static int BTAC1C2_PredictSample_Pfn0(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { ps(psamp, idx, 1) }
}

/// `static int BTAC1C2_PredictSample_Pfn1(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { (2i32.wrapping_mul(ps(psamp, idx, 1))).wrapping_sub(ps(psamp, idx, 2)) }
}

/// `static int BTAC1C2_PredictSample_Pfn2(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { ((3i32.wrapping_mul(ps(psamp, idx, 1))).wrapping_sub(ps(psamp, idx, 2))) >> 1 }
}

/// `static int BTAC1C2_PredictSample_Pfn3(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { ((5i32.wrapping_mul(ps(psamp, idx, 1))).wrapping_sub(ps(psamp, idx, 2))) >> 2 }
}

/// `static int BTAC1C2_PredictSample_Pfn4(...)`
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

/// `static int BTAC1C2_PredictSample_Pfn5(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1).wrapping_add(ps(psamp, idx, 2));
        let p1 = ps(psamp, idx, 2).wrapping_add(ps(psamp, idx, 3));
        (3i32.wrapping_mul(p0).wrapping_sub(p1)) >> 2
    }
}

/// `static int BTAC1C2_PredictSample_Pfn6(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, 1).wrapping_add(ps(psamp, idx, 2));
        let p1 = ps(psamp, idx, 2).wrapping_add(ps(psamp, idx, 3));
        (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3
    }
}

/// `static int BTAC1C2_PredictSample_Pfn7(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (18i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(4i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(3i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(2i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(1i32.wrapping_mul(ps(psamp, idx, 5))))
        .wrapping_div(16)
    }
}

/// `static int BTAC1C2_PredictSample_Pfn8(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (72i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(16i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(12i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(8i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(ps(psamp, idx, 5)))
            .wrapping_sub(3i32.wrapping_mul(ps(psamp, idx, 6)))
            .wrapping_add(3i32.wrapping_mul(ps(psamp, idx, 7)))
            .wrapping_sub(1i32.wrapping_mul(ps(psamp, idx, 8))))
        .wrapping_div(64)
    }
}

/// `static int BTAC1C2_PredictSample_Pfn9(...)`
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (76i32
            .wrapping_mul(ps(psamp, idx, 1))
            .wrapping_sub(17i32.wrapping_mul(ps(psamp, idx, 2)))
            .wrapping_add(10i32.wrapping_mul(ps(psamp, idx, 3)))
            .wrapping_sub(7i32.wrapping_mul(ps(psamp, idx, 4)))
            .wrapping_add(5i32.wrapping_mul(ps(psamp, idx, 5)))
            .wrapping_sub(4i32.wrapping_mul(ps(psamp, idx, 6)))
            .wrapping_add(4i32.wrapping_mul(ps(psamp, idx, 7)))
            .wrapping_sub(3i32.wrapping_mul(ps(psamp, idx, 8))))
        .wrapping_div(64)
    }
}

/// `static int BTAC1C2_PredictSample_Pfn10(...)`
///
/// Note the `>> 3` here versus `>> 4` in the `case 10:` arm of
/// `BTAC1C2_PredictSample`; the inconsistency is present in the C source and is
/// reproduced as-is.
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
        (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3
    }
}

/// `static int BTAC1C2_PredictSample_Pfn11(...)`
///
/// Note the `>> 1` here versus `>> 3` in the `case 11:` arm of
/// `BTAC1C2_PredictSample`; again, kept exactly as the C has it.
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
        (p0.wrapping_add(p1)) >> 1
    }
}

/// `static void *BTAC1C2_GetPredictFunc(int pfcn)`
fn BTAC1C2_GetPredictFunc(pfcn: c_int) -> *const () {
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
    fcn as *const ()
}

/// `int call_predict(int pfcn)`
///
/// The only symbol exported by the C shared library. It asks the dispatcher for
/// a function pointer and reports whether that pointer is the specialised
/// helper corresponding to `pfcn`; anything outside `0..=11` falls through the
/// `default:` label and yields `0`.
#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => result = (fcn == BTAC1C2_PredictSample_Pfn0 as *const ()) as c_int,
        1 => result = (fcn == BTAC1C2_PredictSample_Pfn1 as *const ()) as c_int,
        2 => result = (fcn == BTAC1C2_PredictSample_Pfn2 as *const ()) as c_int,
        3 => result = (fcn == BTAC1C2_PredictSample_Pfn3 as *const ()) as c_int,
        4 => result = (fcn == BTAC1C2_PredictSample_Pfn4 as *const ()) as c_int,
        5 => result = (fcn == BTAC1C2_PredictSample_Pfn5 as *const ()) as c_int,
        6 => result = (fcn == BTAC1C2_PredictSample_Pfn6 as *const ()) as c_int,
        7 => result = (fcn == BTAC1C2_PredictSample_Pfn7 as *const ()) as c_int,
        8 => result = (fcn == BTAC1C2_PredictSample_Pfn8 as *const ()) as c_int,
        9 => result = (fcn == BTAC1C2_PredictSample_Pfn9 as *const ()) as c_int,
        10 => result = (fcn == BTAC1C2_PredictSample_Pfn10 as *const ()) as c_int,
        11 => result = (fcn == BTAC1C2_PredictSample_Pfn11 as *const ()) as c_int,
        _ => {}
    }
    result
}
