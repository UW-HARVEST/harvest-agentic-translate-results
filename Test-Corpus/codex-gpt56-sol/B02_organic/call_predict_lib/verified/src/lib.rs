use std::ffi::{c_int, c_uchar, c_ushort};

#[repr(C)]
struct Btac1cIdxState {
    idx: c_ushort,
    lpred: i16,
    rpred: i16,
    tag: c_uchar,
    bcfcn: c_uchar,
    bsfcn: c_uchar,
    usefx: c_uchar,
    firfx: [[i16; 8]; 4],
}

type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut Btac1cIdxState) -> c_int;

#[inline]
unsafe fn sample(psamp: *mut c_int, idx: c_int, offset: c_int) -> c_int {
    let slot = idx.wrapping_sub(offset) & 7;
    unsafe { psamp.add(slot as usize).read() }
}

unsafe extern "C" fn predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut Btac1cIdxState,
) -> c_int {
    let s = |offset| unsafe { sample(psamp, idx, offset) };

    match pfcn {
        0 => s(1),
        1 => s(1).wrapping_mul(2).wrapping_sub(s(2)),
        2 => s(1).wrapping_mul(3).wrapping_sub(s(2)) >> 1,
        3 => s(1).wrapping_mul(5).wrapping_sub(s(2)) >> 2,
        4 => {
            let p0 = s(1).wrapping_add(s(2));
            let p1 = s(2).wrapping_add(s(3));
            p0.wrapping_sub(p1 >> 1)
        }
        5 => {
            let p0 = s(1).wrapping_add(s(2));
            let p1 = s(2).wrapping_add(s(3));
            p0.wrapping_mul(3).wrapping_sub(p1) >> 2
        }
        6 => {
            let p0 = s(1).wrapping_add(s(2));
            let p1 = s(2).wrapping_add(s(3));
            p0.wrapping_mul(5).wrapping_sub(p1) >> 3
        }
        7 => {
            s(1).wrapping_mul(18)
                .wrapping_sub(s(2).wrapping_mul(4))
                .wrapping_add(s(3).wrapping_mul(3))
                .wrapping_sub(s(4).wrapping_mul(2))
                .wrapping_add(s(5))
                / 16
        }
        8 => {
            s(1).wrapping_mul(72)
                .wrapping_sub(s(2).wrapping_mul(16))
                .wrapping_add(s(3).wrapping_mul(12))
                .wrapping_sub(s(4).wrapping_mul(8))
                .wrapping_add(s(5).wrapping_mul(5))
                .wrapping_sub(s(6).wrapping_mul(3))
                .wrapping_add(s(7).wrapping_mul(3))
                .wrapping_sub(s(8))
                / 64
        }
        9 => {
            s(1).wrapping_mul(76)
                .wrapping_sub(s(2).wrapping_mul(17))
                .wrapping_add(s(3).wrapping_mul(10))
                .wrapping_sub(s(4).wrapping_mul(7))
                .wrapping_add(s(5).wrapping_mul(5))
                .wrapping_sub(s(6).wrapping_mul(4))
                .wrapping_add(s(7).wrapping_mul(4))
                .wrapping_sub(s(8).wrapping_mul(3))
                / 64
        }
        10 => {
            let p0 = s(1)
                .wrapping_add(s(2))
                .wrapping_add(s(3))
                .wrapping_add(s(4));
            let p1 = s(5)
                .wrapping_add(s(6))
                .wrapping_add(s(7))
                .wrapping_add(s(8));
            p0.wrapping_mul(5).wrapping_sub(p1) >> 4
        }
        11 => {
            let p0 = s(1)
                .wrapping_add(s(2))
                .wrapping_add(s(3))
                .wrapping_add(s(4));
            let p1 = s(5)
                .wrapping_add(s(6))
                .wrapping_add(s(7))
                .wrapping_add(s(8));
            p0.wrapping_add(p1) >> 3
        }
        12..=15 => {
            let row = (pfcn - 12) as usize;
            let mut pred = 0_i32;
            for offset in 1..=8 {
                let coefficient = unsafe { (*ridx).firfx[row][offset - 1] } as c_int;
                pred = pred.wrapping_add(coefficient.wrapping_mul(s(offset as c_int)));
            }
            pred / 256
        }
        _ => 0,
    }
}

unsafe extern "C" fn predict_sample_pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe { sample(psamp, idx, 1) }
}

unsafe extern "C" fn predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        sample(psamp, idx, 1)
            .wrapping_mul(2)
            .wrapping_sub(sample(psamp, idx, 2))
    }
}

