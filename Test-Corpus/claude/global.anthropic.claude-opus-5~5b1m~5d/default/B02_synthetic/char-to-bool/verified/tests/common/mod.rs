//! Shared harness for the differential tests.
//!
//! Both programs are driven *as subprocesses*, exactly the way a shell would
//! run them: bytes on stdin, bytes compared on stdout/stderr, plus the exit
//! status.  Nothing here links against the crate as a library.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{OnceLock, RwLock};

/// Serialises `fork` against raw file-descriptor manipulation.
///
/// The broken-pipe helpers create a pipe with `pipe(2)` and close its read end.
/// Those descriptors do not have `FD_CLOEXEC`, so a `fork` in another test
/// thread *between* those two calls would inherit the read end and keep it
/// alive - and then writing to the pipe would not raise `SIGPIPE` at all.  That
/// made the broken-pipe tests intermittently disagree with C.
///
/// Every spawn therefore takes a shared lock, and the broken-pipe helpers hold
/// the exclusive lock across `pipe` + `close` + `spawn`.
static SPAWN_LOCK: RwLock<()> = RwLock::new(());

/// `Command::spawn` under a shared lock, so no fork can race a raw `pipe(2)`.
fn spawn_guarded(cmd: &mut Command) -> std::io::Result<Child> {
    let _guard = SPAWN_LOCK.read().expect("spawn lock poisoned");
    cmd.spawn()
}

/// Result of running one of the two programs.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    pub code: Option<i32>,
    /// `Some(signal)` on unix when the process was killed by a signal.
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} code={:?} signal={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

/// Exit status of a finished child, including the signal that killed it.
fn status_of(status: std::process::ExitStatus) -> (Option<i32>, Option<i32>) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        (status.code(), status.signal())
    }
    #[cfg(not(unix))]
    {
        (status.code(), None)
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path of the compiled C program, building it with CMake if necessary.
///
/// Set `C_DRIVER` to point at a different build of the C program (used to run
/// the same suite against a coverage-instrumented or unoptimised build).
pub fn c_binary() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(p) = std::env::var_os("C_DRIVER") {
            let p = PathBuf::from(p);
            assert!(p.is_file(), "C_DRIVER={} is not a file", p.display());
            return p;
        }
        let root = workspace_root();
        let c_src = root.join("c_src");
        let candidates = [
            c_src.join("build/driver"),
            c_src.join("build/Debug/driver"),
            c_src.join("build/Release/driver"),
            c_src.join("build/driver.exe"),
            root.join("build/driver"),
        ];
        for c in &candidates {
            if c.is_file() {
                return c.clone();
            }
        }

        // Not built yet: build it the documented way.
        let build_dir = c_src.join("build");
        std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to spawn `cmake` - is CMake installed?");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );
        let bld = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to spawn `cmake --build`");
        assert!(
            bld.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        for c in &candidates {
            if c.is_file() {
                return c.clone();
            }
        }
        panic!(
            "C program built but no executable found; looked in {:?}",
            candidates
        );
    })
}

/// Path of the compiled Rust program (the binary target, not the library).
pub fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Feed `input` to `child`'s stdin, then collect everything it produced.
fn finish(mut child: Child, input: &[u8]) -> Run {
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The program may exit before consuming all of stdin (e.g. very long
        // inputs); a broken pipe is not a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");
    let (code, signal) = status_of(out.status);
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code,
        signal,
    }
}

