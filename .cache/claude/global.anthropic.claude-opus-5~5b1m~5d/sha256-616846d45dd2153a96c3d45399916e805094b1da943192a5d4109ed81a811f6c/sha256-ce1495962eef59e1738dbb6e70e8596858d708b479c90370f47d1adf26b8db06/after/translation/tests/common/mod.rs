//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function
//! through its exported C symbol — the Rust crate is *never* linked directly,
//! so the `#[no_mangle]` / `extern "C"` wrappers are under test too.

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::c_int;
use std::ffi::c_void;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// FFI types (mirror c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

impl Debug for C2v {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "c2v{{x:{:?}/{:#010x}, y:{:?}/{:#010x}}}",
            self.x,
            self.x.to_bits(),
            self.y,
            self.y.to_bits()
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

impl Debug for C2Circle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "c2Circle{{p:{:?}, r:{:?}/{:#010x}}}", self.p, self.r, self.r.to_bits())
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

impl Debug for C2Aabb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "c2AABB{{min:{:?}, max:{:?}}}", self.min, self.max)
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

impl Debug for C2Capsule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "c2Capsule{{a:{:?}, b:{:?}, r:{:?}/{:#010x}}}",
            self.a, self.b, self.r, self.r.to_bits()
        )
    }
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// Sanity: the ABI shapes the SysV classifier sees must match the C ones.
const _: () = assert!(std::mem::size_of::<C2v>() == 8);
const _: () = assert!(std::mem::size_of::<C2Circle>() == 12);
const _: () = assert!(std::mem::size_of::<C2Aabb>() == 16);
const _: () = assert!(std::mem::size_of::<C2Capsule>() == 20);

// ---------------------------------------------------------------------------
// Loaded API
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: String,
    pub path: PathBuf,
    _lib: &'static libloading::Library,
    pub c2V: extern "C" fn(f32, f32) -> C2v,
    pub c2Mulvs: extern "C" fn(C2v, f32) -> C2v,
    pub c2Maxv: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Minv: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Clampv: extern "C" fn(C2v, C2v, C2v) -> C2v,
    pub c2Sub: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Dot: extern "C" fn(C2v, C2v) -> f32,
    pub c2CircletoCircle: extern "C" fn(C2Circle, C2Circle) -> c_int,
    pub c2CircletoAABB: extern "C" fn(C2Circle, C2Aabb) -> c_int,
    pub c2CircletoCapsule: extern "C" fn(C2Circle, C2Capsule) -> c_int,
    pub c2Collided: unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int,
    pub circle_collide: extern "C" fn(f32, f32, f32) -> c_int,
}

macro_rules! load {
    ($lib:expr, $path:expr, $sym:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get(concat!($sym, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("symbol `{}` missing from {:?}: {}", $sym, $path, e));
        // SAFETY: the symbol lives as long as the leaked `Library`.
        unsafe { std::mem::transmute::<$ty, $ty>(*s) }
    }};
}

impl Api {
    fn open(name: &str, path: &Path) -> Api {
        let lib: &'static libloading::Library = Box::leak(Box::new(
            unsafe { libloading::Library::new(path) }
                .unwrap_or_else(|e| panic!("cannot dlopen {path:?}: {e}")),
        ));
        Api {
            name: name.to_string(),
            path: path.to_path_buf(),
            c2V: load!(lib, path, "c2V", extern "C" fn(f32, f32) -> C2v),
            c2Mulvs: load!(lib, path, "c2Mulvs", extern "C" fn(C2v, f32) -> C2v),
            c2Maxv: load!(lib, path, "c2Maxv", extern "C" fn(C2v, C2v) -> C2v),
            c2Minv: load!(lib, path, "c2Minv", extern "C" fn(C2v, C2v) -> C2v),
            c2Clampv: load!(lib, path, "c2Clampv", extern "C" fn(C2v, C2v, C2v) -> C2v),
            c2Sub: load!(lib, path, "c2Sub", extern "C" fn(C2v, C2v) -> C2v),
            c2Dot: load!(lib, path, "c2Dot", extern "C" fn(C2v, C2v) -> f32),
            c2CircletoCircle: load!(
                lib,
                path,
                "c2CircletoCircle",
                extern "C" fn(C2Circle, C2Circle) -> c_int
            ),
            c2CircletoAABB: load!(
                lib,
                path,
                "c2CircletoAABB",
                extern "C" fn(C2Circle, C2Aabb) -> c_int
            ),
            c2CircletoCapsule: load!(
                lib,
                path,
                "c2CircletoCapsule",
                extern "C" fn(C2Circle, C2Capsule) -> c_int
            ),
            c2Collided: load!(
                lib,
                path,
                "c2Collided",
                unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int
            ),
            circle_collide: load!(lib, path, "circle_collide", extern "C" fn(f32, f32, f32) -> c_int),
            _lib: lib,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
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
    assert!(
        !found.is_empty(),
        "no C .so found in {build:?}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    found.remove(0)
}

fn find_rust_sos() -> Vec<(String, PathBuf)> {
    if let Ok(p) = std::env::var("RUST_SO") {
        return vec![("rust(env)".to_string(), PathBuf::from(p))];
    }
    let mut out = Vec::new();
    for profile in ["release", "debug"] {
        let p = manifest_dir()
            .join("target")
            .join(profile)
            .join("libcircle_collide_lib.so");
        if p.exists() {
            out.push((format!("rust/{profile}"), p));
        }
    }
    assert!(
        !out.is_empty(),
        "no Rust .so found; build it with `cargo build --release`"
    );
    out
}

pub struct Apis {
    pub c: Api,
    pub rust: Vec<Api>,
}

static APIS: OnceLock<Apis> = OnceLock::new();

pub fn apis() -> &'static Apis {
    APIS.get_or_init(|| {
        let c_path = find_c_so();
        let c = Api::open("c", &c_path);
        let rust = find_rust_sos()
            .into_iter()
            .map(|(n, p)| Api::open(&n, &p))
            .collect();
        Apis { c, rust }
    })
}

