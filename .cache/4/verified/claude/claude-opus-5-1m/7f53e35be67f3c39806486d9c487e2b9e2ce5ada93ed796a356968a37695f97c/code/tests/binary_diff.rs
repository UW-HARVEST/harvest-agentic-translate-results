//! Phase B/C, rows 25-27 of `CONFIGS.md` and row 22 of `ERRORS.md` —
//! process-level differential tests of the two `driver` programs.
//!
//! `c_src/CMakeLists.txt` builds an executable, so this file compares the real
//! artifacts end to end: the cmake-built C binary against this crate's `driver`
//! binary, over stdout bytes, stderr bytes and the exact termination status
//! (exit code *or* killing signal). It complements the `.so`-level tests, which
//! cannot observe process-wide effects such as `SIGPIPE`.

mod common;

use std::fs;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use common::Rng;

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn outcome(out: std::process::Output) -> Outcome {
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn progs() -> [PathBuf; 2] {
    [common::c_bin(), common::rust_bin()]
}

/// stdin from a pipe (written by this process), stdout/stderr captured.
fn run_piped(prog: &PathBuf, input: &[u8]) -> Outcome {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        // Deliver the bytes in small chunks so the callee sees short reads.
        for chunk in input.chunks(7) {
            if stdin.write_all(chunk).is_err() {
                break;
            }
            let _ = stdin.flush();
        }
    }
    outcome(child.wait_with_output().expect("wait"))
}

