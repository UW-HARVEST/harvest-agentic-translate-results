//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row is driven with MANY randomized
//! inputs from a fixed seed (SplitMix64) so that value-dependent behaviour
//! (`strtod` rounding, saturation branch selection, prefix consumption, offset
//! arithmetic) is exercised rather than a single hand-picked value.

mod common;

use common::*;

/// Bytes the C `switch` accepts.
const CHARSET: &[u8] = b"0123456789+-.eE";
/// Bytes that hit `default:` and stop the scan.
const TERMINATORS: &[u8] = &[
    b',', b']', b'}', b':', b'"', b' ', b'\t', b'\n', b'\r', 0x00, b'a', b'x', b'A', b'/', b'\\',
    b'[', b'{', 0x80, 0xFF, 0x7F, b'*',
];

// ---------------------------------------------------------------------------
// C1 — pure-digit tokens (has_decimal_point == false)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c1_pure_digits() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..4000 {
        let n = rng.range(1, 18);
        let s = rng.digits(n);
        let out = assert_same(&Case::new(&s));
        assert_eq!(out.ret, C_TRUE, "{}", escape(&s));
        assert_eq!(out.item_type, CJSON_NUMBER);
        assert_eq!(out.buf_offset, n);
    }
    for s in ["0", "1", "9", "10", "00", "007", "0000000000", "9999999999999999999"] {
        assert_eq!(assert_same_str(s).ret, C_TRUE, "{s:?}");
    }
}

// ---------------------------------------------------------------------------
// C2 — signed integers
// ---------------------------------------------------------------------------
#[test]
fn cfg_c2_signed_integers() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..4000 {
        let n = rng.range(1, 18);
        let mut s = vec![if rng.bool() { b'-' } else { b'+' }];
        s.extend_from_slice(&rng.digits(n));
        let out = assert_same(&Case::new(&s));
        assert_eq!(out.ret, C_TRUE, "{}", escape(&s));
        assert_eq!(out.buf_offset, n + 1);
    }
}

// ---------------------------------------------------------------------------
// C3 — has_decimal_point == true (rewrite loop runs)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c3_decimal_point() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..4000 {
        let a = rng.range(1, 12);
        let b = rng.range(1, 20);
        let mut s = rng.digits(a);
        s.push(b'.');
        s.extend_from_slice(&rng.digits(b));
        let out = assert_same(&Case::new(&s));
        assert_eq!(out.ret, C_TRUE, "{}", escape(&s));
        assert_eq!(out.buf_offset, s.len());
        assert_eq!(out.item_type, CJSON_NUMBER);
    }
}

// ---------------------------------------------------------------------------
// C4 — signed decimals, plus leading/trailing '.'
// ---------------------------------------------------------------------------
#[test]
fn cfg_c4_signed_decimal() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..4000 {
        let mut s = Vec::new();
        if rng.bool() {
            s.push(if rng.bool() { b'-' } else { b'+' });
        }
        let a = rng.below(8); // may be zero -> ".5" shape
        s.extend_from_slice(&rng.digits(a));
        s.push(b'.');
        let b = rng.below(12); // may be zero -> "5." shape
        s.extend_from_slice(&rng.digits(b));
        if a == 0 && b == 0 {
            continue; // "." / "+." are E9 rejections, covered there
        }
        assert_same(&Case::new(&s));
    }
    for s in [".5", "-.5", "+.5", "5.", "-5.", "+5.", "0.", ".0", "-.0"] {
        assert_eq!(assert_same_str(s).ret, C_TRUE, "{s:?}");
    }
}

// ---------------------------------------------------------------------------
// C5 — unsigned exponents
// ---------------------------------------------------------------------------
#[test]
fn cfg_c5_exponent_unsigned() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..4000 {
        let mut s = rng.digits_range(1, 6);
        if rng.bool() {
            s.push(b'.');
            s.extend_from_slice(&rng.digits_range(1, 6));
        }
        s.push(if rng.bool() { b'e' } else { b'E' });
        s.extend_from_slice(&rng.digits_range(1, 3));
        let out = assert_same(&Case::new(&s));
        assert_eq!(out.ret, C_TRUE, "{}", escape(&s));
        assert_eq!(out.buf_offset, s.len(), "{}", escape(&s));
    }
}

