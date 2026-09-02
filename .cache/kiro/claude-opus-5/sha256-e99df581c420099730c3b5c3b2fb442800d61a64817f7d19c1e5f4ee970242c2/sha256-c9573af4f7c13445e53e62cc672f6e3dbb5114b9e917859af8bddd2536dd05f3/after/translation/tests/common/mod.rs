//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called only through their exported `hdr_compare` symbol, so the tests
//! exercise the real ABI surface (including the Rust `#[no_mangle]` wrapper)
//! exactly as an external C consumer would.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Signature of the one exported function, per `c_src/include/lib.h`:
/// `int hdr_compare(const uint8_t *h1, const uint8_t *h2);`
pub type HdrCompareFn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

pub struct Libs {
    // Kept alive for the process lifetime so the raw fn pointers stay valid.
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    pub c: HdrCompareFn,
    pub rust: HdrCompareFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// SAFETY: both libraries are pure, stateless, read-only functions (no globals,
// no allocation, no TLS), so sharing them across test threads is sound.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object built by `c_src/CMakeLists.txt`. The library name
/// is derived from the parent directory name by CMake, so it is discovered by
/// scanning rather than hard-coded.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("HDR_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!("no lib*.so found in {}", build_dir.display());
    })
}

/// Locate the Rust `cdylib`. Prefers the release artifact (what an external
/// consumer would link against); falls back to the debug artifact.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("HDR_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libhdr_compare_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libhdr_compare_lib.so not found under {}/target/{{release,debug}}; run `cargo build --release` first",
        root.display()
    );
}

fn load(path: &Path) -> (libloading::Library, HdrCompareFn) {
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let sym: libloading::Symbol<HdrCompareFn> = lib
            .get(b"hdr_compare\0")
            .unwrap_or_else(|e| panic!("{} does not export `hdr_compare`: {e}", path.display()));
        let f = *sym;
        (lib, f)
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        let (c_lib, c) = load(&c_path);
        let (rust_lib, rust) = load(&rust_path);
        Libs {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Call both `.so` exports on the given 3-byte headers and assert the returned
/// `int`s are byte-identical.
#[track_caller]
pub fn assert_same(h1: &[u8; 3], h2: &[u8; 3]) -> c_int {
    let l = libs();
    let (c, r) = unsafe {
        (
            (l.c)(h1.as_ptr(), h2.as_ptr()),
            (l.rust)(h1.as_ptr(), h2.as_ptr()),
        )
    };
    assert_eq!(
        c, r,
        "divergence for h1={h1:02x?} h2={h2:02x?}: C returned {c}, Rust returned {r}"
    );
    c
}

/// Same as [`assert_same`] but for raw pointers (null / unmapped / aliased).
#[track_caller]
pub fn assert_same_raw(h1: *const u8, h2: *const u8) -> c_int {
    let l = libs();
    let (c, r) = unsafe { ((l.c)(h1, h2), (l.rust)(h1, h2)) };
    assert_eq!(
        c, r,
        "divergence for raw h1={h1:p} h2={h2:p}: C returned {c}, Rust returned {r}"
    );
    c
}

/// Non-panicking single comparison, for hot exhaustive loops: returns
/// `Some((c, rust))` only on divergence so the loop body stays branch-light.
#[inline(always)]
pub fn diff(l: &Libs, h1: &[u8; 3], h2: &[u8; 3]) -> Option<(c_int, c_int)> {
    let (c, r) = unsafe {
        (
            (l.c)(h1.as_ptr(), h2.as_ptr()),
            (l.rust)(h1.as_ptr(), h2.as_ptr()),
        )
    };
    if c == r { None } else { Some((c, r)) }
}

#[track_caller]
pub fn check(l: &Libs, h1: &[u8; 3], h2: &[u8; 3]) {
    if let Some((c, r)) = diff(l, h1, h2) {
        panic!("divergence for h1={h1:02x?} h2={h2:02x?}: C returned {c}, Rust returned {r}");
    }
}

// ---------------------------------------------------------------------------
// Reference predicates transcribed from c_src/src/lib.c (used only to *classify*
// inputs when building test vectors, never as the expected value: expectations
// always come from the C .so itself).
// ---------------------------------------------------------------------------

pub fn byte1_passes_class(b: u8) -> bool {
    (b & 0xF0) == 0xf0 || (b & 0xFE) == 0xe2
}

pub fn byte1_valid(b: u8) -> bool {
    byte1_passes_class(b) && ((b >> 1) & 3) != 0
}

pub fn byte2_valid(b: u8) -> bool {
    (b >> 4) != 15 && ((b >> 2) & 3) != 3
}

/// All 14 `h2[1]` values accepted by `hdr_valid`.
pub fn valid_byte1_values() -> Vec<u8> {
    (0u16..256).map(|v| v as u8).filter(|&b| byte1_valid(b)).collect()
}

/// All 180 `h2[2]` values accepted by `hdr_valid`.
pub fn valid_byte2_values() -> Vec<u8> {
    (0u16..256).map(|v| v as u8).filter(|&b| byte2_valid(b)).collect()
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep every run reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline(always)]
    pub fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
    /// Uniform in `0..n` (n > 0).
    #[inline(always)]
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    #[inline(always)]
    pub fn pick(&mut self, xs: &[u8]) -> u8 {
        xs[self.below(xs.len())]
    }
}
