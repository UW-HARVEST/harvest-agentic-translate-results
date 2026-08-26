//! Shared infrastructure for the C-vs-Rust differential tests.
//!
//! Two comparison channels are provided:
//!
//! * **FFI** — both `cbuild/libcdriver.so` (the C translation unit built with
//!   `gcc -shared`) and `target/<prof>/libdriver.so` (the Rust `cdylib`) are
//!   `dlopen`ed with `libloading` and their exported `driver` / `main` symbols
//!   are called through the C ABI.  The Rust implementation is *never* called
//!   directly, so the `#[no_mangle]` wrappers are part of what is tested.
//! * **process** — the C executable (`c_src/build/driver`) and the Rust
//!   executable (`target/<prof>/driver`) are run with identical stdin and their
//!   stdout is compared byte-for-byte.  This is the only way to exercise
//!   `scanf("%f")`, which is `static`-free but only reachable from `main`.

#![allow(dead_code)]

use std::ffi::CString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Artifact discovery / building
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>` — the directory that holds the Rust artifacts for the
/// profile the tests were compiled with.
pub fn rust_artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test binary>
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    dir
}

fn newer(a: &Path, b: &Path) -> bool {
    let ma = fs::metadata(a).and_then(|m| m.modified()).ok();
    let mb = fs::metadata(b).and_then(|m| m.modified()).ok();
    match (ma, mb) {
        (Some(x), Some(y)) => x > y,
        _ => true,
    }
}

/// The C translation unit compiled as a shared library (built on demand).
pub fn c_shared_lib() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = crate_root();
        let src = root.join("c_src/src/main.c");
        let out_dir = root.join("cbuild");
        fs::create_dir_all(&out_dir).expect("create cbuild/");
        let out = out_dir.join("libcdriver.so");
        if !out.exists() || newer(&src, &out) {
            let st = Command::new("gcc")
                .args(["-shared", "-fPIC", "-fno-strict-aliasing", "-o"])
                .arg(&out)
                .arg(&src)
                .status()
                .expect("run gcc (needed to build the C shared library)");
            assert!(st.success(), "gcc failed to build {}", out.display());
        }
        out
    })
    .clone()
}

/// The C executable.  Prefers the CMake output documented in the task
/// (`c_src/build/driver`) and otherwise compiles it directly with gcc.
pub fn c_exe() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = crate_root();
        let src = root.join("c_src/src/main.c");
        let cmake_out = root.join("c_src/build/driver");
        if cmake_out.exists() && !newer(&src, &cmake_out) {
            return cmake_out;
        }
        let out_dir = root.join("cbuild");
        fs::create_dir_all(&out_dir).expect("create cbuild/");
        let out = out_dir.join("driver_c");
        if !out.exists() || newer(&src, &out) {
            let st = Command::new("gcc")
                .args(["-fno-strict-aliasing", "-o"])
                .arg(&out)
                .arg(&src)
                .status()
                .expect("run gcc (needed to build the C executable)");
            assert!(st.success(), "gcc failed to build {}", out.display());
        }
        out
    })
    .clone()
}

/// The Rust `cdylib`.  `cargo build` produces `target/<prof>/libdriver.so`;
/// plain `cargo test` does *not* emit the cdylib artifact, so if it is absent
/// the very same `src/lib.rs` is compiled to a cdylib with `rustc` directly.
/// Either way the `.so` under test is a real C-ABI shared object built from the
/// crate's own `#[no_mangle]` wrappers.
pub fn rust_shared_lib() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let cargo_built = rust_artifact_dir().join("libdriver.so");
        if cargo_built.exists() {
            return cargo_built;
        }
        let root = crate_root();
        let lib_rs = root.join("src/lib.rs");
        let scan_rs = root.join("src/scan.rs");
        let out_dir = root.join("cbuild");
        fs::create_dir_all(&out_dir).expect("create cbuild/");
        let release = rust_artifact_dir()
            .file_name()
            .map(|n| n == "release")
            .unwrap_or(false);
        let out = out_dir.join(if release {
            "libdriver_rustc_release.so"
        } else {
            "libdriver_rustc_debug.so"
        });
        if !out.exists() || newer(&lib_rs, &out) || newer(&scan_rs, &out) {
            let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
            let mut cmd = Command::new(rustc);
            cmd.args(["--edition=2021", "--crate-type=cdylib", "--crate-name=driver"]);
            if release {
                cmd.args(["-C", "opt-level=3"]);
            }
            cmd.arg("-o").arg(&out).arg(&lib_rs);
            let st = cmd
                .status()
                .expect("run rustc to build the Rust cdylib for the FFI tests");
            assert!(st.success(), "rustc failed to build {}", out.display());
        }
        out
    })
    .clone()
}

