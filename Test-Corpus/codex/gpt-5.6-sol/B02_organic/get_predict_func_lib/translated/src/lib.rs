use std::ffi::c_int;

#[repr(C)]
struct IdxState {
    idx: u16,
    lpred: i16,
    rpred: i16,
    tag: u8,
    bcfcn: u8,
    bsfcn: u8,
    usefx: u8,
    firfx: [[i16; 8]; 4],
}

type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut IdxState) -> c_int;

unsafe fn sample(samples: *mut c_int, index: c_int) -> c_int {
    unsafe { *samples.add((index & 7) as usize) }
}

unsafe extern "C" fn predict_sample(
    samples: *mut c_int,
    index: c_int,
    function: c_int,
    state: *mut IdxState,
) -> c_int {
    unsafe {
        match function {
            0 => sample(samples, index - 1),
            1 => 2 * sample(samples, index - 1) - sample(samples, index - 2),
            2 => (3 * sample(samples, index - 1) - sample(samples, index - 2)) >> 1,
            3 => (5 * sample(samples, index - 1) - sample(samples, index - 2)) >> 2,
            4 => {
                let p0 = sample(samples, index - 1) + sample(samples, index - 2);
                let p1 = sample(samples, index - 2) + sample(samples, index - 3);
                p0 - (p1 >> 1)
            }
            5 => {
                let p0 = sample(samples, index - 1) + sample(samples, index - 2);
                let p1 = sample(samples, index - 2) + sample(samples, index - 3);
                (3 * p0 - p1) >> 2
            }
            6 => {
                let p0 = sample(samples, index - 1) + sample(samples, index - 2);
                let p1 = sample(samples, index - 2) + sample(samples, index - 3);
                (5 * p0 - p1) >> 3
            }
            7 => {
                (18 * sample(samples, index - 1) - 4 * sample(samples, index - 2)
                    + 3 * sample(samples, index - 3)
                    - 2 * sample(samples, index - 4)
                    + sample(samples, index - 5))
                    / 16
            }
            8 => {
                (72 * sample(samples, index - 1) - 16 * sample(samples, index - 2)
                    + 12 * sample(samples, index - 3)
                    - 8 * sample(samples, index - 4)
                    + 5 * sample(samples, index - 5)
                    - 3 * sample(samples, index - 6)
                    + 3 * sample(samples, index - 7)
                    - sample(samples, index - 8))
                    / 64
            }
            9 => {
                (76 * sample(samples, index - 1) - 17 * sample(samples, index - 2)
                    + 10 * sample(samples, index - 3)
                    - 7 * sample(samples, index - 4)
                    + 5 * sample(samples, index - 5)
                    - 4 * sample(samples, index - 6)
                    + 4 * sample(samples, index - 7)
                    - 3 * sample(samples, index - 8))
                    / 64
            }
            10 => {
                let p0 = sample(samples, index - 1)
                    + sample(samples, index - 2)
                    + sample(samples, index - 3)
                    + sample(samples, index - 4);
                let p1 = sample(samples, index - 5)
                    + sample(samples, index - 6)
                    + sample(samples, index - 7)
                    + sample(samples, index - 8);
                (5 * p0 - p1) >> 4
            }
            11 => {
                let p0 = sample(samples, index - 1)
                    + sample(samples, index - 2)
                    + sample(samples, index - 3)
                    + sample(samples, index - 4);
                let p1 = sample(samples, index - 5)
                    + sample(samples, index - 6)
                    + sample(samples, index - 7)
                    + sample(samples, index - 8);
                (p0 + p1) >> 3
            }
            12..=15 => {
                let coefficients = &(*state).firfx[(function - 12) as usize];
                (coefficients[0] as c_int * sample(samples, index - 1)
                    + coefficients[1] as c_int * sample(samples, index - 2)
                    + coefficients[2] as c_int * sample(samples, index - 3)
                    + coefficients[3] as c_int * sample(samples, index - 4)
                    + coefficients[4] as c_int * sample(samples, index - 5)
                    + coefficients[5] as c_int * sample(samples, index - 6)
                    + coefficients[6] as c_int * sample(samples, index - 7)
                    + coefficients[7] as c_int * sample(samples, index - 8))
                    / 256
            }
            _ => 0,
        }
    }
}

unsafe extern "C" fn predict_sample_pfn0(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe { sample(samples, index - 1) }
}

unsafe extern "C" fn predict_sample_pfn1(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe { 2 * sample(samples, index - 1) - sample(samples, index - 2) }
}

unsafe extern "C" fn predict_sample_pfn2(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe { (3 * sample(samples, index - 1) - sample(samples, index - 2)) >> 1 }
}

unsafe extern "C" fn predict_sample_pfn3(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe { (5 * sample(samples, index - 1) - sample(samples, index - 2)) >> 2 }
}

