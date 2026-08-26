//! Phase C — error-path differential tests of the complete programs
//! (ERRORS.md rows 1, 3, 4, 6–14, 16–18, 20 at entry point `EP3`).

mod common;

use common::{c_exe, run_exe, rust_exe, Rng, RunOut, SEED};

const TAG: &str = "cli_errors";

const E1: &str = "Error: should only be a single (integer) argument!\n";
const E2: &str = "Error: first argument must be an integer!\n";

fn both(args: &[Vec<u8>]) -> (RunOut, RunOut) {
    let c = run_exe(&c_exe(TAG), args);
    let r = run_exe(&rust_exe(), args);
    common::assert_same(&c, &r, args);
    (c, r)
}

/// C and Rust must agree *and* produce the documented rejection.
fn expect_error(args: &[Vec<u8>], expected: &str, ctx: &str) {
    let (c, _) = both(args);
    assert_eq!(
        c.code,
        Some(1),
        "{ctx}: expected exit status 1, got {:?} (signal {:?})",
        c.code,
        c.signal
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected,
        "{ctx}: wrong rejection message"
    );
    assert!(c.stderr.is_empty(), "{ctx}: unexpected stderr");
}

fn expect_ok(arg: &[u8], ctx: &str) {
    let (c, _) = both(&[arg.to_vec()]);
    assert_eq!(c.code, Some(0), "{ctx}: expected exit status 0");
    assert_eq!(
        c.stdout.iter().filter(|&&b| b == b'\n').count(),
        10,
        "{ctx}: expected 10 lines"
    );
}

// ------------------------------------------------------- ERRORS.md rows 1-4 --

#[test]
fn err_argc_1_cli() {
    expect_error(&[], E1, "row 1 (no operand)");
}

#[test]
fn err_argc_3_cli() {
    expect_error(&[b"1".to_vec(), b"2".to_vec()], E1, "row 3 (argc == 3)");
    expect_error(&[b"1".to_vec(), b"".to_vec()], E1, "row 3 (second empty)");
    expect_error(&[b"".to_vec(), b"".to_vec()], E1, "row 3 (both empty)");
    expect_error(
        &[b"abc".to_vec(), b"def".to_vec()],
        E1,
        "row 3 (both invalid)",
    );
}

#[test]
fn err_argc_many_cli() {
    let mut args: Vec<Vec<u8>> = vec![b"1".to_vec(), b"2".to_vec()];
    while args.len() <= 63 {
        args.push(format!("{}", args.len()).into_bytes());
        expect_error(&args, E1, &format!("row 4 (argc == {})", args.len() + 1));
    }
}

// ----------------------------------------------------- ERRORS.md rows 6-14 --

#[test]
fn err_no_conversion_cli() {
    let rejected: &[&[u8]] = &[
        // row 6: empty
        b"",
        // row 7: whitespace only
        b" ",
        b"\t",
        b"\n",
        b"\x0b",
        b"\x0c",
        b"\r",
        b"   ",
        b" \t\n\x0b\x0c\r",
        // row 8: sign only
        b"+",
        b"-",
        b"--",
        b"++",
        b"+-",
        b"-+",
        b"-+3",
        b"+-3",
        b"---5",
        // row 9: leading non-digit
        b"abc",
        b"x1",
        b".5",
        b"/9",
        b":0",
        b"e5",
        b"#",
        b"A",
        b"z",
        b"_1",
        b"(3)",
        b"'7'",
        b"%2",
        b"*3",
        b"$4",
        b"@5",
        b"~6",
        b"^7",
        b"&8",
        b"!9",
        b"?0",
        b"<1",
        b">2",
        b"=3",
        b"|4",
        b"\\5",
        b";6",
        b"`7",
        b"{8",
        b"}9",
        // row 10: sign then non-digit
        b"+a",
        b"-x",
        b"+ 1",
        b"- 1",
        b"+.1",
        b"-.1",
        b"+/",
        b"-:",
        // row 11: space, sign, non-digit
        b" +z",
        b"\t-",
        b" + 1",
        b" - 1",
        b"\n\r+q",
        b"  -  7",
        // row 12: prefixes without a leading digit
        b"x10",
        b"#10",
        b"b1",
        b"o7",
        b"h9",
        // row 14: separators
        b",",
        b"_",
        b"'",
        b",5",
        b"_5",
        b"'5",
        b".",
        b"..1",
    ];
    for arg in rejected {
        expect_error(
            &[arg.to_vec()],
            E2,
            &format!("rows 6-14 ({:?})", String::from_utf8_lossy(arg)),
        );
    }

    // Controls: these look similar but ARE accepted by the C code.
    for arg in [&b"0"[..], b"0x10", b"0b101", b"5abc", b" 7", b"+2", b"-0"] {
        expect_ok(arg, "rows 6-14 control (accepted)");
    }
}

