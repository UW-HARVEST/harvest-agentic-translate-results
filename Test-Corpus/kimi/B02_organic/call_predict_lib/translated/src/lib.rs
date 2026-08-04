use std::os::raw::{c_int, c_void};

#[repr(C)]
pub struct Btac1cIdxstate {
    pub idx: u16,
    pub lpred: i16,
    pub rpred: i16,
    pub tag: u8,
    pub bcfcn: u8,
    pub bsfcn: u8,
    pub usefx: u8,
    pub firfx: [[i16; 8]; 4],
}

type PredictFunc = unsafe extern "C" fn(*const c_int, c_int, c_int, *mut Btac1cIdxstate) -> c_int;

unsafe extern "C" fn btac1c2_predict_sample_pfn0(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    *psamp.add(((idx - 1) & 7) as usize)
}

unsafe extern "C" fn btac1c2_predict_sample_pfn1(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    2 * *psamp.add(((idx - 1) & 7) as usize) - *psamp.add(((idx - 2) & 7) as usize)
}

unsafe extern "C" fn btac1c2_predict_sample_pfn2(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (3 * *psamp.add(((idx - 1) & 7) as usize) - *psamp.add(((idx - 2) & 7) as usize)) >> 1
}

unsafe extern "C" fn btac1c2_predict_sample_pfn3(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (5 * *psamp.add(((idx - 1) & 7) as usize) - *psamp.add(((idx - 2) & 7) as usize)) >> 2
}

unsafe extern "C" fn btac1c2_predict_sample_pfn4(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let p0 = *psamp.add(((idx - 1) & 7) as usize) + *psamp.add(((idx - 2) & 7) as usize);
    let p1 = *psamp.add(((idx - 2) & 7) as usize) + *psamp.add(((idx - 3) & 7) as usize);
    p0 - (p1 >> 1)
}

unsafe extern "C" fn btac1c2_predict_sample_pfn5(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let p0 = *psamp.add(((idx - 1) & 7) as usize) + *psamp.add(((idx - 2) & 7) as usize);
    let p1 = *psamp.add(((idx - 2) & 7) as usize) + *psamp.add(((idx - 3) & 7) as usize);
    (3 * p0 - p1) >> 2
}

