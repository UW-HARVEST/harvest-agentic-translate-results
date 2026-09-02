// Shared harness for the differential tests.
//
// Both programs are driven as SUBPROCESSES, exactly the way a shell would run
// them: bytes on stdin, bytes compared on stdout/stderr, plus exit status.
// Nothing here links the Rust crate as a library.
//
// Each integration test binary includes this module separately and uses a
// different subset of it, so unused-item warnings are not meaningful here.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Workspace root: the directory that holds both `c_src/` and `translation/`.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary produced by this crate.
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use if needed.
pub fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let conf = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                conf.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&conf.stdout),
                String::from_utf8_lossy(&conf.stderr)
            );
            let built = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                built.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&built.stdout),
                String::from_utf8_lossy(&built.stderr)
            );
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
    .as_path()
}

/// How the child's exit status is reported, comparably across both programs.
#[derive(Debug, PartialEq, Eq)]
pub struct Status {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

fn status_of(out: &Output) -> Status {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        Status {
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }
    #[cfg(not(unix))]
    {
        Status {
            code: out.status.code(),
            signal: None,
        }
    }
}

/// Run one program with `stdin_bytes` on stdin and `args` on the command line.
///
/// stdin is written from a helper thread so that inputs larger than a pipe
/// buffer cannot deadlock against the child's own writes.
fn run(bin: &Path, args: &[&str], stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    let mut sink = child.stdin.take().expect("piped stdin");
    let data = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        // Ignore EPIPE: the C program may stop reading (it only ever reads
        // 1000 bytes) and close stdin while we are still writing.
        let _ = sink.write_all(&data);
        let _ = sink.flush();
        drop(sink);
    });

    let out = child.wait_with_output().expect("collect child output");
    let _ = writer.join();
    out
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
pub fn assert_same_with_args(name: &str, args: &[&str], stdin_bytes: &[u8]) {
    let c = run(c_bin(), args, stdin_bytes);
    let r = run(&rust_bin(), args, stdin_bytes);

    let (cs, rs) = (status_of(&c), status_of(&r));

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs\n  input ({} bytes): {}\n  C    : {:?}\n  Rust : {:?}",
        stdin_bytes.len(),
        preview(stdin_bytes),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs\n  input ({} bytes): {}\n  C    : {:?}\n  Rust : {:?}",
        stdin_bytes.len(),
        preview(stdin_bytes),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        cs,
        rs,
        "[{name}] exit status differs\n  input ({} bytes): {}",
        stdin_bytes.len(),
        preview(stdin_bytes),
    );
}

/// Assert agreement for the no-arguments case (how the program is normally run).
pub fn assert_same(name: &str, stdin_bytes: &[u8]) {
    assert_same_with_args(name, &[], stdin_bytes);
}

/// Run one program while feeding stdin in small, separated chunks, so a single
/// `read(2)` cannot return everything. This exercises `fread`'s internal
/// looping (and the equivalent read loop in the Rust program).
fn run_chunked(bin: &Path, stdin_bytes: &[u8], chunk: usize) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    let mut sink = child.stdin.take().expect("piped stdin");
    let data = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        for part in data.chunks(chunk.max(1)) {
            if sink.write_all(part).is_err() {
                break;
            }
            let _ = sink.flush();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        drop(sink);
    });

    let out = child.wait_with_output().expect("collect child output");
    let _ = writer.join();
    out
}

/// Assert agreement when stdin arrives in dribbles rather than one block.
pub fn assert_same_chunked(name: &str, stdin_bytes: &[u8], chunk: usize) {
    let c = run_chunked(c_bin(), stdin_bytes, chunk);
    let r = run_chunked(&rust_bin(), stdin_bytes, chunk);
    let (cs, rs) = (status_of(&c), status_of(&r));

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs (chunked by {chunk})\n  input: {}\n  C    : {:?}\n  Rust : {:?}",
        preview(stdin_bytes),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs (chunked by {chunk})",
    );
    assert_eq!(cs, rs, "[{name}] exit status differs (chunked by {chunk})");
}

