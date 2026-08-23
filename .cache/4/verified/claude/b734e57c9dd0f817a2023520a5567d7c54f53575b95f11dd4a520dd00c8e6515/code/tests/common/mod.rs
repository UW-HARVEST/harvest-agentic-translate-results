//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` (RTLD_LOCAL, so their
//! identically-named exports do not collide) and every test drives *both*
//! through the FFI boundary only — the Rust crate is never called directly, so
//! the `#[no_mangle]` export wrappers are part of what is under test.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    pub tag: &'static str,
    pub path: PathBuf,
    lib: libloading::Library,
}

impl Lib {
    fn open(tag: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) [{}] failed: {e}", path.display(), tag));
        Lib { tag, path, lib }
    }

    /// Look a symbol up. Panics if the symbol is absent — an absent symbol is a
    /// translation completeness failure, never something to skip over.
    pub fn sym<T>(&self, name: &str) -> libloading::Symbol<'_, T> {
        let mut b = Vec::with_capacity(name.len() + 1);
        b.extend_from_slice(name.as_bytes());
        b.push(0);
        unsafe { self.lib.get::<T>(&b) }
            .unwrap_or_else(|e| panic!("[{}] missing symbol `{name}`: {e}", self.tag))
    }

    pub fn has(&self, name: &str) -> bool {
        let mut b = Vec::with_capacity(name.len() + 1);
        b.extend_from_slice(name.as_bytes());
        b.push(0);
        unsafe { self.lib.get::<*const c_void>(&b) }.is_ok()
    }
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn resolve(env_key: &str, candidates: &[&str], what: &str) -> PathBuf {
    if let Ok(p) = std::env::var(env_key) {
        let p = PathBuf::from(p);
        assert!(p.exists(), "{env_key}={} does not exist", p.display());
        return p;
    }
    let root = manifest_dir();
    for c in candidates {
        let p = root.join(c);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "could not locate the {what} shared object; tried {:?} under {} (override with {env_key})",
        candidates,
        root.display()
    );
}

/// `cargo test --test <name>` does **not** rebuild the `cdylib` (the test target
/// has no dependency on it), so it is entirely possible to test a stale library
/// and see phantom passes or phantom failures. Refuse to run in that case.
fn assert_not_stale(so: &std::path::Path, roots: &[&str], label: &str) {
    let so_time = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let root = manifest_dir();
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut stack: Vec<PathBuf> = roots.iter().map(|r| root.join(r)).collect();
    while let Some(p) = stack.pop() {
        let md = match std::fs::metadata(&p) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if md.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&p) {
                for e in rd.flatten() {
                    stack.push(e.path());
                }
            }
        } else if let Ok(t) = md.modified() {
            if newest.as_ref().map(|(nt, _)| t > *nt).unwrap_or(true) {
                newest = Some((t, p));
            }
        }
    }
    if let Some((t, which)) = newest {
        assert!(
            t <= so_time,
            "STALE {label} LIBRARY: {} is older than {}.\n\
             `cargo test --test <name>` does not rebuild the cdylib — run\n\
             `cargo build --release` (or ./run_difftests.sh) first.",
            so.display(),
            which.display()
        );
    }
}

pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| {
        let cpath = resolve(
            "ZSTD_C_SO",
            &["c_src/build/libzstd.so", "cbuild/libzstd.so"],
            "C",
        );
        let rpath = resolve(
            "ZSTD_RUST_SO",
            &["target/release/libzstd.so", "target/debug/libzstd.so"],
            "Rust",
        );
        assert_not_stale(&rpath, &["src", "Cargo.toml"], "RUST");
        assert_not_stale(&cpath, &["c_src/src"], "C");
        assert_ne!(
            cpath, rpath,
            "the C and Rust shared objects resolved to the same file"
        );
        Pair {
            c: Lib::open("C", cpath),
            r: Lib::open("RUST", rpath),
        }
    })
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// A byte buffer whose `Debug` output stays readable even when it is megabytes
/// long: length, FNV-1a digest and a short prefix.
#[derive(Clone, PartialEq, Eq)]
pub struct Blob(pub Vec<u8>);

impl Blob {
    pub fn new(v: Vec<u8>) -> Blob {
        Blob(v)
    }
}

impl fmt::Debug for Blob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blob{{len={},fnv=0x{:016x}", self.0.len(), fnv1a64(&self.0))?;
        let n = self.0.len().min(24);
        write!(f, ",head=")?;
        for b in &self.0[..n] {
            write!(f, "{b:02x}")?;
        }
        if n < self.0.len() {
            write!(f, "..")?;
        }
        write!(f, "}}")
    }
}

pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

/// Report the first differing byte between two buffers, for actionable failures.
pub fn first_diff(a: &[u8], b: &[u8]) -> Option<String> {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            let lo = i.saturating_sub(8);
            let hi = (i + 8).min(n);
            return Some(format!(
                "first differing byte at index {i}: C=0x{:02x} RUST=0x{:02x}\n  C   [{lo}..{hi}] = {:02x?}\n  RUST[{lo}..{hi}] = {:02x?}",
                a[i], b[i], &a[lo..hi], &b[lo..hi]
            ));
        }
    }
    if a.len() != b.len() {
        return Some(format!(
            "buffers share a common prefix of {n} bytes but differ in length: C={} RUST={}",
            a.len(),
            b.len()
        ));
    }
    None
}

/// Run `f` against the C library and the Rust library and require identical
/// results. Returns the (identical) value so callers can chain on it.
/// Set `ZSTD_DIFF_TRACE=1` to print each case label before it runs — the only
/// way to localise a *crash* (as opposed to a mismatch), since a SIGSEGV/SIGFPE
/// inside either `.so` leaves no Rust backtrace.
fn trace(label: &str, which: &str) {
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("ZSTD_DIFF_TRACE").is_some()) {
        eprintln!("[trace {which}] {label}");
    }
}

#[track_caller]
pub fn diff<T, F>(label: &str, f: F) -> T
where
    T: PartialEq + fmt::Debug,
    F: Fn(&Lib) -> T,
{
    let p = pair();
    trace(label, "C");
    let a = f(&p.c);
    trace(label, "RUST");
    let b = f(&p.r);
    if a != b {
        panic!("DIVERGENCE [{label}]\n  C    = {a:?}\n  RUST = {b:?}");
    }
    a
}

/// Same as [`diff`] but additionally pinpoints the first differing byte of the
/// two buffers named by `extract`.
#[track_caller]
pub fn diff_bytes<T, F>(label: &str, f: F) -> T
where
    T: PartialEq + fmt::Debug + AsBlob,
    F: Fn(&Lib) -> T,
{
    let p = pair();
    trace(label, "C");
    let a = f(&p.c);
    trace(label, "RUST");
    let b = f(&p.r);
    if a != b {
        let extra = match (a.as_blob(), b.as_blob()) {
            (Some(x), Some(y)) => first_diff(x, y).unwrap_or_default(),
            _ => String::new(),
        };
        panic!("DIVERGENCE [{label}]\n  C    = {a:?}\n  RUST = {b:?}\n{extra}");
    }
    a
}

pub trait AsBlob {
    fn as_blob(&self) -> Option<&[u8]>;
}
impl AsBlob for Blob {
    fn as_blob(&self) -> Option<&[u8]> {
        Some(&self.0)
    }
}
impl<A: fmt::Debug> AsBlob for (A, Blob) {
    fn as_blob(&self) -> Option<&[u8]> {
        Some(&self.1 .0)
    }
}
impl<A: fmt::Debug, B: fmt::Debug> AsBlob for (A, B, Blob) {
    fn as_blob(&self) -> Option<&[u8]> {
        Some(&self.2 .0)
    }
}

/// Tuples of arity 4..=8 whose LAST element is the interesting buffer. This is
/// the common shape when a test reports several statuses plus one output blob.
macro_rules! as_blob_last {
    ($($idx:tt : $($g:ident),+);* $(;)?) => {
        $(
            impl<$($g: fmt::Debug),+> AsBlob for ($($g,)+ Blob) {
                fn as_blob(&self) -> Option<&[u8]> {
                    Some(&self.$idx.0)
                }
            }
        )*
    };
}
as_blob_last! {
    3: A, B, C;
    4: A, B, C, D;
    5: A, B, C, D, E;
    6: A, B, C, D, E, F;
    7: A, B, C, D, E, F, G;
}

