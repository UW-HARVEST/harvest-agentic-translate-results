//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the complete observable outcome byte-for-byte.

mod common;

use common::*;

/* ---- helpers -------------------------------------------------------- */

fn sweep(label: &str, cases: usize, make: impl Fn(&Rng) -> Vec<u8>) {
    let rng = Rng::new(SEED);
    for k in 0..cases {
        let s = make(&rng);
        diff_str(&format!("{label}#{k}"), &s);
    }
}

/* ---- row 1: plain non-negative integers ----------------------------- */

#[test]
fn cfg01_plain_integers() {
    for s in [
        &b"0"[..], b"1", b"7", b"9", b"10", b"42", b"100", b"999999",
        b"2147483647", b"4294967295", b"9007199254740993",
        b"18446744073709551616", b"000", b"0000000001",
    ] {
        diff_str("cfg01/fixed", s);
    }
    sweep("cfg01/rand", CASES, |rng| {
        let n = rng.range(1, 18) as usize;
        rng.digits(n)
    });
}

/* ---- row 2: signed integers ----------------------------------------- */

#[test]
fn cfg02_signed_integers() {
    for s in [
        &b"-0"[..], b"+0", b"-1", b"+1", b"-42", b"+42", b"-2147483648",
        b"-2147483649", b"+2147483648", b"-9223372036854775808",
    ] {
        diff_str("cfg02/fixed", s);
    }
    sweep("cfg02/rand", CASES, |rng| {
        let mut v = vec![*rng.pick(b"+-")];
        let n = rng.range(1, 18) as usize;
        v.extend(rng.digits(n));
        v
    });
}

/* ---- row 3: decimals (has_decimal_point path) ----------------------- */

#[test]
fn cfg03_decimals() {
    for s in [
        &b"0.0"[..], b"1.5", b"-1.5", b"+1.5", b".5", b"-.5", b"5.", b"-5.",
        b"3.14159265358979323846", b"0.000000000000000001",
        b"123456789.987654321", b"-0.0", b"0.1", b"2.675",
    ] {
        diff_str("cfg03/fixed", s);
    }
    sweep("cfg03/rand", CASES, |rng| {
        let mut v = Vec::new();
        if rng.bool() {
            v.push(*rng.pick(b"+-"));
        }
        v.extend(rng.digits(rng.range(0, 12) as usize));
        v.push(b'.');
        v.extend(rng.digits(rng.range(0, 14) as usize));
        v
    });
}

/* ---- row 4: scientific notation ------------------------------------- */

#[test]
fn cfg04_scientific() {
    for s in [
        &b"1e5"[..], b"1E5", b"1e+5", b"1E+5", b"1e-5", b"1E-5", b"0e0",
        b"-1e5", b"+1E-5", b"9e99", b"1e308", b"1e-308", b"2e-1",
    ] {
        diff_str("cfg04/fixed", s);
    }
    sweep("cfg04/rand", CASES, |rng| {
        let mut v = Vec::new();
        if rng.bool() {
            v.push(*rng.pick(b"+-"));
        }
        v.extend(rng.digits(rng.range(1, 6) as usize));
        v.push(if rng.bool() { b'e' } else { b'E' });
        match rng.below(3) {
            0 => v.push(b'+'),
            1 => v.push(b'-'),
            _ => {}
        }
        v.extend(rng.digits(rng.range(1, 3) as usize));
        v
    });
}

/* ---- row 5: decimal + scientific ----------------------------------- */

#[test]
fn cfg05_decimal_scientific() {
    for s in [
        &b"-1.2345e-17"[..], b"1.0E+300", b".5e-5", b"5.e5", b"-0.0e0",
        b"+12.34E-56", b"1.7976931348623157e308", b"2.2250738585072014e-308",
    ] {
        diff_str("cfg05/fixed", s);
    }
    sweep("cfg05/rand", CASES, |rng| {
        let mut v = Vec::new();
        if rng.bool() {
            v.push(*rng.pick(b"+-"));
        }
        v.extend(rng.digits(rng.range(0, 8) as usize));
        v.push(b'.');
        v.extend(rng.digits(rng.range(0, 10) as usize));
        v.push(if rng.bool() { b'e' } else { b'E' });
        if rng.bool() {
            v.push(*rng.pick(b"+-"));
        }
        v.extend(rng.digits(rng.range(1, 3) as usize));
        v
    });
}

