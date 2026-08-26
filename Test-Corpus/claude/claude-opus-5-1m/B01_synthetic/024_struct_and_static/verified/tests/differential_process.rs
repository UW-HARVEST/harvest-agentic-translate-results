//! Phase B — valid-path differential tests at the process level.
//!
//! Every row of `CONFIGS.md` that drives the `main` entry point lives here.
//! The C reference executable (`c_src/build/driver`) and the Rust executable
//! (`target/*/driver`) are both spawned with identical stdin/argv and their
//! stdout, stderr and exit status are compared byte for byte.

mod common;

use common::*;
use std::io::Write;

fn setup() {
    ensure_c_artifacts();
}

/// CONFIGS rows 1-6: baseline values and line terminators.
#[test]
fn row01_06_basic_values_and_terminators() {
    setup();
    let cases: &[&str] = &[
        "0\n",   // row 1
        "3\n",   // row 2
        "-4\n",  // row 3
        "+7\n",  // row 4
        "3",     // row 5: EOF right after the digits
        "3\r\n", // row 6
        "0",
        "-0\n",
        "+0\n",
        "1\n",
        "-1\n",
        "42\n",
        "-42\n",
        "\n",
        "",
    ];
    for c in cases {
        diff_input("basic", c.as_bytes());
    }
}

/// CONFIGS row 7: every whitespace class `isspace` skips, alone and mixed.
#[test]
fn row07_whitespace_prefixes() {
    setup();
    let ws = [" ", "\t", "\n", "\x0b", "\x0c", "\r"];
    for w in ws.iter() {
        for n in [1usize, 2, 5] {
            let prefix = w.repeat(n);
            diff_input("ws-prefix", format!("{prefix}5\n").as_bytes());
            diff_input("ws-prefix-neg", format!("{prefix}-5\n").as_bytes());
            diff_input("ws-only", prefix.as_bytes());
        }
    }
    // Mixed whitespace, deterministic order.
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    for i in 0..100 {
        let mut s = String::new();
        for _ in 0..rng.below(12) {
            s.push_str(rng.pick(&ws));
        }
        s.push_str(&format!("{}", (rng.next_u32() as i32) / 3));
        if i % 2 == 0 {
            s.push_str(rng.pick(&ws));
        }
        diff_input("ws-mixed", s.as_bytes());
    }
}

/// CONFIGS row 8: the token straddles the 4096-byte stdio buffer boundary.
#[test]
fn row08_buffer_boundary() {
    setup();
    for pad in [4090usize, 4094, 4095, 4096, 4097, 4098, 8191, 8192, 8193] {
        let mut s = " ".repeat(pad);
        s.push_str("-1234567\n");
        diff_input("buffer-boundary", s.as_bytes());

        // A digit run that itself spans the boundary.
        let mut s2 = " ".repeat(pad - 3);
        s2.push_str("1234567890\n");
        diff_input("buffer-boundary-digits", s2.as_bytes());
    }
}

/// CONFIGS row 9: leading zeros and digit-run lengths 1..=25.
#[test]
fn row09_digit_runs() {
    setup();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for len in 1..=25usize {
        for sign in ["", "+", "-"] {
            let mut digits = String::new();
            for i in 0..len {
                let d = if i == 0 {
                    // avoid an all-zero prefix half of the time
                    b'1' + (rng.below(9) as u8)
                } else {
                    b'0' + (rng.below(10) as u8)
                };
                digits.push(d as char);
            }
            diff_input("digit-run", format!("{sign}{digits}\n").as_bytes());
            diff_input(
                "digit-run-zeros",
                format!("{sign}{}{digits}\n", "0".repeat(len)).as_bytes(),
            );
        }
    }
    for z in [1usize, 5, 18, 19, 20, 40] {
        diff_input("zeros-then-5", format!("{}5\n", "0".repeat(z)).as_bytes());
        diff_input("all-zeros", format!("{}\n", "0".repeat(z)).as_bytes());
        diff_input("neg-zeros", format!("-{}7\n", "0".repeat(z)).as_bytes());
    }
}

