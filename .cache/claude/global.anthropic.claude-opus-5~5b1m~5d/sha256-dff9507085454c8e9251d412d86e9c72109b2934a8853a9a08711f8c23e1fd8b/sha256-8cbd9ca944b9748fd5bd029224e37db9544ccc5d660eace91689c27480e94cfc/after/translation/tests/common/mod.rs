//! Shared differential-testing harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and called through their exported `driver` symbol.  The Rust
//! implementation is NEVER called directly as a Rust function — every call goes
//! through `dlsym`, exactly as an external C consumer would, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! `driver` communicates only through `stdout`, so the harness redirects fd 1
//! into a temporary file around each call and compares the captured bytes
//! byte-for-byte.

#![allow(dead_code)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Read;
use std::io::Seek;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;

pub type DriverFn = unsafe extern "C" fn(c_char);

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C stream, including the `stdout` the
    /// shared objects print through.
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

/// glibc's `LC_ALL`.
pub const LC_ALL: c_int = 6;

/// Sets the process locale; returns `true` if the locale was available.
pub fn set_process_locale(name: &str) -> bool {
    let mut buf: Vec<u8> = name.as_bytes().to_vec();
    buf.push(0);
    unsafe { !setlocale(LC_ALL, buf.as_ptr() as *const c_char).is_null() }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

struct Libs {
    // Keep the libraries alive for the whole process; the function pointers
    // below borrow from them.
    _c: libloading::Library,
    _rust: libloading::Library,
    c_driver: DriverFn,
    rust_driver: DriverFn,
    c_path: PathBuf,
    rust_path: PathBuf,
}

// The raw `extern "C"` pointers are plain code addresses; sharing them across
// threads is fine (all actual calls are serialised by `STDOUT_LOCK` anyway).
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.exists()).cloned()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let candidates = vec![
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    first_existing(&candidates).unwrap_or_else(|| {
        panic!(
            "C shared library not found; build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             Looked in: {candidates:?}"
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    let candidates = vec![
        md.join("target/release/libdriver.so"),
        md.join("target/debug/libdriver.so"),
    ];
    first_existing(&candidates).unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found; build it with:\n  cd translation && cargo build --release\n\
             Looked in: {candidates:?}"
        )
    })
}

/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` library (the
/// integration-test targets do not link against it), so it is entirely possible
/// to run the whole differential suite against a STALE `libdriver.so` and get a
/// green run that proves nothing.  This guard makes that impossible: the `.so`
/// must be at least as new as every Rust source file and as `Cargo.toml`.
fn assert_so_is_fresh(so: &std::path::Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("cannot stat the Rust .so");

    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![manifest_dir().join("src")];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    files.push(p);
                }
            }
        }
    }
    files.push(manifest_dir().join("Cargo.toml"));

    for p in files {
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if newest.as_ref().is_none_or(|(_, n)| t > *n) {
                newest = Some((p, t));
            }
        }
    }

    if let Some((path, t)) = newest {
        assert!(
            so_mtime >= t,
            "STALE Rust shared object!\n  {} is older than {}\n\
             `cargo test` does not rebuild a cdylib-only crate.  Run:\n    \
             cargo build --release   (or use ./run_tests.sh)\n\
             before `cargo test`, otherwise the differential suite tests an old binary.",
            so.display(),
            path.display()
        );
    }
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert_so_is_fresh(&rust_path);
        unsafe {
            // `Library::new` uses RTLD_LOCAL, so the two identically named
            // `driver` symbols do not collide.
            let c = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let rust = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));
            let c_sym: libloading::Symbol<DriverFn> = c
                .get(b"driver\0")
                .expect("C .so does not export `driver`");
            let rust_sym: libloading::Symbol<DriverFn> = rust
                .get(b"driver\0")
                .expect("Rust .so does not export `driver`");
            let c_driver = *c_sym;
            let rust_driver = *rust_sym;
            Libs {
                _c: c,
                _rust: rust,
                c_driver,
                rust_driver,
                c_path,
                rust_path,
            }
        }
    })
}

pub fn c_driver() -> DriverFn {
    libs().c_driver
}

pub fn rust_driver() -> DriverFn {
    libs().rust_driver
}

pub fn so_paths() -> (PathBuf, PathBuf) {
    let l = libs();
    (l.c_path.clone(), l.rust_path.clone())
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so captures must be serialised.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with fd 1 redirected into a temporary file and returns everything
/// written to it (raw bytes, NULs included).
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_capture_{}_{:?}.bin",
        std::process::id(),
        std::thread::current().id()
    ));

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("cannot create capture file");

    let out = unsafe {
        // Flush anything already pending so it is not swept into the capture.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        let mut buf = Vec::new();
        file.rewind().expect("rewind");
        file.read_to_end(&mut buf).expect("read capture");
        buf
    };

    drop(file);
    let _ = std::fs::remove_file(&path);
    out
}

/// Captures the output of a single `driver(c)` call on the C library.
pub fn c_output(c: c_char) -> Vec<u8> {
    let f = c_driver();
    capture(|| unsafe { f(c) })
}

/// Captures the output of a single `driver(c)` call on the Rust library.
pub fn rust_output(c: c_char) -> Vec<u8> {
    let f = rust_driver();
    capture(|| unsafe { f(c) })
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            0 => s.push_str("\\0"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Asserts C and Rust produce byte-identical stdout for `driver(c)`.
pub fn assert_same(c: c_char, context: &str) {
    let expected = c_output(c);
    let actual = rust_output(c);
    if expected != actual {
        // Find the first differing byte for a readable message.
        let pos = expected
            .iter()
            .zip(actual.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(expected.len().min(actual.len()));
        panic!(
            "DIVERGENCE [{context}] driver({c}) (byte 0x{:02x})\n  \
             first difference at offset {pos}\n  \
             C   ({} bytes): {}\n  \
             RUST({} bytes): {}",
            c as u8,
            expected.len(),
            show(&expected),
            actual.len(),
            show(&actual),
        );
    }
    // Structural sanity: the C output must always contain the 14 documented
    // fields.  (The newline COUNT is not 14 in general: when `c` is `'\n'` the
    // two `%c` conversions emit newlines of their own.)
    let text = show(&expected);
    for field in FIELDS {
        assert!(
            text.contains(&format!("{field}: ")),
            "C output for driver({c}) is missing field `{field}`: {text}"
        );
    }
}

/// The 14 field labels `driver` prints, in order.
pub const FIELDS: [&str; 14] = [
    "alphanumeric",
    "alphabetic",
    "lowercase",
    "uppercase",
    "digit",
    "hexadecimal",
    "control",
    "graphical",
    "space",
    "blank",
    "printing",
    "punctuation",
    "to lower",
    "to upper",
];

/// Number of newline bytes `driver(c)` emits: 14 line terminators, plus one per
/// `%c` conversion that happens to print a newline.
pub fn expected_newlines(c: c_char) -> usize {
    14 + if c == b'\n' as c_char { 2 } else { 0 }
}

pub fn assert_same_set(values: &[c_char], context: &str) {
    for &v in values {
        assert_same(v, context);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn next_i8(&mut self) -> i8 {
        self.next_u8() as i8
    }
    /// Uniform pick from a slice.
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            xs.swap(i, j);
        }
    }
}

/// All 256 `char` values in signed form.
pub fn all_chars() -> Vec<c_char> {
    (0u16..256).map(|b| b as u8 as c_char).collect()
}

/// The byte range `lo..=hi` as signed `char`s.
pub fn range(lo: u8, hi: u8) -> Vec<c_char> {
    (lo as u16..=hi as u16).map(|b| b as u8 as c_char).collect()
}
