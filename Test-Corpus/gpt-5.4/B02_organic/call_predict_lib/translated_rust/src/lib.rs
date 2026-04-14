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

unsafe extern "C" fn btac1_c2_predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let i = idx;
    match pfcn {
        0 => unsafe { *psamp.add(((i - 1) & 7) as usize) },
        1 => unsafe {
            2 * *psamp.add(((i - 1) & 7) as usize) - *psamp.add(((i - 2) & 7) as usize)
        },
        2 => unsafe {
            (3 * *psamp.add(((i - 1) & 7) as usize) - *psamp.add(((i - 2) & 7) as usize)) >> 1
        },
        3 => unsafe {
            (5 * *psamp.add(((i - 1) & 7) as usize) - *psamp.add(((i - 2) & 7) as usize)) >> 2
        },
        4 => unsafe {
            let p0 = *psamp.add(((i - 1) & 7) as usize) + *psamp.add(((i - 2) & 7) as usize);
            let p1 = *psamp.add(((i - 2) & 7) as usize) + *psamp.add(((i - 3) & 7) as usize);
            p0 - (p1 >> 1)
        },
        5 => unsafe {
            let p0 = *psamp.add(((i - 1) & 7) as usize) + *psamp.add(((i - 2) & 7) as usize);
            let p1 = *psamp.add(((i - 2) & 7) as usize) + *psamp.add(((i - 3) & 7) as usize);
            (3 * p0 - p1) >> 2
        },
        6 => unsafe {
            let p0 = *psamp.add(((i - 1) & 7) as usize) + *psamp.add(((i - 2) & 7) as usize);
            let p1 = *psamp.add(((i - 2) & 7) as usize) + *psamp.add(((i - 3) & 7) as usize);
            (5 * p0 - p1) >> 3
        },
        7 => unsafe {
            (18 * *psamp.add(((i - 1) & 7) as usize)
                - 4 * *psamp.add(((i - 2) & 7) as usize)
                + 3 * *psamp.add(((i - 3) & 7) as usize)
                - 2 * *psamp.add(((i - 4) & 7) as usize)
                + *psamp.add(((i - 5) & 7) as usize))
                / 16
        },
        8 => unsafe {
            (72 * *psamp.add(((i - 1) & 7) as usize)
                - 16 * *psamp.add(((i - 2) & 7) as usize)
                + 12 * *psamp.add(((i - 3) & 7) as usize)
                - 8 * *psamp.add(((i - 4) & 7) as usize)
                + 5 * *psamp.add(((i - 5) & 7) as usize)
                - 3 * *psamp.add(((i - 6) & 7) as usize)
                + 3 * *psamp.add(((i - 7) & 7) as usize)
                - *psamp.add(((i - 8) & 7) as usize))
                / 64
        },
        9 => unsafe {
            (76 * *psamp.add(((i - 1) & 7) as usize)
                - 17 * *psamp.add(((i - 2) & 7) as usize)
                + 10 * *psamp.add(((i - 3) & 7) as usize)
                - 7 * *psamp.add(((i - 4) & 7) as usize)
                + 5 * *psamp.add(((i - 5) & 7) as usize)
                - 4 * *psamp.add(((i - 6) & 7) as usize)
                + 4 * *psamp.add(((i - 7) & 7) as usize)
                - 3 * *psamp.add(((i - 8) & 7) as usize))
                / 64
        },
        10 => unsafe {
            let p0 = *psamp.add(((i - 1) & 7) as usize)
                + *psamp.add(((i - 2) & 7) as usize)
                + *psamp.add(((i - 3) & 7) as usize)
                + *psamp.add(((i - 4) & 7) as usize);
            let p1 = *psamp.add(((i - 5) & 7) as usize)
                + *psamp.add(((i - 6) & 7) as usize)
                + *psamp.add(((i - 7) & 7) as usize)
                + *psamp.add(((i - 8) & 7) as usize);
            (5 * p0 - p1) >> 4
        },
        11 => unsafe {
            let p0 = *psamp.add(((i - 1) & 7) as usize)
                + *psamp.add(((i - 2) & 7) as usize)
                + *psamp.add(((i - 3) & 7) as usize)
                + *psamp.add(((i - 4) & 7) as usize);
            let p1 = *psamp.add(((i - 5) & 7) as usize)
                + *psamp.add(((i - 6) & 7) as usize)
                + *psamp.add(((i - 7) & 7) as usize)
                + *psamp.add(((i - 8) & 7) as usize);
            (p0 + p1) >> 3
        },
        12..=15 => unsafe {
            let fir = &(*ridx).firfx[(pfcn - 12) as usize];
            (fir[0] as c_int * *psamp.add(((i - 1) & 7) as usize)
                + fir[1] as c_int * *psamp.add(((i - 2) & 7) as usize)
                + fir[2] as c_int * *psamp.add(((i - 3) & 7) as usize)
                + fir[3] as c_int * *psamp.add(((i - 4) & 7) as usize)
                + fir[4] as c_int * *psamp.add(((i - 5) & 7) as usize)
                + fir[5] as c_int * *psamp.add(((i - 6) & 7) as usize)
                + fir[6] as c_int * *psamp.add(((i - 7) & 7) as usize)
                + fir[7] as c_int * *psamp.add(((i - 8) & 7) as usize))
                / 256
        },
        _ => 0,
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { *psamp.add(((idx - 1) & 7) as usize) }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { 2 * *psamp.add(((idx - 1) & 7) as usize) - *psamp.add(((idx - 2) & 7) as usize) }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { (3 * *psamp.add(((idx - 1) & 7) as usize) - *psamp.add(((idx - 2) & 7) as usize)) >> 1 }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { (5 * *psamp.add(((idx - 1) & 7) as usize) - *psamp.add(((idx - 2) & 7) as usize)) >> 2 }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.add(((idx - 1) & 7) as usize) + *psamp.add(((idx - 2) & 7) as usize);
        let p1 = *psamp.add(((idx - 2) & 7) as usize) + *psamp.add(((idx - 3) & 7) as usize);
        p0 - (p1 >> 1)
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.add(((idx - 1) & 7) as usize) + *psamp.add(((idx - 2) & 7) as usize);
        let p1 = *psamp.add(((idx - 2) & 7) as usize) + *psamp.add(((idx - 3) & 7) as usize);
        (3 * p0 - p1) >> 2
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.add(((idx - 1) & 7) as usize) + *psamp.add(((idx - 2) & 7) as usize);
        let p1 = *psamp.add(((idx - 2) & 7) as usize) + *psamp.add(((idx - 3) & 7) as usize);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (18 * *psamp.add(((idx - 1) & 7) as usize)
            - 4 * *psamp.add(((idx - 2) & 7) as usize)
            + 3 * *psamp.add(((idx - 3) & 7) as usize)
            - 2 * *psamp.add(((idx - 4) & 7) as usize)
            + *psamp.add(((idx - 5) & 7) as usize))
            / 16
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (72 * *psamp.add(((idx - 1) & 7) as usize)
            - 16 * *psamp.add(((idx - 2) & 7) as usize)
            + 12 * *psamp.add(((idx - 3) & 7) as usize)
            - 8 * *psamp.add(((idx - 4) & 7) as usize)
            + 5 * *psamp.add(((idx - 5) & 7) as usize)
            - 3 * *psamp.add(((idx - 6) & 7) as usize)
            + 3 * *psamp.add(((idx - 7) & 7) as usize)
            - *psamp.add(((idx - 8) & 7) as usize))
            / 64
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (76 * *psamp.add(((idx - 1) & 7) as usize)
            - 17 * *psamp.add(((idx - 2) & 7) as usize)
            + 10 * *psamp.add(((idx - 3) & 7) as usize)
            - 7 * *psamp.add(((idx - 4) & 7) as usize)
            + 5 * *psamp.add(((idx - 5) & 7) as usize)
            - 4 * *psamp.add(((idx - 6) & 7) as usize)
            + 4 * *psamp.add(((idx - 7) & 7) as usize)
            - 3 * *psamp.add(((idx - 8) & 7) as usize))
            / 64
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.add(((idx - 1) & 7) as usize)
            + *psamp.add(((idx - 2) & 7) as usize)
            + *psamp.add(((idx - 3) & 7) as usize)
            + *psamp.add(((idx - 4) & 7) as usize);
        let p1 = *psamp.add(((idx - 5) & 7) as usize)
            + *psamp.add(((idx - 6) & 7) as usize)
            + *psamp.add(((idx - 7) & 7) as usize)
            + *psamp.add(((idx - 8) & 7) as usize);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn btac1_c2_predict_sample_pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.add(((idx - 1) & 7) as usize)
            + *psamp.add(((idx - 2) & 7) as usize)
            + *psamp.add(((idx - 3) & 7) as usize)
            + *psamp.add(((idx - 4) & 7) as usize);
        let p1 = *psamp.add(((idx - 5) & 7) as usize)
            + *psamp.add(((idx - 6) & 7) as usize)
            + *psamp.add(((idx - 7) & 7) as usize)
            + *psamp.add(((idx - 8) & 7) as usize);
        (p0 + p1) >> 1
    }
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
pub extern "C" fn get_predict_func(pfcn: c_int) -> *mut c_void {
    btac1_c2_get_predict_func(pfcn) as *const () as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let fcn = btac1_c2_get_predict_func(pfcn);
    match pfcn {
        0 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn0 as PredictFn)) as c_int,
        1 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn1 as PredictFn)) as c_int,
        2 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn2 as PredictFn)) as c_int,
        3 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn3 as PredictFn)) as c_int,
        4 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn4 as PredictFn)) as c_int,
        5 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn5 as PredictFn)) as c_int,
        6 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn6 as PredictFn)) as c_int,
        7 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn7 as PredictFn)) as c_int,
        8 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn8 as PredictFn)) as c_int,
        9 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn9 as PredictFn)) as c_int,
        10 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn10 as PredictFn)) as c_int,
        11 => (std::ptr::fn_addr_eq(fcn, btac1_c2_predict_sample_pfn11 as PredictFn)) as c_int,
        _ => 0,
    }
}
