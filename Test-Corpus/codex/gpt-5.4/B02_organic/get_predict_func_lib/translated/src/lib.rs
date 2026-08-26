use std::ffi::c_int;

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

type PredictFn = fn(*mut c_int, c_int, c_int, *mut Btac1cIdxState) -> c_int;

#[inline]
unsafe fn psamp_at(psamp: *mut c_int, idx: c_int) -> c_int {
    *psamp.add((idx & 7) as usize)
}

fn btac1c2_predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut Btac1cIdxState,
) -> c_int {
    let i = idx;
    match pfcn {
        0 => unsafe { psamp_at(psamp, i.wrapping_sub(1)) },
        1 => unsafe {
            psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_mul(2)
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)))
        },
        2 => unsafe {
            psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_mul(3)
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)))
                >> 1
        },
        3 => unsafe {
            psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_mul(5)
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)))
                >> 2
        },
        4 => unsafe {
            let p0 = psamp_at(psamp, i.wrapping_sub(1)).wrapping_add(psamp_at(psamp, i.wrapping_sub(2)));
            let p1 = psamp_at(psamp, i.wrapping_sub(2)).wrapping_add(psamp_at(psamp, i.wrapping_sub(3)));
            p0.wrapping_sub(p1 >> 1)
        },
        5 => unsafe {
            let p0 = psamp_at(psamp, i.wrapping_sub(1)).wrapping_add(psamp_at(psamp, i.wrapping_sub(2)));
            let p1 = psamp_at(psamp, i.wrapping_sub(2)).wrapping_add(psamp_at(psamp, i.wrapping_sub(3)));
            p0.wrapping_mul(3).wrapping_sub(p1) >> 2
        },
        6 => unsafe {
            let p0 = psamp_at(psamp, i.wrapping_sub(1)).wrapping_add(psamp_at(psamp, i.wrapping_sub(2)));
            let p1 = psamp_at(psamp, i.wrapping_sub(2)).wrapping_add(psamp_at(psamp, i.wrapping_sub(3)));
            p0.wrapping_mul(5).wrapping_sub(p1) >> 3
        },
        7 => unsafe {
            psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_mul(18)
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)).wrapping_mul(4))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)).wrapping_mul(3))
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(4)).wrapping_mul(2))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(5)))
                / 16
        },
        8 => unsafe {
            psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_mul(72)
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)).wrapping_mul(16))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)).wrapping_mul(12))
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(4)).wrapping_mul(8))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(5)).wrapping_mul(5))
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(6)).wrapping_mul(3))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(7)).wrapping_mul(3))
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(8)))
                / 64
        },
        9 => unsafe {
            psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_mul(76)
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(2)).wrapping_mul(17))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)).wrapping_mul(10))
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(4)).wrapping_mul(7))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(5)).wrapping_mul(5))
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(6)).wrapping_mul(4))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(7)).wrapping_mul(4))
                .wrapping_sub(psamp_at(psamp, i.wrapping_sub(8)).wrapping_mul(3))
                / 64
        },
        10 => unsafe {
            let p0 = psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(2)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(4)));
            let p1 = psamp_at(psamp, i.wrapping_sub(5))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(6)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(7)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(8)));
            p0.wrapping_mul(5).wrapping_sub(p1) >> 4
        },
        11 => unsafe {
            let p0 = psamp_at(psamp, i.wrapping_sub(1))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(2)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(3)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(4)));
            let p1 = psamp_at(psamp, i.wrapping_sub(5))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(6)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(7)))
                .wrapping_add(psamp_at(psamp, i.wrapping_sub(8)));
            p0.wrapping_add(p1) >> 3
        },
        12..=15 => unsafe {
            let ridx = &*ridx;
            let fir = &ridx.firfx[(pfcn - 12) as usize];
            (c_int::from(fir[0]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(1)))
                .wrapping_add(c_int::from(fir[1]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(2))))
                .wrapping_add(c_int::from(fir[2]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(3))))
                .wrapping_add(c_int::from(fir[3]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(4))))
                .wrapping_add(c_int::from(fir[4]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(5))))
                .wrapping_add(c_int::from(fir[5]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(6))))
                .wrapping_add(c_int::from(fir[6]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(7))))
                .wrapping_add(c_int::from(fir[7]).wrapping_mul(psamp_at(psamp, i.wrapping_sub(8)))))
                / 256
        },
        _ => 0,
    }
}