// ---------------------------------------------------------------------------
// C6 — signed exponents (widest charset row)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c6_exponent_signed() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..8000 {
        let mut s = Vec::new();
        if rng.bool() {
            s.push(if rng.bool() { b'-' } else { b'+' });
        }
        s.extend_from_slice(&rng.digits_range(1, 8));
        if rng.bool() {
            s.push(b'.');
            s.extend_from_slice(&rng.digits_range(0, 8));
        }
        s.push(if rng.bool() { b'e' } else { b'E' });
        s.push(if rng.bool() { b'-' } else { b'+' });
        s.extend_from_slice(&rng.digits_range(1, 4));
        let out = assert_same(&Case::new(&s));
        assert_eq!(out.ret, C_TRUE, "{}", escape(&s));
    }
}

// ---------------------------------------------------------------------------
// C7 — overflow to +/- inf
// ---------------------------------------------------------------------------
#[test]
fn cfg_c7_overflow_inf() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..2000 {
        let neg = rng.bool();
        let exp = rng.range(309, 99999);
        let mut s = Vec::new();
        if neg {
            s.push(b'-');
        }
        s.extend_from_slice(&rng.digits_range(1, 4));
        s.push(if rng.bool() { b'e' } else { b'E' });
        if rng.bool() {
            s.push(b'+');
        }
        s.extend_from_slice(exp.to_string().as_bytes());
        let out = assert_same(&Case::new(&s));
        assert_eq!(out.ret, C_TRUE, "{}", escape(&s));
        let v = f64::from_bits(out.item_double_bits);
        assert!(v.is_infinite() || v == 0.0, "{} -> {v}", escape(&s));
        if v.is_infinite() {
            assert_eq!(
                out.item_valueint,
                if v > 0.0 { i32::MAX } else { i32::MIN },
                "{}",
                escape(&s)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C8 — underflow / subnormals
// ---------------------------------------------------------------------------
#[test]
fn cfg_c8_underflow_subnormal() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..2000 {
        let exp = rng.range(300, 99999);
        let mut s = Vec::new();
        if rng.bool() {
            s.push(b'-');
        }
        s.extend_from_slice(&rng.digits_range(1, 4));
        s.push(if rng.bool() { b'e' } else { b'E' });
        s.push(b'-');
        s.extend_from_slice(exp.to_string().as_bytes());
        let out = assert_same(&Case::new(&s));
        assert_eq!(out.ret, C_TRUE, "{}", escape(&s));
        assert_eq!(out.item_valueint, 0, "{}", escape(&s));
    }
    // Exhaustive sweep across the subnormal boundary.
    for e in 300..=340 {
        for m in 1..=9 {
            assert_same_str(format!("{m}e-{e}"));
            assert_same_str(format!("-{m}e-{e}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C9 — INT_MAX boundary sweep
// ---------------------------------------------------------------------------
#[test]
fn cfg_c9_int_max_boundary() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..4000 {
        let delta = rng.next_u64() % (1 << 20);
        let sign = rng.below(2);
        let v = if sign == 0 {
            (i32::MAX as f64) + delta as f64
        } else {
            (i32::MAX as f64) - (delta % 5) as f64
        };
        let frac = (rng.next_u64() % 1_000_000) as f64 / 1_000_000.0;
        let s = format!("{:.6}", v + frac);
        let out = assert_same_str(&s);
        assert_eq!(out.ret, C_TRUE, "{s}");
        let parsed = f64::from_bits(out.item_double_bits);
        if parsed >= i32::MAX as f64 {
            assert_eq!(out.item_valueint, i32::MAX, "{s}");
        } else {
            assert_eq!(out.item_valueint, parsed as i32, "{s}");
        }
    }
    for s in [
        "2147483645", "2147483646", "2147483647", "2147483648", "2147483649", "2147483650",
        "2147483646.999999", "2147483647.000001", "2147483647.5",
    ] {
        assert_eq!(assert_same_str(s).ret, C_TRUE, "{s}");
    }
}

// ---------------------------------------------------------------------------
// C10 — INT_MIN boundary sweep
// ---------------------------------------------------------------------------
#[test]
fn cfg_c10_int_min_boundary() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..4000 {
        let delta = rng.next_u64() % (1 << 20);
        let sign = rng.below(2);
        let v = if sign == 0 {
            (i32::MIN as f64) - delta as f64
        } else {
            (i32::MIN as f64) + (delta % 5) as f64
        };
        let frac = (rng.next_u64() % 1_000_000) as f64 / 1_000_000.0;
        let s = format!("{:.6}", v - frac);
        let out = assert_same_str(&s);
        assert_eq!(out.ret, C_TRUE, "{s}");
        let parsed = f64::from_bits(out.item_double_bits);
        if parsed <= i32::MIN as f64 {
            assert_eq!(out.item_valueint, i32::MIN, "{s}");
        } else {
            assert_eq!(out.item_valueint, parsed as i32, "{s}");
        }
    }
    for s in [
        "-2147483645", "-2147483646", "-2147483647", "-2147483648", "-2147483649",
        "-2147483650", "-2147483647.999999", "-2147483648.000001", "-2147483647.5",
    ] {
        assert_eq!(assert_same_str(s).ret, C_TRUE, "{s}");
    }
}

// ---------------------------------------------------------------------------
// C11 — in-range truncation toward zero, both signs
// ---------------------------------------------------------------------------
#[test]
fn cfg_c11_in_range_truncation() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..8000 {
        let whole = (rng.next_u64() % 4294967295u64) as i64 - 2147483647;
        let frac = rng.next_u64() % 1_000_000_000;
        let s = if whole < 0 {
            format!("{whole}.{frac:09}")
        } else {
            format!("{whole}.{frac:09}")
        };
        let out = assert_same_str(&s);
        assert_eq!(out.ret, C_TRUE, "{s}");
        let parsed = f64::from_bits(out.item_double_bits);
        let want = if parsed >= i32::MAX as f64 {
            i32::MAX
        } else if parsed <= i32::MIN as f64 {
            i32::MIN
        } else {
            parsed as i32
        };
        assert_eq!(out.item_valueint, want, "{s}");
    }
}

// ---------------------------------------------------------------------------
// C12 — zeroes (sign of zero must survive)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c12_zeroes() {
    for s in [
        "0", "-0", "+0", "0.0", "-0.0", "+0.0", "0e0", "-0e0", "-0e-0", "0.000", "-0.000",
        "0E+0", "-0E-0", "00", "-00", "0.0e-99999", "-0.0e99999",
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert_eq!(out.item_valueint, 0, "{s:?}");
        assert_eq!(
            f64::from_bits(out.item_double_bits).abs(),
            0.0,
            "{s:?} must be a zero"
        );
    }
}

// ---------------------------------------------------------------------------
// C13 — non-zero offset
// ---------------------------------------------------------------------------
#[test]
fn cfg_c13_nonzero_offset() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..6000 {
        // Random non-charset prefix, then a token, then a terminator.
        let plen = rng.range(1, 8);
        let mut content: Vec<u8> = (0..plen).map(|_| *rng.pick(TERMINATORS)).collect();
        let off = content.len();
        let mut token = Vec::new();
        if rng.bool() {
            token.push(if rng.bool() { b'-' } else { b'+' });
        }
        token.extend_from_slice(&rng.digits_range(1, 10));
        if rng.bool() {
            token.push(b'.');
            token.extend_from_slice(&rng.digits_range(1, 8));
        }
        content.extend_from_slice(&token);
        content.push(*rng.pick(TERMINATORS));
        let case = Case::new(&content).offset(off);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_TRUE, "{}", case.label());
        assert_eq!(
            out.buf_offset,
            off + token.len(),
            "{}",
            case.label()
        );
    }
}

// ---------------------------------------------------------------------------
// C14 — every terminator byte class
// ---------------------------------------------------------------------------
#[test]
fn cfg_c14_all_terminators() {
    let tokens: &[&[u8]] = &[
        b"1", b"-1", b"+1", b"0", b"12345", b"1.5", b"-1.5e3", b".5", b"5.", b"1e10", b"-0",
    ];
    for b in 0u16..=255 {
        let b = b as u8;
        if CHARSET.contains(&b) {
            continue;
        }
        for tok in tokens {
            let mut content = tok.to_vec();
            content.push(b);
            content.extend_from_slice(b"999");
            let case = Case::new(&content);
            let out = assert_same(&case);
            assert_eq!(out.ret, C_TRUE, "term {b:#04x} {}", case.label());
            assert_eq!(out.buf_offset, tok.len(), "term {b:#04x} {}", case.label());
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — `length` truncates mid-token but the visible prefix is still parsable
// ---------------------------------------------------------------------------
#[test]
fn cfg_c15_truncated_but_parsable() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..6000 {
        let mut content = Vec::new();
        if rng.bool() {
            content.push(if rng.bool() { b'-' } else { b'+' });
        }
        content.extend_from_slice(&rng.digits_range(2, 20));
        if rng.bool() {
            content.push(b'.');
            content.extend_from_slice(&rng.digits_range(1, 10));
        }
        // Cut anywhere; there is no '\0' terminator to rely on.
        let len = rng.range(0, content.len());
        let case = Case::new(&content).length(len);
        let out = assert_same(&case);
        assert!(out.buf_offset <= len, "{}", case.label());
    }
    for &(c, l, r) in &[
        (&b"12345"[..], 2usize, 2usize),
        (&b"-12345"[..], 3, 3),
        (&b"1.5e10"[..], 3, 3),
        (&b"1.5e10"[..], 4, 3),
        (&b"1.5e10"[..], 5, 5),
    ] {
        let case = Case::new(c).length(l);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_TRUE, "{}", case.label());
        assert_eq!(out.buf_offset, r, "{}", case.label());
    }
}

// ---------------------------------------------------------------------------
// C16 — `length` far beyond the token, scan stopped by a non-charset byte
// ---------------------------------------------------------------------------
#[test]
fn cfg_c16_length_beyond_token() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..4000 {
        let mut token = Vec::new();
        if rng.bool() {
            token.push(if rng.bool() { b'-' } else { b'+' });
        }
        token.extend_from_slice(&rng.digits_range(1, 10));
        if rng.bool() {
            token.push(b'.');
            token.extend_from_slice(&rng.digits_range(1, 6));
        }
        let mut content = token.clone();
        content.push(*rng.pick(TERMINATORS)); // in-buffer stop byte
        content.extend_from_slice(b"padpadpad");
        let len = *rng.pick(&[usize::MAX, usize::MAX - 1, 1usize << 40, 1usize << 32]);
        let case = Case::new(&content).length(len);
        let out = assert_same(&case);
        assert_eq!(out.ret, C_TRUE, "{}", case.label());
        assert_eq!(out.buf_offset, token.len(), "{}", case.label());
        assert_eq!(out.buf_length, len);
    }
}

// ---------------------------------------------------------------------------
// C17 — strict-prefix consumption by strtod
// ---------------------------------------------------------------------------
#[test]
fn cfg_c17_prefix_consumption() {
    for s in [
        "1e", "1E", "1e+", "1e-", "1.2.3", "1-2", "1+2", "1e5e5", "3..4", "7ee7", "9--",
        "1.2e3.4", "0.1e", "5.5.5.5", "-1e", "+2E", "8e+e", "4.e", "6-", "6+", "2.5-3",
    ] {
        let out = assert_same_str(s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
        assert!(out.buf_offset < s.len(), "{s:?} offset={}", out.buf_offset);
    }
    // Randomized: digit run followed by random charset noise.
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..10000 {
        let mut s = rng.digits_range(1, 4);
        let noise = rng.range(1, 6);
        for _ in 0..noise {
            s.push(*rng.pick(CHARSET));
        }
        let out = assert_same(&Case::new(&s));
        assert_eq!(out.ret, C_TRUE, "{}", escape(&s));
        assert!(out.buf_offset >= 1 && out.buf_offset <= s.len(), "{}", escape(&s));
    }
}

// ---------------------------------------------------------------------------
// C18 — cross-product fuzz over the interesting alphabet
// ---------------------------------------------------------------------------
#[test]
fn cfg_c18_random_alphabet_fuzz() {
    const ALPHABET: &[u8] = b"0123456789+-.eE,]} \0a";
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..20000 {
        let n = rng.below(24);
        let content: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHABET)).collect();
        // length <= content.len() keeps the C's reads inside the real allocation;
        // offset may exceed length (the C then does nothing).
        let length = rng.below(content.len() + 1);
        let offset = match rng.below(6) {
            0 => 0,
            1 => length,
            2 => length + 1 + rng.below(3),
            3 => usize::MAX - rng.below(3),
            _ => rng.below(content.len() + 1),
        };
        let case = Case::new(&content)
            .length(length)
            .offset(offset)
            .depth(rng.next_u64() as usize)
            .item_state(rng.next_u64() as i32, rng.next_u64() as i32, rng.next_u64());
        assert_same(&case);
    }
}

