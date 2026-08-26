//! Differential tests for the whole program (entry point `main`).
//!
//! Each case runs the **C** executable and the **Rust** executable with an
//! identical stdin configuration and requires byte-identical stdout, stderr,
//! exit code and terminating signal.
//!
//! Covers `CONFIGS.md` rows C1–C27 and `ERRORS.md` rows E3–E14 / G6.
//!
//! Note on cost: a SIGSEGV costs ~0.4 s in this environment because
//! `kernel.core_pattern` pipes into `systemd-coredump` (not suppressible
//! without root, and `RLIMIT_CORE`/`PR_SET_DUMPABLE` do not bypass a pipe
//! pattern). Fault-path cases are therefore sampled with a deliberate budget
//! rather than swept exhaustively; every other case is ~2 ms.

mod common;

use common::{c_exe, run_tty, Diff, In, Rng};

/// Decimal spelling helper.
fn line(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(b'\n');
    v
}

// ---------------------------------------------------------------------------
// C1..C5, C7 (non-negative half), C12..C19, C26 — the cheap, non-faulting bulk
// ---------------------------------------------------------------------------

/// C7 (non-negative half) + C1 + C2 + C3 + C4 + C5: exhaustive sweep of every
/// value from 0 to 300, which covers the whole copying range (0..=99), the
/// `data < 100` boundary (99/100) and the rejected range (100..=300).
#[test]
fn c7_exhaustive_non_negative_0_to_300() {
    let mut d = Diff::new("C1-C5,C7 exhaustive 0..=300");
    for v in 0..=300i32 {
        d.check_line(&line(&v.to_string()));
    }
    d.finish();
}

/// C1/C2: alternative spellings of in-range values — zero padding, leading
/// zeros, and no-newline variants must parse the same.
#[test]
fn c1_c2_alternative_spellings() {
    let mut d = Diff::new("C1,C2 alternative spellings");
    let mut rng = Rng::with_seed(0xA11CE);
    for _ in 0..200 {
        let v = rng.range_i64(0, 99);
        let pad = rng.below(6) as usize;
        let s = format!("{}{}", "0".repeat(pad), v);
        // Only spellings that still fit the 13-byte fgets window.
        if s.len() <= 13 {
            d.check_line(&line(&s));
        }
        // Same value with no trailing newline at all (C18).
        if s.len() <= 13 {
            d.check_run(
                &format!("no-newline {s:?}"),
                &In::Pipe(s.as_bytes().to_vec()),
            );
        }
    }
    for s in ["0", "00", "000000000", "0000000000000", "1", "01", "099", "0099"] {
        d.check_line(&line(s));
        d.check_run(&format!("no-newline {s:?}"), &In::Pipe(s.as_bytes().to_vec()));
    }
    d.finish();
}

/// C5: values well above the `data < 100` cut, including `INT_MAX` and random
/// large positives.
#[test]
fn c5_above_the_range_check() {
    let mut d = Diff::new("C5 data >= 100");
    let mut rng = Rng::with_seed(0xB0B);
    for s in [
        "100", "101", "150", "999", "1000", "65535", "65536", "100000",
        "1000000", "2147483646", "2147483647",
    ] {
        d.check_line(&line(s));
    }
    for _ in 0..150 {
        let v = rng.range_i64(100, 9_999_999);
        d.check_line(&line(&v.to_string()));
    }
    d.finish();
}

/// C9/C10: leading whitespace and an explicit `+` sign — glibc's `atoi` skips
/// `[ \t\n\v\f\r]` and accepts one optional sign.
#[test]
fn c9_c10_whitespace_and_plus_sign() {
    let mut d = Diff::new("C9,C10 whitespace and '+'");
    let ws = [" ", "\t", "\x0b", "\x0c", "\r", "  ", " \t", "\t ", "\r\n"];
    for w in ws {
        for v in ["0", "1", "50", "99", "100", "12345"] {
            let s = format!("{w}{v}");
            if s.len() <= 13 {
                d.check_line(s.as_bytes());
            }
            let s = format!("{w}+{v}");
            if s.len() <= 13 {
                d.check_line(s.as_bytes());
            }
        }
    }
    let mut rng = Rng::with_seed(0xC0FFEE);
    for _ in 0..150 {
        let v = rng.range_i64(0, 999);
        let w = *rng.pick(&ws);
        let sign = if rng.below(2) == 0 { "+" } else { "" };
        let s = format!("{w}{sign}{v}");
        if s.len() <= 13 {
            d.check_line(s.as_bytes());
        }
    }
    d.finish();
}

