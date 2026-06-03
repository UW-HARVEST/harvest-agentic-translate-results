// Translation of c_src/src/lib.c to Rust.
//
// The C code defines a set of sample-prediction functions used by an audio
// codec, plus dispatch helpers that select a function pointer for a given
// `pfcn` and a `call_predict` helper that returns 1 when the dispatcher
// returned the function we expect for that `pfcn` and 0 otherwise.
//
// This translation preserves the same behavior using safe Rust where possible.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub type Btac1cU16 = u16;
pub type Btac1cS16 = i16;
pub type Btac1cByte = u8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Btac1cIdxstate {
    pub idx: Btac1cU16,
    pub lpred: Btac1cS16,
    pub rpred: Btac1cS16,
    pub tag: Btac1cByte,
    pub bcfcn: Btac1cByte,
    pub bsfcn: Btac1cByte,
    pub usefx: Btac1cByte,
    pub firfx: [[Btac1cS16; 8]; 4],
}

/// Type of a prediction function. Mirrors the C signature
/// `int (*)(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx)`.
pub type PredictFn = fn(psamp: &[i32; 8], idx: i32, pfcn: i32, ridx: &Btac1cIdxstate) -> i32;

#[inline(always)]
fn at(psamp: &[i32; 8], idx: i32, off: i32) -> i32 {
    // Replicates `psamp[(idx - off) & 7]` in C. We mask to 3 bits so the
    // index is always in range, regardless of the sign of `idx - off`.
    let i = (idx.wrapping_sub(off)) & 7;
    psamp[i as usize]
}

