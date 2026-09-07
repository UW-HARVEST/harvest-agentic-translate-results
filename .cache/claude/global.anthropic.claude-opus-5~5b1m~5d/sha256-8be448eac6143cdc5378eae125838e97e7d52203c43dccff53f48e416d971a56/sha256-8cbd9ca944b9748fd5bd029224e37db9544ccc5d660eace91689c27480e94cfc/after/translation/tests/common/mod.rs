//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes a `Libs` value that
//! holds one typed function pointer per exported symbol for each library. No
//! Rust function is ever called directly — everything goes through the `.so`
//! export, so the `#[no_mangle] extern "C"` wrappers are under test too.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// FFI type mirrors (must be byte-identical to both libraries' layouts)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(v: f32) -> u32 {
    v.to_bits()
}

pub fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Bit-exact float equality (NaN payload and sign of zero included).
pub fn feq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn veq(a: C2v, b: C2v) -> bool {
    feq(a.x, b.x) && feq(a.y, b.y)
}

pub fn rceq(a: C2Raycast, b: C2Raycast) -> bool {
    feq(a.t, b.t) && veq(a.n, b.n)
}

pub fn fs(v: f32) -> String {
    format!("{:e} (0x{:08x})", v, v.to_bits())
}

pub fn vs(v: C2v) -> String {
    format!("({}, {})", fs(v.x), fs(v.y))
}

pub fn rcs(v: C2Raycast) -> String {
    format!("{{ t: {}, n: {} }}", fs(v.t), vs(v.n))
}

pub fn rays(r: C2Ray) -> String {
    format!("Ray{{p:{}, d:{}, t:{}}}", vs(r.p), vs(r.d), fs(r.t))
}

pub fn circs(c: C2Circle) -> String {
    format!("Circle{{p:{}, r:{}}}", vs(c.p), fs(c.r))
}

pub fn aabbs(c: C2AABB) -> String {
    format!("AABB{{min:{}, max:{}}}", vs(c.min), vs(c.max))
}

pub fn caps(c: C2Capsule) -> String {
    format!("Capsule{{a:{}, b:{}, r:{}}}", vs(c.a), vs(c.b), fs(c.r))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Fully unconstrained `f32`: any bit pattern, so NaNs get random payloads
    /// and signs, and subnormals / infinities show up naturally.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// Uniform in `[-range, range]`, always finite — for geometric scenes.
    pub fn geo(&mut self, range: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * range
    }
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + u * (hi - lo)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn any_v(&mut self) -> C2v {
        C2v {
            x: self.any_f32(),
            y: self.any_f32(),
        }
    }
    pub fn geo_v(&mut self, range: f32) -> C2v {
        C2v {
            x: self.geo(range),
            y: self.geo(range),
        }
    }
    /// Mixes fully-random bit patterns with values drawn from `SPECIALS`, so
    /// both "wild" and "interesting boundary" inputs occur.
    pub fn mixed_f32(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 {
            self.any_f32()
        } else {
            SPECIALS[self.below(SPECIALS.len() as u32) as usize]
        }
    }
    pub fn mixed_v(&mut self) -> C2v {
        C2v {
            x: self.mixed_f32(),
            y: self.mixed_f32(),
        }
    }
}

/// Boundary / special values the C arithmetic and comparisons distinguish.
/// Several *distinct* NaN payloads and both NaN signs are included on purpose:
/// `addss`/`mulss`/`subss` return the destination operand when both operands
/// are NaN, so these values pin down the operand order of every float op.
pub const SPECIALS: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    1e-38,                              // near-normal-min
    -1e-38,
    1e-45,                              // subnormal (smallest positive)
    -1e-45,
    1e38,                               // near-overflow: x*x -> inf
    -1e38,
    3.4028235e38,                       // f32::MAX
    -3.4028235e38,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,                           // +qNaN 0x7fc00000
    -f32::NAN,                          // -qNaN 0xffc00000
    f32::from_bits(0x7fc0_1234),        // +qNaN, distinct payload
    f32::from_bits(0xffc0_abcd),        // -qNaN, distinct payload
    f32::from_bits(0x7f80_0001),        // +sNaN
];

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, got {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // The test binary lives in target/<profile>/deps/, so the sibling cdylib is
    // two levels up. Fall back to scanning target/*/.
    let exe = std::env::current_exe().expect("current_exe");
    let cand = exe
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("libgen_ray_lib.so"));
    if let Some(c) = cand {
        if c.exists() {
            return c;
        }
    }
    for profile in ["release", "debug"] {
        let p = manifest_dir()
            .join("target")
            .join(profile)
            .join("libgen_ray_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libgen_ray_lib.so not found — run `cargo build` (and/or `--release`) first");
}

macro_rules! api {
    ($($field:ident : $sym:literal => $ty:ty),* $(,)?) => {
        pub struct Api {
            _lib: Library,
            $(pub $field: $ty,)*
        }
        impl Api {
            unsafe fn load(path: &std::path::Path) -> Api {
                let lib = unsafe { Library::new(path) }
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
                $(
                    let $field: $ty = unsafe {
                        let s: Symbol<$ty> = lib.get($sym)
                            .unwrap_or_else(|e| panic!("dlsym {:?} in {}: {e}", std::str::from_utf8($sym), path.display()));
                        *s
                    };
                )*
                Api { _lib: lib, $($field,)* }
            }
        }
    };
}

