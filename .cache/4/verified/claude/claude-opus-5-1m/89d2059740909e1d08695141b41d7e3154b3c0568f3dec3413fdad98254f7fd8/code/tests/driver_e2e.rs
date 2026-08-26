//! Phase B rows B43–B47 and the `main`-loop rows of `ERRORS.md` (32–39).
//!
//! Two independent comparisons of the whole program:
//!   1. `main_symbol_scenarios` calls the **exported `main` symbol** of each
//!      shared library in a forked child whose fd 0/1 are wired to files, so the
//!      C ABI entry point is what runs;
//!   2. `stdin_scenarios` runs the two *executables* (`driver_c` built from the C
//!      sources by `build.rs`, and this crate's `driver` binary) on the same
//!      stdin and compares stdout, stderr and exit status.

mod common;

use std::io::{Read, Seek, Write};
use std::process::{Command, Stdio};

use common::*;
use libc::c_int;
use libloading::Library;

/// Every stdin shape the `main` loop distinguishes.
fn scenarios() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = vec![
        // B43 — each menu choice on its own (then EOF).
        ("choice 1".into(), b"1\n".to_vec()),
        ("choice 2".into(), b"2\n".to_vec()),
        ("choice 3".into(), b"3\n".to_vec()),
        ("choice 4".into(), b"4\n".to_vec()),
        ("choice 5".into(), b"5\n".to_vec()),
        ("choice 6".into(), b"6\n".to_vec()),
        ("choice 7".into(), b"7\n".to_vec()),
        // B44 — sequences and repeats.
        ("all demos then exit".into(), b"6\n7\n".to_vec()),
        ("1..5 then exit".into(), b"1\n2\n3\n4\n5\n7\n".to_vec()),
        ("1..5 then EOF".into(), b"1\n2\n3\n4\n5\n".to_vec()),
        ("repeated demo".into(), b"3\n3\n3\n7\n".to_vec()),
        ("exit stops reading".into(), b"7\n1\n2\n".to_vec()),
        ("all choices then EOF".into(), b"1\n2\n3\n4\n5\n6\n".to_vec()),
        // ERRORS 32 — fgets returns NULL.
        ("empty stdin".into(), Vec::new()),
        // ERRORS 33 — sscanf matching failure.
        ("newline only".into(), b"\n".to_vec()),
        ("spaces only".into(), b"   \n".to_vec()),
        ("letters".into(), b"abc\n".to_vec()),
        ("plus only".into(), b"+\n".to_vec()),
        ("minus only".into(), b"-\n".to_vec()),
        ("dot only".into(), b".\n".to_vec()),
        ("letter then digit".into(), b"x1\n".to_vec()),
        ("invalid then valid".into(), b"abc\n7\n".to_vec()),
        ("invalid then EOF".into(), b"abc".to_vec()),
        // ERRORS 34/35 — out-of-range menu values (one step past each end).
        ("zero".into(), b"0\n".to_vec()),
        ("eight".into(), b"8\n".to_vec()),
        ("negative one".into(), b"-1\n".to_vec()),
        ("int max".into(), b"2147483647\n".to_vec()),
        ("int min".into(), b"-2147483648\n".to_vec()),
        ("out of range then exit".into(), b"0\n8\n-1\n7\n".to_vec()),
        // ERRORS 36 — values that overflow `long` inside sscanf.
        ("int max plus one".into(), b"2147483648\n".to_vec()),
        ("2^32".into(), b"4294967296\n".to_vec()),
        ("long overflow".into(), b"99999999999999999999\n".to_vec()),
        (
            "negative long overflow".into(),
            b"-99999999999999999999\n".to_vec(),
        ),
        (
            "huge digit run".into(),
            {
                let mut s = vec![b'9'; 200];
                s.push(b'\n');
                s
            },
        ),
        // ERRORS 38 — accepted but unusual spellings.
        ("leading spaces".into(), b"   3\n".to_vec()),
        ("leading tab".into(), b"\t7\n".to_vec()),
        ("explicit plus".into(), b"+3\n".to_vec()),
        ("leading zeros".into(), b"007\n".to_vec()),
        ("trailing junk".into(), b"3junk\n".to_vec()),
        ("two numbers".into(), b"3 4\n".to_vec()),
        ("crlf".into(), b"3\r\n".to_vec()),
        ("no trailing newline".into(), b"3".to_vec()),
        ("exit without newline".into(), b"7".to_vec()),
        // ERRORS 39 — embedded NUL terminates the sscanf string.
        ("embedded NUL".into(), b"3\x009\n".to_vec()),
        ("NUL first".into(), b"\x003\n".to_vec()),
    ];

    // ERRORS 37 — lines longer than the 256-byte fgets buffer.
    let mut line300 = vec![b'1'; 300];
    line300.push(b'\n');
    v.push(("300-byte digit line".into(), line300));

    let mut line600 = b"7".to_vec();
    line600.extend(std::iter::repeat(b'x').take(600));
    line600.push(b'\n');
    v.push(("600-byte line starting with 7".into(), line600));

    let mut exact255 = vec![b'3'; 254];
    exact255.push(b'\n');
    v.push(("line exactly 255 bytes".into(), exact255));

    let mut exact256 = vec![b'3'; 255];
    exact256.push(b'\n');
    v.push(("line exactly 256 bytes".into(), exact256));

    // A longer mixed session.
    v.push((
        "mixed session".into(),
        b"9\nabc\n2\n0\n  4  \n+1\n-5\n7\n".to_vec(),
    ));

    v
}

// ============================================================================
// 1. The exported `main` symbol of each shared library
// ============================================================================

fn tmp_path(tag: &str) -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(dir).join(format!("driver_main_{}_{tag}_{n}", std::process::id()))
}