pub fn c() -> &'static Api {
    &apis().c
}

pub fn rusts() -> &'static [Api] {
    &apis().rust
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Run `f` against the C `.so` and every Rust `.so`, assert bit-identical
/// results, and return the (shared) result so callers can measure branch /
/// outcome coverage. `ctx` is only formatted on failure.
#[track_caller]
pub fn diff<T, C, F>(ctx: C, f: F) -> T
where
    T: PartialEq + Debug + Copy,
    C: Fn() -> String,
    F: Fn(&Api) -> T,
{
    let expected = f(c());
    for r in rusts() {
        let got = f(r);
        if got != expected {
            panic!(
                "DIVERGENCE\n  input : {}\n  C ({}) => {:?}\n  {} ({}) => {:?}",
                ctx(),
                c().path.display(),
                expected,
                r.name,
                r.path.display(),
                got
            );
        }
    }
    expected
}

/// Bit-exact representation of a `c2v` return value.
pub fn vbits(v: C2v) -> (u32, u32) {
    (v.x.to_bits(), v.y.to_bits())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    /// Any of the 2^32 `f32` encodings, uniformly (NaNs, infinities,
    /// denormals, negative zero included).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "reasonable" finite float: mixes small integers, tiny and huge
    /// magnitudes so that overflow / cancellation are both reachable.
    pub fn finite_f32(&mut self) -> f32 {
        match self.below(8) {
            0 => self.range(-4.0, 4.0),
            1 => self.range(-150.0, 150.0),
            2 => self.range(-1e6, 1e6),
            3 => self.range(-1e-6, 1e-6),
            4 => (self.below(21) as i32 - 10) as f32,
            5 => self.range(-1e30, 1e30),
            6 => self.range(-1e-30, 1e-30),
            _ => self.range(-1.0, 1.0),
        }
    }
    /// Finite, non-negative — for radii.
    pub fn radius(&mut self) -> f32 {
        match self.below(6) {
            0 => 0.0,
            1 => self.range(0.0, 1.0),
            2 => self.range(0.0, 60.0),
            3 => self.range(0.0, 1e6),
            4 => (self.below(40)) as f32,
            _ => self.range(0.0, 1e30),
        }
    }
    /// Weighted mix: mostly a special value, sometimes a random encoding.
    pub fn pathological_f32(&mut self) -> f32 {
        let sv = special_values();
        match self.below(3) {
            0 => self.any_f32(),
            1 => sv[self.below(sv.len() as u32) as usize],
            _ => {
                // random NaN / inf with a random payload and sign
                let sign = (self.next_u32() & 1) << 31;
                let payload = self.next_u32() & 0x007F_FFFF;
                f32::from_bits(sign | 0x7F80_0000 | payload)
            }
        }
    }
    pub fn v_finite(&mut self) -> C2v {
        C2v {
            x: self.finite_f32(),
            y: self.finite_f32(),
        }
    }
    pub fn v_any(&mut self) -> C2v {
        C2v {
            x: self.any_f32(),
            y: self.any_f32(),
        }
    }
    pub fn v_path(&mut self) -> C2v {
        C2v {
            x: self.pathological_f32(),
            y: self.pathological_f32(),
        }
    }
}

