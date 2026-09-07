//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls the exported
//! `normalize` symbol on each. The Rust implementation is NEVER called
//! directly -- always through its `.so` export, exactly like an external C
//! consumer, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::alloc::{alloc, dealloc, Layout};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    // Keep the libraries alive for the whole process.
    _c: Library,
    _rust: Library,
    pub c: NormalizeFn,
    pub rust: NormalizeFn,
}

// Safety: the loaded function is a pure, thread-compatible leaf function
// operating only on caller-supplied buffers.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("manifest has a parent").to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with: \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    let target = repo_root().join("translation").join("target");
    // Prefer the release artifact (the real shipped library), fall back to debug.
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libnormalize_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libnormalize_lib.so not found under {}. Build it with: cargo build --release",
        target.display()
    )
}

fn load(path: &Path) -> (Library, NormalizeFn) {
    unsafe {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let f: Symbol<NormalizeFn> = lib
            .get(b"normalize\0")
            .unwrap_or_else(|e| panic!("symbol `normalize` missing from {}: {e}", path.display()));
        let raw = *f;
        (lib, raw)
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let (lc, c) = load(&find_c_so());
        let (lr, rust) = load(&find_rust_so());
        Libs { _c: lc, _rust: lr, c, rust }
    })
}

// ---------------------------------------------------------------------------
// 64-byte aligned scratch buffer
// ---------------------------------------------------------------------------

pub struct AlignedBuf {
    ptr: *mut u8,
    len: usize,
    layout: Layout,
}

impl AlignedBuf {
    pub fn new(len_bytes: usize) -> AlignedBuf {
        let len = len_bytes.max(1);
        let layout = Layout::from_size_align(len, 64).unwrap();
        let ptr = unsafe { alloc(layout) };
        assert!(!ptr.is_null(), "allocation failed");
        unsafe { std::ptr::write_bytes(ptr, 0, len) };
        AlignedBuf { ptr, len: len_bytes, layout }
    }

    pub fn from_words(words: &[u32]) -> AlignedBuf {
        let b = AlignedBuf::new(words.len() * 4);
        unsafe {
            std::ptr::copy_nonoverlapping(words.as_ptr() as *const u8, b.ptr, words.len() * 4);
        }
        b
    }

    pub fn word_ptr(&self, word_index: usize) -> *mut f32 {
        assert!(word_index * 4 <= self.len);
        unsafe { self.ptr.add(word_index * 4) as *mut f32 }
    }

    pub fn words(&self) -> Vec<u32> {
        let n = self.len / 4;
        let mut out = vec![0u32; n];
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr, out.as_mut_ptr() as *mut u8, n * 4);
        }
        out
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64)
// ---------------------------------------------------------------------------

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0, n).
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    pub fn sign(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 }
    }
}

// ---------------------------------------------------------------------------
// Value-class generators (axis D of CONFIGS.md)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ValueClass {
    /// Random normals in [-1, 1).
    UnitNormal,
    /// Random values spanning 2^-60 .. 2^60 with random signs.
    WideExponent,
    /// Exact powers of two (sum is exactly representable).
    PowerOfTwo,
    /// Magnitudes near FLT_MAX so the sum of squares overflows to +inf.
    HugeOverflow,
    /// Denormal / near-FLT_MIN magnitudes.
    Denormal,
    /// All +0.0.
    PositiveZero,
    /// All -0.0.
    NegativeZero,
    /// Random mix of +0.0 and -0.0.
    MixedZero,
    /// Random normals with NaNs (quiet + signalling payloads) sprinkled in.
    WithNan,
    /// Random normals with +-inf sprinkled in.
    WithInf,
    /// Random mix of every class above, element by element.
    Mixed,
    /// Completely random 32-bit patterns reinterpreted as f32.
    RandomBits,
}

pub const ALL_CLASSES: &[ValueClass] = &[
    ValueClass::UnitNormal,
    ValueClass::WideExponent,
    ValueClass::PowerOfTwo,
    ValueClass::HugeOverflow,
    ValueClass::Denormal,
    ValueClass::PositiveZero,
    ValueClass::NegativeZero,
    ValueClass::MixedZero,
    ValueClass::WithNan,
    ValueClass::WithInf,
    ValueClass::Mixed,
    ValueClass::RandomBits,
];

