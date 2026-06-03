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
        // The C code uses double-precision cos/sin (since M_PI is double and
        // the divisions/calls promote), then truncates the result to f32.
        let theta = 2.0_f64 * std::f64::consts::PI / (n as f64);
        self.real = theta.cos() as f32;
        self.imag = -theta.sin() as f32;
    }
}

// ---------------------------------------------------------------------------
// Helper used by `fft_raw` to perform one butterfly pass with the given step.
// Mirrors the C macro `DO_BUTTERFLY`.
// ---------------------------------------------------------------------------
fn do_butterfly(arr: &mut [FftComplex], step: usize) {
    let mut unit = FftComplex { real: 0.0, imag: 0.0 };
    unit.unit_root_recip(step);
    let half = step / 2;
    let n = arr.len();
    let mut p = 0;
    while p < n {
        // i == 0, j == half
        let t = arr[p + half];
        let u = arr[p];
        arr[p] = u.add(&t);
        arr[p + half] = u.sub(&t);
        if half > 1 {
            // i == 1, j == half + 1
            let mut root = unit;
            let t = root.mul(&arr[p + half + 1]);
            let u = arr[p + 1];
            arr[p + 1] = u.add(&t);
            arr[p + half + 1] = u.sub(&t);
            // i from 2 to half - 1
            let mut i = 2usize;
            while i < half {
                let j = half + i;
                root.self_mul(&unit);
                let t = root.mul(&arr[p + j]);
                let u = arr[p + i];
                arr[p + i] = u.add(&t);
                arr[p + j] = u.sub(&t);
                i += 1;
            }
        }
        p += step;
    }
}

// Function Declarations
/// Returns the number of leading zeros in the binary representation of `n`.
pub fn fft_clz(n: usize) -> usize {
    n.leading_zeros() as usize
}
/// Computes the next reversed number given `reversed_n` and `shift`.
pub fn next_reversed_n(reversed_n: usize, shift: usize) -> usize {
    let intbits = usize::BITS as usize;
    // reversed_n <<= shift
    let mut r: usize = (reversed_n as usize)
        .checked_shl(shift as u32)
        .unwrap_or(0);
    // count_leading_ones = fft_clz(~reversed_n)
    let count_leading_ones = fft_clz(!r);
    // reversed_n <<= count_leading_ones
    r = r.checked_shl(count_leading_ones as u32).unwrap_or(0);
    // reversed_n |= 1 << (INTBITS - 1)
    r |= 1usize << (intbits - 1);
    // reversed_n >>= shift + count_leading_ones
    let total_shift = shift.wrapping_add(count_leading_ones);
    r = r.checked_shr(total_shift as u32).unwrap_or(0);
    r
}
/// Performs the Fast Fourier Transform on the input array `x` and stores the result in `X`.
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}
/// Performs the Rader's algorithm on the input array `array` and stores the result in `target`.
pub fn rader(array: &[FftComplex], target: &mut [FftComplex], logsize: usize) {
    let intbits = usize::BITS as usize;
    let size: usize = if logsize >= intbits { 0 } else { 1usize << logsize };
    // shift = INTBITS - logsize
    let shift = intbits.wrapping_sub(logsize);
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

    do_butterfly(x, 2);

    if logsize == 1 {
        return;
    }

    do_butterfly(x, 4);

    if logsize == 2 {
        return;
    }

    let size: usize = 1usize << logsize;
    let mut step = 8usize;
    while step <= size {
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
    let intbits = usize::BITS as usize;
    let size: usize = if logsize >= intbits { 0 } else { 1usize << logsize };
    let shift = intbits.wrapping_sub(logsize);
    if size < 2 {
        return;
    }
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
