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
        let angle = 2.0 * std::f64::consts::PI / (n as f64);
        self.real = angle.cos() as f32;
        self.imag = -(angle.sin() as f32);
    }
}

const USIZE_BITS: usize = std::mem::size_of::<usize>() * 8;

// Function Declarations
/// Returns the number of leading zeros in the binary representation of `n`.
pub fn fft_clz(n: usize) -> usize {
    n.leading_zeros() as usize
}
/// Computes the next reversed number given `reversed_n` and `shift`.
pub fn next_reversed_n(mut reversed_n: usize, shift: usize) -> usize {
    reversed_n = reversed_n.wrapping_shl(shift as u32);
    let count_leading_ones = fft_clz(!reversed_n);
    // remove leading ones
    reversed_n = reversed_n.wrapping_shl(count_leading_ones as u32);
    reversed_n |= 1usize << (USIZE_BITS - 1);
    reversed_n = reversed_n.wrapping_shr((shift + count_leading_ones) as u32);
    reversed_n
}
/// Performs the Fast Fourier Transform on the input array `x` and stores the result in `X`.
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}
/// Performs the Rader's algorithm on the input array `array` and stores the result in `target`.
pub fn rader(array: &[FftComplex], target: &mut [FftComplex], logsize: usize) {
    let size: usize = 1usize << logsize;
    let shift = USIZE_BITS - logsize;
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
    let size: usize = 1usize << logsize;
    let slice = &mut x[..size];

    do_butterfly(slice, 2);
    if logsize == 1 {
        return;
    }

    do_butterfly(slice, 4);
    if logsize == 2 {
        return;
    }

    let mut step: usize = 8;
    while step <= size {
        do_butterfly(slice, step);
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
    let size: usize = 1usize << logsize;
    let shift = USIZE_BITS - logsize;
    // Nothing should be done for 0 and (size - 1).
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

/// Performs the butterfly operation on the slice with the given step.
fn do_butterfly(slice: &mut [FftComplex], step: usize) {
    let mut unit = FftComplex { real: 0.0, imag: 0.0 };
    unit.unit_root_recip(step);
    let half = step / 2;
    let len = slice.len();

    let mut base: usize = 0;
    while base < len {
        // i == 0, j == half
        let t = slice[base + half];
        let u = slice[base];
        slice[base] = u.add(&t);
        slice[base + half] = u.sub(&t);

        if half <= 1 {
            base += step;
            continue;
        }

        // i == 1, j == half + 1
        let mut root = unit;
        let t = root.mul(&slice[base + half + 1]);
        let u = slice[base + 1];
        slice[base + 1] = u.add(&t);
        slice[base + half + 1] = u.sub(&t);

        // i in 2..half, j in half+2..
        let mut i = 2usize;
        let mut j = half + 2;
        while i < half {
            root.self_mul(&unit);
            let t = root.mul(&slice[base + j]);
            let u = slice[base + i];
            slice[base + i] = u.add(&t);
            slice[base + j] = u.sub(&t);
            i += 1;
            j += 1;
        }

        base += step;
    }
}
