//! Shared harness: loads the C reference `.so` and the Rust `.so` and exposes
//! matched pairs of symbols so every call crosses the real FFI boundary.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Libs {
    pub c: Library,
    pub r: Library,
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("LZ4_C_SO") {
        return PathBuf::from(p);
    }
    // CWD when running `cargo test` is the crate root (translation/).
    let mut candidates = Vec::new();
    let cwd = std::env::current_dir().unwrap();
    candidates.push(cwd.join("../c_src/build/liblz4.so"));
    candidates.push(cwd.join("c_src/build/liblz4.so"));
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("could not locate C liblz4.so; tried {:?}", candidates);
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("LZ4_RUST_SO") {
        return PathBuf::from(p);
    }
    let cwd = std::env::current_dir().unwrap();
    let mut candidates = Vec::new();
    // Prefer the cdylib built for the same profile as this test binary:
    // the test executable lives at target/<profile>/deps/<name>.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            candidates.push(profile_dir.join("liblz4.so"));
        }
    }
    candidates.push(cwd.join("target/release/liblz4.so"));
    candidates.push(cwd.join("target/debug/liblz4.so"));
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "could not locate Rust liblz4.so; run `cargo build --release` first. tried {:?}",
        candidates
    );
}

/// `cargo test` does not build a `cdylib`-only library target, so the `.so` under
/// `target/` can easily be stale relative to `src/`. Testing a stale artifact
/// produces results that have nothing to do with the current source, so fail
/// loudly instead.
fn assert_not_stale(so: &std::path::Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let mut src = std::env::current_dir().unwrap().join("src");
    if !src.exists() {
        src = std::env::current_dir().unwrap().join("translation/src");
    }
    let Ok(entries) = std::fs::read_dir(&src) else {
        return;
    };
    for e in entries.flatten() {
        let Ok(m) = e.metadata() else { continue };
        let Ok(t) = m.modified() else { continue };
        if t > so_mtime {
            panic!(
                "{} is older than {} — run `cargo build --release` before `cargo test`",
                so.display(),
                e.path().display()
            );
        }
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let cp = find_c_so();
        let rp = find_rust_so();
        assert_not_stale(&rp);
        unsafe {
            Libs {
                c: Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {:?}: {}", cp, e)),
                r: Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {:?}: {}", rp, e)),
            }
        }
    })
}

impl Libs {
    pub unsafe fn get<T>(&self, which: bool, name: &str) -> Symbol<'_, T> {
        let lib = if which { &self.r } else { &self.c };
        unsafe {
            lib.get(name.as_bytes())
                .unwrap_or_else(|e| panic!("symbol {} missing: {}", name, e))
        }
    }
}

/// `beq!(a, b, "ctx {}", x)` — compact byte-buffer equality assertion.
#[macro_export]
macro_rules! beq {
    ($a:expr, $b:expr, $($f:tt)*) => {
        $crate::common::cmp_bytes(&$a[..], &$b[..], &format!($($f)*))
    };
    ($a:expr, $b:expr) => {
        $crate::common::cmp_bytes(&$a[..], &$b[..], "bytes")
    };
}

/// `let (cf, rf) = pair!("LZ4_compressBound", fn(i32) -> i32);`
#[macro_export]
macro_rules! pair {
    ($name:literal, fn($($a:ty),* $(,)?) -> $r:ty) => {{
        type __F = unsafe extern "C" fn($($a),*) -> $r;
        let l = $crate::common::libs();
        #[allow(unused_unsafe)]
        let __v = unsafe { (l.get::<__F>(false, $name), l.get::<__F>(true, $name)) };
        __v
    }};
    ($name:literal, fn($($a:ty),* $(,)?)) => {{
        type __F = unsafe extern "C" fn($($a),*);
        let l = $crate::common::libs();
        #[allow(unused_unsafe)]
        let __v = unsafe { (l.get::<__F>(false, $name), l.get::<__F>(true, $name)) };
        __v
    }};
}

/* ------------------------------------------------------------------ */
/* deterministic data generators                                       */
/* ------------------------------------------------------------------ */

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
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
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
}