/// The Rust executable.
pub fn rust_exe() -> PathBuf {
    let p = rust_artifact_dir().join("driver");
    assert!(
        p.exists(),
        "{} is missing -- run `tools/build_all.sh` (or `cargo build`) first",
        p.display()
    );
    p
}

// ---------------------------------------------------------------------------
// Process channel
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone)]
pub struct ProgOut {
    pub stdout: Vec<u8>,
    pub code: Option<i32>,
}

impl std::fmt::Debug for ProgOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ stdout: {:?}, exit: {:?} }}",
            String::from_utf8_lossy(&self.stdout),
            self.code
        )
    }
}

pub fn run_prog(path: &Path, input: &[u8]) -> ProgOut {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", path.display()));
    let mut stdin = child.stdin.take().expect("stdin");
    // Small inputs fit in the pipe buffer, so they can be written inline; a
    // large input needs a writer thread because the child exits without
    // draining stdin (write then fails with EPIPE, which is expected).
    let handle = if input.len() <= 4096 {
        let _ = stdin.write_all(input);
        drop(stdin);
        None
    } else {
        let data = input.to_vec();
        Some(std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
            let _ = stdin.flush();
        }))
    };
    let out = child.wait_with_output().expect("wait_with_output");
    if let Some(h) = handle {
        let _ = h.join();
    }
    ProgOut {
        stdout: out.stdout,
        code: out.status.code(),
    }
}

pub fn c_run(input: &[u8]) -> ProgOut {
    run_prog(&c_exe(), input)
}

pub fn rust_run(input: &[u8]) -> ProgOut {
    run_prog(&rust_exe(), input)
}

/// Compares the two executables on one input.  Returns an error description on
/// divergence so callers can accumulate failures.
pub fn diff_one(input: &[u8]) -> Result<(), String> {
    let c = c_run(input);
    let r = rust_run(input);
    if c == r {
        Ok(())
    } else {
        Err(format!(
            "input {:?} (len {}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&input[..input.len().min(120)]),
            input.len(),
            c,
            r
        ))
    }
}

/// Runs `diff_one` over a whole corpus and panics with a report if anything
/// diverged.
pub fn diff_all<I, T>(label: &str, corpus: I)
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    let mut failures: Vec<String> = Vec::new();
    let mut n = 0usize;
    for case in corpus {
        n += 1;
        if let Err(e) = diff_one(case.as_ref()) {
            if failures.len() < 25 {
                failures.push(e);
            }
        }
    }
    assert!(n > 0, "{label}: empty corpus");
    if !failures.is_empty() {
        panic!(
            "{label}: {} of {} inputs diverged:\n{}",
            failures.len(),
            n,
            failures.join("\n")
        );
    }
    eprintln!("{label}: {n} inputs matched");
}

/// Like [`diff_all`], but additionally pins down *what* the shared result must
/// be, so an error-path test asserts a concrete sentinel rather than merely
/// "both behaved the same".
pub fn diff_and_expect<I, T>(label: &str, corpus: I, expected_stdout: &str)
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    let mut failures: Vec<String> = Vec::new();
    let mut n = 0usize;
    for case in corpus {
        let input = case.as_ref();
        n += 1;
        let c = c_run(input);
        let r = rust_run(input);
        let shown = String::from_utf8_lossy(&input[..input.len().min(120)]).to_string();
        if c != r {
            failures.push(format!("input {shown:?}: C={c:?} Rust={r:?} (diverged)"));
        } else if c.stdout != expected_stdout.as_bytes() {
            failures.push(format!(
                "input {shown:?}: both printed {:?}, expected {:?}",
                String::from_utf8_lossy(&c.stdout),
                expected_stdout
            ));
        } else if c.code != Some(0) {
            failures.push(format!("input {shown:?}: exit code {:?}, expected 0", c.code));
        }
        if failures.len() >= 25 {
            break;
        }
    }
    assert!(n > 0, "{label}: empty corpus");
    if !failures.is_empty() {
        panic!(
            "{label}: {} of {} inputs wrong:\n{}",
            failures.len(),
            n,
            failures.join("\n")
        );
    }
    eprintln!("{label}: {n} inputs matched, all -> {expected_stdout:?}");
}

