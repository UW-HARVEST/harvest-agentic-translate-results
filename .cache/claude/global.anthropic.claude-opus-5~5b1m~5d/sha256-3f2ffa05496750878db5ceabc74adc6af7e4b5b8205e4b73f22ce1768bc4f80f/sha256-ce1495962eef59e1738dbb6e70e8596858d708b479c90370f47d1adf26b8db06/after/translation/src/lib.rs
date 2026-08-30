//! Rust translation of `c_src/src/lib.c`.
//!
//! The C translation unit compiles to a shared library that exports exactly one
//! public symbol, `get_predict_func` (declared in `c_src/include/lib.h`).  Every
//! other function in the translation unit is `static`, i.e. internal linkage, so
//! it is reproduced here as a private Rust function.  The internal functions are
//! nevertheless translated faithfully because `get_predict_func`'s result is
//! derived from comparing their addresses.
//!
//! All arithmetic below is translated verbatim, including the quirks of the
//! original code (for example `Pfn10`/`Pfn11` use different shift amounts than
//! the corresponding `case 10`/`case 11` arms of `BTAC1C2_PredictSample`).  Such
//! discrepancies are *not* fixed: they are reproduced exactly.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::c_int;

// ---------------------------------------------------------------------------
// typedefs / struct layout from lib.c
// ---------------------------------------------------------------------------

/// `typedef unsigned short btac1c_u16;`
pub type btac1c_u16 = u16;
/// `typedef signed short btac1c_s16;`
pub type btac1c_s16 = i16;
/// `typedef unsigned char btac1c_byte;`
pub type btac1c_byte = u8;

/// ```c
/// struct btac1c_idxstate_s {
///     btac1c_u16  idx;
///     btac1c_s16  lpred;
///     btac1c_s16  rpred;
///     btac1c_byte tag;
///     btac1c_byte bcfcn;
///     btac1c_byte bsfcn;
///     btac1c_byte usefx;
///     btac1c_s16  firfx[4][8];
/// };
/// ```
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

/// The common signature shared by `BTAC1C2_PredictSample` and every
/// `BTAC1C2_PredictSample_PfnN` helper:
/// `int f(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx)`.
type PredictFn =
    unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `psamp[(i - k) & 7]` — the C code masks a possibly-negative index with 7,
/// which on two's-complement hardware always yields a value in `0..=7`.
#[inline(always)]
unsafe fn s(psamp: *const c_int, i: c_int, k: c_int) -> c_int {
    let off = (i.wrapping_sub(k)) & 7;
    unsafe { *psamp.offset(off as isize) }
}

