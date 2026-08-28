//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes every exported
//! symbol as a typed function pointer, so all calls (C *and* Rust) cross a real
//! FFI boundary and therefore also exercise the `#[no_mangle]` wrappers.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C ABI types (mirrors of c_src/include/lib.h + the private structs in lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

pub fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(a: f32) -> u32 {
    a.to_bits()
}

pub fn vbits(a: c2v) -> (u32, u32) {
    (a.x.to_bits(), a.y.to_bits())
}

pub fn rcbits(a: &c2Raycast) -> (u32, u32, u32) {
    (a.t.to_bits(), a.n.x.to_bits(), a.n.y.to_bits())
}

/// Formats an `f32` as hex bits + value so divergences are readable.
pub fn fs(a: f32) -> String {
    format!("{:#010x}({})", a.to_bits(), a)
}

pub fn vs(a: c2v) -> String {
    format!("({}, {})", fs(a.x), fs(a.y))
}

pub fn rcs(a: &c2Raycast) -> String {
    format!("{{t: {}, n: {}}}", fs(a.t), vs(a.n))
}

// ---------------------------------------------------------------------------
// Function-pointer signatures
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVV_F = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnV_F = unsafe extern "C" fn(c2v) -> f32;
pub type FnVV_V = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVF_V = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnV_V = unsafe extern "C" fn(c2v) -> c2v;
pub type FnMV_V = unsafe extern "C" fn(c2m, c2v) -> c2v;
pub type FnAABBAABB_I = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBV_I = unsafe extern "C" fn(c2AABB, c2v) -> c_int;
pub type FnCircleV_I = unsafe extern "C" fn(c2Circle, c2v) -> c_int;
pub type FnRayCircle = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
pub type FnRayAABB = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
pub type FnRayCapsule = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
pub type FnCastRay =
    unsafe extern "C" fn(c2Ray, *const c_void, c_int, *mut c2Raycast) -> c_int;
pub type FnGenRay = unsafe extern "C" fn(
    *mut c2Raycast,
    *mut c2Raycast,
    *mut c2Raycast,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
) -> c_int;

// ---------------------------------------------------------------------------
// Loaded library
// ---------------------------------------------------------------------------

pub struct Lib {
    #[allow(unused)]
    lib: libloading::Library,
    pub name: &'static str,
    pub c2V: FnV,
    pub c2Dot: FnVV_F,
    pub c2Len: FnV_F,
    pub c2Add: FnVV_V,
    pub c2Sub: FnVV_V,
    pub c2Mulvs: FnVF_V,
    pub c2Div: FnVF_V,
    pub c2Norm: FnV_V,
    pub c2Minv: FnVV_V,
    pub c2Maxv: FnVV_V,
    pub c2Skew: FnV_V,
    pub c2Absv: FnV_V,
    pub c2CCW90: FnV_V,
    pub c2MulmvT: FnMV_V,
    pub c2AABBtoAABB: FnAABBAABB_I,
    pub c2AABBtoPoint: FnAABBV_I,
    pub c2CircleToPoint: FnCircleV_I,
    pub c2RaytoCircle: FnRayCircle,
    pub c2RaytoAABB: FnRayAABB,
    pub c2RaytoCapsule: FnRayCapsule,
    pub c2CastRay: FnCastRay,
    pub gen_ray: FnGenRay,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
        };
        unsafe { *s.into_raw() }
    }};
}

