//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `cdylib` with `libloading` and exposes
//! matched symbol pairs. Nothing here calls a Rust function directly — every
//! call goes through the dynamic-symbol table, exactly like an external C
//! consumer, so the `#[no_mangle]`/`extern "C"` wrappers are under test too.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI types (mirroring c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;

// ---------------------------------------------------------------------------
// Function-pointer signature aliases
// ---------------------------------------------------------------------------

pub type FnC2V = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnV2V = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnV3V = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnDot = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnCC = unsafe extern "C" fn(c2Circle, c2Circle) -> c_int;
pub type FnCA = unsafe extern "C" fn(c2Circle, c2AABB) -> c_int;
pub type FnAA = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnF2 = unsafe extern "C" fn(*const c_void, c_uint, *const c_void, c_uint) -> c_int;
pub type FnF3 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnF4 = unsafe extern "C" fn(*mut cn_rnd_t) -> f64;
pub type FnF5 = unsafe extern "C" fn(u32) -> u32;
pub type FnF7 = unsafe extern "C" fn(u32, u32, u32) -> u32;
pub type FnF9 = unsafe extern "C" fn(lm_vec2, lm_vec2, lm_vec2, lm_vec2) -> lm_vec2;
pub type FnF10 = unsafe extern "C" fn(u16) -> f32;
pub type FnTri = unsafe extern "C" fn(*mut f32, *const f32);

#[allow(clippy::too_many_arguments)]
pub type FnAgglom = unsafe extern "C" fn(
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    c_int,
    c_int,
    u64,
    u64,
    u32,
    u32,
    u32,
    u32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    u16,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
) -> f64;

// ---------------------------------------------------------------------------
// One side of the comparison: an entire loaded library
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub c2V: FnC2V,
    pub c2Maxv: FnV2V,
    pub c2Minv: FnV2V,
    pub c2Clampv: FnV3V,
    pub c2Sub: FnV2V,
    pub c2Dot: FnDot,
    pub c2CircletoCircle: FnCC,
    pub c2CircletoAABB: FnCA,
    pub c2AABBtoAABB: FnAA,
    pub f2: FnF2,
    pub f3: FnF3,
    pub f4: FnF4,
    pub f5: FnF5,
    pub f7: FnF7,
    pub f9: FnF9,
    pub f10: FnF10,
    pub f11: FnTri,
    pub f12: FnTri,
    pub f13: FnTri,
    pub agglom: FnAgglom,
}

unsafe fn get<T: Copy>(lib: &Library, name: &str) -> T {
    let s: Symbol<T> = lib
        .get(format!("{name}\0").as_bytes())
        .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"));
    *s
}

impl Api {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        Api {
            name,
            c2V: get(&lib, "c2V"),
            c2Maxv: get(&lib, "c2Maxv"),
            c2Minv: get(&lib, "c2Minv"),
            c2Clampv: get(&lib, "c2Clampv"),
            c2Sub: get(&lib, "c2Sub"),
            c2Dot: get(&lib, "c2Dot"),
            c2CircletoCircle: get(&lib, "c2CircletoCircle"),
            c2CircletoAABB: get(&lib, "c2CircletoAABB"),
            c2AABBtoAABB: get(&lib, "c2AABBtoAABB"),
            f2: get(&lib, "f2"),
            f3: get(&lib, "f3"),
            f4: get(&lib, "f4"),
            f5: get(&lib, "f5"),
            f7: get(&lib, "f7"),
            f9: get(&lib, "f9"),
            f10: get(&lib, "f10"),
            f11: get(&lib, "f11"),
            f12: get(&lib, "f12"),
            f13: get(&lib, "f13"),
            agglom: get(&lib, "agglom"),
            _lib: lib,
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so under {}. Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locate the C `.so` (public, so the Phase D symbol test can `nm` it).
pub fn c_so_path() -> PathBuf {
    find_c_so()
}

/// Locate the Rust cdylib (public, so the Phase D symbol test can `nm` it).
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

fn find_rust_so() -> PathBuf {
    // Explicit override, used to re-run the whole suite against the *release*
    // cdylib (`AGGLOM_RUST_SO=target/release/libagglom_lib.so cargo test`).
    if let Ok(p) = std::env::var("AGGLOM_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "AGGLOM_RUST_SO={} is not a file", p.display());
        return p;
    }
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib sits two levels up. Try that first, then the usual profiles.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                roots.push(profile.to_path_buf());
            }
        }
    }
    let target = repo_root().join("translation").join("target");
    roots.push(target.join("debug"));
    roots.push(target.join("release"));

    for r in &roots {
        let p = r.join("libagglom_lib.so");
        if p.is_file() {
            assert_not_stale(&p);
            return p;
        }
    }
    panic!(
        "libagglom_lib.so not found in any of {:?}. Run `cargo build` first.",
        roots
    );
}