/// Calls `main` from `lib` in a forked child with fd 0 fed from `input` and fd 1
/// captured, returning `(stdout bytes, exit status word)`.
fn run_main_symbol(lib: &'static Library, input: &[u8]) -> (Vec<u8>, c_int) {
    use std::os::unix::io::AsRawFd;

    let main_fn: libloading::Symbol<'static, unsafe extern "C" fn() -> c_int> =
        unsafe { sym(lib, "main") };

    let in_path = tmp_path("in");
    std::fs::write(&in_path, input).expect("write stdin file");
    let in_file = std::fs::File::open(&in_path).expect("open stdin file");

    let out_path = tmp_path("out");
    let mut out_file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&out_path)
        .expect("create stdout file");

    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let status;
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            libc::dup2(in_file.as_raw_fd(), 0);
            libc::dup2(out_file.as_raw_fd(), 1);
            let rc = main_fn();
            libc::fflush(std::ptr::null_mut());
            libc::_exit(rc);
        }
        let mut st: c_int = 0;
        assert!(libc::waitpid(pid, &mut st, 0) == pid, "waitpid failed");
        status = st;
    }

    let mut buf = Vec::new();
    out_file.seek(std::io::SeekFrom::Start(0)).expect("seek");
    out_file.read_to_end(&mut buf).expect("read output");
    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);
    (buf, status)
}

/// B47 for the library entry point: the exported `main` of both `.so`s must
/// produce identical output and exit status for every stdin shape.
#[test]
fn main_symbol_scenarios() {
    for (name, input) in scenarios() {
        let (c_out, c_status) = run_main_symbol(c_lib(), &input);
        let (rs_out, rs_status) = run_main_symbol(rs_lib(), &input);
        assert_eq!(
            c_status, rs_status,
            "main() exit status differs for scenario {name:?} (C {c_status:#x} vs Rust {rs_status:#x})"
        );
        assert!(
            c_out == rs_out,
            "main() output differs for scenario {name:?}\n{}",
            first_diff(&c_out, &rs_out)
        );
        assert!(
            !c_out.is_empty(),
            "scenario {name:?} produced no output at all (harness bug?)"
        );
    }
}

// ============================================================================
// 2. The two executables
// ============================================================================

fn run_exe(path: &std::path::Path, input: &[u8]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", path.display()));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(input)
        .or_else(|e| {
            // The program may exit (choice 7) before consuming all input.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("write stdin");
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait");
    (out.stdout, out.stderr, out.status.code())
}

/// B43–B47 — the whole program, end to end, for every stdin shape.
#[test]
fn stdin_scenarios() {
    let c_exe = c_exe();
    let rs_exe = std::path::PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    assert!(c_exe.exists(), "missing C executable {}", c_exe.display());
    assert!(rs_exe.exists(), "missing Rust executable {}", rs_exe.display());

    for (name, input) in scenarios() {
        let (c_out, c_err, c_code) = run_exe(&c_exe, &input);
        let (rs_out, rs_err, rs_code) = run_exe(&rs_exe, &input);
        assert_eq!(c_code, rs_code, "exit code differs for scenario {name:?}");
        assert!(
            c_err.is_empty() && rs_err.is_empty(),
            "unexpected stderr for scenario {name:?}: C {:?}, Rust {:?}",
            String::from_utf8_lossy(&c_err),
            String::from_utf8_lossy(&rs_err)
        );
        assert!(
            c_out == rs_out,
            "stdout differs for scenario {name:?}\n{}",
            first_diff(&c_out, &rs_out)
        );
    }
}

/// Randomized stdin: random menu tokens, junk and whitespace, to catch shapes
/// the hand-written list missed.
#[test]
fn random_stdin_sessions() {
    let c_exe = c_exe();
    let rs_exe = std::path::PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    let mut rng = Rng::new(0xB4A_0047);

    for session in 0..60 {
        let mut input: Vec<u8> = Vec::new();
        let lines = 1 + rng.below(6);
        for _ in 0..lines {
            match rng.below(10) {
                0 => input.extend(format!("{}\n", rng.below(12)).as_bytes()),
                1 => input.extend(format!("{}\n", rng.i32()).as_bytes()),
                2 => input.extend(b"\n"),
                3 => input.extend(b"   \n"),
                4 => {
                    let junk = rng.ascii_string(8);
                    input.extend(&junk);
                    input.push(b'\n');
                }
                5 => input.extend(format!("  {}  junk\n", 1 + rng.below(7)).as_bytes()),
                6 => input.extend(format!("+{}\n", rng.below(9)).as_bytes()),
                7 => input.extend(format!("-{}\n", rng.below(9)).as_bytes()),
                8 => {
                    let n = 250 + rng.below(60);
                    let mut long: Vec<u8> = vec![b'1' + (rng.below(7) as u8)];
                    long.extend(std::iter::repeat(b'z').take(n));
                    long.push(b'\n');
                    input.extend(long);
                }
                _ => input.extend(format!("{}\n", 1 + rng.below(7)).as_bytes()),
            }
        }
        let (c_out, c_err, c_code) = run_exe(&c_exe, &input);
        let (rs_out, rs_err, rs_code) = run_exe(&rs_exe, &input);
        assert_eq!(
            c_code, rs_code,
            "exit code differs for random session #{session} input {:?}",
            String::from_utf8_lossy(&input)
        );
        assert_eq!(c_err.is_empty(), rs_err.is_empty());
        assert!(
            c_out == rs_out,
            "stdout differs for random session #{session} input {:?}\n{}",
            String::from_utf8_lossy(&input),
            first_diff(&c_out, &rs_out)
        );
    }
}
