//! `pow` plus the `errno` behaviour of glibc's implementation.

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Errno {
    None,
    Edom,
    Erange,
}

/// Compute `pow(x, y)` and the value glibc would leave in `errno`.
///
/// glibc reports:
///   * `EDOM` for a negative finite base with a finite non-integer exponent
///     (invalid operation),
///   * `ERANGE` for the pole `pow(±0, y<0)`, for overflow (an infinite result
///     from finite operands) and for underflow *to zero* — a merely subnormal
///     result does not set `errno`,
///   * nothing at all for NaN operands, for `y == 0`, for `x == 1` and whenever
///     an operand is infinite.
pub fn pow_with_errno(x: f64, y: f64) -> (f64, Errno) {
    let r = x.powf(y);

    if x.is_nan() || y.is_nan() {
        return (r, Errno::None);
    }
    if y == 0.0 || x == 1.0 {
        return (r, Errno::None);
    }
    if y.is_infinite() || x.is_infinite() {
        return (r, Errno::None);
    }
    if x == 0.0 {
        // Pole error for a negative exponent, exact zero otherwise.
        if y < 0.0 {
            return (r, Errno::Erange);
        }
        return (r, Errno::None);
    }
    if x < 0.0 && y.fract() != 0.0 {
        return (r, Errno::Edom);
    }
    if r.is_infinite() {
        return (r, Errno::Erange);
    }
    if r == 0.0 {
        return (r, Errno::Erange);
    }
    (r, Errno::None)
}