/// A plain `Vec<Blob>` result: report the first blob that differs.
impl AsBlob for Vec<Blob> {
    fn as_blob(&self) -> Option<&[u8]> {
        self.first().map(|b| b.0.as_slice())
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seeds, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        let mut i = 0;
        while i < out.len() {
            let v = self.next_u64().to_le_bytes();
            let n = (out.len() - i).min(8);
            out[i..i + n].copy_from_slice(&v[..n]);
            i += n;
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
}

// ---------------------------------------------------------------------------
// Corpora — the input *shapes* the compressor branches on
// ---------------------------------------------------------------------------

/// Distinct data shapes. Each stresses a different part of the encoder:
/// entropy-coding corner cases, match-finder behaviour, literal handling.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Corpus {
    /// All zero bytes — maximal RLE / single-symbol alphabet.
    Zeros,
    /// One repeated non-zero byte — RLE block, `set_rle` literals.
    OneByte,
    /// Cryptographically-flat pseudo-random — incompressible, forces raw blocks.
    Random,
    /// Small alphabet (4 symbols) — tiny Huffman table, high match density.
    SmallAlphabet,
    /// English-like text built from a word list — realistic literal histogram.
    Text,
    /// Long-range duplicated regions — exercises LDM and the window logic.
    LongRepeats,
    /// Alternating runs of random and constant — many block-boundary decisions.
    Mixed,
    /// Strictly increasing bytes mod 256 — perfectly predictable, no repeats
    /// within the minimum match length.
    Counter,
    /// Two interleaved periodic patterns — repcode-friendly.
    Periodic,
    /// Sparse: mostly zeros with rare random bytes — huge literal runs.
    Sparse,
}

pub const ALL_CORPORA: &[Corpus] = &[
    Corpus::Zeros,
    Corpus::OneByte,
    Corpus::Random,
    Corpus::SmallAlphabet,
    Corpus::Text,
    Corpus::LongRepeats,
    Corpus::Mixed,
    Corpus::Counter,
    Corpus::Periodic,
    Corpus::Sparse,
];

const WORDS: &[&str] = &[
    "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog", "zstandard", "compression",
    "dictionary", "entropy", "huffman", "finite", "state", "sequence", "literal", "match",
    "offset", "window", "block", "frame", "stream", "buffer", "context", "parameter",
];

pub fn corpus(kind: Corpus, len: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed ^ (kind as u64) << 32);
    let mut out = Vec::with_capacity(len);
    match kind {
        Corpus::Zeros => out.resize(len, 0),
        Corpus::OneByte => out.resize(len, 0xA5),
        Corpus::Random => {
            out.resize(len, 0);
            rng.fill(&mut out);
        }
        Corpus::SmallAlphabet => {
            let alpha = [b'a', b'b', b'c', b'd'];
            while out.len() < len {
                out.push(alpha[rng.below(4)]);
            }
        }
        Corpus::Text => {
            while out.len() < len {
                out.extend_from_slice(rng.pick(WORDS).as_bytes());
                out.push(if rng.below(8) == 0 { b'\n' } else { b' ' });
            }
            out.truncate(len);
        }
        Corpus::LongRepeats => {
            // Build a base chunk then re-emit earlier regions verbatim so the
            // matches span far more than the default window at small windowLog.
            let base: Vec<u8> = {
                let mut b = Vec::new();
                while b.len() < 4096 {
                    b.extend_from_slice(rng.pick(WORDS).as_bytes());
                    b.push(b' ');
                }
                b
            };
            while out.len() < len {
                if !out.is_empty() && rng.below(3) != 0 {
                    let start = rng.below(out.len());
                    let n = (rng.below(8192) + 1).min(out.len() - start);
                    let slice = out[start..start + n].to_vec();
                    out.extend_from_slice(&slice);
                } else {
                    out.extend_from_slice(&base);
                }
            }
            out.truncate(len);
        }
        Corpus::Mixed => {
            while out.len() < len {
                let n = rng.below(300) + 1;
                if rng.bool() {
                    for _ in 0..n {
                        out.push(rng.u8());
                    }
                } else {
                    let b = rng.u8();
                    for _ in 0..n {
                        out.push(b);
                    }
                }
            }
            out.truncate(len);
        }
        Corpus::Counter => {
            for i in 0..len {
                out.push((i & 0xFF) as u8);
            }
        }
        Corpus::Periodic => {
            let p1: Vec<u8> = (0..37u16).map(|i| (i * 7 + 3) as u8).collect();
            let p2: Vec<u8> = (0..11u16).map(|i| (i * 31 + 1) as u8).collect();
            let mut i = 0usize;
            while out.len() < len {
                if (i / 64) % 2 == 0 {
                    out.push(p1[i % p1.len()]);
                } else {
                    out.push(p2[i % p2.len()]);
                }
                i += 1;
            }
        }
        Corpus::Sparse => {
            out.resize(len, 0);
            let n = len / 64 + 1;
            for _ in 0..n {
                if len > 0 {
                    let i = rng.below(len);
                    out[i] = rng.u8() | 1;
                }
            }
        }
    }
    debug_assert_eq!(out.len(), len);
    out
}

