//! Shared differential-test harness.
//!
//! Both shared objects are loaded with `libloading` and every call goes through
//! `dlsym`, so the `#[no_mangle]` export wrappers of the Rust `cdylib` are what
//! is under test — no Rust function is ever called directly.
//!
//! `stdout` is captured by `dup2`-ing fd 1 onto a temporary file around each
//! call. Both `.so`s write through the *process's* libc `stdout` FILE, so the
//! capture is byte-exact for either side, including buffering behaviour.

#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    static stdout: *mut c_void;
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {p:?}.\nBuild it first:\n  cd c_src && mkdir -p build \
         && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Locate the Rust `cdylib` under test, and refuse to run against a stale one.
///
/// This matters: `cargo test` does **not** build a `cdylib`-only lib target, so
/// without this guard the tests would happily `dlopen` an artifact left over
/// from an earlier `cargo build` and report a pass for source they never
/// exercised. The `.so` is resolved inside the *same* profile directory as the
/// running test binary (no cross-profile fallback, which would silently test the
/// release artifact from a debug run) and must be newer than every source file.
pub fn rust_so_path() -> PathBuf {
    let so = if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        PathBuf::from(p)
    } else {
        // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/libdriver.so
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|d| d.parent())
            .expect("test binary lives in target/<profile>/deps/")
            .to_path_buf();
        profile_dir.join("libdriver.so")
    };

    assert!(
        so.is_file(),
        "Rust cdylib not found at {so:?}.\n\
         `cargo test` does not build a cdylib-only lib target, so it must be built \
         explicitly first:\n  cd translation && cargo build   # add --release for the release profile\n\
         (or point RUST_DRIVER_SO at the .so to use)"
    );

    assert_not_stale(&so);
    so
}

/// Panic if `so` is older than any crate source file.
fn assert_not_stale(so: &Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat the Rust .so");

    let mut newer: Vec<PathBuf> = Vec::new();
    let mut sources: Vec<PathBuf> = vec![manifest_dir().join("Cargo.toml")];
    let src = manifest_dir().join("src");
    if let Ok(rd) = std::fs::read_dir(&src) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rs") {
                sources.push(p);
            }
        }
    }
    for s in sources {
        if let Ok(t) = std::fs::metadata(&s).and_then(|m| m.modified()) {
            if t > so_mtime {
                newer.push(s);
            }
        }
    }
    assert!(
        newer.is_empty(),
        "STALE ARTIFACT: {so:?} is older than {newer:?}.\n\
         Rebuild before testing:\n  cd translation && cargo build"
    );
}

// ---------------------------------------------------------------------------
// The two libraries, loaded once
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

// SAFETY: after loading we only ever read function pointers out of these.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        // SAFETY: both objects are plain C ABI libraries with no initializers
        // beyond libc's own. They are loaded RTLD_LOCAL (libloading's default),
        // so their identically-named symbols cannot shadow each other and each
        // library's internal calls bind to its own definitions.
        let c = unsafe { Library::new(c_so_path()) }.expect("dlopen C libdriver.so");
        let rs = unsafe { Library::new(rust_so_path()) }.expect("dlopen Rust libdriver.so");
        Libs { c, rs }
    })
}

/// Which of the two implementations a closure should drive.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
    fn lib(self) -> &'static Library {
        match self {
            Impl::C => &libs().c,
            Impl::Rust => &libs().rs,
        }
    }
}

// ---------------------------------------------------------------------------
// Typed accessors for the five exported symbols
// ---------------------------------------------------------------------------

pub type FnPrintLine = unsafe extern "C" fn(*const c_char);
pub type FnPrintIntLine = unsafe extern "C" fn(c_int);
pub type FnVoid = unsafe extern "C" fn();

/// Every exported symbol of the C `.so`, as reported by `nm -D --defined-only`.
pub const EXPORTED_SYMBOLS: &[&str] = &["bad", "driver", "good", "printIntLine", "printLine"];

pub struct Api {
    pub print_line: FnPrintLine,
    pub print_int_line: FnPrintIntLine,
    pub bad: FnVoid,
    pub good: FnVoid,
    pub driver: FnVoid,
}

