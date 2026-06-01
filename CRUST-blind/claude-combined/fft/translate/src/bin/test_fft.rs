use fft::fft::{
    fft, fft_clz, fft_inplace, fft_raw, next_reversed_n, rader, rader_inplace, FftComplex,
};

fn main() {}

fn assert_complex_eq(expected: FftComplex, actual: FftComplex, idx: usize, label: &str) {
    assert_eq!(
        actual.real.to_bits(),
        expected.real.to_bits(),
        "{}: real mismatch at index {}: expected {:#010x} ({}), got {:#010x} ({})",
        label,
        idx,
        expected.real.to_bits(),
        expected.real,
        actual.real.to_bits(),
        actual.real
    );
    assert_eq!(
        actual.imag.to_bits(),
        expected.imag.to_bits(),
        "{}: imag mismatch at index {}: expected {:#010x} ({}), got {:#010x} ({})",
        label,
        idx,
        expected.imag.to_bits(),
        expected.imag,
        actual.imag.to_bits(),
        actual.imag
    );
}

fn assert_arr_eq(expected: &[FftComplex], actual: &[FftComplex], label: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{}: length mismatch",
        label
    );
    for i in 0..expected.len() {
        assert_complex_eq(expected[i], actual[i], i, label);
    }
}

#[test]
fn test_fft_clz_basics() {
    // Compare with the equivalent of leading_zeros for a 64-bit usize.
    assert_eq!(fft_clz(1), (usize::BITS - 1) as usize);
    assert_eq!(fft_clz(2), (usize::BITS - 2) as usize);
    assert_eq!(fft_clz(3), (usize::BITS - 2) as usize);
    assert_eq!(fft_clz(4), (usize::BITS - 3) as usize);
    assert_eq!(fft_clz(7), (usize::BITS - 3) as usize);
    assert_eq!(fft_clz(8), (usize::BITS - 4) as usize);
    assert_eq!(fft_clz(usize::MAX), 0);
    // Note: C's __builtin_clz(0) is undefined; the Rust version uses
    // leading_zeros which yields the bit width for 0.
    assert_eq!(fft_clz(0), usize::BITS as usize);
}

#[test]
fn test_next_reversed_n_logsize3() {
    // For logsize = 3, shift = BITS - 3.
    let bits = usize::BITS as usize;
    let shift = bits - 3;
    let mut r = 0usize;
    let expected = [4usize, 2, 6, 1, 5, 3, 7, 0];
    for &want in &expected {
        r = next_reversed_n(r, shift);
        assert_eq!(r, want, "next_reversed_n logsize=3 failed");
    }
}

#[test]
fn test_next_reversed_n_logsize2() {
    let bits = usize::BITS as usize;
    let shift = bits - 2;
    let mut r = 0usize;
    let expected = [2usize, 1, 3, 0];
    for &want in &expected {
        r = next_reversed_n(r, shift);
        assert_eq!(r, want, "next_reversed_n logsize=2 failed");
    }
}

#[test]
fn test_next_reversed_n_logsize4() {
    // bit-reverse permutation order for logsize=4
    let bits = usize::BITS as usize;
    let shift = bits - 4;
    let mut r = 0usize;
    let expected = [
        8usize, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15, 0,
    ];
    for &want in &expected {
        r = next_reversed_n(r, shift);
        assert_eq!(r, want, "next_reversed_n logsize=4 failed");
    }
}

#[test]
fn test_rader_oop_size8() {
    let mut input = [FftComplex { real: 0.0, imag: 0.0 }; 8];
    for i in 0..8 {
        input[i].real = i as f32;
        input[i].imag = 0.0;
    }
    let mut output = [FftComplex { real: 0.0, imag: 0.0 }; 8];
    rader(&input, &mut output, 3);
    let expected = [
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) }, // 0
        FftComplex { real: f32::from_bits(0x40800000), imag: f32::from_bits(0x00000000) }, // 4
        FftComplex { real: f32::from_bits(0x40000000), imag: f32::from_bits(0x00000000) }, // 2
        FftComplex { real: f32::from_bits(0x40c00000), imag: f32::from_bits(0x00000000) }, // 6
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) }, // 1
        FftComplex { real: f32::from_bits(0x40a00000), imag: f32::from_bits(0x00000000) }, // 5
        FftComplex { real: f32::from_bits(0x40400000), imag: f32::from_bits(0x00000000) }, // 3
        FftComplex { real: f32::from_bits(0x40e00000), imag: f32::from_bits(0x00000000) }, // 7
    ];
    assert_arr_eq(&expected, &output, "rader_oop_size8");
}

