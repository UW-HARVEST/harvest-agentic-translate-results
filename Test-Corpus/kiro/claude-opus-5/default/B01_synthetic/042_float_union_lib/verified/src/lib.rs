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
//   * `%.4f`  -- fixed notation with exactly four fractional digits.
//
// Two pieces of ambient configuration influence the floating point conversions,
// and both are honoured here to match glibc exactly:
//
//   * Locale: glibc uses the `LC_NUMERIC` decimal point (from `localeconv()`)
//     for the radix character of BOTH `%a` and `%.4f`. That decimal point is a
//     NUL-terminated byte string, not a single `char` -- e.g. under `ps_AF.utf8`
//     it is the two bytes `d9 ab` (U+066B). We therefore read it fresh on every
//     `driver` call (the locale can change between calls) and splice those bytes
//     in verbatim, building the whole output line as a `Vec<u8>`. The `%llx`
//     conversion and the `nan`/`inf`/`0x`/`p+`/`p-` literals are unaffected.
//
//   * Rounding mode: `%.4f` rounds the exact binary value to four fractional
//     digits honouring the current hardware rounding direction (`fegetround()`),
//     applied to the SIGNED value with true IEEE semantics. The sign is always
//     taken from the sign bit, even when the magnitude rounds to zero (so `-0.0`
//     prints `-0.0000`, and a tiny negative rounding down to zero keeps its
//     `-`). Digit generation uses exact integer/bignum arithmetic -- no floating
//     point -- so the result is bit-exact rather than relying on Rust's
//     `{:.4}` (which always rounds half-to-even and ignores `fegetround`).

/// IEEE-754 binary64 exponent bias, as glibc spells it (`IEEE754_DOUBLE_BIAS`).
const IEEE754_DOUBLE_BIAS: i32 = 1023;

/// Number of hexadecimal digits needed for the 52 stored mantissa bits: 52 / 4.
const MANTISSA_HEX_DIGITS: usize = 13;

const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
const EXP_MASK: u64 = 0x7ff0_0000_0000_0000;
const MANTISSA_MASK: u64 = 0x000f_ffff_ffff_ffff;

// ---------------------------------------------------------------------------
// Locale radix character
// ---------------------------------------------------------------------------

/// Prefix of glibc's `struct lconv`. Only the first field, `char *decimal_point`,
/// is read; placing it first is ABI-correct because it is the first member of
/// the real struct.
#[repr(C)]
struct Lconv {
    decimal_point: *const core::ffi::c_char,
}

extern "C" {
    fn localeconv() -> *const Lconv;
}

/// Returns the current `LC_NUMERIC` decimal point as a byte string (without the
/// terminating NUL). Falls back to `"."` if `localeconv()` or its
/// `decimal_point` pointer is null, or if the string is empty.
fn locale_decimal_point() -> Vec<u8> {
    unsafe {
        let lc = localeconv();
        if lc.is_null() {
            return vec![b'.'];
        }
        let dp = (*lc).decimal_point;
        if dp.is_null() {
            return vec![b'.'];
        }
        let mut bytes = Vec::new();
        let mut p = dp;
        // Walk the NUL-terminated C string one byte at a time.
        loop {
            let b = *p as u8;
            if b == 0 {
                break;
            }
            bytes.push(b);
            p = p.add(1);
        }
        if bytes.is_empty() {
            vec![b'.']
        } else {
            bytes
        }
    }
}

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
/// The layout is `[-]0x<leading>[<radix><digits>]p<sign><exponent>` where:
///   * `<leading>` is `'0'` when the biased exponent field is zero (zero and
///     subnormals) and `'1'` otherwise -- glibc does not normalise subnormals.
///   * `<digits>` is the 52-bit mantissa as exactly 13 zero-padded hex digits
///     with trailing zeroes removed; the radix is omitted when nothing remains.
///   * `<radix>` is the locale decimal point (verbatim bytes).
///   * the exponent is decimal with an explicit sign, `p+0` for zero, and
///     `p-1022` (`BIAS - 1`) for every subnormal.
///
/// The result is appended to `out` as raw bytes; only the radix character has
/// changed from the previously verified structure.
fn format_hex_double(f: f64, radix: &[u8], out: &mut Vec<u8>) {
    let v = Ieee754Double::new(f);

    if v.negative {
        out.push(b'-');
    }

    // glibc emits the special names for the lowercase specifier and still
    // honours the sign bit, so negative NaNs come out as "-nan".
    if v.is_nan() {
        out.extend_from_slice(b"nan");
        return;
    }
    if v.is_inf() {
        out.extend_from_slice(b"inf");
        return;
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
        out.extend_from_slice(radix);
        out.extend_from_slice(digits.as_bytes());
    }
    out.push(b'p');
    out.push(if exp_negative { b'-' } else { b'+' });
    out.extend_from_slice(exponent.to_string().as_bytes());
}

