//! End-to-end differential tests for the `driver` executable
//! (`CONFIGS.md` rows C39-C41 and `ERRORS.md` rows E21-E26).
//!
//! The C binary is the one cmake builds from the untouched sources
//! (`c_src/build/driver`); the Rust binary is `target/*/driver`.  stdout,
//! stderr and the exit code are all compared byte-for-byte.

mod common;

use common::Rng;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Builds `c_src/build/driver` with cmake exactly as documented (idempotent).
fn c_binary() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = manifest_dir();
        let build = root.join("c_src/build");
        let exe = build.join("driver");

        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("mkdir c_src/build");
            let cfg = Command::new("cmake")
                .current_dir(&build)
                .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                .output()
                .expect("spawn cmake");
            assert!(
                cfg.status.success(),
                "cmake configure failed: {}",
                String::from_utf8_lossy(&cfg.stderr)
            );
            let b = Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .output()
                .expect("spawn cmake --build");
            assert!(
                b.status.success(),
                "cmake build failed: {}",
                String::from_utf8_lossy(&b.stderr)
            );
        }

        assert!(exe.is_file(), "missing {}", exe.display());
        exe
    })
}

/// `DRIVER_RUST_BIN` overrides the binary under test so the release build can be
/// exercised too (see `common::rust_library_path`).
fn rust_binary() -> &'static str {
    static P: OnceLock<String> = OnceLock::new();
    P.get_or_init(|| match std::env::var("DRIVER_RUST_BIN") {
        Ok(p) => {
            assert!(PathBuf::from(&p).is_file(), "DRIVER_RUST_BIN={p} is not a file");
            p
        }
        Err(_) => env!("CARGO_BIN_EXE_driver").to_string(),
    })
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