// ---------------------------------------------------------------------------
// C19 — fully random bytes
// ---------------------------------------------------------------------------
#[test]
fn cfg_c19_random_bytes_fuzz() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..20000 {
        let n = rng.below(40);
        let content: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let length = rng.below(content.len() + 1);
        let offset = if rng.bool() {
            rng.below(content.len() + 1)
        } else {
            rng.below(length.max(1))
        };
        let case = Case::new(&content)
            .length(length)
            .offset(offset)
            .depth(rng.next_u64() as usize)
            .item_state(rng.next_u64() as i32, rng.next_u64() as i32, rng.next_u64());
        assert_same(&case);
    }
}

// ---------------------------------------------------------------------------
// C20 — a SEQUENCE of calls sharing one parse_buffer / cJSON (composed pipeline)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c20_sequential_document() {
    /// Skip cJSON-style separators / whitespace between numbers.
    fn skip(buf: &mut ParseBuffer, content: &[u8]) {
        while buf.offset < buf.length && buf.offset < content.len() {
            match content[buf.offset] {
                b',' | b' ' | b'\t' | b'\n' | b'\r' | b'[' | b']' => buf.offset += 1,
                _ => break,
            }
        }
    }

    for doc in [
        &b"1,2.5,-3e2,4"[..],
        b"[1, 2, 3]",
        b"0,-0,+0,0.0",
        b"2147483647,-2147483648,1e999,-1e999",
        b"1e,2e,3e",
        b"1.2.3,4.5.6",
        b"",
        b",,,",
        b"1",
    ] {
        assert_same_sequence(doc, 8, skip);
    }

    // Randomized documents.
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..3000 {
        let count = rng.range(1, 6);
        let mut doc = Vec::new();
        for i in 0..count {
            if i > 0 {
                doc.push(*rng.pick(&[b',', b' ', b'\t']));
            }
            if rng.bool() {
                doc.push(if rng.bool() { b'-' } else { b'+' });
            }
            doc.extend_from_slice(&rng.digits_range(1, 6));
            if rng.bool() {
                doc.push(b'.');
                doc.extend_from_slice(&rng.digits_range(0, 4));
            }
            if rng.bool() {
                doc.push(if rng.bool() { b'e' } else { b'E' });
                if rng.bool() {
                    doc.push(if rng.bool() { b'-' } else { b'+' });
                }
                doc.extend_from_slice(&rng.digits_range(1, 3));
            }
        }
        assert_same_sequence(&doc, count + 2, skip);
    }
}

