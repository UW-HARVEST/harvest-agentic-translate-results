#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]

pub type btac1c_u16 = u16;
pub type btac1c_s16 = i16;
pub type btac1c_byte = u8;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct btac1c_idxstate {
    pub idx: btac1c_u16,
    pub lpred: btac1c_s16,
    pub rpred: btac1c_s16,
    pub tag: btac1c_byte,
    pub bcfcn: btac1c_byte,
    pub bsfcn: btac1c_byte,
    pub usefx: btac1c_byte,
    pub firfx: [[btac1c_s16; 8]; 4],
}

pub type PredictFn = fn(psamp: &[i32], idx: i32, pfcn: i32, ridx: &btac1c_idxstate) -> i32;

fn BTAC1C2_PredictSample(psamp: &[i32], idx: i32, pfcn: i32, ridx: &btac1c_idxstate) -> i32 {
    let i = idx;
    let pred: i32;
    let p0: i32;
    let p1: i32;
    match pfcn {
        0 => {
            pred = psamp[((i - 1) & 7) as usize];
        }
        1 => {
            pred = 2 * psamp[((i - 1) & 7) as usize] - psamp[((i - 2) & 7) as usize];
        }
        2 => {
            pred = (3 * psamp[((i - 1) & 7) as usize] - psamp[((i - 2) & 7) as usize]) >> 1;
        }
        3 => {
            pred = (5 * psamp[((i - 1) & 7) as usize] - psamp[((i - 2) & 7) as usize]) >> 2;
        }
        4 => {
            p0 = psamp[((i - 1) & 7) as usize] + psamp[((i - 2) & 7) as usize];
            p1 = psamp[((i - 2) & 7) as usize] + psamp[((i - 3) & 7) as usize];
            pred = p0 - (p1 >> 1);
        }
        5 => {
            p0 = psamp[((i - 1) & 7) as usize] + psamp[((i - 2) & 7) as usize];
            p1 = psamp[((i - 2) & 7) as usize] + psamp[((i - 3) & 7) as usize];
            pred = (3 * p0 - p1) >> 2;
        }
        6 => {
            p0 = psamp[((i - 1) & 7) as usize] + psamp[((i - 2) & 7) as usize];
            p1 = psamp[((i - 2) & 7) as usize] + psamp[((i - 3) & 7) as usize];
            pred = (5 * p0 - p1) >> 3;
        }
        7 => {
            pred = (18 * psamp[((i - 1) & 7) as usize]
                - 4 * psamp[((i - 2) & 7) as usize]
                + 3 * psamp[((i - 3) & 7) as usize]
                - 2 * psamp[((i - 4) & 7) as usize]
                + 1 * psamp[((i - 5) & 7) as usize])
                / 16;
        }
        8 => {
            pred = (72 * psamp[((i - 1) & 7) as usize]
                - 16 * psamp[((i - 2) & 7) as usize]
                + 12 * psamp[((i - 3) & 7) as usize]
                - 8 * psamp[((i - 4) & 7) as usize]
                + 5 * psamp[((i - 5) & 7) as usize]
                - 3 * psamp[((i - 6) & 7) as usize]
                + 3 * psamp[((i - 7) & 7) as usize]
                - 1 * psamp[((i - 8) & 7) as usize])
                / 64;
        }
        9 => {
            pred = (76 * psamp[((i - 1) & 7) as usize]
                - 17 * psamp[((i - 2) & 7) as usize]
                + 10 * psamp[((i - 3) & 7) as usize]
                - 7 * psamp[((i - 4) & 7) as usize]
                + 5 * psamp[((i - 5) & 7) as usize]
                - 4 * psamp[((i - 6) & 7) as usize]
                + 4 * psamp[((i - 7) & 7) as usize]
                - 3 * psamp[((i - 8) & 7) as usize])
                / 64;
        }
        10 => {
            p0 = psamp[((i - 1) & 7) as usize]
                + psamp[((i - 2) & 7) as usize]
                + psamp[((i - 3) & 7) as usize]
                + psamp[((i - 4) & 7) as usize];
            p1 = psamp[((i - 5) & 7) as usize]
                + psamp[((i - 6) & 7) as usize]
                + psamp[((i - 7) & 7) as usize]
                + psamp[((i - 8) & 7) as usize];
            pred = (5 * p0 - p1) >> 4;
        }
        11 => {
            p0 = psamp[((i - 1) & 7) as usize]
                + psamp[((i - 2) & 7) as usize]
                + psamp[((i - 3) & 7) as usize]
                + psamp[((i - 4) & 7) as usize];
            p1 = psamp[((i - 5) & 7) as usize]
                + psamp[((i - 6) & 7) as usize]
                + psamp[((i - 7) & 7) as usize]
                + psamp[((i - 8) & 7) as usize];
            pred = (p0 + p1) >> 3;
        }
        12 | 13 | 14 | 15 => {
            let row = (pfcn - 12) as usize;
            pred = (ridx.firfx[row][0] as i32 * psamp[((i - 1) & 7) as usize]
                + ridx.firfx[row][1] as i32 * psamp[((i - 2) & 7) as usize]
                + ridx.firfx[row][2] as i32 * psamp[((i - 3) & 7) as usize]
                + ridx.firfx[row][3] as i32 * psamp[((i - 4) & 7) as usize]
                + ridx.firfx[row][4] as i32 * psamp[((i - 5) & 7) as usize]
                + ridx.firfx[row][5] as i32 * psamp[((i - 6) & 7) as usize]
                + ridx.firfx[row][6] as i32 * psamp[((i - 7) & 7) as usize]
                + ridx.firfx[row][7] as i32 * psamp[((i - 8) & 7) as usize])
                / 256;
        }
        _ => {
            pred = 0;
        }
    }
    pred
}

