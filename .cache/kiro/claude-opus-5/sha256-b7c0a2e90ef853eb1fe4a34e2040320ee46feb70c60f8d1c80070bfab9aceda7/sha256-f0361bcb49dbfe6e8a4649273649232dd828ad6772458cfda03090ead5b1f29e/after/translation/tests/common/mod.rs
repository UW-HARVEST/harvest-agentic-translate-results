//! Shared differential-testing harness.
//!
//! Loads BOTH shared objects with `libloading` and calls every function through
//! its exported C symbol — never by calling the Rust crate directly. That way
//! the `#[no_mangle]` / `extern "C"` export wrappers are under test too.
//!
//! stdout is the library's only observable output, so every comparison captures
//! fd 1 around the call and compares the raw bytes.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust shared library not found under {}\nBuild it with:\n  cd translation && cargo build --release",
        base.display()
    );
}

/// The two loaded libraries. `RTLD_LOCAL` (libloading's default) keeps each
/// library's internal calls bound to its own definitions, so the C `driver`
/// cannot accidentally call the Rust `printLine` or vice versa.
pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("failed to dlopen the C libdriver.so");
        let rs = Library::new(rust_so_path()).expect("failed to dlopen the Rust libdriver.so");
        Libs { c, rs }
    })
}

// ---------------------------------------------------------------------------
// Typed symbol lookup (goes through dlsym on the specific library handle)
// ---------------------------------------------------------------------------

pub type FnVoidPtr = unsafe extern "C" fn(*const c_char);
pub type FnVoidInt = unsafe extern "C" fn(c_int);
pub type FnVoidIntInt = unsafe extern "C" fn(c_int, c_int);

fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe {
        lib.get(name.as_bytes())
            .unwrap_or_else(|e| panic!("symbol `{name}` missing from library: {e}"))
    }
}

macro_rules! accessors {
    ($($fname:ident : $ty:ty = $sym:literal;)*) => {
        $(
            pub mod $fname {
                use super::*;
                pub fn c() -> Symbol<'static, $ty> { sym(&libs().c, $sym) }
                pub fn rs() -> Symbol<'static, $ty> { sym(&libs().rs, $sym) }
            }
        )*
    };
}

accessors! {
    print_line:     FnVoidPtr    = "printLine";
    print_int_line: FnVoidInt    = "printIntLine";
    bad:            FnVoidInt    = "bad";
    good:           FnVoidInt    = "good";
    driver:         FnVoidIntInt = "driver";
}

/// Every symbol the C `.so` exports, as verified by `nm -D`.
pub const EXPORTED_SYMBOLS: &[&str] = &["printLine", "printIntLine", "bad", "good", "driver"];

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so captures must not overlap.
fn capture_lock() -> &'static Mutex<()> {
    static L: Mutex<()> = Mutex::new(());
    &L
}

/// Runs `f` with fd 1 redirected to a temp file and returns the bytes written.
///
/// Three things must be flushed/serialised for the capture to be clean:
///
/// 1. **libc's `stdout`** (`fflush(NULL)`) — the C and Rust `.so`s share one
///    glibc `FILE`, so unflushed bytes would leak across capture boundaries.
/// 2. **Rust's `std::io::Stdout`** — a *separate* `LineWriter`. libtest writes
///    `"test foo ... "` (no newline) before running each test, so that text sits
///    in Rust's buffer and would otherwise be flushed into our temp file.
/// 3. **fd 1 itself**, which is process-global — hence the mutex. Tests must
///    additionally run single-threaded, which `.cargo/config.toml` enforces via
///    `RUST_TEST_THREADS=1`; otherwise another thread's libtest progress output
///    lands inside our window.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Write;

    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_diff_{}_{}.out", std::process::id(), n));

    let file = File::create(&path).expect("create capture temp file");

    let saved = unsafe {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
        saved
    };

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }

    drop(file);
    let bytes = std::fs::read(&path).expect("read capture temp file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Guards against the capture being silently corrupted by parallel libtest
/// threads. Called by a dedicated test in each integration-test binary.
pub fn assert_single_threaded() {
    let v = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        v, "1",
        "these differential tests redirect the process-global fd 1 and MUST run \
         single-threaded. Expected RUST_TEST_THREADS=1 (set by \
         translation/.cargo/config.toml); got {v:?}. Run via \
         `cargo test` from the translation/ directory, or pass \
         `-- --test-threads=1`."
    );
}

// ---------------------------------------------------------------------------
// Differential assertion
// ---------------------------------------------------------------------------

fn render(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if s.len() <= 2000 => format!("{s:?}"),
        Ok(s) => format!("{:?}… ({} bytes)", &s[..2000], s.len()),
        Err(_) => format!("<{} non-utf8 bytes> {:?}", bytes.len(), &bytes[..bytes.len().min(200)]),
    }
}