// ---------------------------------------------------------------------------
// C21 — very long tokens
// ---------------------------------------------------------------------------
#[test]
fn cfg_c21_long_tokens() {
    let mut rng = Rng::new(SEED ^ 21);
    for &n in &[100usize, 200, 512, 1000, 2048, 4096] {
        for variant in 0..6 {
            let mut s = Vec::new();
            match variant {
                0 => s.extend_from_slice(&rng.digits(n)),
                1 => {
                    s.push(b'-');
                    s.extend_from_slice(&rng.digits(n));
                }
                2 => {
                    s.extend_from_slice(&rng.digits(n / 2));
                    s.push(b'.');
                    s.extend_from_slice(&rng.digits(n / 2));
                }
                3 => {
                    s.push(b'0');
                    s.push(b'.');
                    for _ in 0..n {
                        s.push(b'0');
                    }
                    s.extend_from_slice(b"1");
                }
                4 => {
                    s.extend_from_slice(&rng.digits(8));
                    s.push(b'e');
                    s.extend_from_slice(&rng.digits(n.min(300)));
                }
                _ => {
                    s.extend_from_slice(&rng.digits(8));
                    s.push(b'e');
                    s.push(b'-');
                    s.extend_from_slice(&rng.digits(n.min(300)));
                }
            }
            let case = Case::new(&s);
            let out = assert_same(&case);
            assert_eq!(out.ret, C_TRUE, "variant {variant} n={n}");
        }
    }
    // Long tokens with a terminator and a non-zero offset.
    for _ in 0..200 {
        let n = rng.range(50, 600);
        let mut content = vec![b' '; rng.range(0, 5)];
        let off = content.len();
        let tok = rng.digits(n);
        content.extend_from_slice(&tok);
        content.push(b',');
        let case = Case::new(&content).offset(off);
        let out = assert_same(&case);
        assert_eq!(out.buf_offset, off + n);
    }
}

