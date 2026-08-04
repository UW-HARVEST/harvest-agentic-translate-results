use std::ffi::c_void;
use std::os::raw::c_int;

pub type Btac1cU16 = u16;
pub type Btac1cS16 = i16;
pub type Btac1cByte = u8;

#[repr(C)]
pub struct btac1c_idxstate {
    pub idx: Btac1cU16,
    pub lpred: Btac1cS16,
    pub rpred: Btac1cS16,
    pub tag: Btac1cByte,
    pub bcfcn: Btac1cByte,
    pub bsfcn: Btac1cByte,
    pub usefx: Btac1cByte,
    pub firfx: [[Btac1cS16; 8]; 4],
}

type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut btac1c_idxstate) -> c_int;

fn psamp_at(psamp: *mut c_int, idx: c_int) -> c_int {
    let pos = (idx & 7) as isize;
    unsafe { *psamp.offset(pos) }
}

unsafe extern "C" fn btac1_c2_predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let i = idx;
    match pfcn {
        0 => psamp_at(psamp, i - 1),
        1 => 2 * psamp_at(psamp, i - 1) - psamp_at(psamp, i - 2),
        2 => (3 * psamp_at(psamp, i - 1) - psamp_at(psamp, i - 2)) >> 1,
        3 => (5 * psamp_at(psamp, i - 1) - psamp_at(psamp, i - 2)) >> 2,
        4 => {
            let p0 = psamp_at(psamp, i - 1) + psamp_at(psamp, i - 2);
            let p1 = psamp_at(psamp, i - 2) + psamp_at(psamp, i - 3);
            p0 - (p1 >> 1)
        }
        5 => {
            let p0 = psamp_at(psamp, i - 1) + psamp_at(psamp, i - 2);
            let p1 = psamp_at(psamp, i - 2) + psamp_at(psamp, i - 3);
            (3 * p0 - p1) >> 2
        }
        6 => {
            let p0 = psamp_at(psamp, i - 1) + psamp_at(psamp, i - 2);
            let p1 = psamp_at(psamp, i - 2) + psamp_at(psamp, i - 3);
            (5 * p0 - p1) >> 3
        }
        7 => {
            (18 * psamp_at(psamp, i - 1)
                - 4 * psamp_at(psamp, i - 2)
                + 3 * psamp_at(psamp, i - 3)
                - 2 * psamp_at(psamp, i - 4)
                + psamp_at(psamp, i - 5))
                / 16
        }
        8 => {
            (72 * psamp_at(psamp, i - 1)
                - 16 * psamp_at(psamp, i - 2)
                + 12 * psamp_at(psamp, i - 3)
                - 8 * psamp_at(psamp, i - 4)
                + 5 * psamp_at(psamp, i - 5)
                - 3 * psamp_at(psamp, i - 6)
                + 3 * psamp_at(psamp, i - 7)
                - psamp_at(psamp, i - 8))
                / 64
        }
        9 => {
            (76 * psamp_at(psamp, i - 1)
                - 17 * psamp_at(psamp, i - 2)
                + 10 * psamp_at(psamp, i - 3)
                - 7 * psamp_at(psamp, i - 4)
                + 5 * psamp_at(psamp, i - 5)
                - 4 * psamp_at(psamp, i - 6)
                + 4 * psamp_at(psamp, i - 7)
                - 3 * psamp_at(psamp, i - 8))
                / 64
        }
        10 => {
            let p0 = psamp_at(psamp, i - 1)
                + psamp_at(psamp, i - 2)
                + psamp_at(psamp, i - 3)
                + psamp_at(psamp, i - 4);
            let p1 = psamp_at(psamp, i - 5)
                + psamp_at(psamp, i - 6)
                + psamp_at(psamp, i - 7)
                + psamp_at(psamp, i - 8);
            (5 * p0 - p1) >> 4
        }
        11 => {
            let p0 = psamp_at(psamp, i - 1)
                + psamp_at(psamp, i - 2)
                + psamp_at(psamp, i - 3)
                + psamp_at(psamp, i - 4);
            let p1 = psamp_at(psamp, i - 5)
                + psamp_at(psamp, i - 6)
                + psamp_at(psamp, i - 7)
                + psamp_at(psamp, i - 8);
            (p0 + p1) >> 3
        }
        12..=15 => {
            let rf = unsafe { &(*ridx).firfx[(pfcn - 12) as usize] };
            (rf[0] as c_int * psamp_at(psamp, i - 1)
                + rf[1] as c_int * psamp_at(psamp, i - 2)
                + rf[2] as c_int * psamp_at(psamp, i - 3)
                + rf[3] as c_int * psamp_at(psamp, i - 4)
                + rf[4] as c_int * psamp_at(psamp, i - 5)
                + rf[5] as c_int * psamp_at(psamp, i - 6)
                + rf[6] as c_int * psamp_at(psamp, i - 7)
                + rf[7] as c_int * psamp_at(psamp, i - 8))
                / 256
        }
        _ => 0,
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    psamp_at(psamp, idx - 1)
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    2 * psamp_at(psamp, idx - 1) - psamp_at(psamp, idx - 2)
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (3 * psamp_at(psamp, idx - 1) - psamp_at(psamp, idx - 2)) >> 1
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (5 * psamp_at(psamp, idx - 1) - psamp_at(psamp, idx - 2)) >> 2
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx - 1) + psamp_at(psamp, idx - 2);
    let p1 = psamp_at(psamp, idx - 2) + psamp_at(psamp, idx - 3);
    p0 - (p1 >> 1)
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx - 1) + psamp_at(psamp, idx - 2);
    let p1 = psamp_at(psamp, idx - 2) + psamp_at(psamp, idx - 3);
    (3 * p0 - p1) >> 2
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx - 1) + psamp_at(psamp, idx - 2);
    let p1 = psamp_at(psamp, idx - 2) + psamp_at(psamp, idx - 3);
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (18 * psamp_at(psamp, idx - 1)
        - 4 * psamp_at(psamp, idx - 2)
        + 3 * psamp_at(psamp, idx - 3)
        - 2 * psamp_at(psamp, idx - 4)
        + psamp_at(psamp, idx - 5))
        / 16
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (72 * psamp_at(psamp, idx - 1)
        - 16 * psamp_at(psamp, idx - 2)
        + 12 * psamp_at(psamp, idx - 3)
        - 8 * psamp_at(psamp, idx - 4)
        + 5 * psamp_at(psamp, idx - 5)
        - 3 * psamp_at(psamp, idx - 6)
        + 3 * psamp_at(psamp, idx - 7)
        - psamp_at(psamp, idx - 8))
        / 64
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    (76 * psamp_at(psamp, idx - 1)
        - 17 * psamp_at(psamp, idx - 2)
        + 10 * psamp_at(psamp, idx - 3)
        - 7 * psamp_at(psamp, idx - 4)
        + 5 * psamp_at(psamp, idx - 5)
        - 4 * psamp_at(psamp, idx - 6)
        + 4 * psamp_at(psamp, idx - 7)
        - 3 * psamp_at(psamp, idx - 8))
        / 64
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx - 1)
        + psamp_at(psamp, idx - 2)
        + psamp_at(psamp, idx - 3)
        + psamp_at(psamp, idx - 4);
    let p1 = psamp_at(psamp, idx - 5)
        + psamp_at(psamp, idx - 6)
        + psamp_at(psamp, idx - 7)
        + psamp_at(psamp, idx - 8);
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    let p0 = psamp_at(psamp, idx - 1)
        + psamp_at(psamp, idx - 2)
        + psamp_at(psamp, idx - 3)
        + psamp_at(psamp, idx - 4);
    let p1 = psamp_at(psamp, idx - 5)
        + psamp_at(psamp, idx - 6)
        + psamp_at(psamp, idx - 7)
        + psamp_at(psamp, idx - 8);
    (p0 + p1) >> 1
}