/// Runs a program with fd 0 *closed* (not just empty) so that the read fails
/// with `EBADF` instead of returning EOF.
pub fn run_with_closed_stdin(path: &Path) -> ProgOut {
    use std::os::unix::process::CommandExt;
    let mut cmd = Command::new(path);
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            close(0);
            Ok(())
        });
    }
    let out = cmd.output().expect("spawn with closed stdin");
    ProgOut {
        stdout: out.stdout,
        code: out.status.code(),
    }
}

// ---------------------------------------------------------------------------
// FFI channel
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn open(path: *const i8, flags: i32, ...) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
    fn lseek(fd: i32, off: i64, whence: i32) -> i64;
}

const O_RDONLY: i32 = 0;
const O_RDWR: i32 = 2;
const O_CREAT: i32 = 0o100;
const O_TRUNC: i32 = 0o1000;

fn fd_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

fn tmp_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "driver_diff_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    p
}

/// Keeps fd 1 redirected to a scratch file for as long as it lives, so that
/// many `dlopen`ed calls can be captured back-to-back without re-doing the
/// redirection each time (`read_chunk` returns only what was written since the
/// previous call).  Both C `stdio` and Rust `io::stdout` end up on fd 1, so
/// this captures either implementation.
pub struct StdoutTap {
    _guard: std::sync::MutexGuard<'static, ()>,
    saved: i32,
    wfd: i32,
    reader: fs::File,
    path: PathBuf,
}

impl StdoutTap {
    pub fn new() -> Self {
        let guard = fd_lock().lock().unwrap();
        let path = tmp_path("tap");
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        unsafe {
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());
            let wfd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600i32);
            assert!(wfd >= 0, "open scratch stdout file");
            let saved = dup(1);
            assert!(saved >= 0, "dup(1)");
            assert!(dup2(wfd, 1) >= 0, "dup2(wfd, 1)");
            let reader = fs::File::open(&path).expect("reopen scratch stdout file");
            StdoutTap {
                _guard: guard,
                saved,
                wfd,
                reader,
                path,
            }
        }
    }

    /// Everything written to fd 1 since the previous `read_chunk`.
    pub fn read_chunk(&mut self) -> Vec<u8> {
        unsafe {
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());
        }
        let mut buf = Vec::new();
        self.reader
            .read_to_end(&mut buf)
            .expect("read scratch stdout file");
        buf
    }

    /// Runs `f` and returns its result together with what it printed.
    pub fn run<R, F: FnOnce() -> R>(&mut self, f: F) -> (R, Vec<u8>) {
        let r = f();
        let out = self.read_chunk();
        (r, out)
    }
}

impl Drop for StdoutTap {
    fn drop(&mut self) {
        unsafe {
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
            close(self.wfd);
        }
        let _ = fs::remove_file(&self.path);
    }
}

/// Runs `f` with fd 1 redirected to a scratch file and returns everything that
/// was written there.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let mut tap = StdoutTap::new();
    tap.run(f)
}