fn BTAC1C2_PredictSample_Pfn0(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    psamp[((idx - 1) & 7) as usize]
}

fn BTAC1C2_PredictSample_Pfn1(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    2 * psamp[((idx - 1) & 7) as usize] - psamp[((idx - 2) & 7) as usize]
}

fn BTAC1C2_PredictSample_Pfn2(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    (3 * psamp[((idx - 1) & 7) as usize] - psamp[((idx - 2) & 7) as usize]) >> 1
}

fn BTAC1C2_PredictSample_Pfn3(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    (5 * psamp[((idx - 1) & 7) as usize] - psamp[((idx - 2) & 7) as usize]) >> 2
}

fn BTAC1C2_PredictSample_Pfn4(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    let p0 = psamp[((idx - 1) & 7) as usize] + psamp[((idx - 2) & 7) as usize];
    let p1 = psamp[((idx - 2) & 7) as usize] + psamp[((idx - 3) & 7) as usize];
    p0 - (p1 >> 1)
}

fn BTAC1C2_PredictSample_Pfn5(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    let p0 = psamp[((idx - 1) & 7) as usize] + psamp[((idx - 2) & 7) as usize];
    let p1 = psamp[((idx - 2) & 7) as usize] + psamp[((idx - 3) & 7) as usize];
    (3 * p0 - p1) >> 2
}

fn BTAC1C2_PredictSample_Pfn6(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    let p0 = psamp[((idx - 1) & 7) as usize] + psamp[((idx - 2) & 7) as usize];
    let p1 = psamp[((idx - 2) & 7) as usize] + psamp[((idx - 3) & 7) as usize];
    (5 * p0 - p1) >> 3
}

fn BTAC1C2_PredictSample_Pfn7(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    (18 * psamp[((idx - 1) & 7) as usize]
        - 4 * psamp[((idx - 2) & 7) as usize]
        + 3 * psamp[((idx - 3) & 7) as usize]
        - 2 * psamp[((idx - 4) & 7) as usize]
        + 1 * psamp[((idx - 5) & 7) as usize])
        / 16
}

