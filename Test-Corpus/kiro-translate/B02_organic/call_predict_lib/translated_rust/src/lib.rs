use std::ffi::c_int;

#[repr(C)]
struct BtacIdxstate {
    idx: u16,
    lpred: i16,
    rpred: i16,
    tag: u8,
    bcfcn: u8,
    bsfcn: u8,
    usefx: u8,
    firfx: [[i16; 8]; 4],
}

type PredictFn = unsafe fn(*mut c_int, c_int, c_int, *mut BtacIdxstate) -> c_int;

unsafe fn predict_sample(psamp: *mut c_int, idx: c_int, pfcn: c_int, ridx: *mut BtacIdxstate) -> c_int {
    let i = idx;
    match pfcn {
        0 => *psamp.offset(((i - 1) & 7) as isize),
        1 => 2 * *psamp.offset(((i - 1) & 7) as isize) - *psamp.offset(((i - 2) & 7) as isize),
        2 => (3 * *psamp.offset(((i - 1) & 7) as isize) - *psamp.offset(((i - 2) & 7) as isize)) >> 1,
        3 => (5 * *psamp.offset(((i - 1) & 7) as isize) - *psamp.offset(((i - 2) & 7) as isize)) >> 2,
        4 => {
            let p0 = *psamp.offset(((i - 1) & 7) as isize) + *psamp.offset(((i - 2) & 7) as isize);
            let p1 = *psamp.offset(((i - 2) & 7) as isize) + *psamp.offset(((i - 3) & 7) as isize);
            p0 - (p1 >> 1)
        }
        5 => {
            let p0 = *psamp.offset(((i - 1) & 7) as isize) + *psamp.offset(((i - 2) & 7) as isize);
            let p1 = *psamp.offset(((i - 2) & 7) as isize) + *psamp.offset(((i - 3) & 7) as isize);
            (3 * p0 - p1) >> 2
        }
        6 => {
            let p0 = *psamp.offset(((i - 1) & 7) as isize) + *psamp.offset(((i - 2) & 7) as isize);
            let p1 = *psamp.offset(((i - 2) & 7) as isize) + *psamp.offset(((i - 3) & 7) as isize);
            (5 * p0 - p1) >> 3
        }
        7 => {
            (18 * *psamp.offset(((i - 1) & 7) as isize) - 4 * *psamp.offset(((i - 2) & 7) as isize)
                + 3 * *psamp.offset(((i - 3) & 7) as isize) - 2 * *psamp.offset(((i - 4) & 7) as isize)
                + *psamp.offset(((i - 5) & 7) as isize)) / 16
        }
        8 => {
            (72 * *psamp.offset(((i - 1) & 7) as isize) - 16 * *psamp.offset(((i - 2) & 7) as isize)
                + 12 * *psamp.offset(((i - 3) & 7) as isize) - 8 * *psamp.offset(((i - 4) & 7) as isize)
                + 5 * *psamp.offset(((i - 5) & 7) as isize) - 3 * *psamp.offset(((i - 6) & 7) as isize)
                + 3 * *psamp.offset(((i - 7) & 7) as isize) - *psamp.offset(((i - 8) & 7) as isize)) / 64
        }
        9 => {
            (76 * *psamp.offset(((i - 1) & 7) as isize) - 17 * *psamp.offset(((i - 2) & 7) as isize)
                + 10 * *psamp.offset(((i - 3) & 7) as isize) - 7 * *psamp.offset(((i - 4) & 7) as isize)
                + 5 * *psamp.offset(((i - 5) & 7) as isize) - 4 * *psamp.offset(((i - 6) & 7) as isize)
                + 4 * *psamp.offset(((i - 7) & 7) as isize) - 3 * *psamp.offset(((i - 8) & 7) as isize)) / 64
        }
        10 => {
            let p0 = *psamp.offset(((i - 1) & 7) as isize) + *psamp.offset(((i - 2) & 7) as isize)
                + *psamp.offset(((i - 3) & 7) as isize) + *psamp.offset(((i - 4) & 7) as isize);
            let p1 = *psamp.offset(((i - 5) & 7) as isize) + *psamp.offset(((i - 6) & 7) as isize)
                + *psamp.offset(((i - 7) & 7) as isize) + *psamp.offset(((i - 8) & 7) as isize);
            (5 * p0 - p1) >> 4
        }
        11 => {
            let p0 = *psamp.offset(((i - 1) & 7) as isize) + *psamp.offset(((i - 2) & 7) as isize)
                + *psamp.offset(((i - 3) & 7) as isize) + *psamp.offset(((i - 4) & 7) as isize);
            let p1 = *psamp.offset(((i - 5) & 7) as isize) + *psamp.offset(((i - 6) & 7) as isize)
                + *psamp.offset(((i - 7) & 7) as isize) + *psamp.offset(((i - 8) & 7) as isize);
            (p0 + p1) >> 3
        }
        12..=15 => {
            let r = &*ridx;
            let fi = (pfcn - 12) as usize;
            (r.firfx[fi][0] as c_int * *psamp.offset(((i - 1) & 7) as isize)
                + r.firfx[fi][1] as c_int * *psamp.offset(((i - 2) & 7) as isize)
                + r.firfx[fi][2] as c_int * *psamp.offset(((i - 3) & 7) as isize)
                + r.firfx[fi][3] as c_int * *psamp.offset(((i - 4) & 7) as isize)
                + r.firfx[fi][4] as c_int * *psamp.offset(((i - 5) & 7) as isize)
                + r.firfx[fi][5] as c_int * *psamp.offset(((i - 6) & 7) as isize)
                + r.firfx[fi][6] as c_int * *psamp.offset(((i - 7) & 7) as isize)
                + r.firfx[fi][7] as c_int * *psamp.offset(((i - 8) & 7) as isize)) / 256
        }
        _ => 0,
    }
}

