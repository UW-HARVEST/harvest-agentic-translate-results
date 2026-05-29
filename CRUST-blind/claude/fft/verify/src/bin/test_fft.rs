use fft::fft::{
    fft, fft_clz, fft_inplace, fft_raw, next_reversed_n, rader, rader_inplace, FftComplex,
};

fn c(real: f32, imag: f32) -> FftComplex {
    FftComplex { real, imag }
}

fn approx(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn assert_close(actual: FftComplex, expected: FftComplex, eps: f32) {
    assert!(
        approx(actual.real, expected.real, eps) && approx(actual.imag, expected.imag, eps),
        "expected ({}, {}), got ({}, {})",
        expected.real,
        expected.imag,
        actual.real,
        actual.imag,
    );
}

fn assert_arr_close(actual: &[FftComplex], expected: &[FftComplex], eps: f32) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "length mismatch: {} vs {}",
        actual.len(),
        expected.len()
    );
    for i in 0..actual.len() {
        assert!(
            approx(actual[i].real, expected[i].real, eps)
                && approx(actual[i].imag, expected[i].imag, eps),
            "at index {}: expected ({}, {}), got ({}, {})",
            i,
            expected[i].real,
            expected[i].imag,
            actual[i].real,
            actual[i].imag,
        );
    }
}

// ----------------------------------------------------------------------------
// FftComplex struct tests
// ----------------------------------------------------------------------------

#[test]
fn test_fft_complex_construction() {
    let z = FftComplex { real: 3.0, imag: -4.5 };
    assert_eq!(z.real, 3.0);
    assert_eq!(z.imag, -4.5);
}

#[test]
fn test_fft_complex_copy_clone() {
    let z = FftComplex { real: 1.5, imag: 2.5 };
    let z2 = z; // Copy
    let z3 = z.clone();
    assert_eq!(z2.real, 1.5);
    assert_eq!(z2.imag, 2.5);
    assert_eq!(z3.real, 1.5);
    assert_eq!(z3.imag, 2.5);
}

// ----------------------------------------------------------------------------
// fft_clz tests
// ----------------------------------------------------------------------------

#[test]
fn test_fft_clz_zero() {
    let bits = usize::BITS as usize;
    assert_eq!(fft_clz(0), bits);
}

#[test]
fn test_fft_clz_one() {
    let bits = usize::BITS as usize;
    assert_eq!(fft_clz(1), bits - 1);
}

#[test]
fn test_fft_clz_two() {
    let bits = usize::BITS as usize;
    assert_eq!(fft_clz(2), bits - 2);
}

#[test]
fn test_fft_clz_0xff() {
    let bits = usize::BITS as usize;
    assert_eq!(fft_clz(0xff), bits - 8);
}

#[test]
fn test_fft_clz_max() {
    assert_eq!(fft_clz(usize::MAX), 0);
}

#[test]
fn test_fft_clz_msb() {
    let bits = usize::BITS as usize;
    let msb: usize = 1usize << (bits - 1);
    assert_eq!(fft_clz(msb), 0);
}

// ----------------------------------------------------------------------------
// next_reversed_n tests (matches the bit-reversed counter sequence in C)
// ----------------------------------------------------------------------------

#[test]
fn test_next_reversed_n_logsize1() {
    let bits = usize::BITS as usize;
    let shift = bits - 1;
    // Expected sequence (from C harness): 0 1
    let mut r = 0usize;
    let seq: Vec<usize> = (0..2)
        .map(|_| {
            let cur = r;
            r = next_reversed_n(r, shift);
            cur
        })
        .collect();
    assert_eq!(seq, vec![0, 1]);
}

#[test]
fn test_next_reversed_n_logsize2() {
    let bits = usize::BITS as usize;
    let shift = bits - 2;
    // Expected sequence (from C harness): 0 2 1 3
    let mut r = 0usize;
    let seq: Vec<usize> = (0..4)
        .map(|_| {
            let cur = r;
            r = next_reversed_n(r, shift);
            cur
        })
        .collect();
    assert_eq!(seq, vec![0, 2, 1, 3]);
}

