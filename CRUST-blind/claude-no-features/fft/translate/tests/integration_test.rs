use fft::fft::{fft_inplace, FftComplex};

#[test]
fn test_simple_fft() {
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
    let expected = [0.0f32, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0];
    fft_inplace(&mut data, 3);
    for (i, c) in data.iter().enumerate() {
        assert!(
            (c.real - expected[i]).abs() < 1e-4,
            "mismatch at index {}: got {} expected {}",
            i,
            c.real,
            expected[i]
        );
    }
}