/// Sizes that straddle every interesting boundary: empty, sub-minmatch, the
/// 1/2/4-byte frame-content-size encodings, block size (128 KB), multi-block.
pub const SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 63, 64, 100, 127, 128, 255, 256, 257, 1000, 1024, 4096,
    16384, 65535, 65536, 131071, 131072, 131073, 200000, 300000,
];

// ---------------------------------------------------------------------------
// C types mirrored from the public headers
// ---------------------------------------------------------------------------

pub type SizeT = usize;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_bounds {
    pub error: SizeT,
    pub lowerBound: c_int,
    pub upperBound: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ZSTD_inBuffer {
    pub src: *const c_void,
    pub size: SizeT,
    pub pos: SizeT,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ZSTD_outBuffer {
    pub dst: *mut c_void,
    pub size: SizeT,
    pub pos: SizeT,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_Sequence {
    pub offset: c_uint,
    pub litLength: c_uint,
    pub matchLength: c_uint,
    pub rep: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_compressionParameters {
    pub windowLog: c_uint,
    pub chainLog: c_uint,
    pub hashLog: c_uint,
    pub searchLog: c_uint,
    pub minMatch: c_uint,
    pub targetLength: c_uint,
    pub strategy: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_frameParameters {
    pub contentSizeFlag: c_int,
    pub checksumFlag: c_int,
    pub noDictIDFlag: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_parameters {
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_FrameHeader {
    pub frameContentSize: c_ulonglong,
    pub windowSize: c_ulonglong,
    pub blockSizeMax: c_uint,
    pub frameType: c_int,
    pub headerSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
    pub _reserved1: c_uint,
    pub _reserved2: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_frameProgression {
    pub ingested: c_ulonglong,
    pub consumed: c_ulonglong,
    pub produced: c_ulonglong,
    pub flushed: c_ulonglong,
    pub currentJobID: c_uint,
    pub nbActiveWorkers: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZDICT_params_t {
    pub compressionLevel: c_int,
    pub notificationLevel: c_uint,
    pub dictID: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct ZDICT_cover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct ZDICT_fastCover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub f: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub accel: c_uint,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct ZDICT_legacy_params_t {
    pub selectivityLevel: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_customMem {
    pub customAlloc: Option<extern "C" fn(*mut c_void, SizeT) -> *mut c_void>,
    pub customFree: Option<extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

impl Default for ZSTD_customMem {
    fn default() -> Self {
        ZSTD_customMem {
            customAlloc: None,
            customFree: None,
            opaque: std::ptr::null_mut(),
        }
    }
}

// ---------------------------------------------------------------------------
// Constants from the public headers
// ---------------------------------------------------------------------------

pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = u64::MAX - 0;
pub const ZSTD_CONTENTSIZE_ERROR: u64 = u64::MAX - 1;

pub const ZSTD_MAGICNUMBER: u32 = 0xFD2F_B528;
pub const ZSTD_MAGIC_DICTIONARY: u32 = 0xEC30_A437;
pub const ZSTD_MAGIC_SKIPPABLE_START: u32 = 0x184D_2A50;
pub const ZSTD_MAGIC_SKIPPABLE_MASK: u32 = 0xFFFF_FFF0;

pub const ZSTD_BLOCKSIZE_MAX: usize = 1 << 17;

// ZSTD_cParameter
pub const ZSTD_c_compressionLevel: c_int = 100;
pub const ZSTD_c_windowLog: c_int = 101;
pub const ZSTD_c_hashLog: c_int = 102;
pub const ZSTD_c_chainLog: c_int = 103;
pub const ZSTD_c_searchLog: c_int = 104;
pub const ZSTD_c_minMatch: c_int = 105;
pub const ZSTD_c_targetLength: c_int = 106;
pub const ZSTD_c_strategy: c_int = 107;
pub const ZSTD_c_targetCBlockSize: c_int = 130;
pub const ZSTD_c_enableLongDistanceMatching: c_int = 160;
pub const ZSTD_c_ldmHashLog: c_int = 161;
pub const ZSTD_c_ldmMinMatch: c_int = 162;
pub const ZSTD_c_ldmBucketSizeLog: c_int = 163;
pub const ZSTD_c_ldmHashRateLog: c_int = 164;
pub const ZSTD_c_contentSizeFlag: c_int = 200;
pub const ZSTD_c_checksumFlag: c_int = 201;
pub const ZSTD_c_dictIDFlag: c_int = 202;
pub const ZSTD_c_nbWorkers: c_int = 400;
pub const ZSTD_c_jobSize: c_int = 401;
pub const ZSTD_c_overlapLog: c_int = 402;
// experimental (documented aliases)
pub const ZSTD_c_rsyncable: c_int = 500;
pub const ZSTD_c_format: c_int = 10;
pub const ZSTD_c_forceMaxWindow: c_int = 1000;
pub const ZSTD_c_forceAttachDict: c_int = 1001;
pub const ZSTD_c_literalCompressionMode: c_int = 1002;
pub const ZSTD_c_srcSizeHint: c_int = 1004;
pub const ZSTD_c_enableDedicatedDictSearch: c_int = 1005;
pub const ZSTD_c_stableInBuffer: c_int = 1006;
pub const ZSTD_c_stableOutBuffer: c_int = 1007;
pub const ZSTD_c_blockDelimiters: c_int = 1008;
pub const ZSTD_c_validateSequences: c_int = 1009;
pub const ZSTD_c_splitAfterSequences: c_int = 1010;
pub const ZSTD_c_useRowMatchFinder: c_int = 1011;
pub const ZSTD_c_deterministicRefPrefix: c_int = 1012;
pub const ZSTD_c_prefetchCDictTables: c_int = 1013;
pub const ZSTD_c_enableSeqProducerFallback: c_int = 1014;
pub const ZSTD_c_maxBlockSize: c_int = 1015;
pub const ZSTD_c_repcodeResolution: c_int = 1016;
pub const ZSTD_c_blockSplitterLevel: c_int = 1017;

/// Every documented `ZSTD_cParameter` value, in header order.
pub const ALL_CPARAMS: &[(&str, c_int)] = &[
    ("compressionLevel", ZSTD_c_compressionLevel),
    ("windowLog", ZSTD_c_windowLog),
    ("hashLog", ZSTD_c_hashLog),
    ("chainLog", ZSTD_c_chainLog),
    ("searchLog", ZSTD_c_searchLog),
    ("minMatch", ZSTD_c_minMatch),
    ("targetLength", ZSTD_c_targetLength),
    ("strategy", ZSTD_c_strategy),
    ("targetCBlockSize", ZSTD_c_targetCBlockSize),
    ("enableLongDistanceMatching", ZSTD_c_enableLongDistanceMatching),
    ("ldmHashLog", ZSTD_c_ldmHashLog),
    ("ldmMinMatch", ZSTD_c_ldmMinMatch),
    ("ldmBucketSizeLog", ZSTD_c_ldmBucketSizeLog),
    ("ldmHashRateLog", ZSTD_c_ldmHashRateLog),
    ("contentSizeFlag", ZSTD_c_contentSizeFlag),
    ("checksumFlag", ZSTD_c_checksumFlag),
    ("dictIDFlag", ZSTD_c_dictIDFlag),
    ("nbWorkers", ZSTD_c_nbWorkers),
    ("jobSize", ZSTD_c_jobSize),
    ("overlapLog", ZSTD_c_overlapLog),
    ("rsyncable", ZSTD_c_rsyncable),
    ("format", ZSTD_c_format),
    ("forceMaxWindow", ZSTD_c_forceMaxWindow),
    ("forceAttachDict", ZSTD_c_forceAttachDict),
    ("literalCompressionMode", ZSTD_c_literalCompressionMode),
    ("srcSizeHint", ZSTD_c_srcSizeHint),
    ("enableDedicatedDictSearch", ZSTD_c_enableDedicatedDictSearch),
    ("stableInBuffer", ZSTD_c_stableInBuffer),
    ("stableOutBuffer", ZSTD_c_stableOutBuffer),
    ("blockDelimiters", ZSTD_c_blockDelimiters),
    ("validateSequences", ZSTD_c_validateSequences),
    ("splitAfterSequences", ZSTD_c_splitAfterSequences),
    ("useRowMatchFinder", ZSTD_c_useRowMatchFinder),
    ("deterministicRefPrefix", ZSTD_c_deterministicRefPrefix),
    ("prefetchCDictTables", ZSTD_c_prefetchCDictTables),
    ("enableSeqProducerFallback", ZSTD_c_enableSeqProducerFallback),
    ("maxBlockSize", ZSTD_c_maxBlockSize),
    ("repcodeResolution", ZSTD_c_repcodeResolution),
    ("blockSplitterLevel", ZSTD_c_blockSplitterLevel),
];

// ZSTD_dParameter
pub const ZSTD_d_windowLogMax: c_int = 100;
pub const ZSTD_d_format: c_int = 1000;
pub const ZSTD_d_stableOutBuffer: c_int = 1001;
pub const ZSTD_d_forceIgnoreChecksum: c_int = 1002;
pub const ZSTD_d_refMultipleDDicts: c_int = 1003;
pub const ZSTD_d_disableHuffmanAssembly: c_int = 1004;
pub const ZSTD_d_maxBlockSize: c_int = 1005;

pub const ALL_DPARAMS: &[(&str, c_int)] = &[
    ("windowLogMax", ZSTD_d_windowLogMax),
    ("format", ZSTD_d_format),
    ("stableOutBuffer", ZSTD_d_stableOutBuffer),
    ("forceIgnoreChecksum", ZSTD_d_forceIgnoreChecksum),
    ("refMultipleDDicts", ZSTD_d_refMultipleDDicts),
    ("disableHuffmanAssembly", ZSTD_d_disableHuffmanAssembly),
    ("maxBlockSize", ZSTD_d_maxBlockSize),
];

// ZSTD_ResetDirective
pub const ZSTD_reset_session_only: c_int = 1;
pub const ZSTD_reset_parameters: c_int = 2;
pub const ZSTD_reset_session_and_parameters: c_int = 3;

// ZSTD_EndDirective
pub const ZSTD_e_continue: c_int = 0;
pub const ZSTD_e_flush: c_int = 1;
pub const ZSTD_e_end: c_int = 2;

// ZSTD_strategy
pub const ZSTD_fast: c_int = 1;
pub const ZSTD_dfast: c_int = 2;
pub const ZSTD_greedy: c_int = 3;
pub const ZSTD_lazy: c_int = 4;
pub const ZSTD_lazy2: c_int = 5;
pub const ZSTD_btlazy2: c_int = 6;
pub const ZSTD_btopt: c_int = 7;
pub const ZSTD_btultra: c_int = 8;
pub const ZSTD_btultra2: c_int = 9;
pub const ALL_STRATEGIES: &[c_int] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];

// misc enums
pub const ZSTD_dct_auto: c_int = 0;
pub const ZSTD_dct_rawContent: c_int = 1;
pub const ZSTD_dct_fullDict: c_int = 2;
pub const ZSTD_dlm_byCopy: c_int = 0;
pub const ZSTD_dlm_byRef: c_int = 1;
pub const ZSTD_f_zstd1: c_int = 0;
pub const ZSTD_f_zstd1_magicless: c_int = 1;
pub const ZSTD_ps_auto: c_int = 0;
pub const ZSTD_ps_enable: c_int = 1;
pub const ZSTD_ps_disable: c_int = 2;
pub const ZSTD_lcm_auto: c_int = 0;
pub const ZSTD_lcm_huffman: c_int = 1;
pub const ZSTD_lcm_uncompressed: c_int = 2;
pub const ZSTD_sf_noBlockDelimiters: c_int = 0;
pub const ZSTD_sf_explicitBlockDelimiters: c_int = 1;

// ---------------------------------------------------------------------------
// Thin typed wrappers over the exported C ABI
// ---------------------------------------------------------------------------

pub type FnVersionNumber = unsafe extern "C" fn() -> c_uint;
pub type FnVersionString = unsafe extern "C" fn() -> *const c_char;
pub type FnIsError = unsafe extern "C" fn(SizeT) -> c_uint;
pub type FnGetErrorName = unsafe extern "C" fn(SizeT) -> *const c_char;
pub type FnGetErrorCode = unsafe extern "C" fn(SizeT) -> c_int;
pub type FnGetErrorString = unsafe extern "C" fn(c_int) -> *const c_char;
pub type FnCompressBound = unsafe extern "C" fn(SizeT) -> SizeT;
pub type FnMinCLevel = unsafe extern "C" fn() -> c_int;
pub type FnMaxCLevel = unsafe extern "C" fn() -> c_int;
pub type FnDefaultCLevel = unsafe extern "C" fn() -> c_int;

pub type FnCompress =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, c_int) -> SizeT;
pub type FnDecompress = unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
pub type FnGetFrameContentSize = unsafe extern "C" fn(*const c_void, SizeT) -> c_ulonglong;
pub type FnFindFrameCompressedSize = unsafe extern "C" fn(*const c_void, SizeT) -> SizeT;
pub type FnGetDictIDFromFrame = unsafe extern "C" fn(*const c_void, SizeT) -> c_uint;

pub type FnCreateCCtx = unsafe extern "C" fn() -> *mut c_void;
pub type FnFreeCCtx = unsafe extern "C" fn(*mut c_void) -> SizeT;
pub type FnCreateDCtx = unsafe extern "C" fn() -> *mut c_void;
pub type FnFreeDCtx = unsafe extern "C" fn(*mut c_void) -> SizeT;
pub type FnCCtxSetParameter = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> SizeT;
pub type FnCCtxGetParameter = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> SizeT;
pub type FnDCtxSetParameter = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> SizeT;
pub type FnCParamGetBounds = unsafe extern "C" fn(c_int) -> ZSTD_bounds;
pub type FnDParamGetBounds = unsafe extern "C" fn(c_int) -> ZSTD_bounds;
pub type FnCCtxReset = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;
pub type FnDCtxReset = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;
pub type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
pub type FnCompressCCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT, c_int) -> SizeT;
pub type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
pub type FnCompressStream2 = unsafe extern "C" fn(
    *mut c_void,
    *mut ZSTD_outBuffer,
    *mut ZSTD_inBuffer,
    c_int,
) -> SizeT;
pub type FnDecompressStream =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> SizeT;

/// RAII wrapper so a leaked context cannot mask a divergence between runs.
pub struct Ctx<'l> {
    pub lib: &'l Lib,
    pub ptr: *mut c_void,
    free: &'static str,
}

impl<'l> Ctx<'l> {
    pub fn new(lib: &'l Lib, create: &str, free: &'static str) -> Ctx<'l> {
        let f = lib.sym::<FnCreateCCtx>(create);
        let ptr = unsafe { f() };
        assert!(!ptr.is_null(), "[{}] {create} returned NULL", lib.tag);
        Ctx { lib, ptr, free }
    }
    pub fn cctx(lib: &'l Lib) -> Ctx<'l> {
        Ctx::new(lib, "ZSTD_createCCtx", "ZSTD_freeCCtx")
    }
    pub fn dctx(lib: &'l Lib) -> Ctx<'l> {
        Ctx::new(lib, "ZSTD_createDCtx", "ZSTD_freeDCtx")
    }
    pub fn cstream(lib: &'l Lib) -> Ctx<'l> {
        Ctx::new(lib, "ZSTD_createCStream", "ZSTD_freeCStream")
    }
    pub fn dstream(lib: &'l Lib) -> Ctx<'l> {
        Ctx::new(lib, "ZSTD_createDStream", "ZSTD_freeDStream")
    }
    /// Wrap an already-created raw pointer (e.g. from `*_advanced`).
    pub fn from_raw(lib: &'l Lib, ptr: *mut c_void, free: &'static str) -> Ctx<'l> {
        Ctx { lib, ptr, free }
    }
}

impl Drop for Ctx<'_> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let f = self.lib.sym::<FnFreeCCtx>(self.free);
            unsafe { f(self.ptr) };
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience helpers used by many tests
// ---------------------------------------------------------------------------

pub fn is_error(l: &Lib, code: SizeT) -> bool {
    let f = l.sym::<FnIsError>("ZSTD_isError");
    unsafe { f(code) != 0 }
}

pub fn err_code(l: &Lib, code: SizeT) -> c_int {
    let f = l.sym::<FnGetErrorCode>("ZSTD_getErrorCode");
    unsafe { f(code) }
}

pub fn err_name(l: &Lib, code: SizeT) -> String {
    let f = l.sym::<FnGetErrorName>("ZSTD_getErrorName");
    unsafe { cstr(f(code)) }
}

pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
}

/// The canonical way this suite reports a size_t return: either the exact value
/// or the symbolic error name, so a divergence names the error rather than a
/// raw `-N` bit pattern.
#[derive(Clone, PartialEq, Eq)]
pub enum R {
    Ok(SizeT),
    Err(c_int, String),
}

impl fmt::Debug for R {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            R::Ok(n) => write!(f, "Ok({n})"),
            R::Err(c, s) => write!(f, "Err({c}:{s})"),
        }
    }
}

pub fn res(l: &Lib, code: SizeT) -> R {
    if is_error(l, code) {
        R::Err(err_code(l, code), err_name(l, code))
    } else {
        R::Ok(code)
    }
}

/// One-shot compress with `ZSTD_compress`, returning the status plus the whole
/// destination buffer (so trailing bytes the C leaves untouched are compared).
pub fn compress_simple(l: &Lib, src: &[u8], level: c_int, cap: usize) -> (R, Blob) {
    let f = l.sym::<FnCompress>("ZSTD_compress");
    let mut dst = vec![0xCDu8; cap];
    let n = unsafe {
        f(
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            level,
        )
    };
    let r = res(l, n);
    if let R::Ok(n) = r {
        dst.truncate(n);
    }
    (r, Blob(dst))
}

pub fn decompress_simple(l: &Lib, src: &[u8], cap: usize) -> (R, Blob) {
    let f = l.sym::<FnDecompress>("ZSTD_decompress");
    let mut dst = vec![0xCDu8; cap];
    let n = unsafe {
        f(
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let r = res(l, n);
    if let R::Ok(n) = r {
        dst.truncate(n);
    }
    (r, Blob(dst))
}

pub fn compress_bound(l: &Lib, n: usize) -> usize {
    let f = l.sym::<FnCompressBound>("ZSTD_compressBound");
    unsafe { f(n) }
}

/// Compress with the C library only — handy to build a fixture that both
/// libraries then decompress.
pub fn c_compress(src: &[u8], level: c_int) -> Vec<u8> {
    let l = &pair().c;
    let cap = compress_bound(l, src.len()).max(64);
    let (r, b) = compress_simple(l, src, level, cap);
    match r {
        R::Ok(_) => b.0,
        R::Err(c, s) => panic!("C-side fixture compression failed: {c}:{s}"),
    }
}

// ---------------------------------------------------------------------------
// Row coverage recording (Phase B / Phase C completion gate)
// ---------------------------------------------------------------------------

/// Record that the calling test exercises specific rows of `CONFIGS.md` /
/// `ERRORS.md`. Tags are appended to `target/difftest-coverage/`, and
/// `coverage.py` folds them back into the two tables so the check-boxes are
/// derived from what actually ran rather than from a claim in a comment.
///
/// Tag forms:
///   * `CFG:17`         — CONFIGS.md row 17
///   * `CFG:17-25`      — CONFIGS.md rows 17..=25
///   * `ERR:decompress/zstd_decompress.c:512` — every ERRORS.md row at that site
///   * `ERR:common/entropy_common.c:301,decompress/huf_decompress.c:88` — several
pub fn covers(tags: &[&str]) {
    use std::io::Write;
    let dir = manifest_dir().join("target/difftest-coverage");
    let _ = std::fs::create_dir_all(&dir);
    // Name the file after the TEST BINARY as well as the pid. The pid alone is
    // not enough: `cargo test` runs the binaries one after another, so the OS
    // can hand the same pid to a later binary, and two suites then share a file.
    // Including the executable's own name makes the mapping one-file-per-suite
    // and keeps a recycled pid from hiding anyone's tags.
    let who = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "unknown".to_string());
    let file = dir.join(format!("{who}.{}.txt", std::process::id()));

    // Build the whole block first, then issue exactly ONE `write_all`. All the
    // tests of a binary share this file and run on several `--test-threads`, and
    // `writeln!` on a `File` can emit the payload and the newline as separate
    // `write` syscalls — two threads then interleave into lines like
    // `CFG:100CFG:41`, which `coverage.py` cannot parse, silently UNDER-counting
    // coverage. A single `write_all` to an O_APPEND fd is atomic for a payload
    // this small, so the lines stay whole.
    let mut buf = String::new();
    for t in tags {
        // A tag may name several rows separated by commas. The `CFG:`/`ERR:`
        // prefix appears once, at the front, so it has to be re-applied to every
        // subsequent part — otherwise `coverage.py` drops all but the first.
        let prefix = if t.starts_with("CFG:") {
            "CFG:"
        } else if t.starts_with("ERR:") {
            "ERR:"
        } else {
            ""
        };
        for part in t.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if !(part.starts_with("CFG:") || part.starts_with("ERR:")) {
                buf.push_str(prefix);
            }
            buf.push_str(part);
            buf.push('\n');
        }
    }
    if buf.is_empty() {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
        let _ = f.write_all(buf.as_bytes());
    }
}
