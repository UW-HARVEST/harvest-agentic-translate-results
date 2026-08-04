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
        // Compute in f64 to mirror C's cos/sin which return double
        let theta = 2.0_f64 * std::f64::consts::PI / (n as f64);
        self.real = theta.cos() as f32;
        self.imag = -(theta.sin()) as f32;
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
    // reversed_n <<= shift
    let r = if shift >= bits {
        0
    } else {
        reversed_n.wrapping_shl(shift as u32)
    };
    // count_leading_ones = fft_clz(~reversed_n)
    let count_leading_ones = fft_clz(!r);
    // reversed_n <<= count_leading_ones (remove leading ones)
    let r = if count_leading_ones >= bits {
        0
    } else {
        r.wrapping_shl(count_leading_ones as u32)
    };
    // reversed_n |= 1 << (INTBITS - 1)
    let r = r | (1usize << (bits - 1));
    // reversed_n >>= shift + count_leading_ones
    let total_shift = shift + count_leading_ones;
    if total_shift >= bits {
        0
    } else {
        r >> total_shift
    }
}
/// Performs the Fast Fourier Transform on the input array `x` and stores the result in `X`.
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}
/// Performs the Rader's algorithm on the input array `array` and stores the result in `target`.
pub fn rader(array: &[FftComplex], target: &mut [FftComplex], logsize: usize) {
    let bits = usize::BITS as usize;
    let size: usize = 1usize << logsize;
    let shift = bits - logsize;
    let mut reversed_n: usize = 0;
    for n in 0..size {
        target[reversed_n] = array[n];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}
/// Performs the raw FFT on the input array `x`.
pub fn fft_raw(x: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }

    do_butterfly(x, 2, logsize);

    if logsize == 1 {
        return;
    }

    do_butterfly(x, 4, logsize);

    if logsize == 2 {
        return;
    }

    let total: usize = 1usize << logsize;
    let mut step: usize = 8;
    while step <= total {
        do_butterfly(x, step, logsize);
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
    let bits = usize::BITS as usize;
    let size: usize = 1usize << logsize;
    let shift = bits - logsize;
    // nothing should be done for 0 and 0b111...11(size - 1).
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

/// Helper: Performs a single butterfly stage with the given `step` size on the
/// entire slice `x` (whose length is `1 << logsize`).
fn do_butterfly(x: &mut [FftComplex], step: usize, _logsize: usize) {
    let mut unit = FftComplex { real: 0.0, imag: 0.0 };
    unit.unit_root_recip(step);
    let half = step / 2;
    let len = x.len();
    let mut start: usize = 0;
    while start < len {
        // i == 0, j == half
        let t = x[start + half];
        let u = x[start];
        x[start] = u.add(&t);
        x[start + half] = u.sub(&t);

        if half > 1 {
            // i == 1, j == half + 1
            let mut root = unit;
            let t = root.mul(&x[start + half + 1]);
            let u = x[start + 1];
            x[start + 1] = u.add(&t);
            x[start + half + 1] = u.sub(&t);

            let mut i: usize = 2;
            let mut j: usize = half + 2;
            while i < half {
                root.self_mul(&unit);
                let t = root.mul(&x[start + j]);
                let u = x[start + i];
                x[start + i] = u.add(&t);
                x[start + j] = u.sub(&t);
                i += 1;
                j += 1;
            }
        }

        start += step;
    }
}
