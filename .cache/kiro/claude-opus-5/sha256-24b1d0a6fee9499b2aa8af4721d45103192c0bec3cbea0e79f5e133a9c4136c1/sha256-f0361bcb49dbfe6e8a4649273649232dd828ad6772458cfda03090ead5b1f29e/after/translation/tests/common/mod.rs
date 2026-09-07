//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and driven purely through their exported `encode_base64`
//! symbol. The Rust implementation is *never* called directly, so the
//! `#[no_mangle] extern "C"` wrapper is exercised exactly as an external
//! consumer would exercise it.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

unsafe extern "C" {
    fn free(p: *mut std::ffi::c_void);
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  cd c_src && mkdir -p build \
         && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    // Allow an explicit override so the same suite can be pointed at the debug
    // cdylib (overflow checks ON) as well as the release one.
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO points at a missing file: {p:?}");
        return p;
    }
    let base = repo_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {base:?}. Build it with:\n  cd translation && cargo build --release"
    );
}

/// One loaded implementation, plus its resolved `encode_base64` entry point.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    func: EncodeBase64,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {path:?}: {e}"));
        let func = {
            let sym: Symbol<EncodeBase64> = unsafe { lib.get(b"encode_base64\0") }
                .unwrap_or_else(|e| panic!("{name}: symbol `encode_base64` not exported: {e}"));
            // Safe: `sym` borrows `lib`, which we move into `Impl` alongside the
            // raw fn pointer; the library therefore outlives every call.
            *sym
        };
        Impl {
            name,
            _lib: lib,
            func,
        }
    }

    /// Call `encode_base64(size, src)` and capture the observable result.
    ///
    /// `src` is passed as a raw pointer built from `src_bytes`; the caller is
    /// responsible for including a terminating NUL when `size == 0`.
    pub fn call(&self, size: c_int, src: Option<&[u8]>) -> Outcome {
        let ptr = match src {
            None => std::ptr::null(),
            Some(b) => b.as_ptr() as *const c_char,
        };
        let out = unsafe { (self.func)(size, ptr) };
        if out.is_null() {
            return Outcome::Null;
        }
        // The C contract is "returns an encoded string", i.e. the result is
        // read as a NUL-terminated string. `calloc` zeroing supplies the NUL.
        let mut bytes = Vec::new();
        unsafe {
            let mut p = out as *const u8;
            while *p != 0 {
                bytes.push(*p);
                p = p.add(1);
            }
            // Both implementations allocate with libc `calloc`, so libc `free`
            // is the correct release path for either.
            free(out as *mut std::ffi::c_void);
        }
        Outcome::Str(bytes)
    }

    /// Call and return the ENTIRE allocated buffer (`cap` bytes), not just the
    /// NUL-terminated prefix. This catches divergences hiding past the
    /// terminator -- e.g. an explicitly written NUL, or stale bytes where the C
    /// relies on `calloc` zeroing.
    pub fn call_full_buffer(&self, size: c_int, src: Option<&[u8]>) -> Option<Vec<u8>> {
        self.call_full_buffer_with_cap(size, src, cap_bytes(size))
    }

    /// As `call_full_buffer`, but with the capacity supplied by the caller.
    /// Needed in `strlen` mode (`size == 0`), where the C derives the capacity
    /// from `strlen(src)` rather than from the `size` argument.
    pub fn call_full_buffer_with_cap(
        &self,
        size: c_int,
        src: Option<&[u8]>,
        cap: usize,
    ) -> Option<Vec<u8>> {
        let ptr = match src {
            None => std::ptr::null(),
            Some(b) => b.as_ptr() as *const c_char,
        };
        let out = unsafe { (self.func)(size, ptr) };
        if out.is_null() {
            return None;
        }
        let buf = unsafe { std::slice::from_raw_parts(out as *const u8, cap).to_vec() };
        unsafe { free(out as *mut std::ffi::c_void) };
        Some(buf)
    }

    /// Call and immediately release, reporting only whether the allocation
    /// succeeded. Used where reading the buffer would be unsound or huge.
    pub fn call_nullness(&self, size: c_int, src: Option<&[u8]>) -> bool {
        let ptr = match src {
            None => std::ptr::null(),
            Some(b) => b.as_ptr() as *const c_char,
        };
        let out = unsafe { (self.func)(size, ptr) };
        let is_null = out.is_null();
        if !is_null {
            unsafe { free(out as *mut std::ffi::c_void) };
        }
        is_null
    }
}

