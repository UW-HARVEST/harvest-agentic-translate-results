//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test calls the exported `main` of BOTH shared objects (C and Rust)
//! through `libloading`, with the identical `argv` vector, and compares the
//! returned status and the bytes written to stdout.

mod common;

use common::*;

const PROG: &[u8] = b"driver";

/// C1 — argc == 2, empty string.
#[test]
fn cfg_c1_argc2_empty() {
    let out = assert_same(&[PROG, b""], Layout::Contiguous);
    assert_eq!(out.status, 0);
    assert_eq!(out.stdout, b"\n");
    // same thing with the other layout
    assert_same(&[PROG, b""], Layout::Separate);
}

/// C2 — argc == 2, every possible single (non-NUL) byte.
#[test]
fn cfg_c2_argc2_single_byte() {
    for b in 1u8..=255 {
        let s = [b];
        let out = assert_same(&[PROG, &s], Layout::Contiguous);
        assert_eq!(out.status, 0);
        assert_eq!(out.stdout, [b, b'\n']);
    }
}

/// C3 — argc == 2, random short ASCII strings.
#[test]
fn cfg_c3_argc2_short_ascii() {
    let rng = Rng::new(0xC3_0000_0001);
    for _ in 0..400 {
        let s = random_ascii(&rng, rng.range(0, 32) as usize);
        let out = assert_same(&[PROG, &s], Layout::Contiguous);
        assert_eq!(out.status, 0);
        let mut want = s.clone();
        want.push(b'\n');
        assert_eq!(out.stdout, want);
    }
}

/// C4 — argc == 2, arbitrary bytes (high bit, whitespace, newlines).
#[test]
fn cfg_c4_argc2_random_bytes() {
    let rng = Rng::new(0xC4_0000_0001);
    for _ in 0..400 {
        let s = random_string_shape(&rng);
        assert_same(&[PROG, &s], Layout::Contiguous);
    }
}

/// C5 — argc == 2, long strings (up to 4 KiB).
#[test]
fn cfg_c5_argc2_long_string() {
    let rng = Rng::new(0xC5_0000_0001);
    for _ in 0..40 {
        let s = random_bytes(&rng, rng.range(1000, 4096) as usize);
        let out = assert_same(&[PROG, &s], Layout::Contiguous);
        assert_eq!(out.status, 0);
        assert_eq!(out.stdout.len(), s.len() + 1);
    }
}

/// C6 — argc == 3, start == 0 over all string shapes.
#[test]
fn cfg_c6_argc3_start_zero() {
    let rng = Rng::new(0xC6_0000_0001);
    for _ in 0..300 {
        let s = random_string_shape(&rng);
        let zero: &[&[u8]] = &[b"0", b"+0", b"00", b" 0", b"\t+000", b"-0"];
        let n = rng.pick(zero);
        let out = assert_same(&[PROG, &s, n], Layout::Contiguous);
        assert_eq!(out.status, 0);
        let mut want = s.clone();
        want.push(b'\n');
        assert_eq!(out.stdout, want);
    }
}

/// C7 — argc == 3, start == len (upper boundary, prints just the newline).
#[test]
fn cfg_c7_argc3_start_eq_len() {
    let rng = Rng::new(0xC7_0000_0001);
    for _ in 0..300 {
        let s = random_string_shape(&rng);
        let n = decorate_number(&rng, s.len() as i64);
        let out = assert_same(&[PROG, &s, &n], Layout::Contiguous);
        assert_eq!(out.status, 0, "start == len must be accepted");
        assert_eq!(out.stdout, b"\n");
    }
}

/// C8 — argc == 3, 0 < start < len.
#[test]
fn cfg_c8_argc3_start_interior() {
    let rng = Rng::new(0xC8_0000_0001);
    for _ in 0..500 {
        let s = random_bytes(&rng, rng.range(1, 80) as usize);
        let start = rng.below(s.len() as u64) as i64;
        let n = decorate_number(&rng, start);
        let out = assert_same(&[PROG, &s, &n], Layout::Contiguous);
        assert_eq!(out.status, 0);
        let mut want = s[start as usize..].to_vec();
        want.push(b'\n');
        assert_eq!(out.stdout, want);
    }
}

/// C9 — argc == 3, start beyond the end (one past, and far past) => E4.
#[test]
fn cfg_c9_argc3_start_past_end() {
    let rng = Rng::new(0xC9_0000_0001);
    for _ in 0..300 {
        let s = random_string_shape(&rng);
        let over = rng.range(1, 1000) as i64;
        let n = decorate_number(&rng, s.len() as i64 + over);
        let out = assert_same(&[PROG, &s, &n], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, b"Error: start is off the end of the string!\n");
    }
}

