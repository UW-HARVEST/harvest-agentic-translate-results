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
        // e^(-i * 2 * pi / N): (cos(2*pi/N), -sin(2*pi/N))
        // Match the C behavior: compute in double precision then narrow to f32.
        let angle = 2.0_f64 * std::f64::consts::PI / (n as f64);
        self.real = angle.cos() as f32;
        self.imag = -(angle.sin()) as f32;
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
    if shift >= intbits {
        return 0;
    }
    let r = reversed_n << shift;
    let count_leading_ones = (!r).leading_zeros() as usize;
    let total_shift = shift + count_leading_ones;
    if total_shift >= intbits {
        // This corresponds to the case where after the bit-reversal traversal
        // wraps around (e.g., the call after reaching size-1).
        return 0;
    }
    let shifted = if count_leading_ones >= intbits {
        0usize
    } else {
        r << count_leading_ones
    };
    let with_msb = shifted | (1usize << (intbits - 1));
    with_msb >> total_shift
}

// Internal butterfly operation. Mirrors the DO_BUTTERFLY macro in C.
fn do_butterfly(x: &mut [FftComplex], step: usize) {
    let mut unit = FftComplex { real: 0.0, imag: 0.0 };
    unit.unit_root_recip(step);
    let half = step / 2;
    let len = x.len();
    let mut p = 0usize;
    while p < len {
        // i == 0, j == half
        let t = x[p + half];
        let u = x[p];
        x[p] = u.add(&t);
        x[p + half] = u.sub(&t);

        if half > 1 {
            // i == 1, j == half + 1
            let mut root = unit;
            let t1 = root.mul(&x[p + half + 1]);
            let u1 = x[p + 1];
            x[p + 1] = u1.add(&t1);
            x[p + half + 1] = u1.sub(&t1);

            let mut i = 2usize;
            let mut j = half + 2;
            while i < half {
                root.self_mul(&unit);
                let ti = root.mul(&x[p + j]);
                let ui = x[p + i];
                x[p + i] = ui.add(&ti);
                x[p + j] = ui.sub(&ti);
                i += 1;
                j += 1;
            }
        }

        p += step;
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
    let intbits = usize::BITS as usize;
    let shift = intbits - logsize;
    let mut reversed_n = 0usize;
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

    let max_step = 1usize << logsize;
    let mut step = 8usize;
    while step <= max_step {
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
    if logsize == 0 {
        return;
    }
    let size = 1usize << logsize;
    let intbits = usize::BITS as usize;
    let shift = intbits - logsize;
    // nothing should be done for 0 and 0b111...11 (size - 1).
    let mut reversed_n = size >> 1;
    let upper = size.saturating_sub(1);
    let mut n = 1usize;
    while n < upper {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
        n += 1;
    }
}
