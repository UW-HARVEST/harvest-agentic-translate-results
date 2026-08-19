//! Phase B/C — differential tests for the seed validation the port reimplements
//! in `src/strtoul.rs` + `program::parse_seed`
//! (CONFIGS.md rows 15–19, ERRORS.md rows 10–34).
//!
//! Ground truth is the *real* glibc `strtoul` in this process — the function the
//! C `.so` imports — driven through exactly the C source's decision:
//!
//! ```c
//! errno = 0;
//! unsigned long temp_seed = strtoul(argv[1], &endptr, 10);
//! if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX)  -> reject
//! unsigned int seed = (unsigned int) temp_seed;               -> accept
//! ```
//!
//! The Rust side is reached only through `libdriver.so`'s exported
//! `harness_parse_seed`, which calls the same `program::parse_seed` that `main`
//! uses. (`tests/errors.rs` additionally drives the same inputs through the
//! exported `main`, proving the decision reaches the user as the same bytes.)

mod common;

use common::{rust_impl, Impl, Rng};
use std::ffi::CString;
use std::os::raw::c_char;

const UINT_MAX: u64 = u32::MAX as u64;

/// The C validation block, executed with real glibc `strtoul`.
fn glibc_decision(arg: &[u8]) -> Result<u32, ()> {
    let c = CString::new(arg).expect("argv strings cannot contain NUL");
    unsafe {
        *libc::__errno_location() = 0;
        let mut endptr: *mut c_char = std::ptr::null_mut();
        let temp_seed = libc::strtoul(c.as_ptr(), &mut endptr, 10);
        let errno = *libc::__errno_location();
        if *endptr != 0 || errno != 0 || temp_seed > UINT_MAX {
            Err(())
        } else {
            Ok(temp_seed as u32)
        }
    }
}

#[track_caller]
fn assert_decision_matches(rust: &Impl, arg: &[u8]) {
    let expected = glibc_decision(arg);
    let got = rust.harness_parse_seed(arg);
    assert_eq!(
        expected,
        got,
        "argv[1] = {:?} (bytes {:?}): glibc/C says {:?} but rust says {:?}",
        String::from_utf8_lossy(arg),
        arg,
        expected,
        got
    );
}

/// CONFIGS.md row 15 — every accepted textual form.
#[test]
fn accepted_forms() {
    let rust = rust_impl();

    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),   // ERRORS.md row 28: accepted, seed 0
        b"0".to_vec(),
        b"1".to_vec(),
        b"7".to_vec(),
        b"42".to_vec(),
        b"12345".to_vec(),
        b"+0".to_vec(),
        b"-0".to_vec(),
        b"+42".to_vec(),
        b"-000".to_vec(),
        b"2147483647".to_vec(),
        b"2147483648".to_vec(),
        b"4294967295".to_vec(), // UINT_MAX
    ];

    // leading zeros, 1..=64 of them
    for n in 1..=64 {
        let mut s = vec![b'0'; n];
        s.extend_from_slice(b"42");
        cases.push(s);
    }

    // every isspace() byte, alone and combined, in front of a valid number
    let spaces: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    for s in spaces {
        cases.push([&[s][..], b"42"].concat());
        cases.push([&[s, s, s][..], b"42"].concat());
        cases.push([&[s][..], b"+42"].concat());
        cases.push([&[s][..], b"-0"].concat());
    }
    cases.push([&spaces[..], b"42"].concat());
    cases.push([&spaces[..], b"4294967295"].concat());

    for c in &cases {
        assert_decision_matches(&rust, c);
    }

    // sanity: the list really does contain accepted inputs
    let accepted = cases.iter().filter(|c| glibc_decision(c).is_ok()).count();
    assert_eq!(accepted, cases.len(), "expected every case to be accepted");
}

/// CONFIGS.md row 16 / ERRORS.md rows 23, 24, 27, 29 — the `UINT_MAX`,
/// `LONG_MAX` and `ULONG_MAX` boundaries, ±1 around each.
#[test]
fn uint_max_boundary() {
    let rust = rust_impl();
    let anchors: [u128; 6] = [
        u32::MAX as u128,                // 4294967295
        u32::MAX as u128 + 1,            // 4294967296  -> rejected
        i32::MAX as u128,                // 2147483647
        i64::MAX as u128,                // 9223372036854775807
        i64::MAX as u128 + 1,            // 9223372036854775808
        u64::MAX as u128,                // 18446744073709551615 (ULONG_MAX, no ERANGE)
    ];
    for a in anchors {
        for d in -2i128..=2 {
            let v = a as i128 + d;
            if v < 0 {
                continue;
            }
            let s = v.to_string().into_bytes();
            assert_decision_matches(&rust, &s);
            assert_decision_matches(&rust, &[b"+".to_vec(), s.clone()].concat());
            assert_decision_matches(&rust, &[b"0000".to_vec(), s.clone()].concat());
            assert_decision_matches(&rust, &[b" ".to_vec(), s].concat());
        }
    }

    // Explicit expectations, so a *matching pair of wrong answers* is impossible.
    assert_eq!(glibc_decision(b"4294967295"), Ok(u32::MAX));
    assert_eq!(glibc_decision(b"4294967296"), Err(()));
    assert_eq!(glibc_decision(b"18446744073709551615"), Err(()));
    assert_eq!(rust.harness_parse_seed(b"4294967295"), Ok(u32::MAX));
    assert_eq!(rust.harness_parse_seed(b"4294967296"), Err(()));
}