fn btac1c2_predict_sample_pfn0(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe { psamp_at(psamp, idx.wrapping_sub(1)) }
}

fn btac1c2_predict_sample_pfn1(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        psamp_at(psamp, idx.wrapping_sub(1))
            .wrapping_mul(2)
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)))
    }
}

fn btac1c2_predict_sample_pfn2(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        psamp_at(psamp, idx.wrapping_sub(1))
            .wrapping_mul(3)
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)))
            >> 1
    }
}

fn btac1c2_predict_sample_pfn3(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        psamp_at(psamp, idx.wrapping_sub(1))
            .wrapping_mul(5)
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)))
            >> 2
    }
}

fn btac1c2_predict_sample_pfn4(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        let p0 = psamp_at(psamp, idx.wrapping_sub(1)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)));
        let p1 = psamp_at(psamp, idx.wrapping_sub(2)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)));
        p0.wrapping_sub(p1 >> 1)
    }
}

fn btac1c2_predict_sample_pfn5(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        let p0 = psamp_at(psamp, idx.wrapping_sub(1)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)));
        let p1 = psamp_at(psamp, idx.wrapping_sub(2)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)));
        p0.wrapping_mul(3).wrapping_sub(p1) >> 2
    }
}

fn btac1c2_predict_sample_pfn6(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        let p0 = psamp_at(psamp, idx.wrapping_sub(1)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)));
        let p1 = psamp_at(psamp, idx.wrapping_sub(2)).wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)));
        p0.wrapping_mul(5).wrapping_sub(p1) >> 3
    }
}

fn btac1c2_predict_sample_pfn7(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        psamp_at(psamp, idx.wrapping_sub(1))
            .wrapping_mul(18)
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)).wrapping_mul(4))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)).wrapping_mul(3))
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(4)).wrapping_mul(2))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(5)))
            / 16
    }
}

fn btac1c2_predict_sample_pfn8(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        psamp_at(psamp, idx.wrapping_sub(1))
            .wrapping_mul(72)
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)).wrapping_mul(16))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)).wrapping_mul(12))
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(4)).wrapping_mul(8))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(5)).wrapping_mul(5))
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(6)).wrapping_mul(3))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(7)).wrapping_mul(3))
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(8)))
            / 64
    }
}

fn btac1c2_predict_sample_pfn9(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        psamp_at(psamp, idx.wrapping_sub(1))
            .wrapping_mul(76)
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(2)).wrapping_mul(17))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)).wrapping_mul(10))
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(4)).wrapping_mul(7))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(5)).wrapping_mul(5))
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(6)).wrapping_mul(4))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(7)).wrapping_mul(4))
            .wrapping_sub(psamp_at(psamp, idx.wrapping_sub(8)).wrapping_mul(3))
            / 64
    }
}

fn btac1c2_predict_sample_pfn10(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        let p0 = psamp_at(psamp, idx.wrapping_sub(1))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(4)));
        let p1 = psamp_at(psamp, idx.wrapping_sub(5))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(6)))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(7)))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(8)));
        p0.wrapping_mul(5).wrapping_sub(p1) >> 3
    }
}

fn btac1c2_predict_sample_pfn11(
    psamp: *mut c_int,
    idx: c_int,
    _pfcn: c_int,
    _ridx: *mut Btac1cIdxState,
) -> c_int {
    unsafe {
        let p0 = psamp_at(psamp, idx.wrapping_sub(1))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(2)))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(3)))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(4)));
        let p1 = psamp_at(psamp, idx.wrapping_sub(5))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(6)))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(7)))
            .wrapping_add(psamp_at(psamp, idx.wrapping_sub(8)));
        p0.wrapping_add(p1) >> 1
    }
}

fn btac1c2_get_predict_func(pfcn: c_int) -> PredictFn {
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
    match pfcn {
        0 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn0 as PredictFn) as c_int,
        1 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn1 as PredictFn) as c_int,
        2 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn2 as PredictFn) as c_int,
        3 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn3 as PredictFn) as c_int,
        4 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn4 as PredictFn) as c_int,
        5 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn5 as PredictFn) as c_int,
        6 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn6 as PredictFn) as c_int,
        7 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn7 as PredictFn) as c_int,
        8 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn8 as PredictFn) as c_int,
        9 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn9 as PredictFn) as c_int,
        10 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn10 as PredictFn) as c_int,
        11 => std::ptr::fn_addr_eq(fcn, btac1c2_predict_sample_pfn11 as PredictFn) as c_int,
        _ => 0,
    }
}
