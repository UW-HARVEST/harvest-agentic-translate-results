// Shared differential-test harness.
//
// This module is `include!`d by the differential test files.  It loads BOTH
// shared objects -- the C one built by CMake and the Rust `cdylib` -- through
// `libloading` and calls every function through `dlsym`, exactly as an
// external C consumer would.  Rust functions are NEVER called directly, so
// the `#[unsafe(no_mangle)] extern "C"` export wrappers are themselves under
// test.
//
// The library under test communicates exclusively through `printf` to
// `stdout`, so the differential observation is "the bytes written to fd 1".
// `capture()` redirects fd 1 to a temp file around the call, flushes libc's
// stdio, and reads the bytes back.

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for stdout redirection and process isolation.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    /// glibc's `FILE *stdout` global — the very stream the library prints to.
    static mut stdout: *mut c_void;
}

const IONBF: c_int = 2; // glibc _IONBF

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of the C shared object (overridable with `C_DRIVER_SO`).
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let candidates = [
        manifest_dir().join("../c_src/build/libdriver.so"),
        manifest_dir().join("../c_src/build/Release/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "C shared object not found. Build it first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         (or set C_DRIVER_SO)"
    );
}

/// Path of the Rust `cdylib` (overridable with `RUST_DRIVER_SO`).
///
/// `cargo test` does not itself emit the `cdylib` artifact, so `run_all.sh`
/// runs `cargo build` beforehand.  We prefer the profile matching this test
/// binary, then fall back to the other one.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let (first, second) = if cfg!(debug_assertions) {
        ("debug", "release")
    } else {
        ("release", "debug")
    };
    for profile in [first, second] {
        let p = manifest_dir().join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found. Build it first:\n  \
         cd translation && cargo build --offline && cargo build --offline --release\n\
         (or set RUST_DRIVER_SO)"
    );
}

/// EVERY Rust `cdylib` that exists on disk (debug and release).
///
/// Optimisation level changes code generation in ways that are observable
/// through the ABI -- most notably whether the internal
/// `driver` -> `printHexCharLine` call survives as an interposable symbol
/// reference or gets inlined away.  Profile-sensitive checks must therefore run
/// against every profile that was built, not just the one matching this test
/// binary.
fn rust_so_paths() -> Vec<PathBuf> {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return vec![PathBuf::from(p)];
    }
    let v: Vec<PathBuf> = ["debug", "release"]
        .iter()
        .map(|p| manifest_dir().join("target").join(p).join("libdriver.so"))
        .filter(|p| p.exists())
        .collect();
    assert!(!v.is_empty(), "no Rust cdylib found; run `cargo build` first");
    v
}

fn c_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| {
        let p = c_so_path();
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
    })
}

fn rust_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| {
        let p = rust_so_path();
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
    })
}

/// `void f(char)` — the real prototype.
type FnChar = unsafe extern "C" fn(c_char);
/// `void f(int)` — deliberately WRONG width, to probe what the callee does
/// with the upper 24 bits of the argument register (see ERRORS.md row 6).
type FnInt = unsafe extern "C" fn(c_int);
/// `void f(long)` — same idea, 64-bit garbage in the argument register.
type FnLong = unsafe extern "C" fn(i64);

fn sym_char(lib: &'static Library, name: &[u8]) -> FnChar {
    unsafe { *lib.get::<FnChar>(name).expect("dlsym") }
}
fn sym_int(lib: &'static Library, name: &[u8]) -> FnInt {
    unsafe { *lib.get::<FnInt>(name).expect("dlsym") }
}
fn sym_long(lib: &'static Library, name: &[u8]) -> FnLong {
    unsafe { *lib.get::<FnLong>(name).expect("dlsym") }
}

const DRIVER: &[u8] = b"driver\0";
const PRINT_HEX: &[u8] = b"printHexCharLine\0";

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global state, so captures must be serialized.
fn cap_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

fn temp_path() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("driver_diff_{}_{}.out", std::process::id(), n))
}