#[test]
fn test_next_reversed_n_logsize3() {
    let bits = usize::BITS as usize;
    let shift = bits - 3;
    // Expected sequence (from C harness): 0 4 2 6 1 5 3 7
    let mut r = 0usize;
    let seq: Vec<usize> = (0..8)
        .map(|_| {
            let cur = r;
            r = next_reversed_n(r, shift);
            cur
        })
        .collect();
    assert_eq!(seq, vec![0, 4, 2, 6, 1, 5, 3, 7]);
}

#[test]
fn test_next_reversed_n_logsize4() {
    let bits = usize::BITS as usize;
    let shift = bits - 4;
    // Expected sequence (from C harness): 0 8 4 12 2 10 6 14 1 9 5 13 3 11 7 15
    let mut r = 0usize;
    let seq: Vec<usize> = (0..16)
        .map(|_| {
            let cur = r;
            r = next_reversed_n(r, shift);
            cur
        })
        .collect();
    assert_eq!(seq, vec![0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15]);
}

// ----------------------------------------------------------------------------
// rader / rader_inplace tests
// ----------------------------------------------------------------------------

#[test]
fn test_rader_logsize3() {
    // Bit-reversal permutation: target[reversed_n] = source[n]
    // For logsize=3, the reversed indices written are 0,4,2,6,1,5,3,7.
    // So if source = [0,1,2,3,4,5,6,7] then target = [0,4,2,6,1,5,3,7]
    let source: Vec<FftComplex> = (0..8).map(|i| c(i as f32, 0.0)).collect();
    let mut target: Vec<FftComplex> = vec![c(0.0, 0.0); 8];
    rader(&source, &mut target, 3);
    let expected_indices = [0, 4, 2, 6, 1, 5, 3, 7];
    for (i, &idx) in expected_indices.iter().enumerate() {
        assert_eq!(target[i].real, idx as f32);
        assert_eq!(target[i].imag, 0.0);
    }
}

#[test]
fn test_rader_logsize2() {
    // For logsize=2, sequence is 0,2,1,3. So source [0,1,2,3] -> target [0,2,1,3].
    let source: Vec<FftComplex> = (0..4).map(|i| c(i as f32, 0.0)).collect();
    let mut target: Vec<FftComplex> = vec![c(0.0, 0.0); 4];
    rader(&source, &mut target, 2);
    assert_eq!(target[0].real, 0.0);
    assert_eq!(target[1].real, 2.0);
    assert_eq!(target[2].real, 1.0);
    assert_eq!(target[3].real, 3.0);
    for v in &target {
        assert_eq!(v.imag, 0.0);
    }
}

#[test]
fn test_rader_logsize1() {
    let source = vec![c(10.0, 1.0), c(20.0, 2.0)];
    let mut target = vec![c(0.0, 0.0); 2];
    rader(&source, &mut target, 1);
    // Sequence is 0, 1: identity for size 2.
    assert_eq!(target[0].real, 10.0);
    assert_eq!(target[0].imag, 1.0);
    assert_eq!(target[1].real, 20.0);
    assert_eq!(target[1].imag, 2.0);
}

#[test]
fn test_rader_inplace_logsize3() {
    // Starting array [0,1,2,3,4,5,6,7], after rader_inplace should match the same
    // bit-reversal as rader (since the operation is its own inverse / a permutation).
    let mut arr: Vec<FftComplex> = (0..8).map(|i| c(i as f32, 0.0)).collect();
    rader_inplace(&mut arr, 3);
    let expected = [0.0_f32, 4.0, 2.0, 6.0, 1.0, 5.0, 3.0, 7.0];
    for (i, &v) in expected.iter().enumerate() {
        assert_eq!(arr[i].real, v);
        assert_eq!(arr[i].imag, 0.0);
    }
}

#[test]
fn test_rader_inplace_logsize4() {
    let mut arr: Vec<FftComplex> = (0..16).map(|i| c(i as f32, 0.0)).collect();
    rader_inplace(&mut arr, 4);
    let expected = [
        0.0_f32, 8.0, 4.0, 12.0, 2.0, 10.0, 6.0, 14.0, 1.0, 9.0, 5.0, 13.0, 3.0, 11.0, 7.0, 15.0,
    ];
    for (i, &v) in expected.iter().enumerate() {
        assert_eq!(arr[i].real, v);
        assert_eq!(arr[i].imag, 0.0);
    }
}

