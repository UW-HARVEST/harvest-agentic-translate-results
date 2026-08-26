//! Shared differential-test harness.
//!
//! Both the C reference `.so` and the translated Rust `.so` are loaded with
//! `libloading` and driven **only** through their exported `to_barycentric`
//! symbol. No Rust function is ever called directly, so the `#[no_mangle]`
//! `extern "C"` wrapper and the SysV register-level ABI are under test too.

#![allow(dead_code)]

use libloading::Library;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Mirrors `typedef struct lm_vec2 { float x, y; } lm_vec2;`.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }
    pub fn bits(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

impl std::fmt::Debug for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:e}|{:#010x}, {:e}|{:#010x})",
            self.x,
            self.x.to_bits(),
            self.y,
            self.y.to_bits()
        )
    }
}

pub type ToBarycentric = unsafe extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2;

pub struct Api {
    pub c: ToBarycentric,
    pub rust: ToBarycentric,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

static API: OnceLock<Api> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared library produced by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent* directory name, so the
/// file is normally `libtranslated_rust.so`; scan for any `.so` to stay robust.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locate the translated Rust cdylib.
///
/// `RUST_SO_PATH` overrides; otherwise prefer `release`, then `debug`. Set by
/// `verify_all.sh` so the same suite can be run against both opt levels.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let name = "libto_barycentric_lib.so";
    for profile in ["release", "debug"] {
        let p = manifest_dir().join("target").join(profile).join(name);
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust .so found; run `cargo build --release` (and/or `cargo build`) first");
}

fn load(path: &Path) -> ToBarycentric {
    // SAFETY: loading a trusted, locally built shared object.
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    // SAFETY: the symbol's C signature is `lm_vec2 (*)(lm_vec2, lm_vec2, lm_vec2, lm_vec2)`.
    let f: ToBarycentric = unsafe {
        *lib.get::<ToBarycentric>(b"to_barycentric\0")
            .unwrap_or_else(|e| panic!("dlsym(to_barycentric) in {} failed: {e}", path.display()))
    };
    // Keep the library mapped for the whole process lifetime.
    std::mem::forget(lib);
    f
}

pub fn api() -> &'static Api {
    API.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        Api {
            c: load(&c_path),
            rust: load(&rust_path),
            c_path,
            rust_path,
        }
    })
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Call both libraries and require **bit-identical** results.
///
/// Bit comparison (not `==`) so that `+0.0` vs `-0.0` and differing NaN
/// signs/payloads are treated as failures, which is the whole point here.
#[track_caller]
pub fn diff(case: &str, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) {
    let a = api();
    // SAFETY: both symbols have the declared C signature.
    let c = unsafe { (a.c)(p1, p2, p3, p) };
    // SAFETY: ditto.
    let r = unsafe { (a.rust)(p1, p2, p3, p) };
    if c.bits() != r.bits() {
        panic!(
            "MISMATCH [{case}]\n  \
             inputs : p1={p1:?}\n           p2={p2:?}\n           p3={p3:?}\n           p ={p:?}\n  \
             C      : {c:?}\n  rust   : {r:?}\n  \
             C .so  : {}\n  rust.so: {}",
            a.c_path.display(),
            a.rust_path.display()
        );
    }
}

/// Call only the C reference (used for sanity checks, e.g. "this row really is
/// a finite/NaN case", so a row cannot pass vacuously).
pub fn c_call(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    // SAFETY: declared C signature.
    unsafe { (api().c)(p1, p2, p3, p) }
}

