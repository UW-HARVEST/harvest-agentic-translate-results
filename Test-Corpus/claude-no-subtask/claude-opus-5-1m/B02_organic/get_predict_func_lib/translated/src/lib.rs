use std::ffi::c_int;

#[allow(non_camel_case_types)]
type btac1c_u16 = u16;
#[allow(non_camel_case_types)]
type btac1c_s16 = i16;
#[allow(non_camel_case_types)]
type btac1c_byte = u8;

#[repr(C)]
#[allow(non_camel_case_types)]
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

unsafe extern "C" fn btac1c2_predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    let i = idx;
    let p = |off: c_int| -> c_int { unsafe { *psamp.offset(((i - off) & 7) as isize) } };
    let pred: c_int;
    let p0: c_int;
    let p1: c_int;
    match pfcn {
        0 => {
            pred = p(1);
        }
        1 => {
            pred = 2 * p(1) - p(2);
        }
        2 => {
            pred = (3 * p(1) - p(2)) >> 1;
        }
        3 => {
            pred = (5 * p(1) - p(2)) >> 2;
        }
        4 => {
            p0 = p(1) + p(2);
            p1 = p(2) + p(3);
            pred = p0 - (p1 >> 1);
        }
        5 => {
            p0 = p(1) + p(2);
            p1 = p(2) + p(3);
            pred = (3 * p0 - p1) >> 2;
        }
        6 => {
            p0 = p(1) + p(2);
            p1 = p(2) + p(3);
            pred = (5 * p0 - p1) >> 3;
        }
        7 => {
            pred = (18 * p(1) - 4 * p(2) + 3 * p(3) - 2 * p(4) + 1 * p(5)) / 16;
        }
        8 => {
            pred = (72 * p(1) - 16 * p(2) + 12 * p(3) - 8 * p(4) + 5 * p(5)
                - 3 * p(6)
                + 3 * p(7)
                - 1 * p(8))
                / 64;
        }
        9 => {
            pred = (76 * p(1) - 17 * p(2) + 10 * p(3) - 7 * p(4) + 5 * p(5)
                - 4 * p(6)
                + 4 * p(7)
                - 3 * p(8))
                / 64;
        }
        10 => {
            p0 = p(1) + p(2) + p(3) + p(4);
            p1 = p(5) + p(6) + p(7) + p(8);
            pred = (5 * p0 - p1) >> 4;
        }
        11 => {
            p0 = p(1) + p(2) + p(3) + p(4);
            p1 = p(5) + p(6) + p(7) + p(8);
            pred = (p0 + p1) >> 3;
        }
        12 | 13 | 14 | 15 => unsafe {
            let row = &(*ridx).firfx[(pfcn - 12) as usize];
            pred = (row[0] as c_int * p(1)
                + row[1] as c_int * p(2)
                + row[2] as c_int * p(3)
                + row[3] as c_int * p(4)
                + row[4] as c_int * p(5)
                + row[5] as c_int * p(6)
                + row[6] as c_int * p(7)
                + row[7] as c_int * p(8))
                / 256;
        },
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
    unsafe { *psamp.offset(((idx - 1) & 7) as isize) }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        2 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (3 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize))
            >> 1
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        (5 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize))
            >> 2
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
        p0 - (p1 >> 1)
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
        (3 * p0 - p1) >> 2
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut btac1c_idxstate,
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
    _ridx: *mut btac1c_idxstate,
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
    _ridx: *mut btac1c_idxstate,
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
    _ridx: *mut btac1c_idxstate,
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
    _ridx: *mut btac1c_idxstate,
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

fn btac1c2_get_predict_func(pfcn: c_int) -> *const () {
    let fcn: PredictFn = match pfcn {
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
    fcn as *const ()
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let mut result: c_int = 0;
    let fcn = btac1c2_get_predict_func(pfcn);
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