/// Guard against silently testing a stale `.so`: if the cdylib predates any
/// Rust source file, the results would be meaningless.
fn assert_not_stale(so: &std::path::Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat cdylib");
    let src = repo_root().join("translation").join("src");
    for name in ["lib.rs", "tables.rs"] {
        let f = src.join(name);
        if let Ok(m) = std::fs::metadata(&f).and_then(|m| m.modified()) {
            assert!(
                so_mtime >= m,
                "STALE cdylib: {} is older than {}. Run `cargo build` before `cargo test`.",
                so.display(),
                f.display()
            );
        }
    }
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn apis() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Api::load("C", &find_c_so()),
            r: Api::load("Rust", &find_rust_so()),
        }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_f32(what: &str, ctx: &dyn std::fmt::Debug, c: f32, r: f32) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{what}: C={c:?} (0x{:08x}) != Rust={r:?} (0x{:08x})  input={ctx:?}",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_f64(what: &str, ctx: &dyn std::fmt::Debug, c: f64, r: f64) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{what}: C={c:?} (0x{:016x}) != Rust={r:?} (0x{:016x})  input={ctx:?}",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_v2(what: &str, ctx: &dyn std::fmt::Debug, c: c2v, r: c2v) {
    eq_f32(&format!("{what}.x"), ctx, c.x, r.x);
    eq_f32(&format!("{what}.y"), ctx, c.y, r.y);
}

#[track_caller]
pub fn eq_lm(what: &str, ctx: &dyn std::fmt::Debug, c: lm_vec2, r: lm_vec2) {
    eq_f32(&format!("{what}.x"), ctx, c.x, r.x);
    eq_f32(&format!("{what}.y"), ctx, c.y, r.y);
}

#[track_caller]
pub fn eq_i32(what: &str, ctx: &dyn std::fmt::Debug, c: c_int, r: c_int) {
    assert_eq!(c, r, "{what}: C={c} != Rust={r}  input={ctx:?}");
}

#[track_caller]
pub fn eq_u32(what: &str, ctx: &dyn std::fmt::Debug, c: u32, r: u32) {
    assert_eq!(
        c, r,
        "{what}: C={c} (0x{c:08x}) != Rust={r} (0x{r:08x})  input={ctx:?}"
    );
}

#[track_caller]
pub fn eq_tri(what: &str, ctx: &dyn std::fmt::Debug, c: &[f32; 3], r: &[f32; 3]) {
    for i in 0..3 {
        eq_f32(&format!("{what}[{i}]"), ctx, c[i], r[i]);
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Rng {
        Rng(s)
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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Any `f32` bit pattern — every float class, including NaNs with random
    /// payloads and both signs.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "reasonable" finite float in `[-lo, hi]`-ish range.
    pub fn finite_f32(&mut self, scale: f32) -> f32 {
        let t = (self.next_u32() as f64) / (u32::MAX as f64); // [0,1]
        ((t * 2.0 - 1.0) as f32) * scale
    }
    /// Small integral-ish float, good for hitting exact-equality branches.
    pub fn small_f32(&mut self) -> f32 {
        (self.below(41) as i32 - 20) as f32 * 0.25
    }
    /// Pick uniformly from a slice.
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.below(xs.len() as u32)) as usize]
    }
}