fn run(exe: &str, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {exe}: {e}"));

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin_bytes)
        .or_else(|e| {
            // The child may have exited before consuming all of stdin.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("write stdin");
    drop(child.stdin.take());

    let out = child.wait_with_output().expect("wait");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

#[track_caller]
fn assert_same_run(stdin_bytes: &[u8], ctx: &str) {
    let c = run(c_binary().to_str().unwrap(), stdin_bytes);
    let r = run(rust_binary(), stdin_bytes);

    let show = String::from_utf8_lossy(stdin_bytes).into_owned();
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch [{ctx}] for stdin {show:?} ({:02x?})\n  C   -> {:?}\n  Rust-> {:?}",
        stdin_bytes,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch [{ctx}] for stdin {show:?}\n  C   -> {:?}\n  Rust-> {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "exit code mismatch [{ctx}] for stdin {show:?}: C={:?} Rust={:?}",
        c.code, r.code
    );
}

fn stdin_for(op: &str, param: &str, decisions: &str) -> Vec<u8> {
    format!("{op}\n{param}\n{decisions}\n").into_bytes()
}

// ===========================================================================
// C39 — end-to-end equality across the whole option/shape space
// ===========================================================================

#[test]
fn c39_end_to_end_all_operations() {
    let decisions = [
        "", "y", "n", "Y", "N", "yy", "yn", "ny", "nn", "yyy", "yyn", "yny", "ynn", "nyy",
        "nyn", "nny", "nnn", "YyNn", "ynynynynyn", "yyyyn", "ynnnn", "yynnyynn", "abc",
        "y?n", "0101",
    ];

    for op in ["-2", "-1", "0", "1", "2", "3", "4", "5"] {
        for param in ["-1", "0", "1", "2", "3", "4"] {
            for d in decisions {
                assert_same_run(&stdin_for(op, param, d), "C39");
            }
        }
    }
}

#[test]
fn c39b_end_to_end_randomized() {
    let mut rng = Rng::new(0xC39_B);
    for _ in 0..4_000 {
        let len = rng.below(48);
        let d: String = (0..len)
            .map(|_| match rng.below(6) {
                0 => '?',
                1 => 'Y',
                2 => 'N',
                3 => 'y',
                4 => 'n',
                _ => (b'a' + rng.below(26) as u8) as char,
            })
            .collect();
        let op = rng.range(-3, 6).to_string();
        let param = rng.range(-3, 6).to_string();
        assert_same_run(&stdin_for(&op, &param, &d), "C39b");
    }
}

#[test]
fn c39c_end_to_end_long_inputs() {
    for op in ["0", "1", "2", "3"] {
        for param in ["0", "1", "2", "3"] {
            for len in [31usize, 32, 33, 1022, 1023] {
                let all_y = "y".repeat(len);
                assert_same_run(&stdin_for(op, param, &all_y), "C39c-y");
                let all_n = "n".repeat(len);
                assert_same_run(&stdin_for(op, param, &all_n), "C39c-n");
                let alt: String = (0..len)
                    .map(|i| if i % 2 == 0 { 'y' } else { 'n' })
                    .collect();
                assert_same_run(&stdin_for(op, param, &alt), "C39c-alt");
            }
        }
    }
}

// ===========================================================================
// C40 / E25 — atoi shapes for the operation and parameter lines
// ===========================================================================

const ATOI_SHAPES: &[&str] = &[
    "",
    " ",
    "\t",
    "   \t  ",
    "0",
    "00",
    "007",
    "1",
    "2",
    "3",
    "4",
    "+1",
    "+3",
    "-1",
    "-0",
    "-3",
    "  2  ",
    "\t3",
    "3abc",
    "abc",
    "abc3",
    "1x2",
    "2.9",
    "1e3",
    "++1",
    "--1",
    "+-1",
    "-",
    "+",
    "  +  3",
    "0x10",
    "0b11",
    " -0003 ",
    "3 4",
    "4294967297",
    "9223372036854775806",
    "-9223372036854775806",
    "2147483647",
    "2147483648",
    "-2147483648",
    "-2147483649",
    "4294967296",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "-9223372036854775809",
    "99999999999999999999",
    "-99999999999999999999",
    "0000000000000000000000003",
];

#[test]
fn c40_atoi_shapes_operation_line() {
    for shape in ATOI_SHAPES {
        assert_same_run(&stdin_for(shape, "0", "yyy"), "C40-op");
        assert_same_run(&stdin_for(shape, "2", "ynynyn"), "C40-op2");
    }
}

/// C40 (randomized): fuzz the two numeric lines with characters that exercise
/// every branch of glibc's `atoi` (= `(int)strtol(s, NULL, 10)`): whitespace,
/// signs, digits, digit-adjacent letters, radix prefixes, NUL and CR.
#[test]
fn c40b_atoi_randomized_fuzz() {
    const ALPHABET: &[u8] = b"0123456789+-  \t\r.eExXabcz\0";
    let mut rng = Rng::new(0xA701);

    for _ in 0..600 {
        let mut line = |max: usize| -> Vec<u8> {
            let n = rng.below(max + 1);
            (0..n).map(|_| ALPHABET[rng.below(ALPHABET.len())]).collect()
        };
        let op = line(12);
        let param = line(12);
        let decisions: &[u8] = match rng.below(7) {
            0 => b"yyy",
            1 => b"ynn",
            2 => b"y",
            3 => b"",
            4 => b"ynynyn",
            5 => b"nnn",
            _ => b"yyyyn",
        };

        let mut stdin = Vec::new();
        stdin.extend_from_slice(&op);
        stdin.push(b'\n');
        stdin.extend_from_slice(&param);
        stdin.push(b'\n');
        stdin.extend_from_slice(decisions);
        stdin.push(b'\n');
        assert_same_run(&stdin, "C40b");
    }

    // Digit strings around every representable boundary.
    let boundaries = [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551616",
        "-18446744073709551617",
        "1111111111111111111111111",
        "-999999999999999999999999",
        "000000000000000000000000005",
    ];
    for v in boundaries {
        assert_same_run(&stdin_for(v, "0", "yyy"), "C40b-boundary-op");
        assert_same_run(&stdin_for("1", v, "yyy"), "C40b-boundary-param");
    }
}

#[test]
fn err_e25_atoi_edge_cases() {
    for shape in ATOI_SHAPES {
        for op in ["0", "1", "2", "3", "7"] {
            assert_same_run(&stdin_for(op, shape, "yyy"), "E25-param");
        }
    }
    // Both lines garbage at once.
    for a in ATOI_SHAPES {
        for b in ["", "abc", "-1", "99999999999999999999"] {
            assert_same_run(&stdin_for(a, b, "ynn"), "E25-both");
        }
    }
}

// ===========================================================================
// C41 / E26 — decision-line shapes
// ===========================================================================

#[test]
fn c41_decision_line_shapes() {
    // No trailing newline at all (EOF-terminated third line).
    for op in ["0", "1", "2", "3"] {
        assert_same_run(format!("{op}\n0\nyyn").as_bytes(), "C41-no-newline");
        assert_same_run(format!("{op}\n0\nyyn\n").as_bytes(), "C41-newline");
        // Empty decision line.
        assert_same_run(format!("{op}\n0\n\n").as_bytes(), "C41-empty");
        // CRLF: the '\r' is an ordinary byte that parse_bool maps to false.
        assert_same_run(format!("{op}\n0\nyyn\r\n").as_bytes(), "C41-crlf");
        assert_same_run(format!("{op}\r\n0\r\nyyn\r\n").as_bytes(), "C41-crlf-all");
        // Trailing bytes after the third line are simply never read.
        assert_same_run(format!("{op}\n0\nyyn\nextra\nmore\n").as_bytes(), "C41-extra");
    }
}

#[test]
fn err_e26_embedded_nul_and_no_newline() {
    for op in ["0", "1", "2", "3"] {
        // strlen() stops at the embedded NUL, shortening `len`.
        let mut s = format!("{op}\n0\n").into_bytes();
        s.extend_from_slice(b"yy\x00nnnn\n");
        assert_same_run(&s, "E26-nul-mid");

        let mut s = format!("{op}\n0\n").into_bytes();
        s.extend_from_slice(b"\x00yyy\n");
        assert_same_run(&s, "E26-nul-first");

        let mut s = format!("{op}\n0\n").into_bytes();
        s.extend_from_slice(b"y\x00\n");
        assert_same_run(&s, "E26-nul-after-one");

        // NUL in the numeric lines too.
        let mut s = Vec::new();
        s.extend_from_slice(format!("{op}\x004\n").as_bytes());
        s.extend_from_slice(b"1\x002\n");
        s.extend_from_slice(b"yyn\n");
        assert_same_run(&s, "E26-nul-numeric");

        // High bytes / non-UTF8 in the decision line.
        let mut s = format!("{op}\n0\n").into_bytes();
        s.extend_from_slice(b"y\xff\x80\xfen\n");
        assert_same_run(&s, "E26-high-bytes");
    }
}

/// E24 / C41: a line longer than MAX_INPUT_SIZE - 1 (1023) makes `fgets`
/// truncate, and the remainder becomes the *next* line.
#[test]
fn err_e24_line_longer_than_1023() {
    // Long operation line: the overflow becomes the parameter line.
    let mut s = Vec::new();
    s.extend_from_slice(&vec![b'0'; 1030]);
    s.push(b'\n');
    s.extend_from_slice(b"2\n");
    s.extend_from_slice(b"yyn\n");
    assert_same_run(&s, "E24-long-op");

    // Long parameter line.
    let mut s = Vec::new();
    s.extend_from_slice(b"1\n");
    s.extend_from_slice(&vec![b'3'; 1200]);
    s.push(b'\n');
    s.extend_from_slice(b"yyn\n");
    assert_same_run(&s, "E24-long-param");

    // Long decision line: exactly 1023, 1024 and 2000 bytes.
    for total in [1022usize, 1023, 1024, 1025, 2000] {
        for op in ["0", "1", "2", "3"] {
            let mut s = format!("{op}\n0\n").into_bytes();
            s.extend_from_slice(&vec![b'y'; total]);
            s.push(b'\n');
            assert_same_run(&s, "E24-long-decision-y");

            let mut s = format!("{op}\n0\n").into_bytes();
            let alt: Vec<u8> = (0..total)
                .map(|i| if i % 2 == 0 { b'y' } else { b'n' })
                .collect();
            s.extend_from_slice(&alt);
            s.push(b'\n');
            assert_same_run(&s, "E24-long-decision-alt");
        }
    }
}

// ===========================================================================
// E21-E23 — fgets returning NULL
// ===========================================================================

#[test]
fn err_e21_missing_operation_line() {
    let c = run(c_binary().to_str().unwrap(), b"");
    assert_eq!(c.stderr, b"Error reading operation\n");
    assert_eq!(c.code, Some(1));
    assert!(c.stdout.is_empty());
    assert_same_run(b"", "E21");
}

#[test]
fn err_e22_missing_parameter_line() {
    let c = run(c_binary().to_str().unwrap(), b"0\n");
    assert_eq!(c.stderr, b"Error reading parameter\n");
    assert_eq!(c.code, Some(1));
    assert_same_run(b"0\n", "E22");
    // Also with no trailing newline on the only line.
    assert_same_run(b"0", "E22-no-newline");
    assert_same_run(b"garbage", "E22-garbage");
}

#[test]
fn err_e23_missing_decision_line() {
    let c = run(c_binary().to_str().unwrap(), b"0\n1\n");
    assert_eq!(c.stderr, b"Error reading decision string\n");
    assert_eq!(c.code, Some(1));
    assert_same_run(b"0\n1\n", "E23");
    assert_same_run(b"0\n1", "E23-no-newline");
    for op in ["-1", "0", "1", "2", "3", "4"] {
        assert_same_run(format!("{op}\n0\n").as_bytes(), "E23-sweep");
    }
}

// ===========================================================================
// C43-C45 — signal disposition / write-failure handling
//
// A C `main` inherits the DEFAULT SIGPIPE disposition, so writing to a pipe with
// no reader kills the process with signal 13.  The Rust runtime installs
// SIG_IGN before `main`, which would otherwise turn that into a silent EPIPE
// (exit 0) or, for `eprint!`, a panic (exit 101).
// ===========================================================================

/// Spawns `exe` with `stdout`/`stderr` bound to the write end of a pipe whose
/// read end has already been closed, and reports the raw wait status.
fn run_with_broken_pipe(exe: &str, stdin_bytes: &[u8], broken: Broken) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;

    // A pipe whose read end is dropped immediately.
    let (read_fd, write_fd) = {
        let mut fds = [0i32; 2];
        let rc = unsafe { libc_pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() failed");
        (fds[0], fds[1])
    };
    unsafe { libc_close(read_fd) };

    let stdio = || unsafe {
        use std::os::fd::FromRawFd;
        let dup = libc_dup(write_fd);
        assert!(dup >= 0, "dup() failed");
        Stdio::from(std::fs::File::from_raw_fd(dup))
    };

    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::piped());
    match broken {
        Broken::Stdout => {
            cmd.stdout(stdio()).stderr(Stdio::null());
        }
        Broken::Stderr => {
            cmd.stderr(stdio()).stdout(Stdio::null());
        }
    }

    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {exe}: {e}"));
    let _ = child.stdin.as_mut().unwrap().write_all(stdin_bytes);
    drop(child.stdin.take());
    let status = child.wait().expect("wait");
    unsafe { libc_close(write_fd) };

    (status.code(), status.signal())
}

#[derive(Copy, Clone)]
enum Broken {
    Stdout,
    Stderr,
}

extern "C" {
    #[link_name = "pipe"]
    fn libc_pipe(fds: *mut i32) -> i32;
    #[link_name = "close"]
    fn libc_close(fd: i32) -> i32;
    #[link_name = "dup"]
    fn libc_dup(fd: i32) -> i32;
}

#[test]
fn c43_broken_stdout_pipe_matches() {
    for stdin_bytes in [
        b"0\n0\nyyn\n".as_slice(),
        b"1\n2\nyny\n",
        b"2\n0\nyynnyy\n",
        b"3\n0\nynynyn\n",
        b"9\n0\nyyy\n",
    ] {
        let c = run_with_broken_pipe(c_binary().to_str().unwrap(), stdin_bytes, Broken::Stdout);
        let r = run_with_broken_pipe(rust_binary(), stdin_bytes, Broken::Stdout);
        assert_eq!(
            c, r,
            "broken-stdout status mismatch for {:?}: C=(code {:?}, signal {:?}) \
             Rust=(code {:?}, signal {:?})",
            String::from_utf8_lossy(stdin_bytes),
            c.0,
            c.1,
            r.0,
            r.1
        );
        // Sanity: the C really is dying from SIGPIPE, so this test has teeth.
        assert_eq!(c.1, Some(13), "expected the C to be killed by SIGPIPE");
    }
}

#[test]
fn c44_broken_stderr_pipe_matches() {
    // stdin at EOF makes `main` take the fprintf(stderr, ...) error path.
    for stdin_bytes in [b"".as_slice(), b"0\n", b"0\n1\n"] {
        let c = run_with_broken_pipe(c_binary().to_str().unwrap(), stdin_bytes, Broken::Stderr);
        let r = run_with_broken_pipe(rust_binary(), stdin_bytes, Broken::Stderr);
        assert_eq!(
            c, r,
            "broken-stderr status mismatch for {:?}: C=(code {:?}, signal {:?}) \
             Rust=(code {:?}, signal {:?})",
            String::from_utf8_lossy(stdin_bytes),
            c.0,
            c.1,
            r.0,
            r.1
        );
        assert_eq!(c.1, Some(13), "expected the C to be killed by SIGPIPE");
    }
}

/// A closed (rather than broken-pipe) stdout/stderr yields EBADF, not SIGPIPE;
/// C ignores the `printf`/`fprintf` failure, so the Rust must not panic either.
#[test]
fn c45_closed_stdout_and_stderr_match() {
    for stdin_bytes in [b"0\n0\nyyn\n".as_slice(), b"3\n0\nynyn\n", b"".as_slice()] {
        for (label, redirect) in [("stdout", "1>&-"), ("stderr", "2>&-"), ("both", "1>&- 2>&-")] {
            let script = |exe: &str| format!("exec {redirect}; exec {exe}");
            let mut c = Command::new("sh")
                .arg("-c")
                .arg(script(c_binary().to_str().unwrap()))
                .stdin(Stdio::piped())
                .spawn()
                .expect("spawn sh (C)");
            let _ = c.stdin.as_mut().unwrap().write_all(stdin_bytes);
            drop(c.stdin.take());
            let c_status = c.wait().unwrap();

            let mut r = Command::new("sh")
                .arg("-c")
                .arg(script(rust_binary()))
                .stdin(Stdio::piped())
                .spawn()
                .expect("spawn sh (Rust)");
            let _ = r.stdin.as_mut().unwrap().write_all(stdin_bytes);
            drop(r.stdin.take());
            let r_status = r.wait().unwrap();

            assert_eq!(
                c_status.code(),
                r_status.code(),
                "closed-{label} exit code mismatch for {:?}: C={:?} Rust={:?}",
                String::from_utf8_lossy(stdin_bytes),
                c_status.code(),
                r_status.code()
            );
        }
    }
}
