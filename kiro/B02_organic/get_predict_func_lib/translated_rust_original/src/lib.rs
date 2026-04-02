#![allow(unpredictable_function_pointer_comparisons)]

use std::os::raw::c_int;

type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut Btac1cIdxstate) -> c_int;

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

unsafe extern "C" fn predict_sample(
    psamp: *mut c_int, idx: c_int, pfcn: c_int, ridx: *mut Btac1cIdxstate,
) -> c_int {
    let i = idx as usize;
    match pfcn {
        0 => *psamp.add((i.wrapping_sub(1)) & 7),
        1 => 2 * *psamp.add((i.wrapping_sub(1)) & 7) - *psamp.add((i.wrapping_sub(2)) & 7),
        2 => (3 * *psamp.add((i.wrapping_sub(1)) & 7) - *psamp.add((i.wrapping_sub(2)) & 7)) >> 1,
        3 => (5 * *psamp.add((i.wrapping_sub(1)) & 7) - *psamp.add((i.wrapping_sub(2)) & 7)) >> 2,
        4 => {
            let p0 = *psamp.add((i.wrapping_sub(1)) & 7) + *psamp.add((i.wrapping_sub(2)) & 7);
            let p1 = *psamp.add((i.wrapping_sub(2)) & 7) + *psamp.add((i.wrapping_sub(3)) & 7);
            p0 - (p1 >> 1)
        }
        5 => {
            let p0 = *psamp.add((i.wrapping_sub(1)) & 7) + *psamp.add((i.wrapping_sub(2)) & 7);
            let p1 = *psamp.add((i.wrapping_sub(2)) & 7) + *psamp.add((i.wrapping_sub(3)) & 7);
            (3 * p0 - p1) >> 2
        }
        6 => {
            let p0 = *psamp.add((i.wrapping_sub(1)) & 7) + *psamp.add((i.wrapping_sub(2)) & 7);
            let p1 = *psamp.add((i.wrapping_sub(2)) & 7) + *psamp.add((i.wrapping_sub(3)) & 7);
            (5 * p0 - p1) >> 3
        }
        7 => {
            (18 * *psamp.add((i.wrapping_sub(1)) & 7)
                - 4 * *psamp.add((i.wrapping_sub(2)) & 7)
                + 3 * *psamp.add((i.wrapping_sub(3)) & 7)
                - 2 * *psamp.add((i.wrapping_sub(4)) & 7)
                + *psamp.add((i.wrapping_sub(5)) & 7))
                / 16
        }
        8 => {
            (72 * *psamp.add((i.wrapping_sub(1)) & 7)
                - 16 * *psamp.add((i.wrapping_sub(2)) & 7)
                + 12 * *psamp.add((i.wrapping_sub(3)) & 7)
                - 8 * *psamp.add((i.wrapping_sub(4)) & 7)
                + 5 * *psamp.add((i.wrapping_sub(5)) & 7)
                - 3 * *psamp.add((i.wrapping_sub(6)) & 7)
                + 3 * *psamp.add((i.wrapping_sub(7)) & 7)
                - *psamp.add((i.wrapping_sub(8)) & 7))
                / 64
        }
        9 => {
            (76 * *psamp.add((i.wrapping_sub(1)) & 7)
                - 17 * *psamp.add((i.wrapping_sub(2)) & 7)
                + 10 * *psamp.add((i.wrapping_sub(3)) & 7)
                - 7 * *psamp.add((i.wrapping_sub(4)) & 7)
                + 5 * *psamp.add((i.wrapping_sub(5)) & 7)
                - 4 * *psamp.add((i.wrapping_sub(6)) & 7)
                + 4 * *psamp.add((i.wrapping_sub(7)) & 7)
                - 3 * *psamp.add((i.wrapping_sub(8)) & 7))
                / 64
        }
        10 => {
            let p0 = *psamp.add((i.wrapping_sub(1)) & 7)
                + *psamp.add((i.wrapping_sub(2)) & 7)
                + *psamp.add((i.wrapping_sub(3)) & 7)
                + *psamp.add((i.wrapping_sub(4)) & 7);
            let p1 = *psamp.add((i.wrapping_sub(5)) & 7)
                + *psamp.add((i.wrapping_sub(6)) & 7)
                + *psamp.add((i.wrapping_sub(7)) & 7)
                + *psamp.add((i.wrapping_sub(8)) & 7);
            (5 * p0 - p1) >> 4
        }
        11 => {
            let p0 = *psamp.add((i.wrapping_sub(1)) & 7)
                + *psamp.add((i.wrapping_sub(2)) & 7)
                + *psamp.add((i.wrapping_sub(3)) & 7)
                + *psamp.add((i.wrapping_sub(4)) & 7);
            let p1 = *psamp.add((i.wrapping_sub(5)) & 7)
                + *psamp.add((i.wrapping_sub(6)) & 7)
                + *psamp.add((i.wrapping_sub(7)) & 7)
                + *psamp.add((i.wrapping_sub(8)) & 7);
            (p0 + p1) >> 3
        }
        12..=15 => unsafe {
            let r = &*ridx;
            let fi = (pfcn - 12) as usize;
            (r.firfx[fi][0] as c_int * *psamp.add((i.wrapping_sub(1)) & 7)
                + r.firfx[fi][1] as c_int * *psamp.add((i.wrapping_sub(2)) & 7)
                + r.firfx[fi][2] as c_int * *psamp.add((i.wrapping_sub(3)) & 7)
                + r.firfx[fi][3] as c_int * *psamp.add((i.wrapping_sub(4)) & 7)
                + r.firfx[fi][4] as c_int * *psamp.add((i.wrapping_sub(5)) & 7)
                + r.firfx[fi][5] as c_int * *psamp.add((i.wrapping_sub(6)) & 7)
                + r.firfx[fi][6] as c_int * *psamp.add((i.wrapping_sub(7)) & 7)
                + r.firfx[fi][7] as c_int * *psamp.add((i.wrapping_sub(8)) & 7))
                / 256
        },
        _ => 0,
    }
}

