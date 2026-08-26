//! Deterministic input corpora for the differential tests.
//!
//! Everything here is seeded, so a divergence found in CI reproduces exactly.

#![allow(dead_code)]

/// Small, fast, deterministic PRNG (SplitMix64) — no external dependency, so
/// the generated corpus is identical on every machine and every toolchain.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn chance(&mut self, num: u32, den: u32) -> bool {
        (self.next_u32() % den) < num
    }
}

const DIGITS: &[u8] = b"0123456789";
const HEXD: &[u8] = b"0123456789abcdefABCDEF";
const JUNK: &[u8] = b"+-.eEpPxX inf0123456789";
const WS: &[u8] = b" \t\n\r\x0b\x0c";

fn rand_str(rng: &mut Rng, alphabet: &[u8], len: usize) -> String {
    (0..len)
        .map(|_| *rng.pick(alphabet) as char)
        .collect()
}

/// Hand-picked adversarial literals, grouped so tests can target one axis.
pub mod fixed {
    pub const WHITESPACE_ONLY: &[&str] = &[
        "", " ", "   ", "\n", "\t", "\r", "\x0b", "\x0c", "\r\n",
        "  \n\t \x0b\x0c\r  ", "\n\n\n", "\t\t",
    ];

    pub const INTEGERS: &[&str] = &[
        "0", "1", "2", "9", "10", "42", "-0", "-1", "+1", "+0", "007", "0000",
        "-007", "16777216", "16777217", "16777218", "33554431", "33554433",
        "2147483647", "4294967295", "18446744073709551616", "123456789",
        "-2147483648", "999999999999999999999",
    ];

    pub const DECIMALS: &[&str] = &[
        "0.0", "-0.0", "+0.0", "1.5", "-1.5", "+1.5", "2.25", "-2.25",
        "0.1", "0.2", "0.3", "3.14159265358979", "2.718281828459045",
        "1.0000000000000000000000001", "0.000000000000000000000001",
        "123.456", "-123.456", "0.5", "0.25", "0.125",
    ];

    pub const POINT_LEADING: &[&str] = &[".5", "-.5", "+.5", ".25", ".0", ".1", ".000001"];
    pub const POINT_TRAILING: &[&str] = &["5.", "-5.", "+5.", "0.", "1.", "100."];

    pub const EXP_UNSIGNED_E: &[&str] = &["1e0", "1e1", "1e5", "1e10", "2e3", "0e0", "5e2"];
    pub const EXP_UPPER_E: &[&str] = &["1E0", "1E5", "1E10", "3E2", "0E0"];
    pub const EXP_SIGNED: &[&str] =
        &["1e+5", "1e-5", "1e+38", "1e-38", "1E+5", "1E-5", "-1e+5", "-1e-5"];
    pub const EXP_NO_DIGITS: &[&str] = &["1e", "1e+", "1e-", "1ee", "1E", "1E+", "1.5e", "0e"];
    pub const EXP_HUGE: &[&str] = &[
        "1e999999999999999999999", "1e-999999999999999999999",
        "1e2147483647", "1e-2147483648", "1e4294967296", "1e-4294967296",
        "1e99999999999999999999999999999999999999",
        "-1e999999999999999999999", "-1e-999999999999999999999",
        "1e18446744073709551616", "1e-18446744073709551616",
    ];
    pub const EXP_HUGE_ZERO_MANTISSA: &[&str] = &[
        "0e999999999999999999999", "0e-999999999999999999999",
        "0.0e999999999999999999999", "-0e999999999999999999999",
        "0e2147483647", "0.000e999999999999999999999",
    ];

