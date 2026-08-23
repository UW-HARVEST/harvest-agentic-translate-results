//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` (built by `c_src/CMakeLists.txt`) and the Rust
//! `cdylib` through `libloading` and exposes them as a pair, so every test
//! calls both implementations exactly as an external C consumer would.
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test binary's location
/// (`target/<profile>/deps/<test>-<hash>`).
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

static C_LIB: OnceLock<Library> = OnceLock::new();
static R_LIB: OnceLock<Library> = OnceLock::new();

pub fn c_lib() -> &'static Library {
    C_LIB.get_or_init(|| {
        let p = manifest_dir().join("c_src/build/liblz4.so");
        assert!(
            p.exists(),
            "C shared library not found at {p:?}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        unsafe { Library::new(&p) }.expect("dlopen C liblz4.so")
    })
}

pub fn r_lib() -> &'static Library {
    R_LIB.get_or_init(|| {
        let p = target_profile_dir().join("liblz4.so");
        assert!(p.exists(), "Rust cdylib not found at {p:?}");
        unsafe { Library::new(&p) }.expect("dlopen Rust liblz4.so")
    })
}

/// Fetch the same symbol from both libraries. Returns `(c_fn, rust_fn)`.
pub fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let cn: Symbol<'static, T> = unsafe { c_lib().get(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
    let rn: Symbol<'static, T> = unsafe { r_lib().get(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
    (cn, rn)
}

/// Declare a `(c, rust)` function-pointer pair for an exported symbol.
///
/// ```ignore
/// sym!(bound, "LZ4_compressBound", unsafe extern "C" fn(i32) -> i32);
/// let (c, r) = (bound.0, bound.1);
/// ```
#[macro_export]
macro_rules! sym {
    ($var:ident, $name:literal, $ty:ty) => {
        let $var: (
            libloading::Symbol<'static, $ty>,
            libloading::Symbol<'static, $ty>,
        ) = $crate::common::pair::<$ty>($name);
    };
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed => reproducible property-style testing)
// ---------------------------------------------------------------------------

/// splitmix64 — small, fast, deterministic, and identical across platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo + 1)
        }
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

// ---------------------------------------------------------------------------
// Input-shape generators (the "data shape" axis of CONFIGS.md)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Uniform random bytes — incompressible, exercises the literal paths.
    Random,
    /// Long runs of one byte — maximally compressible, long matches.
    Runs,
    /// Small alphabet, short period — many short matches.
    Periodic,
    /// Text-like: small alphabet with words, realistic match distribution.
    Texty,
    /// Mostly zeros with sparse random bytes.
    Sparse,
    /// Random blocks that repeat at a distance near `LZ4_DISTANCE_MAX`.
    FarMatches,
    /// Two halves: incompressible then highly compressible.
    Mixed,
}

pub const ALL_SHAPES: &[Shape] = &[
    Shape::Random,
    Shape::Runs,
    Shape::Periodic,
    Shape::Texty,
    Shape::Sparse,
    Shape::FarMatches,
    Shape::Mixed,
];

pub fn gen_data(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = vec![0u8; len];
    match shape {
        Shape::Random => rng.fill(&mut v),
        Shape::Runs => {
            let mut i = 0;
            while i < len {
                let run = rng.range(1, 300).min(len - i);
                let b = rng.next_u8();
                for k in 0..run {
                    v[i + k] = b;
                }
                i += run;
            }
        }
        Shape::Periodic => {
            let period = rng.range(1, 64);
            let mut pat = vec![0u8; period];
            for p in pat.iter_mut() {
                *p = rng.next_u8() & 0x0F;
            }
            for i in 0..len {
                v[i] = pat[i % period];
            }
        }
        Shape::Texty => {
            const AL: &[u8] = b"abcdefghijklmnopqrstuvwxyz ";
            let nwords = 64;
            let words: Vec<Vec<u8>> = (0..nwords)
                .map(|_| {
                    let n = rng.range(2, 9);
                    (0..n).map(|_| AL[rng.below(AL.len())]).collect()
                })
                .collect();
            let mut i = 0;
            while i < len {
                let w = &words[rng.below(nwords)];
                for &b in w.iter() {
                    if i < len {
                        v[i] = b;
                        i += 1;
                    }
                }
                if i < len {
                    v[i] = b' ';
                    i += 1;
                }
            }
        }
        Shape::Sparse => {
            let n = len / 50;
            for _ in 0..n {
                let i = rng.below(len.max(1));
                if i < len {
                    v[i] = rng.next_u8();
                }
            }
        }
        Shape::FarMatches => {
            // Fill the first chunk randomly, then copy it back from a distance
            // that straddles LZ4_DISTANCE_MAX (65535).
            let chunk = 4096.min(len);
            if chunk > 0 {
                rng.fill(&mut v[..chunk]);
            }
            let mut i = chunk;
            while i < len {
                let dist = *[65535usize, 65536, 65534, 32768, 1024]
                    .get(rng.below(5))
                    .unwrap();
                let n = rng.range(4, 512).min(len - i);
                for k in 0..n {
                    v[i + k] = if i + k >= dist { v[i + k - dist] } else { 0 };
                }
                i += n;
            }
        }
        Shape::Mixed => {
            let half = len / 2;
            rng.fill(&mut v[..half]);
            let mut i = half;
            while i < len {
                let run = rng.range(1, 200).min(len - i);
                let b = rng.next_u8();
                for k in 0..run {
                    v[i + k] = b;
                }
                i += run;
            }
        }
    }
    v
}

