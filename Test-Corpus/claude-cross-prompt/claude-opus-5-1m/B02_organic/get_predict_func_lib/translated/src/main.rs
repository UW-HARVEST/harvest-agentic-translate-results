// Translation of c_src/src/lib.c — a library with no main function.
// The original C builds as a shared library (per CMakeLists.txt) and produces
// no output on its own. This executable mirrors that by producing no output.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::too_many_arguments)]

type Btac1cU16 = u16;
type Btac1cS16 = i16;
type Btac1cByte = u8;

#[derive(Default, Clone)]
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

type PredictFn = fn(&[i32; 8], i32, i32, &Btac1cIdxstate) -> i32;

fn idx_mod(i: i32, off: i32) -> usize {
    ((i - off) & 7) as usize
}

fn btac1c2_predict_sample(
    psamp: &[i32; 8],
    idx: i32,
    pfcn: i32,
    ridx: &Btac1cIdxstate,
) -> i32 {
    let i = idx;
    let pred: i32;
    let p0: i32;
    let p1: i32;
    match pfcn {
        0 => {
            pred = psamp[idx_mod(i, 1)];
        }
        1 => {
            pred = 2 * psamp[idx_mod(i, 1)] - psamp[idx_mod(i, 2)];
        }
        2 => {
            pred = (3 * psamp[idx_mod(i, 1)] - psamp[idx_mod(i, 2)]) >> 1;
        }
        3 => {
            pred = (5 * psamp[idx_mod(i, 1)] - psamp[idx_mod(i, 2)]) >> 2;
        }
        4 => {
            p0 = psamp[idx_mod(i, 1)] + psamp[idx_mod(i, 2)];
            p1 = psamp[idx_mod(i, 2)] + psamp[idx_mod(i, 3)];
            pred = p0 - (p1 >> 1);
        }
        5 => {
            p0 = psamp[idx_mod(i, 1)] + psamp[idx_mod(i, 2)];
            p1 = psamp[idx_mod(i, 2)] + psamp[idx_mod(i, 3)];
            pred = (3 * p0 - p1) >> 2;
        }
        6 => {
            p0 = psamp[idx_mod(i, 1)] + psamp[idx_mod(i, 2)];
            p1 = psamp[idx_mod(i, 2)] + psamp[idx_mod(i, 3)];
            pred = (5 * p0 - p1) >> 3;
        }
        7 => {
            pred = (18 * psamp[idx_mod(i, 1)] - 4 * psamp[idx_mod(i, 2)]
                + 3 * psamp[idx_mod(i, 3)]
                - 2 * psamp[idx_mod(i, 4)]
                + 1 * psamp[idx_mod(i, 5)])
                / 16;
        }
        8 => {
            pred = (72 * psamp[idx_mod(i, 1)] - 16 * psamp[idx_mod(i, 2)]
                + 12 * psamp[idx_mod(i, 3)]
                - 8 * psamp[idx_mod(i, 4)]
                + 5 * psamp[idx_mod(i, 5)]
                - 3 * psamp[idx_mod(i, 6)]
                + 3 * psamp[idx_mod(i, 7)]
                - 1 * psamp[idx_mod(i, 8)])
                / 64;
        }
        9 => {
            pred = (76 * psamp[idx_mod(i, 1)] - 17 * psamp[idx_mod(i, 2)]
                + 10 * psamp[idx_mod(i, 3)]
                - 7 * psamp[idx_mod(i, 4)]
                + 5 * psamp[idx_mod(i, 5)]
                - 4 * psamp[idx_mod(i, 6)]
                + 4 * psamp[idx_mod(i, 7)]
                - 3 * psamp[idx_mod(i, 8)])
                / 64;
        }
        10 => {
            p0 = psamp[idx_mod(i, 1)]
                + psamp[idx_mod(i, 2)]
                + psamp[idx_mod(i, 3)]
                + psamp[idx_mod(i, 4)];
            p1 = psamp[idx_mod(i, 5)]
                + psamp[idx_mod(i, 6)]
                + psamp[idx_mod(i, 7)]
                + psamp[idx_mod(i, 8)];
            pred = (5 * p0 - p1) >> 4;
        }
        11 => {
            p0 = psamp[idx_mod(i, 1)]
                + psamp[idx_mod(i, 2)]
                + psamp[idx_mod(i, 3)]
                + psamp[idx_mod(i, 4)];
            p1 = psamp[idx_mod(i, 5)]
                + psamp[idx_mod(i, 6)]
                + psamp[idx_mod(i, 7)]
                + psamp[idx_mod(i, 8)];
            pred = (p0 + p1) >> 3;
        }
        12 | 13 | 14 | 15 => {
            let row = (pfcn - 12) as usize;
            let f = &ridx.firfx[row];
            pred = (f[0] as i32 * psamp[idx_mod(i, 1)]
                + f[1] as i32 * psamp[idx_mod(i, 2)]
                + f[2] as i32 * psamp[idx_mod(i, 3)]
                + f[3] as i32 * psamp[idx_mod(i, 4)]
                + f[4] as i32 * psamp[idx_mod(i, 5)]
                + f[5] as i32 * psamp[idx_mod(i, 6)]
                + f[6] as i32 * psamp[idx_mod(i, 7)]
                + f[7] as i32 * psamp[idx_mod(i, 8)])
                / 256;
        }
        _ => {
            pred = 0;
        }
    }
    pred
}