/// Every IEEE-754 class the code can be handed, plus the boundary constants
/// the C's unguarded arithmetic overflows on.
pub fn special_values() -> &'static [f32] {
    static SV: OnceLock<Vec<f32>> = OnceLock::new();
    SV.get_or_init(|| {
        vec![
            0.0f32,
            -0.0f32,
            1.0,
            -1.0,
            2.0,
            -2.0,
            0.5,
            -0.5,
            f32::MIN_POSITIVE,             // smallest normal
            -f32::MIN_POSITIVE,
            f32::from_bits(0x0000_0001),   // smallest denormal
            f32::from_bits(0x8000_0001),   // -smallest denormal
            f32::from_bits(0x007F_FFFF),   // largest denormal
            f32::from_bits(0x807F_FFFF),
            f32::MAX,
            f32::MIN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7FC0_0000),   // +QNaN, zero payload (default NaN)
            f32::from_bits(0xFFC0_0000),   // -QNaN, zero payload
            f32::from_bits(0x7FC0_1234),   // +QNaN, payload A
            f32::from_bits(0xFFCA_BCDE),   // -QNaN, payload B
            f32::from_bits(0x7F80_0001),   // +SNaN
            f32::from_bits(0xFF80_4321),   // -SNaN
        ]
    })
}

pub fn qnan(payload: u32, neg: bool) -> f32 {
    let s = if neg { 0x8000_0000u32 } else { 0 };
    f32::from_bits(s | 0x7FC0_0000 | (payload & 0x003F_FFFF))
}

pub fn snan(payload: u32, neg: bool) -> f32 {
    let s = if neg { 0x8000_0000u32 } else { 0 };
    let p = (payload & 0x007F_FFFF).max(1);
    f32::from_bits(s | 0x7F80_0000 | p)
}

// ---------------------------------------------------------------------------
// Geometry helpers (used to *steer* random generation into a given branch;
// never used as an oracle — the C `.so` is always the oracle)
// ---------------------------------------------------------------------------

/// One ULP up (toward +inf for positive, toward 0 for negative) — raw
/// increment of the significand, matching `nextafter` for finite values.
pub fn ulp_up(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let b = x.to_bits();
    if x > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

pub fn ulp_down(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(0x8000_0001);
    }
    let b = x.to_bits();
    if x > 0.0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

/// Which of `c2CircletoCapsule`'s three regions the point falls in:
/// `0` = `da < 0` (before-A cap), `1` = shaft (`da>=0, db<0`),
/// `2` = after-B cap (`da>=0, db>=0`). Mirrors `lib.c:84-99`.
pub fn capsule_region(p: C2v, cap: C2Capsule) -> u8 {
    let n = C2v {
        x: cap.b.x - cap.a.x,
        y: cap.b.y - cap.a.y,
    };
    let ap = C2v {
        x: p.x - cap.a.x,
        y: p.y - cap.a.y,
    };
    let da = ap.x * n.x + ap.y * n.y;
    if da < 0.0 {
        return 0;
    }
    let bp = C2v {
        x: p.x - cap.b.x,
        y: p.y - cap.b.y,
    };
    let db = bp.x * n.x + bp.y * n.y;
    if db < 0.0 { 1 } else { 2 }
}

/// Point at parameter `t` along the capsule axis, offset `s` perpendicular.
/// `t < 0` ⇒ region 0, `0 < t < 1` ⇒ region 1, `t > 1` ⇒ region 2.
pub fn point_on_capsule_axis(cap: C2Capsule, t: f32, s: f32) -> C2v {
    let nx = cap.b.x - cap.a.x;
    let ny = cap.b.y - cap.a.y;
    C2v {
        x: cap.a.x + nx * t - ny * s,
        y: cap.a.y + ny * t + nx * s,
    }
}

/// Copy `val` into `buf` starting at `offset`, returning a (possibly
/// unaligned) pointer to it. C's `*(c2Circle *)A` places no alignment
/// requirement beyond what the caller's pointer happens to have, and GCC
/// lowers the load to plain `mov`/`movss`, so unaligned works.
pub unsafe fn place<T: Copy>(buf: &mut [u8], offset: usize, val: T) -> *const std::ffi::c_void {
    assert!(offset + std::mem::size_of::<T>() <= buf.len());
    unsafe {
        std::ptr::copy_nonoverlapping(
            (&raw const val).cast::<u8>(),
            buf.as_mut_ptr().add(offset),
            std::mem::size_of::<T>(),
        );
        buf.as_ptr().add(offset).cast()
    }
}
