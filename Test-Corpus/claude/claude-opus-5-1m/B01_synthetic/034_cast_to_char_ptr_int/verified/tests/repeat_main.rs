//! Phase B — row C28 of CONFIGS.md: the exported `main` symbol called several
//! times in one process.
//!
//! This is the composed-pipeline case that per-call tests cannot see: glibc's
//! `scanf` keeps state in the `stdin` FILE object between calls (the byte it
//! pushes back with `ungetc` when the conversion stops, and the sticky EOF
//! indicator), so the second call continues exactly where the first stopped.
//! The translation has to reproduce that.

mod common;

use common::*;

/// Call `main` `n` times in a single child process and return everything it
/// printed (9 bytes per call).
fn call_main_n_times(which: &'static str, input: &[u8], n: usize, pipe: bool) -> Run {
    let p = pair();
    let f = if which == "C" { p.c.main } else { p.rs.main };
    let stdin = if pipe {
        Stdin::Pipe(input)
    } else {
        Stdin::File(input)
    };
    run_child(stdin, Stdout::File, move || {
        let mut rc = 0;
        for _ in 0..n {
            rc |= unsafe { f() };
        }
        rc
    })
}

#[track_caller]
fn diff_repeated(input: &[u8], n: usize, pipe: bool) {
    let c = call_main_n_times("C", input, n, pipe);
    let r = call_main_n_times("Rust", input, n, pipe);
    assert_eq!(
        (as_text(&c.out), c.status),
        (as_text(&r.out), r.status),
        "`main` x{n} diverged for input {} (pipe stdin: {pipe})",
        preview(input)
    );
    assert_eq!(
        c.out.len(),
        9 * n,
        "expected {n} records for {}",
        preview(input)
    );
}

/// C28 — several numbers in one stream: each call consumes one conversion.
#[test]
fn c28_repeated_main_multiple_numbers() {
    let cases: [&[u8]; 12] = [
        b"1 2 3",
        b"1 2 3\n",
        b"1\n2\n3\n",
        b"-1 -2 -3",
        b"+1 +2 +3",
        b"  10\t20\r\n30\x0b40 ",
        b"1234567890 -1234567890",
        b"2147483648 -2147483649",
        b"9223372036854775808 9223372036854775808",
        b"0 0 0 0 0",
        b"007 008 009",
        b"1  2   3    4",
    ];
    for input in cases {
        for n in [2usize, 3, 5] {
            diff_repeated(input, n, false);
        }
    }
}

/// C28 — the interesting part: the conversion stops on a *non-whitespace* byte,
/// which C pushes back, so the next call sees that byte again.
#[test]
fn c28_repeated_main_pushback_byte() {
    let cases: [&[u8]; 14] = [
        b"12x34",
        b"12x",
        b"12abc34",
        b"12.34",
        b"12-34",
        b"12+34",
        b"12,34",
        b"12\034",
        b"12\xff34",
        b"-12x-34",
        b"x12",
        b"-x12",
        b"12x34x56",
        b"2147483648x2147483648",
    ];
    for input in cases {
        for n in [2usize, 3, 4] {
            diff_repeated(input, n, false);
        }
    }
}

/// C28 — reading past EOF: the second call sees the sticky EOF indicator.
#[test]
fn c28_repeated_main_past_eof() {
    let cases: [&[u8]; 8] = [b"", b"   ", b"1", b"1 ", b"1\n", b"-", b"junk", b"1 2"];
    for input in cases {
        for n in [2usize, 3, 6] {
            diff_repeated(input, n, false);
        }
    }
}

/// C28 — same, with an unseekable pipe as stdin.
#[test]
fn c28_repeated_main_pipe_stdin() {
    let cases: [&[u8]; 6] = [b"1 2 3", b"12x34", b"", b"1", b"-5 -6", b"junk 7"];
    for input in cases {
        for n in [2usize, 3] {
            diff_repeated(input, n, true);
        }
    }
}

/// C28 — many calls (20) draining a long stream of tokens.
#[test]
fn c28_repeated_main_long_stream() {
    let mut rng = Rng::new(0xC028_0002);
    for _ in 0..25 {
        let mut input = String::new();
        for _ in 0..20 {
            let sign = rng.pick(&["", "-", "+"]);
            let len = 1 + rng.below(22) as usize;
            let sep = rng.pick(&[" ", "\n", "\t\t", "  ", "\r\n"]);
            input.push_str(&format!("{sign}{}{sep}", rng.digits(len)));
        }
        diff_repeated(input.as_bytes(), 20, false);
        // One extra call past the end of the stream.
        diff_repeated(input.as_bytes(), 22, false);
    }
}

/// C28 — repeated calls with an unreadable stdin (fd 0 closed): every call must
/// fail the same way.
#[test]
fn c28_repeated_main_unreadable_stdin() {
    let p = pair();
    for n in [1usize, 2, 4] {
        let c = run_child(Stdin::Closed, Stdout::File, || {
            let mut rc = 0;
            for _ in 0..n {
                rc |= unsafe { (p.c.main)() };
            }
            rc
        });
        let r = run_child(Stdin::Closed, Stdout::File, || {
            let mut rc = 0;
            for _ in 0..n {
                rc |= unsafe { (p.rs.main)() };
            }
            rc
        });
        assert_eq!(
            (as_text(&c.out), c.status),
            (as_text(&r.out), r.status),
            "`main` x{n} with fd 0 closed"
        );
        assert_eq!(c.out.len(), 9 * n);
    }
}

/// C28 — randomised streams of many tokens, consumed by many calls.
#[test]
fn c28_repeated_main_randomised_streams() {
    let mut rng = Rng::new(0xC028);
    for _ in 0..400 {
        let tokens = 1 + rng.below(6) as usize;
        let mut input = String::new();
        for _ in 0..tokens {
            let sep = rng.pick(&[" ", "\n", "\t", "", "x", ".", "\r\n", "  "]);
            let sign = rng.pick(&["", "-", "+"]);
            let len = 1 + rng.below(22) as usize;
            input.push_str(&format!("{sign}{}{sep}", rng.digits(len)));
        }
        diff_repeated(input.as_bytes(), 1 + rng.below(6) as usize, false);
    }
}