/* ---- row 6: trailing delimiter hits `default: goto loop_end` -------- */

#[test]
fn cfg06_trailing_delimiter() {
    const DELIMS: &[u8] = b",]}\":\t\n\r \0[{ax_/\\'";
    let rng = Rng::new(SEED);
    for d in DELIMS {
        for num in [&b"1"[..], b"-2.5", b"3e4", b"0", b"12345.678e-9"] {
            let mut v = num.to_vec();
            v.push(*d);
            v.extend_from_slice(b"trailing garbage 999");
            let o = diff_str(&format!("cfg06/{d:#04x}"), &v);
            assert_eq!(o.ret, 1, "expected success for {:?}", String::from_utf8_lossy(num));
            assert_eq!(o.buf_offset, num.len(), "offset must stop at the numeric run");
        }
    }
    for k in 0..CASES {
        let n = rng.range(1, 10) as usize;
        let mut v = rng.digits(n);
        // any byte outside the accepted class
        loop {
            let b = rng.byte();
            if !NUMERIC_ALPHABET.contains(&b) {
                v.push(b);
                break;
            }
        }
        v.extend(rand_from(&rng, b"0123456789.eE+-xyz", rng.range(0, 8) as usize));
        diff_str(&format!("cfg06/rand#{k}"), &v);
    }
}

/* ---- row 7: non-zero offset ---------------------------------------- */

#[test]
fn cfg07_nonzero_offset() {
    let content = b"XXXXX-12.5e3,rest";
    for off in 0..content.len() {
        diff(
            &format!("cfg07/off{off}"),
            Some(content),
            content.len(),
            off,
            0,
            CJson::garbage(),
        );
    }
    let rng = Rng::new(SEED);
    for k in 0..CASES {
        let prefix = rand_from(&rng, b"abc{[,: ", rng.range(0, 6) as usize);
        let num = {
            let mut v = rng.digits(rng.range(1, 8) as usize);
            if rng.bool() {
                v.push(b'.');
                v.extend(rng.digits(rng.range(1, 5) as usize));
            }
            v
        };
        let mut content = prefix.clone();
        content.extend_from_slice(&num);
        content.extend_from_slice(b"}suffix");
        let off = rng.below(content.len() as u64 + 2) as usize;
        diff(
            &format!("cfg07/rand#{k}"),
            Some(&content),
            content.len(),
            off,
            0,
            CJson::garbage(),
        );
    }
}

/* ---- row 8: length truncates the numeric run ----------------------- */

#[test]
fn cfg08_length_truncates_run() {
    let content = b"123456789.123456789e12";
    for len in 0..=content.len() {
        let o = diff(
            &format!("cfg08/len{len}"),
            Some(content),
            len,
            0,
            0,
            CJson::garbage(),
        );
        if len > 0 {
            assert_eq!(o.ret, 1, "len={len}");
        }
    }
    let rng = Rng::new(SEED);
    for k in 0..CASES {
        let s = rand_from(&rng, NUMERIC_ALPHABET, rng.range(1, 20) as usize);
        let len = rng.below(s.len() as u64 + 1) as usize;
        let off = rng.below(len as u64 + 1) as usize;
        diff(
            &format!("cfg08/rand#{k}"),
            Some(&s),
            len,
            off,
            0,
            CJson::garbage(),
        );
    }
}

/* ---- row 9: no NUL terminator, run reaches exactly `length` --------- */

#[test]
fn cfg09_no_terminator() {
    // Heap buffer with NO trailing NUL: exactly the numeric bytes.
    let rng = Rng::new(SEED);
    for k in 0..CASES {
        let s = {
            let mut v = Vec::new();
            if rng.bool() {
                v.push(*rng.pick(b"+-"));
            }
            v.extend(rng.digits(rng.range(1, 10) as usize));
            if rng.bool() {
                v.push(b'.');
                v.extend(rng.digits(rng.range(1, 8) as usize));
            }
            if rng.bool() {
                v.push(if rng.bool() { b'e' } else { b'E' });
                if rng.bool() {
                    v.push(*rng.pick(b"+-"));
                }
                v.extend(rng.digits(rng.range(1, 3) as usize));
            }
            v
        };
        // Vec<u8> with exact capacity: no guaranteed NUL after the last byte.
        let exact: Box<[u8]> = s.clone().into_boxed_slice();
        let o = diff(
            &format!("cfg09/rand#{k}"),
            Some(&exact),
            exact.len(),
            0,
            0,
            CJson::garbage(),
        );
        let _ = o;
    }
    // Followed by a *numeric* byte outside the window: proves `length` and not
    // a terminator bounds the scan.
    let content = b"12345";
    let o = diff("cfg09/window", Some(content), 3, 0, 0, CJson::garbage());
    assert_eq!(o.ret, 1);
    assert_eq!(o.buf_offset, 3);
    assert_eq!(o.valueint, 123);
}

