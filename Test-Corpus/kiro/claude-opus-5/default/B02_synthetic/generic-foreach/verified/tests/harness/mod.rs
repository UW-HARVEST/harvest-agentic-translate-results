#![allow(dead_code)]
//! Shared machinery for the differential tests.
//!
//! Both programs are driven as subprocesses exactly the way a shell would drive
//! them: bytes in on stdin, bytes out on stdout/stderr, and an exit status.
//! Nothing here loads the Rust code as a library.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Normalised process outcome, so that a signal death is distinguishable from a
/// plain exit code and both are comparable between the two binaries.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Status {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl Status {
    fn from(status: std::process::ExitStatus) -> Status {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            Status {
                code: status.code(),
                signal: status.signal(),
            }
        }
        #[cfg(not(unix))]
        {
            Status {
                code: status.code(),
                signal: None,
            }
        }
    }
}

pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Status,
}

/// Workspace root: the directory holding both `c_src/` and `translation/`.
fn root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("translation/ must have a parent directory")
            .to_path_buf()
    })
}

/// Path to the C `driver`, building it with CMake on first use if needed.
pub fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake not available; build c_src manually first");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr)
            );
            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("cmake --build failed to start");
            assert!(
                compile.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
}

/// Path to the Rust `driver`. Cargo builds and points at it for us; an override
/// lets the same suite be aimed at the `--release` artefact.
pub fn rust_bin() -> &'static Path {
    static RUST_BIN: OnceLock<PathBuf> = OnceLock::new();
    RUST_BIN.get_or_init(|| match std::env::var_os("RUST_DRIVER_BIN") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_BIN_EXE_driver")),
    })
}

/// Run `bin` with `input` on stdin, collecting stdout, stderr and the status.
///
/// stdin is fed from a separate thread and stdout/stderr are drained
/// concurrently, so neither side can deadlock on a full pipe regardless of how
/// much the program writes.
pub fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let mut sink = child.stdin.take().expect("stdin pipe");
    let owned = input.to_vec();
    let feeder = std::thread::spawn(move || {
        // A broken pipe here mirrors the program exiting before reading
        // everything, which is normal for the `7` (Exit) branch.
        let _ = sink.write_all(&owned);
        let _ = sink.flush();
        drop(sink);
    });

    let mut out_pipe = child.stdout.take().expect("stdout pipe");
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out_pipe.read_to_end(&mut buf);
        buf
    });

    let mut err_pipe = child.stderr.take().expect("stderr pipe");
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err_pipe.read_to_end(&mut buf);
        buf
    });

    let status = child.wait().expect("wait failed");
    let stdout = out_reader.join().expect("stdout reader panicked");
    let stderr = err_reader.join().expect("stderr reader panicked");
    feeder.join().expect("stdin feeder panicked");

    Run {
        stdout,
        stderr,
        status: Status::from(status),
    }
}

/// Run `bin` with stdin replaced by `/dev/null` (immediate EOF) or, when
/// `close_stdin` is set, with file descriptor 0 closed outright so that C's
/// `fgets` fails with `EBADF` instead of seeing a clean end of file.
pub fn run_without_stdin(bin: &Path, close_stdin: bool) -> Run {
    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    if close_stdin {
        extern "C" {
            fn close(fd: i32) -> i32;
        }
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                close(0);
                Ok(())
            });
        }
    }

    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));

    Run {
        status: Status::from(output.status),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

/// Render bytes readably so a failure message is actually diagnosable.
fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// First differing offset between two byte streams, with surrounding context.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let at = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let lo = at.saturating_sub(60);
    format!(
        "first difference at byte {at} (C len {}, Rust len {})\n  C   : ...{}\n  Rust: ...{}",
        a.len(),
        b.len(),
        show(&a[lo..(at + 60).min(a.len())]),
        show(&b[lo..(at + 60).min(b.len())]),
    )
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
pub fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs for input {:?}\n{}",
        show(input),
        first_diff(&c.stdout, &r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs for input {:?}\n  C   : {}\n  Rust: {}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "[{label}] exit status differs for input {:?}",
        show(input)
    );
}

/// Run `bin`, read only `take` bytes of its stdout, then close the pipe.
///
/// Used to compare what each program does when its consumer goes away: the C
/// program is killed by `SIGPIPE`, and the Rust one must be too.
#[cfg(unix)]
pub fn run_with_early_close(bin: &Path, input: &[u8], take: usize) -> Status {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let mut sink = child.stdin.take().expect("stdin pipe");
    let owned = input.to_vec();
    let feeder = std::thread::spawn(move || {
        let _ = sink.write_all(&owned);
        let _ = sink.flush();
        drop(sink);
    });

    {
        let mut out = child.stdout.take().expect("stdout pipe");
        let mut buf = vec![0u8; take];
        let mut got = 0;
        while got < take {
            match out.read(&mut buf[got..]) {
                Ok(0) | Err(_) => break,
                Ok(n) => got += n,
            }
        }
        // Dropping `out` closes the read end; the next write in the child fails.
    }

    let status = child.wait().expect("wait failed");
    feeder.join().expect("stdin feeder panicked");
    Status::from(status)
}

/// Deterministic xorshift64* PRNG, so fuzz cases are reproducible without
/// pulling in a dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9e3779b97f4a7c15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}
