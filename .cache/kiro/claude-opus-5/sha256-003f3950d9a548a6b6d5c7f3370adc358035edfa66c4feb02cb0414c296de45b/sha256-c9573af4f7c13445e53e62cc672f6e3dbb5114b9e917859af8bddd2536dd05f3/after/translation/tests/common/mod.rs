//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and looks up
//! exported symbols by name. Rust functions are NEVER called directly — every
//! call goes through the `.so` export, exactly as an external C caller would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Pair {
    pub c: Library,
    pub rs: Library,
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest().join("../c_src/build/liblz4.so")
}

fn rs_so() -> PathBuf {
    let rel = manifest().join("target/release/liblz4.so");
    if rel.exists() {
        return rel;
    }
    manifest().join("target/debug/liblz4.so")
}

pub fn libs() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| {
        let cp = c_so();
        let rp = rs_so();
        assert!(cp.exists(), "C .so not built: {}", cp.display());
        assert!(rp.exists(), "Rust .so not built: {}", rp.display());
        unsafe {
            Pair {
                c: Library::new(&cp).expect("load C .so"),
                rs: Library::new(&rp).expect("load Rust .so"),
            }
        }
    })
}

/// Fetch symbol `name` from both libraries, typed as `T`.
pub fn sym<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    let mut cn = name.as_bytes().to_vec();
    cn.push(0);
    unsafe {
        let c: Symbol<T> = l
            .c
            .get(&cn)
            .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
        let r: Symbol<T> = l
            .rs
            .get(&cn)
            .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
        (c, r)
    }
}

/// Declare a pair-getter for one exported symbol.
///
/// `decl_fn!(compress_default, "LZ4_compress_default",
///           unsafe extern "C" fn(*const u8,*mut u8,i32,i32)->i32);`
#[macro_export]
macro_rules! decl_fn {
    ($id:ident, $name:literal, $t:ty) => {
        #[allow(non_snake_case, dead_code)]
        pub fn $id() -> (
            libloading::Symbol<'static, $t>,
            libloading::Symbol<'static, $t>,
        ) {
            $crate::common::sym::<$t>($name)
        }
    };
}

// ---------------------------------------------------------------- determinism

/// splitmix64-based PRNG: reproducible across runs and platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    /// Uniform in `[0, n)`. `n == 0` yields 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo)
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Data shapes that drive different LZ4 match-finder paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Uniform random bytes: essentially incompressible.
    Random,
    /// Small alphabet: many short matches.
    LowEntropy,
    /// Long runs of one byte: very long matches.
    Runs,
    /// Repeated block of text: long-distance matches.
    Repetitive,
    /// All zeros: maximal compression.
    Zeros,
    /// English-ish text.
    Text,
    /// Random with periodic repeats: mixed.
    Mixed,
}

pub const SHAPES: [Shape; 7] = [
    Shape::Random,
    Shape::LowEntropy,
    Shape::Runs,
    Shape::Repetitive,
    Shape::Zeros,
    Shape::Text,
    Shape::Mixed,
];

pub fn make_data(rng: &mut Rng, len: usize, shape: Shape) -> Vec<u8> {
    // Always back the vector with a real allocation so that `as_ptr()` on an
    // empty buffer yields a dereferenceable address rather than the dangling
    // sentinel `Vec` uses for zero capacity. Some LZ4 code paths (notably the
    // HC lz4opt levels) touch `src` even when `srcSize == 0`, and passing the
    // sentinel would fault in the C reference itself.
    let mut v = Vec::with_capacity(len.max(1));
    match shape {
        Shape::Random => {
            for _ in 0..len {
                v.push(rng.byte());
            }
        }
        Shape::LowEntropy => {
            let alpha = [b'a', b'b', b'c', b'd'];
            for _ in 0..len {
                v.push(alpha[rng.below(4)]);
            }
        }
        Shape::Runs => {
            while v.len() < len {
                let b = rng.byte();
                let n = rng.range(1, 300).min(len - v.len());
                for _ in 0..n {
                    v.push(b);
                }
            }
        }
        Shape::Repetitive => {
            let unit_len = rng.range(4, 200);
            let mut unit = Vec::with_capacity(unit_len);
            for _ in 0..unit_len {
                unit.push(rng.byte());
            }
            while v.len() < len {
                let n = unit.len().min(len - v.len());
                v.extend_from_slice(&unit[..n]);
            }
        }
        Shape::Zeros => v.resize(len, 0),
        Shape::Text => {
            const W: [&str; 16] = [
                "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ", "lorem ",
                "ipsum ", "dolor ", "sit ", "amet ", "consectetur ", "adipiscing ", "elit ",
            ];
            while v.len() < len {
                let w = W[rng.below(16)].as_bytes();
                let n = w.len().min(len - v.len());
                v.extend_from_slice(&w[..n]);
            }
        }
        Shape::Mixed => {
            while v.len() < len {
                if rng.bool() && !v.is_empty() {
                    // copy an earlier span
                    let start = rng.below(v.len());
                    let n = rng.range(1, 64).min(v.len() - start).min(len - v.len());
                    let span: Vec<u8> = v[start..start + n].to_vec();
                    v.extend_from_slice(&span);
                } else {
                    let n = rng.range(1, 32).min(len - v.len());
                    for _ in 0..n {
                        v.push(rng.byte());
                    }
                }
            }
        }
    }
    v.truncate(len);
    v
}

