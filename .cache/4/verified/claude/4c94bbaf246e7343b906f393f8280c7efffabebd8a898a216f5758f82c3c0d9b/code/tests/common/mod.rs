//! Shared differential-test harness.
//!
//! Both the C implementation (`c_src/build/libdriver.so`) and the Rust
//! implementation (`target/<profile>/libdriver.so`) are loaded with
//! `libloading` and driven **only** through their exported C symbols, exactly
//! as an external consumer would.  No Rust function of the crate under test is
//! ever called directly, so the `#[no_mangle] extern "C"` wrappers are part of
//! what gets verified.
//!
//! The library communicates exclusively through `stdout` (every public function
//! returns `void`), so "output" means the raw bytes written to file descriptor
//! 1.  `capture()` redirects fd 1 to a scratch file around the call, which
//! works for the C `.so`, the Rust `.so`, and the shared libc `FILE *stdout`
//! buffer alike.

#![allow(dead_code)]

use std::ffi::CString;
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::{c_char, c_float, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed to capture file descriptor 1
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// `fflush(NULL)` — flushes *every* open C stream, which is what makes the
/// capture reliable regardless of whether `stdout` happens to be line buffered
/// or fully buffered.
fn fflush_all() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// The C ABI surface under test (all 5 exported symbols)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Api {
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub print_int_line: unsafe extern "C" fn(c_int),
    pub bad: unsafe extern "C" fn(c_float),
    pub good: unsafe extern "C" fn(c_float),
    pub driver: unsafe extern "C" fn(c_float, c_float),
}

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub api: Api,
    // Keeps the `dlopen` handle alive for the whole process; the raw function
    // pointers above borrow from it.
    _lib: libloading::Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: libloading::Symbol<$t> = unsafe { lib.get($n) }.unwrap_or_else(|e| {
                    panic!(
                        "symbol {} missing from {} ({}): {e}",
                        String::from_utf8_lossy(&$n[..$n.len() - 1]),
                        name,
                        path.display()
                    )
                });
                *s
            }};
        }
        let api = Api {
            print_line: sym!(unsafe extern "C" fn(*const c_char), b"printLine\0"),
            print_int_line: sym!(unsafe extern "C" fn(c_int), b"printIntLine\0"),
            bad: sym!(unsafe extern "C" fn(c_float), b"bad\0"),
            good: sym!(unsafe extern "C" fn(c_float), b"good\0"),
            driver: sym!(unsafe extern "C" fn(c_float, c_float), b"driver\0"),
        };
        Impl {
            name,
            path,
            api,
            _lib: lib,
        }
    }
}