    pub const HEX_SIMPLE: &[&str] =
        &["0x0", "0x1", "0X1", "0xf", "0xF", "0x10", "0xff", "0xFF", "0xa", "0X0"];
    pub const HEX_POINT: &[&str] =
        &["0x1.8", "0x.8", "0x1.", "0x0.8", "0x.1", "0x1.f", "0xf.f", "0X1.8"];
    pub const HEX_EXP: &[&str] = &[
        "0x1p0", "0x1p1", "0x1p-1", "0x1P4", "0x1p+4", "0x1.8p1", "0x.8p1",
        "0x1.p1", "0x1.8p2", "0x1.8P+1", "0x1.8p-1", "-0x1p1", "+0x1p1",
        "0x3p10", "0xffp-8",
    ];
    pub const HEX_EXP_NO_DIGITS: &[&str] =
        &["0x1p", "0x1p+", "0x1p-", "0x1.8p", "0x1.8p+", "0xfp", "0x1pp"];
    /// Mantissas short enough to land exactly in the 60-bit accumulator.
    pub const HEX_SHORT_MANTISSA: &[&str] = &[
        "0x1p0", "0x12p0", "0x123p0", "0x1234p0", "0x12345p0", "0x123456p0",
        "0x1234567p0", "0x12345678p0", "0x123456789p0", "0x123456789ap0",
        "0x123456789abp0", "0x123456789abcp0", "0x123456789abcdp0",
        "0x123456789abcdep0",
    ];
    /// Mantissas long enough to overflow the accumulator and set the sticky bit.
    pub const HEX_STICKY: &[&str] = &[
        "0x123456789abcdef0123456789abcdef",
        "0x123456789abcdef0123456789abcdefp0",
        "0xfffffffffffffffffffffffffffffffp0",
        "0x1000000000000000000000000000001p0",
        "0x1.23456789abcdefp10",
        "0x1.0000000000000000000000000000001p0",
        "0x1.ffffffffffffffffffffffffffffffffp0",
        "0x8000000000000000000000000000001p-100",
    ];
    pub const HEX_OVERFLOW: &[&str] = &[
        "0x1p128", "0x1p129", "0x1p1000", "0x1.ffffffp127", "0x2p127",
        "-0x1p128", "0x1p2147483647", "0x1p999999999999999999",
    ];
    pub const HEX_SUBNORMAL: &[&str] = &[
        "0x1p-126", "0x1p-127", "0x1p-149", "0x1p-150", "0x1p-151",
        "0x0.000002p-126", "0x1.8p-149", "0x1p-1000", "-0x1p-149",
        "0x1p-2147483648", "0x1p-999999999999999999",
    ];
    pub const HEX_BAD: &[&str] = &[
        "0x", "0X", "0xg", "0x.g", "0xz", "0x-1", "0x+1", "-0x", "+0X",
        "0x.", "0X.", "0x.p1",
    ];

    pub const INF: &[&str] = &[
        "inf", "INF", "Inf", "iNf", "inF", "-inf", "+inf", "-INF", "+INF",
    ];
    pub const INFINITY: &[&str] = &[
        "infinity", "INFINITY", "InFiNiTy", "-infinity", "+infinity",
        "Infinity", "iNfInItY",
    ];
    pub const INF_PARTIAL: &[&str] = &[
        "i", "in", "I", "IN", "ix", "inx", "in5", "infi", "infin", "infini",
        "infinit", "INFINIT", "infix", "infinitx", "-i", "-in", "-infi",
    ];
    pub const INF_TRAILING: &[&str] =
        &["inf1", "infx", "inf ", "inf\n", "inf.", "inf+", "infinityx", "infinity1"];

    pub const NAN: &[&str] = &["nan", "NAN", "NaN", "nAn", "-nan", "+nan", "-NAN"];
    pub const NAN_PAYLOAD: &[&str] = &[
        "nan(", "nan()", "nan(1)", "nan(123)", "nan(0x7f)", "-nan(5)",
        "NAN(abc)", "nan(_)", "nan(x", "+nan(1)",
    ];
    pub const NAN_PARTIAL: &[&str] =
        &["n", "N", "na", "NA", "nax", "n5", "-na", "nAx", "-n", "nanx", "nan1"];

    pub const BAD_START: &[&str] = &[
        "abc", "x", "z", "@", "/", "]", "e", "E", "p", "P", "*", "#",
        "q1", "_1", "'1",
    ];
    pub const BAD_POINT: &[&str] = &[".", "..", ".e5", ".-5", ".E+", "...", ".+", ".e"];
    pub const BAD_SIGN: &[&str] =
        &["-", "+", "--1", "++1", "-+1", "+-1", "- 1", "+ 1", "-.", "+.", "-x", "+x"];

    pub const TRAILING_JUNK: &[&str] = &[
        "1.5abc", "1.5 ", "1.5\n", "1.5\n\n", "0x1p3xyz", "inf\ninf",
        "1 2", "1\n2", "1.2.3", "1,5", "12 34", "1;2", "5)",
    ];

    pub const LEADING_WS: &[&str] = &[
        "   1.5", "\n\n1.5", "\t-2.5e3", "\r\n\x0b\x0c 0x1p3", " \n inf",
        " 0", "\t\t\t42", "\n-1e-5", "\x0b1", "\x0c1", "\r1",
        "                                        1.5",
    ];