/* ---- row 10: single-byte window, all 256 byte values --------------- */

#[test]
fn cfg10_single_byte_all_256() {
    for b in 0u16..=255 {
        let byte = b as u8;
        let content = [byte, b'9', b'9'];
        diff(
            &format!("cfg10/{byte:#04x}"),
            Some(&content),
            1,
            0,
            0,
            CJson::garbage(),
        );
        // same byte at offset 1 with length 2 => single-byte window
        let content2 = [b'Z', byte, b'9'];
        diff(
            &format!("cfg10/off1/{byte:#04x}"),
            Some(&content2),
            2,
            1,
            0,
            CJson::garbage(),
        );
    }
}

/* ---- row 11: strtod partially consumes the scanned prefix ---------- */

#[test]
fn cfg11_partial_consumption() {
    for s in [
        &b"1e"[..], b"1e+", b"1e-", b"1E", b"1E+", b"1.", b"1.e",
        b"12E-", b"1.2e", b"1.2e+", b"0e", b"0e+", b"-1e", b"+1E-",
        b"1e+e", b"1.5e-e", b"9999e", b"1e++5", b"1e--5", b"1e5e5",
    ] {
        diff_str("cfg11/fixed", s);
    }
    let rng = Rng::new(SEED);
    for k in 0..CASES {
        let mut v = rng.digits(rng.range(1, 5) as usize);
        if rng.bool() {
            v.push(b'.');
            v.extend(rng.digits(rng.below(4) as usize));
        }
        v.push(if rng.bool() { b'e' } else { b'E' });
        // deliberately produce a dangling / malformed exponent
        match rng.below(4) {
            0 => {}
            1 => v.push(b'+'),
            2 => v.push(b'-'),
            _ => v.extend_from_slice(b"+-"),
        }
        diff_str(&format!("cfg11/rand#{k}"), &v);
    }
}

/* ---- row 12: multiple signs / dots / exponents -------------------- */

#[test]
fn cfg12_multi_sign_dot_exp() {
    for s in [
        &b"1.2.3"[..], b"1e2e3", b"1-2", b"+1+2", b"--1", b"1..2", b"..1",
        b"1E2E3", b"-+1", b"+-1", b"1-", b"1+", b"...", b"1.2.3.4e5e6",
        b"-1-2-3", b"1e2.3", b"12.34.56e78",
    ] {
        diff_str("cfg12/fixed", s);
    }
    sweep("cfg12/rand", CASES, |rng| {
        rand_from(rng, b"0123456789..++--eeEE", rng.range(1, 14) as usize)
    });
}

/* ---- row 13: zeros and negative zero ------------------------------ */

#[test]
fn cfg13_zeros_and_neg_zero() {
    for s in [
        &b"0"[..], b"-0", b"+0", b"0.0", b"-0.0", b"+0.0", b"0e0", b"-0e0",
        b"0e-0", b"-0.000e10", b"00000", b"-00000.00000", b"0e999",
        b"-0e999", b"0e-999",
    ] {
        let o = diff_str("cfg13/fixed", s);
        assert_eq!(o.ret, 1, "{:?}", String::from_utf8_lossy(s));
        assert_eq!(o.valueint, 0);
    }
    sweep("cfg13/rand", CASES, |rng| {
        let mut v = Vec::new();
        if rng.bool() {
            v.push(*rng.pick(b"+-"));
        }
        v.extend(std::iter::repeat(b'0').take(rng.range(1, 6) as usize));
        if rng.bool() {
            v.push(b'.');
            v.extend(std::iter::repeat(b'0').take(rng.range(1, 6) as usize));
        }
        if rng.bool() {
            v.push(b'e');
            if rng.bool() {
                v.push(*rng.pick(b"+-"));
            }
            v.extend(rng.digits(rng.range(1, 3) as usize));
        }
        v
    });
}

/* ---- row 14: in-range (int) truncation ---------------------------- */