unsafe extern "C" fn pfn0(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    *psamp.add((idx as usize).wrapping_sub(1) & 7)
}
unsafe extern "C" fn pfn1(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    2 * *psamp.add((idx as usize).wrapping_sub(1) & 7) - *psamp.add((idx as usize).wrapping_sub(2) & 7)
}
unsafe extern "C" fn pfn2(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (3 * *psamp.add((idx as usize).wrapping_sub(1) & 7) - *psamp.add((idx as usize).wrapping_sub(2) & 7)) >> 1
}
unsafe extern "C" fn pfn3(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    (5 * *psamp.add((idx as usize).wrapping_sub(1) & 7) - *psamp.add((idx as usize).wrapping_sub(2) & 7)) >> 2
}
unsafe extern "C" fn pfn4(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx as usize;
    let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7);
    let p1 = *psamp.add(i.wrapping_sub(2) & 7) + *psamp.add(i.wrapping_sub(3) & 7);
    p0 - (p1 >> 1)
}
unsafe extern "C" fn pfn5(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx as usize;
    let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7);
    let p1 = *psamp.add(i.wrapping_sub(2) & 7) + *psamp.add(i.wrapping_sub(3) & 7);
    (3 * p0 - p1) >> 2
}
unsafe extern "C" fn pfn6(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx as usize;
    let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7);
    let p1 = *psamp.add(i.wrapping_sub(2) & 7) + *psamp.add(i.wrapping_sub(3) & 7);
    (5 * p0 - p1) >> 3
}
unsafe extern "C" fn pfn7(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx as usize;
    (18 * *psamp.add(i.wrapping_sub(1) & 7)
        - 4 * *psamp.add(i.wrapping_sub(2) & 7)
        + 3 * *psamp.add(i.wrapping_sub(3) & 7)
        - 2 * *psamp.add(i.wrapping_sub(4) & 7)
        + *psamp.add(i.wrapping_sub(5) & 7))
        / 16
}
unsafe extern "C" fn pfn8(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx as usize;
    (72 * *psamp.add(i.wrapping_sub(1) & 7)
        - 16 * *psamp.add(i.wrapping_sub(2) & 7)
        + 12 * *psamp.add(i.wrapping_sub(3) & 7)
        - 8 * *psamp.add(i.wrapping_sub(4) & 7)
        + 5 * *psamp.add(i.wrapping_sub(5) & 7)
        - 3 * *psamp.add(i.wrapping_sub(6) & 7)
        + 3 * *psamp.add(i.wrapping_sub(7) & 7)
        - *psamp.add(i.wrapping_sub(8) & 7))
        / 64
}
unsafe extern "C" fn pfn9(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx as usize;
    (76 * *psamp.add(i.wrapping_sub(1) & 7)
        - 17 * *psamp.add(i.wrapping_sub(2) & 7)
        + 10 * *psamp.add(i.wrapping_sub(3) & 7)
        - 7 * *psamp.add(i.wrapping_sub(4) & 7)
        + 5 * *psamp.add(i.wrapping_sub(5) & 7)
        - 4 * *psamp.add(i.wrapping_sub(6) & 7)
        + 4 * *psamp.add(i.wrapping_sub(7) & 7)
        - 3 * *psamp.add(i.wrapping_sub(8) & 7))
        / 64
}
// Note: Pfn10 uses >>3 (not >>4 like PredictSample case 10) — preserving C behavior
unsafe extern "C" fn pfn10(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx as usize;
    let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7)
        + *psamp.add(i.wrapping_sub(3) & 7) + *psamp.add(i.wrapping_sub(4) & 7);
    let p1 = *psamp.add(i.wrapping_sub(5) & 7) + *psamp.add(i.wrapping_sub(6) & 7)
        + *psamp.add(i.wrapping_sub(7) & 7) + *psamp.add(i.wrapping_sub(8) & 7);
    (5 * p0 - p1) >> 3
}
// Note: Pfn11 uses >>1 (not >>3 like PredictSample case 11) — preserving C behavior
unsafe extern "C" fn pfn11(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx as usize;
    let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7)
        + *psamp.add(i.wrapping_sub(3) & 7) + *psamp.add(i.wrapping_sub(4) & 7);
    let p1 = *psamp.add(i.wrapping_sub(5) & 7) + *psamp.add(i.wrapping_sub(6) & 7)
        + *psamp.add(i.wrapping_sub(7) & 7) + *psamp.add(i.wrapping_sub(8) & 7);
    (p0 + p1) >> 1
}

const PREDICT_FNS: [PredictFn; 12] = [
    pfn0, pfn1, pfn2, pfn3, pfn4, pfn5, pfn6, pfn7, pfn8, pfn9, pfn10, pfn11,
];

fn get_predict_func_impl(pfcn: c_int) -> PredictFn {
    if pfcn >= 0 && (pfcn as usize) < PREDICT_FNS.len() {
        PREDICT_FNS[pfcn as usize]
    } else {
        predict_sample
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let fcn = get_predict_func_impl(pfcn);
    match pfcn {
        0..=11 => (fcn == PREDICT_FNS[pfcn as usize]) as c_int,
        _ => 0,
    }
}