/// Run `f` with libc `stdout` redirected to a temp file; return the bytes it
/// wrote.
fn capture(f: impl FnOnce()) -> Vec<u8> {
    let guard = cap_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Push anything the Rust test harness has buffered out to the *real* fd 1
    // before we steal it, and likewise anything libc has buffered.
    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();
    unsafe { fflush(std::ptr::null_mut()) };

    let path = temp_path();
    let file = fs::File::create(&path).expect("create temp capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    f();

    // Flush the library's output into the temp file, then put fd 1 back.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore fd 1 failed");
    unsafe { close(saved) };
    drop(file);

    let out = fs::read(&path).expect("read temp capture file");
    fs::remove_file(&path).ok();
    drop(guard);
    out
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// One capture per input value: `f(v)` on C vs Rust, byte-for-byte.
fn diff_char_each(sym: &[u8], row: &str, inputs: &[i8]) {
    let cf = sym_char(c_lib(), sym);
    let rf = sym_char(rust_lib(), sym);
    for &v in inputs {
        let c = capture(|| unsafe { cf(v) });
        let r = capture(|| unsafe { rf(v) });
        assert_eq!(
            c,
            r,
            "[{row}] {} (0x{:02x} / {}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&sym[..sym.len() - 1]),
            v as u8,
            v,
            show(&c),
            show(&r)
        );
        assert!(!c.is_empty(), "[{row}] C produced no output at all");
    }
}

/// All inputs in a SINGLE capture, so call ordering and stdio buffering are
/// compared too.
fn diff_char_batch(sym: &[u8], row: &str, inputs: &[i8]) {
    let cf = sym_char(c_lib(), sym);
    let rf = sym_char(rust_lib(), sym);
    let c = capture(|| {
        for &v in inputs {
            unsafe { cf(v) }
        }
    });
    let r = capture(|| {
        for &v in inputs {
            unsafe { rf(v) }
        }
    });
    assert_eq!(
        c,
        r,
        "[{row}] batch of {} calls to {} diverged\nC   ={:?}\nRust={:?}",
        inputs.len(),
        String::from_utf8_lossy(&sym[..sym.len() - 1]),
        show(&c),
        show(&r)
    );
}

/// Call through the wrong-width `int` prototype.
fn diff_int_each(sym: &[u8], row: &str, inputs: &[i32]) {
    let cf = sym_int(c_lib(), sym);
    let rf = sym_int(rust_lib(), sym);
    for &v in inputs {
        let c = capture(|| unsafe { cf(v) });
        let r = capture(|| unsafe { rf(v) });
        assert_eq!(
            c,
            r,
            "[{row}] {} as fn(int) with {v} (0x{:08x}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&sym[..sym.len() - 1]),
            v as u32,
            show(&c),
            show(&r)
        );
    }
}

/// Call through the wrong-width `long` prototype (64-bit register garbage).
fn diff_long_each(sym: &[u8], row: &str, inputs: &[i64]) {
    let cf = sym_long(c_lib(), sym);
    let rf = sym_long(rust_lib(), sym);
    for &v in inputs {
        let c = capture(|| unsafe { cf(v) });
        let r = capture(|| unsafe { rf(v) });
        assert_eq!(
            c,
            r,
            "[{row}] {} as fn(long) with 0x{:016x}: C={:?} Rust={:?}",
            String::from_utf8_lossy(&sym[..sym.len() - 1]),
            v as u64,
            show(&c),
            show(&r)
        );
    }
}

/// Run `f` in a forked child and return the raw `waitpid` status, so a crash /
/// abort / non-zero exit in either library is observable.  Used for the
/// "printf fails and the library ignores it" row.
fn status_in_child(f: impl FnOnce()) -> c_int {
    let guard = cap_lock().lock().unwrap_or_else(|e| e.into_inner());
    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: make stdout unbuffered so printf really issues the write(2)
        // (and really sees it fail), then break fd 1.
        unsafe {
            setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
            close(1);
        }
        f();
        unsafe { _exit(0) };
    }
    let mut status: c_int = -1;
    let w = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    drop(guard);
    status
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64), fixed seed for reproducibility.
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_D1FF;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform-ish `u8` in `lo..=hi`.
    fn u8_in(&mut self, lo: u8, hi: u8) -> u8 {
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u8
    }
    fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    fn i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
}

/// `n` random bytes drawn from `lo..=hi`, reinterpreted as `char` (i8).
fn random_chars(n: usize, lo: u8, hi: u8, seed: u64) -> Vec<i8> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rng.u8_in(lo, hi) as i8).collect()
}

/// Every one of the 256 `char` bit patterns, ascending by unsigned value.
fn all_256() -> Vec<i8> {
    (0u16..256).map(|v| v as u8 as i8).collect()
}