#[test]
fn cfg14_int_truncation() {
    for (s, want) in [
        (&b"1.9"[..], 1),
        (b"-1.9", -1),
        (b"0.9", 0),
        (b"-0.9", 0),
        (b"2147483646.9", 2147483646),
        (b"-2147483647.9", -2147483647),
        (b"1e9", 1000000000),
        (b"-1e9", -1000000000),
        (b"123.456", 123),
        (b"-123.456", -123),
    ] {
        let o = diff_str("cfg14/fixed", s);
        assert_eq!(o.ret, 1);
        assert_eq!(o.valueint, want, "{:?}", String::from_utf8_lossy(s));
    }
    let rng = Rng::new(SEED);
    for k in 0..CASES {
        let mag = rng.below(2_147_483_600);
        let mut v = Vec::new();
        if rng.bool() {
            v.push(b'-');
        }
        v.extend(mag.to_string().into_bytes());
        v.push(b'.');
        v.extend(rng.digits(rng.range(1, 6) as usize));
        diff_str(&format!("cfg14/rand#{k}"), &v);
    }
}

/* ---- row 15: saturation branches ---------------------------------- */

#[test]
fn cfg15_saturation() {
    for (s, want) in [
        (&b"2147483647"[..], i32::MAX),
        (b"2147483648", i32::MAX),
        (b"2147483647.5", i32::MAX),
        (b"1e999", i32::MAX),
        (b"1e300", i32::MAX),
        (b"99999999999999999999", i32::MAX),
        (b"-2147483648", i32::MIN),
        (b"-2147483649", i32::MIN),
        (b"-1e999", i32::MIN),
        (b"-1e300", i32::MIN),
        (b"-99999999999999999999", i32::MIN),
    ] {
        let o = diff_str("cfg15/fixed", s);
        assert_eq!(o.ret, 1);
        assert_eq!(o.valueint, want, "{:?}", String::from_utf8_lossy(s));
    }
    let rng = Rng::new(SEED);
    for k in 0..CASES {
        let mut v = Vec::new();
        if rng.bool() {
            v.push(b'-');
        }
        // magnitudes straddling 2^31
        let base: u64 = 2_147_483_648;
        let delta = rng.below(64) as i64 - 32;
        let mag = (base as i64 + delta).max(0) as u64;
        v.extend(mag.to_string().into_bytes());
        if rng.bool() {
            v.push(b'.');
            v.extend(rng.digits(rng.range(1, 4) as usize));
        }
        diff_str(&format!("cfg15/rand#{k}"), &v);
    }
}

/* ---- row 16: extreme exponents ------------------------------------ */

#[test]
fn cfg16_extreme_exponents() {
    for s in [
        &b"1e999"[..], b"-1e999", b"1e-999", b"-1e-999", b"1e308", b"1e309",
        b"1e-308", b"1e-320", b"1e-324", b"1e-325", b"5e-324", b"4.9e-324",
        b"1.7976931348623157e308", b"1.7976931348623159e308",
        b"2.2250738585072014e-308", b"1e1000000", b"1e-1000000",
        b"1e99999999999999999999", b"1e-99999999999999999999",
    ] {
        diff_str("cfg16/fixed", s);
    }
    sweep("cfg16/rand", CASES, |rng| {
        let mut v = Vec::new();
        if rng.bool() {
            v.push(b'-');
        }
        v.extend(rng.digits(rng.range(1, 3) as usize));
        if rng.bool() {
            v.push(b'.');
            v.extend(rng.digits(rng.range(1, 4) as usize));
        }
        v.push(if rng.bool() { b'e' } else { b'E' });
        if rng.bool() {
            v.push(b'-');
        }
        v.extend(rng.digits(rng.range(1, 8) as usize));
        v
    });
}

/* ---- row 17: very long digit strings ------------------------------ */

#[test]
fn cfg17_long_digit_strings() {
    let rng = Rng::new(SEED);
    for k in 0..600 {
        let n = rng.range(30, 400) as usize;
        let mut v = Vec::new();
        if rng.bool() {
            v.push(b'-');
        }
        v.extend(rng.digits(n));
        if rng.bool() {
            v.push(b'.');
            v.extend(rng.digits(rng.range(20, 300) as usize));
        }
        if rng.bool() {
            v.push(b'e');
            if rng.bool() {
                v.push(b'-');
            }
            v.extend(rng.digits(rng.range(1, 4) as usize));
        }
        diff_str(&format!("cfg17/rand#{k}"), &v);
    }
    // exact half-way rounding cases
    for s in [
        &b"0.5000000000000000000000000000001"[..],
        b"1.0000000000000000000000000000001",
        b"9007199254740993",
        b"9007199254740992.5",
        b"123456789012345678901234567890.12345678901234567890",
    ] {
        diff_str("cfg17/round", s);
    }
}

