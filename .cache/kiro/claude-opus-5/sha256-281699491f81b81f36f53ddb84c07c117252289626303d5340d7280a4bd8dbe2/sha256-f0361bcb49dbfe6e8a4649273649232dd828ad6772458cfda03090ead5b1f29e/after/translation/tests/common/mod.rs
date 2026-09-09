//! Shared differential-testing harness.
//!
//! Loads BOTH the C `liblz4.so` and the Rust `liblz4.so` via `libloading` and
//! exposes them as a pair, so every test calls the exact exported `#[no_mangle]`
//! symbols an external consumer would.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::sync::OnceLock;

pub const C_SO: &str = "../c_src/build/liblz4.so";
pub const RS_SO: &str = "target/release/liblz4.so";

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

impl Pair {
    /// Fetch the same symbol from both libraries.
    pub fn sym<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let cs = unsafe { self.c.get::<T>(name) }
            .unwrap_or_else(|e| panic!("C  .so missing {}: {e}", String::from_utf8_lossy(name)));
        let rs = unsafe { self.r.get::<T>(name) }.unwrap_or_else(|e| {
            panic!(
                "Rust .so missing {}: {e}",
                String::from_utf8_lossy(name)
            )
        });
        (cs, rs)
    }
}

static LIBS: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    LIBS.get_or_init(|| {
        let c = unsafe { Library::new(C_SO) }
            .unwrap_or_else(|e| panic!("cannot load {C_SO}: {e} (build the C lib first)"));
        let r = unsafe { Library::new(RS_SO) }
            .unwrap_or_else(|e| panic!("cannot load {RS_SO}: {e} (cargo build --release first)"));
        Pair { c, r }
    })
}

/// Grab a matching pair of function pointers (C first, Rust second) by symbol
/// name. `T` must be a plain `extern "C"` fn pointer type. The owning
/// `Library` handles live in a `OnceLock` for the whole process, so the copied
/// pointers stay valid.
pub fn syms<T: Copy>(name: &str) -> (T, T) {
    let mut c_name = String::with_capacity(name.len() + 1);
    c_name.push_str(name);
    c_name.push('\0');
    let l = libs();
    let cs: Symbol<T> = unsafe { l.c.get(c_name.as_bytes()) }
        .unwrap_or_else(|e| panic!("C  .so missing `{name}`: {e}"));
    let rs: Symbol<T> = unsafe { l.r.get(c_name.as_bytes()) }
        .unwrap_or_else(|e| panic!("Rust .so missing `{name}`: {e}"));
    (*cs, *rs)
}

// ---------------------------------------------------------------- PRNG -------

/// splitmix64 — deterministic, seedable, no external crates.
#[derive(Clone)]
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xDEAD_BEEF_CAFE_BABE)
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
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// inclusive range
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo + 1)
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

// ------------------------------------------------------- data generators -----

/// The kind of payload to generate. Each shape drives different code paths in
/// the match finder (incompressible -> literal runs, repetitive -> long
/// matches, small alphabet -> many short matches).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Uniform random bytes: essentially incompressible.
    Random,
    /// A handful of distinct bytes: lots of short matches.
    SmallAlphabet,
    /// One repeated byte: maximal-length matches, pattern analysis.
    Constant,
    /// Repeating short period (exercises LZ4HC pattern analysis / rep-detect).
    Periodic,
    /// Text-like: words drawn from a small dictionary.
    Textish,
    /// Random blocks that are re-used, so long matches at long distances.
    Chunky,
}

pub const ALL_SHAPES: [Shape; 6] = [
    Shape::Random,
    Shape::SmallAlphabet,
    Shape::Constant,
    Shape::Periodic,
    Shape::Textish,
    Shape::Chunky,
];

/// Build a payload of exactly `len` bytes.
///
/// The returned `Vec` is always backed by an allocation of at least 64
/// zero-initialised bytes, so `as_ptr()` is a valid readable pointer even when
/// `len == 0`. This matters because some LZ4 code paths (notably the level-10+
/// optimal parser reached through `LZ4_compress_HC_destSize`) dereference `src`
/// before consulting `srcSize`; a zero-capacity `Vec`'s dangling pointer makes
/// BOTH libraries segfault identically, which tests nothing.
pub fn mkdata(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    match shape {
        Shape::Random => {
            while v.len() < len {
                v.extend_from_slice(&rng.next_u64().to_le_bytes());
            }
            v.truncate(len);
        }
        Shape::SmallAlphabet => {
            let n = rng.range(2, 6);
            let alpha: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            for _ in 0..len {
                let i = rng.below(alpha.len());
                v.push(alpha[i]);
            }
        }
        Shape::Constant => {
            let b = rng.byte();
            v.resize(len, b);
        }
        Shape::Periodic => {
            let p = rng.range(1, 17);
            let pat: Vec<u8> = (0..p).map(|_| rng.byte()).collect();
            while v.len() < len {
                v.extend_from_slice(&pat);
            }
            v.truncate(len);
        }
        Shape::Textish => {
            const WORDS: [&[u8]; 12] = [
                b"the ", b"quick ", b"brown ", b"fox ", b"jumps ", b"over ", b"lazy ", b"dog ",
                b"lorem ", b"ipsum ", b"dolor ", b"sit ",
            ];
            while v.len() < len {
                v.extend_from_slice(WORDS[rng.below(WORDS.len())]);
            }
            v.truncate(len);
        }
        Shape::Chunky => {
            let bs = rng.range(8, 64);
            let nblocks = rng.range(1, 8);
            let pool: Vec<Vec<u8>> = (0..nblocks)
                .map(|_| (0..bs).map(|_| rng.byte()).collect())
                .collect();
            while v.len() < len {
                v.extend_from_slice(&pool[rng.below(pool.len())]);
            }
            v.truncate(len);
        }
    }
    debug_assert_eq!(v.len(), len);
    let mut out = vec![0u8; len.max(64)];
    out[..len].copy_from_slice(&v);
    out.truncate(len);
    out
}

