//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and called *only* through their exported `custom_strdup`
//! symbol. Nothing in this file (or in any test) calls the Rust crate
//! directly — that is deliberate, so the `#[no_mangle]`/`extern "C"` export
//! wrapper is under test too, exactly as an external C consumer would see it.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The ABI under test: `char *custom_strdup(const char *str)`.
pub type CustomStrdupFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

/// Which implementation a result came from (for assertion messages).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Impl {
    C,
    Rust,
}

pub struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: CustomStrdupFn,
    pub rust: CustomStrdupFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// SAFETY: after loading, we only ever read the resolved function pointers; the
// two `Library` handles are kept alive for the whole process lifetime.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

/// Directory containing this crate (`translation/`).
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared library built by `c_src/CMakeLists.txt`.
fn c_so_path() -> PathBuf {
    let root = manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf();

    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
        root.join("c_src/build/Debug/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {candidates:#?}"
    );
}

/// Locate the Rust cdylib for the profile the tests are currently running under.
///
/// `current_exe()` is `target/<profile>/deps/<testname>-<hash>`, so the cdylib
/// sits two levels up. This keeps debug/release runs pointed at the right
/// artifact without guessing.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_DRIVER_SO does not point at a file: {p:?}");
        return p;
    }

    if let Ok(exe) = std::env::current_exe() {
        // target/<profile>/deps/<test bin>  ->  target/<profile>/libdriver.so
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            let cand = profile_dir.join("libdriver.so");
            if cand.is_file() {
                return cand;
            }
        }
    }

    let target = manifest_dir().join("target");
    let ordered: [PathBuf; 2] = if cfg!(debug_assertions) {
        [
            target.join("debug/libdriver.so"),
            target.join("release/libdriver.so"),
        ]
    } else {
        [
            target.join("release/libdriver.so"),
            target.join("debug/libdriver.so"),
        ]
    };
    for c in &ordered {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("Rust cdylib libdriver.so not found; looked in: {ordered:#?}");
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Load both shared objects (once per process) and resolve `custom_strdup`
/// from each.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({c_path:?}) failed: {e}"));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({rust_path:?}) failed: {e}"));

            let c_sym: Symbol<CustomStrdupFn> = c_lib
                .get(b"custom_strdup\0")
                .expect("C .so does not export `custom_strdup`");
            let rust_sym: Symbol<CustomStrdupFn> = rust_lib
                .get(b"custom_strdup\0")
                .expect("Rust .so does not export `custom_strdup`");

            let c = *c_sym;
            let rust = *rust_sym;

            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    })
}

unsafe extern "C" {
    fn free(p: *mut std::ffi::c_void);
}

/// Release a pointer returned by either implementation using libc `free`, which
/// is what the C contract demands (`malloc`-allocated storage). If the Rust
/// translation had used the Rust global allocator, this would corrupt the heap.
pub unsafe fn c_free(p: *mut c_char) {
    if !p.is_null() {
        unsafe { free(p as *mut std::ffi::c_void) }
    }
}

/// Read back a NUL-terminated result as raw bytes *including* the terminator.
pub unsafe fn bytes_with_nul(p: *const c_char) -> Vec<u8> {
    assert!(!p.is_null());
    let mut out = Vec::new();
    let mut i = 0isize;
    loop {
        let b = unsafe { *p.offset(i) } as u8;
        out.push(b);
        if b == 0 {
            break;
        }
        i += 1;
    }
    out
}

/// The core differential assertion.
///
/// Calls both exported `custom_strdup` implementations on the *same* input bytes
/// (`input` must contain its own NUL terminator) and asserts:
///  * both return NULL, or both return non-NULL (same rejection decision);
///  * the copied bytes are byte-for-byte identical, terminator included;
///  * the copy equals the expected prefix of the source (up to the first NUL);
///  * the result does not alias the source.
///
/// Frees both results with libc `free`.
#[track_caller]
pub fn assert_same(input: &[u8], label: &str) {
    assert!(
        input.contains(&0),
        "{label}: test input must be NUL-terminated"
    );
    let src = input.as_ptr() as *const c_char;
    let l = libs();

    let (cp, rp) = unsafe { ((l.c)(src), (l.rust)(src)) };

    assert_eq!(
        cp.is_null(),
        rp.is_null(),
        "{label}: NULL-ness diverged (C null={}, Rust null={})",
        cp.is_null(),
        rp.is_null()
    );

    if cp.is_null() {
        return;
    }

    let expected: Vec<u8> = {
        let n = input.iter().position(|&b| b == 0).unwrap();
        let mut v = input[..n].to_vec();
        v.push(0);
        v
    };

    let cb = unsafe { bytes_with_nul(cp) };
    let rb = unsafe { bytes_with_nul(rp) };

    assert_eq!(
        cb, rb,
        "{label}: copied bytes diverged (len C={}, Rust={})",
        cb.len(),
        rb.len()
    );
    assert_eq!(cb, expected, "{label}: C output did not match source prefix");
    assert_eq!(
        rb, expected,
        "{label}: Rust output did not match source prefix"
    );

    assert_ne!(cp as *const c_char, src, "{label}: C result aliases input");
    assert_ne!(rp as *const c_char, src, "{label}: Rust result aliases input");
    assert_ne!(cp, rp, "{label}: the two results must be distinct buffers");

    unsafe {
        c_free(cp);
        c_free(rp);
    }
}

/// Convenience: build a NUL-terminated buffer from payload bytes and diff it.
#[track_caller]
pub fn assert_same_payload(payload: &[u8], label: &str) {
    assert!(
        !payload.contains(&0),
        "{label}: payload must not contain NUL (use assert_same for that)"
    );
    let mut buf = payload.to_vec();
    buf.push(0);
    assert_same(&buf, label);
}

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

    pub fn new() -> Self {
        Rng(Self::SEED)
    }

    pub fn with_seed(seed: u64) -> Self {
        Rng(if seed == 0 { Self::SEED } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }

    pub fn in_range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }

    /// A non-NUL byte (`0x01..=0xFF`).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }

    /// `len` random non-NUL bytes.
    pub fn payload(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }

    /// `len` random printable-ASCII bytes (`0x20..=0x7E`).
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| (0x20 + self.below(0x7F - 0x20)) as u8)
            .collect()
    }

    /// `len` random high-bit bytes (`0x80..=0xFF`) — invalid UTF-8.
    pub fn high_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (0x80 + self.below(0x80)) as u8).collect()
    }
}