fn BTAC1C2_PredictSample_Pfn8(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    (72 * psamp[((idx - 1) & 7) as usize]
        - 16 * psamp[((idx - 2) & 7) as usize]
        + 12 * psamp[((idx - 3) & 7) as usize]
        - 8 * psamp[((idx - 4) & 7) as usize]
        + 5 * psamp[((idx - 5) & 7) as usize]
        - 3 * psamp[((idx - 6) & 7) as usize]
        + 3 * psamp[((idx - 7) & 7) as usize]
        - 1 * psamp[((idx - 8) & 7) as usize])
        / 64
}

fn BTAC1C2_PredictSample_Pfn9(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    (76 * psamp[((idx - 1) & 7) as usize]
        - 17 * psamp[((idx - 2) & 7) as usize]
        + 10 * psamp[((idx - 3) & 7) as usize]
        - 7 * psamp[((idx - 4) & 7) as usize]
        + 5 * psamp[((idx - 5) & 7) as usize]
        - 4 * psamp[((idx - 6) & 7) as usize]
        + 4 * psamp[((idx - 7) & 7) as usize]
        - 3 * psamp[((idx - 8) & 7) as usize])
        / 64
}

fn BTAC1C2_PredictSample_Pfn10(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    let p0 = psamp[((idx - 1) & 7) as usize]
        + psamp[((idx - 2) & 7) as usize]
        + psamp[((idx - 3) & 7) as usize]
        + psamp[((idx - 4) & 7) as usize];
    let p1 = psamp[((idx - 5) & 7) as usize]
        + psamp[((idx - 6) & 7) as usize]
        + psamp[((idx - 7) & 7) as usize]
        + psamp[((idx - 8) & 7) as usize];
    (5 * p0 - p1) >> 3
}

fn BTAC1C2_PredictSample_Pfn11(psamp: &[i32], idx: i32, _pfcn: i32, _ridx: &btac1c_idxstate) -> i32 {
    let p0 = psamp[((idx - 1) & 7) as usize]
        + psamp[((idx - 2) & 7) as usize]
        + psamp[((idx - 3) & 7) as usize]
        + psamp[((idx - 4) & 7) as usize];
    let p1 = psamp[((idx - 5) & 7) as usize]
        + psamp[((idx - 6) & 7) as usize]
        + psamp[((idx - 7) & 7) as usize]
        + psamp[((idx - 8) & 7) as usize];
    (p0 + p1) >> 1
}

fn BTAC1C2_GetPredictFunc(pfcn: i32) -> PredictFn {
    match pfcn {
        0 => BTAC1C2_PredictSample_Pfn0,
        1 => BTAC1C2_PredictSample_Pfn1,
        2 => BTAC1C2_PredictSample_Pfn2,
        3 => BTAC1C2_PredictSample_Pfn3,
        4 => BTAC1C2_PredictSample_Pfn4,
        5 => BTAC1C2_PredictSample_Pfn5,
        6 => BTAC1C2_PredictSample_Pfn6,
        7 => BTAC1C2_PredictSample_Pfn7,
        8 => BTAC1C2_PredictSample_Pfn8,
        9 => BTAC1C2_PredictSample_Pfn9,
        10 => BTAC1C2_PredictSample_Pfn10,
        11 => BTAC1C2_PredictSample_Pfn11,
        _ => BTAC1C2_PredictSample,
    }
}

#[no_mangle]
pub extern "C" fn get_predict_func(pfcn: i32) -> i32 {
    let mut result: i32 = 0;
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    let fcn_addr = fcn as usize;
    match pfcn {
        0 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn0 as *const () as usize) as i32;
        }
        1 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn1 as *const () as usize) as i32;
        }
        2 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn2 as *const () as usize) as i32;
        }
        3 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn3 as *const () as usize) as i32;
        }
        4 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn4 as *const () as usize) as i32;
        }
        5 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn5 as *const () as usize) as i32;
        }
        6 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn6 as *const () as usize) as i32;
        }
        7 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn7 as *const () as usize) as i32;
        }
        8 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn8 as *const () as usize) as i32;
        }
        9 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn9 as *const () as usize) as i32;
        }
        10 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn10 as *const () as usize) as i32;
        }
        11 => {
            result = (fcn_addr == BTAC1C2_PredictSample_Pfn11 as *const () as usize) as i32;
        }
        _ => {}
    }
    result
}