fn one_value(class: ValueClass, rng: &mut Rng) -> u32 {
    let f: f32 = match class {
        ValueClass::UnitNormal => rng.sign() * rng.unit(),
        ValueClass::WideExponent => {
            let e = rng.below(121) as i32 - 60; // -60 .. 60
            rng.sign() * rng.unit().max(f32::MIN_POSITIVE) * 2f32.powi(e)
        }
        ValueClass::PowerOfTwo => {
            let e = rng.below(41) as i32 - 20;
            rng.sign() * 2f32.powi(e)
        }
        ValueClass::HugeOverflow => {
            // 1e19..3.4e38 -- squaring any of these overflows f32.
            let e = rng.below(64) as i32 + 64; // 2^64 .. 2^127
            rng.sign() * (1.0 + rng.unit()) * 2f32.powi(e.min(127))
        }
        ValueClass::Denormal => {
            if rng.next_u32() & 1 == 0 {
                // true subnormal
                let bits = 1 + rng.below(0x0080_0000 - 1);
                return bits | ((rng.next_u32() & 1) << 31);
            } else {
                let e = rng.below(20) as i32; // 2^-126 .. 2^-107
                rng.sign() * f32::MIN_POSITIVE * 2f32.powi(e)
            }
        }
        ValueClass::PositiveZero => 0.0,
        ValueClass::NegativeZero => -0.0,
        ValueClass::MixedZero => {
            if rng.next_u32() & 1 == 0 { 0.0 } else { -0.0 }
        }
        ValueClass::WithNan => {
            if rng.below(4) == 0 {
                // quiet or signalling NaN with a random payload
                let payload = 1 + rng.below(0x003F_FFFF);
                let quiet = (rng.next_u32() & 1) << 22;
                let sign = (rng.next_u32() & 1) << 31;
                return sign | 0x7F80_0000 | quiet | payload;
            }
            rng.sign() * rng.unit()
        }
        ValueClass::WithInf => {
            if rng.below(4) == 0 {
                let sign = (rng.next_u32() & 1) << 31;
                return sign | 0x7F80_0000;
            }
            rng.sign() * rng.unit()
        }
        ValueClass::Mixed => {
            let sub = [
                ValueClass::UnitNormal,
                ValueClass::WideExponent,
                ValueClass::PowerOfTwo,
                ValueClass::HugeOverflow,
                ValueClass::Denormal,
                ValueClass::MixedZero,
                ValueClass::WithNan,
                ValueClass::WithInf,
            ];
            let pick = sub[rng.below(sub.len() as u32) as usize];
            return one_value(pick, rng);
        }
        ValueClass::RandomBits => return rng.next_u32(),
    };
    f.to_bits()
}

pub fn gen_values(class: ValueClass, n: usize, rng: &mut Rng) -> Vec<u32> {
    (0..n).map(|_| one_value(class, rng)).collect()
}

// ---------------------------------------------------------------------------
// Buffer placement (axis B of CONFIGS.md)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum Placement {
    /// dest and src are disjoint windows (`dest != src`).
    Separate,
    /// `dest == src` exactly -- the pointer-equality guard is hit.
    Aliased,
    /// `dest == src + k`, k >= 1 (overlapping, but `dest != src`).
    ForwardOverlap(usize),
    /// `src == dest + k`, k >= 1.
    BackwardOverlap(usize),
}

pub const GUARD_WORDS: usize = 8;

/// A concrete memory layout: one aligned buffer with a dest window and a src
/// window at given word offsets, plus poison guard words everywhere else.
pub struct Layout2 {
    pub words: Vec<u32>,
    pub dest_word: usize,
    pub src_word: usize,
    pub size: i32,
}

fn poison(idx: usize) -> u32 {
    // Recognisable, and not a value the code would ever legitimately produce.
    0xDEAD_0000u32 | ((idx as u32).wrapping_mul(2654435761) & 0xFFFF)
}

/// Build a layout. `extra_shift` shifts both windows by that many words to
/// break 16/32-byte alignment (axis: misaligned buffers).
pub fn build_layout(
    placement: Placement,
    size: i32,
    src_values: &[u32],
    extra_shift: usize,
) -> Layout2 {
    let n = size.max(0) as usize;
    assert_eq!(src_values.len(), n);
    let g = GUARD_WORDS;
    let base = g + extra_shift;

    let (dest_word, src_word, total) = match placement {
        Placement::Separate => {
            let src = base;
            let dest = base + n + g;
            (dest, src, dest + n + g)
        }
        Placement::Aliased => {
            let s = base;
            (s, s, s + n + g)
        }
        Placement::ForwardOverlap(k) => {
            let src = base;
            let dest = base + k;
            (dest, src, dest.max(src) + n + g)
        }
        Placement::BackwardOverlap(k) => {
            let dest = base;
            let src = base + k;
            (dest, src, dest.max(src) + n + g)
        }
    };

    let mut words: Vec<u32> = (0..total).map(poison).collect();
    words[src_word..src_word + n].copy_from_slice(src_values);

    Layout2 { words, dest_word, src_word, size }
}

// ---------------------------------------------------------------------------
// The differential call
// ---------------------------------------------------------------------------

