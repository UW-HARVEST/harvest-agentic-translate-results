#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_int;

type btac1c_u16 = u16;
type btac1c_s16 = i16;
type btac1c_byte = u8;

#[repr(C)]
struct btac1c_idxstate {
    idx: btac1c_u16,
    lpred: btac1c_s16,
    rpred: btac1c_s16,
    tag: btac1c_byte,
    bcfcn: btac1c_byte,
    bsfcn: btac1c_byte,
    usefx: btac1c_byte,
    firfx: [[btac1c_s16; 8]; 4],
}

type PredictFn = unsafe extern "C" fn(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int;

#[inline(always)]
unsafe fn ps(psamp: *mut c_int, idx: c_int, off: c_int) -> c_int {
    // psamp[(idx - off) & 7]
    let i = ((idx - off) & 7) as isize;
    *psamp.offset(i)
}

unsafe extern "C" fn BTAC1C2_PredictSample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let i = idx;
    let pred: c_int;
    let p0: c_int;
    let p1: c_int;
    match pfcn {
        0 => {
            pred = ps(psamp, i, 1);
        }
        1 => {
            pred = 2 * ps(psamp, i, 1) - ps(psamp, i, 2);
        }
        2 => {
            pred = (3 * ps(psamp, i, 1) - ps(psamp, i, 2)) >> 1;
        }
        3 => {
            pred = (5 * ps(psamp, i, 1) - ps(psamp, i, 2)) >> 2;
        }
        4 => {
            p0 = ps(psamp, i, 1) + ps(psamp, i, 2);
            p1 = ps(psamp, i, 2) + ps(psamp, i, 3);
            pred = p0 - (p1 >> 1);
        }
        5 => {
            p0 = ps(psamp, i, 1) + ps(psamp, i, 2);
            p1 = ps(psamp, i, 2) + ps(psamp, i, 3);
            pred = (3 * p0 - p1) >> 2;
        }
        6 => {
            p0 = ps(psamp, i, 1) + ps(psamp, i, 2);
            p1 = ps(psamp, i, 2) + ps(psamp, i, 3);
            pred = (5 * p0 - p1) >> 3;
        }
        7 => {
            pred = (18 * ps(psamp, i, 1) - 4 * ps(psamp, i, 2)
                + 3 * ps(psamp, i, 3)
                - 2 * ps(psamp, i, 4)
                + 1 * ps(psamp, i, 5))
                / 16;
        }
        8 => {
            pred = (72 * ps(psamp, i, 1) - 16 * ps(psamp, i, 2)
                + 12 * ps(psamp, i, 3)
                - 8 * ps(psamp, i, 4)
                + 5 * ps(psamp, i, 5)
                - 3 * ps(psamp, i, 6)
                + 3 * ps(psamp, i, 7)
                - 1 * ps(psamp, i, 8))
                / 64;
        }
        9 => {
            pred = (76 * ps(psamp, i, 1) - 17 * ps(psamp, i, 2)
                + 10 * ps(psamp, i, 3)
                - 7 * ps(psamp, i, 4)
                + 5 * ps(psamp, i, 5)
                - 4 * ps(psamp, i, 6)
                + 4 * ps(psamp, i, 7)
                - 3 * ps(psamp, i, 8))
                / 64;
        }
        10 => {
            p0 = ps(psamp, i, 1) + ps(psamp, i, 2) + ps(psamp, i, 3) + ps(psamp, i, 4);
            p1 = ps(psamp, i, 5) + ps(psamp, i, 6) + ps(psamp, i, 7) + ps(psamp, i, 8);
            pred = (5 * p0 - p1) >> 4;
        }
        11 => {
            p0 = ps(psamp, i, 1) + ps(psamp, i, 2) + ps(psamp, i, 3) + ps(psamp, i, 4);
            p1 = ps(psamp, i, 5) + ps(psamp, i, 6) + ps(psamp, i, 7) + ps(psamp, i, 8);
            pred = (p0 + p1) >> 3;
        }
        12 | 13 | 14 | 15 => {
            let row = (pfcn - 12) as usize;
            let r = &(*ridx).firfx[row];
            pred = ((r[0] as c_int) * ps(psamp, i, 1)
                + (r[1] as c_int) * ps(psamp, i, 2)
                + (r[2] as c_int) * ps(psamp, i, 3)
                + (r[3] as c_int) * ps(psamp, i, 4)
                + (r[4] as c_int) * ps(psamp, i, 5)
                + (r[5] as c_int) * ps(psamp, i, 6)
                + (r[6] as c_int) * ps(psamp, i, 7)
                + (r[7] as c_int) * ps(psamp, i, 8))
                / 256;
        }
        _ => {
            pred = 0;
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
    ps(psamp, idx, 1)
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    2 * ps(psamp, idx, 1) - ps(psamp, idx, 2)
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (3 * ps(psamp, idx, 1) - ps(psamp, idx, 2)) >> 1
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (5 * ps(psamp, idx, 1) - ps(psamp, idx, 2)) >> 2
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = ps(psamp, idx, 1) + ps(psamp, idx, 2);
    let p1 = ps(psamp, idx, 2) + ps(psamp, idx, 3);
    p0 - (p1 >> 1)
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = ps(psamp, idx, 1) + ps(psamp, idx, 2);
    let p1 = ps(psamp, idx, 2) + ps(psamp, idx, 3);
    (3 * p0 - p1) >> 2
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = ps(psamp, idx, 1) + ps(psamp, idx, 2);
    let p1 = ps(psamp, idx, 2) + ps(psamp, idx, 3);
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (18 * ps(psamp, idx, 1) - 4 * ps(psamp, idx, 2)
        + 3 * ps(psamp, idx, 3)
        - 2 * ps(psamp, idx, 4)
        + 1 * ps(psamp, idx, 5))
        / 16
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (72 * ps(psamp, idx, 1) - 16 * ps(psamp, idx, 2)
        + 12 * ps(psamp, idx, 3)
        - 8 * ps(psamp, idx, 4)
        + 5 * ps(psamp, idx, 5)
        - 3 * ps(psamp, idx, 6)
        + 3 * ps(psamp, idx, 7)
        - 1 * ps(psamp, idx, 8))
        / 64
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (76 * ps(psamp, idx, 1) - 17 * ps(psamp, idx, 2)
        + 10 * ps(psamp, idx, 3)
        - 7 * ps(psamp, idx, 4)
        + 5 * ps(psamp, idx, 5)
        - 4 * ps(psamp, idx, 6)
        + 4 * ps(psamp, idx, 7)
        - 3 * ps(psamp, idx, 8))
        / 64
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = ps(psamp, idx, 1) + ps(psamp, idx, 2) + ps(psamp, idx, 3) + ps(psamp, idx, 4);
    let p1 = ps(psamp, idx, 5) + ps(psamp, idx, 6) + ps(psamp, idx, 7) + ps(psamp, idx, 8);
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = ps(psamp, idx, 1) + ps(psamp, idx, 2) + ps(psamp, idx, 3) + ps(psamp, idx, 4);
    let p1 = ps(psamp, idx, 5) + ps(psamp, idx, 6) + ps(psamp, idx, 7) + ps(psamp, idx, 8);
    (p0 + p1) >> 1
}

fn BTAC1C2_GetPredictFunc(pfcn: c_int) -> *mut std::ffi::c_void {
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
    fcn as *mut std::ffi::c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn0 as *mut std::ffi::c_void) as c_int;
        }
        1 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn1 as *mut std::ffi::c_void) as c_int;
        }
        2 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn2 as *mut std::ffi::c_void) as c_int;
        }
        3 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn3 as *mut std::ffi::c_void) as c_int;
        }
        4 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn4 as *mut std::ffi::c_void) as c_int;
        }
        5 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn5 as *mut std::ffi::c_void) as c_int;
        }
        6 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn6 as *mut std::ffi::c_void) as c_int;
        }
        7 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn7 as *mut std::ffi::c_void) as c_int;
        }
        8 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn8 as *mut std::ffi::c_void) as c_int;
        }
        9 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn9 as *mut std::ffi::c_void) as c_int;
        }
        10 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn10 as *mut std::ffi::c_void) as c_int;
        }
        11 => {
            result = (fcn == BTAC1C2_PredictSample_Pfn11 as *mut std::ffi::c_void) as c_int;
        }
        _ => {}
    }
    result
}
