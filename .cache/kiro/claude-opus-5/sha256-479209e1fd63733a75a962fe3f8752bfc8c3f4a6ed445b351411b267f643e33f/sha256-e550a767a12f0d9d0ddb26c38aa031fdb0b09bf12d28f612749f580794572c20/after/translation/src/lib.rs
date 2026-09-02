// Translation of c_src/src/driver.c -- glibc-compatible `printf` conversions.
//
// The C code is:
//
//     void driver(double f) {
//         raw_double_t u = {.f = f};
//         printf("%llx %a %.4f\n", u.x, f, f);
//     }
//
// Reproducing that byte-for-byte requires emulating three glibc conversions:
//
//   * `%llx`  -- lowercase hexadecimal of an `unsigned long long`, no padding.
//   * `%a`    -- glibc's hexadecimal floating point form (`printf_fphex.c`).
//   * `%.4f`  -- fixed notation with exactly four fractional digits, using the
//                exact decimal expansion of the binary value and round-half-to-even
//                (glibc's `__printf_fp` under the default FE_TONEAREST mode).

/// IEEE-754 binary64 exponent bias, as glibc spells it (`IEEE754_DOUBLE_BIAS`).
const IEEE754_DOUBLE_BIAS: i32 = 1023;

/// Number of hexadecimal digits needed for the 52 stored mantissa bits: 52 / 4.
const MANTISSA_HEX_DIGITS: usize = 13;

const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
const EXP_MASK: u64 = 0x7ff0_0000_0000_0000;
const MANTISSA_MASK: u64 = 0x000f_ffff_ffff_ffff;

/// Decomposed binary64, mirroring glibc's `union ieee754_double` accesses.
struct Ieee754Double {
    negative: bool,
    /// Raw biased exponent field (0 for zero/subnormal, 0x7ff for inf/nan).
    exponent: i32,
    /// Raw 52-bit stored mantissa, without any implicit leading bit.
    mantissa: u64,
}

impl Ieee754Double {
    fn new(f: f64) -> Self {
        let bits = f.to_bits();
        Ieee754Double {
            negative: (bits & SIGN_MASK) != 0,
            exponent: ((bits & EXP_MASK) >> 52) as i32,
            mantissa: bits & MANTISSA_MASK,
        }
    }

    fn is_nan(&self) -> bool {
        self.exponent == 0x7ff && self.mantissa != 0
    }

    fn is_inf(&self) -> bool {
        self.exponent == 0x7ff && self.mantissa == 0
    }
}

/// `%llx` on the raw bit pattern: lowercase hex, no leading zeroes, no padding.
///
/// A value of zero still prints a single `0`, which is what Rust's `{:x}` does.
fn format_llx(x: u64) -> String {
    format!("{:x}", x)
}

/// `%a`, following glibc's `__printf_fphex`.
///
/// The layout is `[-]0x<leading>[.<digits>]p<sign><exponent>` where:
///   * `<leading>` is `'0'` when the biased exponent field is zero (zero and
///     subnormals) and `'1'` otherwise -- glibc does not normalise subnormals.
///   * `<digits>` is the 52-bit mantissa as exactly 13 zero-padded hex digits
///     with trailing zeroes removed; the `.` is omitted when nothing remains.
///   * the exponent is decimal with an explicit sign, `p+0` for zero, and
///     `p-1022` (`BIAS - 1`) for every subnormal.
fn format_hex_double(f: f64) -> String {
    let v = Ieee754Double::new(f);

    let mut out = String::new();
    if v.negative {
        out.push('-');
    }

    // glibc emits the special names for the lowercase specifier and still
    // honours the sign bit, so negative NaNs come out as "-nan".
    if v.is_nan() {
        out.push_str("nan");
        return out;
    }
    if v.is_inf() {
        out.push_str("inf");
        return out;
    }

    let zero_mantissa = v.mantissa == 0;

    // Mantissa digits, zero filled on the left to the full 13 hex digits.
    let mut digits = format!("{:0width$x}", v.mantissa, width = MANTISSA_HEX_DIGITS);
    if zero_mantissa {
        // Precision collapses to zero, so no radix character is printed.
        digits.clear();
    } else {
        while digits.ends_with('0') {
            digits.pop();
        }
    }

    let leading = if v.exponent == 0 { '0' } else { '1' };

    let (exp_negative, exponent) = if v.exponent == 0 {
        if zero_mantissa {
            (false, 0)
        } else {
            // Subnormal: glibc reports BIAS - 1 rather than re-normalising.
            (true, IEEE754_DOUBLE_BIAS - 1)
        }
    } else if v.exponent >= IEEE754_DOUBLE_BIAS {
        (false, v.exponent - IEEE754_DOUBLE_BIAS)
    } else {
        (true, -(v.exponent - IEEE754_DOUBLE_BIAS))
    };

    out.push_str("0x");
    out.push(leading);
    if !digits.is_empty() {
        out.push('.');
        out.push_str(&digits);
    }
    out.push('p');
    out.push(if exp_negative { '-' } else { '+' });
    out.push_str(&exponent.to_string());
    out
}

/// `%.4f`.
///
/// For finite values Rust's `{:.4}` already performs the exact decimal
/// expansion with round-half-to-even, matching glibc under the default
/// rounding mode. Only the non-finite spellings differ (`nan`/`inf` in C
/// versus `NaN`/`inf` in Rust), and the sign of a NaN must be preserved.
fn format_fixed_4(f: f64) -> String {
    let v = Ieee754Double::new(f);

    if v.is_nan() || v.is_inf() {
        let mut out = String::new();
        if v.negative {
            out.push('-');
        }
        out.push_str(if v.is_nan() { "nan" } else { "inf" });
        return out;
    }

    format!("{:.4}", f)
}

/// Renders the whole `printf` format string for one call.
fn render(f: f64) -> String {
    let bits = f.to_bits();
    format!(
        "{} {} {}\n",
        format_llx(bits),
        format_hex_double(f),
        format_fixed_4(f)
    )
}

// glibc's `stdout`. Emitting through the same `FILE` object that C `printf`
// would have used keeps buffering -- and therefore the interleaving with any
// other C output in the process -- identical to the original library.
extern "C" {
    static mut stdout: *mut core::ffi::c_void;

    fn fwrite(
        ptr: *const core::ffi::c_void,
        size: usize,
        nitems: usize,
        stream: *mut core::ffi::c_void,
    ) -> usize;
}

fn write_stdout(s: &str) {
    if s.is_empty() {
        return;
    }
    unsafe {
        let stream = core::ptr::addr_of!(stdout).read();
        fwrite(
            s.as_ptr() as *const core::ffi::c_void,
            1,
            s.len(),
            stream,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: core::ffi::c_double) {
    write_stdout(&render(f));
}
