//! Differential tests: run the original C program and the Rust translation as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr, and the same exit status.
//!
//! The Rust code is never linked in as a library; only the built binary is
//! driven, because that is how the two programs are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary produced by this crate.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // tests/ lives in the crate; the C sources are the crate's sibling.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake the first time if needed.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr),
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr),
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));

    // Write on a helper thread so a program that never drains stdin (or exits
    // early) cannot deadlock the test.
    let mut sink = child.stdin.take().expect("piped stdin");
    let payload = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
        drop(sink);
    });

    let out = child.wait_with_output().expect("collect child output");
    let _ = writer.join();

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    };

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

fn show_status(st: &Result<i32, i32>) -> String {
    match st {
        Ok(c) => format!("exit {c}"),
        Err(sig) => format!("signal {sig}"),
    }
}

/// The core assertion: identical stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    let head: Vec<u8> = stdin_bytes.iter().copied().take(64).collect();
    let ctx = format!(
        "case {label}\n  stdin ({} bytes, first {}): \"{}\"",
        stdin_bytes.len(),
        head.len(),
        show(&head)
    );

    assert_eq!(
        c.stdout,
        r.stdout,
        "{ctx}\n  stdout differs\n    C   : \"{}\"\n    Rust: \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "{ctx}\n  stderr differs\n    C   : \"{}\"\n    Rust: \"{}\"",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "{ctx}\n  exit status differs\n    C   : {}\n    Rust: {}",
        show_status(&c.status),
        show_status(&r.status)
    );
}

#[track_caller]
fn assert_same_str(label: &str, stdin_text: &str) {
    assert_same(label, stdin_text.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist, run, and agree on the simplest input.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(&rust_bin(), b"1\n");
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust produced no stdout");
    assert_eq!(c.status, Ok(0), "C should exit 0 (main returns 0)");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status, r.status);
}

/// The struct dump is 16 bytes -> 32 hex digits plus a newline. Pinning the
/// shape here catches a wrong `sizeof(house_t)` in the translation even if both
/// programs somehow agreed on garbage.
#[test]
fn output_shape_is_32_hex_digits_and_newline() {
    for input in ["", "0", "7", "-7"] {
        let c = run(c_bin(), input.as_bytes());
        assert_eq!(
            c.stdout.len(),
            33,
            "C stdout for {input:?} was {:?}",
            show(&c.stdout)
        );
        assert_eq!(*c.stdout.last().unwrap(), b'\n');
        assert!(c.stdout[..32].iter().all(|b| b.is_ascii_hexdigit()));
        assert_same_str("shape", input);
    }
}

// ---------------------------------------------------------------------------
// Phase B: the inputs the C actually branches on.
//
// main() is `scanf("%d", &x); driver(x);` so the branch structure lives inside
// glibc's %d conversion:
//   * EOF before any non-space input  -> scanf returns EOF, x keeps its 0
//   * matching failure (no digits)    -> scanf returns 0,   x keeps its 0
//   * successful conversion           -> x = (int)(long)value
//   * out-of-range                    -> strtol saturates to LONG_MAX/LONG_MIN,
//                                        then narrows to int
// print_hex's loop is `for (i = 0; i < len; i++)` with len == sizeof(house_t),
// so it always runs 16 times; the "length" input class is the struct layout,
// covered by output_shape_is_32_hex_digits_and_newline.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_zero() {
    // EOF path: scanf returns EOF without touching x.
    assert_same("empty", b"");
}

#[test]
fn whitespace_only_input_is_eof() {
    // %d skips leading whitespace, then hits EOF.
    for (i, s) in [" ", "\n", "\n\n\n", "   \t\r\n ", "\u{b}\u{c}"]
        .iter()
        .enumerate()
    {
        assert_same_str(&format!("ws-only/{i}"), s);
    }
}

#[test]
fn single_value() {
    assert_same_str("single/1", "1");
    assert_same_str("single/1-nl", "1\n");
    assert_same_str("single/0", "0");
    assert_same_str("single/3", "3");
}