pub struct Libs {
    pub c: Impl,
    pub rs: Impl,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| Libs {
        c: Impl::load("C", c_so_path()),
        rs: Impl::load("Rust", rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built on demand with cmake.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let src = manifest_dir().join("c_src");
    let build = src.join("build");
    let so = build.join("libdriver.so");
    if !so.exists() {
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let cfg = std::process::Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status();
        let ok = matches!(cfg, Ok(s) if s.success())
            && matches!(
                std::process::Command::new("cmake")
                    .current_dir(&build)
                    .args(["--build", "."])
                    .status(),
                Ok(s) if s.success()
            );
        assert!(
            ok && so.exists(),
            "could not build the C reference library.  Build it manually with:\n\
             \x20 cd c_src && mkdir -p build && cd build && \\\n\
             \x20   cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             (or point $DRIVER_C_SO at an existing libdriver.so)"
        );
    }
    so
}

/// `target/<profile>/libdriver.so`.
///
/// `cargo test` does **not** build a `cdylib`-only library target, so the
/// harness triggers `cargo build` itself.  The crate declares no `[features]`,
/// so a plain build reproduces exactly the configuration the test binary was
/// compiled under (see `CONFIGS.md`, Axis 1).
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // .../target/<profile>/deps/<test-binary> -> .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("test binary layout")
        .to_path_buf();
    let so = profile_dir.join("libdriver.so");

    let release = profile_dir.file_name().map(|s| s == "release").unwrap_or(false);
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(manifest_dir())
        .arg("build")
        .arg("--offline")
        .env_remove("CARGO_MAKEFLAGS")
        .env_remove("MAKEFLAGS")
        .env_remove("RUSTC_WORKSPACE_WRAPPER");
    if release {
        cmd.arg("--release");
    }
    let _ = cmd.status();

    assert!(
        so.exists(),
        "the Rust cdylib {} does not exist; build it with `cargo build{}`",
        so.display(),
        if release { " --release" } else { "" }
    );
    so
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

struct Capture {
    file: File,
}

fn capture_state() -> MutexGuard<'static, Capture> {
    static STATE: OnceLock<Mutex<Capture>> = OnceLock::new();
    STATE
        .get_or_init(|| {
            let mut path = std::env::temp_dir();
            path.push(format!("driver-difftest-{}.out", std::process::id()));
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .unwrap_or_else(|e| panic!("cannot create scratch file {}: {e}", path.display()));
            // The file is unlinked immediately; the open handle keeps it alive.
            let _ = std::fs::remove_file(&path);
            Mutex::new(Capture { file })
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with file descriptor 1 pointing at a scratch file and returns every
/// byte the callee wrote to it.
///
/// Serialised through a process-wide mutex because fd redirection is global
/// state.  The test binaries use `harness = false` (see `Cargo.toml`) so that
/// nothing else — in particular no test-framework progress reporting from
/// another thread — can write to fd 1 while it is redirected.
///
/// If `f` panics the descriptor is restored before the unwind continues, so a
/// failing assertion cannot leave the process without a usable stdout.
pub fn capture<R>(f: impl FnOnce() -> R) -> (Vec<u8>, R) {
    let mut st = capture_state();

    // Make sure Rust's own buffered stdout is empty as well, otherwise a
    // pending `print!` without a trailing newline would land in the capture.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    fflush_all();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");

    st.file.seek(SeekFrom::Start(0)).expect("seek scratch");
    st.file.set_len(0).expect("truncate scratch");
    let fd = st.file.as_raw_fd();
    assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 onto stdout failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    fflush_all();
    assert!(unsafe { dup2(saved, 1) } >= 0, "restoring stdout failed");
    unsafe { close(saved) };

    let mut bytes = Vec::new();
    st.file.seek(SeekFrom::Start(0)).expect("rewind scratch");
    st.file.read_to_end(&mut bytes).expect("read scratch");
    match result {
        Ok(r) => (bytes, r),
        Err(p) => std::panic::resume_unwind(p),
    }
}

// ---------------------------------------------------------------------------
// Minimal single-threaded test runner (`harness = false`)
// ---------------------------------------------------------------------------

pub struct Case {
    pub name: &'static str,
    pub f: fn(),
}

#[macro_export]
macro_rules! cases {
    ($($f:ident),+ $(,)?) => {
        &[$( $crate::common::Case { name: stringify!($f), f: $f } ),+]
    };
}

/// Runs every case sequentially, in this thread, honouring an optional
/// substring filter passed on the command line (so `cargo test <name>` keeps
/// working).  Exits with status 1 if any case failed.
pub fn run(cases: &[Case]) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut filters: Vec<String> = Vec::new();
    let mut skip_next = false;
    for a in &args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a == "--test-threads" || a == "--format" || a == "--logfile" || a == "--skip" {
            skip_next = true;
            continue;
        }
        if a.starts_with('-') {
            continue;
        }
        filters.push(a.clone());
    }
    let selected: Vec<&Case> = cases
        .iter()
        .filter(|c| filters.is_empty() || filters.iter().any(|f| c.name.contains(f.as_str())))
        .collect();

    // Load (and, if necessary, build) both shared objects *before* the first
    // capture window, so that no build output can ever land in a capture.
    let l = libs();
    println!(
        "\nC   : {}\nRust: {}",
        l.c.path.display(),
        l.rs.path.display()
    );
    println!("running {} differential tests", selected.len());
    let mut failed: Vec<&str> = Vec::new();
    for c in &selected {
        print!("test {} ... ", c.name);
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let ok = std::panic::catch_unwind(c.f).is_ok();
        if ok {
            println!("ok");
        } else {
            println!("FAILED");
            failed.push(c.name);
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
    println!(
        "\ntest result: {}. {} passed; {} failed; {} filtered out\n",
        if failed.is_empty() { "ok" } else { "FAILED" },
        selected.len() - failed.len(),
        failed.len(),
        cases.len() - selected.len()
    );
    if !failed.is_empty() {
        println!("failures:");
        for f in &failed {
            println!("    {f}");
        }
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Differential comparison helpers
// ---------------------------------------------------------------------------

pub fn esc(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => {
                let _ = write!(s, "\\x{b:02x}");
            }
        }
    }
    s
}

/// Calls `f` once against the C `.so` and once against the Rust `.so` and
/// asserts the captured stdout bytes are identical.
pub fn compare_one(what: &str, f: impl Fn(&Api)) {
    let l = libs();
    let (c_out, ()) = capture(|| f(&l.c.api));
    let (r_out, ()) = capture(|| f(&l.rs.api));
    assert_eq!(
        c_out,
        r_out,
        "\nDIVERGENCE for {what}\n  C   : \"{}\"\n  Rust: \"{}\"\n",
        esc(&c_out),
        esc(&r_out)
    );
}

/// Batched variant: every input of `inputs` is applied inside a *single*
/// capture window (so stdio buffering / call interleaving is compared too).
/// On mismatch the inputs are replayed one at a time to report exactly which
/// one diverged.
pub fn compare_batch<T: std::fmt::Debug>(what: &str, inputs: &[T], f: impl Fn(&Api, &T)) {
    let l = libs();
    let (c_out, ()) = capture(|| {
        for i in inputs {
            f(&l.c.api, i);
        }
    });
    let (r_out, ()) = capture(|| {
        for i in inputs {
            f(&l.rs.api, i);
        }
    });
    if c_out == r_out {
        return;
    }
    // Narrow it down to the first offending input.
    for i in inputs {
        let (c1, ()) = capture(|| f(&l.c.api, i));
        let (r1, ()) = capture(|| f(&l.rs.api, i));
        assert_eq!(
            c1,
            r1,
            "\nDIVERGENCE in {what} for input {i:?}\n  C   : \"{}\"\n  Rust: \"{}\"\n",
            esc(&c1),
            esc(&r1)
        );
    }
    panic!(
        "\nDIVERGENCE in {what} across the batch but not for any single input \
         (stdio buffering / interleaving difference)\n  C   : \"{}\"\n  Rust: \"{}\"\n",
        esc(&c_out),
        esc(&r_out)
    );
}

/// Convenience: differential check of `printLine` over a set of byte strings.
/// Each element must not contain an interior NUL unless that is the point of
/// the test (use `compare_print_line_raw` for that).
pub fn compare_print_line(what: &str, strings: &[Vec<u8>]) {
    let owned: Vec<CString> = strings
        .iter()
        .map(|s| CString::new(s.clone()).expect("interior NUL - use compare_print_line_raw"))
        .collect();
    compare_batch(what, &owned, |api, s| unsafe {
        (api.print_line)(s.as_ptr());
    });
}

/// `printLine` over raw NUL-terminated buffers (allows interior NULs so the
/// truncation point can be observed).
pub fn compare_print_line_raw(what: &str, buffers: &[Vec<u8>]) {
    for b in buffers {
        assert_eq!(b.last(), Some(&0u8), "buffer must be NUL terminated");
    }
    compare_batch(what, buffers, |api, b| unsafe {
        (api.print_line)(b.as_ptr() as *const c_char);
    });
}

// ---------------------------------------------------------------------------
// Debug wrappers that show exact bit patterns (so a NaN-payload divergence is
// identifiable from the failure message)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub struct F(pub f32);

impl std::fmt::Debug for F {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(w, "{:?}[0x{:08x}]", self.0, self.0.to_bits())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct FF(pub f32, pub f32);

impl std::fmt::Debug for FF {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            w,
            "(good={:?}[0x{:08x}], bad={:?}[0x{:08x}])",
            self.0,
            self.0.to_bits(),
            self.1,
            self.1.to_bits()
        )
    }
}

pub fn wrap(v: &[f32]) -> Vec<F> {
    v.iter().copied().map(F).collect()
}

/// Differential check of `bad` over a set of floats.
pub fn compare_bad(what: &str, v: &[f32]) {
    compare_batch(what, &wrap(v), |api, x| unsafe { (api.bad)(x.0) });
}

/// Differential check of `good` over a set of floats.
pub fn compare_good(what: &str, v: &[f32]) {
    compare_batch(what, &wrap(v), |api, x| unsafe { (api.good)(x.0) });
}

/// Differential check of `driver` over a set of float pairs.
pub fn compare_driver(what: &str, v: &[FF]) {
    compare_batch(what, v, |api, x| unsafe { (api.driver)(x.0, x.1) });
}

/// Differential check of `printIntLine` over a set of ints.
pub fn compare_print_int_line(what: &str, v: &[c_int]) {
    compare_batch(what, v, |api, x| unsafe { (api.print_int_line)(*x) });
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_5678_9ABC;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// An `f32` built from a uniformly random 32-bit pattern: covers normals,
    /// subnormals, zeros, infinities and every NaN payload class.
    pub fn f32_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A finite `f32` whose magnitude is log-uniform in
    /// `[10^lo_exp, 10^hi_exp]`, with a random sign.
    pub fn f32_log_uniform(&mut self, lo_exp: f64, hi_exp: f64) -> f32 {
        let e = lo_exp + self.unit() * (hi_exp - lo_exp);
        let mag = 10f64.powf(e) as f32;
        let mag = if mag.is_finite() { mag } else { f32::MAX };
        if self.next_u32() & 1 == 0 {
            mag
        } else {
            -mag
        }
    }
}

// ---------------------------------------------------------------------------
// Value corpora derived from the branches in c_src/src/driver.c
// ---------------------------------------------------------------------------

/// Signalling NaN (payload with the quiet bit clear).
pub const SNAN: f32 = f32::from_bits(0x7f80_0001);
/// Negative signalling NaN.
pub const NEG_SNAN: f32 = f32::from_bits(0xff80_0001);
/// `100.0 / 2^31` — smallest magnitude whose quotient still fits an `int`
/// boundary region for `cvttsd2si`.
pub const CVT_BOUNDARY: f64 = 100.0 / 2147483648.0;

pub fn next_up(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() + 1)
    } else {
        f32::from_bits(x.to_bits() - 1)
    }
}

pub fn next_down(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return -f32::from_bits(1);
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() - 1)
    } else {
        f32::from_bits(x.to_bits() + 1)
    }
}