    pub const TIES: &[&str] = &[
        "8388608.5", "8388609.5", "8388610.5", "8388611.5", "16777215.5",
        "16777216.5", "16777217.5", "1.0000000596046448",
        "1.00000005960464477539062", "1.00000005960464477539063",
        "1.00000005960464477539061", "0.5000000000000001",
        "0.99999999999999999999", "1.99999999999999999999",
        "2.00000000000000000001", "1.5000000000000000001",
        "2097152.5", "4194304.5", "0x1.0000001p0", "0x1.0000003p0",
    ];

    pub const OVERFLOW: &[&str] = &[
        "1e39", "1e40", "1e100", "1e308", "3.4028236e38", "3.40282357e38",
        "340282346638528859811704183484516925441",
        "340282356779733661637539395458142568448",
        "340282366920938463463374607431768211456",
        "-340282366920938463463374607431768211456",
        "-1e39", "1.0e39", "99999999999999999999999999999999999999999",
    ];

    pub const FLT_MAX_EDGE: &[&str] = &[
        "3.4028235e38", "3.40282347e38", "3.4028234663852886e38",
        "340282346638528859811704183484516925440",
        "340282346638528859811704183484516925439",
        "-3.4028235e38", "0x1.fffffep127",
    ];

    pub const SUBNORMAL: &[&str] = &[
        "1.17549435e-38", "1.1754942106924411e-38", "1.1754943508222875e-38",
        "5.877471754111438e-39", "1.401298464324817e-45", "1.4e-45",
        "7.0064923216240854e-46", "7.0064923216240855e-46", "7e-46",
        "2.938735877055719e-39", "1e-45", "1e-44",
        "0.000000000000000000000000000000000000000000001",
        "-1.4e-45", "-7e-46", "1e-46", "1e-50", "1e-60", "1e-100",
    ];

    pub const LONG: &[&str] = &["1e-1", "1e1"]; // placeholder; real ones built at runtime

    pub const NUL_AND_BINARY: &[&str] =
        &["\0", "\x001", "1\x002", "1.5\0", "\0\0\0", "1\0.5"];
}

/// Everything in `fixed`, concatenated.
pub fn all_fixed() -> Vec<String> {
    use fixed::*;
    let groups: &[&[&str]] = &[
        WHITESPACE_ONLY, INTEGERS, DECIMALS, POINT_LEADING, POINT_TRAILING,
        EXP_UNSIGNED_E, EXP_UPPER_E, EXP_SIGNED, EXP_NO_DIGITS, EXP_HUGE,
        EXP_HUGE_ZERO_MANTISSA, HEX_SIMPLE, HEX_POINT, HEX_EXP,
        HEX_EXP_NO_DIGITS, HEX_SHORT_MANTISSA, HEX_STICKY, HEX_OVERFLOW,
        HEX_SUBNORMAL, HEX_BAD, INF, INFINITY, INF_PARTIAL, INF_TRAILING,
        NAN, NAN_PAYLOAD, NAN_PARTIAL, BAD_START, BAD_POINT, BAD_SIGN,
        TRAILING_JUNK, LEADING_WS, TIES, OVERFLOW, FLT_MAX_EDGE, SUBNORMAL,
        NUL_AND_BINARY,
    ];
    let mut v: Vec<String> = Vec::new();
    for g in groups {
        v.extend(g.iter().map(|s| s.to_string()));
    }
    v
}

/// Very long literals — built at runtime because they do not fit nicely in
/// source constants.
pub fn long_literals() -> Vec<String> {
    vec![
        format!("1{}", "0".repeat(500)),
        format!("0.{}1", "0".repeat(500)),
        format!("1.{}", "2".repeat(1000)),
        format!("{}1", "0".repeat(500)),
        format!("1e{}", "9".repeat(100)),
        format!("0x1.{}p0", "f".repeat(200)),
        format!("0x{}p0", "1".repeat(300)),
        format!("{}", "9".repeat(2000)),
        format!("0.{}", "9".repeat(2000)),
        format!("-1.{}e-30", "7".repeat(800)),
        format!("{}.{}", "1".repeat(400), "1".repeat(400)),
        format!("{}1e-500", "0".repeat(400)),
        format!("1e-{}", "9".repeat(50)),
        format!("{}5", " ".repeat(4096)),
    ]
}

// ---------------------------------------------------------------------------
// randomized generators (one per CONFIGS.md row group)
// ---------------------------------------------------------------------------

