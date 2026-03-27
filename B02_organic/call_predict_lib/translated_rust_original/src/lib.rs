use std::os::raw::c_int;

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

type PredictFn = unsafe fn(*mut c_int, c_int, c_int, *mut Btac1cIdxstate) -> c_int;

unsafe fn predict_sample(psamp: *mut c_int, idx: c_int, pfcn: c_int, ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx;
    let s = |off: c_int| -> c_int { *psamp.offset(((i - off) & 7) as isize) };
    match pfcn {
        0 => s(1),
        1 => 2 * s(1) - s(2),
        2 => (3 * s(1) - s(2)) >> 1,
        3 => (5 * s(1) - s(2)) >> 2,
        4 => {
            let p0 = s(1) + s(2);
            let p1 = s(2) + s(3);
            p0 - (p1 >> 1)
        }
        5 => {
            let p0 = s(1) + s(2);
            let p1 = s(2) + s(3);
            (3 * p0 - p1) >> 2
        }
        6 => {
            let p0 = s(1) + s(2);
            let p1 = s(2) + s(3);
            (5 * p0 - p1) >> 3
        }
        7 => {
            (18 * s(1) - 4 * s(2) + 3 * s(3) - 2 * s(4) + 1 * s(5)) / 16
        }
        8 => {
            (72 * s(1) - 16 * s(2) + 12 * s(3) - 8 * s(4) + 5 * s(5) - 3 * s(6) + 3 * s(7) - 1 * s(8)) / 64
        }
        9 => {
            (76 * s(1) - 17 * s(2) + 10 * s(3) - 7 * s(4) + 5 * s(5) - 4 * s(6) + 4 * s(7) - 3 * s(8)) / 64
        }
        10 => {
            let p0 = s(1) + s(2) + s(3) + s(4);
            let p1 = s(5) + s(6) + s(7) + s(8);
            (5 * p0 - p1) >> 4
        }
        11 => {
            let p0 = s(1) + s(2) + s(3) + s(4);
            let p1 = s(5) + s(6) + s(7) + s(8);
            (p0 + p1) >> 3
        }
        12..=15 => {
            let r = &(*ridx).firfx[(pfcn - 12) as usize];
            (r[0] as c_int * s(1) + r[1] as c_int * s(2) + r[2] as c_int * s(3) + r[3] as c_int * s(4)
                + r[4] as c_int * s(5) + r[5] as c_int * s(6) + r[6] as c_int * s(7) + r[7] as c_int * s(8))
                / 256
        }
        _ => 0,
    }
}

unsafe fn predict_pfn0(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    *psamp.offset(((idx - 1) & 7) as isize)
}

unsafe fn predict_pfn1(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    2 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)
}

unsafe fn predict_pfn2(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (3 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)) >> 1
}

unsafe fn predict_pfn3(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (5 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)) >> 2
}

unsafe fn predict_pfn4(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
    let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
    p0 - (p1 >> 1)
}

unsafe fn predict_pfn5(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
    let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
    (3 * p0 - p1) >> 2
}

unsafe fn predict_pfn6(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
    let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
    (5 * p0 - p1) >> 3
}

unsafe fn predict_pfn7(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let s = |off: c_int| *psamp.offset(((idx - off) & 7) as isize);
    (18 * s(1) - 4 * s(2) + 3 * s(3) - 2 * s(4) + 1 * s(5)) / 16
}

unsafe fn predict_pfn8(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let s = |off: c_int| *psamp.offset(((idx - off) & 7) as isize);
    (72 * s(1) - 16 * s(2) + 12 * s(3) - 8 * s(4) + 5 * s(5) - 3 * s(6) + 3 * s(7) - 1 * s(8)) / 64
}

unsafe fn predict_pfn9(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let s = |off: c_int| *psamp.offset(((idx - off) & 7) as isize);
    (76 * s(1) - 17 * s(2) + 10 * s(3) - 7 * s(4) + 5 * s(5) - 4 * s(6) + 4 * s(7) - 3 * s(8)) / 64
}

// NOTE: C original uses >> 3 here, while the switch-case version uses >> 4. Preserving the C bug.
unsafe fn predict_pfn10(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let s = |off: c_int| *psamp.offset(((idx - off) & 7) as isize);
    let p0 = s(1) + s(2) + s(3) + s(4);
    let p1 = s(5) + s(6) + s(7) + s(8);
    (5 * p0 - p1) >> 3
}

// NOTE: C original uses >> 1 here, while the switch-case version uses >> 3. Preserving the C bug.
unsafe fn predict_pfn11(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let s = |off: c_int| *psamp.offset(((idx - off) & 7) as isize);
    let p0 = s(1) + s(2) + s(3) + s(4);
    let p1 = s(5) + s(6) + s(7) + s(8);
    (p0 + p1) >> 1
}

fn get_predict_func_impl(pfcn: c_int) -> PredictFn {
    match pfcn {
        0 => predict_pfn0,
        1 => predict_pfn1,
        2 => predict_pfn2,
        3 => predict_pfn3,
        4 => predict_pfn4,
        5 => predict_pfn5,
        6 => predict_pfn6,
        7 => predict_pfn7,
        8 => predict_pfn8,
        9 => predict_pfn9,
        10 => predict_pfn10,
        11 => predict_pfn11,
        _ => predict_sample,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let fcn = get_predict_func_impl(pfcn);
    let expected: PredictFn = match pfcn {
        0 => predict_pfn0,
        1 => predict_pfn1,
        2 => predict_pfn2,
        3 => predict_pfn3,
        4 => predict_pfn4,
        5 => predict_pfn5,
        6 => predict_pfn6,
        7 => predict_pfn7,
        8 => predict_pfn8,
        9 => predict_pfn9,
        10 => predict_pfn10,
        11 => predict_pfn11,
        _ => return 0,
    };
    (fcn as usize == expected as usize) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    call_predict(pfcn)
}
