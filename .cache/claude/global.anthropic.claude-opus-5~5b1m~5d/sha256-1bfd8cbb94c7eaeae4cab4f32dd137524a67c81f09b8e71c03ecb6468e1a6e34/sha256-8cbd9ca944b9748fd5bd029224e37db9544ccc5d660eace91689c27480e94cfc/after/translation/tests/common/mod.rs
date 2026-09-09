//! Shared harness: loads BOTH the C libzstd.so and the Rust libzstd.so via
//! libloading and exposes helpers to call the same symbol in both and compare.
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

static C_LIB: OnceLock<Library> = OnceLock::new();
static R_LIB: OnceLock<Library> = OnceLock::new();

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest().parent().unwrap().join("c_src/build/libzstd.so")
}

pub fn r_so_path() -> PathBuf {
    let rel = manifest().join("target/release/libzstd.so");
    if rel.exists() {
        return rel;
    }
    manifest().join("target/debug/libzstd.so")
}

pub fn clib() -> &'static Library {
    C_LIB.get_or_init(|| {
        let p = c_so_path();
        assert!(p.exists(), "C shared library not built: {:?}", p);
        unsafe { Library::new(&p).expect("load C .so") }
    })
}

pub fn rlib() -> &'static Library {
    R_LIB.get_or_init(|| {
        let p = r_so_path();
        assert!(p.exists(), "Rust shared library not built: {:?}", p);
        unsafe { Library::new(&p).expect("load Rust .so") }
    })
}

/// Fetch the same symbol from both libraries.
pub unsafe fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let n = name.as_bytes();
    let c: Symbol<'static, T> = clib()
        .get(n)
        .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
    let r: Symbol<'static, T> = rlib()
        .get(n)
        .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
    (c, r)
}

/// Declare `let (c_f, r_f) = sym!("NAME", fn_type);`
#[macro_export]
macro_rules! sym {
    ($name:expr, $t:ty) => {{ unsafe { $crate::common::pair::<$t>($name) } }};
}

// ---------------------------------------------------------------- PRNG -----

/// Deterministic xoshiro-ish PRNG (fixed seed => reproducible runs).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        // inclusive lo, exclusive hi
        if hi <= lo { lo } else { lo + self.below(hi - lo) }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

// ------------------------------------------------------------ generators ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// pure random bytes: incompressible
    Random,
    /// single repeated byte: RLE-able
    Rle,
    /// highly repetitive short-period pattern
    Repetitive,
    /// text-like: small alphabet, word structure
    Text,
    /// long matches far apart (exercises long-distance matching)
    LongMatches,
    /// mixed compressible / incompressible sections
    Mixed,
    /// mostly zeroes with sparse noise
    Sparse,
}

pub const ALL_SHAPES: &[Shape] = &[
    Shape::Random,
    Shape::Rle,
    Shape::Repetitive,
    Shape::Text,
    Shape::LongMatches,
    Shape::Mixed,
    Shape::Sparse,
];

pub fn gen(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    match shape {
        Shape::Random => {
            while v.len() < len {
                v.extend_from_slice(&rng.next_u64().to_le_bytes());
            }
            v.truncate(len);
        }
        Shape::Rle => {
            let b = rng.byte();
            v.resize(len, b);
        }
        Shape::Repetitive => {
            let period = rng.range(1, 40);
            let mut pat = Vec::with_capacity(period);
            for _ in 0..period {
                pat.push(rng.byte());
            }
            while v.len() < len {
                v.extend_from_slice(&pat);
            }
            v.truncate(len);
        }
        Shape::Text => {
            const WORDS: &[&str] = &[
                "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ", "zstd ",
                "compression ", "ratio ", "and ", "speed ", "\n",
            ];
            while v.len() < len {
                v.extend_from_slice(WORDS[rng.below(WORDS.len())].as_bytes());
            }
            v.truncate(len);
        }
        Shape::LongMatches => {
            let chunk = 4096.min(len.max(1));
            let mut base = Vec::with_capacity(chunk);
            for _ in 0..chunk {
                base.push(rng.byte());
            }
            while v.len() < len {
                v.extend_from_slice(&base);
                // insert some noise so it's not a pure repeat
                for _ in 0..rng.below(16) {
                    v.push(rng.byte());
                }
            }
            v.truncate(len);
        }
        Shape::Mixed => {
            while v.len() < len {
                if rng.next_u32() & 1 == 0 {
                    let n = rng.range(1, 512);
                    for _ in 0..n {
                        v.push(rng.byte());
                    }
                } else {
                    let n = rng.range(1, 512);
                    let b = rng.byte();
                    for _ in 0..n {
                        v.push(b);
                    }
                }
            }
            v.truncate(len);
        }
        Shape::Sparse => {
            v.resize(len, 0);
            let n = len / 64 + 1;
            for _ in 0..n {
                let i = rng.below(len.max(1));
                if i < v.len() {
                    v[i] = rng.byte();
                }
            }
        }
    }
    v
}

// --------------------------------------------------------------- utils -----

pub fn hexdump(b: &[u8], max: usize) -> String {
    let n = b.len().min(max);
    let mut s = String::new();
    for x in &b[..n] {
        s.push_str(&format!("{:02x}", x));
    }
    if b.len() > n {
        s.push_str("...");
    }
    s
}

/// Panic with a helpful diff if two byte buffers differ.
pub fn assert_bytes_eq(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let first = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(c.len().min(r.len()));
    panic!(
        "{ctx}: byte mismatch\n  C   len={} {}\n  Rust len={} {}\n  first diff at {}",
        c.len(),
        hexdump(&c[first.saturating_sub(8)..], 48),
        r.len(),
        hexdump(&r[first.saturating_sub(8)..], 48),
        first
    );
}

pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = u64::MAX;
pub const ZSTD_CONTENTSIZE_ERROR: u64 = u64::MAX - 1;

pub fn is_error(code: usize) -> bool {
    // ZSTD_isError(): `code > (size_t)-ZSTD_error_maxCode` with maxCode == 120,
    // i.e. an error iff 1 <= (0 - code) < 120 (exactly -120 is NOT an error).
    let neg = 0usize.wrapping_sub(code);
    code != 0 && neg < 120
}

pub fn err_code(code: usize) -> usize {
    0usize.wrapping_sub(code)
}