unsafe fn predict_pfn0(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    *psamp.offset(((idx - 1) & 7) as isize)
}

unsafe fn predict_pfn1(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    2 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)
}

unsafe fn predict_pfn2(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    (3 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)) >> 1
}

unsafe fn predict_pfn3(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    (5 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)) >> 2
}

unsafe fn predict_pfn4(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
    let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
    p0 - (p1 >> 1)
}

unsafe fn predict_pfn5(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
    let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
    (3 * p0 - p1) >> 2
}

unsafe fn predict_pfn6(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
    let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
    (5 * p0 - p1) >> 3
}

unsafe fn predict_pfn7(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    (18 * *psamp.offset(((idx - 1) & 7) as isize) - 4 * *psamp.offset(((idx - 2) & 7) as isize)
        + 3 * *psamp.offset(((idx - 3) & 7) as isize) - 2 * *psamp.offset(((idx - 4) & 7) as isize)
        + *psamp.offset(((idx - 5) & 7) as isize)) / 16
}

unsafe fn predict_pfn8(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    (72 * *psamp.offset(((idx - 1) & 7) as isize) - 16 * *psamp.offset(((idx - 2) & 7) as isize)
        + 12 * *psamp.offset(((idx - 3) & 7) as isize) - 8 * *psamp.offset(((idx - 4) & 7) as isize)
        + 5 * *psamp.offset(((idx - 5) & 7) as isize) - 3 * *psamp.offset(((idx - 6) & 7) as isize)
        + 3 * *psamp.offset(((idx - 7) & 7) as isize) - *psamp.offset(((idx - 8) & 7) as isize)) / 64
}

unsafe fn predict_pfn9(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    (76 * *psamp.offset(((idx - 1) & 7) as isize) - 17 * *psamp.offset(((idx - 2) & 7) as isize)
        + 10 * *psamp.offset(((idx - 3) & 7) as isize) - 7 * *psamp.offset(((idx - 4) & 7) as isize)
        + 5 * *psamp.offset(((idx - 5) & 7) as isize) - 4 * *psamp.offset(((idx - 6) & 7) as isize)
        + 4 * *psamp.offset(((idx - 7) & 7) as isize) - 3 * *psamp.offset(((idx - 8) & 7) as isize)) / 64
}

// NOTE: Bug preserved from C — uses >> 3 instead of >> 4 as in the switch-case version
unsafe fn predict_pfn10(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize)
        + *psamp.offset(((idx - 3) & 7) as isize) + *psamp.offset(((idx - 4) & 7) as isize);
    let p1 = *psamp.offset(((idx - 5) & 7) as isize) + *psamp.offset(((idx - 6) & 7) as isize)
        + *psamp.offset(((idx - 7) & 7) as isize) + *psamp.offset(((idx - 8) & 7) as isize);
    (5 * p0 - p1) >> 3
}

// NOTE: Bug preserved from C — uses >> 1 instead of >> 3 as in the switch-case version
unsafe fn predict_pfn11(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize)
        + *psamp.offset(((idx - 3) & 7) as isize) + *psamp.offset(((idx - 4) & 7) as isize);
    let p1 = *psamp.offset(((idx - 5) & 7) as isize) + *psamp.offset(((idx - 6) & 7) as isize)
        + *psamp.offset(((idx - 7) & 7) as isize) + *psamp.offset(((idx - 8) & 7) as isize);
    (p0 + p1) >> 1
}

fn get_predict_func_inner(pfcn: c_int) -> PredictFn {
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
    let fcn = get_predict_func_inner(pfcn);
    let fcn_addr = fcn as *const () as usize;
    let result = match pfcn {
        0 => (fcn_addr == predict_pfn0 as *const () as usize) as c_int,
        1 => (fcn_addr == predict_pfn1 as *const () as usize) as c_int,
        2 => (fcn_addr == predict_pfn2 as *const () as usize) as c_int,
        3 => (fcn_addr == predict_pfn3 as *const () as usize) as c_int,
        4 => (fcn_addr == predict_pfn4 as *const () as usize) as c_int,
        5 => (fcn_addr == predict_pfn5 as *const () as usize) as c_int,
        6 => (fcn_addr == predict_pfn6 as *const () as usize) as c_int,
        7 => (fcn_addr == predict_pfn7 as *const () as usize) as c_int,
        8 => (fcn_addr == predict_pfn8 as *const () as usize) as c_int,
        9 => (fcn_addr == predict_pfn9 as *const () as usize) as c_int,
        10 => (fcn_addr == predict_pfn10 as *const () as usize) as c_int,
        11 => (fcn_addr == predict_pfn11 as *const () as usize) as c_int,
        _ => 0,
    };
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    call_predict(pfcn)
}