/// CONFIGS.md row 17 / ERRORS.md rows 19–22 — the `ERANGE` region.
#[test]
fn erange_boundary() {
    let rust = rust_impl();

    let mut cases: Vec<Vec<u8>> = vec![
        b"18446744073709551615".to_vec(), // ULONG_MAX: no ERANGE
        b"18446744073709551616".to_vec(), // ULONG_MAX + 1: ERANGE
        b"18446744073709551617".to_vec(),
        b"-18446744073709551616".to_vec(), // negated overflow
        b"-18446744073709551617".to_vec(),
        b"99999999999999999999".to_vec(),
    ];
    // 20 .. 300 digits, with and without sign / leading zeros
    for n in [20usize, 21, 25, 32, 40, 64, 100, 200, 300] {
        let nines = vec![b'9'; n];
        cases.push(nines.clone());
        cases.push([b"-".to_vec(), nines.clone()].concat());
        cases.push([b"+".to_vec(), nines.clone()].concat());
        cases.push([vec![b'0'; 40], nines.clone()].concat());
        cases.push([nines.clone(), b"x".to_vec()].concat()); // ERANGE *and* bad endptr
        let ones = vec![b'1'; n];
        cases.push(ones.clone());
        cases.push([b"-".to_vec(), ones].concat());
    }

    for c in &cases {
        assert_decision_matches(&rust, c);
    }

    // sanity: this set really does trip ERANGE (otherwise the row is vacuous)
    let rejected = cases.iter().filter(|c| glibc_decision(c).is_err()).count();
    assert_eq!(rejected, cases.len(), "expected every case to be rejected");
    // and prove ERANGE is what did it for a representative case
    unsafe {
        *libc::__errno_location() = 0;
        let s = CString::new("18446744073709551616").unwrap();
        let mut end: *mut c_char = std::ptr::null_mut();
        let v = libc::strtoul(s.as_ptr(), &mut end, 10);
        assert_eq!(*libc::__errno_location(), libc::ERANGE, "expected ERANGE");
        assert_eq!(v, u64::MAX);
        assert_eq!(*end, 0, "endptr should be at the terminator");
    }
}

/// CONFIGS.md row 18 / ERRORS.md rows 25, 26, 31, 34 — glibc's unsigned negation.
#[test]
fn negative_wraparound() {
    let rust = rust_impl();
    let cases: Vec<Vec<u8>> = vec![
        b"-0".to_vec(),                     // 0            -> accepted
        b"-1".to_vec(),                     // ULONG_MAX    -> rejected
        b"-2".to_vec(),
        b"-999999".to_vec(),
        b"-4294967295".to_vec(),
        b"-4294967296".to_vec(),
        b"-18446744073709551615".to_vec(),  // wraps to 1        -> accepted
        b"-18446744073709551614".to_vec(),  // wraps to 2        -> accepted
        b"-18446744073709547521".to_vec(),  // wraps to 4095     -> accepted
        b"-18446744069414584321".to_vec(),  // wraps to UINT_MAX -> accepted
        b"-18446744069414584320".to_vec(),  // wraps to UINT_MAX+1 -> rejected
        b"-18446744069414584319".to_vec(),  // wraps past it     -> rejected
        b"-9223372036854775808".to_vec(),
        b"  -18446744073709551615".to_vec(),
        b"-00018446744073709551615".to_vec(),
    ];
    for c in &cases {
        assert_decision_matches(&rust, c);
    }

    // Pin the interesting ones down explicitly.
    assert_eq!(glibc_decision(b"-18446744073709551615"), Ok(1));
    assert_eq!(glibc_decision(b"-18446744073709547521"), Ok(4095));
    assert_eq!(glibc_decision(b"-18446744069414584321"), Ok(u32::MAX));
    assert_eq!(glibc_decision(b"-18446744069414584320"), Err(()));
    assert_eq!(glibc_decision(b"-1"), Err(()));
    assert_eq!(glibc_decision(b"-0"), Ok(0));
    // the same conclusions must come out of the Rust export
    assert_eq!(rust.harness_parse_seed(b"-18446744073709551615"), Ok(1));
    assert_eq!(rust.harness_parse_seed(b"-18446744069414584321"), Ok(u32::MAX));
    assert_eq!(rust.harness_parse_seed(b"-18446744069414584320"), Err(()));
}