#[test]
fn test_rader_inplace_logsize0() {
    // Should not panic / not modify
    let mut arr = vec![c(7.0, 11.0)];
    rader_inplace(&mut arr, 0);
    assert_eq!(arr[0].real, 7.0);
    assert_eq!(arr[0].imag, 11.0);
}

#[test]
fn test_rader_inplace_logsize1() {
    // size = 2; loop runs from n=1 to n<1 (doesn't run). Identity.
    let mut arr = vec![c(3.0, 4.0), c(5.0, 6.0)];
    rader_inplace(&mut arr, 1);
    assert_eq!(arr[0].real, 3.0);
    assert_eq!(arr[0].imag, 4.0);
    assert_eq!(arr[1].real, 5.0);
    assert_eq!(arr[1].imag, 6.0);
}

// ----------------------------------------------------------------------------
// fft tests with values from the C harness
// ----------------------------------------------------------------------------

#[test]
fn test_fft_alt8() {
    // From C harness: alt8 fft - all zeros except [4]=8
    let input = vec![
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
    ];
    let mut output = vec![c(0.0, 0.0); 8];
    fft(&input, &mut output, 3);
    let expected = vec![
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(8.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
    ];
    assert_arr_close(&output, &expected, 1e-5);
}

#[test]
fn test_fft_logsize0() {
    // From C harness: single fft - identity
    let input = vec![c(5.0, 7.0)];
    let mut output = vec![c(0.0, 0.0); 1];
    fft(&input, &mut output, 0);
    assert_eq!(output[0].real, 5.0);
    assert_eq!(output[0].imag, 7.0);
}

#[test]
fn test_fft_logsize1() {
    // From C harness: two fft = [(4,6), (-2,-2)]
    let input = vec![c(1.0, 2.0), c(3.0, 4.0)];
    let mut output = vec![c(0.0, 0.0); 2];
    fft(&input, &mut output, 1);
    assert_eq!(output[0].real, 4.0);
    assert_eq!(output[0].imag, 6.0);
    assert_eq!(output[1].real, -2.0);
    assert_eq!(output[1].imag, -2.0);
}

#[test]
fn test_fft_logsize2() {
    // From C harness: four fft = [(10,0), (-2,2), (-2,0), (-2,-2)]
    let input = vec![c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0), c(4.0, 0.0)];
    let mut output = vec![c(0.0, 0.0); 4];
    fft(&input, &mut output, 2);
    let expected = vec![c(10.0, 0.0), c(-2.0, 2.0), c(-2.0, 0.0), c(-2.0, -2.0)];
    assert_arr_close(&output, &expected, 1e-5);
}

#[test]
fn test_fft_ramp16() {
    // From C harness: sixteen ramp fft
    let input: Vec<FftComplex> = (0..16).map(|i| c(i as f32, 0.0)).collect();
    let mut output = vec![c(0.0, 0.0); 16];
    fft(&input, &mut output, 4);
    let expected = vec![
        c(120.0, 0.0),
        c(-7.99999952, 40.2187195),
        c(-8.0, 19.3137074),
        c(-8.00000095, 11.9728441),
        c(-8.0, 7.99999952),
        c(-7.99999952, 5.34542847),
        c(-7.99999952, 3.31370783),
        c(-7.99999952, 1.59129524),
        c(-8.0, 0.0),
        c(-8.0, -1.59129906),
        c(-7.99999952, -3.31370783),
        c(-7.99999905, -5.34542942),
        c(-8.0, -7.99999952),
        c(-8.0, -11.9728451),
        c(-8.0, -19.3137074),
        c(-8.0, -40.2187119),
    ];
    assert_arr_close(&output, &expected, 1e-3);
}

#[test]
fn test_fft_complex_data() {
    // From C harness: cplx8 fft
    let input = vec![
        c(1.5, 2.5),
        c(-3.25, 0.125),
        c(0.0, -1.0),
        c(7.5, 8.25),
        c(-0.5, 0.0),
        c(1.0, 1.0),
        c(2.0, -2.0),
        c(3.5, 4.5),
    ];
    let mut output = vec![c(0.0, 0.0); 8];
    fft(&input, &mut output, 3);
    let expected = vec![
        c(11.75, 13.375),
        c(-0.800698757, 1.40640783),
        c(-12.624999, 18.75),
        c(8.86656189, 3.94714522),
        c(-5.75, -14.375),
        c(6.80069876, 7.59359217),
        c(10.624999, -7.74999905),
        c(-6.86656189, -2.94714522),
    ];
    assert_arr_close(&output, &expected, 1e-4);
}

