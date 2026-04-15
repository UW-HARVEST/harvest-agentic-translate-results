use std::os::raw::c_int;

#[repr(C)]
pub struct btac1c_idxstate {
    pub idx: u16,
    pub lpred: i16,
    pub rpred: i16,
    pub tag: u8,
    pub bcfcn: u8,
    pub bsfcn: u8,
    pub usefx: u8,
    pub firfx: [[i16; 8]; 4],
}

macro_rules! psamp {
    ($psamp:expr, $idx:expr, $k:expr) => {
        unsafe { *$psamp.add((($idx - $k) & 7) as usize) }
    };
}

unsafe fn BTAC1C2_PredictSample(psamp: *mut i32, idx: i32, pfcn: i32, ridx: *mut btac1c_idxstate) -> i32 {
    let i = idx;
    let mut pred = 0;
    let p0;
    let p1;
    match pfcn {
        0 => {
            pred = psamp!(psamp, i, 1);
        }
        1 => {
            pred = 2 * psamp!(psamp, i, 1) - psamp!(psamp, i, 2);
        }
        2 => {
            pred = (3 * psamp!(psamp, i, 1) - psamp!(psamp, i, 2)) >> 1;
        }
        3 => {
            pred = (5 * psamp!(psamp, i, 1) - psamp!(psamp, i, 2)) >> 2;
        }
        4 => {
            p0 = psamp!(psamp, i, 1) + psamp!(psamp, i, 2);
            p1 = psamp!(psamp, i, 2) + psamp!(psamp, i, 3);
            pred = p0 - (p1 >> 1);
        }
        5 => {
            p0 = psamp!(psamp, i, 1) + psamp!(psamp, i, 2);
            p1 = psamp!(psamp, i, 2) + psamp!(psamp, i, 3);
            pred = (3 * p0 - p1) >> 2;
        }
        6 => {
            p0 = psamp!(psamp, i, 1) + psamp!(psamp, i, 2);
            p1 = psamp!(psamp, i, 2) + psamp!(psamp, i, 3);
            pred = (5 * p0 - p1) >> 3;
        }
        7 => {
            pred = (18 * psamp!(psamp, i, 1) - 4 * psamp!(psamp, i, 2) +
                    3 * psamp!(psamp, i, 3) - 2 * psamp!(psamp, i, 4) +
                    1 * psamp!(psamp, i, 5)) / 16;
        }
        8 => {
            pred = (72 * psamp!(psamp, i, 1) - 16 * psamp!(psamp, i, 2) +
                    12 * psamp!(psamp, i, 3) - 8 * psamp!(psamp, i, 4) +
                    5 * psamp!(psamp, i, 5) - 3 * psamp!(psamp, i, 6) +
                    3 * psamp!(psamp, i, 7) - 1 * psamp!(psamp, i, 8)) / 64;
        }
        9 => {
            pred = (76 * psamp!(psamp, i, 1) - 17 * psamp!(psamp, i, 2) +
                    10 * psamp!(psamp, i, 3) - 7 * psamp!(psamp, i, 4) +
                    5 * psamp!(psamp, i, 5) - 4 * psamp!(psamp, i, 6) +
                    4 * psamp!(psamp, i, 7) - 3 * psamp!(psamp, i, 8)) / 64;
        }
        10 => {
            p0 = psamp!(psamp, i, 1) + psamp!(psamp, i, 2) + psamp!(psamp, i, 3) + psamp!(psamp, i, 4);
            p1 = psamp!(psamp, i, 5) + psamp!(psamp, i, 6) + psamp!(psamp, i, 7) + psamp!(psamp, i, 8);
            pred = (5 * p0 - p1) >> 4;
        }
        11 => {
            p0 = psamp!(psamp, i, 1) + psamp!(psamp, i, 2) + psamp!(psamp, i, 3) + psamp!(psamp, i, 4);
            p1 = psamp!(psamp, i, 5) + psamp!(psamp, i, 6) + psamp!(psamp, i, 7) + psamp!(psamp, i, 8);
            pred = (p0 + p1) >> 3;
        }
        12 | 13 | 14 | 15 => {
            let firfx = unsafe { (*ridx).firfx[(pfcn - 12) as usize] };
            pred = (firfx[0] as i32 * psamp!(psamp, i, 1) +
                    firfx[1] as i32 * psamp!(psamp, i, 2) +
                    firfx[2] as i32 * psamp!(psamp, i, 3) +
                    firfx[3] as i32 * psamp!(psamp, i, 4) +
                    firfx[4] as i32 * psamp!(psamp, i, 5) +
                    firfx[5] as i32 * psamp!(psamp, i, 6) +
                    firfx[6] as i32 * psamp!(psamp, i, 7) +
                    firfx[7] as i32 * psamp!(psamp, i, 8)) / 256;
        }
        _ => {
            pred = 0;
        }
    }
    pred
}

unsafe fn BTAC1C2_PredictSample_Pfn0(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    psamp!(psamp, idx, 1)
}

unsafe fn BTAC1C2_PredictSample_Pfn1(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    2 * psamp!(psamp, idx, 1) - psamp!(psamp, idx, 2)
}

unsafe fn BTAC1C2_PredictSample_Pfn2(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    (3 * psamp!(psamp, idx, 1) - psamp!(psamp, idx, 2)) >> 1
}

unsafe fn BTAC1C2_PredictSample_Pfn3(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    (5 * psamp!(psamp, idx, 1) - psamp!(psamp, idx, 2)) >> 2
}