/// Number of C runs used to establish the modal output for an input that makes
/// the C program read out of bounds. See `assert_same_modal`.
const MODAL_RUNS: usize = 15;

/// Comparison for inputs of >= 1000 bytes, where the C program fills `in[1000]`
/// completely and `strchr` then runs off the end of the buffer (see ERRORS.md).
/// The bytes it reads there are 8 uninitialised stack padding bytes followed by
/// the saved `rbp`, i.e. a randomised stack address, so the C program's own
/// output is not a function of its input: repeated runs on identical input
/// disagree a small fraction of the time.
///
/// The Rust program must reproduce the C program's *stable* behaviour, so this
/// asserts:
///   * Rust is deterministic over the same number of runs, and
///   * Rust's output equals the C program's modal output,
///   * stderr and exit status match on every single run.
///
/// A plain byte-for-byte assert here would fail a couple of percent of the time
/// because of the C program's own nondeterminism, not because of a translation
/// defect.
pub fn assert_same_modal(name: &str, stdin_bytes: &[u8]) {
    assert_same_modal_runs(name, stdin_bytes, MODAL_RUNS)
}

/// True when the C program's `strchr` will run off the end of `in[1000]`:
/// the buffer gets completely filled and none of those 1000 bytes is a NUL, so
/// nothing inside the buffer terminates the string.
pub fn reads_out_of_bounds(stdin_bytes: &[u8]) -> bool {
    stdin_bytes.len() >= 1000 && !stdin_bytes[..1000].contains(&0)
}

/// Exact comparison for well-defined inputs, modal comparison for the inputs
/// that make the C program read out of bounds.
pub fn assert_same_auto(name: &str, stdin_bytes: &[u8], modal_runs: usize) {
    if reads_out_of_bounds(stdin_bytes) {
        assert_same_modal_runs(name, stdin_bytes, modal_runs);
    } else {
        assert_same(name, stdin_bytes);
    }
}

pub fn assert_same_modal_runs(name: &str, stdin_bytes: &[u8], runs: usize) {
    let runs = runs.max(1);
    let mut counts: std::collections::BTreeMap<Vec<u8>, usize> = Default::default();

    for _ in 0..runs {
        let c = run(c_bin(), &[], stdin_bytes);
        let r = run(&rust_bin(), &[], stdin_bytes);
        assert_eq!(
            c.stderr,
            r.stderr,
            "[{name}] stderr differs\n  input: {}",
            preview(stdin_bytes)
        );
        assert_eq!(
            status_of(&c),
            status_of(&r),
            "[{name}] exit status differs\n  input: {}",
            preview(stdin_bytes)
        );
        *counts.entry(c.stdout).or_insert(0) += 1;
    }

    let modal = counts
        .iter()
        .max_by_key(|(_, &n)| n)
        .map(|(out, _)| out.clone())
        .expect("at least one C run");

    // The Rust program must be deterministic on a fixed input.
    let mut rust_outputs = std::collections::BTreeSet::new();
    for _ in 0..runs {
        rust_outputs.insert(run(&rust_bin(), &[], stdin_bytes).stdout);
    }
    assert_eq!(
        rust_outputs.len(),
        1,
        "[{name}] Rust output is not deterministic: {:?}",
        rust_outputs
            .iter()
            .map(|o| String::from_utf8_lossy(o).to_string())
            .collect::<Vec<_>>()
    );

    let rust_out = rust_outputs.into_iter().next().unwrap();
    assert_eq!(
        modal,
        rust_out,
        "[{name}] Rust does not match the C program's modal stdout\n  \
         input ({} bytes): {}\n  C tally: {:?}\n  Rust   : {:?}",
        stdin_bytes.len(),
        preview(stdin_bytes),
        counts
            .iter()
            .map(|(o, n)| (String::from_utf8_lossy(o).to_string(), *n))
            .collect::<Vec<_>>(),
        String::from_utf8_lossy(&rust_out),
    );
}