/// Run the layout through both `.so`s on independent copies of the memory and
/// return `(c_result_words, rust_result_words)` -- the ENTIRE buffer, so writes
/// outside the `dest` window are caught too.
pub fn run_both(l: &Layout2) -> (Vec<u32>, Vec<u32>) {
    let f = libs();

    let cb = AlignedBuf::from_words(&l.words);
    unsafe { (f.c)(cb.word_ptr(l.dest_word), cb.word_ptr(l.src_word) as *const f32, l.size) };
    let cout = cb.words();

    let rb = AlignedBuf::from_words(&l.words);
    unsafe { (f.rust)(rb.word_ptr(l.dest_word), rb.word_ptr(l.src_word) as *const f32, l.size) };
    let rout = rb.words();

    (cout, rout)
}

fn fmt_word(w: u32) -> String {
    let f = f32::from_bits(w);
    format!("0x{w:08X}({f:e})")
}

/// Assert byte-identical results, with a precise diff report on failure.
pub fn assert_same(ctx: &str, l: &Layout2, cout: &[u32], rout: &[u32]) {
    if cout == rout {
        return;
    }
    let mut diffs = String::new();
    let mut n = 0;
    for i in 0..cout.len() {
        if cout[i] != rout[i] {
            n += 1;
            if n <= 12 {
                let where_ = if l.size > 0
                    && i >= l.dest_word
                    && i < l.dest_word + l.size as usize
                {
                    "dest"
                } else if l.size > 0 && i >= l.src_word && i < l.src_word + l.size as usize {
                    "src"
                } else {
                    "GUARD(out-of-window write!)"
                };
                diffs.push_str(&format!(
                    "\n  word[{i}] {where_}: C={} RUST={}",
                    fmt_word(cout[i]),
                    fmt_word(rout[i])
                ));
            }
        }
    }
    panic!(
        "DIVERGENCE {ctx}\n  size={} dest_word={} src_word={} buf_words={}\n  \
         {n} differing word(s):{diffs}\n  src input = {:?}",
        l.size,
        l.dest_word,
        l.src_word,
        l.words.len(),
        l.words
            .iter()
            .skip(l.src_word)
            .take((l.size.max(0) as usize).min(24))
            .map(|w| fmt_word(*w))
            .collect::<Vec<_>>(),
    );
}

/// Full convenience wrapper: build, run, compare.
pub fn check(ctx: &str, placement: Placement, size: i32, src_values: &[u32], shift: usize) {
    let l = build_layout(placement, size, src_values, shift);
    let (c, r) = run_both(&l);
    assert_same(ctx, &l, &c, &r);
}

// ---------------------------------------------------------------------------
// Size sweep (axis C of CONFIGS.md)
// ---------------------------------------------------------------------------

pub const SIZES: &[i32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000,
];

/// Trials per (row, size, class) combination.
pub const TRIALS: usize = 64;

pub const SEED: u64 = 0x5EED_1234;

// ---------------------------------------------------------------------------
// Fault parity (for the undefined-behaviour rows of ERRORS.md)
// ---------------------------------------------------------------------------
//
// Some inputs the C accepts without validating are outright UB (an ~18-exabyte
// `memset`, a null dereference). Their only OBSERVABLE behaviour is how the
// process dies. To compare that differentially we re-exec this very test binary
// as a child with `DIFF_FAULT_SIDE` set; the child performs the raw call
// against one `.so` and we compare the two children's wait statuses.

pub const SIDE_ENV: &str = "DIFF_FAULT_SIDE";

/// Which side a fault-mode child should exercise, or `None` in the parent.
pub fn fault_side() -> Option<String> {
    std::env::var(SIDE_ENV).ok()
}

#[derive(Debug, PartialEq, Eq)]
pub struct Death {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

fn spawn_side(test_name: &str, side: &str) -> Death {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe().expect("current_exe");
    let mut child = Command::new(exe)
        .args(["--exact", test_name, "--test-threads=1", "--nocapture"])
        .env(SIDE_ENV, side)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn fault-mode child");

    // Bounded wait so a pathological case can never hang the suite.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(st) => {
                return Death { code: st.code(), signal: st.signal() };
            }
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Death { code: None, signal: Some(-1) }; // sentinel: timed out
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
}

/// Run `test_name` in two children (C side and Rust side) and assert that both
/// terminate in exactly the same way (same exit code, or same fatal signal).
pub fn assert_fault_parity(test_name: &str) {
    let c = spawn_side(test_name, "c");
    let r = spawn_side(test_name, "rust");
    assert_eq!(
        c, r,
        "termination mismatch for `{test_name}`: C died as {c:?} but RUST died as {r:?}"
    );
    assert_ne!(
        c.signal,
        Some(-1),
        "`{test_name}` timed out in both children -- inconclusive"
    );
    eprintln!("[fault-parity] {test_name}: both sides terminated as {c:?}");
}

/// In a fault-mode child: pick the `normalize` under test for this side.
pub fn side_fn(side: &str) -> NormalizeFn {
    let f = libs();
    match side {
        "c" => f.c,
        "rust" => f.rust,
        other => panic!("unknown {SIDE_ENV}={other}"),
    }
}