#[test]
fn test_fft_impulse() {
    // Impulse: the FFT of [1,0,0,0,...,0] is all 1s
    let mut input = vec![c(0.0, 0.0); 8];
    input[0] = c(1.0, 0.0);
    let mut output = vec![c(0.0, 0.0); 8];
    fft(&input, &mut output, 3);
    for v in &output {
        assert!(approx(v.real, 1.0, 1e-5));
        assert!(approx(v.imag, 0.0, 1e-5));
    }
}

#[test]
fn test_fft_dc() {
    // DC: FFT of all 1s is [N, 0, 0, ...]
    let input = vec![c(1.0, 0.0); 8];
    let mut output = vec![c(0.0, 0.0); 8];
    fft(&input, &mut output, 3);
    assert!(approx(output[0].real, 8.0, 1e-5));
    assert!(approx(output[0].imag, 0.0, 1e-5));
    for i in 1..8 {
        assert!(approx(output[i].real, 0.0, 1e-5));
        assert!(approx(output[i].imag, 0.0, 1e-5));
    }
}

// ----------------------------------------------------------------------------
// fft_inplace tests (should match fft results exactly)
// ----------------------------------------------------------------------------

#[test]
fn test_fft_inplace_alt8() {
    // From C harness: matches fft output
    let mut data = vec![
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
    ];
    fft_inplace(&mut data, 3);
    let expected = vec![
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(8.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
    ];
    assert_arr_close(&data, &expected, 1e-5);
}

#[test]
fn test_fft_inplace_logsize0() {
    let mut data = vec![c(5.0, 7.0)];
    fft_inplace(&mut data, 0);
    assert_eq!(data[0].real, 5.0);
    assert_eq!(data[0].imag, 7.0);
}

#[test]
fn test_fft_inplace_logsize1() {
    let mut data = vec![c(1.0, 2.0), c(3.0, 4.0)];
    fft_inplace(&mut data, 1);
    assert_eq!(data[0].real, 4.0);
    assert_eq!(data[0].imag, 6.0);
    assert_eq!(data[1].real, -2.0);
    assert_eq!(data[1].imag, -2.0);
}

#[test]
fn test_fft_inplace_logsize2() {
    let mut data = vec![c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0), c(4.0, 0.0)];
    fft_inplace(&mut data, 2);
    let expected = vec![c(10.0, 0.0), c(-2.0, 2.0), c(-2.0, 0.0), c(-2.0, -2.0)];
    assert_arr_close(&data, &expected, 1e-5);
}

#[test]
fn test_fft_inplace_ramp16() {
    let mut data: Vec<FftComplex> = (0..16).map(|i| c(i as f32, 0.0)).collect();
    fft_inplace(&mut data, 4);
    let expected = vec![
        c(120.0, 0.0),
        c(-7.99999952, 40.2187195),
        c(-8.0, 19.3137074),
        c(-8.00000095, 11.9728441),
        c(-8.0, 7.99999952),
        c(-7.99999952, 5.34542847),
        c(-7.99999952, 3.31370783),
        c(-7.99999952, 1.59129524),
        c(-8.0, 0.0),
        c(-8.0, -1.59129906),
        c(-7.99999952, -3.31370783),
        c(-7.99999905, -5.34542942),
        c(-8.0, -7.99999952),
        c(-8.0, -11.9728451),
        c(-8.0, -19.3137074),
        c(-8.0, -40.2187119),
    ];
    assert_arr_close(&data, &expected, 1e-3);
}