/// Short, escaped rendering of an input for failure messages.
pub fn preview(bytes: &[u8]) -> String {
    let head: String = bytes
        .iter()
        .take(48)
        .flat_map(|&b| std::ascii::escape_default(b))
        .map(char::from)
        .collect();
    if bytes.len() > 48 {
        format!("\"{head}\"... (+{} bytes)", bytes.len() - 48)
    } else {
        format!("\"{head}\"")
    }
}

/// Run `bin` with its stdout connected to a pipe whose reader has already
/// exited, so the very first write to stdout fails.
///
/// The read end is closed before `bin` is even spawned (a throwaway `true`
/// process supplies the pipe and then exits), so there is no race.
#[cfg(unix)]
fn run_with_dead_stdout_reader(bin: &Path, stdin_bytes: &[u8]) -> Output {
    let mut donor = Command::new("true")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn `true` to donate a pipe");
    let write_end = donor.stdin.take().expect("piped stdin");
    donor.wait().expect("wait for `true`"); // reader is now gone

    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(write_end))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    let mut sink = child.stdin.take().expect("piped stdin");
    let data = stdin_bytes.to_vec();
    let w = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
    });
    let out = child.wait_with_output().expect("collect child output");
    let _ = w.join();
    out
}

/// Both programs must react identically to a stdout whose reader has vanished.
#[cfg(unix)]
pub fn assert_same_dead_stdout_reader(name: &str, stdin_bytes: &[u8]) {
    let c = run_with_dead_stdout_reader(c_bin(), stdin_bytes);
    let r = run_with_dead_stdout_reader(&rust_bin(), stdin_bytes);
    assert_eq!(
        c.stderr, r.stderr,
        "[{name}] stderr differs with a dead stdout reader"
    );
    assert_eq!(
        status_of(&c),
        status_of(&r),
        "[{name}] exit status differs with a dead stdout reader \
         (C's printf is killed by SIGPIPE; Rust must not silently ignore it)"
    );
}

/// Run `bin` through `sh` with file descriptor 1 closed outright, so `printf`
/// fails with `EBADF` rather than `EPIPE`.
#[cfg(unix)]
fn run_with_closed_stdout(bin: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("exec \"$0\" >&-")
        .arg(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sh");
    let mut sink = child.stdin.take().expect("piped stdin");
    let data = stdin_bytes.to_vec();
    let w = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
    });
    let out = child.wait_with_output().expect("collect child output");
    let _ = w.join();
    out
}

/// Both programs must react identically to a closed stdout.
#[cfg(unix)]
pub fn assert_same_closed_stdout(name: &str, stdin_bytes: &[u8]) {
    let c = run_with_closed_stdout(c_bin(), stdin_bytes);
    let r = run_with_closed_stdout(&rust_bin(), stdin_bytes);
    assert_eq!(
        c.stderr, r.stderr,
        "[{name}] stderr differs with stdout closed"
    );
    assert_eq!(
        status_of(&c),
        status_of(&r),
        "[{name}] exit status differs with stdout closed"
    );
}

/// Run `bin` with an unreadable stdin (a directory), so `fread` fails instead of
/// reaching EOF.
#[cfg(unix)]
pub fn assert_same_unreadable_stdin(name: &str, dir: &Path) {
    let go = |bin: &Path| {
        Command::new(bin)
            .stdin(Stdio::from(
                std::fs::File::open(dir).expect("open directory as stdin"),
            ))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run with directory stdin")
    };
    let c = go(c_bin());
    let r = go(&rust_bin());
    assert_eq!(
        c.stdout, r.stdout,
        "[{name}] stdout differs with an unreadable stdin"
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{name}] stderr differs with an unreadable stdin"
    );
    assert_eq!(
        status_of(&c),
        status_of(&r),
        "[{name}] exit status differs with an unreadable stdin"
    );
}

/// Deterministic pseudo-random byte generator (xorshift64*), so fuzz cases are
/// reproducible and need no external crates.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}
