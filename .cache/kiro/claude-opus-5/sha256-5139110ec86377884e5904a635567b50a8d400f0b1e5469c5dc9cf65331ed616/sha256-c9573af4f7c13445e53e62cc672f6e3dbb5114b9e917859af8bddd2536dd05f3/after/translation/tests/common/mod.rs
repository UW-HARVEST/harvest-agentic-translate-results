//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported `decode_base64` symbol. The Rust
//! implementation is NEVER called directly, so the `#[no_mangle] extern "C"`
//! export wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type DecodeBase64Fn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    /// glibc extension: the usable size of a heap block. Used so that the
    /// `calloc(sizeof(char), l + 13)` / `malloc(l)` sizing contract is actually
    /// OBSERVABLE across the FFI boundary instead of taken on trust.
    fn malloc_usable_size(ptr: *mut c_void) -> usize;
    fn mallopt(param: c_int, value: c_int) -> c_int;
}

/// `M_MMAP_THRESHOLD` from glibc's `malloc.h`.
const M_MMAP_THRESHOLD: c_int = -3;

/// Pin the mmap threshold so the allocator at least stops self-tuning between
/// the C call and the Rust call. (This is not enough to make
/// `malloc_usable_size` a size oracle — binned chunks are reused ahead of it —
/// but it removes one source of run-to-run variation.)
fn force_deterministic_allocator() {
    unsafe {
        assert_eq!(mallopt(M_MMAP_THRESHOLD, 1), 1, "mallopt(M_MMAP_THRESHOLD) failed");
    }
}

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_decode: DecodeBase64Fn,
    pub rust_decode: DecodeBase64Fn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// `libloading::Library` is `Send + Sync` on all supported platforms; the raw
// function pointers are plain `fn` items.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("translation has a parent").to_path_buf()
}

fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.exists()).cloned()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/libdriver.dylib"),
    ];
    first_existing(&candidates).unwrap_or_else(|| {
        panic!(
            "C shared library not found. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             Searched: {candidates:?}"
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer the release artifact (the one that ships, with panic="abort"),
    // fall back to the debug artifact that `cargo test` produces.
    let candidates = [
        manifest.join("target/release/libdriver.so"),
        manifest.join("target/debug/libdriver.so"),
    ];
    first_existing(&candidates).unwrap_or_else(|| {
        panic!("Rust cdylib not found; run `cargo build --release`. Searched: {candidates:?}")
    })
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        force_deterministic_allocator();
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));
            let c_sym: Symbol<DecodeBase64Fn> = c_lib
                .get(b"decode_base64\0")
                .expect("C .so does not export decode_base64");
            let rust_sym: Symbol<DecodeBase64Fn> = rust_lib
                .get(b"decode_base64\0")
                .expect("Rust .so does not export decode_base64 (missing #[no_mangle]?)");
            let c_decode = *c_sym;
            let rust_decode = *rust_sym;
            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_decode,
                rust_decode,
                c_path,
                rust_path,
            }
        }
    })
}

/// Result of one `decode_base64` call, captured as owned bytes so the FFI
/// allocation can be released immediately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The function returned `NULL` (the library's only error sentinel).
    Null,
    /// Non-NULL: the *entire* `calloc`ed region, byte for byte.
    ///
    /// The C allocates `strlen(src) + 1 + 13` bytes with `calloc`, so the full
    /// region is well defined and zero initialised. Comparing all of it (rather
    /// than `strlen(dest)`) is required because decoded output legitimately
    /// contains interior `0x00` bytes.
    Buffer {
        full: Vec<u8>,
        c_strlen: usize,
        /// `malloc_usable_size` of the returned block — makes the C's
        /// `calloc(1, strlen(src)+1+13)` sizing observable.
        usable: usize,
    },
}

/// Call one implementation with a NUL-terminated copy of `input` and capture
/// the outcome, freeing the returned allocation.
///
/// `input` must not contain interior NUL bytes (it is a C string).
unsafe fn call_one(f: DecodeBase64Fn, input: &[u8]) -> Outcome {
    debug_assert!(!input.contains(&0), "input must not contain interior NUL");
    let mut cstr = Vec::with_capacity(input.len() + 1);
    cstr.extend_from_slice(input);
    cstr.push(0);

    let alloc_len = input.len() + 1 + 13;
    let ret = f(cstr.as_ptr() as *const c_char);
    if ret.is_null() {
        return Outcome::Null;
    }
    let full = std::slice::from_raw_parts(ret as *const u8, alloc_len).to_vec();
    let c_strlen = strlen(ret as *const c_char);
    let usable = malloc_usable_size(ret as *mut c_void);
    free(ret as *mut c_void);
    Outcome::Buffer { full, c_strlen, usable }
}