api! {
    c2V:             b"c2V\0"             => extern "C" fn(f32, f32) -> C2v,
    c2Dot:           b"c2Dot\0"           => extern "C" fn(C2v, C2v) -> f32,
    c2Len:           b"c2Len\0"           => extern "C" fn(C2v) -> f32,
    c2Add:           b"c2Add\0"           => extern "C" fn(C2v, C2v) -> C2v,
    c2Sub:           b"c2Sub\0"           => extern "C" fn(C2v, C2v) -> C2v,
    c2Mulvs:         b"c2Mulvs\0"         => extern "C" fn(C2v, f32) -> C2v,
    c2Div:           b"c2Div\0"           => extern "C" fn(C2v, f32) -> C2v,
    c2Norm:          b"c2Norm\0"          => extern "C" fn(C2v) -> C2v,
    c2Minv:          b"c2Minv\0"          => extern "C" fn(C2v, C2v) -> C2v,
    c2Maxv:          b"c2Maxv\0"          => extern "C" fn(C2v, C2v) -> C2v,
    c2Skew:          b"c2Skew\0"          => extern "C" fn(C2v) -> C2v,
    c2Absv:          b"c2Absv\0"          => extern "C" fn(C2v) -> C2v,
    c2CCW90:         b"c2CCW90\0"         => extern "C" fn(C2v) -> C2v,
    c2MulmvT:        b"c2MulmvT\0"        => extern "C" fn(C2m, C2v) -> C2v,
    c2AABBtoAABB:    b"c2AABBtoAABB\0"    => extern "C" fn(C2AABB, C2AABB) -> c_int,
    c2AABBtoPoint:   b"c2AABBtoPoint\0"   => extern "C" fn(C2AABB, C2v) -> c_int,
    c2CircleToPoint: b"c2CircleToPoint\0" => extern "C" fn(C2Circle, C2v) -> c_int,
    c2RaytoCircle:   b"c2RaytoCircle\0"   => unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int,
    c2RaytoAABB:     b"c2RaytoAABB\0"     => unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int,
    c2RaytoCapsule:  b"c2RaytoCapsule\0"  => unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int,
    c2CastRay:       b"c2CastRay\0"       => unsafe extern "C" fn(C2Ray, *const c_void, c_int, *mut C2Raycast) -> c_int,
    gen_ray:         b"gen_ray\0"         => unsafe extern "C" fn(
                                              *mut C2Raycast, *mut C2Raycast, *mut C2Raycast,
                                              f32, f32, f32, f32, f32, f32, f32,
                                              f32, f32, f32, f32, f32,
                                              f32, f32, f32, f32) -> c_int,
}

pub struct Libs {
    pub c: Api,
    pub r: Api,
}

/// Loads both shared objects once per test process.
pub fn libs() -> &'static Libs {
    use std::sync::OnceLock;
    static ONCE: OnceLock<Libs> = OnceLock::new();
    ONCE.get_or_init(|| {
        let cp = find_c_so();
        let rp = find_rust_so();
        eprintln!("C   .so: {}", cp.display());
        eprintln!("Rust.so: {}", rp.display());
        Libs {
            c: unsafe { Api::load(&cp) },
            r: unsafe { Api::load(&rp) },
        }
    })
}

// ---------------------------------------------------------------------------
// Divergence accounting
// ---------------------------------------------------------------------------

pub struct Diffs {
    pub name: &'static str,
    pub checked: usize,
    pub failures: Vec<String>,
}

impl Diffs {
    pub fn new(name: &'static str) -> Self {
        Diffs {
            name,
            checked: 0,
            failures: Vec::new(),
        }
    }
    pub fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        self.checked += 1;
        if !ok && self.failures.len() < 12 {
            self.failures.push(msg());
        } else if !ok {
            self.failures.push(String::from("<...>"));
        }
    }
    pub fn finish(self) {
        if !self.failures.is_empty() {
            let shown: Vec<&String> = self.failures.iter().take(12).collect();
            panic!(
                "[{}] {} / {} cases DIVERGED between C and Rust:\n{}",
                self.name,
                self.failures.len(),
                self.checked,
                shown
                    .iter()
                    .map(|s| format!("  {s}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        assert!(self.checked > 0, "[{}] no cases were checked", self.name);
        eprintln!("[{}] OK — {} cases matched bit-for-bit", self.name, self.checked);
    }
}

/// Compares one raycast-style call: return value, and the out-struct contents
/// bit-for-bit (including the case where the C leaves `out` untouched, which is
/// detected by pre-filling both with the same sentinel).
pub const SENTINEL: C2Raycast = C2Raycast {
    t: f32::from_bits(0xDEAD_BEEF),
    n: C2v {
        x: f32::from_bits(0xCAFE_BABE),
        y: f32::from_bits(0x0BAD_F00D),
    },
};
