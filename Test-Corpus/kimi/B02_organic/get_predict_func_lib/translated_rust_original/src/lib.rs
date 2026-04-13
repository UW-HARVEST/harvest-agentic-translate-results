use std::os::raw::{c_int, c_void};

#[repr(C)]
struct Btac1cIdxstate {
    idx: u16,
    lpred: i16,
    rpred: i16,
    tag: u8,
    bcfcn: u8,
    bsfcn: u8,
    usefx: u8,
    firfx: [[i16; 8]; 4],
}

fn btac1c2_predict_sample(psamp: &[i32; 8], idx: usize, pfcn: i32, ridx: *const Btac1cIdxstate) -> i32 {
    let i = idx;
    match pfcn {
        0 => psamp[(i.wrapping_sub(1)) & 7],
        1 => 2 * psamp[(i.wrapping_sub(1)) & 7] - psamp[(i.wrapping_sub(2)) & 7],
        2 => (3 * psamp[(i.wrapping_sub(1)) & 7] - psamp[(i.wrapping_sub(2)) & 7]) >> 1,
        3 => (5 * psamp[(i.wrapping_sub(1)) & 7] - psamp[(i.wrapping_sub(2)) & 7]) >> 2,
        4 => {
            let p0 = psamp[(i.wrapping_sub(1)) & 7] + psamp[(i.wrapping_sub(2)) & 7];
            let p1 = psamp[(i.wrapping_sub(2)) & 7] + psamp[(i.wrapping_sub(3)) & 7];
            p0 - (p1 >> 1)
        }
        5 => {
            let p0 = psamp[(i.wrapping_sub(1)) & 7] + psamp[(i.wrapping_sub(2)) & 7];
            let p1 = psamp[(i.wrapping_sub(2)) & 7] + psamp[(i.wrapping_sub(3)) & 7];
            (3 * p0 - p1) >> 2
        }
        6 => {
            let p0 = psamp[(i.wrapping_sub(1)) & 7] + psamp[(i.wrapping_sub(2)) & 7];
            let p1 = psamp[(i.wrapping_sub(2)) & 7] + psamp[(i.wrapping_sub(3)) & 7];
            (5 * p0 - p1) >> 3
        }
        7 => (18 * psamp[(i.wrapping_sub(1)) & 7] - 4 * psamp[(i.wrapping_sub(2)) & 7]
            + 3 * psamp[(i.wrapping_sub(3)) & 7] - 2 * psamp[(i.wrapping_sub(4)) & 7]
            + 1 * psamp[(i.wrapping_sub(5)) & 7]) / 16,
        8 => (72 * psamp[(i.wrapping_sub(1)) & 7] - 16 * psamp[(i.wrapping_sub(2)) & 7]
            + 12 * psamp[(i.wrapping_sub(3)) & 7] - 8 * psamp[(i.wrapping_sub(4)) & 7]
            + 5 * psamp[(i.wrapping_sub(5)) & 7] - 3 * psamp[(i.wrapping_sub(6)) & 7]
            + 3 * psamp[(i.wrapping_sub(7)) & 7] - 1 * psamp[(i.wrapping_sub(8)) & 7]) / 64,
        9 => (76 * psamp[(i.wrapping_sub(1)) & 7] - 17 * psamp[(i.wrapping_sub(2)) & 7]
            + 10 * psamp[(i.wrapping_sub(3)) & 7] - 7 * psamp[(i.wrapping_sub(4)) & 7]
            + 5 * psamp[(i.wrapping_sub(5)) & 7] - 4 * psamp[(i.wrapping_sub(6)) & 7]
            + 4 * psamp[(i.wrapping_sub(7)) & 7] - 3 * psamp[(i.wrapping_sub(8)) & 7]) / 64,
        10 => {
            let p0 = psamp[(i.wrapping_sub(1)) & 7] + psamp[(i.wrapping_sub(2)) & 7]
                + psamp[(i.wrapping_sub(3)) & 7] + psamp[(i.wrapping_sub(4)) & 7];
            let p1 = psamp[(i.wrapping_sub(5)) & 7] + psamp[(i.wrapping_sub(6)) & 7]
                + psamp[(i.wrapping_sub(7)) & 7] + psamp[(i.wrapping_sub(8)) & 7];
            (5 * p0 - p1) >> 3
        }
        11 => {
            let p0 = psamp[(i.wrapping_sub(1)) & 7] + psamp[(i.wrapping_sub(2)) & 7]
                + psamp[(i.wrapping_sub(3)) & 7] + psamp[(i.wrapping_sub(4)) & 7];
            let p1 = psamp[(i.wrapping_sub(5)) & 7] + psamp[(i.wrapping_sub(6)) & 7]
                + psamp[(i.wrapping_sub(7)) & 7] + psamp[(i.wrapping_sub(8)) & 7];
            (p0 + p1) >> 1
        }
        12..=15 => {
            let fx = unsafe { &(*ridx).firfx[(pfcn - 12) as usize] };
            (fx[0] as i32 * psamp[(i.wrapping_sub(1)) & 7]
                + fx[1] as i32 * psamp[(i.wrapping_sub(2)) & 7]
                + fx[2] as i32 * psamp[(i.wrapping_sub(3)) & 7]
                + fx[3] as i32 * psamp[(i.wrapping_sub(4)) & 7]
                + fx[4] as i32 * psamp[(i.wrapping_sub(5)) & 7]
                + fx[5] as i32 * psamp[(i.wrapping_sub(6)) & 7]
                + fx[6] as i32 * psamp[(i.wrapping_sub(7)) & 7]
                + fx[7] as i32 * psamp[(i.wrapping_sub(8)) & 7]) / 256
        }
        _ => 0,
    }
}