// ---------------------------------------------------------------------------
// static int BTAC1C2_PredictSample(int *psamp, int idx, int pfcn,
//                                  btac1c_idxstate *ridx)
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
                pred = 2i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(s(psamp, i, 2));
            }
            2 => {
                pred = (3i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(s(psamp, i, 2)))
                    >> 1;
            }
            3 => {
                pred = (5i32
                    .wrapping_mul(s(psamp, i, 1))
                    .wrapping_sub(s(psamp, i, 2)))
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
                let row = &(*ridx).firfx[(pfcn - 12) as usize];
                let mut acc: c_int = (row[0] as c_int).wrapping_mul(s(psamp, i, 1));
                acc = acc.wrapping_add((row[1] as c_int).wrapping_mul(s(psamp, i, 2)));
                acc = acc.wrapping_add((row[2] as c_int).wrapping_mul(s(psamp, i, 3)));
                acc = acc.wrapping_add((row[3] as c_int).wrapping_mul(s(psamp, i, 4)));
                acc = acc.wrapping_add((row[4] as c_int).wrapping_mul(s(psamp, i, 5)));
                acc = acc.wrapping_add((row[5] as c_int).wrapping_mul(s(psamp, i, 6)));
                acc = acc.wrapping_add((row[6] as c_int).wrapping_mul(s(psamp, i, 7)));
                acc = acc.wrapping_add((row[7] as c_int).wrapping_mul(s(psamp, i, 8)));
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
// The twelve specialised predictors.  Each keeps the four-parameter C signature
// even though `pfcn` and `ridx` go unused, because the address of each one is
// stored into a `void *` of that shape.
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
        (3i32
            .wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(s(psamp, idx, 2)))
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
        (5i32
            .wrapping_mul(s(psamp, idx, 1))
            .wrapping_sub(s(psamp, idx, 2)))
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

/// NOTE: the C source shifts by 3 here, while `case 10` of
/// `BTAC1C2_PredictSample` shifts by 4.  Reproduced verbatim.
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

/// NOTE: the C source shifts by 1 here, while `case 11` of
/// `BTAC1C2_PredictSample` shifts by 3.  Reproduced verbatim.
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

// ---------------------------------------------------------------------------
// int get_predict_func(int pfcn)   <-- the library's only exported symbol
// ---------------------------------------------------------------------------

/// `int get_predict_func(int pfcn);` from `include/lib.h`.
///
/// Returns 1 when `BTAC1C2_GetPredictFunc(pfcn)` hands back exactly the
/// specialised predictor that corresponds to `pfcn`, and 0 otherwise (which
/// includes every `pfcn` outside `0..=11`, where the `default:` arm leaves
/// `result` at its initial value of 0).
#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn: *const () = BTAC1C2_GetPredictFunc(pfcn);

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

// ---------------------------------------------------------------------------
// Verification-only hook.  Not part of the library's ABI: it is compiled only
// when the `difftest` feature is enabled, which the default build never does,
// so the shipped cdylib exports `get_predict_func` and nothing else.  It exists
// so the internal predictors -- unreachable through the public ABI -- can be
// differentially tested against the original C translation unit.
// ---------------------------------------------------------------------------
#[cfg(feature = "difftest")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __difftest_predict(
    which: c_int,
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let f: PredictFn = match which {
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
    unsafe { f(psamp, idx, pfcn, ridx) }
}

/// Verification-only hook exposing `BTAC1C2_GetPredictFunc`'s *choice*.
///
/// Returns `0..=11` when the selector handed back `BTAC1C2_PredictSample_PfnN`,
/// `12` when it handed back the generic `BTAC1C2_PredictSample`, and `-1` for an
/// unrecognised pointer.  Without this hook the selector's `default:` arm is
/// unobservable (the public wrapper's own `default:` never inspects the pointer),
/// so a wrong fallback would go undetected.
#[cfg(feature = "difftest")]
#[unsafe(no_mangle)]
pub extern "C" fn __difftest_selector(pfcn: c_int) -> c_int {
    let fcn: *const () = BTAC1C2_GetPredictFunc(pfcn);
    let table: [PredictFn; 12] = [
        BTAC1C2_PredictSample_Pfn0,
        BTAC1C2_PredictSample_Pfn1,
        BTAC1C2_PredictSample_Pfn2,
        BTAC1C2_PredictSample_Pfn3,
        BTAC1C2_PredictSample_Pfn4,
        BTAC1C2_PredictSample_Pfn5,
        BTAC1C2_PredictSample_Pfn6,
        BTAC1C2_PredictSample_Pfn7,
        BTAC1C2_PredictSample_Pfn8,
        BTAC1C2_PredictSample_Pfn9,
        BTAC1C2_PredictSample_Pfn10,
        BTAC1C2_PredictSample_Pfn11,
    ];
    let mut i = 0usize;
    while i < 12 {
        if fcn == table[i] as *const () {
            return i as c_int;
        }
        i += 1;
    }
    if fcn == BTAC1C2_PredictSample as PredictFn as *const () {
        return 12;
    }
    -1
}

/// Verification-only hook that invokes whatever `BTAC1C2_GetPredictFunc`
/// selected, so the selector and the predictors are exercised as a composed
/// pipeline rather than as isolated units.
#[cfg(feature = "difftest")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __difftest_call_selected(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let fcn: *const () = BTAC1C2_GetPredictFunc(pfcn);
    let f: PredictFn = unsafe { core::mem::transmute::<*const (), PredictFn>(fcn) };
    unsafe { f(psamp, idx, pfcn, ridx) }
}

/// Verification-only hook mirroring `__difftest_layout` in the C test shim, so
/// the differential test can prove the Rust `btac1c_idxstate` has byte-identical
/// layout to the C `struct btac1c_idxstate_s`.
#[cfg(feature = "difftest")]
#[unsafe(no_mangle)]
pub extern "C" fn __difftest_layout(what: c_int) -> c_int {
    use core::mem::{align_of, size_of};
    // `offset_of!` is stable since 1.77.
    match what {
        0 => size_of::<btac1c_idxstate>() as c_int,
        1 => core::mem::offset_of!(btac1c_idxstate, idx) as c_int,
        2 => core::mem::offset_of!(btac1c_idxstate, lpred) as c_int,
        3 => core::mem::offset_of!(btac1c_idxstate, rpred) as c_int,
        4 => core::mem::offset_of!(btac1c_idxstate, tag) as c_int,
        5 => core::mem::offset_of!(btac1c_idxstate, bcfcn) as c_int,
        6 => core::mem::offset_of!(btac1c_idxstate, bsfcn) as c_int,
        7 => core::mem::offset_of!(btac1c_idxstate, usefx) as c_int,
        8 => core::mem::offset_of!(btac1c_idxstate, firfx) as c_int,
        9 => size_of::<[[btac1c_s16; 8]; 4]>() as c_int,
        10 => align_of::<btac1c_idxstate>() as c_int,
        _ => -1,
    }
}
