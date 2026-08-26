//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called through their exported C ABI symbols — the Rust functions are never
//! called directly, so the `#[no_mangle]`/`extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type DecodeFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

unsafe extern "C" {
    pub fn free(ptr: *mut c_void);
    pub fn strlen(s: *const c_char) -> usize;
    /// glibc: the usable size of a heap block. Both implementations must hand
    /// back a block of the same size (`calloc(1, strlen+14)`), so this catches
    /// allocation-size divergences that the byte comparison cannot see.
    pub fn malloc_usable_size(ptr: *mut c_void) -> usize;
}

pub struct Api {
    pub c: DecodeFn,
    pub rust: DecodeFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
    // keep the libraries alive for the whole process
    _libs: Vec<Library>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/libdriver.so` — derived from the running test binary
/// (`target/<profile>/deps/<name>-<hash>`) so it works for any profile.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let candidates = [
        profile.join("libdriver.so"),
        deps.join("libdriver.so"),
        manifest_dir().join("target/debug/libdriver.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found; looked in {:?}. Run `cargo build` first.",
        candidates
    );
}

pub fn c_so_path() -> PathBuf {
    // `C_DRIVER_SO` allows pointing the suite at a differently-configured C
    // build (e.g. an optimized one) without touching c_src/.
    if let Some(p) = std::env::var_os("C_DRIVER_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "C_DRIVER_SO={p:?} does not exist");
        return p;
    }
    let p = manifest_dir().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

static API: OnceLock<Api> = OnceLock::new();

pub fn api() -> &'static Api {
    API.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let clib = Library::new(&c_path).expect("dlopen C lib");
            let rlib = Library::new(&rust_path).expect("dlopen Rust lib");
            let cf: Symbol<DecodeFn> = clib
                .get(b"decode_base64\0")
                .expect("C .so does not export decode_base64");
            let rf: Symbol<DecodeFn> = rlib
                .get(b"decode_base64\0")
                .expect("Rust .so does not export decode_base64");
            let c = *cf;
            let rust = *rf;
            Api {
                c,
                rust,
                c_path,
                rust_path,
                _libs: vec![clib, rlib],
            }
        }
    })
}

// ---------------------------------------------------------------------------
// differential comparison
// ---------------------------------------------------------------------------

fn hex(b: &[u8]) -> String {
    let mut s = String::new();
    for (i, x) in b.iter().enumerate() {
        if i == 64 {
            s.push_str("...");
            break;
        }
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// Result of one call, captured so the two sides can be compared.
pub struct Captured {
    pub was_null: bool,
    pub buf: Vec<u8>,
    pub usable: usize,
}

/// Call one implementation on a NUL-terminated buffer and snapshot the full
/// destination allocation (`strlen(src) + 1 + 13` bytes, exactly what the C
/// `calloc`s — so the trailing zero fill takes part in the comparison).
fn call_and_capture(f: DecodeFn, src: *const c_char, cap: usize) -> Captured {
    let p = unsafe { f(src) };
    if p.is_null() {
        return Captured {
            was_null: true,
            buf: Vec::new(),
            usable: 0,
        };
    }
    let usable = unsafe { malloc_usable_size(p as *mut c_void) };
    let buf = unsafe { std::slice::from_raw_parts(p as *const u8, cap) }.to_vec();
    unsafe { free(p as *mut c_void) };
    Captured {
        was_null: false,
        buf,
        usable,
    }
}

/// Core differential check: run both implementations on `input` (which MUST end
/// with a NUL byte) and assert byte-identical behaviour.
#[track_caller]
pub fn diff_buf(input: &[u8], label: &str) {
    assert_eq!(
        input.last().copied(),
        Some(0u8),
        "test bug: input must be NUL terminated ({label})"
    );
    let src = input.as_ptr() as *const c_char;
    let l = unsafe { strlen(src) } as i64 + 1; // int l = strlen(src) + 1
    let cap = (l + 13) as usize; // calloc(1, l + 13)

    let a = api();
    let got_c = call_and_capture(a.c, src, cap);
    let got_r = call_and_capture(a.rust, src, cap);

    let shown: Vec<u8> = input[..input.len().min(96)].to_vec();
    assert_eq!(
        got_c.was_null,
        got_r.was_null,
        "NULL-ness mismatch for {label}: C returned {}, Rust returned {} (input len {}, input {:?} / hex {})",
        if got_c.was_null { "NULL" } else { "non-NULL" },
        if got_r.was_null { "NULL" } else { "non-NULL" },
        input.len() - 1,
        String::from_utf8_lossy(&shown),
        hex(&shown),
    );
    if got_c.was_null {
        return;
    }
    // Both must hand back a block that really holds `calloc(1, strlen+14)`
    // bytes. (Exact equality of `malloc_usable_size` is NOT asserted: glibc may
    // serve a request from a larger free chunk, so the usable size depends on
    // heap history, not only on the requested size. For big, mmap-served
    // requests the value is deterministic, so those are compared exactly.)
    assert!(
        got_c.usable >= cap && got_r.usable >= cap,
        "destination allocation too small for {label} (input len {}): C block {} \
         usable bytes, Rust block {} usable bytes, the C requests \
         calloc(1, strlen(src) + 14) = {cap} bytes",
        input.len() - 1,
        got_c.usable,
        got_r.usable,
    );
    if cap >= (1 << 20) {
        assert_eq!(
            got_c.usable, got_r.usable,
            "mmap-served destination allocation size mismatch for {label} \
             (input len {}): C {} vs Rust {} usable bytes (requested {cap})",
            input.len() - 1,
            got_c.usable,
            got_r.usable,
        );
    }
    if got_c.buf != got_r.buf {
        let first = got_c
            .buf
            .iter()
            .zip(got_r.buf.iter())
            .position(|(x, y)| x != y)
            .unwrap_or(0);
        panic!(
            "output mismatch for {label}\n  input (len {}): {:?}\n  input hex: {}\n  first differing byte index {first}: C=0x{:02x} Rust=0x{:02x}\n  C   : {}\n  Rust: {}",
            input.len() - 1,
            String::from_utf8_lossy(&shown),
            hex(&shown),
            got_c.buf[first],
            got_r.buf[first],
            hex(&got_c.buf),
            hex(&got_r.buf),
        );
    }
}

/// Convenience: `s` without a NUL, appended here.
#[track_caller]
pub fn diff(s: &[u8], label: &str) {
    let mut v = s.to_vec();
    v.push(0);
    diff_buf(&v, label);
}

/// Both must return NULL.
#[track_caller]
pub fn diff_null(src: *const c_char, label: &str) {
    let a = api();
    let pc = unsafe { (a.c)(src) };
    let pr = unsafe { (a.rust)(src) };
    let cn = pc.is_null();
    let rn = pr.is_null();
    if !cn {
        unsafe { free(pc as *mut c_void) };
    }
    if !rn {
        unsafe { free(pr as *mut c_void) };
    }
    assert!(cn, "{label}: C was expected to return NULL but did not");
    assert!(
        rn,
        "{label}: C returned NULL but Rust returned a non-NULL pointer"
    );
}

// ---------------------------------------------------------------------------
// deterministic PRNG (fixed seed, no external deps)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ SEED | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
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
    /// uniform in `0..n`
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// any non-NUL byte (`0x01..=0xFF`)
    pub fn nonzero_byte(&mut self) -> u8 {
        let b = self.byte();
        if b == 0 {
            1
        } else {
            b
        }
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
}

// ---------------------------------------------------------------------------
// base64 helpers
// ---------------------------------------------------------------------------

/// `[A-Za-z0-9+/]` plus `'='` at index 64 — i.e. every byte `is_base64`
/// accepts, in the order `decode` classifies them.
pub const ALPHABET: &[u8; 65] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
pub const ALPHABET_NOPAD: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bytes that `is_base64` rejects (so `decode_base64` must skip them).
pub fn invalid_bytes() -> Vec<u8> {
    (1u16..=255)
        .map(|b| b as u8)
        .filter(|&b| {
            !(b.is_ascii_uppercase()
                || b.is_ascii_lowercase()
                || b.is_ascii_digit()
                || b == b'+'
                || b == b'/'
                || b == b'=')
        })
        .collect()
}

/// Standard RFC-4648 base64 encoder, used only to *generate* well-formed input.
pub fn b64_encode(data: &[u8], pad: bool) -> Vec<u8> {
    let t = ALPHABET_NOPAD;
    let mut out = Vec::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(t[(n >> 18) as usize & 63]);
        out.push(t[(n >> 12) as usize & 63]);
        if chunk.len() > 1 {
            out.push(t[(n >> 6) as usize & 63]);
        } else if pad {
            out.push(b'=');
        }
        if chunk.len() > 2 {
            out.push(t[n as usize & 63]);
        } else if pad {
            out.push(b'=');
        }
    }
    out
}

pub fn random_alphabet(rng: &mut Rng, len: usize, with_pad: bool) -> Vec<u8> {
    let set: &[u8] = if with_pad { ALPHABET } else { ALPHABET_NOPAD };
    (0..len).map(|_| rng.pick(set)).collect()
}
