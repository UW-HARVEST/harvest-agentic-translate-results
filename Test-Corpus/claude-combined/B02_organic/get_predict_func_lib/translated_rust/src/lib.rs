// Translation of c_src/src/lib.c to Rust.
//
// The original C code uses function-pointer comparisons inside
// `get_predict_func` to determine which prediction function was returned by
// `BTAC1C2_GetPredictFunc`. Because both routines live in the same compilation
// unit, the comparisons resolve at compile time to equivalent boolean
// expressions: for each `pfcn` in 0..=11, `BTAC1C2_GetPredictFunc(pfcn)`
// returns the matching `_PfnN` function and `get_predict_func` returns 1.
// For any other `pfcn`, `BTAC1C2_GetPredictFunc` returns `BTAC1C2_PredictSample`
// (which is not one of the `_PfnN` symbols), so `get_predict_func` returns 0.
//
// We preserve that observable behaviour exactly using actual function pointers
// and the same `match`/`switch` structure, so any future change in the C code
// would be straightforward to mirror.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_int;

type Btac1cU16 = u16;
type Btac1cS16 = i16;
type Btac1cByte = u8;

#[repr(C)]
struct Btac1cIdxstate {
    idx: Btac1cU16,
    lpred: Btac1cS16,
    rpred: Btac1cS16,
    tag: Btac1cByte,
    bcfcn: Btac1cByte,
    bsfcn: Btac1cByte,
    usefx: Btac1cByte,
    firfx: [[Btac1cS16; 8]; 4],
}

type PredictFn =
    unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut Btac1cIdxstate) -> c_int;

unsafe extern "C" fn btac1c2_predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut Btac1cIdxstate,
) -> c_int {
    let i = idx;
    let s = |off: c_int| -> c_int {
        let k = ((i - off) & 7) as isize;
        unsafe { *psamp.offset(k) }
    };
    let pred: c_int;
    match pfcn {
        0 => {
            pred = s(1);
        }
        1 => {
            pred = 2 * s(1) - s(2);
        }
        2 => {
            pred = (3 * s(1) - s(2)) >> 1;
        }
        3 => {
            pred = (5 * s(1) - s(2)) >> 2;
        }
        4 => {
            let p0 = s(1) + s(2);
            let p1 = s(2) + s(3);
            pred = p0 - (p1 >> 1);
        }
        5 => {
            let p0 = s(1) + s(2);
            let p1 = s(2) + s(3);
            pred = (3 * p0 - p1) >> 2;
        }
        6 => {
            let p0 = s(1) + s(2);
            let p1 = s(2) + s(3);
            pred = (5 * p0 - p1) >> 3;
        }
        7 => {
            pred = (18 * s(1) - 4 * s(2) + 3 * s(3) - 2 * s(4) + 1 * s(5)) / 16;
        }
        8 => {
            pred = (72 * s(1) - 16 * s(2) + 12 * s(3) - 8 * s(4)
                + 5 * s(5) - 3 * s(6) + 3 * s(7) - 1 * s(8))
                / 64;
        }
        9 => {
            pred = (76 * s(1) - 17 * s(2) + 10 * s(3) - 7 * s(4)
                + 5 * s(5) - 4 * s(6) + 4 * s(7) - 3 * s(8))
                / 64;
        }
        10 => {
            let p0 = s(1) + s(2) + s(3) + s(4);
            let p1 = s(5) + s(6) + s(7) + s(8);
            pred = (5 * p0 - p1) >> 4;
        }
        11 => {
            let p0 = s(1) + s(2) + s(3) + s(4);
            let p1 = s(5) + s(6) + s(7) + s(8);
            pred = (p0 + p1) >> 3;
        }
        12 | 13 | 14 | 15 => {
            let row = (pfcn - 12) as usize;
            let r = unsafe { &(*ridx).firfx[row] };
            pred = (r[0] as c_int * s(1)
                + r[1] as c_int * s(2)
                + r[2] as c_int * s(3)
                + r[3] as c_int * s(4)
                + r[4] as c_int * s(5)
                + r[5] as c_int * s(6)
                + r[6] as c_int * s(7)
                + r[7] as c_int * s(8))
                / 256;
        }
        _ => {
            pred = 0;
        }
    }
    pred
}

