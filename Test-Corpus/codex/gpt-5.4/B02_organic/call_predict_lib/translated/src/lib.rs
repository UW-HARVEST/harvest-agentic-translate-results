use std::ffi::{c_int, c_void};

#[repr(C)]
struct Btac1cIdxState {
    idx: u16,
    lpred: i16,
    rpred: i16,
    tag: u8,
    bcfcn: u8,
    bsfcn: u8,
    usefx: u8,
    firfx: [[i16; 8]; 4],
}

type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut Btac1cIdxState) -> c_int;

#[inline]
unsafe fn psamp_at(psamp: *mut c_int, idx: c_int) -> c_int {
    *psamp.add((idx & 7) as usize)
}

unsafe extern "C" fn btac1c2_predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut Btac1cIdxState,
) -> c_int {
    let i = idx;
    match pfcn {
        0 => psamp_at(psamp, i.wrapping_sub(1)),
        1 => psamp_at(psamp, i.wrapping_sub(1))
            .wrapping_mul(2)
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2))),
        2 => (psamp_at(psamp, i.wrapping_sub(1))
            .wrapping_mul(3)
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2))))
            >> 1,
        3 => (psamp_at(psamp, i.wrapping_sub(1))
            .wrapping_mul(5)
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2))))
            >> 2,
        4 => {
            let p0 = psamp_at(psamp, i.wrapping_sub(1)).wrapping_add(psamp_at(psamp, i.wrapping_sub(2)));
            let p1 = psamp_at(psamp, i.wrapping_sub(2)).wrapping_add(psamp_at(psamp, i.wrapping_sub(3)));
            p0.wrapping_sub(p1 >> 1)
        }
        5 => {
            let p0 = psamp_at(psamp, i.wrapping_sub(1)).wrapping_add(psamp_at(psamp, i.wrapping_sub(2)));
            let p1 = psamp_at(psamp, i.wrapping_sub(2)).wrapping_add(psamp_at(psamp, i.wrapping_sub(3)));
            (p0.wrapping_mul(3).wrapping_sub(p1)) >> 2
        }
        6 => {
            let p0 = psamp_at(psamp, i.wrapping_sub(1)).wrapping_add(psamp_at(psamp, i.wrapping_sub(2)));
            let p1 = psamp_at(psamp, i.wrapping_sub(2)).wrapping_add(psamp_at(psamp, i.wrapping_sub(3)));
            (p0.wrapping_mul(5).wrapping_sub(p1)) >> 3
        }
        7 => (psamp_at(psamp, i.wrapping_sub(1))
            .wrapping_mul(18)
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)).wrapping_mul(4))
            .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)).wrapping_mul(3))
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(4)).wrapping_mul(2))
            .wrapping_add(psamp_at(psamp, i.wrapping_sub(5))))
            / 16,
        8 => (psamp_at(psamp, i.wrapping_sub(1))
            .wrapping_mul(72)
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)).wrapping_mul(16))
            .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)).wrapping_mul(12))
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(4)).wrapping_mul(8))
            .wrapping_add(psamp_at(psamp, i.wrapping_sub(5)).wrapping_mul(5))
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(6)).wrapping_mul(3))
            .wrapping_add(psamp_at(psamp, i.wrapping_sub(7)).wrapping_mul(3))
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(8))))
            / 64,
        9 => (psamp_at(psamp, i.wrapping_sub(1))
            .wrapping_mul(76)
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)).wrapping_mul(17))
            .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)).wrapping_mul(10))
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(4)).wrapping_mul(7))
            .wrapping_add(psamp_at(psamp, i.wrapping_sub(5)).wrapping_mul(5))
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(6)).wrapping_mul(4))
            .wrapping_add(psamp_at(psamp, i.wrapping_sub(7)).wrapping_mul(4))
            .wrapping_sub(psamp_at(psamp, i.wrapping_sub(8)).wrapping_mul(3)))
            / 64,
        10 => {
            let p0 = psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(2)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(4)));
            let p1 = psamp_at(psamp, i.wrapping_sub(5))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(6)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(7)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(8)));
            (p0.wrapping_mul(5).wrapping_sub(p1)) >> 4
        }
        11 => {
            let p0 = psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(2)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(4)));
            let p1 = psamp_at(psamp, i.wrapping_sub(5))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(6)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(7)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(8)));
            p0.wrapping_add(p1) >> 3
        }
        12..=15 => {
            let ridx = &*ridx;
            (c_int::from(ridx.firfx[(pfcn - 12) as usize][0])
                .wrapping_mul(psamp_at(psamp, i.wrapping_sub(1)))
                .wrapping_add(c_int::from(ridx.firfx[(pfcn - 12) as usize][1]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(2))))
                .wrapping_add(c_int::from(ridx.firfx[(pfcn - 12) as usize][2]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(3))))
                .wrapping_add(c_int::from(ridx.firfx[(pfcn - 12) as usize][3]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(4))))
                .wrapping_add(c_int::from(ridx.firfx[(pfcn - 12) as usize][4]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(5))))
                .wrapping_add(c_int::from(ridx.firfx[(pfcn - 12) as usize][5]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(6))))
                .wrapping_add(c_int::from(ridx.firfx[(pfcn - 12) as usize][6]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(7))))
                .wrapping_add(c_int::from(ridx.firfx[(pfcn - 12) as usize][7]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(8)))))
                / 256
        }
        _ => 0,
    }
}