/// CONFIGS row 10: trailing garbage that `%d` stops at.
#[test]
fn row10_trailing_garbage() {
    setup();
    let cases: &[&str] = &[
        "5abc", "5abc\n", "5.5\n", "5,\n", "5-\n", "5+\n", "5 x\n", "12e5\n", "0x10\n", "0X10\n",
        "-7q\n", "+7q\n", "9]\n", "3\0 9\n", "8\t\tzz\n",
    ];
    for c in cases {
        diff_input("trailing-garbage", c.as_bytes());
    }
}

/// CONFIGS row 11: only the first token is consumed.
#[test]
fn row11_multiple_tokens() {
    setup();
    let cases: &[&str] = &[
        "1 2 3\n",
        "1\n2\n",
        "-5 -6\n",
        "7\t8\n",
        "10 20 30 40 50\n",
        "1 abc\n",
        "1 99999999999999999999\n",
    ];
    for c in cases {
        diff_input("multi-token", c.as_bytes());
    }
}

/// CONFIGS row 12: exact `int` boundaries.
#[test]
fn row12_int_boundaries() {
    setup();
    let cases: &[&str] = &[
        "2147483647",
        "2147483646",
        "-2147483648",
        "-2147483647",
        "2147483647\n",
        "-2147483648\n",
        "+2147483647\n",
        "32767\n",
        "-32768\n",
        "65535\n",
        "65536\n",
        "255\n",
        "256\n",
    ];
    for c in cases {
        diff_input("int-boundary", c.as_bytes());
    }
}

/// CONFIGS row 13: beyond `int`, beyond `long`.
#[test]
fn row13_long_boundaries() {
    setup();
    let cases: &[&str] = &[
        "2147483648",
        "2147483649",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999",
        "-99999999999999999999",
        "18446744073709551615",
        "18446744073709551616",
        "123456789012345678901234567890",
        "-123456789012345678901234567890",
    ];
    for c in cases {
        diff_input("long-boundary", c.as_bytes());
        diff_input("long-boundary-nl", format!("{c}\n").as_bytes());
    }
}

/// CONFIGS row 14: 400 randomized `i32` values with randomized presentation.
#[test]
fn row14_randomized_int_values() {
    setup();
    let mut rng = Rng::new(0xDEAD_BEEF_0000_0001);
    let ws = [" ", "\t", "\n", "\x0b", "\x0c", "\r", ""];
    for _ in 0..400 {
        let v = rng.next_u32() as i32;
        let mut s = String::new();
        for _ in 0..rng.below(3) {
            s.push_str(rng.pick(&ws));
        }
        if v >= 0 && rng.below(2) == 0 {
            s.push('+');
        }
        // Optional leading zeros on the magnitude.
        if v >= 0 {
            s.push_str(&"0".repeat(rng.below(4) as usize));
            s.push_str(&format!("{v}"));
        } else {
            s.push('-');
            s.push_str(&"0".repeat(rng.below(4) as usize));
            s.push_str(&format!("{}", (v as i64).unsigned_abs()));
        }
        s.push_str(rng.pick(&ws));
        diff_input("random-int", s.as_bytes());
    }
}

/// CONFIGS row 15: 300 randomized junk byte strings (including NULs and
/// high bytes) — the value-independent parsing paths.
#[test]
fn row15_randomized_junk() {
    setup();
    let mut rng = Rng::new(0xFEED_FACE_CAFE_0002);
    let alphabet: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabcxyzXYZ.,;:*/\\'\"%$#@!()[]{}\0\x7f\x80\xff";
    for _ in 0..300 {
        let len = rng.below(24) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(rng.pick(alphabet));
        }
        diff_input("random-junk", &buf);
    }
    // Fully random bytes.
    for _ in 0..200 {
        let len = rng.below(16) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push((rng.next_u32() & 0xFF) as u8);
        }
        diff_input("random-bytes", &buf);
    }
}

