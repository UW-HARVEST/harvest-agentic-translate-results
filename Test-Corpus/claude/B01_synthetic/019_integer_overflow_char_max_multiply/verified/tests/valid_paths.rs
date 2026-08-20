//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every assertion compares the C `.so` against the Rust `.so`, both loaded with
//! `libloading`; the `main` rows additionally compare the two standalone
//! executables. Randomized rows use the fixed seed in `common::SEED`.

mod common;

use common::*;
use std::os::raw::c_char;

// =========================================================== printers =====

/// C1 — exhaustive over the whole `char` domain.
#[test]
fn c1_print_hex_char_line_exhaustive_char_domain() {
    let pair = load_pair();
    for v in i8::MIN..=i8::MAX {
        assert_same(&pair, &format!("printHexCharLine({v})"), |lib| unsafe {
            (lib.print_hex_char_line)(v as c_char)
        });
    }
}

/// C2 — 2000 seeded random `i8` values.
#[test]
fn c2_print_hex_char_line_random_values() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC2);
    for _ in 0..2000 {
        let v = rng.i8();
        assert_same(&pair, &format!("printHexCharLine({v}) rnd"), |lib| unsafe {
            (lib.print_hex_char_line)(v as c_char)
        });
    }
}

/// C3 — random batches in a single capture: checks accumulated stream order.
#[test]
fn c3_print_hex_char_line_random_batches() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC3);
    for i in 0..500 {
        let n = rng.range_usize(1, 32);
        let vals: Vec<i8> = (0..n).map(|_| rng.i8()).collect();
        assert_same(&pair, &format!("printHexCharLine batch #{i} ({n})"), |lib| {
            for &v in &vals {
                unsafe { (lib.print_hex_char_line)(v as c_char) }
            }
        });
    }
}

/// C4 — the boundary values named by the C source.
#[test]
fn c4_print_hex_char_line_boundaries() {
    let pair = load_pair();
    // i8::MIN == CHAR_MIN, i8::MAX == CHAR_MAX, 63 == CHAR_MAX/2
    for v in [i8::MIN, -128 + 1, -2, -1, 0, 1, 2, 4, 62, 63, 64, 126, i8::MAX] {
        assert_same(&pair, &format!("printHexCharLine bound {v}"), |lib| unsafe {
            (lib.print_hex_char_line)(v as c_char)
        });
    }
}

/// C5 — the `line != NULL` guard.
#[test]
fn c5_print_line_null() {
    let pair = load_pair();
    assert_same(&pair, "printLine(NULL)", |lib| unsafe {
        (lib.print_line)(std::ptr::null())
    });
}