#[test]
fn test_fft_inplace_complex_data() {
    let mut data = vec![
        c(1.5, 2.5),
        c(-3.25, 0.125),
        c(0.0, -1.0),
        c(7.5, 8.25),
        c(-0.5, 0.0),
        c(1.0, 1.0),
        c(2.0, -2.0),
        c(3.5, 4.5),
    ];
    fft_inplace(&mut data, 3);
    let expected = vec![
        c(11.75, 13.375),
        c(-0.800698757, 1.40640783),
        c(-12.624999, 18.75),
        c(8.86656189, 3.94714522),
        c(-5.75, -14.375),
        c(6.80069876, 7.59359217),
        c(10.624999, -7.74999905),
        c(-6.86656189, -2.94714522),
    ];
    assert_arr_close(&data, &expected, 1e-4);
}

// ----------------------------------------------------------------------------
// fft_raw tests (pre-bit-reversed input expected)
// ----------------------------------------------------------------------------

#[test]
fn test_fft_raw_logsize0_noop() {
    // fft_raw should not modify a length-1 array (logsize=0).
    let mut data = vec![c(42.0, -3.0)];
    fft_raw(&mut data, 0);
    assert_eq!(data[0].real, 42.0);
    assert_eq!(data[0].imag, -3.0);
}

#[test]
fn test_fft_raw_logsize1() {
    // For logsize=1, rader is identity, so fft_raw on input directly = full fft.
    // Use input [(1,2),(3,4)], expected output (4,6),(-2,-2).
    let mut data = vec![c(1.0, 2.0), c(3.0, 4.0)];
    fft_raw(&mut data, 1);
    assert_eq!(data[0].real, 4.0);
    assert_eq!(data[0].imag, 6.0);
    assert_eq!(data[1].real, -2.0);
    assert_eq!(data[1].imag, -2.0);
}

#[test]
fn test_fft_raw_matches_fft_after_rader() {
    // For arbitrary input, after rader bit-reversal, fft_raw should produce the
    // same result as full fft.
    let input = vec![
        c(1.5, 2.5),
        c(-3.25, 0.125),
        c(0.0, -1.0),
        c(7.5, 8.25),
        c(-0.5, 0.0),
        c(1.0, 1.0),
        c(2.0, -2.0),
        c(3.5, 4.5),
    ];

    // Path 1: full fft
    let mut out1 = vec![c(0.0, 0.0); 8];
    fft(&input, &mut out1, 3);

    // Path 2: rader then fft_raw
    let mut out2 = vec![c(0.0, 0.0); 8];
    rader(&input, &mut out2, 3);
    fft_raw(&mut out2, 3);

    for i in 0..8 {
        assert_eq!(out1[i].real.to_bits(), out2[i].real.to_bits());
        assert_eq!(out1[i].imag.to_bits(), out2[i].imag.to_bits());
    }
}

#[test]
fn test_fft_raw_logsize3_via_inplace_path() {
    // Verify fft_raw follows the C definition: rader_inplace + fft_raw == fft_inplace
    let mut buf = vec![
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
        c(1.0, 0.0),
        c(-1.0, 0.0),
    ];
    rader_inplace(&mut buf, 3);
    fft_raw(&mut buf, 3);
    let expected = vec![
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(8.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
        c(0.0, 0.0),
    ];
    assert_arr_close(&buf, &expected, 1e-5);
}

// ----------------------------------------------------------------------------
// Large-size FFT (size 32) - verify dynamic-step butterfly path
// ----------------------------------------------------------------------------

#[test]
fn test_fft_size32_impulse() {
    // For an impulse input of size 32, output should be all 1s.
    let mut input = vec![c(0.0, 0.0); 32];
    input[0] = c(1.0, 0.0);
    let mut output = vec![c(0.0, 0.0); 32];
    fft(&input, &mut output, 5);
    for v in &output {
        assert!(approx(v.real, 1.0, 1e-5));
        assert!(approx(v.imag, 0.0, 1e-5));
    }
}

#[test]
fn test_fft_size32_dc() {
    // For DC input (all 1.0) of size 32, output[0] = 32, rest = 0.
    let input = vec![c(1.0, 0.0); 32];
    let mut output = vec![c(0.0, 0.0); 32];
    fft(&input, &mut output, 5);
    assert!(approx(output[0].real, 32.0, 1e-4));
    assert!(approx(output[0].imag, 0.0, 1e-4));
    for i in 1..32 {
        assert!(approx(output[i].real, 0.0, 1e-3));
        assert!(approx(output[i].imag, 0.0, 1e-3));
    }
}

fn main() {}
