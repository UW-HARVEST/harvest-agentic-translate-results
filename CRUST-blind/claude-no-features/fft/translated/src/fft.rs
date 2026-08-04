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
        let theta = 2.0 * std::f64::consts::PI / (n as f64);
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
    let mut r = (reversed_n as usize)
        .checked_shl(shift as u32)
        .unwrap_or(0);
    let count_leading_ones = fft_clz(!r);
    r = r.checked_shl(count_leading_ones as u32).unwrap_or(0);
    // Set the highest bit
    r |= 1usize << (bits - 1);
    let total_shift = shift + count_leading_ones;
    r.checked_shr(total_shift as u32).unwrap_or(0)
}

/// Internal helper: butterfly stage of size `step` over `x[..]`.
fn do_butterfly(x: &mut [FftComplex], step: usize) {
    let mut unit = FftComplex { real: 0.0, imag: 0.0 };
    unit.unit_root_recip(step);
    let half = step / 2;
    let n = x.len();
    let mut p_start = 0;
    while p_start + step <= n {
        // i == 0, j == half
        let t = x[p_start + half];
        let u = x[p_start];
        x[p_start] = u.add(&t);
        x[p_start + half] = u.sub(&t);

        if half > 1 {
            // i == 1, j == half + 1
            let mut root = unit;
            let t = root.mul(&x[p_start + half + 1]);
            let u = x[p_start + 1];
            x[p_start + 1] = u.add(&t);
            x[p_start + half + 1] = u.sub(&t);

            let mut i = 2usize;
            while i < half {
                let j = half + i;
                root.self_mul(&unit);
                let t = root.mul(&x[p_start + j]);
                let u = x[p_start + i];
                x[p_start + i] = u.add(&t);
                x[p_start + j] = u.sub(&t);
                i += 1;
            }
        }

        p_start += step;
    }
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
        target[reversed_n] = array[n];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// Performs the raw FFT on the input array `x`.
pub fn fft_raw(x: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    let size = 1usize << logsize;
    let slice_len = x.len().min(size);

    {
        let s = &mut x[..slice_len];
        do_butterfly(s, 2);
    }
    if logsize == 1 {
        return;
    }
    {
        let s = &mut x[..slice_len];
        do_butterfly(s, 4);
    }
    if logsize == 2 {
        return;
    }
    let mut step: usize = 8;
    while step <= size {
        let s = &mut x[..slice_len];
        do_butterfly(s, step);
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
    if logsize < 2 {
        // Nothing to swap when size <= 2
        return;
    }
    let size = 1usize << logsize;
    let bits = usize::BITS as usize;
    let shift = bits - logsize;
    let mut reversed_n: usize = size >> 1;
    let mut n: usize = 1;
    while n < size - 1 {
        if n < reversed_n {
            FftComplex::swap(array, n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
        n += 1;
    }
}