impl Lib {
    pub fn open(path: &PathBuf, name: &'static str) -> Lib {
        let lib = unsafe {
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e))
        };
        Lib {
            name,
            c2V: sym!(lib, "c2V", FnV),
            c2Dot: sym!(lib, "c2Dot", FnVV_F),
            c2Len: sym!(lib, "c2Len", FnV_F),
            c2Add: sym!(lib, "c2Add", FnVV_V),
            c2Sub: sym!(lib, "c2Sub", FnVV_V),
            c2Mulvs: sym!(lib, "c2Mulvs", FnVF_V),
            c2Div: sym!(lib, "c2Div", FnVF_V),
            c2Norm: sym!(lib, "c2Norm", FnV_V),
            c2Minv: sym!(lib, "c2Minv", FnVV_V),
            c2Maxv: sym!(lib, "c2Maxv", FnVV_V),
            c2Skew: sym!(lib, "c2Skew", FnV_V),
            c2Absv: sym!(lib, "c2Absv", FnV_V),
            c2CCW90: sym!(lib, "c2CCW90", FnV_V),
            c2MulmvT: sym!(lib, "c2MulmvT", FnMV_V),
            c2AABBtoAABB: sym!(lib, "c2AABBtoAABB", FnAABBAABB_I),
            c2AABBtoPoint: sym!(lib, "c2AABBtoPoint", FnAABBV_I),
            c2CircleToPoint: sym!(lib, "c2CircleToPoint", FnCircleV_I),
            c2RaytoCircle: sym!(lib, "c2RaytoCircle", FnRayCircle),
            c2RaytoAABB: sym!(lib, "c2RaytoAABB", FnRayAABB),
            c2RaytoCapsule: sym!(lib, "c2RaytoCapsule", FnRayCapsule),
            c2CastRay: sym!(lib, "c2CastRay", FnCastRay),
            gen_ray: sym!(lib, "gen_ray", FnGenRay),
            lib,
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let root = workspace_root();
    let build = root.join("c_src").join("build");
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
            "no .so found in {} — build the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    let root = workspace_root();
    // Prefer the profile the tests were built with, but fall back to whatever
    // exists so a `cargo test` (debug) run still finds a `--release` artifact.
    let names = ["libgen_ray_lib.so"];
    let profiles: &[&str] = if cfg!(debug_assertions) {
        &["debug", "release"]
    } else {
        &["release", "debug"]
    };
    for prof in profiles {
        for n in names {
            let p = root.join("translation").join("target").join(prof).join(n);
            if p.exists() {
                return p;
            }
        }
    }
    panic!("libgen_ray_lib.so not found under translation/target/{{debug,release}} — run `cargo build`");
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

/// Loads both libraries. Called once per test (dlopen is refcounted & cheap).
pub fn load() -> Pair {
    Pair {
        c: Lib::open(&find_c_so(), "C"),
        r: Lib::open(&find_rust_so(), "Rust"),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64* — fixed seed, reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform `f32` in `[-scale, scale]`.
    pub fn uniform(&mut self, scale: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * scale
    }
    /// Uniform `f32` in `(0, scale]`.
    pub fn positive(&mut self, scale: f32) -> f32 {
        let u = ((self.next_u32() >> 8) + 1) as f32 / ((1u32 << 24) + 1) as f32;
        u * scale
    }
    /// A completely arbitrary `f32` bit pattern (all classes, incl. sNaN).
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// Picks from the "interesting" special-value pool, else a random normal.
    pub fn spicy(&mut self, scale: f32) -> f32 {
        let k = self.below(24);
        match k {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => -f32::NAN,
            6 => f32::from_bits(0x7f80_0001), // sNaN
            7 => f32::from_bits(0xff80_0001), // -sNaN
            8 => f32::from_bits(0x7fc0_1234), // qNaN, distinct payload
            9 => f32::from_bits(0xffc0_4321), // -qNaN, distinct payload
            10 => f32::MAX,
            11 => f32::MIN,
            12 => f32::MIN_POSITIVE,
            13 => -f32::MIN_POSITIVE,
            14 => f32::from_bits(1),          // smallest subnormal
            15 => f32::from_bits(0x8000_0001), // -smallest subnormal
            16 => 1.0,
            17 => -1.0,
            18 => 0.5,
            19 => -0.5,
            20 => 3.4e38,
            21 => -3.4e38,
            22 => self.any_bits(),
            _ => self.uniform(scale),
        }
    }
    pub fn vec_uniform(&mut self, scale: f32) -> c2v {
        c2v {
            x: self.uniform(scale),
            y: self.uniform(scale),
        }
    }
    pub fn vec_spicy(&mut self, scale: f32) -> c2v {
        c2v {
            x: self.spicy(scale),
            y: self.spicy(scale),
        }
    }
    pub fn vec_bits(&mut self) -> c2v {
        c2v {
            x: self.any_bits(),
            y: self.any_bits(),
        }
    }
}

/// The special-value pool used by the exhaustive cross-product tests.
pub const SPECIALS: &[u32] = &[
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // +min subnormal
    0x8000_0001, // -min subnormal
    0x007f_ffff, // +max subnormal
    0x0080_0000, // +min normal
    0x3f80_0000, // 1.0
    0xbf80_0000, // -1.0
    0x3f00_0000, // 0.5
    0x4048_f5c3, // 3.14
    0x7f7f_ffff, // f32::MAX
    0xff7f_ffff, // -f32::MAX
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x7fc0_0000, // +qNaN
    0xffc0_0000, // -qNaN
    0x7f80_0001, // +sNaN
    0xff80_0001, // -sNaN
    0x7fc0_1234, // +qNaN payload
    0xffc0_4321, // -qNaN payload
];

pub fn specials() -> Vec<f32> {
    SPECIALS.iter().map(|&b| f32::from_bits(b)).collect()
}

/// A recognisable sentinel used to pre-fill `c2Raycast` out-parameters so that
/// "was it written?" is observable.
pub const SENTINEL: c2Raycast = c2Raycast {
    t: 0.0, // overwritten below via `sentinel()`
    n: c2v { x: 0.0, y: 0.0 },
};

pub fn sentinel() -> c2Raycast {
    c2Raycast {
        t: f32::from_bits(0xdead_beef),
        n: c2v {
            x: f32::from_bits(0xcafe_babe),
            y: f32::from_bits(0xfeed_face),
        },
    }
}

// ---------------------------------------------------------------------------
// Differential assertion helpers
// ---------------------------------------------------------------------------

pub struct Diff {
    pub failures: Vec<String>,
    pub checked: u64,
    pub label: &'static str,
}

impl Diff {
    pub fn new(label: &'static str) -> Diff {
        Diff {
            failures: Vec::new(),
            checked: 0,
            label,
        }
    }

    pub fn eq_f32(&mut self, ctx: impl FnOnce() -> String, c: f32, r: f32) {
        self.checked += 1;
        if bits(c) != bits(r) {
            self.push(format!("{}: C={} Rust={}", ctx(), fs(c), fs(r)));
        }
    }

    pub fn eq_v(&mut self, ctx: impl FnOnce() -> String, c: c2v, r: c2v) {
        self.checked += 1;
        if vbits(c) != vbits(r) {
            self.push(format!("{}: C={} Rust={}", ctx(), vs(c), vs(r)));
        }
    }

    pub fn eq_i(&mut self, ctx: impl FnOnce() -> String, c: c_int, r: c_int) {
        self.checked += 1;
        if c != r {
            self.push(format!("{}: C={} Rust={}", ctx(), c, r));
        }
    }

    pub fn eq_cast(
        &mut self,
        ctx: impl Fn() -> String,
        cr: c_int,
        co: &c2Raycast,
        rr: c_int,
        ro: &c2Raycast,
    ) {
        self.checked += 1;
        if cr != rr {
            self.push(format!("{}: ret C={} Rust={}", ctx(), cr, rr));
        }
        if rcbits(co) != rcbits(ro) {
            self.push(format!("{}: out C={} Rust={}", ctx(), rcs(co), rcs(ro)));
        }
    }

    fn push(&mut self, s: String) {
        if self.failures.len() < 25 {
            self.failures.push(s);
        } else if self.failures.len() == 25 {
            self.failures.push("... (further failures suppressed)".to_string());
        }
    }

    pub fn finish(self) {
        if !self.failures.is_empty() {
            panic!(
                "[{}] {} divergence(s) out of {} comparisons:\n{}",
                self.label,
                self.failures.len(),
                self.checked,
                self.failures.join("\n")
            );
        }
        assert!(self.checked > 0, "[{}] no comparisons performed", self.label);
    }
}
