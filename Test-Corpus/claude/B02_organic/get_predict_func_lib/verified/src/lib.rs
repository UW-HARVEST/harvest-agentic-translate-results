#![allow(non_camel_case_types)]
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
unsafe fn psamp_at(psamp: *mut c_int, i: c_int, off: c_int) -> c_int {
    // psamp[(i - off) & 7]
    let idx = ((i - off) & 7) as isize;
    *psamp.offset(idx)
}

unsafe extern "C" fn btac1c2_predict_sample(
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
            pred = psamp_at(psamp, i, 1);
        }
        1 => {
            pred = 2 * psamp_at(psamp, i, 1) - psamp_at(psamp, i, 2);
        }
        2 => {
            pred = (3 * psamp_at(psamp, i, 1) - psamp_at(psamp, i, 2)) >> 1;
        }
        3 => {
            pred = (5 * psamp_at(psamp, i, 1) - psamp_at(psamp, i, 2)) >> 2;
        }
        4 => {
            p0 = psamp_at(psamp, i, 1) + psamp_at(psamp, i, 2);
            p1 = psamp_at(psamp, i, 2) + psamp_at(psamp, i, 3);
            pred = p0 - (p1 >> 1);
        }
        5 => {
            p0 = psamp_at(psamp, i, 1) + psamp_at(psamp, i, 2);
            p1 = psamp_at(psamp, i, 2) + psamp_at(psamp, i, 3);
            pred = (3 * p0 - p1) >> 2;
        }
        6 => {
            p0 = psamp_at(psamp, i, 1) + psamp_at(psamp, i, 2);
            p1 = psamp_at(psamp, i, 2) + psamp_at(psamp, i, 3);
            pred = (5 * p0 - p1) >> 3;
        }
        7 => {
            pred = (18 * psamp_at(psamp, i, 1)
                - 4 * psamp_at(psamp, i, 2)
                + 3 * psamp_at(psamp, i, 3)
                - 2 * psamp_at(psamp, i, 4)
                + 1 * psamp_at(psamp, i, 5))
                / 16;
        }
        8 => {
            pred = (72 * psamp_at(psamp, i, 1)
                - 16 * psamp_at(psamp, i, 2)
                + 12 * psamp_at(psamp, i, 3)
                - 8 * psamp_at(psamp, i, 4)
                + 5 * psamp_at(psamp, i, 5)
                - 3 * psamp_at(psamp, i, 6)
                + 3 * psamp_at(psamp, i, 7)
                - 1 * psamp_at(psamp, i, 8))
                / 64;
        }
        9 => {
            pred = (76 * psamp_at(psamp, i, 1)
                - 17 * psamp_at(psamp, i, 2)
                + 10 * psamp_at(psamp, i, 3)
                - 7 * psamp_at(psamp, i, 4)
                + 5 * psamp_at(psamp, i, 5)
                - 4 * psamp_at(psamp, i, 6)
                + 4 * psamp_at(psamp, i, 7)
                - 3 * psamp_at(psamp, i, 8))
                / 64;
        }
        10 => {
            p0 = psamp_at(psamp, i, 1)
                + psamp_at(psamp, i, 2)
                + psamp_at(psamp, i, 3)
                + psamp_at(psamp, i, 4);
            p1 = psamp_at(psamp, i, 5)
                + psamp_at(psamp, i, 6)
                + psamp_at(psamp, i, 7)
                + psamp_at(psamp, i, 8);
            pred = (5 * p0 - p1) >> 4;
        }
        11 => {
            p0 = psamp_at(psamp, i, 1)
                + psamp_at(psamp, i, 2)
                + psamp_at(psamp, i, 3)
                + psamp_at(psamp, i, 4);
            p1 = psamp_at(psamp, i, 5)
                + psamp_at(psamp, i, 6)
                + psamp_at(psamp, i, 7)
                + psamp_at(psamp, i, 8);
            pred = (p0 + p1) >> 3;
        }
        12 | 13 | 14 | 15 => {
            let row = (pfcn - 12) as usize;
            let firfx = &(*ridx).firfx[row];
            pred = ((firfx[0] as c_int) * psamp_at(psamp, i, 1)
                + (firfx[1] as c_int) * psamp_at(psamp, i, 2)
                + (firfx[2] as c_int) * psamp_at(psamp, i, 3)
                + (firfx[3] as c_int) * psamp_at(psamp, i, 4)
                + (firfx[4] as c_int) * psamp_at(psamp, i, 5)
                + (firfx[5] as c_int) * psamp_at(psamp, i, 6)
                + (firfx[6] as c_int) * psamp_at(psamp, i, 7)
                + (firfx[7] as c_int) * psamp_at(psamp, i, 8))
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
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    psamp_at(psamp, idx, 1)
}