unsafe extern "C" fn predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        sample(psamp, idx, 1)
            .wrapping_mul(3)
            .wrapping_sub(sample(psamp, idx, 2))
            >> 1
    }
}

unsafe extern "C" fn predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        sample(psamp, idx, 1)
            .wrapping_mul(5)
            .wrapping_sub(sample(psamp, idx, 2))
            >> 2
    }
}

unsafe extern "C" fn predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let p0 = unsafe { sample(psamp, idx, 1).wrapping_add(sample(psamp, idx, 2)) };
    let p1 = unsafe { sample(psamp, idx, 2).wrapping_add(sample(psamp, idx, 3)) };
    p0.wrapping_sub(p1 >> 1)
}

unsafe extern "C" fn predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let p0 = unsafe { sample(psamp, idx, 1).wrapping_add(sample(psamp, idx, 2)) };
    let p1 = unsafe { sample(psamp, idx, 2).wrapping_add(sample(psamp, idx, 3)) };
    p0.wrapping_mul(3).wrapping_sub(p1) >> 2
}

unsafe extern "C" fn predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let p0 = unsafe { sample(psamp, idx, 1).wrapping_add(sample(psamp, idx, 2)) };
    let p1 = unsafe { sample(psamp, idx, 2).wrapping_add(sample(psamp, idx, 3)) };
    p0.wrapping_mul(5).wrapping_sub(p1) >> 3
}

unsafe extern "C" fn predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let s = |offset| unsafe { sample(psamp, idx, offset) };
    s(1).wrapping_mul(18)
        .wrapping_sub(s(2).wrapping_mul(4))
        .wrapping_add(s(3).wrapping_mul(3))
        .wrapping_sub(s(4).wrapping_mul(2))
        .wrapping_add(s(5))
        / 16
}

unsafe extern "C" fn predict_sample_pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let s = |offset| unsafe { sample(psamp, idx, offset) };
    s(1).wrapping_mul(72)
        .wrapping_sub(s(2).wrapping_mul(16))
        .wrapping_add(s(3).wrapping_mul(12))
        .wrapping_sub(s(4).wrapping_mul(8))
        .wrapping_add(s(5).wrapping_mul(5))
        .wrapping_sub(s(6).wrapping_mul(3))
        .wrapping_add(s(7).wrapping_mul(3))
        .wrapping_sub(s(8))
        / 64
}

unsafe extern "C" fn predict_sample_pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let s = |offset| unsafe { sample(psamp, idx, offset) };
    s(1).wrapping_mul(76)
        .wrapping_sub(s(2).wrapping_mul(17))
        .wrapping_add(s(3).wrapping_mul(10))
        .wrapping_sub(s(4).wrapping_mul(7))
        .wrapping_add(s(5).wrapping_mul(5))
        .wrapping_sub(s(6).wrapping_mul(4))
        .wrapping_add(s(7).wrapping_mul(4))
        .wrapping_sub(s(8).wrapping_mul(3))
        / 64
}

unsafe extern "C" fn predict_sample_pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let s = |offset| unsafe { sample(psamp, idx, offset) };
    let p0 = s(1)
        .wrapping_add(s(2))
        .wrapping_add(s(3))
        .wrapping_add(s(4));
    let p1 = s(5)
        .wrapping_add(s(6))
        .wrapping_add(s(7))
        .wrapping_add(s(8));
    p0.wrapping_mul(5).wrapping_sub(p1) >> 3
}

unsafe extern "C" fn predict_sample_pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let s = |offset| unsafe { sample(psamp, idx, offset) };
    let p0 = s(1)
        .wrapping_add(s(2))
        .wrapping_add(s(3))
        .wrapping_add(s(4));
    let p1 = s(5)
        .wrapping_add(s(6))
        .wrapping_add(s(7))
        .wrapping_add(s(8));
    p0.wrapping_add(p1) >> 1
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
    let function = get_predict_func(pfcn);
    let expected = match pfcn {
        0 => predict_sample_pfn0 as PredictFn,
        1 => predict_sample_pfn1 as PredictFn,
        2 => predict_sample_pfn2 as PredictFn,
        3 => predict_sample_pfn3 as PredictFn,
        4 => predict_sample_pfn4 as PredictFn,
        5 => predict_sample_pfn5 as PredictFn,
        6 => predict_sample_pfn6 as PredictFn,
        7 => predict_sample_pfn7 as PredictFn,
        8 => predict_sample_pfn8 as PredictFn,
        9 => predict_sample_pfn9 as PredictFn,
        10 => predict_sample_pfn10 as PredictFn,
        11 => predict_sample_pfn11 as PredictFn,
        _ => return 0,
    };

    c_int::from(std::ptr::fn_addr_eq(function, expected))
}