/// The float edge corpus: every value class the C source distinguishes
/// (zeros, subnormals, the `0.000001` guard boundary, the `cvttsd2si` range
/// boundary, infinities, NaNs, extremes) plus a few plain values.
pub fn edge_floats() -> Vec<f32> {
    let guard = 1e-6f32;
    let cvt = CVT_BOUNDARY as f32;
    let cvt_max = (100.0f64 / 2147483647.0) as f32;
    let mut v = vec![
        0.0f32,
        -0.0f32,
        f32::from_bits(1),  // smallest positive subnormal
        -f32::from_bits(1),
        f32::from_bits(0x007f_ffff), // largest subnormal
        -f32::from_bits(0x007f_ffff),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-8,
        -1e-8,
        guard,
        -guard,
        next_up(guard),
        next_down(guard),
        next_up(-guard),
        next_down(-guard),
        cvt,
        next_up(cvt),
        next_down(cvt),
        -cvt,
        next_up(-cvt),
        next_down(-cvt),
        cvt_max,
        next_up(cvt_max),
        next_down(cvt_max),
        -cvt_max,
        0.5,
        -0.5,
        1.0,
        -1.0,
        2.0,
        -2.0,
        3.0,
        -3.0,
        7.0,
        -7.0,
        100.0,
        -100.0,
        0.3,
        -0.3,
        1e6,
        -1e6,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        SNAN,
        NEG_SNAN,
    ];
    v.dedup_by(|a, b| a.to_bits() == b.to_bits());
    v
}

/// A 24-value subset of `edge_floats()` used for the `driver` cross product.
pub fn cross_floats() -> Vec<f32> {
    let guard = 1e-6f32;
    let cvt = CVT_BOUNDARY as f32;
    vec![
        0.0,
        -0.0,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-8,
        -1e-8,
        guard,
        next_up(guard),
        -next_up(guard),
        cvt,
        next_down(cvt),
        0.5,
        1.0,
        -1.0,
        2.0,
        -2.0,
        3.0,
        100.0,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        SNAN,
    ]
}

/// The `int` edge corpus for `printIntLine` (`"%d\n"`).
pub fn edge_ints() -> Vec<c_int> {
    let mut v = vec![
        0,
        1,
        -1,
        9,
        -9,
        10,
        -10,
        99,
        -99,
        100,
        -100,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ];
    for p in 0..10u32 {
        let ten = 10i32.pow(p);
        v.push(ten);
        v.push(-ten);
        v.push(ten - 1);
        v.push(-(ten - 1));
    }
    for s in 0..31u32 {
        let two = 1i32 << s;
        v.push(two);
        v.push(-two);
        v.push(two - 1);
    }
    v
}