unsafe extern "C" fn btac1c2_predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    2 * psamp_at(psamp, idx, 1) - psamp_at(psamp, idx, 2)
}

unsafe extern "C" fn btac1c2_predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (3 * psamp_at(psamp, idx, 1) - psamp_at(psamp, idx, 2)) >> 1
}

unsafe extern "C" fn btac1c2_predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (5 * psamp_at(psamp, idx, 1) - psamp_at(psamp, idx, 2)) >> 2
}

unsafe extern "C" fn btac1c2_predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx, 1) + psamp_at(psamp, idx, 2);
    let p1 = psamp_at(psamp, idx, 2) + psamp_at(psamp, idx, 3);
    p0 - (p1 >> 1)
}

unsafe extern "C" fn btac1c2_predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx, 1) + psamp_at(psamp, idx, 2);
    let p1 = psamp_at(psamp, idx, 2) + psamp_at(psamp, idx, 3);
    (3 * p0 - p1) >> 2
}

unsafe extern "C" fn btac1c2_predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx, 1) + psamp_at(psamp, idx, 2);
    let p1 = psamp_at(psamp, idx, 2) + psamp_at(psamp, idx, 3);
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn btac1c2_predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (18 * psamp_at(psamp, idx, 1) - 4 * psamp_at(psamp, idx, 2)
        + 3 * psamp_at(psamp, idx, 3)
        - 2 * psamp_at(psamp, idx, 4)
        + 1 * psamp_at(psamp, idx, 5))
        / 16
}

unsafe extern "C" fn btac1c2_predict_sample_pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (72 * psamp_at(psamp, idx, 1) - 16 * psamp_at(psamp, idx, 2)
        + 12 * psamp_at(psamp, idx, 3)
        - 8 * psamp_at(psamp, idx, 4)
        + 5 * psamp_at(psamp, idx, 5)
        - 3 * psamp_at(psamp, idx, 6)
        + 3 * psamp_at(psamp, idx, 7)
        - 1 * psamp_at(psamp, idx, 8))
        / 64
}

unsafe extern "C" fn btac1c2_predict_sample_pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (76 * psamp_at(psamp, idx, 1) - 17 * psamp_at(psamp, idx, 2)
        + 10 * psamp_at(psamp, idx, 3)
        - 7 * psamp_at(psamp, idx, 4)
        + 5 * psamp_at(psamp, idx, 5)
        - 4 * psamp_at(psamp, idx, 6)
        + 4 * psamp_at(psamp, idx, 7)
        - 3 * psamp_at(psamp, idx, 8))
        / 64
}

unsafe extern "C" fn btac1c2_predict_sample_pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx, 1)
        + psamp_at(psamp, idx, 2)
        + psamp_at(psamp, idx, 3)
        + psamp_at(psamp, idx, 4);
    let p1 = psamp_at(psamp, idx, 5)
        + psamp_at(psamp, idx, 6)
        + psamp_at(psamp, idx, 7)
        + psamp_at(psamp, idx, 8);
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn btac1c2_predict_sample_pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx, 1)
        + psamp_at(psamp, idx, 2)
        + psamp_at(psamp, idx, 3)
        + psamp_at(psamp, idx, 4);
    let p1 = psamp_at(psamp, idx, 5)
        + psamp_at(psamp, idx, 6)
        + psamp_at(psamp, idx, 7)
        + psamp_at(psamp, idx, 8);
    (p0 + p1) >> 1
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

#[inline(always)]
fn fn_addr(f: PredictFn) -> usize {
    f as *const () as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = btac1c2_get_predict_func(pfcn);
    let fcn_addr = fn_addr(fcn);
    match pfcn {
        0 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn0)) as c_int;
        }
        1 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn1)) as c_int;
        }
        2 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn2)) as c_int;
        }
        3 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn3)) as c_int;
        }
        4 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn4)) as c_int;
        }
        5 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn5)) as c_int;
        }
        6 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn6)) as c_int;
        }
        7 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn7)) as c_int;
        }
        8 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn8)) as c_int;
        }
        9 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn9)) as c_int;
        }
        10 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn10)) as c_int;
        }
        11 => {
            result = (fcn_addr == fn_addr(btac1c2_predict_sample_pfn11)) as c_int;
        }
        _ => {}
    }
    result
}