/// Round-trip formatting of random `f32` bit patterns: `{:?}`, 17 significant
/// digits, and the C99 hex form.
pub fn rand_roundtrip(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n * 3);
    for _ in 0..n {
        let f = f32::from_bits(rng.next_u32());
        out.push(format!("{:?}", f));
        out.push(format!("{:.*e}", 17, f));
        out.push(hex_form(f));
    }
    out
}

/// C99 `%a`-style rendering of an `f32` (exact, so it must re-parse exactly).
fn hex_form(f: f32) -> String {
    let bits = f.to_bits();
    let sign = if bits >> 31 == 1 { "-" } else { "" };
    let exp = ((bits >> 23) & 0xff) as i32;
    let mant = bits & 0x7f_ffff;
    if exp == 0xff {
        return if mant == 0 {
            format!("{sign}inf")
        } else {
            format!("{sign}nan")
        };
    }
    if exp == 0 {
        if mant == 0 {
            return format!("{sign}0x0p+0");
        }
        return format!("{sign}0x0.{:06x}p-126", mant << 1);
    }
    format!("{sign}0x1.{:06x}p{:+}", mant << 1, exp - 127)
}

/// Random decimal literals across the whole sign/point/exponent grid.
pub fn rand_decimal(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let ip_len = rng.below(13);
        let fp_len = rng.below(26);
        let mut s = String::new();
        s.push_str(*rng.pick(&["", "+", "-"]));
        s.push_str(&rand_str(&mut rng, DIGITS, ip_len));
        if rng.chance(7, 10) {
            s.push('.');
            s.push_str(&rand_str(&mut rng, DIGITS, fp_len));
        }
        if rng.chance(6, 10) {
            s.push(if rng.chance(1, 2) { 'e' } else { 'E' });
            s.push_str(*rng.pick(&["", "+", "-"]));
            let elen = rng.below(5);
            s.push_str(&rand_str(&mut rng, DIGITS, elen));
        }
        out.push(s);
    }
    out
}

/// Random hexadecimal-float literals.
pub fn rand_hex(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let ip_len = rng.below(19);
        let fp_len = rng.below(19);
        let mut s = String::new();
        s.push_str(*rng.pick(&["", "+", "-"]));
        s.push_str(*rng.pick(&["0x", "0X"]));
        s.push_str(&rand_str(&mut rng, HEXD, ip_len));
        if rng.chance(7, 10) {
            s.push('.');
            s.push_str(&rand_str(&mut rng, HEXD, fp_len));
        }
        if rng.chance(7, 10) {
            s.push(if rng.chance(1, 2) { 'p' } else { 'P' });
            s.push_str(*rng.pick(&["", "+", "-"]));
            s.push_str(&rng.range(0, 200).to_string());
        }
        out.push(s);
    }
    out
}

/// Random soup over the alphabet that can appear in a float token.
pub fn rand_junk(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let len = 1 + rng.below(14);
            rand_str(&mut rng, JUNK, len)
        })
        .collect()
}

/// leading whitespace ⧺ a known-interesting token ⧺ trailing junk.
pub fn rand_wrapped(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let base = all_fixed();
    (0..n)
        .map(|_| {
            let ws_len = rng.below(5);
            let ws = rand_str(&mut rng, WS, ws_len);
            let val = base[rng.below(base.len())].clone();
            let tail_len = rng.below(5);
            let tail = rand_str(&mut rng, JUNK, tail_len);
            format!("{ws}{val}{tail}")
        })
        .collect()
}

/// Random mantissa with a decimal exponent straddling the whole `f32` range.
pub fn rand_extreme_exp(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let digits = 1 + rng.below(20);
            let mut m = String::new();
            m.push(*rng.pick(b"123456789") as char);
            m.push_str(&rand_str(&mut rng, DIGITS, digits - 1));
            format!("{m}e{}", rng.range(-60, 60))
        })
        .collect()
}

/// 20–60 digit significands: dense in the near-tie region where a naive
/// decimal→binary conversion rounds the wrong way.
pub fn rand_near_tie(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let len = 20 + rng.below(41);
            format!("0.{}e{}", rand_str(&mut rng, DIGITS, len), rng.range(-50, 50))
        })
        .collect()
}

/// 14–40 hex-digit mantissas: always long enough to exercise the sticky bit.
pub fn rand_sticky_hex(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let len = 14 + rng.below(27);
            format!("0x{}p{}", rand_str(&mut rng, HEXD, len), rng.range(-160, 160))
        })
        .collect()
}

