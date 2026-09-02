//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function
//! through its exported symbol, exactly as an external C consumer would.
//! The Rust implementation is never called directly, so the `#[no_mangle]`
//! `extern "C"` wrappers and the ABI of every struct-by-value signature are
//! part of what is under test.

#![allow(non_snake_case)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C ABI types (mirrors of the C definitions in c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CnRnd {
    pub state: [u64; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

pub const C2_TYPE_CIRCLE: u32 = 0;
pub const C2_TYPE_AABB: u32 = 1;

// ---------------------------------------------------------------------------
// Function-pointer type aliases
// ---------------------------------------------------------------------------

pub type FnC2V = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnC2Bin = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnC2Clamp = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
pub type FnC2Dot = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnCircleCircle = unsafe extern "C" fn(C2Circle, C2Circle) -> i32;
pub type FnCircleAabb = unsafe extern "C" fn(C2Circle, C2Aabb) -> i32;
pub type FnAabbAabb = unsafe extern "C" fn(C2Aabb, C2Aabb) -> i32;
pub type FnF2 = unsafe extern "C" fn(*const c_void, u32, *const c_void, u32) -> i32;
pub type FnF3 = unsafe extern "C" fn(i32, i32) -> i32;
pub type FnF4 = unsafe extern "C" fn(*mut CnRnd) -> f64;
pub type FnF5 = unsafe extern "C" fn(u32) -> u32;
pub type FnF7 = unsafe extern "C" fn(u32, u32, u32) -> u32;
pub type FnF9 = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
pub type FnF10 = unsafe extern "C" fn(u16) -> f32;
pub type FnTriple = unsafe extern "C" fn(*mut f32, *const f32);

#[rustfmt::skip]
pub type FnAgglom = unsafe extern "C" fn(
    f32, f32, f32, f32, f32, f32, f32,
    i32, i32,
    u64, u64,
    u32,
    u32, u32, u32,
    f32, f32, f32, f32, f32, f32, f32, f32,
    u16,
    f32, f32, f32,
    f32, f32, f32,
    f32, f32, f32,
) -> f64;

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}. Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib built by the same `cargo test` invocation is one level up.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let direct = dir.join("libagglom_lib.so");
    if direct.exists() {
        return direct;
    }
    for profile in ["release", "debug"] {
        let p = workspace_root()
            .join("translation/target")
            .join(profile)
            .join("libagglom_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libagglom_lib.so not found (looked in {} and target/{{release,debug}})",
        dir.display()
    );
}

/// One loaded shared object plus a cached symbol handle table.
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    pub fn get<T>(&self, sym: &str) -> Symbol<'_, T> {
        unsafe {
            self.lib.get(sym.as_bytes()).unwrap_or_else(|e| {
                panic!("{}: symbol `{sym}` not exported: {e}", self.name)
            })
        }
    }

    /// `dlsym` without panicking — used to assert that `static` C helpers are
    /// *not* exported.
    pub fn has(&self, sym: &str) -> bool {
        unsafe { self.lib.get::<*const ()>(sym.as_bytes()).is_ok() }
    }
}

/// The pair under comparison: `c` is ground truth, `r` is the translation.
pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

pub fn load() -> Pair {
    let cp = find_c_so();
    let rp = find_rust_so();
    unsafe {
        Pair {
            c: Impl {
                name: "C",
                lib: Library::new(&cp)
                    .unwrap_or_else(|e| panic!("load {}: {e}", cp.display())),
            },
            r: Impl {
                name: "Rust",
                lib: Library::new(&rp)
                    .unwrap_or_else(|e| panic!("load {}: {e}", rp.display())),
            },
        }
    }
}

/// Process-wide singleton so each test does not re-`dlopen`.
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static LIBS: OnceLock<Pair> = OnceLock::new();
    LIBS.get_or_init(load)
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// f32 comparison that distinguishes `+0.0`/`-0.0` and every NaN payload.
#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{ctx}: f32 mismatch\n  C    = {c:?} (bits 0x{:08x})\n  Rust = {r:?} (bits 0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_f64(ctx: &str, c: f64, r: f64) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{ctx}: f64 mismatch\n  C    = {c:?} (bits 0x{:016x})\n  Rust = {r:?} (bits 0x{:016x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_i32(ctx: &str, c: i32, r: i32) {
    assert_eq!(c, r, "{ctx}: i32 mismatch (C={c}, Rust={r})");
}