/// C6 — empty string, then every single-byte string.
#[test]
fn c6_print_line_empty_and_single_bytes() {
    let pair = load_pair();
    let empty = cstring(b"");
    assert_same(&pair, "printLine(\"\")", |lib| unsafe {
        (lib.print_line)(empty.as_ptr() as *const c_char)
    });
    for b in 1u8..=255 {
        let s = cstring(&[b]);
        assert_same(&pair, &format!("printLine(single 0x{b:02x})"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C7 — random printable-ASCII payloads.
#[test]
fn c7_print_line_random_ascii() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC7);
    for i in 0..1000 {
        let n = rng.range_usize(0, 64);
        let bytes: Vec<u8> = (0..n).map(|_| 0x20 + (rng.below(95) as u8)).collect();
        let s = cstring(&bytes);
        assert_same(&pair, &format!("printLine ascii #{i}"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C8 — random arbitrary (non-NUL) byte payloads: invalid UTF-8, `%`, controls.
#[test]
fn c8_print_line_random_arbitrary_bytes() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC8);
    for i in 0..1000 {
        let n = rng.range_usize(0, 64);
        let bytes: Vec<u8> = (0..n).map(|_| 1 + (rng.below(255) as u8)).collect();
        let s = cstring(&bytes);
        assert_same(&pair, &format!("printLine bytes #{i}"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C9 — lengths straddling the stdio buffer sizes.
#[test]
fn c9_print_line_length_sweep() {
    let pair = load_pair();
    const PATTERN: &[u8] = b"abcdefghij0123456789%s%n\x01\x7f\xc3\x28";
    for &len in &[
        0usize, 1, 2, 127, 128, 129, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193, 65535,
        65536, 65537,
    ] {
        let bytes: Vec<u8> = (0..len).map(|i| PATTERN[i % PATTERN.len()]).collect();
        let s = cstring(&bytes);
        assert_same(&pair, &format!("printLine len {len}"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C10 — random batches mixing NULL and non-NULL in one capture.
#[test]
fn c10_print_line_random_batches_with_nulls() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xCA);
    for i in 0..200 {
        let n = rng.range_usize(1, 16);
        // `None` means pass NULL.
        let items: Vec<Option<Vec<u8>>> = (0..n)
            .map(|_| {
                if rng.below(4) == 0 {
                    None
                } else {
                    let l = rng.range_usize(0, 24);
                    Some(cstring(
                        &(0..l).map(|_| 1 + (rng.below(255) as u8)).collect::<Vec<u8>>(),
                    ))
                }
            })
            .collect();
        assert_same(&pair, &format!("printLine batch #{i}"), |lib| {
            for it in &items {
                match it {
                    None => unsafe { (lib.print_line)(std::ptr::null()) },
                    Some(s) => unsafe { (lib.print_line)(s.as_ptr() as *const c_char) },
                }
            }
        });
    }
}

/// C11 — the two low-level printers interleaved in one stream.
#[test]
fn c11_printers_interleaved() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xCB);
    enum Op {
        Hex(i8),
        Line(Option<Vec<u8>>),
    }
    for i in 0..300 {
        let n = rng.range_usize(1, 12);
        let ops: Vec<Op> = (0..n)
            .map(|_| {
                if rng.bool() {
                    Op::Hex(rng.i8())
                } else if rng.below(6) == 0 {
                    Op::Line(None)
                } else {
                    let l = rng.range_usize(0, 20);
                    Op::Line(Some(cstring(
                        &(0..l).map(|_| 1 + (rng.below(255) as u8)).collect::<Vec<u8>>(),
                    )))
                }
            })
            .collect();
        assert_same(&pair, &format!("interleaved #{i}"), |lib| {
            for op in &ops {
                match op {
                    Op::Hex(v) => unsafe { (lib.print_hex_char_line)(*v as c_char) },
                    Op::Line(None) => unsafe { (lib.print_line)(std::ptr::null()) },
                    Op::Line(Some(s)) => unsafe {
                        (lib.print_line)(s.as_ptr() as *const c_char)
                    },
                }
            }
        });
    }
}

// ==================================================== bad / good =========

/// C12 — `bad()`, its only configuration.
#[test]
fn c12_bad_single_call() {
    let pair = load_pair();
    assert_same(&pair, "bad()", |lib| unsafe { (lib.bad)() });
}

/// C13 — `bad()` repeated in one capture.
#[test]
fn c13_bad_repeated() {
    let pair = load_pair();
    assert_same(&pair, "bad() x64", |lib| {
        for _ in 0..64 {
            unsafe { (lib.bad)() }
        }
    });
}

/// C14 — `good()`: `goodG2B` then `goodB2G`.
#[test]
fn c14_good_single_call() {
    let pair = load_pair();
    assert_same(&pair, "good()", |lib| unsafe { (lib.good)() });
}

/// C15 — `good()` repeated in one capture.
#[test]
fn c15_good_repeated() {
    let pair = load_pair();
    assert_same(&pair, "good() x64", |lib| {
        for _ in 0..64 {
            unsafe { (lib.good)() }
        }
    });
}

/// C16 — the composed wrappers interleaved.
#[test]
fn c16_good_bad_interleaved() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xD0);
    for i in 0..300 {
        let n = rng.range_usize(1, 10);
        let picks: Vec<bool> = (0..n).map(|_| rng.bool()).collect();
        assert_same(&pair, &format!("good/bad interleaved #{i}"), |lib| {
            for &g in &picks {
                if g {
                    unsafe { (lib.good)() }
                } else {
                    unsafe { (lib.bad)() }
                }
            }
        });
    }
}

// ============================================================ main ========

/// C17 / C18 — the two dispatch branches of `if (x)`.
#[test]
fn c17_c18_main_dispatch_branches() {
    assert_same_stdin("C17 zero", b"0");
    assert_same_stdin("C18 nonzero", b"1");
}

/// C19 — plain 1..9 digit numbers, EOF terminated.
#[test]
fn c19_main_plain_numbers() {
    let mut rng = Rng::new(SEED ^ 0xD1);
    for i in 0..200 {
        let n = rng.range_usize(1, 9);
        let mut s = Vec::new();
        for _ in 0..n {
            s.push(b'0' + rng.below(10) as u8);
        }
        assert_same_stdin(&format!("C19 #{i}"), &s);
    }
}

/// C20 — each C-locale whitespace byte x each sign x random digits.
#[test]
fn c20_main_leading_whitespace_and_sign() {
    let mut rng = Rng::new(SEED ^ 0xD2);
    for &ws in C_WHITESPACE.iter() {
        for sign in [None, Some(b'+'), Some(b'-')] {
            for _ in 0..3 {
                let mut s = vec![ws];
                if let Some(c) = sign {
                    s.push(c);
                }
                let n = rng.range_usize(1, 6);
                for _ in 0..n {
                    s.push(b'0' + rng.below(10) as u8);
                }
                assert_same_stdin(&format!("C20 ws=0x{ws:02x} sign={sign:?}"), &s);
            }
        }
    }
}

/// C21 — random runs of mixed whitespace before the number.
#[test]
fn c21_main_whitespace_runs() {
    let mut rng = Rng::new(SEED ^ 0xD3);
    for i in 0..60 {
        let wsn = rng.range_usize(0, 8);
        let mut s: Vec<u8> = (0..wsn).map(|_| *rng.pick(&C_WHITESPACE)).collect();
        let n = rng.range_usize(1, 6);
        for _ in 0..n {
            s.push(b'0' + rng.below(10) as u8);
        }
        assert_same_stdin(&format!("C21 #{i}"), &s);
    }
}

/// C22 — sign x digit-count class, spanning `int`, `long` and beyond.
#[test]
fn c22_main_digit_count_classes() {
    let mut rng = Rng::new(SEED ^ 0xD4);
    for sign in ["", "+", "-"] {
        for &n in &[1usize, 5, 10, 19, 20, 40] {
            for _ in 0..2 {
                let mut s = sign.as_bytes().to_vec();
                // First digit 1..9 so the length class is meaningful.
                s.push(b'1' + rng.below(9) as u8);
                for _ in 1..n {
                    s.push(b'0' + rng.below(10) as u8);
                }
                assert_same_stdin(&format!("C22 sign={sign:?} ndigits={n}"), &s);
            }
        }
    }
}

/// C23 — the magnitude classes around every width boundary.
#[test]
fn c23_main_magnitude_classes() {
    let mut cases: Vec<String> = Vec::new();
    for base in [
        "0",
        "1",
        "7",
        "2147483646",           // INT_MAX-1
        "2147483647",           // INT_MAX
        "2147483648",           // INT_MAX+1
        "4294967295",           // UINT_MAX
        "4294967296",           // 2^32  -> truncates to 0
        "4294967297",           // 2^32+1
        "8589934592",           // 2^33  -> truncates to 0
        "9223372036854775806",  // LONG_MAX-1
        "9223372036854775807",  // LONG_MAX
        "9223372036854775808",  // LONG_MAX+1 -> saturates
        "18446744073709551615", // ULONG_MAX
        "18446744073709551616", // 2^64
        "99999999999999999999",
    ] {
        cases.push(base.to_string());
        cases.push(format!("+{base}"));
        cases.push(format!("-{base}"));
    }
    for c in cases {
        assert_same_stdin(&format!("C23 {c}"), c.as_bytes());
    }
}

/// C24 — terminator shapes after a valid conversion.
#[test]
fn c24_main_terminators() {
    let mut rng = Rng::new(SEED ^ 0xD5);
    let tails: [&[u8]; 7] = [b"", b"\n", b"\n456", b"abc", b".", b" 789", b"\x00zz"];
    for tail in tails {
        for _ in 0..3 {
            let n = rng.range_usize(1, 6);
            let mut s = Vec::new();
            for _ in 0..n {
                s.push(b'0' + rng.below(10) as u8);
            }
            s.extend_from_slice(tail);
            assert_same_stdin(&format!("C24 tail={}", escape(tail)), &s);
        }
    }
}

/// C25 — leading-zero runs.
#[test]
fn c25_main_leading_zero_runs() {
    let mut rng = Rng::new(SEED ^ 0xD6);
    for &zeros in &[0usize, 1, 2, 20, 400] {
        for _ in 0..3 {
            let mut s = vec![b'0'; zeros];
            let n = rng.range_usize(0, 4);
            for _ in 0..n {
                s.push(b'0' + rng.below(10) as u8);
            }
            if s.is_empty() {
                s.push(b'0');
            }
            assert_same_stdin(&format!("C25 zeros={zeros}"), &s);
        }
    }
}

/// C26 — random blobs from a scanf-interesting alphabet.
#[test]
fn c26_main_random_interesting_alphabet() {
    const ALPHABET: &[u8] = b"0123456789+-\t\n\x0b\x0c\r abcxX.,eE\x00\xff\x80\xa0";
    let mut rng = Rng::new(SEED ^ 0xD7);
    for i in 0..400 {
        let n = rng.range_usize(0, 24);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHABET)).collect();
        assert_same_stdin_so_only(&format!("C26 #{i}"), &s);
        if i % 4 == 0 {
            // Also cross-check through the executables on a quarter of the cases.
            assert_same_stdin(&format!("C26 exe #{i}"), &s);
        }
    }
}

/// C27 — fully uniform random bytes.
#[test]
fn c27_main_random_uniform_bytes() {
    let mut rng = Rng::new(SEED ^ 0xD8);
    for i in 0..200 {
        let n = rng.range_usize(0, 64);
        let s: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        assert_same_stdin_so_only(&format!("C27 #{i}"), &s);
        if i % 4 == 0 {
            assert_same_stdin(&format!("C27 exe #{i}"), &s);
        }
    }
}

/// C28 / C29 / C30 — invocation style and stdout destination.
///
/// `assert_same_stdin` already runs every case through both the `.so` pair
/// (dlopen) and the executable pair with piped stdout (C28/C30); `assert_same`
/// runs the in-process cases with fd 1 pointed at a regular file (C29). This
/// test pins the remaining combination explicitly: `bad`/`good` driven through
/// a *subprocess* whose stdout is a pipe, so buffering mode differs from the
/// regular-file capture used everywhere else.
#[test]
fn c28_c29_c30_invocation_styles_and_stdout_destinations() {
    for sym in ["bad", "good"] {
        let mut c = std::process::Command::new(ffi_runner_path());
        c.arg(c_so_path()).arg(sym);
        let c_run = run_with_stdin(&mut c, b"");

        let mut r = std::process::Command::new(ffi_runner_path());
        r.arg(rust_so_path()).arg(sym);
        let r_run = run_with_stdin(&mut r, b"");

        assert_eq!(
            (
                escape(&c_run.stdout),
                escape(&c_run.stderr),
                c_run.status
            ),
            (
                escape(&r_run.stdout),
                escape(&r_run.stderr),
                r_run.status
            ),
            "subprocess+pipe mismatch for {sym}"
        );
    }

    // And the same two functions via in-process dlopen with fd 1 -> regular file.
    let pair = load_pair();
    assert_same(&pair, "C29 bad via file", |lib| unsafe { (lib.bad)() });
    assert_same(&pair, "C29 good via file", |lib| unsafe { (lib.good)() });
}
