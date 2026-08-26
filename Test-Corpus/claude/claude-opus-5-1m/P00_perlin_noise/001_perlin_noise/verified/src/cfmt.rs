//! Minimal re-implementation of the C `printf` conversions used by the program.

/// Formats `value` the way glibc's `printf("%.<precision>g", value)` does.
///
/// `%g` picks `%e` style when the decimal exponent is `< -4` or `>= precision`,
/// and `%f` style otherwise; in both cases trailing zeros (and a trailing
/// decimal point) are removed because the `#` flag is not used.
pub fn format_g(value: f64, precision: usize) -> String {
    if value.is_nan() {
        // glibc prints the sign of a NaN.
        return if value.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }

    // C: "if the precision is zero, it is taken as 1".
    let p = if precision == 0 { 1 } else { precision };

    // Rust's `{:.*e}` rounds exactly like glibc's `%e` (round-half-to-even on
    // exact decimal ties) and already normalises the exponent after a carry.
    let sci = format!("{:.*e}", p - 1, value);
    let (mantissa, exponent) = match sci.split_once('e') {
        Some(parts) => parts,
        None => (sci.as_str(), "0"),
    };
    let exp: i32 = exponent.parse().unwrap_or(0);

    if exp < -4 || exp >= p as i32 {
        let digits = trim_trailing_zeros(mantissa);
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", digits, sign, exp.unsigned_abs())
    } else {
        let frac_digits = (p as i32 - 1 - exp) as usize;
        let fixed = format!("{:.*}", frac_digits, value);
        trim_trailing_zeros(&fixed)
    }
}

/// Drops trailing zeros of a fractional part, then a bare trailing '.'.
fn trim_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0');
    trimmed.trim_end_matches('.').to_string()
}

#[cfg(test)]
mod glibc_tests {
    //! Differential tests of `format_g` against the very `printf`
    //! implementation the C program uses (glibc).

    use super::format_g;
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_double, c_int};

    extern "C" {
        fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    }

    /// glibc `printf("%.<prec>g", value)`
    fn glibc_g(value: f64, precision: usize) -> String {
        let fmt = CString::new(format!("%.{precision}g")).unwrap();
        let mut buf = vec![0u8; 512];
        let n = unsafe {
            snprintf(
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
                fmt.as_ptr(),
                value as c_double,
            )
        };
        assert!(n >= 0 && (n as usize) < buf.len(), "snprintf overflow");
        unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) }
            .to_string_lossy()
            .into_owned()
    }

    #[track_caller]
    fn check(bits: u32) {
        // The driver promotes a `float` argument to `double` and prints "%.9g".
        let v = f64::from(f32::from_bits(bits));
        assert_eq!(
            glibc_g(v, 9),
            format_g(v, 9),
            "%.9g mismatch for float bits {bits:#010x} ({v:e})"
        );
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
    }

    #[test]
    fn special_floats_match_glibc() {
        for bits in [
            0x0000_0000, // +0
            0x8000_0000, // -0
            0x0000_0001, // smallest subnormal
            0x8000_0001,
            0x007f_ffff, // largest subnormal
            0x0080_0000, // FLT_MIN
            0x3f80_0000, // 1
            0xbf80_0000, // -1
            0x7f7f_ffff, // FLT_MAX
            0xff7f_ffff,
            0x7f80_0000, // +inf
            0xff80_0000, // -inf
            0x7fc0_0000, // +nan
            0xffc0_0000, // -nan
            0x7fc0_0001,
            0x4b7f_ffff, // 16777215
            0x4b80_0000, // 16777216
            0x3727_c5ac, // 1e-5
            0x4cbe_bc20, // 1e8
            0x4e6e_6b28, // 1e9
            0x501a_784a, // 1e10
        ] {
            check(bits);
        }
    }

    #[test]
    fn every_exponent_matches_glibc() {
        // One value per binary exponent, with a few mantissas each.
        for exp in 0u32..=255 {
            for mant in [0u32, 1, 0x0040_0000, 0x007f_ffff, 0x0012_3456] {
                check((exp << 23) | mant);
                check(0x8000_0000 | (exp << 23) | mant);
            }
        }
    }

    #[test]
    fn randomised_floats_match_glibc() {
        let mut rng = Rng(0x0FF1CE);
        for _ in 0..200000 {
            check(rng.next() as u32);
        }
        // Small integers and simple fractions, where the %f/%e style switch and
        // the trailing-zero trimming happen.
        for i in -100000i32..=100000 {
            check((i as f32).to_bits());
            check((i as f32 / 8.0).to_bits());
        }
    }

    #[test]
    fn other_precisions_match_glibc() {
        // `format_g` implements the general conversion, so check a few other
        // precisions as well (the driver only ever uses 9).
        let mut rng = Rng(0xC0FFEE);
        for precision in [0usize, 1, 2, 3, 6, 9, 12, 17] {
            for _ in 0..20000 {
                let v = f64::from(f32::from_bits(rng.next() as u32));
                assert_eq!(
                    glibc_g(v, precision),
                    format_g(v, precision),
                    "%.{precision}g mismatch for {v:e}"
                );
            }
        }
    }
}