/* ---- row 18: depth pass-through ----------------------------------- */

#[test]
fn cfg18_depth_passthrough() {
    for depth in [0usize, 1, 42, 1000, usize::MAX, usize::MAX - 1] {
        for s in [&b"1"[..], b"abc", b"-2.5e3", b""] {
            let o = diff(
                &format!("cfg18/depth{depth}"),
                Some(s),
                s.len(),
                0,
                depth,
                CJson::garbage(),
            );
            assert_eq!(o.buf_depth, depth, "depth must be untouched");
        }
    }
}

/* ---- row 19: item pre-state --------------------------------------- */

#[test]
fn cfg19_item_prestate() {
    let pres = [
        CJson { type_: 0, valueint: 0, valuedouble: 0.0 },
        CJson { type_: -1, valueint: -1, valuedouble: -0.0 },
        CJson { type_: i32::MAX, valueint: i32::MAX, valuedouble: f64::INFINITY },
        CJson { type_: i32::MIN, valueint: i32::MIN, valuedouble: f64::NEG_INFINITY },
        CJson { type_: 8, valueint: 7, valuedouble: f64::NAN },
        CJson { type_: 12345, valueint: -98765, valuedouble: f64::from_bits(0xFFF0_0000_0000_0001) },
        CJson::garbage(),
    ];
    for (pi, pre) in pres.iter().enumerate() {
        for s in [&b"1"[..], b"-2.5", b"1e999", b"abc", b"+", b""] {
            let o = diff(
                &format!("cfg19/pre{pi}"),
                Some(s),
                s.len(),
                0,
                7,
                *pre,
            );
            if o.ret == 0 {
                // failure must preserve the item verbatim
                assert_eq!(o.type_, pre.type_);
                assert_eq!(o.valueint, pre.valueint);
                assert_eq!(o.valuedouble_bits, pre.valuedouble.to_bits());
            } else {
                assert_eq!(o.type_, 8, "cJSON_Number");
            }
        }
    }
    let rng = Rng::new(SEED);
    for k in 0..CASES {
        let pre = CJson {
            type_: rng.next_u64() as i32,
            valueint: rng.next_u64() as i32,
            valuedouble: f64::from_bits(rng.next_u64()),
        };
        let s = rand_from(&rng, NUMERIC_ALPHABET, rng.range(0, 10) as usize);
        diff(&format!("cfg19/rand#{k}"), Some(&s), s.len(), 0, rng.next_u64() as usize, pre);
    }
}

/* ---- row 20: sequential stream (composed pipeline) ---------------- */

#[test]
fn cfg20_sequential_stream() {
    let p = pair();
    let streams: [&[u8]; 6] = [
        b"1,2.5,-3e2,4",
        b"[1,2,3]",
        b"0.1 0.2 0.3",
        b"-1e5;+2E-5;.5;5.",
        b"1abc2",
        b"11,,22",
    ];
    for stream in streams {
        // Walk the whole stream with both libs in lockstep, comparing state
        // after every single call (the composed pipeline).
        let mut c_item = CJson::garbage();
        let mut r_item = CJson::garbage();
        let mut c_buf = ParseBuffer {
            content: stream.as_ptr(),
            length: stream.len(),
            offset: 0,
            depth: 3,
        };
        let mut r_buf = c_buf;
        let mut steps = 0;
        loop {
            let c_ret = unsafe { p.c.parse_number_raw(&mut c_item, &mut c_buf) };
            let r_ret = unsafe { p.rust.parse_number_raw(&mut r_item, &mut r_buf) };
            assert_eq!(
                (c_ret, c_item.type_, c_item.valueint, c_item.valuedouble.to_bits(), c_buf.offset, c_buf.length, c_buf.depth),
                (r_ret, r_item.type_, r_item.valueint, r_item.valuedouble.to_bits(), r_buf.offset, r_buf.length, r_buf.depth),
                "cfg20 divergence in stream {:?} at step {steps}",
                String::from_utf8_lossy(stream)
            );
            if c_ret == 0 {
                // skip one byte and continue, like a real tokenizer would
                if c_buf.offset >= c_buf.length {
                    break;
                }
                c_buf.offset += 1;
                r_buf.offset += 1;
            }
            steps += 1;
            if steps > 64 {
                break;
            }
        }
    }
}