/// C10 — argc == 3, decorated numeric forms (whitespace / sign / leading zeros).
#[test]
fn cfg_c10_argc3_numeric_forms() {
    let rng = Rng::new(0xC10_0000_0001);
    let prefixes: &[&[u8]] = &[
        b"", b" ", b"  ", b"\t", b"\n", b"\r", b"\x0b", b"\x0c", b" \t\n\r\x0b\x0c",
    ];
    let signs: &[&[u8]] = &[b"", b"+", b"-"];
    let zeros: &[&[u8]] = &[b"", b"0", b"00", b"0000000000000000000000"];
    for _ in 0..600 {
        let s = random_bytes(&rng, rng.range(0, 20) as usize);
        let value = rng.range(0, 25);
        let mut n: Vec<u8> = Vec::new();
        n.extend_from_slice(*rng.pick(prefixes));
        n.extend_from_slice(*rng.pick(signs));
        n.extend_from_slice(*rng.pick(zeros));
        n.extend_from_slice(format!("{value}").as_bytes());
        assert_same(&[PROG, &s, &n], Layout::Contiguous);
    }
}

/// C11 — argc == 3, digits followed by junk.
#[test]
fn cfg_c11_argc3_trailing_junk() {
    let rng = Rng::new(0xC11_0000_0001);
    let fixed: &[&[u8]] = &[
        b"0x3", b"3abc", b"3 4", b"3.", b"2e5", b"1_000", b"0X10", b"5)", b"7\n", b"9\t\t",
    ];
    for f in fixed {
        assert_same(&[PROG, b"abcdefghij", f], Layout::Contiguous);
    }
    for _ in 0..400 {
        let s = random_bytes(&rng, rng.range(0, 24) as usize);
        let value = rng.range(0, 30);
        let mut n = decorate_number(&rng, value as i64);
        n.extend_from_slice(&random_junk(&rng));
        assert_same(&[PROG, &s, &n], Layout::Contiguous);
    }
}

/// C12 — argc == 4, random valid windows 0 <= start < stop <= len.
#[test]
fn cfg_c12_argc4_valid_window() {
    let rng = Rng::new(0xC12_0000_0001);
    for _ in 0..600 {
        let s = random_bytes(&rng, rng.range(1, 80) as usize);
        let len = s.len() as u64;
        let start = rng.below(len);
        let stop = rng.range(start + 1, len);
        let a = decorate_number(&rng, start as i64);
        let b = decorate_number(&rng, stop as i64);
        let out = assert_same(&[PROG, &s, &a, &b], Layout::Contiguous);
        assert_eq!(out.status, 0);
        let mut want = s[start as usize..stop as usize].to_vec();
        want.push(b'\n');
        assert_eq!(out.stdout, want);
    }
}

/// C13 — argc == 4, start == 0 && stop == len.
#[test]
fn cfg_c13_argc4_whole_string() {
    let rng = Rng::new(0xC13_0000_0001);
    for _ in 0..300 {
        let s = random_bytes(&rng, rng.range(1, 100) as usize);
        let b = decorate_number(&rng, s.len() as i64);
        let out = assert_same(&[PROG, &s, b"0", &b], Layout::Contiguous);
        assert_eq!(out.status, 0);
        let mut want = s.clone();
        want.push(b'\n');
        assert_eq!(out.stdout, want);
    }
}

/// C14 — argc == 4, single byte windows at every offset.
#[test]
fn cfg_c14_argc4_single_byte_window() {
    let rng = Rng::new(0xC14_0000_0001);
    let s = random_bytes(&rng, 64);
    for start in 0..s.len() {
        let a = format!("{start}").into_bytes();
        let b = format!("{}", start + 1).into_bytes();
        let out = assert_same(&[PROG, &s, &a, &b], Layout::Contiguous);
        assert_eq!(out.status, 0);
        assert_eq!(out.stdout, [s[start], b'\n']);
    }
}

/// C15 — argc == 4, decorated forms on both numeric arguments.
#[test]
fn cfg_c15_argc4_numeric_forms() {
    let rng = Rng::new(0xC15_0000_0001);
    for _ in 0..800 {
        let s = random_bytes(&rng, rng.range(0, 24) as usize);
        let v1 = rng.range(0, 30) as i64;
        let v2 = rng.range(0, 30) as i64;
        let mut a = decorate_number(&rng, v1);
        let mut b = decorate_number(&rng, v2);
        if rng.below(4) == 0 {
            a.extend_from_slice(&random_junk(&rng));
        }
        if rng.below(4) == 0 {
            b.extend_from_slice(&random_junk(&rng));
        }
        if rng.below(8) == 0 {
            a = no_conversion_string(&rng);
        }
        if rng.below(8) == 0 {
            b = no_conversion_string(&rng);
        }
        assert_same(&[PROG, &s, &a, &b], Layout::Contiguous);
    }
}

