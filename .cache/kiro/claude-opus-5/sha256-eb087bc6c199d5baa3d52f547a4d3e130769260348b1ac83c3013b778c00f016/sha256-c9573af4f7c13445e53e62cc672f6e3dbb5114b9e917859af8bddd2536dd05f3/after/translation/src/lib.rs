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
//                exact decimal expansion of the binary value (`printf_fp.c`).
//
// Two pieces of ambient process state feed into glibc's output and therefore
// have to be honoured here as well, because the caller controls them and the C
// library reads them on every call:
//
//   * the `LC_NUMERIC` decimal point (`localeconv()->decimal_point`), used by
//     BOTH `%a` and `%.4f` -- in e.g. a `de_DE` locale glibc prints `0x1,8p+0`
//     and `1,5000`;
//   * the current floating-point rounding direction (`fegetround()`), which
//     `__printf_fp` consults through its `round_away` helper, so `%.4f` of
//     `0.99999` is `1.0000` under `FE_TONEAREST` but `0.9999` under
//     `FE_TOWARDZERO`.
//
// `%llx` is unaffected by both, and `%a` without an explicit precision prints
// the value exactly, so it never rounds.

use core::ffi::{c_char, c_double, c_int, c_void};

/// IEEE-754 binary64 exponent bias, as glibc spells it (`IEEE754_DOUBLE_BIAS`).
const IEEE754_DOUBLE_BIAS: i32 = 1023;

/// Number of hexadecimal digits needed for the 52 stored mantissa bits: 52 / 4.
const MANTISSA_HEX_DIGITS: usize = 13;

/// The precision of the `%.4f` conversion in the format string.
const FIXED_PRECISION: usize = 4;

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

    /// The value as `significand * 2^exp` with an *integer* significand, the
    /// form `__printf_fp` works from.  Subnormals keep the biased exponent's
    /// implied value of 1 and no implicit leading bit.
    fn integer_significand(&self) -> (u64, i32) {
        if self.exponent == 0 {
            (self.mantissa, -1074)
        } else {
            (self.mantissa | (1u64 << 52), self.exponent - 1075)
        }
    }
}

// ---------------------------------------------------------------------------
// Ambient process state that glibc's `printf` reads
// ---------------------------------------------------------------------------

/// Prefix of glibc's `struct lconv`.  Only the first member is read; `lconv`
/// begins with `char *decimal_point` (C99 7.11.2.1 / glibc `locale.h`), and a
/// `#[repr(C)]` prefix has the same layout as the full struct for that member.
#[repr(C)]
struct LconvPrefix {
    decimal_point: *const c_char,
}

extern "C" {
    fn localeconv() -> *const LconvPrefix;
    fn fegetround() -> c_int;
}

/// The `LC_NUMERIC` radix character, exactly as `__printf_fp` and
/// `__printf_fphex` obtain it via `_NL_CURRENT (LC_NUMERIC, DECIMAL_POINT)`.
/// It can be a multi-byte string, so it is returned as bytes.
fn decimal_point() -> Vec<u8> {
    unsafe {
        let lc = localeconv();
        if lc.is_null() {
            return b".".to_vec();
        }
        let p = (*lc).decimal_point;
        if p.is_null() {
            return b".".to_vec();
        }
        let mut out = Vec::new();
        let mut i = 0isize;
        loop {
            let byte = *p.offset(i) as u8;
            if byte == 0 {
                break;
            }
            out.push(byte);
            i += 1;
        }
        out
    }
}

/// The four IEEE-754 rounding directions, in glibc's `FE_*` spelling.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Round {
    ToNearest,
    Downward,
    Upward,
    TowardZero,
}

// `FE_*` are compile-time constants in C, so they have to be restated per
// architecture.  Values taken from glibc's `bits/fenv.h`.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod fe {
    pub const TONEAREST: i32 = 0x000;
    pub const DOWNWARD: i32 = 0x400;
    pub const UPWARD: i32 = 0x800;
    pub const TOWARDZERO: i32 = 0xc00;
}