unsafe extern "C" fn btac1c2_predict_sample_pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe { *psamp.offset(((idx - 1) & 7) as isize) }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        2 * *psamp.offset(((idx - 1) & 7) as isize)
            - *psamp.offset(((idx - 2) & 7) as isize)
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        (3 * *psamp.offset(((idx - 1) & 7) as isize)
            - *psamp.offset(((idx - 2) & 7) as isize))
            >> 1
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        (5 * *psamp.offset(((idx - 1) & 7) as isize)
            - *psamp.offset(((idx - 2) & 7) as isize))
            >> 2
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize)
            + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize)
            + *psamp.offset(((idx - 3) & 7) as isize);
        p0 - (p1 >> 1)
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize)
            + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize)
            + *psamp.offset(((idx - 3) & 7) as isize);
        (3 * p0 - p1) >> 2
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize)
            + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize)
            + *psamp.offset(((idx - 3) & 7) as isize);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        (18 * *psamp.offset(((idx - 1) & 7) as isize)
            - 4 * *psamp.offset(((idx - 2) & 7) as isize)
            + 3 * *psamp.offset(((idx - 3) & 7) as isize)
            - 2 * *psamp.offset(((idx - 4) & 7) as isize)
            + 1 * *psamp.offset(((idx - 5) & 7) as isize))
            / 16
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        (72 * *psamp.offset(((idx - 1) & 7) as isize)
            - 16 * *psamp.offset(((idx - 2) & 7) as isize)
            + 12 * *psamp.offset(((idx - 3) & 7) as isize)
            - 8 * *psamp.offset(((idx - 4) & 7) as isize)
            + 5 * *psamp.offset(((idx - 5) & 7) as isize)
            - 3 * *psamp.offset(((idx - 6) & 7) as isize)
            + 3 * *psamp.offset(((idx - 7) & 7) as isize)
            - 1 * *psamp.offset(((idx - 8) & 7) as isize))
            / 64
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        (76 * *psamp.offset(((idx - 1) & 7) as isize)
            - 17 * *psamp.offset(((idx - 2) & 7) as isize)
            + 10 * *psamp.offset(((idx - 3) & 7) as isize)
            - 7 * *psamp.offset(((idx - 4) & 7) as isize)
            + 5 * *psamp.offset(((idx - 5) & 7) as isize)
            - 4 * *psamp.offset(((idx - 6) & 7) as isize)
            + 4 * *psamp.offset(((idx - 7) & 7) as isize)
            - 3 * *psamp.offset(((idx - 8) & 7) as isize))
            / 64
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize)
            + *psamp.offset(((idx - 2) & 7) as isize)
            + *psamp.offset(((idx - 3) & 7) as isize)
            + *psamp.offset(((idx - 4) & 7) as isize);
        let p1 = *psamp.offset(((idx - 5) & 7) as isize)
            + *psamp.offset(((idx - 6) & 7) as isize)
            + *psamp.offset(((idx - 7) & 7) as isize)
            + *psamp.offset(((idx - 8) & 7) as isize);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize)
            + *psamp.offset(((idx - 2) & 7) as isize)
            + *psamp.offset(((idx - 3) & 7) as isize)
            + *psamp.offset(((idx - 4) & 7) as isize);
        let p1 = *psamp.offset(((idx - 5) & 7) as isize)
            + *psamp.offset(((idx - 6) & 7) as isize)
            + *psamp.offset(((idx - 7) & 7) as isize)
            + *psamp.offset(((idx - 8) & 7) as isize);
        (p0 + p1) >> 1
    }
}

fn btac1c2_get_predict_func(pfcn: c_int) -> PredictFn {
    match pfcn {
        0 => btac1c2_predict_sample_pfn0,
        1 => btac1c2_predict_sample_pfn1,
        2 => btac1c2_predict_sample_pfn2,
        3 => btac1c2_predict_sample_pfn3,
        4 => btac1c2_predict_sample_pfn4,
        5 => btac1c2_predict_sample_pfn5,
        6 => btac1c2_predict_sample_pfn6,
        7 => btac1c2_predict_sample_pfn7,
        8 => btac1c2_predict_sample_pfn8,
        9 => btac1c2_predict_sample_pfn9,
        10 => btac1c2_predict_sample_pfn10,
        11 => btac1c2_predict_sample_pfn11,
        _ => btac1c2_predict_sample,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = btac1c2_get_predict_func(pfcn) as *const ();
    match pfcn {
        0 => {
            result = (fcn == btac1c2_predict_sample_pfn0 as *const ()) as c_int;
        }
        1 => {
            result = (fcn == btac1c2_predict_sample_pfn1 as *const ()) as c_int;
        }
        2 => {
            result = (fcn == btac1c2_predict_sample_pfn2 as *const ()) as c_int;
        }
        3 => {
            result = (fcn == btac1c2_predict_sample_pfn3 as *const ()) as c_int;
        }
        4 => {
            result = (fcn == btac1c2_predict_sample_pfn4 as *const ()) as c_int;
        }
        5 => {
            result = (fcn == btac1c2_predict_sample_pfn5 as *const ()) as c_int;
        }
        6 => {
            result = (fcn == btac1c2_predict_sample_pfn6 as *const ()) as c_int;
        }
        7 => {
            result = (fcn == btac1c2_predict_sample_pfn7 as *const ()) as c_int;
        }
        8 => {
            result = (fcn == btac1c2_predict_sample_pfn8 as *const ()) as c_int;
        }
        9 => {
            result = (fcn == btac1c2_predict_sample_pfn9 as *const ()) as c_int;
        }
        10 => {
            result = (fcn == btac1c2_predict_sample_pfn10 as *const ()) as c_int;
        }
        11 => {
            result = (fcn == btac1c2_predict_sample_pfn11 as *const ()) as c_int;
        }
        _ => {}
    }
    result
}