/* ---- row 21: fully random byte fuzz ------------------------------- */

#[test]
fn cfg21_random_bytes_fuzz() {
    let rng = Rng::new(SEED);
    for k in 0..8000 {
        let n = rng.range(0, 40) as usize;
        let s: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let len = if n == 0 { 0 } else { rng.below(n as u64 + 1) as usize };
        let off = rng.below(len as u64 + 2) as usize;
        diff(
            &format!("cfg21/rand#{k}"),
            Some(&s),
            len,
            off,
            rng.next_u64() as usize,
            CJson::garbage(),
        );
    }
}

/* ---- row 22: random numeric-alphabet windows ---------------------- */

#[test]
fn cfg22_random_numeric_alphabet() {
    let rng = Rng::new(SEED);
    for k in 0..8000 {
        let n = rng.range(0, 24) as usize;
        let s = rand_from(&rng, NUMERIC_ALPHABET, n);
        diff(
            &format!("cfg22/rand#{k}"),
            Some(&s),
            s.len(),
            0,
            0,
            CJson::garbage(),
        );
    }
}

/* ---- row 23: empty window ---------------------------------------- */

#[test]
fn cfg23_empty_window() {
    let content = b"12345";
    // length == 0
    diff("cfg23/len0", Some(content), 0, 0, 0, CJson::garbage());
    // offset == length
    for len in 0..=content.len() {
        let o = diff(
            &format!("cfg23/off==len{len}"),
            Some(content),
            len,
            len,
            0,
            CJson::garbage(),
        );
        assert_eq!(o.ret, 0, "empty window must be rejected");
    }
    // empty slice
    let empty: &[u8] = &[];
    diff("cfg23/empty-slice", Some(empty), 0, 0, 0, CJson::garbage());
}

/* ---- row 24: randomized cross-product sweep ---------------------- */

#[test]
fn cfg24_cross_product_sweep() {
    let rng = Rng::new(SEED ^ 0xA5A5_A5A5_A5A5_A5A5);
    const ALPHABETS: [&[u8]; 4] = [
        b"0123456789",
        b"0123456789+-eE.",
        b"0123456789+-eE.,]} \t\n\"abcXYZ",
        b"+-eE.",
    ];
    for k in 0..12000 {
        let alpha: &[u8] = ALPHABETS[rng.below(4) as usize];
        let n = rng.range(0, 30) as usize;
        let s = rand_from(&rng, alpha, n);
        let length = match rng.below(4) {
            0 => s.len(),
            1 => rng.below(s.len() as u64 + 1) as usize,
            2 => 0,
            _ => s.len(),
        };
        let offset = match rng.below(5) {
            0 => 0,
            1 => rng.below(length.max(1) as u64) as usize,
            2 => length,
            3 => length + 1 + rng.below(4) as usize,
            _ => rng.below(s.len() as u64 + 1) as usize,
        };
        let depth = *rng.pick(&[0usize, 1, 42, usize::MAX]);
        let item_pre = CJson {
            type_: rng.next_u64() as i32,
            valueint: rng.next_u64() as i32,
            valuedouble: f64::from_bits(rng.next_u64()),
        };
        diff(
            &format!("cfg24/rand#{k}"),
            Some(&s),
            length,
            offset,
            depth,
            item_pre,
        );
    }
}

/* ---- ABI layout parity ------------------------------------------- */

#[test]
fn layout_parity() {
    assert_eq!(std::mem::size_of::<ParseBuffer>(), 32);
    assert_eq!(std::mem::align_of::<ParseBuffer>(), 8);
    assert_eq!(std::mem::size_of::<CJson>(), 16);
    assert_eq!(std::mem::align_of::<CJson>(), 8);
    // C `cJSON` field offsets: type 0, valueint 4, valuedouble 8
    let j = CJson { type_: 0, valueint: 0, valuedouble: 0.0 };
    let base = &j as *const CJson as usize;
    assert_eq!(&j.type_ as *const _ as usize - base, 0);
    assert_eq!(&j.valueint as *const _ as usize - base, 4);
    assert_eq!(&j.valuedouble as *const _ as usize - base, 8);
}