#[test]
fn negative_and_signed_values() {
    for s in ["-1", "-0", "+0", "+7", "-42\n", "+2147483647"] {
        assert_same_str(&format!("signed/{s}"), s);
    }
}

#[test]
fn int_boundaries() {
    for s in [
        "2147483646",
        "2147483647",  // INT_MAX
        "2147483648",  // INT_MAX + 1: fits in long, truncates to INT_MIN
        "-2147483647",
        "-2147483648", // INT_MIN
        "-2147483649", // INT_MIN - 1: truncates to INT_MAX
        "4294967295",  // 2^32 - 1 -> -1
        "4294967296",  // 2^32     -> 0
        "4294967297",  // 2^32 + 1 -> 1
        "-4294967296",
    ] {
        assert_same_str(&format!("bound/{s}"), s);
    }
}

#[test]
fn long_boundaries_and_overflow_saturation() {
    for s in [
        "9223372036854775806",
        "9223372036854775807",  // LONG_MAX
        "9223372036854775808",  // LONG_MAX + 1 -> saturates to LONG_MAX
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775809", // LONG_MIN - 1 -> saturates to LONG_MIN
        "18446744073709551615", // 2^64 - 1
        "18446744073709551616", // 2^64
        "99999999999999999999999999999999999999",
        "-99999999999999999999999999999999999999",
    ] {
        assert_same_str(&format!("sat/{s}"), s);
    }
}

#[test]
fn scanf_skips_whitespace_across_newlines() {
    // fgets would stop at the first newline; scanf does not. This is the
    // reading-behaviour difference the translation has to reproduce.
    for s in ["\n\n  \t\n 42", "   \r\n-5", "\n\n\n\n-2147483647", "\t\t9\n"] {
        assert_same(&format!("cross-nl/{}", show(s.as_bytes())), s.as_bytes());
    }
}

#[test]
fn matching_failure_leaves_x_zero() {
    // scanf returns 0 without assigning; x stays at its initialiser.
    for s in [
        "abc", "x", ".", "-", "+", "--5", "+-5", " -  5", "-abc", "+.", "/5", ":", "e5",
    ] {
        assert_same(&format!("nomatch/{}", show(s.as_bytes())), s.as_bytes());
    }
}

#[test]
fn conversion_stops_at_first_non_digit() {
    for s in [
        "7abc",
        "0x10",  // 0, then 'x' terminates the conversion
        "3.9",   // 3
        "1e5",   // 1
        "12 34", // only the first value is read
        "5-6",
        "-5+6",
        "007",
        "0000000000000000000000005",
        "-0000000000000000000000005",
    ] {
        assert_same(&format!("stop/{}", show(s.as_bytes())), s.as_bytes());
    }
}

#[test]
fn trailing_input_after_the_value_is_ignored() {
    // main reads one value and returns; anything left on stdin is unread.
    for s in [
        "5\n6\n7\n",
        "5 junk that is never parsed\n",
        "5\n\n\n\n\n",
    ] {
        assert_same_str("trailing", s);
    }
}

// ---------------------------------------------------------------------------
// Phase C: paths not covered above — binary/non-ASCII stdin, huge inputs,
// unusual stdout targets.
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_and_high_bytes() {
    assert_same("nul-then-digit", b"\x005");
    assert_same("digit-then-nul", b"5\x009");
    assert_same("high-bytes", b"\xff\xfe5");
    assert_same("utf8-then-digit", "\u{e9}5".as_bytes());
    assert_same("nul-only", b"\x00");
    // A NUL is not whitespace, so it is a matching failure even after spaces.
    assert_same("spaces-nul-digit", b"   \x00 5");
}

#[test]
fn c_whitespace_class_exactly() {
    // isspace(): space, \t, \n, \v, \f, \r — and nothing else.
    assert_same("ws-class", b" \t\n\x0b\x0c\r8");
    // \x1c is not whitespace: matching failure.
    assert_same("not-ws-1c", b"\x1c8");
}

