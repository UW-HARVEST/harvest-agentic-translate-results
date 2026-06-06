// Translation of c_src/src/lib.c
// Preserves the exact behavior of the original C code.

use std::ffi::c_int;

// btac1c_u16, btac1c_s16, btac1c_byte type aliases
type Btac1cU16 = u16;
type Btac1cS16 = i16;
type Btac1cByte = u8;

// struct btac1c_idxstate_s
#[repr(C)]
#[allow(dead_code)]
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

// Function pointer signature matching the static C functions:
// static int <name>(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx)
type PredictFn = unsafe extern "C" fn(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut Btac1cIdxstate,
) -> c_int;

// Helper: read psamp at offset (idx + delta) & 7. delta is negative or zero.
// Returns the c_int value at that position.
#[inline]
unsafe fn ps(psamp: *mut c_int, idx: c_int, delta: c_int) -> c_int {
    unsafe { *psamp.offset(((idx + delta) & 7) as isize) }
}

unsafe extern "C" fn btac1c2_predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        let i = idx;
        let pred: c_int;
        let p0: c_int;
        let p1: c_int;
        match pfcn {
            0 => {
                pred = ps(psamp, i, -1);
            }
            1 => {
                pred = 2 * ps(psamp, i, -1) - ps(psamp, i, -2);
            }
            2 => {
                pred = (3 * ps(psamp, i, -1) - ps(psamp, i, -2)) >> 1;
            }
            3 => {
                pred = (5 * ps(psamp, i, -1) - ps(psamp, i, -2)) >> 2;
            }
            4 => {
                p0 = ps(psamp, i, -1) + ps(psamp, i, -2);
                p1 = ps(psamp, i, -2) + ps(psamp, i, -3);
                pred = p0 - (p1 >> 1);
            }
            5 => {
                p0 = ps(psamp, i, -1) + ps(psamp, i, -2);
                p1 = ps(psamp, i, -2) + ps(psamp, i, -3);
                pred = (3 * p0 - p1) >> 2;
            }
            6 => {
                p0 = ps(psamp, i, -1) + ps(psamp, i, -2);
                p1 = ps(psamp, i, -2) + ps(psamp, i, -3);
                pred = (5 * p0 - p1) >> 3;
            }
            7 => {
                pred = (18 * ps(psamp, i, -1) - 4 * ps(psamp, i, -2)
                    + 3 * ps(psamp, i, -3)
                    - 2 * ps(psamp, i, -4)
                    + 1 * ps(psamp, i, -5))
                    / 16;
            }
            8 => {
                pred = (72 * ps(psamp, i, -1) - 16 * ps(psamp, i, -2)
                    + 12 * ps(psamp, i, -3)
                    - 8 * ps(psamp, i, -4)
                    + 5 * ps(psamp, i, -5)
                    - 3 * ps(psamp, i, -6)
                    + 3 * ps(psamp, i, -7)
                    - 1 * ps(psamp, i, -8))
                    / 64;
            }
            9 => {
                pred = (76 * ps(psamp, i, -1) - 17 * ps(psamp, i, -2)
                    + 10 * ps(psamp, i, -3)
                    - 7 * ps(psamp, i, -4)
                    + 5 * ps(psamp, i, -5)
                    - 4 * ps(psamp, i, -6)
                    + 4 * ps(psamp, i, -7)
                    - 3 * ps(psamp, i, -8))
                    / 64;
            }
            10 => {
                p0 = ps(psamp, i, -1)
                    + ps(psamp, i, -2)
                    + ps(psamp, i, -3)
                    + ps(psamp, i, -4);
                p1 = ps(psamp, i, -5)
                    + ps(psamp, i, -6)
                    + ps(psamp, i, -7)
                    + ps(psamp, i, -8);
                pred = (5 * p0 - p1) >> 4;
            }
            11 => {
                p0 = ps(psamp, i, -1)
                    + ps(psamp, i, -2)
                    + ps(psamp, i, -3)
                    + ps(psamp, i, -4);
                p1 = ps(psamp, i, -5)
                    + ps(psamp, i, -6)
                    + ps(psamp, i, -7)
                    + ps(psamp, i, -8);
                pred = (p0 + p1) >> 3;
            }
            12 | 13 | 14 | 15 => {
                let row = (pfcn - 12) as usize;
                let r = &(*ridx).firfx[row];
                pred = (r[0] as c_int * ps(psamp, i, -1)
                    + r[1] as c_int * ps(psamp, i, -2)
                    + r[2] as c_int * ps(psamp, i, -3)
                    + r[3] as c_int * ps(psamp, i, -4)
                    + r[4] as c_int * ps(psamp, i, -5)
                    + r[5] as c_int * ps(psamp, i, -6)
                    + r[6] as c_int * ps(psamp, i, -7)
                    + r[7] as c_int * ps(psamp, i, -8))
                    / 256;
            }
            _ => {
                pred = 0;
            }
        }
        pred
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe { ps(psamp, idx, -1) }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe { 2 * ps(psamp, idx, -1) - ps(psamp, idx, -2) }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe { (3 * ps(psamp, idx, -1) - ps(psamp, idx, -2)) >> 1 }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe { (5 * ps(psamp, idx, -1) - ps(psamp, idx, -2)) >> 2 }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxstate,
) -> c_int {
    unsafe {
        let p0 = ps(psamp, idx, -1) + ps(psamp, idx, -2);
        let p1 = ps(psamp, idx, -2) + ps(psamp, idx, -3);
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
        let p0 = ps(psamp, idx, -1) + ps(psamp, idx, -2);
        let p1 = ps(psamp, idx, -2) + ps(psamp, idx, -3);
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
        let p0 = ps(psamp, idx, -1) + ps(psamp, idx, -2);
        let p1 = ps(psamp, idx, -2) + ps(psamp, idx, -3);
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
        (18 * ps(psamp, idx, -1) - 4 * ps(psamp, idx, -2) + 3 * ps(psamp, idx, -3)
            - 2 * ps(psamp, idx, -4)
            + 1 * ps(psamp, idx, -5))
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
        (72 * ps(psamp, idx, -1) - 16 * ps(psamp, idx, -2) + 12 * ps(psamp, idx, -3)
            - 8 * ps(psamp, idx, -4)
            + 5 * ps(psamp, idx, -5)
            - 3 * ps(psamp, idx, -6)
            + 3 * ps(psamp, idx, -7)
            - 1 * ps(psamp, idx, -8))
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
        (76 * ps(psamp, idx, -1) - 17 * ps(psamp, idx, -2) + 10 * ps(psamp, idx, -3)
            - 7 * ps(psamp, idx, -4)
            + 5 * ps(psamp, idx, -5)
            - 4 * ps(psamp, idx, -6)
            + 4 * ps(psamp, idx, -7)
            - 3 * ps(psamp, idx, -8))
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
        let p0 = ps(psamp, idx, -1)
            + ps(psamp, idx, -2)
            + ps(psamp, idx, -3)
            + ps(psamp, idx, -4);
        let p1 = ps(psamp, idx, -5)
            + ps(psamp, idx, -6)
            + ps(psamp, idx, -7)
            + ps(psamp, idx, -8);
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
        let p0 = ps(psamp, idx, -1)
            + ps(psamp, idx, -2)
            + ps(psamp, idx, -3)
            + ps(psamp, idx, -4);
        let p1 = ps(psamp, idx, -5)
            + ps(psamp, idx, -6)
            + ps(psamp, idx, -7)
            + ps(psamp, idx, -8);
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

#[inline]
fn fn_addr(f: PredictFn) -> usize {
    f as *const () as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
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