/// Interesting float values that hit the special-case branches.
pub const SPECIAL_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    60.0,
    -60.0,
    119.0,
    120.0,
    179.0,
    180.0,
    239.0,
    240.0,
    299.0,
    300.0,
    359.0,
    360.0,
    361.0,
    -1e-30,
    1e-30,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

/// NaNs / near-NaNs with distinct payloads, plus signalling NaNs, to catch
/// NaN-payload-propagation divergence.
pub const NAN_ZOO: &[u32] = &[
    0x7FC0_0000, // canonical qNaN
    0xFFC0_0000, // negative qNaN
    0x7FC0_0001,
    0x7FFF_FFFF,
    0xFFFF_FFFF,
    0x7F80_0001, // sNaN
    0xFF80_0001, // negative sNaN
    0x7FAA_AAAA,
    0xFFD5_5555,
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
];

/// Subnormals and other odd-but-finite values.
pub const ODD_F32: &[u32] = &[
    0x0000_0001, // smallest subnormal
    0x8000_0001,
    0x007F_FFFF, // largest subnormal
    0x807F_FFFF,
    0x0080_0000, // smallest normal
    0x7F7F_FFFF, // f32::MAX
    0x0000_0000,
    0x8000_0000,
];

/// Every value that makes a float "interesting" — union of the above.
pub fn zoo_f32() -> Vec<f32> {
    let mut v: Vec<f32> = SPECIAL_F32.to_vec();
    v.extend(NAN_ZOO.iter().map(|&b| f32::from_bits(b)));
    v.extend(ODD_F32.iter().map(|&b| f32::from_bits(b)));
    v
}

/// Interesting `i32` values (`f3` sign quadrants and overflow edges).
pub const SPECIAL_I32: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    7,
    -7,
    8,
    -8,
    100,
    -100,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x4000_0000,
    -0x4000_0000,
];

/// Interesting `u32` values.
pub const SPECIAL_U32: &[u32] = &[
    0,
    1,
    2,
    3,
    7,
    8,
    16,
    24,
    31,
    32,
    33,
    64,
    0xFF,
    0x100,
    0xFFFF,
    0x1_0000,
    0xFFFF_0000,
    0x8000_0000,
    u32::MAX,
    u32::MAX - 1,
];

/// Interesting `u64` seeds for `cn_rnd_t`.
pub const SPECIAL_U64: &[u64] = &[
    0,
    1,
    2,
    u64::MAX,
    u64::MAX - 1,
    0x8000_0000_0000_0000,
    0x0000_0000_FFFF_FFFF,
    0xFFFF_FFFF_0000_0000,
    0xDEAD_BEEF_CAFE_BABE,
    0x5555_5555_5555_5555,
    0xAAAA_AAAA_AAAA_AAAA,
];

// ---------------------------------------------------------------------------
// Convenience wrappers that run BOTH libraries and compare
// ---------------------------------------------------------------------------

/// Run a `void f(float* dest, const float* src)`-shaped function on both libs
/// with a fresh output buffer each side and compare bitwise.
#[track_caller]
pub fn diff_tri(which: &str, pick: fn(&Api) -> FnTri, src: [f32; 3]) {
    let a = apis();
    let mut dc: [f32; 3] = [0.0, 0.0, 0.0];
    let mut dr: [f32; 3] = [0.0, 0.0, 0.0];
    unsafe {
        (pick(&a.c))(dc.as_mut_ptr(), src.as_ptr());
        (pick(&a.r))(dr.as_mut_ptr(), src.as_ptr());
    }
    eq_tri(which, &BitsTri(src), &dc, &dr);
}

/// Debug wrapper printing a `[f32;3]` together with its bit patterns.
pub struct BitsTri(pub [f32; 3]);

impl std::fmt::Debug for BitsTri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?} (0x{:08x}), {:?} (0x{:08x}), {:?} (0x{:08x})]",
            self.0[0],
            self.0[0].to_bits(),
            self.0[1],
            self.0[1].to_bits(),
            self.0[2],
            self.0[2].to_bits()
        )
    }
}

/// Debug wrapper printing an `f32` together with its bit pattern.
pub struct Bits(pub f32);

impl std::fmt::Debug for Bits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} (0x{:08x})", self.0, self.0.to_bits())
    }
}
