use std::ffi::c_int;

type U16 = u16;
type S16 = i16;
type Byte = u8;

#[repr(C)]
struct IdxState {
    idx: U16,
    lpred: S16,
    rpred: S16,
    tag: Byte,
    bcfcn: Byte,
    bsfcn: Byte,
    usefx: Byte,
    firfx: [[S16; 8]; 4],
}

type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut IdxState) -> c_int;

#[inline]
unsafe fn sample(psamp: *mut c_int, idx: c_int, offset: c_int) -> c_int {
    unsafe { *psamp.add(((idx - offset) & 7) as usize) }
}

unsafe extern "C" fn predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut IdxState,
) -> c_int {
    match pfcn {
        0 => unsafe { sample(psamp, idx, 1) },
        1 => unsafe { 2 * sample(psamp, idx, 1) - sample(psamp, idx, 2) },
        2 => unsafe { (3 * sample(psamp, idx, 1) - sample(psamp, idx, 2)) >> 1 },
        3 => unsafe { (5 * sample(psamp, idx, 1) - sample(psamp, idx, 2)) >> 2 },
        4 => unsafe {
            let p0 = sample(psamp, idx, 1) + sample(psamp, idx, 2);
            let p1 = sample(psamp, idx, 2) + sample(psamp, idx, 3);
            p0 - (p1 >> 1)
        },
        5 => unsafe {
            let p0 = sample(psamp, idx, 1) + sample(psamp, idx, 2);
            let p1 = sample(psamp, idx, 2) + sample(psamp, idx, 3);
            (3 * p0 - p1) >> 2
        },
        6 => unsafe {
            let p0 = sample(psamp, idx, 1) + sample(psamp, idx, 2);
            let p1 = sample(psamp, idx, 2) + sample(psamp, idx, 3);
            (5 * p0 - p1) >> 3
        },
        7 => unsafe {
            (18 * sample(psamp, idx, 1) - 4 * sample(psamp, idx, 2) + 3 * sample(psamp, idx, 3)
                - 2 * sample(psamp, idx, 4)
                + sample(psamp, idx, 5))
                / 16
        },
        8 => unsafe {
            (72 * sample(psamp, idx, 1) - 16 * sample(psamp, idx, 2) + 12 * sample(psamp, idx, 3)
                - 8 * sample(psamp, idx, 4)
                + 5 * sample(psamp, idx, 5)
                - 3 * sample(psamp, idx, 6)
                + 3 * sample(psamp, idx, 7)
                - sample(psamp, idx, 8))
                / 64
        },
        9 => unsafe {
            (76 * sample(psamp, idx, 1) - 17 * sample(psamp, idx, 2) + 10 * sample(psamp, idx, 3)
                - 7 * sample(psamp, idx, 4)
                + 5 * sample(psamp, idx, 5)
                - 4 * sample(psamp, idx, 6)
                + 4 * sample(psamp, idx, 7)
                - 3 * sample(psamp, idx, 8))
                / 64
        },
        10 => unsafe {
            let p0 = sample(psamp, idx, 1)
                + sample(psamp, idx, 2)
                + sample(psamp, idx, 3)
                + sample(psamp, idx, 4);
            let p1 = sample(psamp, idx, 5)
                + sample(psamp, idx, 6)
                + sample(psamp, idx, 7)
                + sample(psamp, idx, 8);
            (5 * p0 - p1) >> 4
        },
        11 => unsafe {
            let p0 = sample(psamp, idx, 1)
                + sample(psamp, idx, 2)
                + sample(psamp, idx, 3)
                + sample(psamp, idx, 4);
            let p1 = sample(psamp, idx, 5)
                + sample(psamp, idx, 6)
                + sample(psamp, idx, 7)
                + sample(psamp, idx, 8);
            (p0 + p1) >> 3
        },
        12..=15 => unsafe {
            let filter = &(*ridx).firfx[(pfcn - 12) as usize];
            let mut prediction = 0;
            for (tap, coefficient) in filter.iter().enumerate() {
                prediction += c_int::from(*coefficient) * sample(psamp, idx, (tap + 1) as c_int);
            }
            prediction / 256
        },
        _ => 0,
    }
}

unsafe extern "C" fn predict_sample_pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe { sample(psamp, idx, 1) }
}

