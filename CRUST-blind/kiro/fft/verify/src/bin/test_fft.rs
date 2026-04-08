use fft::fft::{fft, fft_clz, fft_inplace, fft_raw, next_reversed_n, rader, rader_inplace, FftComplex};

fn c(real: f32, imag: f32) -> FftComplex {
    FftComplex { real, imag }
}

fn assert_complex_near(actual: &[FftComplex], expected: &[(f32, f32)], tol: f32) {
    assert_eq!(actual.len(), expected.len());
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (a.real - e.0).abs() < tol && (a.imag - e.1).abs() < tol,
            "index {}: got ({}, {}), expected ({}, {})",
            i, a.real, a.imag, e.0, e.1
        );
    }
}

// === fft_clz ===

#[test]
fn test_fft_clz_one() {
    assert_eq!(fft_clz(1), 63);
}

#[test]
fn test_fft_clz_powers_of_two() {
    assert_eq!(fft_clz(2), 62);
    assert_eq!(fft_clz(4), 61);
    assert_eq!(fft_clz(8), 60);
    assert_eq!(fft_clz(256), 55);
}

#[test]
fn test_fft_clz_255() {
    assert_eq!(fft_clz(255), 56);
}

#[test]
fn test_fft_clz_max() {
    assert_eq!(fft_clz(usize::MAX), 0);
}

// === next_reversed_n ===

#[test]
fn test_next_reversed_n_logsize3() {
    let shift = usize::BITS as usize - 3;
    let expected = [0, 4, 2, 6, 1, 5, 3, 7];
    let mut rev = 0usize;
    for i in 0..8 {
        assert_eq!(rev, expected[i], "n={}", i);
        rev = next_reversed_n(rev, shift);
    }
}

#[test]
fn test_next_reversed_n_logsize2() {
    let shift = usize::BITS as usize - 2;
    let expected = [0, 2, 1, 3];
    let mut rev = 0usize;
    for i in 0..4 {
        assert_eq!(rev, expected[i], "n={}", i);
        rev = next_reversed_n(rev, shift);
    }
}

#[test]
fn test_next_reversed_n_logsize1() {
    let shift = usize::BITS as usize - 1;
    let mut rev = 0usize;
    assert_eq!(rev, 0);
    rev = next_reversed_n(rev, shift);
    assert_eq!(rev, 1);
}

// === rader ===

#[test]
fn test_rader_logsize3() {
    let x: Vec<FftComplex> = (0..8).map(|i| c(i as f32, 0.0)).collect();
    let mut target = vec![c(0.0, 0.0); 8];
    rader(&x, &mut target, 3);
    // bit-reversal permutation: [0,4,2,6,1,5,3,7]
    let expected_reals = [0.0, 4.0, 2.0, 6.0, 1.0, 5.0, 3.0, 7.0];
    for (i, &er) in expected_reals.iter().enumerate() {
        assert_eq!(target[i].real, er, "rader index {}", i);
    }
}

#[test]
fn test_rader_logsize0() {
    let x = [c(42.0, 7.0)];
    let mut target = [c(0.0, 0.0)];
    rader(&x, &mut target, 0);
    assert_eq!(target[0].real, 42.0);
    assert_eq!(target[0].imag, 7.0);
}

// === rader_inplace ===

#[test]
fn test_rader_inplace_logsize3() {
    let mut x: Vec<FftComplex> = (0..8).map(|i| c(i as f32, 0.0)).collect();
    rader_inplace(&mut x, 3);
    let expected_reals = [0.0, 4.0, 2.0, 6.0, 1.0, 5.0, 3.0, 7.0];
    for (i, &er) in expected_reals.iter().enumerate() {
        assert_eq!(x[i].real, er, "rader_inplace index {}", i);
    }
}

#[test]
fn test_rader_inplace_logsize0() {
    let mut x = [c(5.0, 3.0)];
    rader_inplace(&mut x, 0);
    assert_eq!(x[0].real, 5.0);
    assert_eq!(x[0].imag, 3.0);
}

// === fft_raw ===

#[test]
fn test_fft_raw_logsize0() {
    let mut x = [c(42.0, 7.0)];
    fft_raw(&mut x, 0);
    assert_eq!(x[0].real, 42.0);
    assert_eq!(x[0].imag, 7.0);
}

#[test]
fn test_fft_raw_logsize1() {
    let mut x = [c(1.0, 0.0), c(2.0, 0.0)];
    fft_raw(&mut x, 1);
    assert_eq!(x[0].real, 3.0);
    assert_eq!(x[1].real, -1.0);
}

// === fft ===

#[test]
fn test_fft_logsize0() {
    let x = [c(42.0, 7.0)];
    let mut out = [c(0.0, 0.0)];
    fft(&x, &mut out, 0);
    assert_eq!(out[0].real, 42.0);
    assert_eq!(out[0].imag, 7.0);
}