#[track_caller]
pub fn eq_u32(ctx: &str, c: u32, r: u32) {
    assert_eq!(c, r, "{ctx}: u32 mismatch (C={c}, Rust={r})");
}

#[track_caller]
pub fn eq_vec2(ctx: &str, c: C2v, r: C2v) {
    eq_f32(&format!("{ctx}.x"), c.x, r.x);
    eq_f32(&format!("{ctx}.y"), c.y, r.y);
}

#[track_caller]
pub fn eq_lmvec2(ctx: &str, c: LmVec2, r: LmVec2) {
    eq_f32(&format!("{ctx}.x"), c.x, r.x);
    eq_f32(&format!("{ctx}.y"), c.y, r.y);
}

/// Bit-exact comparison of an `[f32; N]` buffer (plain `assert_eq!` would
/// compare NaN != NaN and report a false mismatch).
#[track_caller]
pub fn eq_bits<const N: usize>(ctx: &str, c: [f32; N], r: [f32; N]) {
    let cb = c.map(f32::to_bits);
    let rb = r.map(f32::to_bits);
    if cb != rb {
        panic!("{ctx}: buffer mismatch\n  C    = {cb:08x?} ({c:?})\n  Rust = {rb:08x?} ({r:?})");
    }
}

#[track_caller]
pub fn eq_triple(ctx: &str, c: [f32; 3], r: [f32; 3]) {
    for i in 0..3 {
        eq_f32(&format!("{ctx}[{i}]"), c[i], r[i]);
    }
}

#[track_caller]
pub fn eq_rnd(ctx: &str, c: CnRnd, r: CnRnd) {
    assert_eq!(
        c, r,
        "{ctx}: cn_rnd_t state mismatch (C={:016x?}, Rust={:016x?})",
        c.state, r.state
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible across runs
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn seeded() -> Self {
        Rng(SEED)
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
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform over `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A float built from **random bits**: covers NaN (all payloads),
    /// ±inf, subnormals and ±0 in their natural proportions.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "tame" finite float in `[-mag, mag]`.
    pub fn finite_f32(&mut self, mag: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * mag
    }
    /// Finite float in `[lo, hi)`.
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + u * (hi - lo)
    }
    /// Mixed generator: mostly tame values, but regularly emits the awkward
    /// specials so every row also sees them.
    pub fn mixed_f32(&mut self) -> f32 {
        match self.below(16) {
            0 => f32::NAN,
            1 => -f32::NAN,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => 0.0,
            5 => -0.0,
            6 => f32::from_bits(self.next_u32() | 0x7F80_0000), // inf/NaN family
            7 => f32::from_bits(self.next_u32() & 0x807F_FFFF), // zero/subnormal
            8 => self.any_f32(),
            9 => f32::MAX,
            10 => f32::MIN,
            11 => f32::MIN_POSITIVE,
            _ => self.finite_f32(100.0),
        }
    }
}

/// A grab-bag of f32 values every "interesting values" row iterates over.
pub fn special_f32s() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),  // smallest subnormal
        f32::from_bits(0x8000_0001),
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_0000), // canonical qNaN
        f32::from_bits(0xFFC0_0000), // negative qNaN
        f32::from_bits(0x7F80_0001), // sNaN, payload 1
        f32::from_bits(0xFF80_0001), // negative sNaN
        f32::from_bits(0x7FAB_CDEF), // sNaN, distinctive payload
        f32::from_bits(0x7FD5_5555), // qNaN, distinctive payload
        60.0,
        120.0,
        180.0,
        240.0,
        300.0,
        360.0,
        -60.0,
        1e30,
        -1e30,
        1e-30,
        16777216.0,  // 2^24
        2147483648.0, // 2^31
        -2147483648.0,
        3.4e38,
    ]
}