#[test]
fn test_rader_inplace_size8() {
    let mut data = [FftComplex { real: 0.0, imag: 0.0 }; 8];
    for i in 0..8 {
        data[i].real = i as f32;
        data[i].imag = 0.0;
    }
    rader_inplace(&mut data, 3);
    // bit-reverse permutation: identical to rader_oop above
    let expected = [
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x40800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x40000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x40c00000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x40a00000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x40400000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x40e00000), imag: f32::from_bits(0x00000000) },
    ];
    assert_arr_eq(&expected, &data, "rader_inplace_size8");
}

#[test]
fn test_fft_raw_size8_already_reversed() {
    // Apply fft_raw to an already bit-reversed-input set [0..7]; this is
    // equivalent to applying fft to [0,4,2,6,1,5,3,7].
    let mut data = [
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: 2.0, imag: 0.0 },
        FftComplex { real: 3.0, imag: 0.0 },
        FftComplex { real: 4.0, imag: 0.0 },
        FftComplex { real: 5.0, imag: 0.0 },
        FftComplex { real: 6.0, imag: 0.0 },
        FftComplex { real: 7.0, imag: 0.0 },
    ];
    fft_raw(&mut data, 3);
    let expected = [
        FftComplex { real: f32::from_bits(0x41e00000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xbf800000), imag: f32::from_bits(0x401a827a) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0x407fffff) },
        FftComplex { real: f32::from_bits(0xbf800000), imag: f32::from_bits(0x3ed413c8) },
        FftComplex { real: f32::from_bits(0xc1800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xbf800000), imag: f32::from_bits(0xbed413cc) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0xc07fffff) },
        FftComplex { real: f32::from_bits(0xbf800000), imag: f32::from_bits(0xc01a8279) },
    ];
    assert_arr_eq(&expected, &data, "fft_raw_size8");
}

#[test]
fn test_fft_inplace_alternating_8() {
    // Original C test case in c_src/tests/test.c
    let mut data = [
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
    ];
    fft_inplace(&mut data, 3);
    let expected = [
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x41000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x00000000) },
    ];
    assert_arr_eq(&expected, &data, "fft_inplace_alternating_8");
}

#[test]
fn test_fft_inplace_impulse_8() {
    let mut data = [FftComplex { real: 0.0, imag: 0.0 }; 8];
    data[0].real = 1.0;
    fft_inplace(&mut data, 3);
    let expected = [
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
    ];
    assert_arr_eq(&expected, &data, "fft_inplace_impulse_8");
}

#[test]
fn test_fft_inplace_shift_8() {
    let mut data = [FftComplex { real: 0.0, imag: 0.0 }; 8];
    data[1].real = 1.0;
    fft_inplace(&mut data, 3);
    let expected = [
        FftComplex { real: f32::from_bits(0x3f800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0x3f3504f3), imag: f32::from_bits(0xbf3504f3) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0xbf7fffff) },
        FftComplex { real: f32::from_bits(0xbf3504f2), imag: f32::from_bits(0xbf3504f2) },
        FftComplex { real: f32::from_bits(0xbf800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xbf3504f3), imag: f32::from_bits(0x3f3504f3) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x3f7fffff) },
        FftComplex { real: f32::from_bits(0x3f3504f2), imag: f32::from_bits(0x3f3504f2) },
    ];
    assert_arr_eq(&expected, &data, "fft_inplace_shift_8");
}

#[test]
fn test_fft_oop_ramp_8() {
    let input = [
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: 2.0, imag: 0.0 },
        FftComplex { real: 3.0, imag: 0.0 },
        FftComplex { real: 4.0, imag: 0.0 },
        FftComplex { real: 5.0, imag: 0.0 },
        FftComplex { real: 6.0, imag: 0.0 },
        FftComplex { real: 7.0, imag: 0.0 },
        FftComplex { real: 8.0, imag: 0.0 },
    ];
    let mut output = [FftComplex { real: 0.0, imag: 0.0 }; 8];
    fft(&input, &mut output, 3);
    let expected = [
        FftComplex { real: f32::from_bits(0x42100000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0x411a827a) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0x407fffff) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0x3fd413c8) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0xbfd413cc) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0xc07fffff) },
        FftComplex { real: f32::from_bits(0xc0800000), imag: f32::from_bits(0xc11a8279) },
    ];
    assert_arr_eq(&expected, &output, "fft_oop_ramp_8");
}

#[test]
fn test_fft_inplace_size4_ramp() {
    let mut data = [
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: 2.0, imag: 0.0 },
        FftComplex { real: 3.0, imag: 0.0 },
        FftComplex { real: 4.0, imag: 0.0 },
    ];
    fft_inplace(&mut data, 2);
    let expected = [
        FftComplex { real: f32::from_bits(0x41200000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xc0000000), imag: f32::from_bits(0x40000000) },
        FftComplex { real: f32::from_bits(0xc0000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xc0000000), imag: f32::from_bits(0xc0000000) },
    ];
    assert_arr_eq(&expected, &data, "fft_inplace_size4_ramp");
}

