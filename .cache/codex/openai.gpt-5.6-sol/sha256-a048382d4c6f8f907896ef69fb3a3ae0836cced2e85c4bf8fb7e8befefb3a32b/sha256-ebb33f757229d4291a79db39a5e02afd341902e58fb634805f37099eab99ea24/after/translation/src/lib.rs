#![allow(dead_code)]

use std::ffi::c_int;

#[repr(C)]
struct IdxState {
    idx: u16,
    lpred: i16,
    rpred: i16,
    tag: u8,
    bcfcn: u8,
    bsfcn: u8,
    usefx: u8,
    firfx: [[i16; 8]; 4],
}

fn sample(samples: &[c_int; 8], idx: c_int, offset: c_int) -> c_int {
    samples[(idx.wrapping_sub(offset) & 7) as usize]
}

fn sum(values: &[c_int]) -> c_int {
    values.iter().copied().fold(0, c_int::wrapping_add)
}

fn weighted_sum(terms: &[(c_int, c_int)]) -> c_int {
    terms.iter().fold(0, |acc, &(coefficient, value)| {
        acc.wrapping_add(coefficient.wrapping_mul(value))
    })
}

fn predict_sample(samples: &[c_int; 8], idx: c_int, pfcn: c_int, state: &IdxState) -> c_int {
    let s = |offset| sample(samples, idx, offset);

    match pfcn {
        0 => s(1),
        1 => 2i32.wrapping_mul(s(1)).wrapping_sub(s(2)),
        2 => 3i32.wrapping_mul(s(1)).wrapping_sub(s(2)) >> 1,
        3 => 5i32.wrapping_mul(s(1)).wrapping_sub(s(2)) >> 2,
        4 => {
            let p0 = s(1).wrapping_add(s(2));
            let p1 = s(2).wrapping_add(s(3));
            p0.wrapping_sub(p1 >> 1)
        }
        5 => {
            let p0 = s(1).wrapping_add(s(2));
            let p1 = s(2).wrapping_add(s(3));
            3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2
        }
        6 => {
            let p0 = s(1).wrapping_add(s(2));
            let p1 = s(2).wrapping_add(s(3));
            5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
        }
        7 => weighted_sum(&[(18, s(1)), (-4, s(2)), (3, s(3)), (-2, s(4)), (1, s(5))]) / 16,
        8 => {
            weighted_sum(&[
                (72, s(1)),
                (-16, s(2)),
                (12, s(3)),
                (-8, s(4)),
                (5, s(5)),
                (-3, s(6)),
                (3, s(7)),
                (-1, s(8)),
            ]) / 64
        }
        9 => {
            weighted_sum(&[
                (76, s(1)),
                (-17, s(2)),
                (10, s(3)),
                (-7, s(4)),
                (5, s(5)),
                (-4, s(6)),
                (4, s(7)),
                (-3, s(8)),
            ]) / 64
        }
        10 => {
            let p0 = sum(&[s(1), s(2), s(3), s(4)]);
            let p1 = sum(&[s(5), s(6), s(7), s(8)]);
            5i32.wrapping_mul(p0).wrapping_sub(p1) >> 4
        }
        11 => {
            let p0 = sum(&[s(1), s(2), s(3), s(4)]);
            let p1 = sum(&[s(5), s(6), s(7), s(8)]);
            p0.wrapping_add(p1) >> 3
        }
        12..=15 => {
            let coefficients = &state.firfx[(pfcn - 12) as usize];
            coefficients
                .iter()
                .enumerate()
                .fold(0 as c_int, |acc, (position, &coefficient)| {
                    acc.wrapping_add(
                        c_int::from(coefficient)
                            .wrapping_mul(s(c_int::try_from(position + 1).unwrap())),
                    )
                })
                / 256
        }
        _ => 0,
    }
}

fn predict_sample_pfn0(samples: &[c_int; 8], idx: c_int) -> c_int {
    sample(samples, idx, 1)
}

fn predict_sample_pfn1(samples: &[c_int; 8], idx: c_int) -> c_int {
    2i32.wrapping_mul(sample(samples, idx, 1))
        .wrapping_sub(sample(samples, idx, 2))
}