unsafe extern "C" fn predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe { 2 * sample(psamp, idx, 1) - sample(psamp, idx, 2) }
}

unsafe extern "C" fn predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe { (3 * sample(psamp, idx, 1) - sample(psamp, idx, 2)) >> 1 }
}

unsafe extern "C" fn predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe { (5 * sample(psamp, idx, 1) - sample(psamp, idx, 2)) >> 2 }
}

unsafe extern "C" fn predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1) + sample(psamp, idx, 2);
        let p1 = sample(psamp, idx, 2) + sample(psamp, idx, 3);
        p0 - (p1 >> 1)
    }
}

unsafe extern "C" fn predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1) + sample(psamp, idx, 2);
        let p1 = sample(psamp, idx, 2) + sample(psamp, idx, 3);
        (3 * p0 - p1) >> 2
    }
}

unsafe extern "C" fn predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1) + sample(psamp, idx, 2);
        let p1 = sample(psamp, idx, 2) + sample(psamp, idx, 3);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe {
        (18 * sample(psamp, idx, 1) - 4 * sample(psamp, idx, 2) + 3 * sample(psamp, idx, 3)
            - 2 * sample(psamp, idx, 4)
            + sample(psamp, idx, 5))
            / 16
    }
}

unsafe extern "C" fn predict_sample_pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe {
        (72 * sample(psamp, idx, 1) - 16 * sample(psamp, idx, 2) + 12 * sample(psamp, idx, 3)
            - 8 * sample(psamp, idx, 4)
            + 5 * sample(psamp, idx, 5)
            - 3 * sample(psamp, idx, 6)
            + 3 * sample(psamp, idx, 7)
            - sample(psamp, idx, 8))
            / 64
    }
}

unsafe extern "C" fn predict_sample_pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe {
        (76 * sample(psamp, idx, 1) - 17 * sample(psamp, idx, 2) + 10 * sample(psamp, idx, 3)
            - 7 * sample(psamp, idx, 4)
            + 5 * sample(psamp, idx, 5)
            - 4 * sample(psamp, idx, 6)
            + 4 * sample(psamp, idx, 7)
            - 3 * sample(psamp, idx, 8))
            / 64
    }
}

unsafe extern "C" fn predict_sample_pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1)
            + sample(psamp, idx, 2)
            + sample(psamp, idx, 3)
            + sample(psamp, idx, 4);
        let p1 = sample(psamp, idx, 5)
            + sample(psamp, idx, 6)
            + sample(psamp, idx, 7)
            + sample(psamp, idx, 8);
        (5 * p0 - p1) >> 3
    }
}

unsafe extern "C" fn predict_sample_pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut IdxState,
) -> c_int {
    unsafe {
        let p0 = sample(psamp, idx, 1)
            + sample(psamp, idx, 2)
            + sample(psamp, idx, 3)
            + sample(psamp, idx, 4);
        let p1 = sample(psamp, idx, 5)
            + sample(psamp, idx, 6)
            + sample(psamp, idx, 7)
            + sample(psamp, idx, 8);
        (p0 + p1) >> 1
    }
}

fn get_predict_func(pfcn: c_int) -> PredictFn {
    match pfcn {
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
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let selected = get_predict_func(pfcn);
    let expected = match pfcn {
        0 => Some(predict_sample_pfn0 as PredictFn),
        1 => Some(predict_sample_pfn1 as PredictFn),
        2 => Some(predict_sample_pfn2 as PredictFn),
        3 => Some(predict_sample_pfn3 as PredictFn),
        4 => Some(predict_sample_pfn4 as PredictFn),
        5 => Some(predict_sample_pfn5 as PredictFn),
        6 => Some(predict_sample_pfn6 as PredictFn),
        7 => Some(predict_sample_pfn7 as PredictFn),
        8 => Some(predict_sample_pfn8 as PredictFn),
        9 => Some(predict_sample_pfn9 as PredictFn),
        10 => Some(predict_sample_pfn10 as PredictFn),
        11 => Some(predict_sample_pfn11 as PredictFn),
        _ => None,
    };

    expected.is_some_and(|function| std::ptr::fn_addr_eq(selected, function)) as c_int
}
