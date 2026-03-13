use std::os::raw::c_int;

type Btac1cU16 = u16;
type Btac1cS16 = i16;
type Btac1cByte = u8;

#[repr(C)]
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

type PredictFn = fn(*mut c_int, c_int, c_int, *mut Btac1cIdxstate) -> c_int;

fn predict_sample(psamp: *mut c_int, idx: c_int, pfcn: c_int, ridx: *mut Btac1cIdxstate) -> c_int {
    let i = idx;
    unsafe {
        let s = |off: c_int| -> c_int { *psamp.offset(((i - off) & 7) as isize) };
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
            12..=15 => {
                let r = &*ridx;
                let f = r.firfx[(pfcn - 12) as usize];
                (f[0] as c_int * s(1) + f[1] as c_int * s(2) + f[2] as c_int * s(3) + f[3] as c_int * s(4)
                    + f[4] as c_int * s(5) + f[5] as c_int * s(6) + f[6] as c_int * s(7) + f[7] as c_int * s(8))
                    / 256
            }
            _ => 0,
        }
    }
}

fn predict_pfn0(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe { *psamp.offset(((idx - 1) & 7) as isize) }
}

fn predict_pfn1(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe { 2 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize) }
}

fn predict_pfn2(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe { (3 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)) >> 1 }
}

fn predict_pfn3(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe { (5 * *psamp.offset(((idx - 1) & 7) as isize) - *psamp.offset(((idx - 2) & 7) as isize)) >> 2 }
}

fn predict_pfn4(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
        p0 - (p1 >> 1)
    }
}

fn predict_pfn5(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
        (3 * p0 - p1) >> 2
    }
}

fn predict_pfn6(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize);
        let p1 = *psamp.offset(((idx - 2) & 7) as isize) + *psamp.offset(((idx - 3) & 7) as isize);
        (5 * p0 - p1) >> 3
    }
}

fn predict_pfn7(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe {
        (18 * *psamp.offset(((idx - 1) & 7) as isize) - 4 * *psamp.offset(((idx - 2) & 7) as isize)
            + 3 * *psamp.offset(((idx - 3) & 7) as isize) - 2 * *psamp.offset(((idx - 4) & 7) as isize)
            + 1 * *psamp.offset(((idx - 5) & 7) as isize))
            / 16
    }
}

fn predict_pfn8(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe {
        (72 * *psamp.offset(((idx - 1) & 7) as isize) - 16 * *psamp.offset(((idx - 2) & 7) as isize)
            + 12 * *psamp.offset(((idx - 3) & 7) as isize) - 8 * *psamp.offset(((idx - 4) & 7) as isize)
            + 5 * *psamp.offset(((idx - 5) & 7) as isize) - 3 * *psamp.offset(((idx - 6) & 7) as isize)
            + 3 * *psamp.offset(((idx - 7) & 7) as isize) - 1 * *psamp.offset(((idx - 8) & 7) as isize))
            / 64
    }
}

fn predict_pfn9(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe {
        (76 * *psamp.offset(((idx - 1) & 7) as isize) - 17 * *psamp.offset(((idx - 2) & 7) as isize)
            + 10 * *psamp.offset(((idx - 3) & 7) as isize) - 7 * *psamp.offset(((idx - 4) & 7) as isize)
            + 5 * *psamp.offset(((idx - 5) & 7) as isize) - 4 * *psamp.offset(((idx - 6) & 7) as isize)
            + 4 * *psamp.offset(((idx - 7) & 7) as isize) - 3 * *psamp.offset(((idx - 8) & 7) as isize))
            / 64
    }
}

// NOTE: C original uses >> 3 here (differs from switch-case version which uses >> 4)
fn predict_pfn10(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize)
            + *psamp.offset(((idx - 3) & 7) as isize) + *psamp.offset(((idx - 4) & 7) as isize);
        let p1 = *psamp.offset(((idx - 5) & 7) as isize) + *psamp.offset(((idx - 6) & 7) as isize)
            + *psamp.offset(((idx - 7) & 7) as isize) + *psamp.offset(((idx - 8) & 7) as isize);
        (5 * p0 - p1) >> 3
    }
}

// NOTE: C original uses >> 1 here (differs from switch-case version which uses >> 3)
fn predict_pfn11(psamp: *mut c_int, idx: c_int, _pfcn: c_int, _ridx: *mut Btac1cIdxstate) -> c_int {
    unsafe {
        let p0 = *psamp.offset(((idx - 1) & 7) as isize) + *psamp.offset(((idx - 2) & 7) as isize)
            + *psamp.offset(((idx - 3) & 7) as isize) + *psamp.offset(((idx - 4) & 7) as isize);
        let p1 = *psamp.offset(((idx - 5) & 7) as isize) + *psamp.offset(((idx - 6) & 7) as isize)
            + *psamp.offset(((idx - 7) & 7) as isize) + *psamp.offset(((idx - 8) & 7) as isize);
        (p0 + p1) >> 1
    }
}

fn get_predict_func(pfcn: c_int) -> PredictFn {
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

#[inline]
fn fn_addr(f: PredictFn) -> usize {
    f as *const () as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    let fcn = fn_addr(get_predict_func(pfcn));
    match pfcn {
        0 => (fcn == fn_addr(predict_pfn0)) as c_int,
        1 => (fcn == fn_addr(predict_pfn1)) as c_int,
        2 => (fcn == fn_addr(predict_pfn2)) as c_int,
        3 => (fcn == fn_addr(predict_pfn3)) as c_int,
        4 => (fcn == fn_addr(predict_pfn4)) as c_int,
        5 => (fcn == fn_addr(predict_pfn5)) as c_int,
        6 => (fcn == fn_addr(predict_pfn6)) as c_int,
        7 => (fcn == fn_addr(predict_pfn7)) as c_int,
        8 => (fcn == fn_addr(predict_pfn8)) as c_int,
        9 => (fcn == fn_addr(predict_pfn9)) as c_int,
        10 => (fcn == fn_addr(predict_pfn10)) as c_int,
        11 => (fcn == fn_addr(predict_pfn11)) as c_int,
        _ => 0,
    }
}