/// Sizes that straddle every documented LZ4 boundary.
pub const BOUNDARY_SIZES: [usize; 24] = [
    0, 1, 2, 3, 4, 5, 12, 13, 15, 16, 63, 64, 255, 256, 4095, 4096, 65535, 65536, 65537, 65540,
    131_071, 131_072, 200_000, 262_144,
];

// ------------------------------------------------------------------ reporting

/// Assert two byte buffers are identical, printing the first divergence.
pub fn eq_bytes(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    if c.len() != r.len() {
        panic!("{ctx}: length differs: C={} Rust={}", c.len(), r.len());
    }
    for i in 0..c.len() {
        if c[i] != r[i] {
            let lo = i.saturating_sub(8);
            let hi = (i + 8).min(c.len());
            panic!(
                "{ctx}: first byte differs at {i}: C=0x{:02x} Rust=0x{:02x}\n  C   [{lo}..{hi}]={:02x?}\n  Rust[{lo}..{hi}]={:02x?}",
                c[i], r[i], &c[lo..hi], &r[lo..hi]
            );
        }
    }
    unreachable!()
}

pub fn eq<T: PartialEq + std::fmt::Debug>(ctx: &str, c: T, r: T) {
    assert_eq!(c, r, "{ctx}: C={c:?} Rust={r:?}");
}

// -------------------------------------------------------------- LZ4 constants

pub const LZ4_MAX_INPUT_SIZE: i32 = 0x7E00_0000;
pub const LZ4_STREAM_SIZE: usize = 16416;
pub const LZ4_STREAMHC_SIZE: usize = 262_200;
pub const LZ4_STREAMDECODE_SIZE: usize = 32;
pub const LZ4HC_CLEVEL_MIN: i32 = 2;
pub const LZ4HC_CLEVEL_DEFAULT: i32 = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: i32 = 10;
pub const LZ4HC_CLEVEL_MAX: i32 = 12;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;

pub fn lz4_compress_bound(n: i32) -> i32 {
    if n > LZ4_MAX_INPUT_SIZE || n < 0 {
        0
    } else {
        n + n / 255 + 16
    }
}

/// Over-aligned scratch buffer for `LZ4_stream_t` / `LZ4_streamHC_t`.
///
/// Allocated on the heap WITHOUT a large stack temporary: `LZ4_streamHC_t` is
/// ~256 KB, so materialising `[0u8; N]` on the stack (as `Box::new([0u8; N])`
/// does) overflows the 2 MB default test-thread stack once a test holds two of
/// them, which aborts the process depending on thread scheduling.
pub struct Aligned<const N: usize> {
    buf: Vec<u8>,
    off: usize,
}

impl<const N: usize> Aligned<N> {
    pub fn new() -> Self {
        let buf = vec![0u8; N + 64];
        let addr = buf.as_ptr() as usize;
        let off = (64 - (addr % 64)) % 64;
        Aligned { buf, off }
    }
    /// 64-byte-aligned pointer to the start of the N-byte region.
    pub fn ptr(&mut self) -> *mut u8 {
        unsafe { self.buf.as_mut_ptr().add(self.off) }
    }
    /// The N-byte region as a slice (excludes alignment padding, so two
    /// instances are directly comparable).
    pub fn bytes(&self) -> &[u8] {
        &self.buf[self.off..self.off + N]
    }
    pub fn fill0(&mut self) {
        let o = self.off;
        self.buf[o..o + N].fill(0);
    }
}

impl<const N: usize> Default for Aligned<N> {
    fn default() -> Self {
        Self::new()
    }
}