/// Input lengths that hit every size-dependent branch in the C:
/// 0/1 (empty & minimal), 11..14 (`LZ4_minLength` = 13 / `MFLIMIT` = 12),
/// 63..65, 65535 (`LZ4_DISTANCE_MAX`), 65536, 65546/65547/65548
/// (`LZ4_64Klimit` — the `byU16`→`byU32` table switch), and past 4 MB blocks.
pub const KEY_LENS: &[usize] = &[
    0, 1, 2, 3, 4, 5, 6, 11, 12, 13, 14, 15, 16, 17, 19, 20, 31, 63, 64, 65, 127, 128, 255, 256,
    511, 512, 1023, 1024, 4095, 4096, 4097, 8192, 16384, 65534, 65535, 65536, 65537, 65546, 65547,
    65548, 100_000, 131_072,
];

/// A smaller sweep for expensive (level 10-12 / optimal-parser) tests.
pub const SMALL_LENS: &[usize] = &[
    0, 1, 12, 13, 14, 63, 64, 65, 255, 1024, 4096, 8192, 65535, 65536, 65547, 100_000,
];

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Assert two byte slices are identical, reporting the first differing index.
#[track_caller]
pub fn assert_bytes_eq(c: &[u8], r: &[u8], ctx: &str) {
    if c == r {
        return;
    }
    assert_eq!(
        c.len(),
        r.len(),
        "{ctx}: length mismatch C={} Rust={}",
        c.len(),
        r.len()
    );
    for i in 0..c.len() {
        if c[i] != r[i] {
            let lo = i.saturating_sub(8);
            let hi = (i + 8).min(c.len());
            panic!(
                "{ctx}: first byte difference at index {i}: C=0x{:02x} Rust=0x{:02x}\n  \
                 C   [{lo}..{hi}] = {:02x?}\n  Rust[{lo}..{hi}] = {:02x?}",
                c[i],
                r[i],
                &c[lo..hi],
                &r[lo..hi]
            );
        }
    }
    unreachable!()
}

/// Assert two return codes match.
#[track_caller]
pub fn assert_ret_eq<T: PartialEq + std::fmt::Debug>(c: T, r: T, ctx: &str) {
    assert_eq!(c, r, "{ctx}: return value mismatch (C vs Rust)");
}

/// Assert a compression result (return code AND produced bytes) matches.
#[track_caller]
pub fn assert_out_eq(cn: i32, cbuf: &[u8], rn: i32, rbuf: &[u8], ctx: &str) {
    assert_eq!(cn, rn, "{ctx}: return value mismatch C={cn} Rust={rn}");
    if cn > 0 {
        let n = cn as usize;
        assert!(n <= cbuf.len() && n <= rbuf.len(), "{ctx}: bogus length {n}");
        assert_bytes_eq(&cbuf[..n], &rbuf[..n], ctx);
    }
}

/// Same for `size_t`-returning frame APIs.
#[track_caller]
pub fn assert_sz_eq(cn: usize, cbuf: &[u8], rn: usize, rbuf: &[u8], ctx: &str) {
    assert_eq!(cn, rn, "{ctx}: return mismatch C={cn:#x} Rust={rn:#x}");
    if !is_lz4f_error(cn) && cn > 0 {
        assert_bytes_eq(&cbuf[..cn], &rbuf[..cn], ctx);
    }
}

/// Mirrors `LZ4F_isError`: `code > (size_t)-LZ4F_ERROR_maxCode` (= `(usize)-24`).
pub fn is_lz4f_error(code: usize) -> bool {
    code > (0usize).wrapping_sub(24)
}

/// `(size_t)-n`, i.e. the value `LZ4F_returnErrorCode` produces for ordinal `n`.
pub fn lz4f_err(n: usize) -> usize {
    (0usize).wrapping_sub(n)
}

// ---------------------------------------------------------------------------
// 8-byte-aligned scratch buffer (LZ4_initStream / LZ4_initStreamHC require it)
// ---------------------------------------------------------------------------

/// A heap buffer guaranteed to be 8-byte aligned (actually 16, via `Vec<u64>`).
pub struct Aligned {
    buf: Vec<u64>,
    len: usize,
}

impl Aligned {
    pub fn new(len: usize) -> Self {
        Aligned {
            buf: vec![0u64; (len + 7) / 8 + 1],
            len,
        }
    }
    pub fn ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr() as *mut u8
    }
    pub fn len(&self) -> usize {
        self.len
    }
}

pub const SIZEOF_LZ4_STREAM_T: usize = 16416;
pub const SIZEOF_LZ4_STREAMHC_T: usize = 262200;
pub const LZ4_MAX_INPUT_SIZE: i32 = 0x7E00_0000;
pub const LZ4_ACCELERATION_MAX: i32 = 65537;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_VERSION: u32 = 100;
pub const LZ4F_MAGICNUMBER: u32 = 0x184D_2204;