fn btac1c2_predict_sample_pfn0(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    psamp[(idx.wrapping_sub(1)) & 7]
}

fn btac1c2_predict_sample_pfn1(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    2 * psamp[(idx.wrapping_sub(1)) & 7] - psamp[(idx.wrapping_sub(2)) & 7]
}

fn btac1c2_predict_sample_pfn2(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    (3 * psamp[(idx.wrapping_sub(1)) & 7] - psamp[(idx.wrapping_sub(2)) & 7]) >> 1
}

fn btac1c2_predict_sample_pfn3(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    (5 * psamp[(idx.wrapping_sub(1)) & 7] - psamp[(idx.wrapping_sub(2)) & 7]) >> 2
}

fn btac1c2_predict_sample_pfn4(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    let p0 = psamp[(idx.wrapping_sub(1)) & 7] + psamp[(idx.wrapping_sub(2)) & 7];
    let p1 = psamp[(idx.wrapping_sub(2)) & 7] + psamp[(idx.wrapping_sub(3)) & 7];
    p0 - (p1 >> 1)
}

fn btac1c2_predict_sample_pfn5(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    let p0 = psamp[(idx.wrapping_sub(1)) & 7] + psamp[(idx.wrapping_sub(2)) & 7];
    let p1 = psamp[(idx.wrapping_sub(2)) & 7] + psamp[(idx.wrapping_sub(3)) & 7];
    (3 * p0 - p1) >> 2
}

fn btac1c2_predict_sample_pfn6(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    let p0 = psamp[(idx.wrapping_sub(1)) & 7] + psamp[(idx.wrapping_sub(2)) & 7];
    let p1 = psamp[(idx.wrapping_sub(2)) & 7] + psamp[(idx.wrapping_sub(3)) & 7];
    (5 * p0 - p1) >> 3
}

fn btac1c2_predict_sample_pfn7(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    (18 * psamp[(idx.wrapping_sub(1)) & 7] - 4 * psamp[(idx.wrapping_sub(2)) & 7]
        + 3 * psamp[(idx.wrapping_sub(3)) & 7] - 2 * psamp[(idx.wrapping_sub(4)) & 7]
        + 1 * psamp[(idx.wrapping_sub(5)) & 7]) / 16
}