#[test]
fn very_long_digit_runs() {
    let mut zeros = "0".repeat(100_000);
    zeros.push('7');
    assert_same("100k-leading-zeros", zeros.as_bytes());

    assert_same("100k-nines", "9".repeat(100_000).as_bytes());
    assert_same("100k-neg-nines", format!("-{}", "9".repeat(100_000)).as_bytes());

    let mut spaces = " ".repeat(100_000);
    spaces.push_str("-42");
    assert_same("100k-spaces", spaces.as_bytes());

    // Long run of newlines, then a value: still crosses them all.
    let mut nls = "\n".repeat(50_000);
    nls.push_str("2147483648");
    assert_same("50k-newlines", nls.as_bytes());
}

#[test]
fn large_unparsable_input() {
    // Big payload whose first non-space byte is not a digit: matching failure,
    // and neither program should read further or hang.
    let mut data = vec![b'z'; 200_000];
    data[0] = b'\n';
    assert_same("200k-junk", &data);
}

#[test]
fn deterministic_pseudorandom_inputs() {
    // Small xorshift so the corpus is fixed run to run.
    let mut state: u64 = 0x9e3779b97f4a7c15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let alphabet: &[u8] = b"0123456789+- \t\n\r.xeE\0\xff";
    for case in 0..200 {
        let len = (next() % 24) as usize;
        let data: Vec<u8> = (0..len)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        assert_same(&format!("fuzz/{case}"), &data);
    }
}

#[test]
fn deterministic_pseudorandom_integers() {
    let mut state: u64 = 0xdeadbeefcafe1234;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for case in 0..100 {
        let v = next() as i64;
        assert_same(&format!("int/{case}"), v.to_string().as_bytes());
        assert_same(
            &format!("uint/{case}"),
            (v as u64).to_string().as_bytes(),
        );
    }
}

#[test]
fn arguments_are_ignored() {
    // main() takes no parameters, so argv must not change behaviour.
    for args in [vec!["--help"], vec!["5"], vec!["-x", "9"]] {
        let mut c = Command::new(c_bin());
        let mut r = Command::new(rust_bin());
        let co = c
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("run C with args");
        let ro = r
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("run Rust with args");
        assert_eq!(co.stdout, ro.stdout, "stdout differs for args {args:?}");
        assert_eq!(co.stderr, ro.stderr, "stderr differs for args {args:?}");
        assert_eq!(
            co.status.code(),
            ro.status.code(),
            "exit status differs for args {args:?}"
        );
    }
}

#[test]
fn closed_stdin() {
    // /dev/null and an immediately-closed pipe are both plain EOF.
    for prog in [c_bin().to_path_buf(), rust_bin()] {
        let out = Command::new(&prog)
            .stdin(Stdio::null())
            .output()
            .expect("run with /dev/null stdin");
        assert!(out.status.success());
    }
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
#[cfg(unix)]
fn stdout_write_failure_behaves_the_same() {
    // Writing to /dev/full fails with ENOSPC. The C code ignores printf's
    // return value and still returns 0; the Rust code must not diverge.
    use std::fs::OpenOptions;

    let open_full = || OpenOptions::new().write(true).open("/dev/full");
    if open_full().is_err() {
        return; // /dev/full unavailable on this host
    }

    let mut results = Vec::new();
    for prog in [c_bin().to_path_buf(), rust_bin()] {
        let out = Command::new(&prog)
            .stdin(Stdio::null())
            .stdout(Stdio::from(open_full().unwrap()))
            .stderr(Stdio::piped())
            .output()
            .expect("run with stdout=/dev/full");
        results.push((out.stderr, out.status.code(), {
            use std::os::unix::process::ExitStatusExt;
            out.status.signal()
        }));
    }
    assert_eq!(results[0], results[1], "behaviour on stdout write failure differs");
}