fn BTAC1C2_PredictSample(
    psamp: &[i32; 8],
    idx: i32,
    pfcn: i32,
    ridx: &Btac1cIdxstate,
) -> i32 {
    let i = idx;
    match pfcn {
        0 => at(psamp, i, 1),
        1 => 2i32
            .wrapping_mul(at(psamp, i, 1))
            .wrapping_sub(at(psamp, i, 2)),
        2 => (3i32.wrapping_mul(at(psamp, i, 1)).wrapping_sub(at(psamp, i, 2))) >> 1,
        3 => (5i32.wrapping_mul(at(psamp, i, 1)).wrapping_sub(at(psamp, i, 2))) >> 2,
        4 => {
            let p0 = at(psamp, i, 1).wrapping_add(at(psamp, i, 2));
            let p1 = at(psamp, i, 2).wrapping_add(at(psamp, i, 3));
            p0.wrapping_sub(p1 >> 1)
        }
        5 => {
            let p0 = at(psamp, i, 1).wrapping_add(at(psamp, i, 2));
            let p1 = at(psamp, i, 2).wrapping_add(at(psamp, i, 3));
            (3i32.wrapping_mul(p0).wrapping_sub(p1)) >> 2
        }
        6 => {
            let p0 = at(psamp, i, 1).wrapping_add(at(psamp, i, 2));
            let p1 = at(psamp, i, 2).wrapping_add(at(psamp, i, 3));
            (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3
        }
        7 => {
            let v = 18i32
                .wrapping_mul(at(psamp, i, 1))
                .wrapping_sub(4i32.wrapping_mul(at(psamp, i, 2)))
                .wrapping_add(3i32.wrapping_mul(at(psamp, i, 3)))
                .wrapping_sub(2i32.wrapping_mul(at(psamp, i, 4)))
                .wrapping_add(1i32.wrapping_mul(at(psamp, i, 5)));
            v / 16
        }
        8 => {
            let v = 72i32
                .wrapping_mul(at(psamp, i, 1))
                .wrapping_sub(16i32.wrapping_mul(at(psamp, i, 2)))
                .wrapping_add(12i32.wrapping_mul(at(psamp, i, 3)))
                .wrapping_sub(8i32.wrapping_mul(at(psamp, i, 4)))
                .wrapping_add(5i32.wrapping_mul(at(psamp, i, 5)))
                .wrapping_sub(3i32.wrapping_mul(at(psamp, i, 6)))
                .wrapping_add(3i32.wrapping_mul(at(psamp, i, 7)))
                .wrapping_sub(1i32.wrapping_mul(at(psamp, i, 8)));
            v / 64
        }
        9 => {
            let v = 76i32
                .wrapping_mul(at(psamp, i, 1))
                .wrapping_sub(17i32.wrapping_mul(at(psamp, i, 2)))
                .wrapping_add(10i32.wrapping_mul(at(psamp, i, 3)))
                .wrapping_sub(7i32.wrapping_mul(at(psamp, i, 4)))
                .wrapping_add(5i32.wrapping_mul(at(psamp, i, 5)))
                .wrapping_sub(4i32.wrapping_mul(at(psamp, i, 6)))
                .wrapping_add(4i32.wrapping_mul(at(psamp, i, 7)))
                .wrapping_sub(3i32.wrapping_mul(at(psamp, i, 8)));
            v / 64
        }
        10 => {
            let p0 = at(psamp, i, 1)
                .wrapping_add(at(psamp, i, 2))
                .wrapping_add(at(psamp, i, 3))
                .wrapping_add(at(psamp, i, 4));
            let p1 = at(psamp, i, 5)
                .wrapping_add(at(psamp, i, 6))
                .wrapping_add(at(psamp, i, 7))
                .wrapping_add(at(psamp, i, 8));
            (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 4
        }
        11 => {
            let p0 = at(psamp, i, 1)
                .wrapping_add(at(psamp, i, 2))
                .wrapping_add(at(psamp, i, 3))
                .wrapping_add(at(psamp, i, 4));
            let p1 = at(psamp, i, 5)
                .wrapping_add(at(psamp, i, 6))
                .wrapping_add(at(psamp, i, 7))
                .wrapping_add(at(psamp, i, 8));
            (p0.wrapping_add(p1)) >> 3
        }
        12 | 13 | 14 | 15 => {
            let row = (pfcn - 12) as usize;
            let coeffs = &ridx.firfx[row];
            let mut acc: i32 = 0;
            for k in 0..8usize {
                let c = coeffs[k] as i32;
                let s = at(psamp, i, (k as i32) + 1);
                acc = acc.wrapping_add(c.wrapping_mul(s));
            }
            acc / 256
        }
        _ => 0,
    }
}

fn BTAC1C2_PredictSample_Pfn0(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    at(psamp, idx, 1)
}

fn BTAC1C2_PredictSample_Pfn1(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    2i32.wrapping_mul(at(psamp, idx, 1))
        .wrapping_sub(at(psamp, idx, 2))
}

fn BTAC1C2_PredictSample_Pfn2(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    (3i32
        .wrapping_mul(at(psamp, idx, 1))
        .wrapping_sub(at(psamp, idx, 2)))
        >> 1
}

fn BTAC1C2_PredictSample_Pfn3(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    (5i32
        .wrapping_mul(at(psamp, idx, 1))
        .wrapping_sub(at(psamp, idx, 2)))
        >> 2
}

fn BTAC1C2_PredictSample_Pfn4(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    let p0 = at(psamp, idx, 1).wrapping_add(at(psamp, idx, 2));
    let p1 = at(psamp, idx, 2).wrapping_add(at(psamp, idx, 3));
    p0.wrapping_sub(p1 >> 1)
}

fn BTAC1C2_PredictSample_Pfn5(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    let p0 = at(psamp, idx, 1).wrapping_add(at(psamp, idx, 2));
    let p1 = at(psamp, idx, 2).wrapping_add(at(psamp, idx, 3));
    (3i32.wrapping_mul(p0).wrapping_sub(p1)) >> 2
}

fn BTAC1C2_PredictSample_Pfn6(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    let p0 = at(psamp, idx, 1).wrapping_add(at(psamp, idx, 2));
    let p1 = at(psamp, idx, 2).wrapping_add(at(psamp, idx, 3));
    (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3
}

fn BTAC1C2_PredictSample_Pfn7(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    let v = 18i32
        .wrapping_mul(at(psamp, idx, 1))
        .wrapping_sub(4i32.wrapping_mul(at(psamp, idx, 2)))
        .wrapping_add(3i32.wrapping_mul(at(psamp, idx, 3)))
        .wrapping_sub(2i32.wrapping_mul(at(psamp, idx, 4)))
        .wrapping_add(1i32.wrapping_mul(at(psamp, idx, 5)));
    v / 16
}

fn BTAC1C2_PredictSample_Pfn8(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    let v = 72i32
        .wrapping_mul(at(psamp, idx, 1))
        .wrapping_sub(16i32.wrapping_mul(at(psamp, idx, 2)))
        .wrapping_add(12i32.wrapping_mul(at(psamp, idx, 3)))
        .wrapping_sub(8i32.wrapping_mul(at(psamp, idx, 4)))
        .wrapping_add(5i32.wrapping_mul(at(psamp, idx, 5)))
        .wrapping_sub(3i32.wrapping_mul(at(psamp, idx, 6)))
        .wrapping_add(3i32.wrapping_mul(at(psamp, idx, 7)))
        .wrapping_sub(1i32.wrapping_mul(at(psamp, idx, 8)));
    v / 64
}

fn BTAC1C2_PredictSample_Pfn9(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    let v = 76i32
        .wrapping_mul(at(psamp, idx, 1))
        .wrapping_sub(17i32.wrapping_mul(at(psamp, idx, 2)))
        .wrapping_add(10i32.wrapping_mul(at(psamp, idx, 3)))
        .wrapping_sub(7i32.wrapping_mul(at(psamp, idx, 4)))
        .wrapping_add(5i32.wrapping_mul(at(psamp, idx, 5)))
        .wrapping_sub(4i32.wrapping_mul(at(psamp, idx, 6)))
        .wrapping_add(4i32.wrapping_mul(at(psamp, idx, 7)))
        .wrapping_sub(3i32.wrapping_mul(at(psamp, idx, 8)));
    v / 64
}

fn BTAC1C2_PredictSample_Pfn10(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    let p0 = at(psamp, idx, 1)
        .wrapping_add(at(psamp, idx, 2))
        .wrapping_add(at(psamp, idx, 3))
        .wrapping_add(at(psamp, idx, 4));
    let p1 = at(psamp, idx, 5)
        .wrapping_add(at(psamp, idx, 6))
        .wrapping_add(at(psamp, idx, 7))
        .wrapping_add(at(psamp, idx, 8));
    (5i32.wrapping_mul(p0).wrapping_sub(p1)) >> 3
}

fn BTAC1C2_PredictSample_Pfn11(
    psamp: &[i32; 8],
    idx: i32,
    _pfcn: i32,
    _ridx: &Btac1cIdxstate,
) -> i32 {
    let p0 = at(psamp, idx, 1)
        .wrapping_add(at(psamp, idx, 2))
        .wrapping_add(at(psamp, idx, 3))
        .wrapping_add(at(psamp, idx, 4));
    let p1 = at(psamp, idx, 5)
        .wrapping_add(at(psamp, idx, 6))
        .wrapping_add(at(psamp, idx, 7))
        .wrapping_add(at(psamp, idx, 8));
    (p0.wrapping_add(p1)) >> 1
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

/// Returns 1 if the dispatcher selected the matching `Pfn<pfcn>` function
/// for `pfcn` in 0..=11, and 0 otherwise. This mirrors `call_predict` in
/// the C source, which compared raw function pointers.
pub fn call_predict(pfcn: i32) -> i32 {
    let fcn = BTAC1C2_GetPredictFunc(pfcn);
    let expected: Option<PredictFn> = match pfcn {
        0 => Some(BTAC1C2_PredictSample_Pfn0),
        1 => Some(BTAC1C2_PredictSample_Pfn1),
        2 => Some(BTAC1C2_PredictSample_Pfn2),
        3 => Some(BTAC1C2_PredictSample_Pfn3),
        4 => Some(BTAC1C2_PredictSample_Pfn4),
        5 => Some(BTAC1C2_PredictSample_Pfn5),
        6 => Some(BTAC1C2_PredictSample_Pfn6),
        7 => Some(BTAC1C2_PredictSample_Pfn7),
        8 => Some(BTAC1C2_PredictSample_Pfn8),
        9 => Some(BTAC1C2_PredictSample_Pfn9),
        10 => Some(BTAC1C2_PredictSample_Pfn10),
        11 => Some(BTAC1C2_PredictSample_Pfn11),
        _ => None,
    };
    match expected {
        Some(e) => {
            // Comparing function pointers: cast through `usize` to avoid
            // any concerns about ABI-specific equality semantics.
            if (fcn as usize) == (e as usize) {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

/// Mirrors `int get_predict_func(int pfcn);` declared in `c_src/include/lib.h`.
/// The C source file does not actually define a `get_predict_func` function,
/// only `call_predict`. We expose a function with the header's name that
/// returns the same value `call_predict` would, so the public surface lines
/// up with the header.
pub fn get_predict_func(pfcn: i32) -> i32 {
    call_predict(pfcn)
}
