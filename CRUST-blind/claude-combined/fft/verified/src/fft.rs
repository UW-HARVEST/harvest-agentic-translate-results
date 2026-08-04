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
        // Use double precision for the trig computation, matching the C code's
        // use of cos/sin (which return double) before truncation to float.
        let theta = 2.0_f64 * std::f64::consts::PI / (n as f64);
        self.real = theta.cos() as f32;
        self.imag = -theta.sin() as f32;
    }
}

// Helper: shift left by n that returns 0 if n >= bit-width.
#[inline]
fn shl_saturating(x: usize, n: usize) -> usize {
    if n >= usize::BITS as usize {
        0
    } else {
        x << n
    }
}

// Helper: shift right by n that returns 0 if n >= bit-width.
#[inline]
fn shr_saturating(x: usize, n: usize) -> usize {
    if n >= usize::BITS as usize {
        0
    } else {
        x >> n
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
    let mut reversed_n = shl_saturating(reversed_n, shift);
    let count_leading_ones = fft_clz(!reversed_n);
    reversed_n = shl_saturating(reversed_n, count_leading_ones); // remove leading ones
    reversed_n |= 1usize << (bits - 1);
    shr_saturating(reversed_n, shift + count_leading_ones)
}
/// Performs the Fast Fourier Transform on the input array `x` and stores the result in `X`.
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}
/// Performs the Rader's algorithm on the input array `array` and stores the result in `target`.
pub fn rader(array: &[FftComplex], target: &mut [FftComplex], logsize: usize) {
    let size = 1usize << logsize;
    let bits = usize::BITS as usize;
    let shift = bits - logsize;
    let mut reversed_n: usize = 0;
    for n in 0..size {
        let mut t = target[reversed_n];
        t.copy(&array[n]);
        target[reversed_n] = t;
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

// Internal helper: perform a single butterfly stage with the given step size.
fn do_butterfly(arr: &mut [FftComplex], step: usize) {
    let mut unit = FftComplex { real: 0.0, imag: 0.0 };
    unit.unit_root_recip(step);
    let half = step / 2;
    let n = arr.len();
    let mut p = 0usize;
    while p < n {
        // i == 0, j == half
        let t = arr[p + half];
        let u = arr[p];
        arr[p] = u.add(&t);
        arr[p + half] = u.sub(&t);
        if half > 1 {
            // i == 1, j == half + 1
            let mut root = unit;
            let t1 = root.mul(&arr[p + half + 1]);
            let u1 = arr[p + 1];
            arr[p + 1] = u1.add(&t1);
            arr[p + half + 1] = u1.sub(&t1);
            // i in [2, half), j = half + i
            let mut i = 2usize;
            while i < half {
                let j = half + i;
                root.self_mul(&unit);
                let t2 = root.mul(&arr[p + j]);
                let u2 = arr[p + i];
                arr[p + i] = u2.add(&t2);
                arr[p + j] = u2.sub(&t2);
                i += 1;
            }
        }
        p += step;
    }
}

/// Performs the raw FFT on the input array `x`.
pub fn fft_raw(x: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    do_butterfly(x, 2);
    if logsize == 1 {
        return;
    }
    do_butterfly(x, 4);
    if logsize == 2 {
        return;
    }
    let max = 1usize << logsize;
    let mut step = 8usize;
    while step <= max {
        do_butterfly(x, step);
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
    let size = 1usize << logsize;
    if size < 2 {
        return;
    }
    let bits = usize::BITS as usize;
    let shift = bits - logsize;
    let mut reversed_n = size >> 1;
    // nothing should be done for 0 and 0b111...11 (size - 1).
    for n in 1..(size - 1) {
        if n < reversed_n {
            FftComplex::swap(array, n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}