/// ERRORS.md rows 10–18 — every `*endptr != '\0'` shape.
#[test]
fn rejected_syntax() {
    let rust = rust_impl();
    let cases: Vec<&[u8]> = vec![
        b"abc",
        b"42abc",
        b"1x",
        b"12 34",
        b"4 2",
        b" ",
        b"   ",
        b"\t",
        b"\n",
        b"\r",
        b"\x0b",
        b"\x0c",
        b" \t\n",
        b"-",
        b"+",
        b"--5",
        b"+-5",
        b"-+5",
        b"- 5",
        b"+ 42",
        b"0x10",
        b"0X10",
        b"0b1",
        b"010z",
        b"42 ",
        b"42\n",
        b"42\t",
        b" 42 ",
        b"1,000",
        b"1_000",
        b"1.0",
        b"1e3",
        b"1E3",
        b"0.0",
        b".5",
        b"4\xff",
        b"\xff",
        b"\xc3\xa9",
        b"\xd9\xa1\xd9\xa2\xd9\xa3", // Arabic-Indic digits
        b"\x80\x81",
        b"seed",
        b"NaN",
        b"inf",
        b"()",
        b"2147483647a",
        b"4294967295 ",
        b"\x0142",
        b"42\x01",
    ];
    for c in &cases {
        assert_decision_matches(&rust, c);
        assert_eq!(
            glibc_decision(c),
            Err(()),
            "case {:?} was expected to be rejected by C",
            String::from_utf8_lossy(c)
        );
    }
}

/// CONFIGS.md row 19 — property-style sweep over 20 000 pseudo-random argument
/// strings built from the alphabet the parser actually branches on.
#[test]
fn random_strings_property() {
    let rust = rust_impl();
    let alphabet: &[u8] = b"0123456789+- \t\n\x0b\x0c\raAxX.,eE_/:9\xff\x80";
    let mut rng = Rng::new(0x5EED_1234);

    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for _ in 0..50_000 {
        let len = rng.below(25) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize])
            .collect();
        assert_decision_matches(&rust, &s);
        if glibc_decision(&s).is_ok() {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    // Both sides of the decision must actually be exercised.
    assert!(accepted > 50, "only {accepted} accepted cases were generated");
    assert!(rejected > 1000, "only {rejected} rejected cases were generated");

    // Digit-heavy strings: hammer the value/overflow boundary specifically.
    let digits: &[u8] = b"0123456789";
    for _ in 0..50_000 {
        let len = 1 + rng.below(25) as usize;
        let mut s: Vec<u8> = Vec::with_capacity(len + 1);
        match rng.below(4) {
            0 => s.push(b'-'),
            1 => s.push(b'+'),
            _ => {}
        }
        for _ in 0..len {
            s.push(digits[rng.below(10) as usize]);
        }
        assert_decision_matches(&rust, &s);
    }
}

/// Exhaustive coverage of the short-string domain: **every** 1-byte and every
/// 2-byte argument (bytes 1..=255, i.e. everything an `argv` string can hold),
/// plus every 3-byte combination over the alphabet the parser branches on.
///
/// This nails the whitespace/sign/digit prefix logic of `strtoul` with no
/// sampling at all: 255 + 65 025 + 17 576 decisions compared against glibc.
#[test]
fn exhaustive_short_strings() {
    let rust = rust_impl();

    let mut n = 0usize;
    for b in 1u16..=255 {
        assert_decision_matches(&rust, &[b as u8]);
        n += 1;
    }
    for a in 1u16..=255 {
        for b in 1u16..=255 {
            assert_decision_matches(&rust, &[a as u8, b as u8]);
            n += 1;
        }
    }
    assert_eq!(n, 255 + 255 * 255);

    let alphabet: &[u8] = b"0123456789+- \t\r\x0b\x0c\nax.\xff\x80\x01/:";
    assert_eq!(alphabet.len(), 26);
    for &a in alphabet {
        for &b in alphabet {
            for &c in alphabet {
                assert_decision_matches(&rust, &[a, b, c]);
            }
        }
    }
}

/// Long-string boundary: for every digit length 1..=40, the largest and smallest
/// number of that length, ±1, with and without a sign — the exact region where
/// `UINT_MAX`, `LONG_MAX`, `ULONG_MAX` and `ERANGE` transitions live.
#[test]
fn digit_length_sweep() {
    let rust = rust_impl();
    for len in 1..=40usize {
        let mut cases: Vec<Vec<u8>> = vec![
            {
                let mut v = vec![b'1'];
                v.extend(std::iter::repeat(b'0').take(len - 1));
                v
            }, // 10^(len-1)
            vec![b'9'; len],
            {
                let mut v = vec![b'9'; len];
                *v.last_mut().unwrap() = b'8';
                v
            },
            {
                let mut v = vec![b'1'];
                v.extend(std::iter::repeat(b'0').take(len - 1));
                *v.last_mut().unwrap() = b'1';
                v
            },
        ];
        let extra: Vec<Vec<u8>> = cases
            .iter()
            .flat_map(|c| {
                vec![
                    [b"-".to_vec(), c.clone()].concat(),
                    [b"+".to_vec(), c.clone()].concat(),
                    [b" ".to_vec(), c.clone()].concat(),
                    [c.clone(), b" ".to_vec()].concat(),
                    [vec![b'0'; 3], c.clone()].concat(),
                ]
            })
            .collect();
        cases.extend(extra);
        for c in &cases {
            assert_decision_matches(&rust, c);
        }
    }
}