#[cfg(target_arch = "aarch64")]
mod fe {
    pub const TONEAREST: i32 = 0x000000;
    pub const UPWARD: i32 = 0x400000;
    pub const DOWNWARD: i32 = 0x800000;
    pub const TOWARDZERO: i32 = 0xc00000;
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
mod fe {
    // Unknown target: only the default direction can be named reliably.  Every
    // other value falls through to `ToNearest` below, which is the mode any
    // process starts in.
    pub const TONEAREST: i32 = 0;
    pub const DOWNWARD: i32 = i32::MIN;
    pub const UPWARD: i32 = i32::MIN + 1;
    pub const TOWARDZERO: i32 = i32::MIN + 2;
}

/// `get_rounding_mode ()` from glibc's `stdlib/rounding-mode.h`.
fn rounding_mode() -> Round {
    let m = unsafe { fegetround() } as i32;
    if m == fe::DOWNWARD {
        Round::Downward
    } else if m == fe::UPWARD {
        Round::Upward
    } else if m == fe::TOWARDZERO {
        Round::TowardZero
    } else {
        // Includes `FE_TONEAREST` and any value glibc does not recognise.
        debug_assert!(m == fe::TONEAREST || true);
        Round::ToNearest
    }
}

/// `round_away ()` from glibc's `stdlib/rounding-mode.h`.
///
/// `half_bit` is set when the discarded remainder is at least one half of the
/// last retained digit, `more_bits` when it is not *exactly* one half.
fn round_away(negative: bool, last_digit_odd: bool, half_bit: bool, more_bits: bool) -> bool {
    match rounding_mode() {
        Round::Downward => negative && (half_bit || more_bits),
        Round::ToNearest => half_bit && (last_digit_odd || more_bits),
        Round::TowardZero => false,
        Round::Upward => !negative && (half_bit || more_bits),
    }
}

// ---------------------------------------------------------------------------
// Minimal arbitrary-precision natural number
//
// `%.4f` of `DBL_MAX` needs 314 significant decimal digits, so the exact
// expansion cannot be done in machine words.  Only the handful of operations
// `__printf_fp`'s algorithm needs are implemented: multiply by a small
// constant, shift, inspect low bits, increment, and convert to decimal.
// ---------------------------------------------------------------------------

/// Little-endian base-2^32 limbs, normalised so the top limb is never zero.
struct Big {
    d: Vec<u32>,
}

impl Big {
    fn from_u64(v: u64) -> Big {
        let mut b = Big {
            d: vec![v as u32, (v >> 32) as u32],
        };
        b.trim();
        b
    }

    fn trim(&mut self) {
        while self.d.last() == Some(&0) {
            self.d.pop();
        }
    }

    fn is_zero(&self) -> bool {
        self.d.is_empty()
    }

    /// Parity of the value, which is also the parity of its last decimal digit.
    fn is_odd(&self) -> bool {
        self.d.first().map_or(false, |x| x & 1 == 1)
    }

    fn mul_small(&mut self, m: u32) {
        if self.is_zero() || m == 1 {
            return;
        }
        let mut carry: u64 = 0;
        for x in self.d.iter_mut() {
            let t = *x as u64 * m as u64 + carry;
            *x = t as u32;
            carry = t >> 32;
        }
        while carry != 0 {
            self.d.push(carry as u32);
            carry >>= 32;
        }
    }

    fn add_small(&mut self, a: u32) {
        let mut carry = a as u64;
        let mut i = 0usize;
        while carry != 0 {
            if i == self.d.len() {
                self.d.push(0);
            }
            let t = self.d[i] as u64 + carry;
            self.d[i] = t as u32;
            carry = t >> 32;
            i += 1;
        }
    }