/// Call only the Rust `.so` export.
pub fn rust_call(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    // SAFETY: declared C signature.
    unsafe { (api().rust)(p1, p2, p3, p) }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x2B7E_1516_28AE_D2A6;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn seeded() -> Self {
        Rng::new(SEED)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, n)`.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    #[inline]
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Uniform `f32` in `[lo, hi]`.
    #[inline]
    pub fn uniform(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u32() as f32) / (u32::MAX as f32);
        lo + t * (hi - lo)
    }
    /// Random sign times `2^exp` scaled by a uniform mantissa: log-uniform-ish
    /// coverage of a chosen binade range.
    pub fn binade(&mut self, min_exp: i32, max_exp: i32) -> f32 {
        let span = (max_exp - min_exp + 1) as u32;
        let e = min_exp + self.below(span) as i32;
        let m = self.uniform(1.0, 2.0);
        let v = m * (2.0f32).powi(e);
        if self.bool() { -v } else { v }
    }
    /// Any of the 2^32 bit patterns (NaN, inf, subnormal, ±0 included).
    #[inline]
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A finite value drawn log-uniformly over the whole normal range.
    pub fn finite(&mut self) -> f32 {
        loop {
            let v = self.binade(-126, 127);
            if v.is_finite() {
                return v;
            }
        }
    }
    /// Subnormal: exponent field zero, random non-zero 23-bit mantissa.
    pub fn subnormal(&mut self) -> f32 {
        let m = (self.next_u32() & 0x007F_FFFF).max(1);
        let s = if self.bool() { 0x8000_0000 } else { 0 };
        f32::from_bits(s | m)
    }
    pub fn vec2(&mut self, lo: f32, hi: f32) -> Vec2 {
        Vec2::new(self.uniform(lo, hi), self.uniform(lo, hi))
    }
}

// ---------------------------------------------------------------------------
// Interesting-value pool
// ---------------------------------------------------------------------------

pub const QNAN: f32 = f32::from_bits(0x7FC0_0000);
pub const QNAN_NEG: f32 = f32::from_bits(0xFFC0_0000); // x86 "indefinite"
pub const SNAN: f32 = f32::from_bits(0x7F80_0001);
pub const SNAN_NEG: f32 = f32::from_bits(0xFF80_0001);
pub const NAN_ALL_ONES: f32 = f32::from_bits(0xFFFF_FFFF);
pub const NAN_PAYLOAD_A: f32 = f32::from_bits(0x7FC0_1234);
pub const NAN_PAYLOAD_B: f32 = f32::from_bits(0x7FDE_ADBE);
pub const SUBNORMAL_MIN: f32 = f32::from_bits(0x0000_0001);
pub const SUBNORMAL_MAX: f32 = f32::from_bits(0x007F_FFFF);

/// Every interesting encoding class, used by the pool-driven tests.
pub const POOL: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -2.0,
    3.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    QNAN,
    QNAN_NEG,
    SNAN,
    SNAN_NEG,
    NAN_ALL_ONES,
    NAN_PAYLOAD_A,
    NAN_PAYLOAD_B,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    SUBNORMAL_MIN,
    -SUBNORMAL_MIN,
    SUBNORMAL_MAX,
    1e-30,
    1e30,
    -1e30,
    1e20,
    16777216.0,  // 2^24, first integer with an even neighbour gap
    16777217.0,  // not representable -> rounds
    8388608.0,   // 2^23
];

/// The 8 scalar input slots, in the order `[p1.x, p1.y, p2.x, p2.y, p3.x, p3.y, p.x, p.y]`.
pub const SLOT_NAMES: [&str; 8] = [
    "p1.x", "p1.y", "p2.x", "p2.y", "p3.x", "p3.y", "p.x", "p.y",
];

pub fn from_slots(s: [f32; 8]) -> (Vec2, Vec2, Vec2, Vec2) {
    (
        Vec2::new(s[0], s[1]),
        Vec2::new(s[2], s[3]),
        Vec2::new(s[4], s[5]),
        Vec2::new(s[6], s[7]),
    )
}

/// Randomized, *non-degenerate* finite slot vector in a given magnitude range.
pub fn random_slots(rng: &mut Rng, lo: f32, hi: f32) -> [f32; 8] {
    let mut s = [0.0f32; 8];
    for v in s.iter_mut() {
        *v = rng.uniform(lo, hi);
    }
    s
}

#[track_caller]
pub fn diff_slots(case: &str, s: [f32; 8]) {
    let (p1, p2, p3, p) = from_slots(s);
    diff(case, p1, p2, p3, p);
}

/// Number of randomized iterations for a `CONFIGS.md` / `ERRORS.md` row.
///
/// `DIFF_SCALE` multiplies every row's count (soak runs); `DIFF_ITERS` forces an
/// absolute count. Each row keeps its own default so that rows sitting inside
/// an 8-slot × N-specials loop are not blown up by the same factor as a flat
/// row.
pub fn iters(default: u32) -> u32 {
    if let Some(n) = std::env::var("DIFF_ITERS").ok().and_then(|s| s.parse().ok()) {
        return n;
    }
    let scale: f64 = std::env::var("DIFF_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);
    ((default as f64) * scale).ceil().min(u32::MAX as f64) as u32
}

// ---------------------------------------------------------------------------
// MXCSR control (rows C30/C31): the ambient FP environment both libraries
// inherit from the caller.
// ---------------------------------------------------------------------------

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod mxcsr {
    pub const ROUND_NEAREST: u32 = 0x0000;
    pub const ROUND_DOWN: u32 = 0x2000;
    pub const ROUND_UP: u32 = 0x4000;
    pub const ROUND_ZERO: u32 = 0x6000;
    pub const ROUND_MASK: u32 = 0x6000;
    pub const FTZ: u32 = 0x8000;
    pub const DAZ: u32 = 0x0040;

    pub fn get() -> u32 {
        let mut v: u32 = 0;
        // SAFETY: `stmxcsr` writes 4 bytes to the supplied address.
        unsafe {
            core::arch::asm!(
                "stmxcsr [{p}]",
                p = in(reg) core::ptr::addr_of_mut!(v),
                options(nostack, preserves_flags),
            );
        }
        v
    }

    pub fn set(v: u32) {
        // SAFETY: `ldmxcsr` reads 4 bytes from the supplied address.
        unsafe {
            core::arch::asm!(
                "ldmxcsr [{p}]",
                p = in(reg) core::ptr::addr_of!(v),
                options(nostack, preserves_flags),
            );
        }
    }

    /// Run `f` with extra MXCSR bits applied, always restoring the old value.
    pub fn with(mask: u32, bits: u32, f: impl FnOnce()) {
        let saved = get();
        set((saved & !mask) | bits);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        set(saved);
        if let Err(e) = r {
            std::panic::resume_unwind(e);
        }
    }
}