/// The exact digit-range boundaries `'/' == '0' - 1` and `':' == '9' + 1`.
#[test]
fn err_digit_boundary_chars() {
    for b in [b'/', b':'] {
        expect_error(
            &[vec![b]],
            E2,
            &format!("row 9 (boundary char {:?})", b as char),
        );
        expect_error(
            &[vec![b, b'1']],
            E2,
            &format!("row 9 (boundary char {:?} then digit)", b as char),
        );
        expect_error(
            &[vec![b'+', b]],
            E2,
            &format!("row 10 (sign then boundary char {:?})", b as char),
        );
    }
    expect_ok(b"0", "row 9 control ('0')");
    expect_ok(b"9", "row 9 control ('9')");
    // A boundary char *after* a digit is trailing garbage, i.e. accepted.
    expect_ok(b"1/", "row 18 control ('1/')");
    expect_ok(b"1:", "row 18 control ('1:')");
}

// -------------------------------------------------------- ERRORS.md row 13 --

#[test]
fn err_non_utf8_arg() {
    for arg in [
        &b"\xff"[..],
        b"\x80",
        b"\x80\x81",
        b"\xc3\xa9",
        b"\xff1",
        b"\xa0 1",
        b"\xe2\x82\xac5",
        b"\xc2\xa0",
        b"\xff\xff\xff\xff",
    ] {
        expect_error(
            &[arg.to_vec()],
            E2,
            &format!("row 13 (high-bit bytes {arg:?})"),
        );
    }
    // High-bit bytes *after* a valid prefix are trailing garbage.
    for arg in [&b"7\xff"[..], b"5\xc3\xa9", b"-3\x80"] {
        expect_ok(arg, "row 13 control (garbage suffix)");
    }
}

// ---------------------------------------------------- ERRORS.md rows 16, 17 --

#[test]
fn err_range_saturation() {
    // `strtol` sets ERANGE and saturates; the program ignores it and truncates.
    for arg in [
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775809",
        "-9223372036854775810",
        "99999999999999999999999999999999999999",
        "-99999999999999999999999999999999999999",
        "2147483648",
        "-2147483649",
        "4294967296",
    ] {
        expect_ok(arg.as_bytes(), &format!("rows 16/17 ({arg})"));
    }
}

// -------------------------------------------------------- ERRORS.md row 20 --

/// stdout is a pipe whose read end is already closed: the C program dies from
/// `SIGPIPE`, so the Rust program must do the same (its runtime ignores
/// `SIGPIPE` by default, which `src/main.rs` undoes).
#[test]
fn err_epipe_kills_process() {
    use std::os::unix::io::FromRawFd;
    use std::process::{Command, Stdio};

    for arg in ["1", "1000000", "abc", ""] {
        let mut results = Vec::new();
        for exe in [c_exe(TAG), rust_exe()] {
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
            // Close the read end *before* the child runs: every write must fail.
            assert_eq!(unsafe { libc::close(fds[0]) }, 0);
            let stdout = unsafe { Stdio::from_raw_fd(fds[1]) };
            let status = Command::new(&exe)
                .arg(arg)
                .stdout(stdout)
                .status()
                .expect("spawn");
            results.push((status.code(), std::os::unix::process::ExitStatusExt::signal(&status)));
        }
        assert_eq!(
            results[0], results[1],
            "closed-pipe stdout: C {:?} vs Rust {:?} for arg {arg:?}",
            results[0], results[1]
        );
    }
}

/// stdout is `/dev/full`: every write fails with `ENOSPC`, which the C code
/// ignores (`printf`'s return value is discarded), so the exit status is 0.
#[test]
fn err_dev_full_ignored() {
    use std::process::{Command, Stdio};

    if !std::path::Path::new("/dev/full").exists() {
        return;
    }
    for arg in ["1", "-7", "abc", ""] {
        let mut results = Vec::new();
        for exe in [c_exe(TAG), rust_exe()] {
            let full = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .expect("open /dev/full");
            let out = Command::new(&exe)
                .arg(arg)
                .stdout(Stdio::from(full))
                .stderr(Stdio::piped())
                .output()
                .expect("spawn");
            results.push((
                out.status.code(),
                std::os::unix::process::ExitStatusExt::signal(&out.status),
                out.stderr,
            ));
        }
        assert_eq!(
            results[0], results[1],
            "/dev/full stdout: C vs Rust differ for arg {arg:?}"
        );
    }
}

// ------------------------------------------------------------ fuzz the gate --

/// Whatever accept/reject decision the C makes for a random argument, the Rust
/// program must make the same one, with the same message and status.
#[test]
fn err_randomized_reject_decision() {
    let mut rng = Rng::new(SEED ^ 0xC1);
    let alphabet: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabcxXeE.,_'/:\xff\x80\xc3\xa9()[]{}#$%&*";
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    for _ in 0..300 {
        let len = rng.below(8) as usize;
        let arg: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize])
            .collect();
        let (c, _) = both(&[arg.clone()]);
        match c.code {
            Some(1) => {
                assert_eq!(
                    String::from_utf8_lossy(&c.stdout),
                    E2,
                    "unexpected message for {arg:?}"
                );
                rejected += 1;
            }
            Some(0) => accepted += 1,
            other => panic!("unexpected exit status {other:?} for {arg:?}"),
        }
    }
    assert!(
        rejected > 10 && accepted > 10,
        "fuzzing should reach both outcomes (accepted = {accepted}, rejected = {rejected})"
    );
}