/// Run `exe` with `input` on stdin and capture everything it produces.
pub fn run(exe: &Path, input: &[u8]) -> Run {
    let child = spawn_guarded(
        Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    )
    .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    finish(child, input)
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
pub fn check(name: &str, input: &[u8]) -> Run {
    let c = run(c_binary(), input);
    let r = run(rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case `{name}`\n  input : {}\n  C     : {:?}\n  Rust  : {:?}",
        pretty(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case `{name}`\n  input : {}\n  C     : {:?}\n  Rust  : {:?}",
        pretty(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit-status mismatch for case `{name}`\n  input : {}\n  C     : {:?}\n  Rust  : {:?}",
        pretty(input),
        c,
        r
    );

    c
}

/// Assert agreement *and* that the C program produced the documented output,
/// so the tests also pin down the expected values instead of only comparing.
pub fn check_expect(name: &str, input: &[u8], stdout: &str, stderr: &str, code: i32) {
    let c = check(name, input);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        stdout,
        "C stdout not as expected for `{name}` (input {})",
        pretty(input)
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        stderr,
        "C stderr not as expected for `{name}` (input {})",
        pretty(input)
    );
    assert_eq!(
        c.code,
        Some(code),
        "C exit code not as expected for `{name}` (input {})",
        pretty(input)
    );
}

/// Convenience: build the three-line stdin the program expects.
pub fn stdin3(operation: i32, param: i32, decisions: &str) -> Vec<u8> {
    format!("{operation}\n{param}\n{decisions}\n").into_bytes()
}

fn pretty(input: &[u8]) -> String {
    let shown: Vec<u8> = input.iter().copied().take(120).collect();
    let mut s = String::new();
    for b in shown {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0 => s.push_str("\\0"),
            0x20..=0x7e => s.push(b as char),
            other => s.push_str(&format!("\\x{other:02x}")),
        }
    }
    if input.len() > 120 {
        s.push_str(&format!("...<{} bytes total>", input.len()));
    }
    format!("\"{s}\"")
}

/// Which stream is connected to a pipe that has no reader.
#[cfg(unix)]
#[derive(Copy, Clone)]
pub enum BrokenStream {
    Stdout,
    Stderr,
}

/// Run `exe` with `input` on stdin and one output stream connected to a pipe
/// whose reader has already been closed, so the first write to it fails.
///
/// A C program has the default `SIGPIPE` disposition and is killed by signal
/// 13; the Rust program must behave the same way rather than panicking.
#[cfg(unix)]
pub fn run_with_broken_stream(exe: &Path, input: &[u8], broken: BrokenStream) -> Run {
    use std::os::unix::io::FromRawFd;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    // Exclusive: no other thread may fork while the read end exists, or the
    // forked child would inherit it and suppress SIGPIPE altogether.
    let guard = SPAWN_LOCK.write().expect("spawn lock poisoned");

    let mut fds = [-1i32; 2];
    let write_end = unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe(2) failed");
        // Drop the read end immediately: nothing will ever read this pipe.
        assert_eq!(close(fds[0]), 0, "close(2) failed");
        Stdio::from(std::fs::File::from_raw_fd(fds[1]))
    };

    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::piped());
    match broken {
        BrokenStream::Stdout => {
            cmd.stdout(write_end).stderr(Stdio::piped());
        }
        BrokenStream::Stderr => {
            cmd.stdout(Stdio::piped()).stderr(write_end);
        }
    }
    let child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    drop(guard);

    let mut out = finish(child, input);
    // The broken stream produced nothing we could read; blank it so the two
    // programs are compared only on the stream that still works.
    match broken {
        BrokenStream::Stdout => out.stdout.clear(),
        BrokenStream::Stderr => out.stderr.clear(),
    }
    out
}

/// Differential check with one output stream pointing at a reader-less pipe.
#[cfg(unix)]
pub fn check_broken_stream(name: &str, input: &[u8], broken: BrokenStream) {
    let which = match broken {
        BrokenStream::Stdout => "stdout",
        BrokenStream::Stderr => "stderr",
    };
    let c = run_with_broken_stream(c_binary(), input, broken);
    let r = run_with_broken_stream(rust_binary(), input, broken);
    assert_eq!(
        format!("{c:?}"),
        format!("{r:?}"),
        "mismatch for broken-{which} case `{name}`\n  input: {}\n  C   : {c:?}\n  Rust: {r:?}",
        pretty(input)
    );
}

/// Run both programs with command-line arguments (`main` is `int main(void)`,
/// so they must be ignored) and `input` on stdin.
pub fn check_with_args(name: &str, args: &[&str], input: &[u8]) {
    let one = |exe: &Path| -> Run {
        let child = spawn_guarded(
            Command::new(exe)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
        finish(child, input)
    };
    let c = one(c_binary());
    let r = one(rust_binary());
    assert_eq!(
        format!("{c:?}"),
        format!("{r:?}"),
        "mismatch for argv case `{name}` with args {args:?}"
    );
}

/// Run both programs with stdin connected to a real file rather than a pipe.
pub fn check_file_stdin(name: &str, input: &[u8]) {
    // CARGO_TARGET_TMPDIR lives inside the crate's own target/ directory, so it
    // is readable and writable no matter how TMPDIR is configured.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("file_stdin");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join(format!("{}.in", name.replace(['/', ' '], "_")));
    std::fs::write(&path, input).expect("write temp input");

    let one = |exe: &Path| -> Run {
        let file = std::fs::File::open(&path).expect("reopen temp input");
        let child = spawn_guarded(
            Command::new(exe)
                .stdin(Stdio::from(file))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
        let out = child.wait_with_output().expect("wait_with_output");
        let (code, signal) = status_of(out.status);
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code,
            signal,
        }
    };
    let c = one(c_binary());
    let r = one(rust_binary());
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        format!("{c:?}"),
        format!("{r:?}"),
        "mismatch for file-stdin case `{name}`"
    );
}