fn btac1c2_predict_sample_pfn0(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    psamp[idx_mod(idx, 1)]
}

fn btac1c2_predict_sample_pfn1(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    2 * psamp[idx_mod(idx, 1)] - psamp[idx_mod(idx, 2)]
}

fn btac1c2_predict_sample_pfn2(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    (3 * psamp[idx_mod(idx, 1)] - psamp[idx_mod(idx, 2)]) >> 1
}

fn btac1c2_predict_sample_pfn3(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    (5 * psamp[idx_mod(idx, 1)] - psamp[idx_mod(idx, 2)]) >> 2
}

fn btac1c2_predict_sample_pfn4(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    let p0 = psamp[idx_mod(idx, 1)] + psamp[idx_mod(idx, 2)];
    let p1 = psamp[idx_mod(idx, 2)] + psamp[idx_mod(idx, 3)];
    p0 - (p1 >> 1)
}

fn btac1c2_predict_sample_pfn5(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    let p0 = psamp[idx_mod(idx, 1)] + psamp[idx_mod(idx, 2)];
    let p1 = psamp[idx_mod(idx, 2)] + psamp[idx_mod(idx, 3)];
    (3 * p0 - p1) >> 2
}

fn btac1c2_predict_sample_pfn6(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    let p0 = psamp[idx_mod(idx, 1)] + psamp[idx_mod(idx, 2)];
    let p1 = psamp[idx_mod(idx, 2)] + psamp[idx_mod(idx, 3)];
    (5 * p0 - p1) >> 3
}

fn btac1c2_predict_sample_pfn7(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    (18 * psamp[idx_mod(idx, 1)] - 4 * psamp[idx_mod(idx, 2)]
        + 3 * psamp[idx_mod(idx, 3)]
        - 2 * psamp[idx_mod(idx, 4)]
        + 1 * psamp[idx_mod(idx, 5)])
        / 16
}

fn btac1c2_predict_sample_pfn8(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    (72 * psamp[idx_mod(idx, 1)] - 16 * psamp[idx_mod(idx, 2)]
        + 12 * psamp[idx_mod(idx, 3)]
        - 8 * psamp[idx_mod(idx, 4)]
        + 5 * psamp[idx_mod(idx, 5)]
        - 3 * psamp[idx_mod(idx, 6)]
        + 3 * psamp[idx_mod(idx, 7)]
        - 1 * psamp[idx_mod(idx, 8)])
        / 64
}

fn btac1c2_predict_sample_pfn9(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    (76 * psamp[idx_mod(idx, 1)] - 17 * psamp[idx_mod(idx, 2)]
        + 10 * psamp[idx_mod(idx, 3)]
        - 7 * psamp[idx_mod(idx, 4)]
        + 5 * psamp[idx_mod(idx, 5)]
        - 4 * psamp[idx_mod(idx, 6)]
        + 4 * psamp[idx_mod(idx, 7)]
        - 3 * psamp[idx_mod(idx, 8)])
        / 64
}