/// C16 — argc == 4, `long`->`int` truncation and `strtol` saturation values.
#[test]
fn cfg_c16_argc4_truncation_values() {
    let interesting: &[&[u8]] = &[
        b"0",
        b"1",
        b"2147483646",
        b"2147483647", // INT_MAX
        b"2147483648", // INT_MAX+1  -> (int)INT_MIN
        b"2147483649",
        b"4294967295", // 2^32-1     -> (int)-1
        b"4294967296", // 2^32       -> (int)0
        b"4294967297", // 2^32+1     -> (int)1
        b"4294967300", // 2^32+4     -> (int)4
        b"-2147483648", // INT_MIN
        b"-2147483649",
        b"-4294967296", // -(2^32)   -> (int)0
        b"-4294967292", // -(2^32)+4 -> (int)4
        b"9223372036854775807",  // LONG_MAX
        b"9223372036854775808",  // > LONG_MAX -> saturates -> (int)-1
        b"99999999999999999999999",
        b"-9223372036854775808", // LONG_MIN -> (int)0
        b"-9223372036854775809", // < LONG_MIN -> saturates -> (int)0
        b"-99999999999999999999999",
    ];
    let strings: &[&[u8]] = &[b"", b"a", b"abcd", b"0123456789abcdef"];
    for s in strings {
        for a in interesting {
            // argc == 3
            assert_same(&[PROG, s, a], Layout::Contiguous);
            for b in interesting {
                // argc == 4
                assert_same(&[PROG, s, a, b], Layout::Contiguous);
            }
        }
    }
}

/// C17 — separately allocated argv strings.
#[test]
fn cfg_c17_layout_separate() {
    let rng = Rng::new(0xC17_0000_0001);
    for _ in 0..500 {
        let s = random_string_shape(&rng);
        let extra = rng.below(3); // 0 => argc 2, 1 => argc 3, 2 => argc 4
        let a = decorate_number(&rng, rng.range(0, s.len() as u64 + 4) as i64);
        let b = decorate_number(&rng, rng.range(0, s.len() as u64 + 4) as i64);
        let args: Vec<&[u8]> = match extra {
            0 => vec![PROG, &s],
            1 => vec![PROG, &s, &a],
            _ => vec![PROG, &s, &a, &b],
        };
        assert_same(&args, Layout::Separate);
    }
}

/// C18 — contiguous, exec-like argv block.
#[test]
fn cfg_c18_layout_contiguous() {
    let rng = Rng::new(0xC18_0000_0001);
    for _ in 0..500 {
        let s = random_string_shape(&rng);
        let extra = rng.below(3);
        let a = decorate_number(&rng, rng.range(0, s.len() as u64 + 4) as i64);
        let b = decorate_number(&rng, rng.range(0, s.len() as u64 + 4) as i64);
        let args: Vec<&[u8]> = match extra {
            0 => vec![PROG, &s],
            1 => vec![PROG, &s, &a],
            _ => vec![PROG, &s, &a, &b],
        };
        assert_same(&args, Layout::Contiguous);
    }
}

/// C19 — aliasing layout: `argv[3]` points into `argv[2]`, the only way the
/// `end == argv[3]` comparison can ever be true.
#[test]
fn cfg_c19_layout_alias() {
    let numbers: &[&[u8]] = &[b"0", b"1", b"12", b"  12", b"+12", b"-3", b"12abc", b"abc", b"7"];
    let strings: &[&[u8]] = &[b"", b"a", b"abcdefgh"];
    let mut saw_third_arg_message = false;
    for s in strings {
        for n in numbers {
            for k in 0..=n.len() {
                let mut argv = Argv::aliased(PROG, s, n, k);
                let out = assert_same_argv(&mut argv, 4, &format!("alias k={k} n={n:?}"));
                if out.stdout == b"Third argument must be an integer!" {
                    saw_third_arg_message = true;
                }
            }
        }
    }
    assert!(
        saw_third_arg_message,
        "the aliasing layout must be able to reach the third-argument message"
    );
}

/// C20 — many calls in one process, alternating configurations.
#[test]
fn cfg_c20_repeated_calls() {
    let rng = Rng::new(0xC20_0000_0001);
    for _ in 0..300 {
        let s = random_string_shape(&rng);
        let n_args = rng.range(1, 5); // 1..5 => argc 1..5, valid and invalid mixed
        let a = decorate_number(&rng, rng.range(0, 40) as i64);
        let b = decorate_number(&rng, rng.range(0, 40) as i64);
        let c = random_junk(&rng);
        let mut args: Vec<&[u8]> = vec![PROG];
        if n_args >= 2 {
            args.push(&s);
        }
        if n_args >= 3 {
            args.push(&a);
        }
        if n_args >= 4 {
            args.push(&b);
        }
        if n_args >= 5 {
            args.push(&c);
        }
        let layout = if rng.bool() {
            Layout::Contiguous
        } else {
            Layout::Separate
        };
        assert_same(&args, layout);
    }
}