fn predict_sample_pfn2(samples: &[c_int; 8], idx: c_int) -> c_int {
    3i32.wrapping_mul(sample(samples, idx, 1))
        .wrapping_sub(sample(samples, idx, 2))
        >> 1
}

fn predict_sample_pfn3(samples: &[c_int; 8], idx: c_int) -> c_int {
    5i32.wrapping_mul(sample(samples, idx, 1))
        .wrapping_sub(sample(samples, idx, 2))
        >> 2
}

fn predictor_pair(samples: &[c_int; 8], idx: c_int) -> (c_int, c_int) {
    let p0 = sample(samples, idx, 1).wrapping_add(sample(samples, idx, 2));
    let p1 = sample(samples, idx, 2).wrapping_add(sample(samples, idx, 3));
    (p0, p1)
}

fn predict_sample_pfn4(samples: &[c_int; 8], idx: c_int) -> c_int {
    let (p0, p1) = predictor_pair(samples, idx);
    p0.wrapping_sub(p1 >> 1)
}

fn predict_sample_pfn5(samples: &[c_int; 8], idx: c_int) -> c_int {
    let (p0, p1) = predictor_pair(samples, idx);
    3i32.wrapping_mul(p0).wrapping_sub(p1) >> 2
}

fn predict_sample_pfn6(samples: &[c_int; 8], idx: c_int) -> c_int {
    let (p0, p1) = predictor_pair(samples, idx);
    5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
}

fn predict_sample_pfn7(samples: &[c_int; 8], idx: c_int) -> c_int {
    weighted_sum(&[
        (18, sample(samples, idx, 1)),
        (-4, sample(samples, idx, 2)),
        (3, sample(samples, idx, 3)),
        (-2, sample(samples, idx, 4)),
        (1, sample(samples, idx, 5)),
    ]) / 16
}

fn predict_sample_pfn8(samples: &[c_int; 8], idx: c_int) -> c_int {
    weighted_sum(&[
        (72, sample(samples, idx, 1)),
        (-16, sample(samples, idx, 2)),
        (12, sample(samples, idx, 3)),
        (-8, sample(samples, idx, 4)),
        (5, sample(samples, idx, 5)),
        (-3, sample(samples, idx, 6)),
        (3, sample(samples, idx, 7)),
        (-1, sample(samples, idx, 8)),
    ]) / 64
}

fn predict_sample_pfn9(samples: &[c_int; 8], idx: c_int) -> c_int {
    weighted_sum(&[
        (76, sample(samples, idx, 1)),
        (-17, sample(samples, idx, 2)),
        (10, sample(samples, idx, 3)),
        (-7, sample(samples, idx, 4)),
        (5, sample(samples, idx, 5)),
        (-4, sample(samples, idx, 6)),
        (4, sample(samples, idx, 7)),
        (-3, sample(samples, idx, 8)),
    ]) / 64
}

fn predictor_groups(samples: &[c_int; 8], idx: c_int) -> (c_int, c_int) {
    let p0 = sum(&[
        sample(samples, idx, 1),
        sample(samples, idx, 2),
        sample(samples, idx, 3),
        sample(samples, idx, 4),
    ]);
    let p1 = sum(&[
        sample(samples, idx, 5),
        sample(samples, idx, 6),
        sample(samples, idx, 7),
        sample(samples, idx, 8),
    ]);
    (p0, p1)
}

fn predict_sample_pfn10(samples: &[c_int; 8], idx: c_int) -> c_int {
    let (p0, p1) = predictor_groups(samples, idx);
    5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3
}

fn predict_sample_pfn11(samples: &[c_int; 8], idx: c_int) -> c_int {
    let (p0, p1) = predictor_groups(samples, idx);
    p0.wrapping_add(p1) >> 1
}

enum Predictor {
    Specialized,
    Generic,
}

fn get_predictor(pfcn: c_int) -> Predictor {
    match pfcn {
        0..=11 => Predictor::Specialized,
        _ => Predictor::Generic,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    match (pfcn, get_predictor(pfcn)) {
        (0..=11, Predictor::Specialized) => 1,
        _ => 0,
    }
}