/// C12: non-numeric garbage — `atoi` yields 0 with no error signal (E5, E11).
#[test]
fn c12_non_numeric_garbage() {
    let mut d = Diff::new("C12 non-numeric input (E5,E11)");
    for s in [
        "", "abc", "ABC", "0x1F", "0X1f", "x5", ".", ".5", "-.5", "+", "-",
        "++5", "--5", "+-5", "- 5", "+ 5", "   ", "\t\t", "\n", "\r", "e10",
        "NaN", "inf", "null", "%d", "%s%s", "'", "\"", "\\", "/", "*", "~",
        "!5", "#5", "$5", "&5", "(5)", "[5]", "{5}", "<5>", "5,5", ";", ":",
    ] {
        d.check_line(s.as_bytes());
    }
    // Random printable junk that never starts with '-' (kept off the fault
    // path so this row stays cheap; the '-' case is row C11).
    let mut rng = Rng::with_seed(0xD15EA5E);
    let alphabet: Vec<u8> = (0x20u8..0x7f).filter(|c| *c != b'-').collect();
    for _ in 0..200 {
        let len = rng.below(14) as usize;
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(&alphabet)).collect();
        d.check_run(
            &format!("junk {:?}", String::from_utf8_lossy(&s)),
            &In::Pipe({
                let mut v = s.clone();
                v.push(b'\n');
                v
            }),
        );
    }
    d.finish();
}

/// C13: digits followed by trailing junk — `atoi` stops at the first
/// non-digit and reports no error.
#[test]
fn c13_digits_then_trailing_junk() {
    let mut d = Diff::new("C13 digits + trailing junk");
    for s in [
        "50abc", "7 8", "12.9", "99x", "100y", "0z", "5+5", "5-5", "42\t9",
        "3;4", "1,000", "8/9", "0.0", "99.99", "7e3", "12 34 56",
    ] {
        d.check_line(s.as_bytes());
    }
    let mut rng = Rng::with_seed(0xE1E1);
    let junk: Vec<u8> = (0x20u8..0x7f).filter(|c| !c.is_ascii_digit()).collect();
    for _ in 0..150 {
        let v = rng.range_i64(0, 199);
        let tail_len = rng.below(6) as usize;
        let tail: Vec<u8> = (0..tail_len).map(|_| *rng.pick(&junk)).collect();
        let mut s = v.to_string().into_bytes();
        s.extend_from_slice(&tail);
        s.truncate(13);
        s.push(b'\n');
        d.check_run(&format!("digits+junk {:?}", String::from_utf8_lossy(&s)), &In::Pipe(s));
    }
    d.finish();
}

/// C14: an embedded NUL byte — `fgets` stores it, `atoi` stops there.
#[test]
fn c14_embedded_nul() {
    let mut d = Diff::new("C14 embedded NUL");
    let cases: Vec<Vec<u8>> = vec![
        b"\0\n".to_vec(),
        b"\0".to_vec(),
        b"5\0 9\n".to_vec(),
        b"\0 -1\n".to_vec(),
        b"\0005\n".to_vec(),
        b"12\0 34\n".to_vec(),
        b" \0 50\n".to_vec(),
        b"99\0\n".to_vec(),
        b"\0\0\0\n".to_vec(),
        b"100\0\n".to_vec(),
    ];
    for c in cases {
        d.check_run(&format!("nul-case {c:?}"), &In::Pipe(c));
    }
    d.finish();
}