/// Length classes that matter for the LZ4 block format and the hash tables.
pub const BOUNDARY_LENS: [usize; 22] = [
    0, 1, 2, 3, 4, 5, 11, 12, 13, 14, 15, 16, 17, 63, 64, 65, 127, 128, 255, 256, 4096, 5000,
];

/// Length classes around LZ4_64KLIMIT (65547) and the 64 KB window.
pub const BIG_LENS: [usize; 8] = [65534, 65535, 65536, 65537, 65546, 65547, 65548, 200000];

// ------------------------------------------------------------- helpers -------

/// Keeps every source block of a streaming session alive.
///
/// The LZ4 streaming APIs require that previously submitted source data stay at
/// the same address: `lz4.h:431` — "The previous 64KB of source data is
/// __assumed__ to remain present, unmodified, at same address in memory!", and
/// `lz4hc.h:134` — "Previous input blocks, including initial dictionary when
/// present, must remain accessible and unmodified during compression."
///
/// Dropping a block between `LZ4_compress_*_continue` calls is a use-after-free.
/// It does not merely produce garbage: the C and Rust `.so`s have independent
/// allocators, so they read *different* residual bytes out of the freed
/// dictionary tail and diverge for reasons that have nothing to do with the
/// translation. Every streaming test therefore parks its blocks here.
pub struct BlockArena(Vec<Vec<u8>>);

impl Default for BlockArena {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockArena {
    pub fn new() -> Self {
        BlockArena(Vec::new())
    }
    /// Take ownership of `v` and hand back a slice valid for as long as the
    /// arena lives. Growing the outer `Vec` relocates only the `Vec<u8>`
    /// headers, never the heap buffers they point at, so the slice stays put.
    pub fn keep(&mut self, v: Vec<u8>) -> &'static [u8] {
        self.0.push(v);
        let last = self.0.last().unwrap();
        unsafe { std::slice::from_raw_parts(last.as_ptr(), last.len()) }
    }
}

pub fn hexish(b: &[u8]) -> String {
    let n = b.len().min(64);
    let mut s = String::new();
    for x in &b[..n] {
        s.push_str(&format!("{x:02x}"));
    }
    if b.len() > n {
        s.push_str("...");
    }
    s
}

/// Assert two (return code, output buffer) results are identical. Only the
/// first `used` bytes are compared when the call reports a length; the full
/// buffer is compared when `full` is true (catches spurious writes).
pub fn same(
    label: &str,
    cr: i64,
    cbuf: &[u8],
    rr: i64,
    rbuf: &[u8],
) {
    assert_eq!(cr, rr, "{label}: return code C={cr} Rust={rr}");
    if cr > 0 {
        let n = (cr as usize).min(cbuf.len()).min(rbuf.len());
        assert_eq!(
            &cbuf[..n],
            &rbuf[..n],
            "{label}: output differs (len {n})\n C={}\n R={}",
            hexish(&cbuf[..n]),
            hexish(&rbuf[..n])
        );
    }
}

/// Full-buffer comparison including scratch beyond the reported length.
pub fn same_full(label: &str, cr: i64, cbuf: &[u8], rr: i64, rbuf: &[u8]) {
    assert_eq!(cr, rr, "{label}: return code C={cr} Rust={rr}");
    assert_eq!(
        cbuf,
        rbuf,
        "{label}: full buffer differs\n C={}\n R={}",
        hexish(cbuf),
        hexish(rbuf)
    );
}

// -------------------------------------------------- common fn signatures -----

pub type FnCompressDefault =
    unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
pub type FnCompressFast =
    unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
pub type FnCompressBound = unsafe extern "C" fn(i32) -> i32;
pub type FnDecompressSafe =
    unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
pub type FnDecompressPartial =
    unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
pub type FnDecompressFast = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
pub type FnCompressHC =
    unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;

/// Convenience: produce a valid compressed block with the C library.
pub fn c_compress(src: &[u8]) -> Vec<u8> {
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (cc, _) = syms::<FnCompressDefault>("LZ4_compress_default");
    let bound = unsafe { cb(src.len() as i32) };
    assert!(bound > 0 || src.is_empty(), "bound {bound}");
    let mut dst = vec![0u8; (bound.max(1)) as usize];
    let n = unsafe {
        cc(
            src.as_ptr(),
            dst.as_mut_ptr(),
            src.len() as i32,
            dst.len() as i32,
        )
    };
    assert!(n > 0, "c_compress failed for len {}", src.len());
    dst.truncate(n as usize);
    dst
}

/// Convenience: HC-compress with the C library at a given level.
pub fn c_compress_hc(src: &[u8], level: i32) -> Vec<u8> {
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (cc, _) = syms::<FnCompressHC>("LZ4_compress_HC");
    let bound = unsafe { cb(src.len() as i32) };
    let mut dst = vec![0u8; bound.max(1) as usize];
    let n = unsafe {
        cc(
            src.as_ptr(),
            dst.as_mut_ptr(),
            src.len() as i32,
            dst.len() as i32,
            level,
        )
    };
    assert!(n > 0, "c_compress_hc failed len {} lvl {level}", src.len());
    dst.truncate(n as usize);
    dst
}

pub mod frame;