/// Ensure the returned buffer is backed by a real allocation with at least 64
/// readable zero bytes past `len`.
///
/// Two reasons this matters:
///  * `Vec::as_ptr()` on a zero-capacity vector returns a dangling pointer such
///    as `0x1`. lz4 computes `iend - MFLIMIT`, which underflows for such tiny
///    addresses and sends the C compressor off scanning wild memory. Real
///    callers always pass a valid pointer, so give it one.
///  * lz4 reads in 8/16-byte units and may touch a few bytes past the logical
///    end of the input; zero padding keeps that deterministic across both
///    libraries.
fn pad(mut v: Vec<u8>) -> Vec<u8> {
    let n = v.len();
    v.resize(n + 64, 0);
    v.truncate(n);
    v
}

/// Random bytes (incompressible).
pub fn gen_random(len: usize, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    pad((0..len).map(|_| (r.next_u32() & 0xFF) as u8).collect())
}

/// Highly compressible: long runs.
pub fn gen_runs(len: usize, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    let mut v = Vec::with_capacity(len + 64);
    while v.len() < len {
        let b = (r.next_u32() & 0x0F) as u8 + b'a';
        let n = (r.below(40) + 1) as usize;
        for _ in 0..n {
            if v.len() == len {
                break;
            }
            v.push(b);
        }
    }
    pad(v)
}

/// Text-like with repeated phrases -> exercises matches at many distances.
pub fn gen_textish(len: usize, seed: u64) -> Vec<u8> {
    const WORDS: [&str; 16] = [
        "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ", "lz4 ",
        "compression ", "algorithm ", "data ", "stream ", "block ", "match ", "literal ",
    ];
    let mut r = Rng::new(seed);
    let mut v = Vec::with_capacity(len + 80);
    while v.len() < len {
        v.extend_from_slice(WORDS[(r.below(16)) as usize].as_bytes());
    }
    v.truncate(len);
    pad(v)
}

/// Small alphabet noise -> moderate compressibility, many short matches.
pub fn gen_lowentropy(len: usize, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    pad((0..len).map(|_| (r.next_u32() % 3) as u8).collect())
}

/// A mixture: compressible prefix then random then compressible.
pub fn gen_mixed(len: usize, seed: u64) -> Vec<u8> {
    let a = len / 3;
    let b = len / 3;
    let c = len - a - b;
    let mut v = gen_textish(a, seed);
    v.extend_from_slice(&gen_random(b, seed ^ 0xABCD));
    v.extend_from_slice(&gen_runs(c, seed ^ 0x1234));
    pad(v)
}

pub type Gen = fn(usize, u64) -> Vec<u8>;

pub const GENS: [(&str, Gen); 5] = [
    ("random", gen_random),
    ("runs", gen_runs),
    ("textish", gen_textish),
    ("lowentropy", gen_lowentropy),
    ("mixed", gen_mixed),
];

/// Interesting input sizes, kept modest so the suite stays fast.
pub const SIZES: [usize; 22] = [
    0, 1, 2, 3, 4, 5, 7, 8, 12, 13, 15, 16, 17, 31, 64, 100, 255, 1000, 4096, 9000, 65536, 100000,
];

/// Aligned scratch buffer usable as an opaque C state.
pub struct Aligned {
    buf: Vec<u64>,
    len: usize,
}

impl Aligned {
    pub fn new(bytes: usize) -> Self {
        Aligned {
            buf: vec![0u64; (bytes + 7) / 8],
            len: bytes,
        }
    }
    pub fn ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr() as *mut u8
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.len) }
    }
    pub fn zero(&mut self) {
        for x in self.buf.iter_mut() {
            *x = 0;
        }
    }
}

pub fn hex(b: &[u8]) -> String {
    let n = b.len().min(64);
    let mut s: String = b[..n].iter().map(|x| format!("{:02x}", x)).collect();
    if b.len() > n {
        s.push_str("...");
    }
    s
}

/// Compact byte-slice comparison: reports the first differing offset instead of
/// dumping the whole buffer into the failure message.
pub fn cmp_bytes(a: &[u8], b: &[u8], ctx: &str) {
    assert_eq!(a.len(), b.len(), "{}: length mismatch", ctx);
    if let Some(i) = (0..a.len()).find(|&i| a[i] != b[i]) {
        panic!(
            "{}: first difference at offset {} of {} (c=0x{:02x}, rust=0x{:02x}); \
             c[{}..]={} rust[{}..]={}",
            ctx,
            i,
            a.len(),
            a[i],
            b[i],
            i,
            hex(&a[i..(i + 16).min(a.len())]),
            i,
            hex(&b[i..(i + 16).min(b.len())])
        );
    }
}