/// C15 / E10: values above `INT_MAX` that still fit `long` — `atoi` is
/// `(int)strtol`, so the value is silently truncated. Some truncations land
/// negative and therefore fault, which is exactly the interaction being
/// checked; the sample size is kept modest for that reason.
#[test]
fn c15_int_overflow_truncation() {
    let mut d = Diff::new("C15,E10 int truncation");
    for s in [
        "2147483648",    // INT_MAX+1 -> INT_MIN (faults)
        "4294967296",    // 2^32      -> 0
        "4294967301",    // 2^32+5    -> 5
        "4294967396",    // 2^32+100  -> 100
        "8589934592",    // 2^33      -> 0
        "9999999999999", // 13 digits
        "1234567890123",
        "9876543210",
    ] {
        d.check_line(&line(s));
    }
    let mut rng = Rng::with_seed(0xF00D);
    for _ in 0..40 {
        let digits = rng.range_i64(10, 13) as usize;
        let mut s = String::new();
        s.push(char::from(b'1' + rng.below(9) as u8));
        for _ in 1..digits {
            s.push(char::from(b'0' + rng.below(10) as u8));
        }
        d.check_line(&line(&s));
    }
    d.finish();
}

/// C16/C17/E9: the 14-byte `fgets` window — exactly 13 bytes, and longer
/// input that is silently truncated with the remainder left unread.
#[test]
fn c16_c17_fgets_buffer_window() {
    let mut d = Diff::new("C16,C17,E9 fgets 14-byte window");
    // Exactly 13 bytes (no room for the newline).
    for s in [
        "1234567890123",
        "0000000000050",
        "0000000000099",
        "0000000000100",
        "             ",
        "aaaaaaaaaaaaa",
    ] {
        assert_eq!(s.len(), 13);
        d.check_line(&line(s)); // newline lands past the window
        d.check_run(&format!("exact13 {s:?}"), &In::Pipe(s.as_bytes().to_vec()));
    }
    // Longer than the window: the tail must be ignored.
    for s in [
        "1234567890123456789",
        "00000000000500000000",
        "50 followed by a lot of text that cannot fit",
        "0000000000000000000000000000099",
    ] {
        d.check_line(s.as_bytes());
    }
    let mut rng = Rng::with_seed(0x1234_5678);
    for _ in 0..120 {
        let len = rng.range_i64(14, 40) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| b'0' + rng.below(10) as u8)
            .collect();
        let mut v = s.clone();
        v.push(b'\n');
        d.check_run(&format!("long {:?}", String::from_utf8_lossy(&s)), &In::Pipe(v));
    }
    d.finish();
}

/// C19/C26: an empty first line, and multi-line input where only the first
/// line may be consumed.
#[test]
fn c19_c26_empty_and_multiline() {
    let mut d = Diff::new("C19,C26 empty line / multi-line");
    let cases: Vec<Vec<u8>> = vec![
        b"\n".to_vec(),
        b"\n\n".to_vec(),
        b"\n50\n".to_vec(),
        b"50\n99\n".to_vec(),
        b"50\n-1\n".to_vec(),
        b"\n-1\n".to_vec(),
        b"100\n50\n".to_vec(),
        b"7\nrest of the file is ignored\n".to_vec(),
        b"\r\n50\n".to_vec(),
        b"0\n0\n0\n".to_vec(),
    ];
    for c in cases {
        d.check_run(&format!("multiline {c:?}"), &In::Pipe(c));
    }
    d.finish();
}

/// C22: a regular file as stdin instead of a pipe.
#[test]
fn c22_regular_file_stdin() {
    let mut d = Diff::new("C22 regular-file stdin");
    for s in ["0", "1", "50", "99", "100", "1000", "abc", "", "  12", "+7"] {
        d.check_run(&format!("file {s:?}"), &In::File(line(s)));
    }
    // One fault case through a file, to confirm the stdin kind does not change
    // the fault behavior.
    d.check_run("file \"-5\"", &In::File(line("-5")));
    d.finish();
}

// ---------------------------------------------------------------------------
// C27 — broad random-byte fuzzing
// ---------------------------------------------------------------------------

/// C27: raw random byte lines over the whole `0x00..=0xff` alphabet. Lines
/// beginning with `-` (after whitespace) fault, which is rare in a uniform
/// sample, keeping this row cheap.
#[test]
fn c27_random_byte_fuzz() {
    let mut d = Diff::new("C27 random raw-byte fuzz");
    let mut rng = Rng::with_seed(0x0F1F_2F3F_4F5F_6F7F);
    for _ in 0..600 {
        let len = rng.below(41) as usize;
        let mut v: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        v.push(b'\n');
        d.check_run(&format!("fuzz {v:?}"), &In::Pipe(v));
    }
    d.finish();
}

