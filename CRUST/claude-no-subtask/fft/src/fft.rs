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
        self.real = other.real;
        self.imag = other.imag;
    }
    fn swap(array: &mut [FftComplex], a: usize, b: usize) {
        array.swap(a, b);
    }
    fn set_one(&mut self) {
        self.real = 1.0;
        self.imag = 0.0;
    }
    fn unit_root_recip(&mut self, n: usize) {
        let theta = 2.0_f64 * std::f64::consts::PI / (n as f64);
        self.real = theta.cos() as f32;
        self.imag = -(theta.sin() as f32);
    }
}
// Function Declarations
/// Returns the number of leading zeros in the binary representation of `n`.
pub fn fft_clz(n: usize) -> usize {
    n.leading_zeros() as usize
}
/// Computes the next reversed number given `reversed_n` and `shift`.
pub fn next_reversed_n(reversed_n: usize, shift: usize) -> usize {
    let bits = usize::BITS as usize;
    let mut reversed_n = reversed_n << shift;
    let count_leading_ones = fft_clz(!reversed_n);
    // Remove leading ones. Shift may equal `bits` only if reversed_n was all
    // ones, which only occurs after the final useful index; guard against that
    // to avoid Rust's overflow panic on full-width shifts.
    if count_leading_ones >= bits {
        // All bits are ones; the next value (after wrap) is conventionally 0.
        return 0;
    }
    reversed_n <<= count_leading_ones;
    reversed_n |= 1usize << (bits - 1);
    let total_shift = shift + count_leading_ones;
    if total_shift >= bits {
        return 0;
    }
    reversed_n >> total_shift
}
/// Performs the Fast Fourier Transform on the input array `x` and stores the result in `X`.
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}
/// Performs the Rader's algorithm on the input array `array` and stores the result in `target`.
pub fn rader(array: &[FftComplex], target: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        if !array.is_empty() && !target.is_empty() {
            target[0] = array[0];
        }
        return;
    }
    let size = 1usize << logsize;
    let shift = (usize::BITS as usize) - logsize;
    let mut reversed_n: usize = 0;
    for n in 0..size {
        target[reversed_n] = array[n];
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

    do_butterfly(x, size, 2);

    if logsize == 1 {
        return;
    }

    do_butterfly(x, size, 4);

    if logsize == 2 {
        return;
    }

    let mut step = 8usize;
    while step <= size {
        do_butterfly(x, size, step);
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
    if logsize <= 1 {
        return;
    }
    let size = 1usize << logsize;
    let shift = (usize::BITS as usize) - logsize;
    let mut reversed_n: usize = size >> 1;
    let mut n: usize = 1;
    while n < size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
        n += 1;
    }
}

/// Performs a single butterfly stage with the given `step`.
fn do_butterfly(x: &mut [FftComplex], size: usize, step: usize) {
    let mut unit = FftComplex { real: 0.0, imag: 0.0 };
    unit.unit_root_recip(step);
    let half = step / 2;

    let mut base = 0usize;
    while base < size {
        // i == 0, j == half
        let t = x[base + half];
        let u = x[base];
        x[base] = u.add(&t);
        x[base + half] = u.sub(&t);

        if half > 1 {
            // i == 1, j == half + 1
            let mut root = unit;
            let t = root.mul(&x[base + half + 1]);
            let u = x[base + 1];
            x[base + 1] = u.add(&t);
            x[base + half + 1] = u.sub(&t);

            let mut i = 2usize;
            let mut j = half + 2;
            while i < half {
                root.self_mul(&unit);
                let t = root.mul(&x[base + j]);
                let u = x[base + i];
                x[base + i] = u.add(&t);
                x[base + j] = u.sub(&t);
                i += 1;
                j += 1;
            }
        }

        base += step;
    }
}
