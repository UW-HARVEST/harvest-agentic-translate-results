// Shared differential-test harness.
//
// The artifact under test is an EXECUTABLE (`add_executable(driver src/main.c)`)
// that exports no dynamic symbols, so the FFI boundary an external caller sees
// is the process boundary: argv + stdin bytes in, stdout/stderr bytes and the
// wait status out.  Every helper below therefore drives the *compiled binaries*
// as child processes; no Rust function of the crate is ever called directly.

#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The Rust binary, as built by cargo for the current profile/feature set.
pub const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

fn up_to_date(bin: &Path, src: &Path) -> bool {
    match (mtime(bin), mtime(src)) {
        (Some(b), Some(s)) => b >= s,
        _ => false,
    }
}

/// Path of the C reference binary, building it with CMake on first use.
/// Never modifies anything inside `c_src/` other than the CMake build tree that
/// the task instructions ask for; if `c_src/build/driver` already exists and is
/// current it is reused as-is.
pub fn c_bin() -> PathBuf {
    static CACHE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CACHE.get_or_init(build_c_bin).clone()
}

fn build_c_bin() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    let src = md.join("c_src/src/main.c");

    let prebuilt = md.join("c_src/build/driver");
    if up_to_date(&prebuilt, &src) {
        return prebuilt;
    }

    let build_dir = md.join("target/c_build");
    let out = build_dir.join("driver");
    if up_to_date(&out, &src) {
        return out;
    }
    std::fs::create_dir_all(&build_dir).expect("create target/c_build");

    let cmake = Command::new("cmake")
        .arg("-S")
        .arg(md.join("c_src"))
        .arg("-B")
        .arg(&build_dir)
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output();
    let built = match cmake {
        Ok(o) if o.status.success() => Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false),
        _ => false,
    };
    if !built || !out.exists() {
        // Fall back to a plain compile equivalent to `add_executable`.
        let st = Command::new("cc")
            .arg(&src)
            .arg("-o")
            .arg(&out)
            .status()
            .expect("run cc to build the C reference binary");
        assert!(st.success(), "failed to compile the C reference binary");
    }
    assert!(out.exists(), "C reference binary missing at {}", out.display());
    out
}

/// Where the child's stdin comes from.
#[derive(Clone, Debug)]
pub enum In<'a> {
    /// Bytes fed through a pipe.
    Pipe(&'a [u8]),
    /// Bytes fed one at a time through a pipe (forces short `read()`s).
    PipeByteAtATime(&'a [u8]),
    /// Bytes placed in a regular (seekable) file.
    File(&'a [u8]),
    /// `< /dev/null`
    DevNull,
    /// `<&-` — file descriptor 0 is closed in the child.
    Closed,
    /// `< <path>` — any existing path (e.g. a directory, for EISDIR).
    Path(PathBuf),
}

/// Where the child's stdout goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    /// A pipe (glibc: fully buffered).
    Pipe,
    /// A regular file (glibc: fully buffered), read back afterwards.
    File,
    /// `> /dev/null`
    DevNull,
    /// `> /dev/full` — every write fails with ENOSPC.
    DevFull,
    /// A pipe whose read end is closed before the child writes (EPIPE/SIGPIPE).
    BrokenPipe,
}

/// The complete observable result of one process run.
#[derive(Clone, PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
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

fn tmp_dir() -> PathBuf {
    let d = std::env::temp_dir().join("driver-difftest");
    std::fs::create_dir_all(&d).ok();
    d
}

fn unique(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{}-{}-{}-{}", tag, std::process::id(), n, "tmp"))
}

/// Runs `bin` with the requested stdin source, stdout sink and argv.
pub fn run_cfg(bin: &Path, input: &In<'_>, out: Out, args: &[&str]) -> Run {
    let mut cmd = Command::new(bin);
    cmd.args(args);
    cmd.stderr(Stdio::piped());

    // ---- stdin ---------------------------------------------------------
    let mut stdin_file_guard: Option<PathBuf> = None;
    match input {
        In::Pipe(_) | In::PipeByteAtATime(_) => {
            cmd.stdin(Stdio::piped());
        }
        In::File(bytes) => {
            let p = unique("stdin");
            std::fs::write(&p, bytes).expect("write stdin temp file");
            cmd.stdin(Stdio::from(
                std::fs::File::open(&p).expect("open stdin temp file"),
            ));
            stdin_file_guard = Some(p);
        }
        In::DevNull => {
            cmd.stdin(Stdio::null());
        }
        In::Path(p) => {
            cmd.stdin(Stdio::from(
                std::fs::File::open(p).unwrap_or_else(|e| panic!("open {}: {e}", p.display())),
            ));
        }
        In::Closed => {
            cmd.stdin(Stdio::null());
            unsafe {
                use std::os::unix::process::CommandExt;
                cmd.pre_exec(|| {
                    extern "C" {
                        fn close(fd: i32) -> i32;
                    }
                    close(0);
                    Ok(())
                });
            }
        }
    }

    // ---- stdout --------------------------------------------------------
    let mut stdout_path: Option<PathBuf> = None;
    match out {
        Out::Pipe => {
            cmd.stdout(Stdio::piped());
        }
        Out::File => {
            let p = unique("stdout");
            cmd.stdout(Stdio::from(
                std::fs::File::create(&p).expect("create stdout temp file"),
            ));
            stdout_path = Some(p);
        }
        Out::DevNull => {
            cmd.stdout(Stdio::null());
        }
        Out::DevFull => {
            cmd.stdout(Stdio::from(
                std::fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .expect("open /dev/full"),
            ));
        }
        Out::BrokenPipe => {
            let (reader, writer) = std::io::pipe().expect("create pipe");
            // Close the read end *before* spawning: if it were closed after the
            // spawn the child could win the race and write into the pipe buffer
            // successfully, making the test flaky.  With no reader at all, the
            // very first write is guaranteed to raise EPIPE/SIGPIPE.
            drop(reader);
            cmd.stdout(Stdio::from(writer));
        }
    }

    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    // Feed stdin from a helper thread: for very large inputs the child may exit
    // before consuming everything, which yields EPIPE here — the same thing the
    // shell would see, and it must not fail the test.
    let writer = match input {
        In::Pipe(bytes) => {
            let mut sink = child.stdin.take().expect("child stdin");
            let data = bytes.to_vec();
            Some(std::thread::spawn(move || {
                let _ = sink.write_all(&data);
                let _ = sink.flush();
            }))
        }
        In::PipeByteAtATime(bytes) => {
            let mut sink = child.stdin.take().expect("child stdin");
            let data = bytes.to_vec();
            Some(std::thread::spawn(move || {
                for b in data {
                    if sink.write_all(&[b]).is_err() || sink.flush().is_err() {
                        break;
                    }
                }
            }))
        }
        _ => None,
    };

    let output = child.wait_with_output().expect("wait for child");
    if let Some(w) = writer {
        let _ = w.join();
    }
    drop(stdin_file_guard.map(std::fs::remove_file));

    let stdout = match (out, &stdout_path) {
        (Out::Pipe, _) => output.stdout,
        (Out::File, Some(p)) => {
            let d = std::fs::read(p).expect("read stdout temp file");
            std::fs::remove_file(p).ok();
            d
        }
        _ => Vec::new(),
    };

    use std::os::unix::process::ExitStatusExt;
    Run {
        stdout,
        stderr: output.stderr,
        code: output.status.code(),
        signal: output.status.signal(),
    }
}

/// Convenience: bytes on a stdin pipe, stdout on a pipe, no argv.
pub fn run(bin: &Path, stdin_bytes: &[u8]) -> Run {
    run_cfg(bin, &In::Pipe(stdin_bytes), Out::Pipe, &[])
}

fn render(bytes: &[u8]) -> String {
    let mut s = String::new();
    for (i, b) in bytes.iter().enumerate() {
        if i == 256 {
            s.push_str(&format!("...(+{} bytes)", bytes.len() - 256));
            break;
        }
        match *b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            0x20..=0x7e => s.push(*b as char),
            other => s.push_str(&format!("\\x{other:02x}")),
        }
    }
    s
}