// ---------------------------------------------------------------------------
// C22 — '.'-heavy tokens (the rewrite loop rewrites many bytes)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c22_many_decimal_points() {
    for s in [
        "1.......2", "..........", "1.2.3.4.5.6", ".1.1.1", "0.0.0.0", "1...", "....1",
        "1.1.", "9.9.9e9.9",
    ] {
        assert_same_str(s);
    }
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..8000 {
        let n = rng.range(1, 24);
        let s: Vec<u8> = (0..n)
            .map(|_| if rng.below(3) == 0 { b'.' } else { rng.digit() })
            .collect();
        assert_same(&Case::new(&s));
    }
    // '.' runs of increasing length inside a number.
    for k in 0..64 {
        let mut s = b"1".to_vec();
        s.extend(std::iter::repeat(b'.').take(k));
        s.extend_from_slice(b"2");
        assert_same(&Case::new(&s));
    }
}

// ---------------------------------------------------------------------------
// C23 — offset == length - 1 (single visible byte), whole charset
// ---------------------------------------------------------------------------
#[test]
fn cfg_c23_offset_last_byte() {
    for b in 0u16..=255 {
        let b = b as u8;
        let content = vec![b'9', b'9', b];
        let case = Case::new(&content).offset(2);
        let out = assert_same(&case);
        if b.is_ascii_digit() {
            assert_eq!(out.ret, C_TRUE, "byte {b:#04x}");
            assert_eq!(out.buf_offset, 3);
        } else {
            assert_eq!(out.ret, C_FALSE, "byte {b:#04x}");
            assert_eq!(out.buf_offset, 2);
        }
    }
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..2000 {
        let n = rng.range(1, 12);
        let content: Vec<u8> = (0..n).map(|_| *rng.pick(CHARSET)).collect();
        let case = Case::new(&content).offset(n - 1);
        assert_same(&case);
    }
}