/// Call with an explicit raw pointer (used for the NULL-pointer error row).
unsafe fn call_one_raw(f: DecodeBase64Fn, ptr: *const c_char, alloc_len_if_ok: usize) -> Outcome {
    let ret = f(ptr);
    if ret.is_null() {
        return Outcome::Null;
    }
    let full = std::slice::from_raw_parts(ret as *const u8, alloc_len_if_ok).to_vec();
    let c_strlen = strlen(ret as *const c_char);
    let usable = malloc_usable_size(ret as *mut c_void);
    free(ret as *mut c_void);
    Outcome::Buffer { full, c_strlen, usable }
}

fn describe(input: &[u8]) -> String {
    let shown: String = input
        .iter()
        .take(96)
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                (b as char).to_string()
            } else {
                format!("\\x{b:02x}")
            }
        })
        .collect();
    format!(
        "len={} bytes={:?}{}",
        input.len(),
        shown,
        if input.len() > 96 { " …(truncated)" } else { "" }
    )
}

/// Core differential assertion: run BOTH `.so`s on `input` and require
/// byte-identical results.
#[track_caller]
pub fn assert_same(input: &[u8]) -> Outcome {
    let i = impls();
    let (c_out, rust_out) = unsafe {
        (
            call_one(i.c_decode, input),
            call_one(i.rust_decode, input),
        )
    };
    compare(&c_out, &rust_out, input.len() + 1 + 13, || describe(input));
    c_out
}

#[track_caller]
fn compare(c_out: &Outcome, rust_out: &Outcome, requested: usize, ctx: impl Fn() -> String) {
    match (c_out, rust_out) {
        (Outcome::Null, Outcome::Null) => {}
        (
            Outcome::Buffer { full: cf, c_strlen: cs, usable: cu },
            Outcome::Buffer { full: rf, c_strlen: rs, usable: ru },
        ) => {
            assert_eq!(cs, rs, "strlen differs for {}", ctx());
            if cf != rf {
                panic!(
                    "DIVERGENCE (buffer contents) for {}\n  C   : {}\n  Rust: {}",
                    ctx(),
                    hex(cf),
                    hex(rf)
                );
            }
            // Under-allocation in either implementation would be UB.
            assert!(*cu >= requested, "C under-allocated: {cu} < {requested}");
            assert!(*ru >= requested, "Rust under-allocated: {ru} < {requested}");
            // NOTE: `malloc_usable_size` is deliberately NOT compared for
            // equality. glibc reuses a binned chunk whenever one is available
            // and hands over a chunk whole when the remainder is too small to
            // split, so the value depends on heap state, not just the requested
            // size — it produced false divergences here. The EXACT requested
            // size is verified deterministically instead, by interposing
            // `calloc` (see tests/alloc_contract.rs). What is asserted here is
            // the soundness direction: neither side may under-allocate.
        }
        _ => panic!(
            "DIVERGENCE (NULL-ness) for {}\n  C   : {}\n  Rust: {}",
            ctx(),
            render(c_out),
            render(rust_out)
        ),
    }
}

/// Differential assertion for a raw pointer (NULL-pointer row).
#[track_caller]
pub fn assert_same_raw(ptr: *const c_char, alloc_len_if_ok: usize) -> Outcome {
    let i = impls();
    let (c_out, rust_out) = unsafe {
        (
            call_one_raw(i.c_decode, ptr, alloc_len_if_ok),
            call_one_raw(i.rust_decode, ptr, alloc_len_if_ok),
        )
    };
    compare(&c_out, &rust_out, alloc_len_if_ok, || format!("raw pointer {ptr:?}"));
    c_out
}

pub fn render(o: &Outcome) -> String {
    match o {
        Outcome::Null => "NULL".to_string(),
        Outcome::Buffer { full, c_strlen, usable } => {
            format!("strlen={c_strlen} usable={usable} full={}", hex(full))
        }
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join("")
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn pick(&mut self, set: &[u8]) -> u8 {
        set[self.below(set.len())]
    }
    /// A byte in `1..=255` (never NUL, so it is valid inside a C string).
    pub fn nonnul_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
}

pub const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
pub const DIGITS: &[u8] = b"0123456789";
pub const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub const ALPHABET_EQ: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
/// ASCII bytes that `is_base64` rejects (no NUL, nothing from the alphabet).
pub const NOISE: &[u8] = b" \t\n\r\x0b\x0c!\"#$%&'()*,-.:;<>?@[\\]^_`{|}~";

/// How many randomized inputs each `CONFIGS.md` row is driven with.
pub const ITERS: usize = 400;

/// Build a random string of `len` bytes drawn from `set`.
pub fn from_set(rng: &mut Rng, set: &[u8], len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.pick(set)).collect()
}
