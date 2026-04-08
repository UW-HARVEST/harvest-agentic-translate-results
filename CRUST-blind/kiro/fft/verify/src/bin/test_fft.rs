use fft::fft::{
    fft_clz, next_reversed_n, rader, rader_inplace, fft, fft_inplace, fft_raw, FftComplex,
};

fn c(real: f32, imag: f32) -> FftComplex {
    FftComplex { real, imag }
}

fn assert_complex_eq(a: &FftComplex, real: f32, imag: f32, eps: f32, label: &str) {
    assert!(
        (a.real - real).abs() <= eps && (a.imag - imag).abs() <= eps,
        "{}: got ({}, {}), expected ({}, {})",
        label, a.real, a.imag, real, imag
    );
}

const INTBITS: usize = std::mem::size_of::<usize>() * 8;

// --- fft_clz ---

#[test]
fn test_fft_clz() {
    assert_eq!(fft_clz(1), 63);
    assert_eq!(fft_clz(2), 62);
    assert_eq!(fft_clz(3), 62);
    assert_eq!(fft_clz(4), 61);
    assert_eq!(fft_clz(7), 61);
    assert_eq!(fft_clz(8), 60);
    assert_eq!(fft_clz(16), 59);
    assert_eq!(fft_clz(255), 56);
    assert_eq!(fft_clz(256), 55);
    assert_eq!(fft_clz(1024), 53);
}

// --- next_reversed_n ---

#[test]
fn test_next_reversed_n_logsize3() {
    let shift = INTBITS - 3;
    let expected = [0, 4, 2, 6, 1, 5, 3, 7];
    let mut rev = 0usize;
    for n in 0..8 {
        assert_eq!(rev, expected[n], "logsize=3 n={}", n);
        rev = next_reversed_n(rev, shift);
    }
}

#[test]
fn test_next_reversed_n_logsize2() {
    let shift = INTBITS - 2;
    let expected = [0, 2, 1, 3];
    let mut rev = 0usize;
    for n in 0..4 {
        assert_eq!(rev, expected[n], "logsize=2 n={}", n);
        rev = next_reversed_n(rev, shift);
    }
}

#[test]
fn test_next_reversed_n_logsize4() {
    let shift = INTBITS - 4;
    let expected = [0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15];
    let mut rev = 0usize;
    for n in 0..16 {
        assert_eq!(rev, expected[n], "logsize=4 n={}", n);
        rev = next_reversed_n(rev, shift);
    }
}

// --- rader (out-of-place bit-reversal) ---

#[test]
fn test_rader_logsize3() {
    let input: Vec<FftComplex> = (0..8).map(|i| c(i as f32, 0.0)).collect();
    let mut output = vec![c(0.0, 0.0); 8];
    rader(&input, &mut output, 3);
    let expected_real = [0.0, 4.0, 2.0, 6.0, 1.0, 5.0, 3.0, 7.0];
    for i in 0..8 {
        assert_complex_eq(&output[i], expected_real[i], 0.0, 0.0, &format!("rader[{}]", i));
    }
}

// --- rader_inplace ---

#[test]
fn test_rader_inplace_logsize3() {
    let mut data: Vec<FftComplex> = (0..8).map(|i| c(i as f32, 0.0)).collect();
    rader_inplace(&mut data, 3);
    let expected_real = [0.0, 4.0, 2.0, 6.0, 1.0, 5.0, 3.0, 7.0];
    for i in 0..8 {
        assert_complex_eq(&data[i], expected_real[i], 0.0, 0.0, &format!("rader_inplace[{}]", i));
    }
}

#[test]
fn test_rader_inplace_logsize1() {
    let mut data = vec![c(10.0, 0.0), c(20.0, 0.0)];
    rader_inplace(&mut data, 1);
    // logsize=1: size=2, loop is n=1..0 which doesn't execute, so no swaps
    assert_complex_eq(&data[0], 10.0, 0.0, 0.0, "ri[0]");
    assert_complex_eq(&data[1], 20.0, 0.0, 0.0, "ri[1]");
}