/// The core differential assertion: identical stdout, stderr and wait status.
pub fn assert_same_cfg(input: &In<'_>, out: Out, args: &[&str], label: &str) {
    let c = run_cfg(&c_bin(), input, out, args);
    let r = run_cfg(Path::new(RUST_BIN), input, out, args);
    if c != r {
        let shape = match input {
            In::Pipe(b) | In::PipeByteAtATime(b) | In::File(b) => render(b),
            other => format!("{other:?}"),
        };
        panic!(
            "DIVERGENCE [{label}]\n  stdin = \"{shape}\"\n  stdout={out:?} argv={args:?}\n  \
             C   : {c:?}\n  Rust: {r:?}"
        );
    }
}

/// Same, for the common case (stdin pipe, stdout pipe, no argv).
pub fn assert_same(stdin_bytes: &[u8], label: &str) {
    assert_same_cfg(&In::Pipe(stdin_bytes), Out::Pipe, &[], label);
}

pub fn assert_same_str(stdin_text: &str, label: &str) {
    assert_same(stdin_text.as_bytes(), label);
}

/// Asserts C and Rust agree *and* that the C output is the expected literal,
/// pinning down the ground truth for the error-surface rows.
pub fn assert_same_and_expect(stdin_bytes: &[u8], expected_stdout: &str, label: &str) {
    assert_same(stdin_bytes, label);
    let c = run(&c_bin(), stdin_bytes);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected_stdout,
        "[{label}] C ground truth changed for stdin \"{}\"",
        render(stdin_bytes)
    );
    assert_eq!(c.code, Some(0), "[{label}] C exit status");
}

/// Deterministic xorshift64* PRNG — fixed seed keeps failures reproducible.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

/// Interesting `int` values: every boundary the narrowing/saturation paths care
/// about, plus the three magic constants of `multi_stage`.
pub const INTERESTING: &[i32] = &[
    0,
    1,
    2,
    3,
    -1,
    -2,
    -3,
    4,
    123,
    -123,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    255,
    256,
    65535,
    65536,
    -65536,
    1000000,
];

/// Whitespace bytes that C's `isspace()` accepts in the "C" locale.
pub const SPACES: &[u8] = &[b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

/// Builds a random whitespace run of length 1..=max using only `isspace` bytes.
pub fn ws(rng: &mut Rng, max: usize) -> String {
    let n = 1 + rng.below(max as u64) as usize;
    (0..n)
        .map(|_| *rng.choose(SPACES) as char)
        .collect::<String>()
}