#[test]
fn test_fft_inplace_size2() {
    let mut data = [
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: 2.0, imag: 0.0 },
    ];
    fft_inplace(&mut data, 1);
    let expected = [
        FftComplex { real: f32::from_bits(0x40400000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xbf800000), imag: f32::from_bits(0x00000000) },
    ];
    assert_arr_eq(&expected, &data, "fft_inplace_size2");
}

#[test]
fn test_fft_inplace_size1_noop() {
    // logsize == 0: the function should be a no-op.
    let mut data = [FftComplex { real: 7.0, imag: 5.0 }];
    fft_inplace(&mut data, 0);
    let expected = [FftComplex {
        real: f32::from_bits(0x40e00000),
        imag: f32::from_bits(0x40a00000),
    }];
    assert_arr_eq(&expected, &data, "fft_inplace_size1_noop");
}

#[test]
fn test_fft_inplace_size16_ramp() {
    let mut data = [FftComplex { real: 0.0, imag: 0.0 }; 16];
    for i in 0..16 {
        data[i].real = (i + 1) as f32;
        data[i].imag = 0.0;
    }
    fft_inplace(&mut data, 4);
    let expected = [
        FftComplex { real: f32::from_bits(0x43080000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xc0ffffff), imag: f32::from_bits(0x4220dff8) },
        FftComplex { real: f32::from_bits(0xc1000000), imag: f32::from_bits(0x419a8279) },
        FftComplex { real: f32::from_bits(0xc1000001), imag: f32::from_bits(0x413f90c5) },
        FftComplex { real: f32::from_bits(0xc1000000), imag: f32::from_bits(0x40ffffff) },
        FftComplex { real: f32::from_bits(0xc0ffffff), imag: f32::from_bits(0x40ab0dc0) },
        FftComplex { real: f32::from_bits(0xc0ffffff), imag: f32::from_bits(0x405413ca) },
        FftComplex { real: f32::from_bits(0xc0ffffff), imag: f32::from_bits(0x3fcbaf90) },
        FftComplex { real: f32::from_bits(0xc1000000), imag: f32::from_bits(0x00000000) },
        FftComplex { real: f32::from_bits(0xc1000000), imag: f32::from_bits(0xbfcbafb0) },
        FftComplex { real: f32::from_bits(0xc0ffffff), imag: f32::from_bits(0xc05413ca) },
        FftComplex { real: f32::from_bits(0xc0fffffe), imag: f32::from_bits(0xc0ab0dc2) },
        FftComplex { real: f32::from_bits(0xc1000000), imag: f32::from_bits(0xc0ffffff) },
        FftComplex { real: f32::from_bits(0xc1000000), imag: f32::from_bits(0xc13f90c6) },
        FftComplex { real: f32::from_bits(0xc1000000), imag: f32::from_bits(0xc19a8279) },
        FftComplex { real: f32::from_bits(0xc1000000), imag: f32::from_bits(0xc220dff6) },
    ];
    assert_arr_eq(&expected, &data, "fft_inplace_size16_ramp");
}

#[test]
fn test_fft_inplace_complex_input_size4() {
    let mut data = [
        FftComplex { real: 1.0, imag: 1.0 },
        FftComplex { real: 2.0, imag: -1.0 },
        FftComplex { real: 0.0, imag: 3.0 },
        FftComplex { real: -1.0, imag: -2.0 },
    ];
    fft_inplace(&mut data, 2);
    let expected = [
        FftComplex { real: f32::from_bits(0x40000000), imag: f32::from_bits(0x3f800000) },
        FftComplex { real: f32::from_bits(0x40000000), imag: f32::from_bits(0xc0a00000) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x40e00000) },
        FftComplex { real: f32::from_bits(0x00000000), imag: f32::from_bits(0x3f800000) },
    ];
    assert_arr_eq(&expected, &data, "fft_inplace_complex_input_size4");
}

#[test]
fn test_fft_dc_value_property() {
    // The DC bin of an FFT is the sum of all input samples.
    let mut data = [FftComplex { real: 0.0, imag: 0.0 }; 8];
    let mut sum_r = 0.0f32;
    let mut sum_i = 0.0f32;
    for i in 0..8 {
        data[i].real = (3 * i as i32 - 5) as f32;
        data[i].imag = (i as f32).sin();
        sum_r += data[i].real;
        sum_i += data[i].imag;
    }
    fft_inplace(&mut data, 3);
    // Allow tiny floating-point error in the property test.
    assert!(
        (data[0].real - sum_r).abs() < 1e-4,
        "DC real expected {}, got {}",
        sum_r,
        data[0].real
    );
    assert!(
        (data[0].imag - sum_i).abs() < 1e-4,
        "DC imag expected {}, got {}",
        sum_i,
        data[0].imag
    );
}
