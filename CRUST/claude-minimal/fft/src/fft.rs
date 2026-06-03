// Import necessary modules
use crate::{fft};
// Define the fft_complex struct
#[derive(Debug, Clone, Copy)]
pub struct FftComplex {
    pub real: f32,
    pub imag: f32,
}
impl FftComplex {
    fn add(&self, other: &FftComplex) -> FftComplex {
        FftComplex {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }
    fn sub(&self, other: &FftComplex) -> FftComplex {
        FftComplex {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }
    fn mul(&self, other: &FftComplex) -> FftComplex {
        FftComplex {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }
    fn self_mul(&mut self, other: &FftComplex) {
        let r = self.real * other.real - self.imag * other.imag;
        let i = self.real * other.imag + self.imag * other.real;
        self.real = r;
        self.imag = i;
    }
    fn copy(&mut self, other: &FftComplex) {
        *self = *other;
    }
    fn swap(array: &mut [FftComplex], a: usize, b: usize) {
        array.swap(a, b);
    }
    fn set_one(&mut self) {
        self.real = 1.0;
        self.imag = 0.0;
    }
    fn unit_root_recip(&mut self, n: usize) {
        // Match C behavior: cos/sin computed in double precision then cast to float.
        let angle = 2.0_f64 * std::f64::consts::PI / (n as f64);
        self.real = angle.cos() as f32;
        self.imag = -angle.sin() as f32;
    }
}
// Function Declarations
/// Returns the number of leading zeros in the binary representation of `n`.
pub fn fft_clz(n: usize) -> usize {
    n.leading_zeros() as usize
}
/// Computes the next reversed number given `reversed_n` and `shift`.
pub fn next_reversed_n(reversed_n: usize, shift: usize) -> usize {
    let mut reversed_n = reversed_n << shift;
    let count_leading_ones = fft_clz(!reversed_n);
    reversed_n <<= count_leading_ones; // remove leading ones
    reversed_n |= 1usize << (usize::BITS as usize - 1);
    reversed_n >>= shift + count_leading_ones;
    reversed_n
}
/// Performs the Fast Fourier Transform on the input array `x` and stores the result in `X`.
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}
/// Performs the Rader's algorithm on the input array `array` and stores the result in `target`.
pub fn rader(array: &[FftComplex], target: &mut [FftComplex], logsize: usize) {
    let size = 1usize << logsize;
    if logsize == 0 {
        if size > 0 {
            target[0] = array[0];
        }
        return;
    }
    // how many bits should be shifted to move the number to the most significant bit
    let shift = (usize::BITS as usize) - logsize;
    let mut reversed_n = 0usize;
    for n in 0..size {
        target[reversed_n] = array[n];
        // Avoid invoking next_reversed_n on the all-ones value (last n), which
        // would otherwise trigger a shift by usize::BITS.
        if n + 1 < size {
            reversed_n = next_reversed_n(reversed_n, shift);
        }
    }
}
/// Performs the raw FFT on the input array `x`.
pub fn fft_raw(x: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    let size = 1usize << logsize;
    let mut step = 2usize;
    while step <= size {
        let mut unit = FftComplex { real: 0.0, imag: 0.0 };
        unit.unit_root_recip(step);
        let half = step / 2;
        let mut p = 0usize;
        while p < size {
            // i == 0, j == half
            let t = x[p + half];
            let u = x[p];
            x[p] = u.add(&t);
            x[p + half] = u.sub(&t);
            if half > 1 {
                // i == 1, j == half + 1
                let mut root = unit;
                let t = root.mul(&x[p + half + 1]);
                let u = x[p + 1];
                x[p + 1] = u.add(&t);
                x[p + half + 1] = u.sub(&t);
                let mut i = 2usize;
                while i < half {
                    let j = half + i;
                    root.self_mul(&unit);
                    let t = root.mul(&x[p + j]);
                    let u = x[p + i];
                    x[p + i] = u.add(&t);
                    x[p + j] = u.sub(&t);
                    i += 1;
                }
            }
            p += step;
        }
        step *= 2;
    }
}
/// Performs the in-place FFT on the input array `x`.
pub fn fft_inplace(x: &mut [FftComplex], logsize: usize) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
/// Performs the in-place Rader's algorithm on the input array `array`.
pub fn rader_inplace(array: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    let size = 1usize << logsize;
    let shift = (usize::BITS as usize) - logsize;
    // nothing should be done for 0 and 0b111...11(size - 1).
    let mut reversed_n = size >> 1;
    let mut n = 1usize;
    while n < size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
        n += 1;
    }
}
