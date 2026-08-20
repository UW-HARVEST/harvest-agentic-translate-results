// Shared differential-test harness.
//
// Both implementations are always reached through their shared objects with
// `libloading` (or through their compiled programs / the `so_runner` example) --
// no Rust function is ever called directly, so the `#[no_mangle]` export
// wrappers in `src/lib.rs` are part of what is under test.
#![allow(dead_code)]

use std::io::Write;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type FmaFn = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
pub type DriverFn = unsafe extern "C" fn(*mut c_int, c_int);
pub type MainFn = unsafe extern "C" fn() -> c_int;

// ---------------------------------------------------------------- paths ----

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory holding the compiled test artefacts (`target/debug`).
pub fn target_dir() -> PathBuf {
    // .../target/debug/deps/<testbin>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/debug")
        .to_path_buf()
}

fn require(p: PathBuf, how: &str) -> PathBuf {
    assert!(
        p.exists(),
        "missing artefact {}\n\nbuild it first with:\n  {}\n(or just run ./run_all.sh)",
        p.display(),
        how
    );
    p
}

/// Lets the whole suite be pointed at a different pair of artefacts, e.g. the
/// release build of the Rust `.so` or an optimised build of the C:
/// `DIFF_RUST_SO=target/release/libdriver.so cargo test`.
fn env_override(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(|v| {
        let p = PathBuf::from(v);
        if p.is_absolute() {
            p
        } else {
            crate_root().join(p)
        }
    })
}

pub fn c_so() -> PathBuf {
    if let Some(p) = env_override("DIFF_C_SO") {
        return require(p, "DIFF_C_SO points at a missing file");
    }
    require(
        crate_root().join("c_src/build/libcdriver.so"),
        "cd c_src/build && gcc -fPIC -shared -o libcdriver.so ../src/main.c",
    )
}

pub fn rust_so() -> PathBuf {
    if let Some(p) = env_override("DIFF_RUST_SO") {
        return require(p, "DIFF_RUST_SO points at a missing file");
    }
    let name = format!(
        "{}driver{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    require(target_dir().join(name), "cargo build --offline")
}

pub fn c_exe() -> PathBuf {
    if let Some(p) = env_override("DIFF_C_EXE") {
        return require(p, "DIFF_C_EXE points at a missing file");
    }
    require(
        crate_root().join("c_src/build/driver"),
        "cd c_src/build && cmake .. && cmake --build .",
    )
}

pub fn rust_exe() -> PathBuf {
    if let Some(p) = env_override("DIFF_RUST_EXE") {
        return require(p, "DIFF_RUST_EXE points at a missing file");
    }
    require(target_dir().join("driver"), "cargo build --offline")
}

pub fn so_runner() -> PathBuf {
    require(
        target_dir().join("examples/so_runner"),
        "cargo build --offline --example so_runner",
    )
}

pub fn tmp_dir() -> PathBuf {
    let d = target_dir().join("difftmp");
    std::fs::create_dir_all(&d).expect("create tmp dir");
    d
}

// ------------------------------------------------------------ libraries ----

/// The two shared objects, opened once per test process.
pub struct Libs {
    pub c: libloading::Library,
    pub rust: libloading::Library,
}

impl Libs {
    pub fn c_fma(&self) -> libloading::Symbol<'_, FmaFn> {
        unsafe { self.c.get(b"fma_array\0") }.expect("C .so exports fma_array")
    }
    pub fn rust_fma(&self) -> libloading::Symbol<'_, FmaFn> {
        unsafe { self.rust.get(b"fma_array\0") }.expect("Rust .so exports fma_array")
    }
    pub fn c_driver(&self) -> libloading::Symbol<'_, DriverFn> {
        unsafe { self.c.get(b"driver\0") }.expect("C .so exports driver")
    }
    pub fn rust_driver(&self) -> libloading::Symbol<'_, DriverFn> {
        unsafe { self.rust.get(b"driver\0") }.expect("Rust .so exports driver")
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c = unsafe { libloading::Library::new(c_so()) }.expect("dlopen C .so");
        let rust = unsafe { libloading::Library::new(rust_so()) }.expect("dlopen Rust .so");
        Libs { c, rust }
    })
}

// --------------------------------------------------------------- stdout ----

fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Redirects fd 1 to a scratch file for the duration of `f` and returns every
/// byte written to it (including bytes that glibc's `stdout` buffered on behalf
/// of the C implementation, which is flushed before the fd is restored).
pub fn capture_stdout<R, F: FnOnce() -> R>(tag: &str, f: F) -> (R, Vec<u8>) {
    let _guard = capture_lock();

    let path = tmp_dir().join(format!("stdout-{}-{}.bin", std::process::id(), tag));
    let file = std::fs::File::create(&path).expect("create capture file");

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let out = unsafe {
        use std::os::unix::io::AsRawFd;
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let r = f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
        r
    };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (out, bytes)
}

// -------------------------------------------------------------- process ----

pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Ok(exit code)` or `Err(signal)`.
    pub status: Result<i32, i32>,
}

impl Run {
    pub fn describe(&self) -> String {
        format!(
            "status={:?} stdout={:?}",
            self.status,
            String::from_utf8_lossy(&self.stdout)
        )
    }
}

pub fn run_program(cmd: &mut Command, stdin_bytes: &[u8]) -> Run {
    use std::os::unix::process::ExitStatusExt;

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut si = child.stdin.take().expect("stdin");
        // A short-lived child may exit before consuming everything; a broken
        // pipe here is not a test failure.
        let _ = si.write_all(stdin_bytes);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait");
    let status = match out.status.code() {
        Some(c) => Ok(c),
        None => Err(out.status.signal().unwrap_or(-1)),
    };
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

/// Runs the compiled C program with `stdin_bytes` on stdin.
pub fn run_c_exe(stdin_bytes: &[u8]) -> Run {
    run_program(&mut Command::new(c_exe()), stdin_bytes)
}

/// Runs the compiled Rust program with `stdin_bytes` on stdin.
pub fn run_rust_exe(stdin_bytes: &[u8]) -> Run {
    run_program(&mut Command::new(rust_exe()), stdin_bytes)
}

/// Calls an export of one of the shared objects out-of-process through the
/// `so_runner` example (see `examples/so_runner.rs`).
pub fn run_so(so: &Path, args: &[&str], stdin_bytes: &[u8]) -> Run {
    let mut cmd = Command::new(so_runner());
    cmd.arg(so);
    for a in args {
        cmd.arg(a);
    }
    run_program(&mut cmd, stdin_bytes)
}

pub fn run_c_so_main(stdin_bytes: &[u8]) -> Run {
    run_so(&c_so(), &["main"], stdin_bytes)
}

pub fn run_rust_so_main(stdin_bytes: &[u8]) -> Run {
    run_so(&rust_so(), &["main"], stdin_bytes)
}

/// Asserts that the C program, the Rust program, the `main` export of the C
/// `.so` and the `main` export of the Rust `.so` all agree byte-for-byte.
pub fn assert_program_matches(label: &str, stdin_bytes: &[u8]) {
    let c = run_c_exe(stdin_bytes);
    let r = run_rust_exe(stdin_bytes);
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] program stdout differs for stdin {:?}\n  C: {}\n  R: {}",
        Escaped(stdin_bytes),
        c.describe(),
        r.describe()
    );
    assert_eq!(
        c.status,
        r.status,
        "[{label}] program exit status differs for stdin {:?}",
        Escaped(stdin_bytes)
    );

    let cso = run_c_so_main(stdin_bytes);
    let rso = run_rust_so_main(stdin_bytes);
    assert_eq!(
        c.stdout,
        cso.stdout,
        "[{label}] C .so `main` export disagrees with the C program for stdin {:?}",
        Escaped(stdin_bytes)
    );
    assert_eq!(
        cso.stdout,
        rso.stdout,
        "[{label}] `main` export stdout differs for stdin {:?}\n  C: {}\n  R: {}",
        Escaped(stdin_bytes),
        cso.describe(),
        rso.describe()
    );
    assert_eq!(
        cso.status,
        rso.status,
        "[{label}] `main` export exit status differs for stdin {:?}",
        Escaped(stdin_bytes)
    );
}

/// Pretty-printer for byte strings in assertion messages.
pub struct Escaped<'a>(pub &'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        for &b in self.0 {
            match b {
                b'\n' => f.write_str("\\n")?,
                b'\t' => f.write_str("\\t")?,
                b'\r' => f.write_str("\\r")?,
                0x0b => f.write_str("\\v")?,
                0x0c => f.write_str("\\f")?,
                0x20..=0x7e => f.write_str(&(b as char).to_string())?,
                _ => f.write_fmt(format_args!("\\x{b:02x}"))?,
            }
        }
        f.write_str("\"")
    }
}

// ------------------------------------------------------- fma differential ----

/// One `fma_array` call description: a single backing allocation plus the
/// element offsets of the four pointer arguments, which makes every aliasing
/// pattern (disjoint, identical, partially overlapping) expressible.
#[derive(Clone, Debug)]
pub struct FmaCase {
    pub buf: Vec<i32>,
    pub out: usize,
    pub mul1: usize,
    pub mul2: usize,
    pub add: usize,
    pub len: c_int,
}

impl FmaCase {
    /// Four disjoint windows of `len` elements inside one allocation.
    pub fn disjoint(values: Vec<i32>, len: c_int) -> Self {
        let n = len.max(0) as usize;
        assert!(values.len() >= 4 * n);
        FmaCase {
            buf: values,
            out: 0,
            mul1: n,
            mul2: 2 * n,
            add: 3 * n,
            len,
        }
    }
}

/// Calls `fma_array` in both shared objects with the exact same input and
/// asserts the whole backing buffer (not just the first `len` elements, so
/// stray writes are caught) comes out identical.
pub fn assert_fma_matches(label: &str, case: &FmaCase) {
    let l = libs();
    let c_fma = l.c_fma();
    let rust_fma = l.rust_fma();

    let mut cbuf = case.buf.clone();
    let mut rbuf = case.buf.clone();

    unsafe {
        let p = cbuf.as_mut_ptr();
        c_fma(
            p.add(case.out),
            p.add(case.mul1),
            p.add(case.mul2),
            p.add(case.add),
            case.len,
        );
        let p = rbuf.as_mut_ptr();
        rust_fma(
            p.add(case.out),
            p.add(case.mul1),
            p.add(case.mul2),
            p.add(case.add),
            case.len,
        );
    }

    if cbuf != rbuf {
        let first = cbuf
            .iter()
            .zip(rbuf.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "[{label}] fma_array buffers differ at index {first}: C={} Rust={}\n \
             len={} offsets out={} mul1={} mul2={} add={}\n input={:?}\n C   ={:?}\n Rust={:?}",
            cbuf[first], rbuf[first], case.len, case.out, case.mul1, case.mul2, case.add,
            case.buf, cbuf, rbuf
        );
    }
}

/// Runs `driver` (or `driver` twice, for `mode == "driver2"`) out-of-process in
/// the given shared object.  `stdout` is exactly what the library printed,
/// `stderr` is the resulting buffer.
pub fn run_driver_so(so: &Path, mode: &str, values: &[i32], len: c_int) -> Run {
    let mut stdin = String::new();
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            stdin.push(' ');
        }
        stdin.push_str(&v.to_string());
    }
    let len_s = len.to_string();
    run_so(so, &[mode, &len_s], stdin.as_bytes())
}

/// Out-of-process differential check for `driver`: both the printed bytes and
/// the in-place mutated buffer must match, as must the exit status.
///
/// Out-of-process because fd 1 is process-global and libtest writes its own
/// progress to it from other threads, which would race with an in-process
/// redirection.  The in-process variant lives in `tests/in_process_stdout.rs`,
/// a `harness = false` binary that has no such competition.
pub fn assert_driver_matches_out_of_process(label: &str, mode: &str, values: &[i32], len: c_int) {
    let c = run_driver_so(&c_so(), mode, values, len);
    let r = run_driver_so(&rust_so(), mode, values, len);

    assert_eq!(
        c.status, r.status,
        "[{label}] {mode} exit status differs (len={len}): C {:?} vs Rust {:?}",
        c.status, r.status
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "[{label}] {mode} stdout differs (len={len}, input={values:?})"
    );
    assert_eq!(c.stdout, r.stdout, "[{label}] {mode} stdout bytes differ");
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "[{label}] {mode} resulting buffer differs (len={len}, input={values:?})"
    );
}

/// Calls `driver` in both shared objects and asserts both the captured stdout
/// bytes and the in-place mutated buffer match.
///
/// Only safe from a `harness = false` test binary -- see
/// [`assert_driver_matches_out_of_process`].
pub fn assert_driver_matches(label: &str, values: &[i32], len: c_int) {
    let l = libs();

    let mut cbuf = values.to_vec();
    let (_, cout) = capture_stdout("c", || {
        let f = l.c_driver();
        unsafe { f(cbuf.as_mut_ptr(), len) };
    });

    let mut rbuf = values.to_vec();
    let (_, rout) = capture_stdout("rust", || {
        let f = l.rust_driver();
        unsafe { f(rbuf.as_mut_ptr(), len) };
    });

    assert_eq!(
        cbuf,
        rbuf,
        "[{label}] driver buffer differs (len={len}, input={:?})",
        values
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "[{label}] driver stdout differs (len={len}, input={:?})",
        values
    );
    assert_eq!(cout, rout, "[{label}] driver stdout bytes differ");
}

/// Same as [`assert_driver_matches`] but calls `driver` twice on the same
/// buffer, exercising the composed pipeline.
pub fn assert_driver_twice_matches(label: &str, values: &[i32], len: c_int) {
    let l = libs();

    let mut cbuf = values.to_vec();
    let (_, cout) = capture_stdout("c", || {
        let f = l.c_driver();
        unsafe {
            f(cbuf.as_mut_ptr(), len);
            f(cbuf.as_mut_ptr(), len);
        }
    });

    let mut rbuf = values.to_vec();
    let (_, rout) = capture_stdout("rust", || {
        let f = l.rust_driver();
        unsafe {
            f(rbuf.as_mut_ptr(), len);
            f(rbuf.as_mut_ptr(), len);
        }
    });

    assert_eq!(cbuf, rbuf, "[{label}] driver x2 buffer differs (len={len})");
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "[{label}] driver x2 stdout differs (len={len}, input={:?})",
        values
    );
}

// ------------------------------------------------------------------ rng ----

/// SplitMix64: deterministic, seeded, no external crate needed.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range_incl(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        lo + self.below(span) as i64
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Values that straddle every interesting boundary of `x*y+z` on `i32`.
pub const EXTREMES: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    46_340, // floor(sqrt(INT_MAX))
    46_341, // 46341^2 overflows
    -46_340,
    -46_341,
    65_535,
    65_536,
    -65_536,
    1 << 15,
    1 << 16,
    1 << 30,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    i32::MAX / 2,
    i32::MIN / 2,
];