/// C27 (numeric-shaped variant): random tokens assembled from the pieces
/// `atoi` actually distinguishes — whitespace, sign, digits, junk — so the
/// parser's branches are hit far more often than by uniform bytes.
#[test]
fn c27_structured_numeric_fuzz() {
    let mut d = Diff::new("C27 structured numeric fuzz");
    let mut rng = Rng::with_seed(0x5EED_BEEF);
    let ws = ["", " ", "\t", "  ", "\r", "\x0b", "\x0c"];
    let signs = ["", "+"]; // '-' is exercised by C11 within its budget
    let junks = ["", "x", ".", " ", "abc", "!", "99"];
    for _ in 0..400 {
        let w = *rng.pick(&ws);
        let s = *rng.pick(&signs);
        let ndigits = rng.below(6) as usize;
        let digits: String = (0..ndigits)
            .map(|_| char::from(b'0' + rng.below(10) as u8))
            .collect();
        let j = *rng.pick(&junks);
        let mut token = format!("{w}{s}{digits}{j}");
        token.truncate(13);
        d.check_line(token.as_bytes());
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// C6, C11, C20, C21 — the fault paths (budgeted: ~0.9 s per case)
// ---------------------------------------------------------------------------

/// C6/E6/E7: negative `data` makes `(size_t)data` huge, so `strncpy`'s
/// NUL-padding runs off the 100-byte stack buffer and the process dies of
/// SIGSEGV with its buffered stdout discarded.
#[test]
fn c6_negative_values_fault() {
    let mut d = Diff::new("C6,E6,E7 negative -> SIGSEGV");
    for s in [
        "-1", "-2", "-3", "-5", "-99", "-100", "-101", "-128", "-255", "-256",
        "-300", "-1000", "-65535", "-65536", "-1000000", "-2147483647",
        "-2147483648",
    ] {
        d.check_line(&line(s));
    }
    // A short exhaustive run of small magnitudes.
    for v in 1..=12i32 {
        d.check_line(&line(&format!("-{v}")));
    }
    d.finish();
}

/// C11: an explicit `-` sign combined with leading whitespace and zero
/// padding — all must reach the same fault.
#[test]
fn c11_minus_sign_spellings() {
    let mut d = Diff::new("C11 '-' spellings -> SIGSEGV");
    for s in [
        "-0000000005", " -5", "\t-5", "\r-5", "  -1", "-5abc", "-5 9",
        "-0000000001", "-9999999999999",
    ] {
        d.check_line(s.as_bytes());
    }
    // "-0" parses to 0 and must NOT fault — the sign alone is not enough.
    for s in ["-0", "-00", "-0000000000000", " -0"] {
        d.check_line(&line(s));
    }
    d.finish();
}

/// C8/G6: random samples from the whole `i32` domain, i.e. arbitrary
/// out-of-range values crossing the boundary into `data`.
#[test]
fn c8_random_i32_domain() {
    let mut d = Diff::new("C8,G6 random i32 domain");
    let mut rng = Rng::with_seed(0x9E37_79B9);
    for _ in 0..80 {
        let v = rng.next_u32() as i32;
        d.check_line(&line(&v.to_string()));
    }
    d.finish();
}

/// C20/C21/E3/E4: `fgets` returning NULL — end of file with nothing to read,
/// and a closed descriptor 0. Both print the diagnostic into a *buffered*
/// stdout and then fault, so the message is lost.
#[test]
fn c20_c21_fgets_failure() {
    let mut d = Diff::new("C20,C21,E3,E4 fgets() failure");
    d.check_run("/dev/null stdin", &In::DevNull);
    d.check_run("empty pipe stdin", &In::Pipe(Vec::new()));
    d.check_run("empty file stdin", &In::File(Vec::new()));
    d.check_run("closed fd 0", &In::ClosedFd);
    d.finish();
}

// ---------------------------------------------------------------------------
// C23, C24, C25 — stdout buffering discipline
// ---------------------------------------------------------------------------

/// C23/C24: stdout on a **pty**, which makes C's stdout line buffered. This is
/// the mirror image of C20: the `"fgets() failed."` line *is* written before
/// the fault, so a translation that buffers unconditionally would diverge here.
#[test]
fn c23_c24_tty_line_buffering() {
    if !common::have_program("script") {
        eprintln!("skipping C23/C24: script(1) not available");
        return;
    }
    let mut d = Diff::new("C23,C24 pty (line-buffered) stdout");

    // C23: fgets fails, message flushed, then the fault.
    let c = run_tty(&c_exe(), "/dev/null");
    let r = run_tty(&common::rust_exe(), "/dev/null");
    d.check("pty + /dev/null stdin", &c, &r);
    assert!(
        c.stdout.starts_with(b"fgets() failed."),
        "sanity: line-buffered C should emit the diagnostic before faulting, got {:?}",
        String::from_utf8_lossy(&c.stdout)
    );

    // C24: ordinary values over a pty.
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("c_build");
    std::fs::create_dir_all(&dir).unwrap();
    for s in ["0", "1", "5", "50", "99", "100", "1000", "abc", "-7"] {
        let path = dir.join(format!("tty_stdin_{}_{s}.txt", std::process::id()));
        std::fs::write(&path, line(s)).unwrap();
        let p = path.to_str().unwrap();
        let c = run_tty(&c_exe(), p);
        let r = run_tty(&common::rust_exe(), p);
        d.check(&format!("pty stdin={s:?}"), &c, &r);
        let _ = std::fs::remove_file(&path);
    }
    d.finish();
}

/// C25: stdout redirected to a **regular file** (fully buffered, like a pipe,
/// but a different descriptor type).
#[test]
fn c25_stdout_to_regular_file() {
    use std::process::{Command, Stdio};
    let mut d = Diff::new("C25 stdout to a regular file");
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("c_build");
    std::fs::create_dir_all(&dir).unwrap();

    let run_to_file = |exe: &std::path::Path, input: &[u8], tag: &str| -> common::Outcome {
        use std::io::Write;
        use std::os::unix::process::ExitStatusExt;
        let out_path = dir.join(format!("stdout_{}_{tag}.bin", std::process::id()));
        let f = std::fs::File::create(&out_path).unwrap();
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(f))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let _ = child.stdin.take().unwrap().write_all(input);
        let out = child.wait_with_output().unwrap();
        let stdout = std::fs::read(&out_path).unwrap();
        let _ = std::fs::remove_file(&out_path);
        common::Outcome {
            stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };

    for (i, s) in ["0", "1", "50", "99", "100", "abc", "", "-3"].iter().enumerate() {
        let input = line(s);
        let c = run_to_file(&c_exe(), &input, &format!("c{i}"));
        let r = run_to_file(&common::rust_exe(), &input, &format!("r{i}"));
        d.check(&format!("stdout=file stdin={s:?}"), &c, &r);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Harness sanity — guards against "both sides equally broken"
// ---------------------------------------------------------------------------

/// If the harness were misconfigured (wrong path, empty output everywhere), the
/// differential checks above would pass vacuously. Pin the C side's actual
/// observable behavior so that cannot happen silently.
#[test]
fn harness_sanity_c_behaviour_is_as_documented() {
    let c = |s: &[u8]| common::run(&c_exe(), &In::Pipe(s.to_vec()));

    let five = c(b"5\n");
    assert_eq!(five.stdout, b"AAAAA\n", "C should print 5 'A's for input 5");
    assert_eq!(five.code, Some(0));
    assert_eq!(five.signal, None);

    let zero = c(b"0\n");
    assert_eq!(zero.stdout, b"\n");
    assert_eq!(zero.code, Some(0));

    let ninetynine = c(b"99\n");
    assert_eq!(ninetynine.stdout, [b"A".repeat(99), b"\n".to_vec()].concat());

    let hundred = c(b"100\n");
    assert_eq!(hundred.stdout, b"\n", "data >= 100 skips the copy");

    let neg = c(b"-1\n");
    assert_eq!(neg.signal, Some(11), "negative length must fault (SIGSEGV)");
    assert_eq!(neg.code, None);
    assert!(neg.stdout.is_empty(), "buffered stdout is lost on the fault");

    let eof = common::run(&c_exe(), &In::DevNull);
    assert_eq!(eof.signal, Some(11));
    assert!(
        eof.stdout.is_empty(),
        "the fgets diagnostic is buffered and lost when stdout is a pipe"
    );
}