// --- fft_raw ---

#[test]
fn test_fft_raw_logsize0() {
    let mut data = vec![c(42.0, 7.0)];
    fft_raw(&mut data, 0);
    assert_complex_eq(&data[0], 42.0, 7.0, 0.0, "fft_raw logsize=0");
}

#[test]
fn test_fft_raw_logsize1() {
    // After rader on [3+1i, 5+2i] with logsize=1, rader is identity (reversed=[0,1])
    // So fft_raw on [3+1i, 5+2i] should give [8+3i, -2-1i]
    let mut data = vec![c(3.0, 1.0), c(5.0, 2.0)];
    fft_raw(&mut data, 1);
    assert_complex_eq(&data[0], 8.0, 3.0, 0.0, "fft_raw[0]");
    assert_complex_eq(&data[1], -2.0, -1.0, 0.0, "fft_raw[1]");
}

// --- fft (out-of-place) ---

#[test]
fn test_fft_logsize0() {
    let input = vec![c(42.0, 7.0)];
    let mut output = vec![c(0.0, 0.0)];
    fft(&input, &mut output, 0);
    assert_complex_eq(&output[0], 42.0, 7.0, 0.0, "fft logsize=0");
}

#[test]
fn test_fft_logsize1() {
    let input = vec![c(3.0, 1.0), c(5.0, 2.0)];
    let mut output = vec![c(0.0, 0.0); 2];
    fft(&input, &mut output, 1);
    assert_complex_eq(&output[0], 8.0, 3.0, 0.0, "fft[0]");
    assert_complex_eq(&output[1], -2.0, -1.0, 0.0, "fft[1]");
}

#[test]
fn test_fft_logsize2_all_ones() {
    let input = vec![c(1.0, 0.0); 4];
    let mut output = vec![c(0.0, 0.0); 4];
    fft(&input, &mut output, 2);
    assert_complex_eq(&output[0], 4.0, 0.0, 1e-6, "fft[0]");
    assert_complex_eq(&output[1], 0.0, 0.0, 1e-6, "fft[1]");
    assert_complex_eq(&output[2], 0.0, 0.0, 1e-6, "fft[2]");
    assert_complex_eq(&output[3], 0.0, 0.0, 1e-6, "fft[3]");
}

#[test]
fn test_fft_logsize2_complex_input() {
    let input = vec![c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0), c(7.0, 8.0)];
    let mut output = vec![c(0.0, 0.0); 4];
    fft(&input, &mut output, 2);
    let eps = 1e-5;
    assert_complex_eq(&output[0], 16.0, 20.0, eps, "fft[0]");
    assert_complex_eq(&output[1], -8.0, 0.0, eps, "fft[1]");
    assert_complex_eq(&output[2], -4.0, -4.0, eps, "fft[2]");
    assert_complex_eq(&output[3], 0.0, -8.0, eps, "fft[3]");
}

#[test]
fn test_fft_logsize3_alternating() {
    let input: Vec<FftComplex> = (0..8).map(|i| c(if i % 2 == 0 { 1.0 } else { -1.0 }, 0.0)).collect();
    let mut output = vec![c(0.0, 0.0); 8];
    fft(&input, &mut output, 3);
    let eps = 1e-5;
    for i in 0..8 {
        let (er, ei) = if i == 4 { (8.0, 0.0) } else { (0.0, 0.0) };
        assert_complex_eq(&output[i], er, ei, eps, &format!("fft_alt[{}]", i));
    }
}

#[test]
fn test_fft_logsize3_impulse() {
    let mut input = vec![c(0.0, 0.0); 8];
    input[0] = c(1.0, 0.0);
    let mut output = vec![c(0.0, 0.0); 8];
    fft(&input, &mut output, 3);
    for i in 0..8 {
        assert_complex_eq(&output[i], 1.0, 0.0, 1e-6, &format!("fft_imp[{}]", i));
    }
}