/// C25 — hammer the `strtol` emulation: `argv[2]` and `argv[3]` are arbitrary
/// byte strings (digits, signs, whitespace, junk, high-bit bytes, any mix), so
/// the Rust reimplementation of `strtol(_, _, 10)` is compared against glibc's
/// on inputs nobody would think to hand-pick.
#[test]
fn cfg_c25_fuzz_numeric_arguments() {
    let rng = Rng::new(0xC25_0000_0001);
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    // Alphabets that make interesting strtol inputs likely.
    let alpha_digits = b"0123456789";
    let alpha_mixed = b"0123456789+- \t\n\r\x0b\x0cabcxX.,";
    for i in 0..6000u32 {
        let s = if i % 3 == 0 {
            random_string_shape(&rng)
        } else {
            random_bytes(&rng, rng.range(0, 24) as usize)
        };
        let gen = |kind: u64| -> Vec<u8> {
            let len = rng.range(0, 14) as usize;
            match kind {
                0 => (0..len).map(|_| *rng.pick(alpha_digits)).collect(),
                1 => (0..len).map(|_| *rng.pick(alpha_mixed)).collect(),
                2 => random_bytes(&rng, len),
                3 => {
                    let mut v: Vec<u8> = (0..rng.range(0, 3))
                        .map(|_| *rng.pick(b" \t\n\r\x0b\x0c"))
                        .collect();
                    let signs: &[&[u8]] = &[b"", b"+", b"-", b"++", b"--", b"+-"];
                    v.extend_from_slice(*rng.pick(signs));
                    v.extend((0..rng.range(0, 25)).map(|_| *rng.pick(alpha_digits)));
                    v.extend_from_slice(&random_junk(&rng));
                    v
                }
                _ => no_conversion_string(&rng),
            }
        };
        let a = gen(rng.below(5));
        let b = gen(rng.below(5));
        let layout = if rng.bool() {
            Layout::Contiguous
        } else {
            Layout::Separate
        };
        let out = if rng.bool() {
            assert_same(&[PROG, &s, &a], layout)
        } else {
            assert_same(&[PROG, &s, &a, &b], layout)
        };
        seen.insert(classify(&out));
    }
    // Prove the fuzz was not vacuous: it must have reached the success path and
    // every rejection that argc 3/4 can produce.
    for want in ["ok", "E3", "E4", "E6", "E7"] {
        assert!(seen.contains(want), "the fuzz never reached {want}: {seen:?}");
    }
}

/// Which branch of the C program an outcome came from.
fn classify(out: &Outcome) -> &'static str {
    match out.stdout.as_slice() {
        b"Second argument must be an integer!" => "E3",
        b"Error: start is off the end of the string!\n" => "E4",
        b"Third argument must be an integer!" => "E5",
        b"Error: stop is off the end of the string!\n" => "E6",
        b"Error: stop must come after start!\n" => "E7",
        s if s.starts_with(b"Error: there should be one to three") => "usage",
        _ => {
            assert_eq!(out.status, 0);
            "ok"
        }
    }
}

/// C26 — very long digit strings: hundreds of leading zeros, hundreds of
/// significant digits (deep inside the `strtol` overflow path).
#[test]
fn cfg_c26_long_digit_strings() {
    let rng = Rng::new(0xC26_0000_0001);
    let s: &[u8] = b"0123456789abcdef";
    for zeros in [1usize, 2, 17, 18, 19, 20, 30, 100, 500] {
        for value in [0u64, 1, 5, 16, 17, 4294967296, 9223372036854775807] {
            let mut n = vec![b'0'; zeros];
            n.extend_from_slice(format!("{value}").as_bytes());
            assert_same(&[PROG, s, &n], Layout::Contiguous);
            let mut m = vec![b'-'];
            m.extend(std::iter::repeat(b'0').take(zeros));
            m.extend_from_slice(format!("{value}").as_bytes());
            assert_same(&[PROG, s, &m], Layout::Contiguous);
            assert_same(&[PROG, s, b"0", &n], Layout::Contiguous);
        }
    }
    for digits in [19usize, 20, 21, 40, 200] {
        for _ in 0..20 {
            let n: Vec<u8> = (0..digits)
                .map(|_| *rng.pick(b"0123456789"))
                .collect();
            assert_same(&[PROG, s, &n], Layout::Contiguous);
            assert_same(&[PROG, s, b"1", &n], Layout::Contiguous);
            let mut m = vec![b'-'];
            m.extend_from_slice(&n);
            assert_same(&[PROG, s, &m], Layout::Contiguous);
            assert_same(&[PROG, s, b"1", &m], Layout::Contiguous);
        }
    }
}