    fn shl(&mut self, bits: usize) {
        if self.is_zero() || bits == 0 {
            return;
        }
        let sh = bits % 32;
        if sh != 0 {
            let mut carry: u32 = 0;
            for x in self.d.iter_mut() {
                let t = ((*x as u64) << sh) | carry as u64;
                *x = t as u32;
                carry = (t >> 32) as u32;
            }
            if carry != 0 {
                self.d.push(carry);
            }
        }
        let limbs = bits / 32;
        if limbs != 0 {
            let mut nd = vec![0u32; limbs];
            nd.extend_from_slice(&self.d);
            self.d = nd;
        }
    }

    fn shr(&mut self, bits: usize) {
        if self.is_zero() || bits == 0 {
            return;
        }
        let limbs = bits / 32;
        if limbs >= self.d.len() {
            self.d.clear();
            return;
        }
        self.d.drain(0..limbs);
        let sh = bits % 32;
        if sh != 0 {
            let mut carry: u32 = 0;
            for i in (0..self.d.len()).rev() {
                let v = self.d[i];
                self.d[i] = (v >> sh) | carry;
                carry = v << (32 - sh);
            }
        }
        self.trim();
    }

    /// Is bit `i` (counting from the least significant) set?
    fn bit(&self, i: usize) -> bool {
        let limb = i / 32;
        limb < self.d.len() && (self.d[limb] >> (i % 32)) & 1 == 1
    }

    /// Is any of the bits `[0, k)` set?
    fn any_bit_below(&self, k: usize) -> bool {
        if k == 0 {
            return false;
        }
        let full = k / 32;
        for i in 0..full.min(self.d.len()) {
            if self.d[i] != 0 {
                return true;
            }
        }
        let rem = k % 32;
        if rem != 0 && full < self.d.len() && self.d[full] & ((1u32 << rem) - 1) != 0 {
            return true;
        }
        false
    }