#[test]
fn test_fft_logsize4() {
    let input: Vec<FftComplex> = (0..16).map(|i| c(i as f32, 0.0)).collect();
    let mut output = vec![c(0.0, 0.0); 16];
    fft(&input, &mut output, 4);
    let expected: [(f32, f32); 16] = [
        (120.0, 0.0),
        (-8.0, 40.218719),
        (-8.0, 19.313707),
        (-8.000001, 11.972844),
        (-8.0, 8.0),
        (-8.0, 5.345428),
        (-8.0, 3.313708),
        (-8.0, 1.591295),
        (-8.0, 0.0),
        (-8.0, -1.591299),
        (-8.0, -3.313708),
        (-7.999999, -5.345429),
        (-8.0, -8.0),
        (-8.0, -11.972845),
        (-8.0, -19.313707),
        (-8.0, -40.218712),
    ];
    let eps = 1e-4;
    for i in 0..16 {
        assert_complex_eq(&output[i], expected[i].0, expected[i].1, eps, &format!("fft16[{}]", i));
    }
}

// --- fft_inplace ---

#[test]
fn test_fft_inplace_logsize3_alternating() {
    let mut data: Vec<FftComplex> = (0..8).map(|i| c(if i % 2 == 0 { 1.0 } else { -1.0 }, 0.0)).collect();
    fft_inplace(&mut data, 3);
    let eps = 1e-5;
    for i in 0..8 {
        let (er, ei) = if i == 4 { (8.0, 0.0) } else { (0.0, 0.0) };
        assert_complex_eq(&data[i], er, ei, eps, &format!("fft_ip[{}]", i));
    }
}

#[test]
fn test_fft_inplace_logsize1() {
    let mut data = vec![c(3.0, 1.0), c(5.0, 2.0)];
    fft_inplace(&mut data, 1);
    assert_complex_eq(&data[0], 8.0, 3.0, 0.0, "fft_ip[0]");
    assert_complex_eq(&data[1], -2.0, -1.0, 0.0, "fft_ip[1]");
}

#[test]
fn test_fft_inplace_logsize4() {
    let mut data: Vec<FftComplex> = (0..16).map(|i| c(i as f32, 0.0)).collect();
    fft_inplace(&mut data, 4);
    let expected: [(f32, f32); 16] = [
        (120.0, 0.0),
        (-8.0, 40.218719),
        (-8.0, 19.313707),
        (-8.000001, 11.972844),
        (-8.0, 8.0),
        (-8.0, 5.345428),
        (-8.0, 3.313708),
        (-8.0, 1.591295),
        (-8.0, 0.0),
        (-8.0, -1.591299),
        (-8.0, -3.313708),
        (-7.999999, -5.345429),
        (-8.0, -8.0),
        (-8.0, -11.972845),
        (-8.0, -19.313707),
        (-8.0, -40.218712),
    ];
    let eps = 1e-4;
    for i in 0..16 {
        assert_complex_eq(&data[i], expected[i].0, expected[i].1, eps, &format!("fft_ip16[{}]", i));
    }
}

// --- fft and fft_inplace produce identical results ---

#[test]
fn test_fft_vs_fft_inplace_logsize3() {
    let input: Vec<FftComplex> = (0..8).map(|i| c((i as f32) * 1.5 + 0.3, (i as f32) * 0.7 - 1.0)).collect();
    let mut out_fft = vec![c(0.0, 0.0); 8];
    fft(&input, &mut out_fft, 3);
    let mut inplace = input.clone();
    fft_inplace(&mut inplace, 3);
    for i in 0..8 {
        assert_complex_eq(&inplace[i], out_fft[i].real, out_fft[i].imag, 1e-6, &format!("cmp[{}]", i));
    }
}

fn main() {}