/// CONFIGS rows 16-17: stdout is a regular file / `/dev/null` instead of a pipe
/// (C's stdio picks full buffering; the bytes must be identical anyway).
#[test]
fn row16_17_stdout_destinations() {
    setup();
    let dir = std::env::temp_dir();
    for input in ["0\n", "9\n", "-3\n", "2147483647\n", "abc"] {
        let mut outs = Vec::new();
        for (tag, exe) in [("c", c_exe()), ("rust", rust_exe())] {
            let path = dir.join(format!("driver_stdout_{tag}_{}.txt", std::process::id()));
            let file = std::fs::File::create(&path).expect("create temp stdout");
            let mut stdin_path = dir.join(format!("driver_stdin_{tag}_{}.txt", std::process::id()));
            {
                let mut f = std::fs::File::create(&stdin_path).expect("create temp stdin");
                f.write_all(input.as_bytes()).unwrap();
            }
            let stdin = std::fs::File::open(&stdin_path).unwrap();
            let outcome = run_prog_with(&exe, stdin.into(), file.into(), &[]);
            let bytes = std::fs::read(&path).expect("read temp stdout");
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(&mut stdin_path);
            outs.push((outcome, bytes));
        }
        assert_eq!(
            outs[0].1, outs[1].1,
            "stdout-to-file bytes differ for input {input:?}"
        );
        assert_eq!(
            (outs[0].0.code, outs[0].0.signal, &outs[0].0.stderr),
            (outs[1].0.code, outs[1].0.signal, &outs[1].0.stderr),
            "stdout-to-file status differs for input {input:?}"
        );

        // /dev/null
        let mut res = Vec::new();
        for exe in [c_exe(), rust_exe()] {
            let devnull = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .unwrap();
            let stdin = std::process::Stdio::null();
            res.push(run_prog_with(&exe, stdin, devnull.into(), &[]));
        }
        assert_eq!(
            (res[0].code, res[0].signal, &res[0].stderr),
            (res[1].code, res[1].signal, &res[1].stderr),
            "/dev/null run differs"
        );
    }
}

/// CONFIGS row 18: `int main()` ignores argv.
#[test]
fn row18_extra_argv() {
    setup();
    for args in [
        vec!["foo"],
        vec!["-x", "--help"],
        vec!["1", "2", "3"],
        vec![""],
    ] {
        diff_input_args("argv", b"6\n", &args);
        diff_input_args("argv-empty-stdin", b"", &args);
    }
}

/// CONFIGS row 28: a seekable stdin — C's `stdio` hands the read-ahead bytes
/// back by seeking to the logically consumed position when the process exits,
/// so `{ driver >/dev/null; cat; } < file` must see identical leftovers.
#[test]
fn row28_seekable_stdin_leftover() {
    setup();
    let inputs: &[&[u8]] = &[
        b"42abcdef",
        b"  \n  7  rest of line\nsecond line\n",
        b"abc def\n",
        b"99999999999999999999xyz\n",
        b"5",
        b" abc",
        b"-x rest",
        b"+ 5",
        b"  - 5",
        b"-",
        b"",
        b"0x10",
        b"  \t\n ",
        b"5 6",
        b"--5",
        b"+",
        b"99z",
        b"  +7q",
        b"0000",
        b".5",
        b"\0junk",
        b"1\n2\n3\n",
        b"2147483648!tail",
    ];
    let dir = std::env::temp_dir();
    for (i, input) in inputs.iter().enumerate() {
        let path = dir.join(format!("driver_leftover_{}_{i}", std::process::id()));
        std::fs::write(&path, input).unwrap();
        let mut leftovers = Vec::new();
        for exe in [c_exe(), rust_exe()] {
            let file = std::fs::File::open(&path).unwrap();
            let out = std::process::Command::new("sh")
                .arg("-c")
                .arg("\"$1\" >/dev/null; cat")
                .arg("sh")
                .arg(&exe)
                .stdin(file)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .expect("run through sh");
            assert!(out.status.success(), "sh failed for {}", exe.display());
            leftovers.push(out.stdout);
        }
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            String::from_utf8_lossy(&leftovers[0]),
            String::from_utf8_lossy(&leftovers[1]),
            "leftover stdin differs for input {:?}",
            String::from_utf8_lossy(input)
        );
    }

    // Randomized leftovers.
    let mut rng = Rng::new(0xABCD_0000_5555_9999);
    let alphabet: &[u8] = b"0123456789+- \t\nabcxyz.,\0";
    for i in 0..200 {
        let len = rng.below(20) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(rng.pick(alphabet));
        }
        let path = dir.join(format!("driver_leftover_rnd_{}_{i}", std::process::id()));
        std::fs::write(&path, &buf).unwrap();
        let mut leftovers = Vec::new();
        for exe in [c_exe(), rust_exe()] {
            let file = std::fs::File::open(&path).unwrap();
            let out = std::process::Command::new("sh")
                .arg("-c")
                .arg("\"$1\" >/dev/null; cat")
                .arg("sh")
                .arg(&exe)
                .stdin(file)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .expect("run through sh");
            leftovers.push(out.stdout);
        }
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            String::from_utf8_lossy(&leftovers[0]),
            String::from_utf8_lossy(&leftovers[1]),
            "leftover stdin differs for random input {:?}",
            String::from_utf8_lossy(&buf)
        );
    }
}

