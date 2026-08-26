// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

// Takes two arguments, a base and an exponent, and returns base^exponent.
//
// This mirrors the C implementation's behavior:
// - If the result would be a domain error (i.e., base < 0 with a non-integer
//   exponent, or base == 0 with exponent <= 0 producing NaN/inf), prints a
//   domain error message to stderr and returns -1.
// - If the result is infinite (overflow) or underflow occurs (non-zero base,
//   zero result), prints a range error message to stderr and returns -1.
#[no_mangle]
pub extern "C" fn my_pow(base: f64, exponent: f64) -> f64 {
    let result = base.powf(exponent);

    // Domain error: result is NaN where the inputs themselves were finite.
    // This happens for things like pow(-1.0, 0.5).
    if result.is_nan() && !base.is_nan() && !exponent.is_nan() {
        eprintln!(
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base, exponent
        );
        return -1.0;
    }

    // Range error: overflow to infinity.
    if result.is_infinite() && base.is_finite() && exponent.is_finite() {
        eprintln!(
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        return -1.0;
    }

    // Range error: underflow (non-zero base and exponent produced a zero result).
    if result == 0.0 && base != 0.0 && exponent != 0.0 && base.is_finite() && exponent.is_finite() {
        eprintln!(
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        return -1.0;
    }

    result
}