unsafe extern "C" fn btac1c2_predict_sample_pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    psamp_at(psamp, idx.wrapping_sub(1))
}

unsafe extern "C" fn btac1c2_predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    psamp_at(psamp, idx.wrapping_sub(1))
        .wrapping_mul(2)
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)))
}

unsafe extern "C" fn btac1c2_predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    (psamp_at(psamp, idx.wrapping_sub(1))
        .wrapping_mul(3)
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2))))
        >> 1
}

unsafe extern "C" fn btac1c2_predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    (psamp_at(psamp, idx.wrapping_sub(1))
        .wrapping_mul(5)
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2))))
        >> 2
}

unsafe extern "C" fn btac1c2_predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let p0 = psamp_at(psamp, idx.wrapping_sub(1)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)));
    let p1 = psamp_at(psamp, idx.wrapping_sub(2)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)));
    p0.wrapping_sub(p1 >> 1)
}

unsafe extern "C" fn btac1c2_predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let p0 = psamp_at(psamp, idx.wrapping_sub(1)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)));
    let p1 = psamp_at(psamp, idx.wrapping_sub(2)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)));
    (p0.wrapping_mul(3).wrapping_sub(p1)) >> 2
}

unsafe extern "C" fn btac1c2_predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let p0 = psamp_at(psamp, idx.wrapping_sub(1)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)));
    let p1 = psamp_at(psamp, idx.wrapping_sub(2)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)));
    (p0.wrapping_mul(5).wrapping_sub(p1)) >> 3
}

unsafe extern "C" fn btac1c2_predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    (psamp_at(psamp, idx.wrapping_sub(1))
        .wrapping_mul(18)
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)).wrapping_mul(4))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)).wrapping_mul(3))
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(4)).wrapping_mul(2))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(5))))
        / 16
}

unsafe extern "C" fn btac1c2_predict_sample_pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    (psamp_at(psamp, idx.wrapping_sub(1))
        .wrapping_mul(72)
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)).wrapping_mul(16))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)).wrapping_mul(12))
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(4)).wrapping_mul(8))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(5)).wrapping_mul(5))
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(6)).wrapping_mul(3))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(7)).wrapping_mul(3))
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(8))))
        / 64
}

unsafe extern "C" fn btac1c2_predict_sample_pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    (psamp_at(psamp, idx.wrapping_sub(1))
        .wrapping_mul(76)
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)).wrapping_mul(17))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)).wrapping_mul(10))
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(4)).wrapping_mul(7))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(5)).wrapping_mul(5))
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(6)).wrapping_mul(4))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(7)).wrapping_mul(4))
        .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(8)).wrapping_mul(3)))
        / 64
}

unsafe extern "C" fn btac1c2_predict_sample_pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let p0 = psamp_at(psamp, idx.wrapping_sub(1))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(4)));
    let p1 = psamp_at(psamp, idx.wrapping_sub(5))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(6)))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(7)))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(8)));
    (p0.wrapping_mul(5).wrapping_sub(p1)) >> 3
}

unsafe extern "C" fn btac1c2_predict_sample_pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    let p0 = psamp_at(psamp, idx.wrapping_sub(1))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(4)));
    let p1 = psamp_at(psamp, idx.wrapping_sub(5))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(6)))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(7)))
        .wrapping_add(psamp_at(psamp, idx.wrapping_sub(8)));
    (p0.wrapping_add(p1)) >> 1
}

fn btac1c2_get_predict_func(pfcn: c_int) -> *const c_void {
    let fcn: PredictFn = match pfcn {
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
    };
    fcn as *const c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let fcn = btac1c2_get_predict_func(pfcn);
    match pfcn {
        0 => (fcn == btac1c2_predict_sample_pfn0 as *const c_void) as c_int,
        1 => (fcn == btac1c2_predict_sample_pfn1 as *const c_void) as c_int,
        2 => (fcn == btac1c2_predict_sample_pfn2 as *const c_void) as c_int,
        3 => (fcn == btac1c2_predict_sample_pfn3 as *const c_void) as c_int,
        4 => (fcn == btac1c2_predict_sample_pfn4 as *const c_void) as c_int,
        5 => (fcn == btac1c2_predict_sample_pfn5 as *const c_void) as c_int,
        6 => (fcn == btac1c2_predict_sample_pfn6 as *const c_void) as c_int,
        7 => (fcn == btac1c2_predict_sample_pfn7 as *const c_void) as c_int,
        8 => (fcn == btac1c2_predict_sample_pfn8 as *const c_void) as c_int,
        9 => (fcn == btac1c2_predict_sample_pfn9 as *const c_void) as c_int,
        10 => (fcn == btac1c2_predict_sample_pfn10 as *const c_void) as c_int,
        11 => (fcn == btac1c2_predict_sample_pfn11 as *const c_void) as c_int,
        _ => 0,
    }
}