pub fn api(which: Impl) -> Api {
    let lib = which.lib();
    unsafe {
        let pl: Symbol<FnPrintLine> = lib
            .get(b"printLine\0")
            .unwrap_or_else(|e| panic!("{} .so is missing `printLine`: {e}", which.name()));
        let pil: Symbol<FnPrintIntLine> = lib
            .get(b"printIntLine\0")
            .unwrap_or_else(|e| panic!("{} .so is missing `printIntLine`: {e}", which.name()));
        let b: Symbol<FnVoid> = lib
            .get(b"bad\0")
            .unwrap_or_else(|e| panic!("{} .so is missing `bad`: {e}", which.name()));
        let g: Symbol<FnVoid> = lib
            .get(b"good\0")
            .unwrap_or_else(|e| panic!("{} .so is missing `good`: {e}", which.name()));
        let d: Symbol<FnVoid> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("{} .so is missing `driver`: {e}", which.name()));
        Api {
            print_line: *pl,
            print_int_line: *pil,
            bad: *b,
            good: *g,
            driver: *d,
        }
    }
}

/// True if `which`'s `.so` exports `sym`.
pub fn exports(which: Impl, sym: &str) -> bool {
    let mut name = sym.as_bytes().to_vec();
    name.push(0);
    unsafe { which.lib().get::<*const c_void>(&name) }.is_ok()
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run `f` with fd 1 redirected to a temp file and return the bytes it wrote.
///
/// Serialized process-wide, because fd 1 is a process-global resource and
/// `cargo test` runs tests on multiple threads.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-difftest-{}-{}.out",
        std::process::id(),
        n
    ));

    let mut tmp = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create temp capture file");

    let out = unsafe {
        // Flush anything the harness itself buffered so it is not attributed
        // to the library under test: Rust's own line-buffered stdout first
        // (libtest's "test foo ... " prefix lives there), then libc's FILE.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(stdout);
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        // Flush the library's output *while* fd 1 is still the temp file.
        fflush(stdout);
        assert!(dup2(saved, 1) >= 0, "restore of stdout failed");
        close(saved);

        r
    };

    let mut buf = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("rewind capture file");
    tmp.read_to_end(&mut buf).expect("read capture file");
    drop(tmp);
    let _ = std::fs::remove_file(&path);

    match out {
        Ok(()) => buf,
        Err(p) => std::panic::resume_unwind(p),
    }
}

/// Run the same closure against the C and the Rust `.so` and assert the
/// captured `stdout` bytes are identical.
pub fn assert_same<F>(label: &str, mut body: F)
where
    F: FnMut(&Api),
{
    let c_out = capture_stdout(|| {
        let a = api(Impl::C);
        body(&a);
    });
    let rs_out = capture_stdout(|| {
        let a = api(Impl::Rust);
        body(&a);
    });

    if c_out != rs_out {
        panic!(
            "stdout divergence in {label}\n  C    ({} bytes): {}\n  Rust ({} bytes): {}\n  first diff at byte {:?}",
            c_out.len(),
            render(&c_out),
            rs_out.len(),
            render(&rs_out),
            c_out.iter().zip(rs_out.iter()).position(|(a, b)| a != b),
        );
    }
}

/// Human-readable, length-capped rendering of a captured byte stream.
pub fn render(bytes: &[u8]) -> String {
    const CAP: usize = 400;
    let shown = &bytes[..bytes.len().min(CAP)];
    let mut s = String::from("\"");
    for &b in shown {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    if bytes.len() > CAP {
        s.push_str(&format!(" …(+{} bytes)", bytes.len() - CAP));
    }
    s
}

// ---------------------------------------------------------------------------
// Payload helpers
// ---------------------------------------------------------------------------

/// A NUL-terminated payload. Interior NULs are rejected by `CString`, and a
/// `const char *` cannot represent them anyway, so callers must avoid `0x00`.
pub fn cstr(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("payload must not contain an interior NUL")
}

// ---------------------------------------------------------------------------
// Deterministic RNG (PCG32) — fixed seed, so every run is reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let mut r = Pcg32 {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u32() as u64 % span) as u32
    }

    /// Random bytes in `1..=255` (never `0x00`, which would terminate the string).
    pub fn bytes_nonzero(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.range(1, 255) as u8).collect()
    }

    /// Random printable-ASCII bytes (`0x20..=0x7e`).
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.range(0x20, 0x7e) as u8).collect()
    }
}