// ---------------------------------------------------------------------------
// C24 — f64 bit-pattern round-trip (strtod correct rounding over full range)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c24_f64_roundtrip() {
    let mut rng = Rng::new(SEED ^ 24);
    let mut done = 0usize;
    while done < 8000 {
        let bits = rng.next_u64();
        let v = f64::from_bits(bits);
        if !v.is_finite() {
            continue;
        }
        done += 1;
        for s in [
            format!("{v:.17e}"),
            format!("{v:e}"),
            format!("{v:.30e}"),
        ] {
            // Rust's `{:e}` uses only bytes from the accepted charset.
            assert!(
                s.bytes().all(|b| CHARSET.contains(&b)),
                "unexpected byte in {s:?}"
            );
            let out = assert_same_str(&s);
            assert_eq!(out.ret, C_TRUE, "{s:?}");
            assert_eq!(out.buf_offset, s.len(), "{s:?}");
        }
    }
    // Fixed-notation round-trip for the in-int-range subset.
    for _ in 0..4000 {
        let v = (rng.next_u64() as i64 as f64) / 1e9;
        if !v.is_finite() {
            continue;
        }
        let s = format!("{v:.12}");
        if !s.bytes().all(|b| CHARSET.contains(&b)) {
            continue;
        }
        let out = assert_same_str(&s);
        assert_eq!(out.ret, C_TRUE, "{s:?}");
    }
}