unsafe extern "C" fn predict_sample_pfn4(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(samples, index - 1) + sample(samples, index - 2);
        let p1 = sample(samples, index - 2) + sample(samples, index - 3);
        p0 - (p1 >> 1)
    }
}

unsafe extern "C" fn predict_sample_pfn5(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(samples, index - 1) + sample(samples, index - 2);
        let p1 = sample(samples, index - 2) + sample(samples, index - 3);
        (3 * p0 - p1) >> 2
    }
}

unsafe extern "C" fn predict_sample_pfn6(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(samples, index - 1) + sample(samples, index - 2);
        let p1 = sample(samples, index - 2) + sample(samples, index - 3);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn predict_sample_pfn7(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe {
        (18 * sample(samples, index - 1) - 4 * sample(samples, index - 2)
            + 3 * sample(samples, index - 3)
            - 2 * sample(samples, index - 4)
            + sample(samples, index - 5))
            / 16
    }
}

unsafe extern "C" fn predict_sample_pfn8(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe {
        (72 * sample(samples, index - 1) - 16 * sample(samples, index - 2)
            + 12 * sample(samples, index - 3)
            - 8 * sample(samples, index - 4)
            + 5 * sample(samples, index - 5)
            - 3 * sample(samples, index - 6)
            + 3 * sample(samples, index - 7)
            - sample(samples, index - 8))
            / 64
    }
}

unsafe extern "C" fn predict_sample_pfn9(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe {
        (76 * sample(samples, index - 1) - 17 * sample(samples, index - 2)
            + 10 * sample(samples, index - 3)
            - 7 * sample(samples, index - 4)
            + 5 * sample(samples, index - 5)
            - 4 * sample(samples, index - 6)
            + 4 * sample(samples, index - 7)
            - 3 * sample(samples, index - 8))
            / 64
    }
}

unsafe extern "C" fn predict_sample_pfn10(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(samples, index - 1)
            + sample(samples, index - 2)
            + sample(samples, index - 3)
            + sample(samples, index - 4);
        let p1 = sample(samples, index - 5)
            + sample(samples, index - 6)
            + sample(samples, index - 7)
            + sample(samples, index - 8);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn predict_sample_pfn11(
    samples: *mut c_int,
    index: c_int,
    _function: c_int,
    _state: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(samples, index - 1)
            + sample(samples, index - 2)
            + sample(samples, index - 3)
            + sample(samples, index - 4);
        let p1 = sample(samples, index - 5)
            + sample(samples, index - 6)
            + sample(samples, index - 7)
            + sample(samples, index - 8);
        (p0 + p1) >> 1
    }
}

fn get_predict_fn(function: c_int) -> PredictFn {
    match function {
        0 => predict_sample_pfn0,
        1 => predict_sample_pfn1,
        2 => predict_sample_pfn2,
        3 => predict_sample_pfn3,
        4 => predict_sample_pfn4,
        5 => predict_sample_pfn5,
        6 => predict_sample_pfn6,
        7 => predict_sample_pfn7,
        8 => predict_sample_pfn8,
        9 => predict_sample_pfn9,
        10 => predict_sample_pfn10,
        11 => predict_sample_pfn11,
        _ => predict_sample,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(function: c_int) -> c_int {
    let selected = get_predict_fn(function);
    match function {
        0 => std::ptr::fn_addr_eq(selected, predict_sample_pfn0 as PredictFn) as c_int,
        1 => std::ptr::fn_addr_eq(selected, predict_sample_pfn1 as PredictFn) as c_int,
        2 => std::ptr::fn_addr_eq(selected, predict_sample_pfn2 as PredictFn) as c_int,
        3 => std::ptr::fn_addr_eq(selected, predict_sample_pfn3 as PredictFn) as c_int,
        4 => std::ptr::fn_addr_eq(selected, predict_sample_pfn4 as PredictFn) as c_int,
        5 => std::ptr::fn_addr_eq(selected, predict_sample_pfn5 as PredictFn) as c_int,
        6 => std::ptr::fn_addr_eq(selected, predict_sample_pfn6 as PredictFn) as c_int,
        7 => std::ptr::fn_addr_eq(selected, predict_sample_pfn7 as PredictFn) as c_int,
        8 => std::ptr::fn_addr_eq(selected, predict_sample_pfn8 as PredictFn) as c_int,
        9 => std::ptr::fn_addr_eq(selected, predict_sample_pfn9 as PredictFn) as c_int,
        10 => std::ptr::fn_addr_eq(selected, predict_sample_pfn10 as PredictFn) as c_int,
        11 => std::ptr::fn_addr_eq(selected, predict_sample_pfn11 as PredictFn) as c_int,
        _ => 0,
    }
}