/// stdin from a regular file, stdout to a regular file.
fn run_files(prog: &PathBuf, input: &[u8], tag: &str) -> Outcome {
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let in_path = dir.join(format!("driver-bin-in-{}-{tag}", std::process::id()));
    let out_path = dir.join(format!("driver-bin-out-{}-{tag}", std::process::id()));
    fs::write(&in_path, input).expect("write stdin file");
    let in_file = fs::File::open(&in_path).expect("open stdin file");
    let out_file = fs::File::create(&out_path).expect("create stdout file");
    let out = Command::new(prog)
        .stdin(Stdio::from(in_file))
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::piped())
        .output()
        .expect("run");
    let stdout = fs::read(&out_path).expect("read stdout file");
    let _ = fs::remove_file(&in_path);
    let _ = fs::remove_file(&out_path);
    Outcome {
        stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// stdout is the write end of a pipe that has no reader at all.
fn run_broken_pipe(prog: &PathBuf, input: &[u8]) -> Outcome {
    let mut fds = [-1i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe");
    // Close the read end *before* spawning, so the very first write fails.
    unsafe { libc::close(fds[0]) };
    let stdout = unsafe { Stdio::from_raw_fd(fds[1]) };
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        let _ = stdin.write_all(input);
    }
    outcome(child.wait_with_output().expect("wait"))
}

/// Runs the program under `sh` so that a standard fd can be *closed* before the
/// exec (`>&-` / `<&-`); handing a closed fd to `Command` directly is rejected
/// by Rust's I/O-safety checks.
fn run_with_closed_fd(prog: &PathBuf, input: &[u8], redirect: &str) -> Outcome {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(format!("exec '{}' {redirect}", prog.display()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sh");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        let _ = stdin.write_all(input);
    }
    outcome(child.wait_with_output().expect("wait"))
}

/// stdout closed outright.
fn run_closed_stdout(prog: &PathBuf, input: &[u8]) -> Outcome {
    run_with_closed_fd(prog, input, ">&-")
}

fn assert_same(input: &[u8], c: Outcome, r: Outcome, what: &str) {
    assert_eq!(
        c,
        r,
        "{what} for stdin {:?}: C={:?} Rust={:?}",
        String::from_utf8_lossy(&input[..input.len().min(48)]),
        c,
        r
    );
}

/// The corpus mirrors every input shape of `CONFIGS.md` rows 13-24 and
/// `ERRORS.md` rows 1-15.
fn corpus() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    let fixed: [&[u8]; 40] = [
        b"",
        b" ",
        b"\n",
        b"\t\x0b\x0c\r ",
        b"0",
        b"-0",
        b"+0",
        b"7",
        b"-7",
        b"42\n",
        b"42\r\n",
        b"   12",
        b"abc",
        b"+",
        b"-",
        b"+-3",
        b"- 5",
        b".5",
        b"1e5",
        b"0x10",
        b"010",
        b"12abc",
        b"1 2",
        b"2147483647",
        b"2147483648",
        b"-2147483648",
        b"-2147483649",
        b"4294967296",
        b"4294967297",
        b"9223372036854775807",
        b"9223372036854775808",
        b"-9223372036854775808",
        b"-9223372036854775809",
        b"99999999999999999999999",
        b"1073741674",
        b"1073741823",
        b"1073741824",
        b"-1073741824",
        b"\xff12",
        b"\x0012",
    ];
    v.extend(fixed.iter().map(|s| s.to_vec()));
    // Long inputs crossing the read chunk.
    for n in [4095usize, 4096, 4097, 10_000] {
        v.push(vec![b'9'; n]);
        let mut ws = vec![b' '; n];
        ws.extend_from_slice(b"-321");
        v.push(ws);
    }
    // Randomized values, fixed seed.
    let mut rng = Rng::new(0xB1_0027);
    for _ in 0..120 {
        v.push(format!("{}", rng.next_i32()).into_bytes());
        v.push(format!("{}", rng.next_i64()).into_bytes());
        v.push(format!("  {}\n", rng.next_i32()).into_bytes());
        v.push(format!("+{}", rng.next_u64()).into_bytes());
    }
    v
}

/// Row 25 — stdin and stdout are regular files.
#[test]
fn row25_regular_files() {
    let [c, r] = progs();
    for (i, input) in corpus().iter().enumerate() {
        let co = run_files(&c, input, &format!("c{i}"));
        let ro = run_files(&r, input, &format!("r{i}"));
        assert_same(input, co, ro, "regular-file stdio");
    }
}

/// Row 26 — stdin is a pipe fed in small chunks, stdout is a pipe.
#[test]
fn row26_pipes_with_short_reads() {
    let [c, r] = progs();
    for input in corpus() {
        let co = run_piped(&c, &input);
        let ro = run_piped(&r, &input);
        assert_same(&input, co, ro, "piped stdio");
    }
}

/// Row 27 — the same corpus, asserting the C ground truth is `exit 0` with
/// exactly one line of output and nothing on stderr.
#[test]
fn row27_ground_truth_shape() {
    let [c, r] = progs();
    for input in corpus() {
        let co = run_piped(&c, &input);
        assert_eq!(co.code, Some(0), "C exit code");
        assert!(co.stderr.is_empty(), "C stderr must stay empty");
        assert_eq!(
            co.stdout.iter().filter(|&&b| b == b'\n').count(),
            1,
            "C must print exactly one line for {:?}",
            String::from_utf8_lossy(&input[..input.len().min(32)])
        );
        let ro = run_piped(&r, &input);
        assert_same(&input, co, ro, "ground-truth shape");
    }
}

/// `ERRORS.md` row 22 — a stdout pipe with no reader must kill both programs
/// with `SIGPIPE` (13), not make one of them exit successfully.
#[test]
fn errors_row22_broken_pipe_kills_both() {
    let [c, r] = progs();
    let co = run_broken_pipe(&c, b"5");
    let ro = run_broken_pipe(&r, b"5");
    assert_eq!(co.signal, Some(13), "C must die from SIGPIPE");
    assert_eq!(co.code, None);
    assert_same(b"5", co, ro, "broken-pipe termination");
}

/// `ERRORS.md` row 21 — fd 1 closed: `printf` fails, exit status stays 0.
#[test]
fn errors_row21_closed_stdout() {
    let [c, r] = progs();
    for input in [&b"5"[..], b"", b"abc", b"-2147483648"] {
        let co = run_closed_stdout(&c, input);
        let ro = run_closed_stdout(&r, input);
        assert_same(input, co, ro, "closed stdout");
    }
}

/// `ERRORS.md` row 6 — fd 0 closed: `scanf` fails, `x` stays 0.
#[test]
fn errors_row06_closed_stdin() {
    let [c, r] = progs();
    let co = run_with_closed_fd(&c, b"", "<&-");
    let ro = run_with_closed_fd(&r, b"", "<&-");
    assert_eq!(
        String::from_utf8_lossy(&co.stdout),
        "300\n",
        "C ground truth with fd 0 closed"
    );
    assert_eq!(co.code, Some(0));
    assert_same(b"", co, ro, "closed stdin");
}
