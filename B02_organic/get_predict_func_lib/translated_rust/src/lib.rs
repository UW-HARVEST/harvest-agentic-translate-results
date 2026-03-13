use std::os::raw::c_int;

type BtacU16 = u16;
type BtacS16 = i16;
type BtacByte = u8;

#[repr(C)]
struct BtacIdxstate {
    idx: BtacU16,
    lpred: BtacS16,
    rpred: BtacS16,
    tag: BtacByte,
    bcfcn: BtacByte,
    bsfcn: BtacByte,
    usefx: BtacByte,
    firfx: [[BtacS16; 8]; 4],
}

type PredictFn = unsafe fn(*mut c_int, c_int, c_int, *mut BtacIdxstate) -> c_int;

unsafe fn predict_sample(psamp: *mut c_int, idx: c_int, pfcn: c_int, ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    let s = |off: usize| -> i32 { unsafe { *psamp.add((i.wrapping_sub(off)) & 7) } };
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
        12..=15 => unsafe {
            let fx = &(*ridx).firfx[(pfcn - 12) as usize];
            (fx[0] as i32 * s(1) + fx[1] as i32 * s(2) + fx[2] as i32 * s(3) + fx[3] as i32 * s(4)
                + fx[4] as i32 * s(5) + fx[5] as i32 * s(6) + fx[6] as i32 * s(7) + fx[7] as i32 * s(8)) / 256
        },
        _ => 0,
    }
}

unsafe fn predict_pfn0(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    unsafe { *psamp.add((idx as usize).wrapping_sub(1) & 7) }
}

unsafe fn predict_pfn1(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe { 2 * *psamp.add(i.wrapping_sub(1) & 7) - *psamp.add(i.wrapping_sub(2) & 7) }
}

unsafe fn predict_pfn2(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe { (3 * *psamp.add(i.wrapping_sub(1) & 7) - *psamp.add(i.wrapping_sub(2) & 7)) >> 1 }
}

unsafe fn predict_pfn3(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe { (5 * *psamp.add(i.wrapping_sub(1) & 7) - *psamp.add(i.wrapping_sub(2) & 7)) >> 2 }
}

unsafe fn predict_pfn4(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe {
        let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7);
        let p1 = *psamp.add(i.wrapping_sub(2) & 7) + *psamp.add(i.wrapping_sub(3) & 7);
        p0 - (p1 >> 1)
    }
}

unsafe fn predict_pfn5(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe {
        let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7);
        let p1 = *psamp.add(i.wrapping_sub(2) & 7) + *psamp.add(i.wrapping_sub(3) & 7);
        (3 * p0 - p1) >> 2
    }
}

unsafe fn predict_pfn6(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe {
        let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7);
        let p1 = *psamp.add(i.wrapping_sub(2) & 7) + *psamp.add(i.wrapping_sub(3) & 7);
        (5 * p0 - p1) >> 3
    }
}

unsafe fn predict_pfn7(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe {
        (18 * *psamp.add(i.wrapping_sub(1) & 7) - 4 * *psamp.add(i.wrapping_sub(2) & 7)
            + 3 * *psamp.add(i.wrapping_sub(3) & 7) - 2 * *psamp.add(i.wrapping_sub(4) & 7)
            + *psamp.add(i.wrapping_sub(5) & 7)) / 16
    }
}

unsafe fn predict_pfn8(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe {
        (72 * *psamp.add(i.wrapping_sub(1) & 7) - 16 * *psamp.add(i.wrapping_sub(2) & 7)
            + 12 * *psamp.add(i.wrapping_sub(3) & 7) - 8 * *psamp.add(i.wrapping_sub(4) & 7)
            + 5 * *psamp.add(i.wrapping_sub(5) & 7) - 3 * *psamp.add(i.wrapping_sub(6) & 7)
            + 3 * *psamp.add(i.wrapping_sub(7) & 7) - *psamp.add(i.wrapping_sub(8) & 7)) / 64
    }
}

unsafe fn predict_pfn9(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe {
        (76 * *psamp.add(i.wrapping_sub(1) & 7) - 17 * *psamp.add(i.wrapping_sub(2) & 7)
            + 10 * *psamp.add(i.wrapping_sub(3) & 7) - 7 * *psamp.add(i.wrapping_sub(4) & 7)
            + 5 * *psamp.add(i.wrapping_sub(5) & 7) - 4 * *psamp.add(i.wrapping_sub(6) & 7)
            + 4 * *psamp.add(i.wrapping_sub(7) & 7) - 3 * *psamp.add(i.wrapping_sub(8) & 7)) / 64
    }
}

// NOTE: C code has >> 3 here (bug vs case 10 in PredictSample which uses >> 4). Preserving exactly.
unsafe fn predict_pfn10(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe {
        let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7)
            + *psamp.add(i.wrapping_sub(3) & 7) + *psamp.add(i.wrapping_sub(4) & 7);
        let p1 = *psamp.add(i.wrapping_sub(5) & 7) + *psamp.add(i.wrapping_sub(6) & 7)
            + *psamp.add(i.wrapping_sub(7) & 7) + *psamp.add(i.wrapping_sub(8) & 7);
        (5 * p0 - p1) >> 3
    }
}

// NOTE: C code has >> 1 here (bug vs case 11 in PredictSample which uses >> 3). Preserving exactly.
unsafe fn predict_pfn11(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut BtacIdxstate) -> c_int {
    let i = idx as usize;
    unsafe {
        let p0 = *psamp.add(i.wrapping_sub(1) & 7) + *psamp.add(i.wrapping_sub(2) & 7)
            + *psamp.add(i.wrapping_sub(3) & 7) + *psamp.add(i.wrapping_sub(4) & 7);
        let p1 = *psamp.add(i.wrapping_sub(5) & 7) + *psamp.add(i.wrapping_sub(6) & 7)
            + *psamp.add(i.wrapping_sub(7) & 7) + *psamp.add(i.wrapping_sub(8) & 7);
        (p0 + p1) >> 1
    }
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
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    let fcn = get_predict_func_inner(pfcn);
    let fp = fcn as *const () as usize;
    match pfcn {
        0 => (fp == predict_pfn0 as *const () as usize) as c_int,
        1 => (fp == predict_pfn1 as *const () as usize) as c_int,
        2 => (fp == predict_pfn2 as *const () as usize) as c_int,
        3 => (fp == predict_pfn3 as *const () as usize) as c_int,
        4 => (fp == predict_pfn4 as *const () as usize) as c_int,
        5 => (fp == predict_pfn5 as *const () as usize) as c_int,
        6 => (fp == predict_pfn6 as *const () as usize) as c_int,
        7 => (fp == predict_pfn7 as *const () as usize) as c_int,
        8 => (fp == predict_pfn8 as *const () as usize) as c_int,
        9 => (fp == predict_pfn9 as *const () as usize) as c_int,
        10 => (fp == predict_pfn10 as *const () as usize) as c_int,
        11 => (fp == predict_pfn11 as *const () as usize) as c_int,
        _ => 0,
    }
}