fn btac1c2_predict_sample_pfn10(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    let p0 = psamp[idx_mod(idx, 1)]
        + psamp[idx_mod(idx, 2)]
        + psamp[idx_mod(idx, 3)]
        + psamp[idx_mod(idx, 4)];
    let p1 = psamp[idx_mod(idx, 5)]
        + psamp[idx_mod(idx, 6)]
        + psamp[idx_mod(idx, 7)]
        + psamp[idx_mod(idx, 8)];
    (5 * p0 - p1) >> 3
}

fn btac1c2_predict_sample_pfn11(psamp: &[i32; 8], idx: i32, _pfcn: i32, _ridx: &Btac1cIdxstate) -> i32 {
    let p0 = psamp[idx_mod(idx, 1)]
        + psamp[idx_mod(idx, 2)]
        + psamp[idx_mod(idx, 3)]
        + psamp[idx_mod(idx, 4)];
    let p1 = psamp[idx_mod(idx, 5)]
        + psamp[idx_mod(idx, 6)]
        + psamp[idx_mod(idx, 7)]
        + psamp[idx_mod(idx, 8)];
    (p0 + p1) >> 1
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum FnSelector {
    Pfn0,
    Pfn1,
    Pfn2,
    Pfn3,
    Pfn4,
    Pfn5,
    Pfn6,
    Pfn7,
    Pfn8,
    Pfn9,
    Pfn10,
    Pfn11,
    Generic,
}

fn btac1c2_get_predict_func(pfcn: i32) -> FnSelector {
    match pfcn {
        0 => FnSelector::Pfn0,
        1 => FnSelector::Pfn1,
        2 => FnSelector::Pfn2,
        3 => FnSelector::Pfn3,
        4 => FnSelector::Pfn4,
        5 => FnSelector::Pfn5,
        6 => FnSelector::Pfn6,
        7 => FnSelector::Pfn7,
        8 => FnSelector::Pfn8,
        9 => FnSelector::Pfn9,
        10 => FnSelector::Pfn10,
        11 => FnSelector::Pfn11,
        _ => FnSelector::Generic,
    }
}

pub fn get_predict_func(pfcn: i32) -> i32 {
    let fcn = btac1c2_get_predict_func(pfcn);
    let result: i32 = match pfcn {
        0 => (fcn == FnSelector::Pfn0) as i32,
        1 => (fcn == FnSelector::Pfn1) as i32,
        2 => (fcn == FnSelector::Pfn2) as i32,
        3 => (fcn == FnSelector::Pfn3) as i32,
        4 => (fcn == FnSelector::Pfn4) as i32,
        5 => (fcn == FnSelector::Pfn5) as i32,
        6 => (fcn == FnSelector::Pfn6) as i32,
        7 => (fcn == FnSelector::Pfn7) as i32,
        8 => (fcn == FnSelector::Pfn8) as i32,
        9 => (fcn == FnSelector::Pfn9) as i32,
        10 => (fcn == FnSelector::Pfn10) as i32,
        11 => (fcn == FnSelector::Pfn11) as i32,
        _ => 0,
    };
    result
}

// Reference the dispatch tables and library functions to silence dead-code
// warnings while preserving the translated logic.
fn _dispatch(psamp: &[i32; 8], idx: i32, pfcn: i32, ridx: &Btac1cIdxstate) -> i32 {
    let fns: [PredictFn; 12] = [
        btac1c2_predict_sample_pfn0,
        btac1c2_predict_sample_pfn1,
        btac1c2_predict_sample_pfn2,
        btac1c2_predict_sample_pfn3,
        btac1c2_predict_sample_pfn4,
        btac1c2_predict_sample_pfn5,
        btac1c2_predict_sample_pfn6,
        btac1c2_predict_sample_pfn7,
        btac1c2_predict_sample_pfn8,
        btac1c2_predict_sample_pfn9,
        btac1c2_predict_sample_pfn10,
        btac1c2_predict_sample_pfn11,
    ];
    if (0..12).contains(&pfcn) {
        fns[pfcn as usize](psamp, idx, pfcn, ridx)
    } else {
        btac1c2_predict_sample(psamp, idx, pfcn, ridx)
    }
}

fn main() {
    // Original C source defines a shared library with no main entry point and
    // produces no stdout/stderr output. Match that behavior exactly.
}