#[test]
fn test_fft_logsize1() {
    let x = [c(1.0, 0.0), c(2.0, 0.0)];
    let mut out = [c(0.0, 0.0); 2];
    fft(&x, &mut out, 1);
    assert_complex_near(&out, &[(3.0, 0.0), (-1.0, 0.0)], 1e-5);
}

#[test]
fn test_fft_logsize2() {
    let x = [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0), c(4.0, 0.0)];
    let mut out = [c(0.0, 0.0); 4];
    fft(&x, &mut out, 2);
    assert_complex_near(
        &out,
        &[(10.0, 0.0), (-2.0, 2.0), (-2.0, 0.0), (-2.0, -2.0)],
        1e-5,
    );
}

#[test]
fn test_fft_logsize3_alternating() {
    let x = [c(1.0,0.0),c(-1.0,0.0),c(1.0,0.0),c(-1.0,0.0),
             c(1.0,0.0),c(-1.0,0.0),c(1.0,0.0),c(-1.0,0.0)];
    let mut out = [c(0.0, 0.0); 8];
    fft(&x, &mut out, 3);
    let expected = [(0.0,0.0),(0.0,0.0),(0.0,0.0),(0.0,0.0),
                    (8.0,0.0),(0.0,0.0),(0.0,0.0),(0.0,0.0)];
    assert_complex_near(&out, &expected, 1e-5);
}

#[test]
fn test_fft_complex_inputs() {
    let x = [c(1.0, 1.0), c(2.0, -1.0), c(-1.0, 2.0), c(0.0, 0.5)];
    let mut out = [c(0.0, 0.0); 4];
    fft(&x, &mut out, 2);
    assert_complex_near(
        &out,
        &[(2.0, 2.5), (0.5, -3.0), (-2.0, 3.5), (3.5, 1.0)],
        1e-4,
    );
}

#[test]
fn test_fft_all_zeros() {
    let x = [c(0.0, 0.0); 4];
    let mut out = [c(0.0, 0.0); 4];
    fft(&x, &mut out, 2);
    assert_complex_near(&out, &[(0.0,0.0),(0.0,0.0),(0.0,0.0),(0.0,0.0)], 1e-5);
}

#[test]
fn test_fft_single_nonzero() {
    let x = [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)];
    let mut out = [c(0.0, 0.0); 4];
    fft(&x, &mut out, 2);
    assert_complex_near(&out, &[(1.0,0.0),(1.0,0.0),(1.0,0.0),(1.0,0.0)], 1e-5);
}

// === fft_inplace ===

#[test]
fn test_fft_inplace_logsize0() {
    let mut x = [c(5.0, 3.0)];
    fft_inplace(&mut x, 0);
    assert_eq!(x[0].real, 5.0);
    assert_eq!(x[0].imag, 3.0);
}

#[test]
fn test_fft_inplace_logsize1() {
    let mut x = [c(1.0, 0.0), c(2.0, 0.0)];
    fft_inplace(&mut x, 1);
    assert_complex_near(&x, &[(3.0, 0.0), (-1.0, 0.0)], 1e-5);
}

#[test]
fn test_fft_inplace_logsize2() {
    let mut x = [c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0), c(4.0, 0.0)];
    fft_inplace(&mut x, 2);
    assert_complex_near(
        &x,
        &[(10.0, 0.0), (-2.0, 2.0), (-2.0, 0.0), (-2.0, -2.0)],
        1e-5,
    );
}

#[test]
fn test_fft_inplace_logsize3_alternating() {
    let mut x = [c(1.0,0.0),c(-1.0,0.0),c(1.0,0.0),c(-1.0,0.0),
                 c(1.0,0.0),c(-1.0,0.0),c(1.0,0.0),c(-1.0,0.0)];
    fft_inplace(&mut x, 3);
    let expected = [(0.0,0.0),(0.0,0.0),(0.0,0.0),(0.0,0.0),
                    (8.0,0.0),(0.0,0.0),(0.0,0.0),(0.0,0.0)];
    assert_complex_near(&x, &expected, 1e-5);
}

// === fft vs fft_inplace consistency ===

#[test]
fn test_fft_vs_fft_inplace_logsize3() {
    let x: Vec<FftComplex> = (0..8).map(|i| c((2*i+1) as f32, (2*i+2) as f32)).collect();
    let mut out = vec![c(0.0, 0.0); 8];
    fft(&x, &mut out, 3);
    let mut x2 = x.clone();
    fft_inplace(&mut x2, 3);
    let expected = [
        (64.0, 72.0),
        (-27.313709, 11.313708),
        (-16.0, 0.0),
        (-11.313707, -4.686293),
        (-8.0, -8.0),
        (-4.686292, -11.313708),
        (0.0, -16.0),
        (11.313707, -27.313707),
    ];
    assert_complex_near(&out, &expected, 1e-3);
    // fft and fft_inplace must match
    for i in 0..8 {
        assert!((out[i].real - x2[i].real).abs() < 1e-5, "real mismatch at {}", i);
        assert!((out[i].imag - x2[i].imag).abs() < 1e-5, "imag mismatch at {}", i);
    }
}

fn main() {}