/// Exact halfway points between consecutive `f32` values, rendered exactly in
/// decimal, plus one decimal step either side. These are the inputs where
/// ties-to-even is observable.
pub fn exact_ties(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for _ in 0..n {
        // pick a finite, non-subnormal f32 with room for a successor
        let bits = rng.next_u32() & 0x7fff_ffff;
        let exp = (bits >> 23) & 0xff;
        if exp == 0 || exp >= 0xfe {
            continue;
        }
        let a = f64::from(f32::from_bits(bits));
        let b = f64::from(f32::from_bits(bits + 1));
        let mid = (a + b) / 2.0;
        // 40 significant digits renders the midpoint exactly (it is a f64
        // representable value with a short binary expansion).
        let s = format!("{:.*e}", 40, mid);
        out.push(s.clone());
        out.push(format!("{:.*e}", 40, mid * (1.0 + f64::EPSILON)));
        out.push(format!("{:.*e}", 40, mid * (1.0 - f64::EPSILON)));
        if rng.chance(1, 2) {
            out.push(format!("-{s}"));
        }
    }
    out
}

/// Raw byte strings (not necessarily valid UTF-8).
pub fn binary_inputs() -> Vec<Vec<u8>> {
    vec![
        vec![0x00],
        vec![0x00, b'1'],
        vec![b'1', 0x00, b'2'],
        vec![0x80],
        vec![0xff],
        vec![0x80, 0xff],
        vec![b'1', 0xc3],
        vec![0xff, 0xfe, b'1', b'.', b'5'],
        vec![b'1', b'.', b'5', 0x80],
        vec![0xc3, 0x28],
        vec![0xed, 0xa0, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![b' ', 0x00, b'1'],
        (0u8..=255).collect(),
        (0u8..=255).rev().collect(),
        vec![b'\r', b'\n', b'1', b'.', b'5', b'\r', b'\n'],
        b"1.5\r\n".to_vec(),
        {
            let mut v = b"1.5".to_vec();
            v.extend(std::iter::repeat(b'x').take(65536));
            v
        },
        {
            let mut v: Vec<u8> = std::iter::repeat(b' ').take(65536).collect();
            v.extend_from_slice(b"2.5");
            v
        },
    ]
}

/// The `f32` values used to drive `driver` directly through the FFI boundary.
pub fn driver_values(seed: u64, n: usize) -> Vec<f32> {
    let mut v: Vec<f32> = Vec::new();

    // named boundary values
    for bits in [
        0x0000_0000u32, // +0
        0x8000_0000,    // -0
        0x0000_0001,    // +smallest subnormal
        0x8000_0001,    // -smallest subnormal
        0x007f_ffff,    // largest subnormal
        0x807f_ffff,
        0x0080_0000, // smallest normal
        0x8080_0000,
        0x3f80_0000, // 1.0
        0xbf80_0000, // -1.0
        0x3f00_0000, // 0.5
        0x4048_f5c3, // ~3.14
        0x7f7f_ffff, // FLT_MAX
        0xff7f_ffff, // -FLT_MAX
        0x7f80_0000, // +inf
        0xff80_0000, // -inf
        0x7fc0_0000, // quiet NaN
        0xffc0_0000, // -quiet NaN
        0x7fa0_0000, // signalling NaN
        0xffa0_0000, // -signalling NaN
        0x7f80_0001, // NaN, minimal payload
        0x7fff_ffff, // NaN, maximal payload
        0xffff_ffff,
        0x4b7f_ffff, // 16777215
        0x4b80_0000, // 16777216
        0x0000_0002,
        0x7f7f_fffe,
    ] {
        v.push(f32::from_bits(bits));
    }

    // every value of every byte lane (row 54)
    for lane in 0..4 {
        for byte in 0..=255u32 {
            v.push(f32::from_bits(byte << (8 * lane)));
        }
    }

    // uniform random patterns (row 55)
    let mut rng = Rng::new(seed);
    for _ in 0..n {
        v.push(f32::from_bits(rng.next_u32()));
    }

    v
}

/// Stride sweep of the full 2^32 space with a prime step (row 56).
pub fn driver_full_sweep(stride: u32) -> impl Iterator<Item = f32> {
    (0u64..)
        .map(move |i| i.wrapping_mul(stride as u64))
        .take_while(|&x| x <= u32::MAX as u64)
        .map(|x| f32::from_bits(x as u32))
}