    fn to_decimal(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut tmp = self.d.clone();
        let mut chunks: Vec<u32> = Vec::new();
        while !tmp.is_empty() {
            let mut rem: u64 = 0;
            for i in (0..tmp.len()).rev() {
                let cur = (rem << 32) | tmp[i] as u64;
                tmp[i] = (cur / 1_000_000_000) as u32;
                rem = cur % 1_000_000_000;
            }
            while tmp.last() == Some(&0) {
                tmp.pop();
            }
            chunks.push(rem as u32);
        }
        let mut s = String::with_capacity(chunks.len() * 9);
        s.push_str(&chunks[chunks.len() - 1].to_string());
        for c in chunks.iter().rev().skip(1) {
            s.push_str(&format!("{:09}", c));
        }
        s
    }
}

// ---------------------------------------------------------------------------
// The three conversions
// ---------------------------------------------------------------------------

/// `%llx` on the raw bit pattern: lowercase hex, no leading zeroes, no padding.
///
/// A value of zero still prints a single `0`, which is what Rust's `{:x}` does.
/// Unaffected by locale and by the rounding direction.
fn format_llx(x: u64) -> String {
    format!("{:x}", x)
}

/// `%a`, following glibc's `__printf_fphex`.
///
/// The layout is `[-]0x<leading>[<radix><digits>]p<sign><exponent>` where:
///   * `<leading>` is `'0'` when the biased exponent field is zero (zero and
///     subnormals) and `'1'` otherwise -- glibc does not normalise subnormals.
///   * `<digits>` is the 52-bit mantissa as exactly 13 zero-padded hex digits
///     with trailing zeroes removed; the radix character is omitted when
///     nothing remains.
///   * `<radix>` is the locale's decimal point, not necessarily `'.'`.
///   * the exponent is decimal with an explicit sign, `p+0` for zero, and
///     `p-1022` (`BIAS - 1`) for every subnormal.
///
/// No precision is given in the format string, so the value is printed exactly
/// and the rounding direction never comes into play.
fn format_hex_double(f: f64) -> Vec<u8> {
    let v = Ieee754Double::new(f);

    let mut out: Vec<u8> = Vec::new();
    if v.negative {
        out.push(b'-');
    }

    // glibc emits the special names for the lowercase specifier and still
    // honours the sign bit, so negative NaNs come out as "-nan".
    if v.is_nan() {
        out.extend_from_slice(b"nan");
        return out;
    }
    if v.is_inf() {
        out.extend_from_slice(b"inf");
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

    let leading = if v.exponent == 0 { b'0' } else { b'1' };

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

    out.extend_from_slice(b"0x");
    out.push(leading);
    if !digits.is_empty() {
        out.extend_from_slice(&decimal_point());
        out.extend_from_slice(digits.as_bytes());
    }
    out.push(b'p');
    out.push(if exp_negative { b'-' } else { b'+' });
    out.extend_from_slice(exponent.to_string().as_bytes());
    out
}

/// `%.4f`, following glibc's `__printf_fp`.
///
/// The value is `significand * 2^exp` with an integer significand, so
/// `|value| * 10^precision` is either an exact integer (`exp >= 0`) or an exact
/// dyadic rational `A / 2^k` (`exp < 0`).  In the latter case the retained
/// integer part is `A >> k` and the discarded remainder is the low `k` bits,
/// which decompose directly into glibc's `half_bit` / `more_bits` pair:
/// `half_bit` is bit `k-1`, `more_bits` is "any lower bit set".
fn format_fixed(f: f64, precision: usize) -> Vec<u8> {
    let v = Ieee754Double::new(f);

    if v.is_nan() || v.is_inf() {
        let mut out: Vec<u8> = Vec::new();
        if v.negative {
            out.push(b'-');
        }
        out.extend_from_slice(if v.is_nan() { b"nan" } else { b"inf" });
        return out;
    }

    let (significand, exp) = v.integer_significand();

    let mut scaled = Big::from_u64(significand);
    let mut pow10: u32 = 1;
    for _ in 0..precision {
        pow10 *= 10;
    }
    scaled.mul_small(pow10);

    let (mut integral, half_bit, more_bits) = if exp >= 0 {
        scaled.shl(exp as usize);
        (scaled, false, false)
    } else {
        let k = (-exp) as usize;
        let half = scaled.bit(k - 1);
        let more = scaled.any_bit_below(k - 1);
        scaled.shr(k);
        (scaled, half, more)
    };

    if round_away(v.negative, integral.is_odd(), half_bit, more_bits) {
        integral.add_small(1);
    }

    let mut digits = integral.to_decimal();
    // At least one digit must remain in front of the radix character.
    while digits.len() <= precision {
        digits.insert(0, '0');
    }
    let split = digits.len() - precision;

    let mut out: Vec<u8> = Vec::new();
    if v.negative {
        out.push(b'-');
    }
    out.extend_from_slice(digits[..split].as_bytes());
    if precision > 0 {
        out.extend_from_slice(&decimal_point());
        out.extend_from_slice(digits[split..].as_bytes());
    }
    out
}

/// Renders the whole `printf` format string for one call.
fn render(f: f64) -> Vec<u8> {
    let bits = f.to_bits();
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(format_llx(bits).as_bytes());
    out.push(b' ');
    out.extend_from_slice(&format_hex_double(f));
    out.push(b' ');
    out.extend_from_slice(&format_fixed(f, FIXED_PRECISION));
    out.push(b'\n');
    out
}

// glibc's `stdout`. Emitting through the same `FILE` object that C `printf`
// would have used keeps buffering -- and therefore the interleaving with any
// other C output in the process -- identical to the original library.
extern "C" {
    static mut stdout: *mut c_void;

    fn fwrite(ptr: *const c_void, size: usize, nitems: usize, stream: *mut c_void) -> usize;
}

fn write_stdout(s: &[u8]) {
    if s.is_empty() {
        return;
    }
    unsafe {
        let stream = core::ptr::addr_of!(stdout).read();
        fwrite(s.as_ptr() as *const c_void, 1, s.len(), stream);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    write_stdout(&render(f));
}