/// Calls the C and the Rust variant, captures each one's stdout, and asserts the
/// byte streams are identical. Returns the (shared) output for extra assertions.
pub fn diff<C, R>(what: &str, call_c: C, call_rs: R) -> Vec<u8>
where
    C: FnOnce(),
    R: FnOnce(),
{
    let out_c = capture(call_c);
    let out_rs = capture(call_rs);

    if out_c != out_rs {
        let first = out_c
            .iter()
            .zip(out_rs.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(out_c.len().min(out_rs.len()));
        panic!(
            "DIVERGENCE in {what}\n  first differing byte offset: {first}\n  C   ({} bytes): {}\n  RUST({} bytes): {}",
            out_c.len(),
            render(&out_c),
            out_rs.len(),
            render(&out_rs),
        );
    }
    out_c
}

// ---------------------------------------------------------------------------
// Convenience differential wrappers, one per exported symbol
// ---------------------------------------------------------------------------

/// `printLine` with an arbitrary byte slice (a NUL terminator is appended).
pub fn diff_print_line_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut buf: Vec<u8> = bytes.to_vec();
    buf.push(0);
    let p = buf.as_ptr() as *const c_char;
    diff(
        &format!("printLine({:?})", String::from_utf8_lossy(bytes)),
        || unsafe { (print_line::c())(p) },
        || unsafe { (print_line::rs())(p) },
    )
}

/// `printLine(NULL)`.
pub fn diff_print_line_null() -> Vec<u8> {
    diff(
        "printLine(NULL)",
        || unsafe { (print_line::c())(std::ptr::null()) },
        || unsafe { (print_line::rs())(std::ptr::null()) },
    )
}

pub fn diff_print_int_line(v: c_int) -> Vec<u8> {
    diff(
        &format!("printIntLine({v})"),
        || unsafe { (print_int_line::c())(v) },
        || unsafe { (print_int_line::rs())(v) },
    )
}

pub fn diff_bad(v: c_int) -> Vec<u8> {
    diff(
        &format!("bad({v})"),
        || unsafe { (bad::c())(v) },
        || unsafe { (bad::rs())(v) },
    )
}

pub fn diff_good(v: c_int) -> Vec<u8> {
    diff(
        &format!("good({v})"),
        || unsafe { (good::c())(v) },
        || unsafe { (good::rs())(v) },
    )
}

pub fn diff_driver(g: c_int, b: c_int) -> Vec<u8> {
    diff(
        &format!("driver({g}, {b})"),
        || unsafe { (driver::c())(g, b) },
        || unsafe { (driver::rs())(g, b) },
    )
}

// ---------------------------------------------------------------------------
// Reproducible randomness (SplitMix64) — no external rand dependency
// ---------------------------------------------------------------------------

/// The fixed seed recorded in `CONFIGS.md`.
pub const SEED: u64 = 0xD123_4567;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        // Never emit 0x00: it would truncate the C string, which is a property
        // of C strings rather than of the translation.
        (0..len).map(|_| (self.below(255) + 1) as u8).collect()
    }
}

/// An `int` drawn from a mixture that concentrates on the boundaries the C
/// guards actually test (`0`, `9`, `10`) while still covering the full range.
pub fn mixed_index(rng: &mut Rng) -> i32 {
    match rng.below(10) {
        0..=3 => rng.range_i32(0, 9),         // in bounds
        4..=5 => rng.range_i32(-4, 14),       // straddles both boundaries
        6 => rng.range_i32(i32::MIN, -1),     // deeply negative
        7 => rng.range_i32(10, 1000),         // just past / well past the top
        8 => [i32::MIN, i32::MAX, -1, 0, 9, 10][rng.below(6) as usize],
        _ => rng.next_i32(),
    }
}

/// Like [`mixed_index`] but safe to feed to `bad()`, whose missing upper-bound
/// check makes any `data >= 10` an out-of-bounds *write*. Only `10` and `11`
/// stay inside `bad`'s own stack frame in the compiled C (see `BAD_OOB_INFRAME`
/// and the note in `CONFIGS.md`); larger values corrupt the *caller's* frame and
/// are therefore not differentially comparable.
pub fn mixed_index_bad_safe(rng: &mut Rng) -> i32 {
    let v = mixed_index(rng);
    if v >= 10 {
        BAD_OOB_INFRAME[rng.below(BAD_OOB_INFRAME.len() as u64) as usize]
    } else {
        v
    }
}

/// The only out-of-bounds `data` values for which the compiled C `bad()` keeps
/// its damage inside its own frame, so that its stdout is well-defined enough to
/// compare against.
///
/// From `objdump -d` on `c_src/build/libdriver.so`:
///
/// ```text
/// bad:  sub $0x40,%rsp
///       buffer -> -0x30(%rbp) .. -0x08(%rbp)   (10 ints)
///       i      -> -0x04(%rbp)
///       data   -> -0x34(%rbp)
///       movl $0x1,-0x30(%rbp,%rax,4)   <- buffer[data] = 1, unchecked
/// ```
///
/// * `buffer[10]` -> `-0x08(%rbp)`: frame padding, unused. Harmless.
/// * `buffer[11]` -> `-0x04(%rbp)`: the loop counter `i` — but the very next
///   instruction is `movl $0x0,-0x4(%rbp)` (`i = 0`), so the write is
///   immediately overwritten. Harmless.
/// * `buffer[12]`, `buffer[13]` -> the saved `%rbp` at `0x0(%rbp)`.
/// * `buffer[14]`, `buffer[15]` -> the return address at `0x8(%rbp)`.
/// * `buffer[16]` and beyond -> the *caller's* frame.
///
/// Empirically (each call in a forked child, so a crash is contained) the C
/// `.so` takes `SIGSEGV` for `data` in `12..=15` and again for `20..=23`, and
/// "survives" other values only by accident of whatever the caller happens to
/// keep at that offset. No translation can reproduce caller-frame corruption,
/// and the outcome is not a property of the translation, so those values are
/// excluded from differential comparison rather than silently passed over.
pub const BAD_OOB_INFRAME: &[i32] = &[10, 11];