/// Runs `f` with fd 0 fed from `input` **and** fd 1 captured.
pub fn capture_with_stdin<R, F: FnOnce() -> R>(input: &[u8], f: F) -> (R, Vec<u8>) {
    let in_path = tmp_path("in");
    fs::write(&in_path, input).expect("write scratch stdin file");
    let cin = CString::new(in_path.to_str().unwrap()).unwrap();
    let res = capture_stdout(|| unsafe {
        let infd = open(cin.as_ptr(), O_RDONLY);
        assert!(infd >= 0, "open scratch stdin file");
        let saved0 = dup(0);
        assert!(dup2(infd, 0) >= 0, "dup2(infd, 0)");
        lseek(0, 0, 0);
        let r = f();
        if saved0 >= 0 {
            dup2(saved0, 0);
            close(saved0);
        }
        close(infd);
        r
    });
    let _ = fs::remove_file(&in_path);
    res
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) -- fixed seeds keep every test reproducible
// ---------------------------------------------------------------------------

/// Interior mutability keeps call sites readable: `rng.digits(rng.below(6), …)`
/// would not borrow-check with `&mut self` methods.
pub struct Rng(std::cell::Cell<u64>);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(std::cell::Cell::new(seed))
    }
    pub fn next_u64(&self) -> u64 {
        let s = self.0.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.0.set(s);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n`.
    pub fn below(&self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..hi`.
    pub fn range_i32(&self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo) as usize) as i32
    }
    pub fn pick<'a, T>(&self, s: &'a [T]) -> &'a T {
        &s[self.below(s.len())]
    }
    pub fn chance(&self, num: usize, den: usize) -> bool {
        self.below(den) < num
    }
    pub fn digits(&self, n: usize, alphabet: &[u8]) -> String {
        (0..n)
            .map(|_| *self.pick(alphabet) as char)
            .collect::<String>()
    }
}

pub const DEC: &[u8] = b"0123456789";
pub const HEX: &[u8] = b"0123456789abcdefABCDEF";

// ---------------------------------------------------------------------------
// Exact float <-> string helpers (used to build rounding-boundary inputs)
// ---------------------------------------------------------------------------

/// Decomposes a finite `f32` into `(negative, mantissa, exp2)` with
/// `value == (-1)^negative * mantissa * 2^exp2` exactly.
pub fn decompose(x: f32) -> (bool, u64, i32) {
    let bits = x.to_bits();
    let neg = bits >> 31 == 1;
    let ef = ((bits >> 23) & 0xff) as i32;
    let frac = (bits & 0x7f_ffff) as u64;
    if ef == 0 {
        (neg, frac, -149)
    } else {
        (neg, frac | 0x80_0000, ef - 127 - 23)
    }
}

/// Exact hexadecimal-float text for `mant * 2^exp2` (a `strtof`-compatible
/// `0x…p…` literal).
pub fn hex_literal(neg: bool, mant: u64, exp2: i32) -> String {
    format!("{}0x{:x}p{}", if neg { "-" } else { "" }, mant, exp2)
}

/// Exact decimal text for `mant * 2^exp2`.
pub fn exact_decimal(neg: bool, mant: u128, exp2: i32) -> String {
    // Digits are kept little-endian, one decimal digit per byte.
    let mut d: Vec<u8> = Vec::new();
    if mant == 0 {
        d.push(0);
    } else {
        let mut m = mant;
        while m > 0 {
            d.push((m % 10) as u8);
            m /= 10;
        }
    }
    let mul = |d: &mut Vec<u8>, k: u8| {
        let mut carry = 0u32;
        for x in d.iter_mut() {
            let v = *x as u32 * k as u32 + carry;
            *x = (v % 10) as u8;
            carry = v / 10;
        }
        while carry > 0 {
            d.push((carry % 10) as u8);
            carry /= 10;
        }
    };
    let point;
    if exp2 >= 0 {
        for _ in 0..exp2 {
            mul(&mut d, 2);
        }
        point = 0usize;
    } else {
        let k = (-exp2) as usize;
        for _ in 0..k {
            mul(&mut d, 5);
        }
        point = k;
    }
    while d.len() <= point {
        d.push(0);
    }
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    for (i, digit) in d.iter().enumerate().rev() {
        if i + 1 == point {
            s.push('.');
        }
        s.push((b'0' + digit) as char);
    }
    if point == 0 {
        s.push('.');
        s.push('0');
    }
    s
}

/// Exact decimal text for a finite `f32`.
pub fn exact_decimal_of(x: f32) -> String {
    let (neg, mant, e) = decompose(x);
    exact_decimal(neg, mant as u128, e)
}