/// The observable result of one `encode_base64` call.
#[derive(PartialEq, Eq, Clone)]
pub enum Outcome {
    Null,
    Str(Vec<u8>),
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Null => write!(f, "NULL"),
            Outcome::Str(b) => write!(f, "{:?} ({} bytes)", String::from_utf8_lossy(b), b.len()),
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The C and Rust implementations, loaded once per test process.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", c_so_path()),
        rs: Impl::load("Rust", rust_so_path()),
    })
}

impl Pair {
    /// Assert both implementations produce byte-identical observable results.
    #[track_caller]
    pub fn assert_same(&self, size: c_int, src: Option<&[u8]>, ctx: &str) {
        let a = self.c.call(size, src);
        let b = self.rs.call(size, src);
        if a != b {
            panic!(
                "divergence [{ctx}]\n  size = {size}\n  src  = {}\n  C    = {a:?}\n  Rust = {b:?}",
                describe(src)
            );
        }
    }

    /// Assert both implementations produce byte-identical results over the
    /// ENTIRE allocated buffer, including every byte past the NUL terminator.
    #[track_caller]
    pub fn assert_same_full_buffer(&self, size: c_int, src: Option<&[u8]>, ctx: &str) {
        let a = self.c.call_full_buffer(size, src);
        let b = self.rs.call_full_buffer(size, src);
        match (&a, &b) {
            (None, None) => {}
            (Some(x), Some(y)) if x == y => {}
            _ => panic!(
                "full-buffer divergence [{ctx}]\n  size = {size}\n  src  = {}\n  \
                 C    = {}\n  Rust = {}",
                describe(src),
                describe_opt(a.as_deref()),
                describe_opt(b.as_deref())
            ),
        }
    }

    /// Assert both agree on `NULL` vs non-`NULL` only.
    #[track_caller]
    pub fn assert_same_nullness(&self, size: c_int, src: Option<&[u8]>, ctx: &str) {
        let a = self.c.call_nullness(size, src);
        let b = self.rs.call_nullness(size, src);
        assert_eq!(
            a, b,
            "nullness divergence [{ctx}]: size = {size}, C returned NULL = {a}, \
             Rust returned NULL = {b}"
        );
    }
}

fn describe(src: Option<&[u8]>) -> String {
    match src {
        None => "NULL".to_string(),
        Some(b) if b.len() <= 48 => format!("{b:02x?} ({} bytes)", b.len()),
        Some(b) => format!("{:02x?}... ({} bytes)", &b[..48], b.len()),
    }
}

fn describe_opt(b: Option<&[u8]>) -> String {
    match b {
        None => "NULL".to_string(),
        Some(b) => describe(Some(b)),
    }
}

/// The exact capacity the C allocates: `size * 4 / 3 + 4` in wrapping signed
/// `int` arithmetic. Only valid (and only used) where the result is >= 0.
/// Note `size == -3` gives exactly 0, and glibc's `calloc(1, 0)` still returns
/// a non-NULL pointer to a zero-byte region.
pub fn cap_bytes(size: c_int) -> usize {
    let cap = size.wrapping_mul(4).wrapping_div(3).wrapping_add(4);
    assert!(cap >= 0, "cap_bytes called with negative cap for size={size}");
    cap as usize
}

/// Deterministic SplitMix64 PRNG — fixed seed per test for reproducibility.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Random bytes over the full `0x00..=0xFF` range.
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }

    /// Random printable-ASCII bytes (`0x20..=0x7E`), never NUL.
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| 0x20 + (self.byte() % 0x5F)).collect()
    }

    /// Random non-zero bytes (`0x01..=0xFF`) — safe for `strlen` mode.
    pub fn nonzero_bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let b = self.byte();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect()
    }
}

/// Reference base64 encoder written straight from the C, used as an extra
/// cross-check that the differential tests are actually exercising the codec
/// (and not, say, comparing two identical failures).
pub fn reference(size: c_int, src: &[u8]) -> Vec<u8> {
    fn enc(u: u8) -> u8 {
        if u < 26 {
            b'A' + u
        } else if u < 52 {
            b'a' + (u - 26)
        } else if u < 62 {
            b'0' + (u - 52)
        } else if u == 62 {
            b'+'
        } else {
            b'/'
        }
    }
    if size <= 0 {
        return Vec::new();
    }
    let n = size as usize;
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let b1 = src[i];
        let b2 = if i + 1 < n { src[i + 1] } else { 0 };
        let b3 = if i + 2 < n { src[i + 2] } else { 0 };
        out.push(enc(b1 >> 2));
        out.push(enc(((b1 & 0x3) << 4) | (b2 >> 4)));
        out.push(if i + 1 < n {
            enc(((b2 & 0xf) << 2) | (b3 >> 6))
        } else {
            b'='
        });
        out.push(if i + 2 < n { enc(b3 & 0x3f) } else { b'=' });
        i += 3;
    }
    out
}