fn btac1_c2_get_predict_func(pfcn: c_int) -> PredictFn {
    match pfcn {
        0 => btac1_c2_predict_sample_pfn0,
        1 => btac1_c2_predict_sample_pfn1,
        2 => btac1_c2_predict_sample_pfn2,
        3 => btac1_c2_predict_sample_pfn3,
        4 => btac1_c2_predict_sample_pfn4,
        5 => btac1_c2_predict_sample_pfn5,
        6 => btac1_c2_predict_sample_pfn6,
        7 => btac1_c2_predict_sample_pfn7,
        8 => btac1_c2_predict_sample_pfn8,
        9 => btac1_c2_predict_sample_pfn9,
        10 => btac1_c2_predict_sample_pfn10,
        11 => btac1_c2_predict_sample_pfn11,
        _ => btac1_c2_predict_sample,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let fcn = btac1_c2_get_predict_func(pfcn) as *const () as *const c_void;
    let result = match pfcn {
        0 => (fcn == btac1_c2_predict_sample_pfn0 as *const () as *const c_void) as c_int,
        1 => (fcn == btac1_c2_predict_sample_pfn1 as *const () as *const c_void) as c_int,
        2 => (fcn == btac1_c2_predict_sample_pfn2 as *const () as *const c_void) as c_int,
        3 => (fcn == btac1_c2_predict_sample_pfn3 as *const () as *const c_void) as c_int,
        4 => (fcn == btac1_c2_predict_sample_pfn4 as *const () as *const c_void) as c_int,
        5 => (fcn == btac1_c2_predict_sample_pfn5 as *const () as *const c_void) as c_int,
        6 => (fcn == btac1_c2_predict_sample_pfn6 as *const () as *const c_void) as c_int,
        7 => (fcn == btac1_c2_predict_sample_pfn7 as *const () as *const c_void) as c_int,
        8 => (fcn == btac1_c2_predict_sample_pfn8 as *const () as *const c_void) as c_int,
        9 => (fcn == btac1_c2_predict_sample_pfn9 as *const () as *const c_void) as c_int,
        10 => (fcn == btac1_c2_predict_sample_pfn10 as *const () as *const c_void) as c_int,
        11 => (fcn == btac1_c2_predict_sample_pfn11 as *const () as *const c_void) as c_int,
        _ => 0,
    };
    result
}