// ---------------------------------------------------------------------------
// Minimal unsigned big integer (little-endian limbs)
// ---------------------------------------------------------------------------

/// A minimal fixed-purpose unsigned bignum used only for the exact `%.4f`
/// digit generation. Limbs are `u64`, stored little-endian (least significant
/// first). No external crates are used. The maximum magnitude encountered is
/// about 1040 bits (`E` can reach `+971`, times `10000`), i.e. ~17 limbs.
struct BigUint {
    limbs: Vec<u64>,
}

impl BigUint {
    /// Constructs a bignum from a single `u64`.
    fn from_u64(v: u64) -> Self {
        BigUint { limbs: vec![v] }
    }

    /// Drops leading zero limbs, keeping at least one limb.
    fn normalize(&mut self) {
        while self.limbs.len() > 1 {
            match self.limbs.last() {
                Some(&0) => {
                    self.limbs.pop();
                }
                _ => break,
            }
        }
    }

    /// `true` iff the value is zero.
    fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&l| l == 0)
    }

    /// `true` iff the value is odd (bit 0 set).
    fn is_odd(&self) -> bool {
        match self.limbs.first() {
            Some(&l) => (l & 1) == 1,
            None => false,
        }
    }

    /// Multiplies in place by a small `u64` factor, using `u128` intermediates.
    fn mul_small(&mut self, factor: u64) {
        let f = factor as u128;
        let mut carry: u128 = 0;
        for limb in self.limbs.iter_mut() {
            let prod = (*limb as u128) * f + carry;
            *limb = prod as u64;
            carry = prod >> 64;
        }
        while carry != 0 {
            self.limbs.push(carry as u64);
            carry >>= 64;
        }
        self.normalize();
    }

    /// Adds a small `u64` value in place, using `u128` intermediates.
    fn add_small(&mut self, addend: u64) {
        let mut carry = addend as u128;
        for limb in self.limbs.iter_mut() {
            if carry == 0 {
                break;
            }
            let sum = (*limb as u128) + carry;
            *limb = sum as u64;
            carry = sum >> 64;
        }
        while carry != 0 {
            self.limbs.push(carry as u64);
            carry >>= 64;
        }
        self.normalize();
    }

    /// Returns bit `i` (0-based, from the least significant bit).
    fn bit(&self, i: u32) -> bool {
        let limb_index = (i / 64) as usize;
        let bit_index = i % 64;
        match self.limbs.get(limb_index) {
            Some(&limb) => ((limb >> bit_index) & 1) == 1,
            None => false,
        }
    }

    /// `true` iff the lowest `n` bits are all zero.
    fn low_bits_all_zero(&self, n: u32) -> bool {
        if n == 0 {
            return true;
        }
        let full_limbs = (n / 64) as usize;
        let rem_bits = n % 64;
        for i in 0..full_limbs {
            if let Some(&limb) = self.limbs.get(i) {
                if limb != 0 {
                    return false;
                }
            }
        }
        if rem_bits != 0 {
            if let Some(&limb) = self.limbs.get(full_limbs) {
                let mask = (1u64 << rem_bits) - 1;
                if (limb & mask) != 0 {
                    return false;
                }
            }
        }
        true
    }

    /// Left shift in place by `n` bits.
    fn shl(&mut self, n: u32) {
        if n == 0 || self.is_zero() {
            return;
        }
        let limb_shift = (n / 64) as usize;
        let bit_shift = n % 64;

        if bit_shift == 0 {
            let mut new_limbs = vec![0u64; limb_shift];
            new_limbs.extend_from_slice(&self.limbs);
            self.limbs = new_limbs;
        } else {
            let mut new_limbs = vec![0u64; limb_shift];
            let mut carry: u64 = 0;
            for &limb in self.limbs.iter() {
                let shifted = ((limb as u128) << bit_shift) | (carry as u128);
                new_limbs.push(shifted as u64);
                carry = (shifted >> 64) as u64;
            }
            if carry != 0 {
                new_limbs.push(carry);
            }
            self.limbs = new_limbs;
        }
        self.normalize();
    }

    /// Right shift in place by `n` bits (floor division by `2^n`).
    fn shr(&mut self, n: u32) {
        if n == 0 || self.is_zero() {
            return;
        }
        let limb_shift = (n / 64) as usize;
        let bit_shift = n % 64;

        if limb_shift >= self.limbs.len() {
            self.limbs = vec![0];
            return;
        }

        // Drop whole limbs first.
        let mut tmp: Vec<u64> = self.limbs[limb_shift..].to_vec();

        if bit_shift != 0 {
            let mut carry: u64 = 0;
            // Process from most significant to least significant limb.
            for limb in tmp.iter_mut().rev() {
                let cur = *limb;
                let new_val = (cur >> bit_shift) | (carry << (64 - bit_shift));
                carry = cur & ((1u64 << bit_shift) - 1);
                *limb = new_val;
            }
        }

        self.limbs = tmp;
        self.normalize();
    }

    /// Renders the value in base 10 with no leading zeros ("0" when zero).
    ///
    /// Uses repeated divmod by `10^19` (which fits in a `u64`) with `u128`
    /// intermediates. All groups except the most significant are zero-padded to
    /// 19 digits.
    fn to_decimal_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }

        const BASE: u64 = 10_000_000_000_000_000_000; // 10^19

        // Working copy of limbs (little-endian).
        let mut work = self.limbs.clone();
        let mut groups: Vec<u64> = Vec::new();

        // Repeated division of the whole number by BASE, collecting remainders.
        loop {
            // Is `work` zero?
            if work.iter().all(|&l| l == 0) {
                break;
            }
            let mut rem: u128 = 0;
            // Divide from most significant limb down.
            for limb in work.iter_mut().rev() {
                let cur = (rem << 64) | (*limb as u128);
                let q = cur / (BASE as u128);
                rem = cur % (BASE as u128);
                *limb = q as u64;
            }
            groups.push(rem as u64);
            // Trim leading zero limbs to keep the loop terminating.
            while work.len() > 1 {
                match work.last() {
                    Some(&0) => {
                        work.pop();
                    }
                    _ => break,
                }
            }
        }

        // Groups are least-significant first; the last is most significant.
        let mut s = String::new();
        if let Some(&most) = groups.last() {
            s.push_str(&most.to_string());
        }
        for &g in groups.iter().rev().skip(1) {
            s.push_str(&format!("{:019}", g));
        }
        s
    }
}