fn btac1c2_predict_sample_pfn8(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    (72 * psamp[(idx.wrapping_sub(1)) & 7] - 16 * psamp[(idx.wrapping_sub(2)) & 7]
        + 12 * psamp[(idx.wrapping_sub(3)) & 7] - 8 * psamp[(idx.wrapping_sub(4)) & 7]
        + 5 * psamp[(idx.wrapping_sub(5)) & 7] - 3 * psamp[(idx.wrapping_sub(6)) & 7]
        + 3 * psamp[(idx.wrapping_sub(7)) & 7] - 1 * psamp[(idx.wrapping_sub(8)) & 7]) / 64
}

fn btac1c2_predict_sample_pfn9(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    (76 * psamp[(idx.wrapping_sub(1)) & 7] - 17 * psamp[(idx.wrapping_sub(2)) & 7]
        + 10 * psamp[(idx.wrapping_sub(3)) & 7] - 7 * psamp[(idx.wrapping_sub(4)) & 7]
        + 5 * psamp[(idx.wrapping_sub(5)) & 7] - 4 * psamp[(idx.wrapping_sub(6)) & 7]
        + 4 * psamp[(idx.wrapping_sub(7)) & 7] - 3 * psamp[(idx.wrapping_sub(8)) & 7]) / 64
}

fn btac1c2_predict_sample_pfn10(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    let p0 = psamp[(idx.wrapping_sub(1)) & 7] + psamp[(idx.wrapping_sub(2)) & 7]
        + psamp[(idx.wrapping_sub(3)) & 7] + psamp[(idx.wrapping_sub(4)) & 7];
    let p1 = psamp[(idx.wrapping_sub(5)) & 7] + psamp[(idx.wrapping_sub(6)) & 7]
        + psamp[(idx.wrapping_sub(7)) & 7] + psamp[(idx.wrapping_sub(8)) & 7];
    (5 * p0 - p1) >> 3
}

fn btac1c2_predict_sample_pfn11(psamp: &[i32; 8], idx: usize, _pfcn: i32, _ridx: *const Btac1cIdxstate) -> i32 {
    let p0 = psamp[(idx.wrapping_sub(1)) & 7] + psamp[(idx.wrapping_sub(2)) & 7]
        + psamp[(idx.wrapping_sub(3)) & 7] + psamp[(idx.wrapping_sub(4)) & 7];
    let p1 = psamp[(idx.wrapping_sub(5)) & 7] + psamp[(idx.wrapping_sub(6)) & 7]
        + psamp[(idx.wrapping_sub(7)) & 7] + psamp[(idx.wrapping_sub(8)) & 7];
    (p0 + p1) >> 1
}

type PredictFn = fn(&[i32; 8], usize, i32, *const Btac1cIdxstate) -> i32;

fn btac1c2_get_predict_func(pfcn: i32) -> PredictFn {
    match pfcn {
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
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let fcn = btac1c2_get_predict_func(pfcn);
    let result = match pfcn {
        0 => fcn as *const c_void == btac1c2_predict_sample_pfn0 as *const c_void,
        1 => fcn as *const c_void == btac1c2_predict_sample_pfn1 as *const c_void,
        2 => fcn as *const c_void == btac1c2_predict_sample_pfn2 as *const c_void,
        3 => fcn as *const c_void == btac1c2_predict_sample_pfn3 as *const c_void,
        4 => fcn as *const c_void == btac1c2_predict_sample_pfn4 as *const c_void,
        5 => fcn as *const c_void == btac1c2_predict_sample_pfn5 as *const c_void,
        6 => fcn as *const c_void == btac1c2_predict_sample_pfn6 as *const c_void,
        7 => fcn as *const c_void == btac1c2_predict_sample_pfn7 as *const c_void,
        8 => fcn as *const c_void == btac1c2_predict_sample_pfn8 as *const c_void,
        9 => fcn as *const c_void == btac1c2_predict_sample_pfn9 as *const c_void,
        10 => fcn as *const c_void == btac1c2_predict_sample_pfn10 as *const c_void,
        11 => fcn as *const c_void == btac1c2_predict_sample_pfn11 as *const c_void,
        _ => false,
    };
    result as c_int
}