unsafe extern "C" fn btac1c2_predict_sample_pfn6(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let p0 = *psamp.add(((idx - 1) & 7) as usize) + *psamp.add(((idx - 2) & 7) as usize);
    let p1 = *psamp.add(((idx - 2) & 7) as usize) + *psamp.add(((idx - 3) & 7) as usize);
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn btac1c2_predict_sample_pfn7(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (18 * *psamp.add(((idx - 1) & 7) as usize) - 4 * *psamp.add(((idx - 2) & 7) as usize) +
     3 * *psamp.add(((idx - 3) & 7) as usize) - 2 * *psamp.add(((idx - 4) & 7) as usize) +
     1 * *psamp.add(((idx - 5) & 7) as usize)) / 16
}

unsafe extern "C" fn btac1c2_predict_sample_pfn8(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (72 * *psamp.add(((idx - 1) & 7) as usize) - 16 * *psamp.add(((idx - 2) & 7) as usize) +
     12 * *psamp.add(((idx - 3) & 7) as usize) - 8 * *psamp.add(((idx - 4) & 7) as usize) +
     5 * *psamp.add(((idx - 5) & 7) as usize) - 3 * *psamp.add(((idx - 6) & 7) as usize) +
     3 * *psamp.add(((idx - 7) & 7) as usize) - 1 * *psamp.add(((idx - 8) & 7) as usize)) / 64
}

unsafe extern "C" fn btac1c2_predict_sample_pfn9(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (76 * *psamp.add(((idx - 1) & 7) as usize) - 17 * *psamp.add(((idx - 2) & 7) as usize) +
     10 * *psamp.add(((idx - 3) & 7) as usize) - 7 * *psamp.add(((idx - 4) & 7) as usize) +
     5 * *psamp.add(((idx - 5) & 7) as usize) - 4 * *psamp.add(((idx - 6) & 7) as usize) +
     4 * *psamp.add(((idx - 7) & 7) as usize) - 3 * *psamp.add(((idx - 8) & 7) as usize)) / 64
}

unsafe extern "C" fn btac1c2_predict_sample_pfn10(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let p0 = *psamp.add(((idx - 1) & 7) as usize) + *psamp.add(((idx - 2) & 7) as usize) +
             *psamp.add(((idx - 3) & 7) as usize) + *psamp.add(((idx - 4) & 7) as usize);
    let p1 = *psamp.add(((idx - 5) & 7) as usize) + *psamp.add(((idx - 6) & 7) as usize) +
             *psamp.add(((idx - 7) & 7) as usize) + *psamp.add(((idx - 8) & 7) as usize);
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn btac1c2_predict_sample_pfn11(psamp: *const c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let p0 = *psamp.add(((idx - 1) & 7) as usize) + *psamp.add(((idx - 2) & 7) as usize) +
             *psamp.add(((idx - 3) & 7) as usize) + *psamp.add(((idx - 4) & 7) as usize);
    let p1 = *psamp.add(((idx - 5) & 7) as usize) + *psamp.add(((idx - 6) & 7) as usize) +
             *psamp.add(((idx - 7) & 7) as usize) + *psamp.add(((idx - 8) & 7) as usize);
    (p0 + p1) >> 1
}

unsafe extern "C" fn btac1c2_predict_sample(psamp: *const c_int, idx: c_int, pfcn: c_int, ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx;
    match pfcn {
        0 => *psamp.add(((i - 1) & 7) as usize),
        1 => 2 * *psamp.add(((i - 1) & 7) as usize) - *psamp.add(((i - 2) & 7) as usize),
        2 => (3 * *psamp.add(((i - 1) & 7) as usize) - *psamp.add(((i - 2) & 7) as usize)) >> 1,
        3 => (5 * *psamp.add(((i - 1) & 7) as usize) - *psamp.add(((i - 2) & 7) as usize)) >> 2,
        4 => {
            let p0 = *psamp.add(((i - 1) & 7) as usize) + *psamp.add(((i - 2) & 7) as usize);
            let p1 = *psamp.add(((i - 2) & 7) as usize) + *psamp.add(((i - 3) & 7) as usize);
            p0 - (p1 >> 1)
        }
        5 => {
            let p0 = *psamp.add(((i - 1) & 7) as usize) + *psamp.add(((i - 2) & 7) as usize);
            let p1 = *psamp.add(((i - 2) & 7) as usize) + *psamp.add(((i - 3) & 7) as usize);
            (3 * p0 - p1) >> 2
        }
        6 => {
            let p0 = *psamp.add(((i - 1) & 7) as usize) + *psamp.add(((i - 2) & 7) as usize);
            let p1 = *psamp.add(((i - 2) & 7) as usize) + *psamp.add(((i - 3) & 7) as usize);
            (5 * p0 - p1) >> 3
        }
        7 => (18 * *psamp.add(((i - 1) & 7) as usize) - 4 * *psamp.add(((i - 2) & 7) as usize) +
              3 * *psamp.add(((i - 3) & 7) as usize) - 2 * *psamp.add(((i - 4) & 7) as usize) +
              1 * *psamp.add(((i - 5) & 7) as usize)) / 16,
        8 => (72 * *psamp.add(((i - 1) & 7) as usize) - 16 * *psamp.add(((i - 2) & 7) as usize) +
              12 * *psamp.add(((i - 3) & 7) as usize) - 8 * *psamp.add(((i - 4) & 7) as usize) +
              5 * *psamp.add(((i - 5) & 7) as usize) - 3 * *psamp.add(((i - 6) & 7) as usize) +
              3 * *psamp.add(((i - 7) & 7) as usize) - 1 * *psamp.add(((i - 8) & 7) as usize)) / 64,
        9 => (76 * *psamp.add(((i - 1) & 7) as usize) - 17 * *psamp.add(((i - 2) & 7) as usize) +
              10 * *psamp.add(((i - 3) & 7) as usize) - 7 * *psamp.add(((i - 4) & 7) as usize) +
              5 * *psamp.add(((i - 5) & 7) as usize) - 4 * *psamp.add(((i - 6) & 7) as usize) +
              4 * *psamp.add(((i - 7) & 7) as usize) - 3 * *psamp.add(((i - 8) & 7) as usize)) / 64,
        10 => {
            let p0 = *psamp.add(((i - 1) & 7) as usize) + *psamp.add(((i - 2) & 7) as usize) +
                     *psamp.add(((i - 3) & 7) as usize) + *psamp.add(((i - 4) & 7) as usize);
            let p1 = *psamp.add(((i - 5) & 7) as usize) + *psamp.add(((i - 6) & 7) as usize) +
                     *psamp.add(((i - 7) & 7) as usize) + *psamp.add(((i - 8) & 7) as usize);
            (5 * p0 - p1) >> 4
        }
        11 => {
            let p0 = *psamp.add(((i - 1) & 7) as usize) + *psamp.add(((i - 2) & 7) as usize) +
                     *psamp.add(((i - 3) & 7) as usize) + *psamp.add(((i - 4) & 7) as usize);
            let p1 = *psamp.add(((i - 5) & 7) as usize) + *psamp.add(((i - 6) & 7) as usize) +
                     *psamp.add(((i - 7) & 7) as usize) + *psamp.add(((i - 8) & 7) as usize);
            (p0 + p1) >> 3
        }
        12..=15 => {
            let fidx = (pfcn - 12) as usize;
            let r = &(*ridx).firfx[fidx];
            (r[0] as c_int * *psamp.add(((i - 1) & 7) as usize) +
             r[1] as c_int * *psamp.add(((i - 2) & 7) as usize) +
             r[2] as c_int * *psamp.add(((i - 3) & 7) as usize) +
             r[3] as c_int * *psamp.add(((i - 4) & 7) as usize) +
             r[4] as c_int * *psamp.add(((i - 5) & 7) as usize) +
             r[5] as c_int * *psamp.add(((i - 6) & 7) as usize) +
             r[6] as c_int * *psamp.add(((i - 7) & 7) as usize) +
             r[7] as c_int * *psamp.add(((i - 8) & 7) as usize)) / 256
        }
        _ => 0,
    }
}

fn btac1c2_get_predict_func(pfcn: c_int) -> *const c_void {
    let fcn: PredictFunc = match pfcn {
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
    };
    fcn as *const c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let fcn = btac1c2_get_predict_func(pfcn);
    let result = match pfcn {
        0 => fcn == btac1c2_predict_sample_pfn0 as *const c_void,
        1 => fcn == btac1c2_predict_sample_pfn1 as *const c_void,
        2 => fcn == btac1c2_predict_sample_pfn2 as *const c_void,
        3 => fcn == btac1c2_predict_sample_pfn3 as *const c_void,
        4 => fcn == btac1c2_predict_sample_pfn4 as *const c_void,
        5 => fcn == btac1c2_predict_sample_pfn5 as *const c_void,
        6 => fcn == btac1c2_predict_sample_pfn6 as *const c_void,
        7 => fcn == btac1c2_predict_sample_pfn7 as *const c_void,
        8 => fcn == btac1c2_predict_sample_pfn8 as *const c_void,
        9 => fcn == btac1c2_predict_sample_pfn9 as *const c_void,
        10 => fcn == btac1c2_predict_sample_pfn10 as *const c_void,
        11 => fcn == btac1c2_predict_sample_pfn11 as *const c_void,
        _ => false,
    };
    result as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> *const c_void {
    btac1c2_get_predict_func(pfcn)
}