// ---------------------------------------------------------------------------
// `fegetround`
// ---------------------------------------------------------------------------

extern "C" {
    fn fegetround() -> core::ffi::c_int;
}

const FE_TONEAREST: core::ffi::c_int = 0x0000;
const FE_DOWNWARD: core::ffi::c_int = 0x0400;
const FE_UPWARD: core::ffi::c_int = 0x0800;
const FE_TOWARDZERO: core::ffi::c_int = 0x0c00;

/// `%.4f`, appended to `out` as raw bytes.
///
/// For non-finite values glibc prints `[-]nan` / `[-]inf` (sign from the sign
/// bit). For finite values we compute the exact decimal expansion of the binary
/// value scaled by `10^4` and round to an integer number of ten-thousandths,
/// honouring `fegetround()` applied to the signed value. The sign is always
/// printed from the sign bit, even when the magnitude rounds to zero.
fn format_fixed_4(f: f64, radix: &[u8], out: &mut Vec<u8>) {
    let v = Ieee754Double::new(f);

    if v.is_nan() || v.is_inf() {
        if v.negative {
            out.push(b'-');
        }
        out.extend_from_slice(if v.is_nan() { b"nan" } else { b"inf" });
        return;
    }

    let bits = f.to_bits();
    let expfield = ((bits >> 52) & 0x7ff) as i32;
    let mant = bits & 0x000f_ffff_ffff_ffff;
    let negative = (bits >> 63) == 1;

    // Exact magnitude = M * 2^E.
    let (m, e): (u64, i32) = if expfield == 0 {
        (mant, -1074)
    } else {
        (mant | (1u64 << 52), expfield - 1075)
    };

    // We want |v| * 10^4 = q + r/2^S with q a non-negative integer.
    let mut q = BigUint::from_u64(m);
    q.mul_small(10_000);

    let (r_is_zero, r_gt_half, r_eq_half): (bool, bool, bool);

    if e >= 0 {
        q.shl(e as u32);
        r_is_zero = true;
        r_gt_half = false;
        r_eq_half = false;
    } else {
        let s = (-e) as u32;
        // N = M * 10000; low S bits are the remainder, high bits are q.
        let n = q; // rename for clarity
        // r == 0 iff low S bits of N are zero.
        r_is_zero = n.low_bits_all_zero(s);
        // Compare r to half = 2^(S-1) using bit (S-1).
        // bit (S-1) == 0  => r < half
        // bit (S-1) == 1  => r >= half; r == half iff bits 0..=S-2 all zero.
        let high_bit = n.bit(s - 1);
        if !high_bit {
            r_gt_half = false;
            r_eq_half = false;
        } else {
            // r >= half. r == half iff all lower bits (0..=S-2) are zero.
            let lower_zero = if s >= 2 { n.low_bits_all_zero(s - 1) } else { true };
            r_eq_half = lower_zero;
            r_gt_half = !lower_zero;
        }
        q = n;
        q.shr(s);
    }

    // Rounding decision.
    let mode = unsafe { fegetround() };
    let increment = match mode {
        FE_TOWARDZERO => false,
        FE_UPWARD => !r_is_zero && !negative,
        FE_DOWNWARD => !r_is_zero && negative,
        // FE_TONEAREST and any unrecognised value: round half to even.
        FE_TONEAREST => r_gt_half || (r_eq_half && q.is_odd()),
        _ => r_gt_half || (r_eq_half && q.is_odd()),
    };

    if increment {
        q.add_small(1);
    }

    // Render decimal digits of q, left-padded with '0' to at least length 5,
    // then splice the radix bytes before the last 4 digits.
    let mut digits = q.to_decimal_string();
    while digits.len() < 5 {
        digits.insert(0, '0');
    }
    let split = digits.len() - 4;
    let int_part = &digits.as_bytes()[..split];
    let frac_part = &digits.as_bytes()[split..];

    if negative {
        out.push(b'-');
    }
    out.extend_from_slice(int_part);
    out.extend_from_slice(radix);
    out.extend_from_slice(frac_part);
}

/// Renders the whole `printf` format string for one call into a byte buffer.
fn render(f: f64) -> Vec<u8> {
    let radix = locale_decimal_point();
    let bits = f.to_bits();

    let mut out = Vec::new();
    out.extend_from_slice(format_llx(bits).as_bytes());
    out.push(b' ');
    format_hex_double(f, &radix, &mut out);
    out.push(b' ');
    format_fixed_4(f, &radix, &mut out);
    out.push(b'\n');
    out
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

fn write_stdout(bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    unsafe {
        let stream = core::ptr::addr_of!(stdout).read();
        fwrite(
            bytes.as_ptr() as *const core::ffi::c_void,
            1,
            bytes.len(),
            stream,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: core::ffi::c_double) {
    write_stdout(&render(f));
}