/// CONFIGS row 29: stdin that never reaches EOF. C's `scanf` stops as soon as
/// the conversion is complete, so the program must terminate; a translation that
/// drained stdin first would hang forever.
#[test]
fn row29_never_ending_stdin() {
    setup();
    // /dev/zero: an endless stream of NUL bytes -> immediate matching failure.
    let mut results = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let zero = std::fs::File::open("/dev/zero").unwrap();
        let child = std::process::Command::new(&exe)
            .stdin(zero)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        results.push(wait_with_deadline(child, 30, &exe));
    }
    assert_eq!(
        String::from_utf8_lossy(&results[0].0),
        String::from_utf8_lossy(&results[1].0),
        "/dev/zero stdin output differs"
    );
    assert_eq!(results[0].1, results[1].1, "/dev/zero stdin status differs");

    // An endless stream of valid input: `yes 5 | driver`.
    let mut results = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let child = std::process::Command::new("sh")
            .arg("-c")
            .arg("yes 5 2>/dev/null | \"$1\"")
            .arg("sh")
            .arg(&exe)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        results.push(wait_with_deadline(child, 30, &exe));
    }
    assert_eq!(
        String::from_utf8_lossy(&results[0].0),
        String::from_utf8_lossy(&results[1].0),
        "endless `yes 5` stdin output differs"
    );
}

/// Waits for `child`, killing it (and failing) if it has not exited within
/// `secs` seconds. Returns its stdout and exit code.
fn wait_with_deadline(
    mut child: std::process::Child,
    secs: u64,
    what: &std::path::Path,
) -> (Vec<u8>, Option<i32>) {
    use std::io::Read;
    let mut out = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    loop {
        match child.try_wait().unwrap() {
            Some(status) => {
                let buf = rx
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap_or_default();
                return (buf, status.code());
            }
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    panic!("{} did not terminate on a never-ending stdin", what.display());
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
}

/// CONFIGS row 19: a large stream with many buffer refills.
#[test]
fn row19_large_input() {
    setup();
    let mut s = " ".repeat(1024 * 1024);
    s.push_str("-99\n");
    diff_input("1MiB-whitespace", s.as_bytes());

    let mut s2 = "\n".repeat(200_000);
    s2.push_str("12345\n");
    diff_input("200k-newlines", s2.as_bytes());

    // Huge digit run (overflow path) crossing many buffers, both signs.
    for sign in ["", "+", "-"] {
        let mut s3 = String::from(sign);
        s3.push_str(&"9".repeat(100_000));
        s3.push('\n');
        diff_input("100k-digits", s3.as_bytes());

        // ... and a huge run of leading zeros followed by a small value.
        let mut s4 = String::from(sign);
        s4.push_str(&"0".repeat(100_000));
        s4.push_str("123\n");
        diff_input("100k-zeros", s4.as_bytes());
    }

    // A stream that never sends EOF-terminated garbage: 64 KiB of digits.
    let mut s5 = String::from("1");
    s5.push_str(&"0".repeat(65_536));
    diff_input("64k-digits-no-newline", s5.as_bytes());
}