unsafe fn BTAC1C2_PredictSample_Pfn4(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    let p0 = psamp!(psamp, idx, 1) + psamp!(psamp, idx, 2);
    let p1 = psamp!(psamp, idx, 2) + psamp!(psamp, idx, 3);
    p0 - (p1 >> 1)
}

unsafe fn BTAC1C2_PredictSample_Pfn5(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    let p0 = psamp!(psamp, idx, 1) + psamp!(psamp, idx, 2);
    let p1 = psamp!(psamp, idx, 2) + psamp!(psamp, idx, 3);
    (3 * p0 - p1) >> 2
}

unsafe fn BTAC1C2_PredictSample_Pfn6(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    let p0 = psamp!(psamp, idx, 1) + psamp!(psamp, idx, 2);
    let p1 = psamp!(psamp, idx, 2) + psamp!(psamp, idx, 3);
    (5 * p0 - p1) >> 3
}

unsafe fn BTAC1C2_PredictSample_Pfn7(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    (18 * psamp!(psamp, idx, 1) - 4 * psamp!(psamp, idx, 2) +
     3 * psamp!(psamp, idx, 3) - 2 * psamp!(psamp, idx, 4) +
     1 * psamp!(psamp, idx, 5)) / 16
}

unsafe fn BTAC1C2_PredictSample_Pfn8(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    (72 * psamp!(psamp, idx, 1) - 16 * psamp!(psamp, idx, 2) +
     12 * psamp!(psamp, idx, 3) - 8 * psamp!(psamp, idx, 4) +
     5 * psamp!(psamp, idx, 5) - 3 * psamp!(psamp, idx, 6) +
     3 * psamp!(psamp, idx, 7) - 1 * psamp!(psamp, idx, 8)) / 64
}

unsafe fn BTAC1C2_PredictSample_Pfn9(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    (76 * psamp!(psamp, idx, 1) - 17 * psamp!(psamp, idx, 2) +
     10 * psamp!(psamp, idx, 3) - 7 * psamp!(psamp, idx, 4) +
     5 * psamp!(psamp, idx, 5) - 4 * psamp!(psamp, idx, 6) +
     4 * psamp!(psamp, idx, 7) - 3 * psamp!(psamp, idx, 8)) / 64
}

unsafe fn BTAC1C2_PredictSample_Pfn10(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    let p0 = psamp!(psamp, idx, 1) + psamp!(psamp, idx, 2) + psamp!(psamp, idx, 3) + psamp!(psamp, idx, 4);
    let p1 = psamp!(psamp, idx, 5) + psamp!(psamp, idx, 6) + psamp!(psamp, idx, 7) + psamp!(psamp, idx, 8);
    (5 * p0 - p1) >> 3
}

unsafe fn BTAC1C2_PredictSample_Pfn11(psamp: *mut i32, idx: i32, _pfcn: i32, _ridx: *mut btac1c_idxstate) -> i32 {
    let p0 = psamp!(psamp, idx, 1) + psamp!(psamp, idx, 2) + psamp!(psamp, idx, 3) + psamp!(psamp, idx, 4);
    let p1 = psamp!(psamp, idx, 5) + psamp!(psamp, idx, 6) + psamp!(psamp, idx, 7) + psamp!(psamp, idx, 8);
    (p0 + p1) >> 1
}

fn BTAC1C2_GetPredictFunc(pfcn: i32) -> usize {
    match pfcn {
        0 => BTAC1C2_PredictSample_Pfn0 as usize,
        1 => BTAC1C2_PredictSample_Pfn1 as usize,
        2 => BTAC1C2_PredictSample_Pfn2 as usize,
        3 => BTAC1C2_PredictSample_Pfn3 as usize,
        4 => BTAC1C2_PredictSample_Pfn4 as usize,
        5 => BTAC1C2_PredictSample_Pfn5 as usize,
        6 => BTAC1C2_PredictSample_Pfn6 as usize,
        7 => BTAC1C2_PredictSample_Pfn7 as usize,
        8 => BTAC1C2_PredictSample_Pfn8 as usize,
        9 => BTAC1C2_PredictSample_Pfn9 as usize,
        10 => BTAC1C2_PredictSample_Pfn10 as usize,
        11 => BTAC1C2_PredictSample_Pfn11 as usize,
        _ => BTAC1C2_PredictSample as usize,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let mut result = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => result = if fcn == BTAC1C2_PredictSample_Pfn0 as usize { 1 } else { 0 },
        1 => result = if fcn == BTAC1C2_PredictSample_Pfn1 as usize { 1 } else { 0 },
        2 => result = if fcn == BTAC1C2_PredictSample_Pfn2 as usize { 1 } else { 0 },
        3 => result = if fcn == BTAC1C2_PredictSample_Pfn3 as usize { 1 } else { 0 },
        4 => result = if fcn == BTAC1C2_PredictSample_Pfn4 as usize { 1 } else { 0 },
        5 => result = if fcn == BTAC1C2_PredictSample_Pfn5 as usize { 1 } else { 0 },
        6 => result = if fcn == BTAC1C2_PredictSample_Pfn6 as usize { 1 } else { 0 },
        7 => result = if fcn == BTAC1C2_PredictSample_Pfn7 as usize { 1 } else { 0 },
        8 => result = if fcn == BTAC1C2_PredictSample_Pfn8 as usize { 1 } else { 0 },
        9 => result = if fcn == BTAC1C2_PredictSample_Pfn9 as usize { 1 } else { 0 },
        10 => result = if fcn == BTAC1C2_PredictSample_Pfn10 as usize { 1 } else { 0 },
        11 => result = if fcn == BTAC1C2_PredictSample_Pfn11 as usize { 1 } else { 0 },
        _ => {}
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    call_predict(pfcn)
}
