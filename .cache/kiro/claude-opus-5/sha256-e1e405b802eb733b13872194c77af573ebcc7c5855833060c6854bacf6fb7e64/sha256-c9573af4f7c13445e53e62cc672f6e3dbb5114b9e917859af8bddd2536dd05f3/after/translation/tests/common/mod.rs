//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls every function through
//! its exported C symbol. No Rust function is ever called directly, so the
//! `#[no_mangle] extern "C"` wrappers and the SysV struct-passing ABI are part
//! of what is under test.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C-ABI mirror types (must match c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
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
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy { radius: 0.0, count: 0, verts: [c2v::default(); 8] }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// `c2sv a, b, c, d; float div; int count;` — 152 bytes, align 4.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

pub const FLT_EPSILON: f32 = 1.19209289550781250000000000000000000e-7;

// ---------------------------------------------------------------------------
// Function-pointer signatures
// ---------------------------------------------------------------------------

pub type FnV = extern "C" fn(f32, f32) -> c2v;
pub type FnVsV = extern "C" fn(c2v, f32) -> c2v;
pub type FnVVV = extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVVV = extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVVf = extern "C" fn(c2v, c2v) -> f32;
pub type FnVV = extern "C" fn(c2v) -> c2v;
pub type FnVf = extern "C" fn(c2v) -> f32;
pub type FnR = extern "C" fn() -> c2r;
pub type FnX = extern "C" fn() -> c2x;
pub type FnRVV = extern "C" fn(c2r, c2v) -> c2v;
pub type FnXVV = extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    c_int,
    *const c2x,
    *const c_void,
    c_int,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;
pub type FnAABBtoAABB = extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBtoCapsule = extern "C" fn(c2AABB, c2Capsule) -> c_int;
pub type FnCapsuletoCapsule = extern "C" fn(c2Capsule, c2Capsule) -> c_int;
pub type FnCircletoCircle = extern "C" fn(c2Circle, c2Circle) -> c_int;
pub type FnCircletoAABB = extern "C" fn(c2Circle, c2AABB) -> c_int;
pub type FnCircletoCapsule = extern "C" fn(c2Circle, c2Capsule) -> c_int;
pub type FnCollided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
pub type FnAabb = extern "C" fn(f32, f32, f32, f32) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    lib: libloading::Library,
}

impl Lib {
    /// Fetch an exported symbol by its exact C name.
    pub fn sym<T>(&self, name: &str) -> libloading::Symbol<'_, T> {
        unsafe {
            self.lib
                .get(format!("{name}\0").as_bytes())
                .unwrap_or_else(|e| panic!("{}: symbol `{name}` not found: {e}", self.name))
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_CDYLIB") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    // Prefer the release cdylib (the artifact an external consumer links), fall
    // back to debug.
    for dir in ["target/release", "target/debug"] {
        let p = md.join(dir).join("libaabb_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libaabb_lib.so not found; run `cargo build --release` first");
}

fn open(path: &Path, name: &'static str) -> Lib {
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    Lib { name, lib }
}

/// The pair of libraries under comparison. `c` is ground truth.
pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn libs() -> Pair {
    Pair { c: open(&find_c_so(), "C"), rs: open(&find_rust_so(), "Rust") }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-exact float comparison, with one deliberate exception: two `NaN`s are
/// considered identical regardless of payload/sign bits.
///
/// Justification (measured, not assumed): the *sign bit* of a `NaN` produced by
/// `mulss`/`addss` depends on which operand the compiler happened to place in
/// the destination register, not on the source. Compiling the very expression
/// from `c2Dot` — `a.x*b.x + a.y*b.y` — with two `NaN` inputs of opposite sign
/// gives `0x7fc00000` at `-O0` and `0xffc00000` at `-O1`/`-O2`/`-O3`/`-Os` from
/// the *same* C source. IEEE-754 leaves the sign of a propagated `NaN`
/// unspecified and C inherits that, so a payload difference is not a
/// translation defect — it is below the resolution of the C's own behaviour.
///
/// Every other property is compared bit-for-bit: `NaN` vs non-`NaN`,
/// `+0.0` vs `-0.0`, `inf` sign, and all finite values including denormals.
pub fn same_f32(a: f32, b: f32) -> bool {
    if a.is_nan() || b.is_nan() {
        return a.is_nan() && b.is_nan();
    }
    a.to_bits() == b.to_bits()
}

pub fn show_f32(v: f32) -> String {
    format!("{v:e} (0x{:08x})", v.to_bits())
}

pub fn same_v(a: c2v, b: c2v) -> bool {
    same_f32(a.x, b.x) && same_f32(a.y, b.y)
}

pub fn show_v(v: c2v) -> String {
    format!("({}, {})", show_f32(v.x), show_f32(v.y))
}

pub fn same_r(a: c2r, b: c2r) -> bool {
    same_f32(a.c, b.c) && same_f32(a.s, b.s)
}

pub fn same_x(a: c2x, b: c2x) -> bool {
    same_v(a.p, b.p) && same_r(a.r, b.r)
}

pub fn same_sv(a: &c2sv, b: &c2sv) -> bool {
    same_v(a.sA, b.sA)
        && same_v(a.sB, b.sB)
        && same_v(a.p, b.p)
        && same_f32(a.u, b.u)
        && a.iA == b.iA
        && a.iB == b.iB
}

pub fn show_sv(v: &c2sv) -> String {
    format!(
        "sA={} sB={} p={} u={} iA={} iB={}",
        show_v(v.sA),
        show_v(v.sB),
        show_v(v.p),
        show_f32(v.u),
        v.iA,
        v.iB
    )
}

/// Compare the whole 152-byte simplex, including the `d` slot the C never uses.
pub fn same_simplex(a: &c2Simplex, b: &c2Simplex) -> bool {
    a.count == b.count
        && same_f32(a.div, b.div)
        && (0..4).all(|i| same_sv(&a.verts[i], &b.verts[i]))
}

pub fn show_simplex(s: &c2Simplex) -> String {
    let mut out = format!("count={} div={}", s.count, show_f32(s.div));
    for (i, v) in s.verts.iter().enumerate() {
        out.push_str(&format!("\n    [{i}] {}", show_sv(v)));
    }
    out
}

pub fn same_proxy(a: &c2Proxy, b: &c2Proxy) -> bool {
    a.count == b.count
        && same_f32(a.radius, b.radius)
        && (0..8).all(|i| same_v(a.verts[i], b.verts[i]))
}

pub fn show_proxy(p: &c2Proxy) -> String {
    let mut out = format!("radius={} count={}", show_f32(p.radius), p.count);
    for (i, v) in p.verts.iter().enumerate() {
        out.push_str(&format!("\n    [{i}] {}", show_v(*v)));
    }
    out
}

pub fn same_cache(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    same_f32(a.metric, b.metric)
        && a.count == b.count
        && a.iA == b.iA
        && a.iB == b.iB
        && same_f32(a.div, b.div)
}

pub fn show_cache(c: &c2GJKCache) -> String {
    format!(
        "metric={} count={} iA={:?} iB={:?} div={}",
        show_f32(c.metric),
        c.count,
        c.iA,
        c.iB,
        show_f32(c.div)
    )
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed) + input generators
// ---------------------------------------------------------------------------

/// SplitMix64 — small, fast, and reproducible across platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in `[0,1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[-m, m]`.
    pub fn sym(&mut self, m: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * m
    }

    /// A "nasty" float: mostly ordinary values, but regularly hits the special
    /// values the C's comparisons branch on.
    pub fn nasty_f32(&mut self) -> f32 {
        match self.below(24) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::MIN_POSITIVE,          // smallest normal
            6 => f32::from_bits(1),          // smallest denormal
            7 => -f32::from_bits(1),
            8 => f32::MAX,
            9 => f32::MIN,
            10 => 1.0,
            11 => -1.0,
            12 => FLT_EPSILON,
            13 => -FLT_EPSILON,
            14 => 1.0e18,
            15 => -1.0e18,
            16 => 1.0e-30,
            17 => self.sym(1.0e8),
            18 => self.sym(1.0e-8),
            // A fully random bit pattern (can be any class of float).
            19 => f32::from_bits(self.next_u32()),
            _ => self.sym(100.0),
        }
    }

    /// An ordinary finite float in a physically plausible range.
    pub fn finite_f32(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => self.sym(1.0),
            2 => self.sym(1000.0),
            3 => (self.sym(20.0) as i32) as f32, // integral values -> exact ties
            _ => self.sym(120.0),
        }
    }

    pub fn nasty_v(&mut self) -> c2v {
        c2v { x: self.nasty_f32(), y: self.nasty_f32() }
    }

    pub fn finite_v(&mut self) -> c2v {
        c2v { x: self.finite_f32(), y: self.finite_f32() }
    }

    /// A `c2r`. Half the time a genuine unit rotation, half the time an
    /// arbitrary (non-normalized) one — the C never validates this.
    pub fn rot(&mut self) -> c2r {
        if self.below(2) == 0 {
            let a = self.unit() * std::f32::consts::TAU;
            c2r { c: a.cos(), s: a.sin() }
        } else {
            c2r { c: self.sym(3.0), s: self.sym(3.0) }
        }
    }

    pub fn xform(&mut self) -> c2x {
        c2x { p: self.finite_v(), r: self.rot() }
    }

    pub fn circle(&mut self) -> c2Circle {
        c2Circle { p: self.finite_v(), r: self.radius() }
    }

    pub fn radius(&mut self) -> f32 {
        match self.below(6) {
            0 => 0.0,
            1 => 1.0,
            2 => self.sym(50.0), // can be negative — the C never rejects it
            3 => 1.0e6,
            _ => self.unit() * 40.0,
        }
    }

    /// A box. Mostly proper (`min <= max`), sometimes degenerate or inverted.
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.finite_v();
        let b = self.finite_v();
        match self.below(8) {
            0 => c2AABB { min: a, max: a },                      // degenerate point
            1 => c2AABB { min: b, max: a },                       // possibly inverted
            2 => c2AABB { min: a, max: c2v { x: a.x, y: b.y } },  // flat in x
            3 => c2AABB { min: a, max: c2v { x: b.x, y: a.y } },  // flat in y
            _ => c2AABB {
                min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
            },
        }
    }

    pub fn capsule(&mut self) -> c2Capsule {
        let a = self.finite_v();
        let b = if self.below(6) == 0 { a } else { self.finite_v() }; // degenerate a == b
        c2Capsule { a, b, r: self.radius() }
    }

    pub fn simplex(&mut self, count: c_int) -> c2Simplex {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i] = c2sv {
                sA: self.finite_v(),
                sB: self.finite_v(),
                p: self.finite_v(),
                u: self.finite_f32(),
                iA: self.below(4) as c_int,
                iB: self.below(4) as c_int,
            };
        }
        // Occasionally duplicate vertices to hit the degenerate branches.
        match self.below(10) {
            0 => s.verts[1].p = s.verts[0].p,
            1 => s.verts[2].p = s.verts[0].p,
            2 => s.verts[2].p = s.verts[1].p,
            3 => {
                // Collinear triple: area == 0.
                let a = s.verts[0].p;
                let d = c2v { x: s.verts[1].p.x - a.x, y: s.verts[1].p.y - a.y };
                let t = self.sym(3.0);
                s.verts[2].p = c2v { x: a.x + d.x * t, y: a.y + d.y * t };
            }
            _ => {}
        }
        s.div = match self.below(6) {
            0 => 0.0,
            1 => 1.0,
            2 => self.sym(1.0),
            _ => self.finite_f32(),
        };
        s.count = count;
        s
    }
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

/// Records failures instead of panicking immediately, so one test run reports
/// every diverging case rather than only the first.
#[derive(Default)]
pub struct Report {
    pub checks: usize,
    pub failures: Vec<String>,
}

impl Report {
    pub fn new() -> Report {
        Report::default()
    }

    pub fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        self.checks += 1;
        if !ok && self.failures.len() < 25 {
            self.failures.push(msg());
        } else if !ok {
            self.failures.push("...".into());
        }
    }

    pub fn finish(self, what: &str) {
        assert!(self.checks > 0, "{what}: no checks ran");
        if !self.failures.is_empty() {
            panic!(
                "{what}: {} of {} differential checks diverged:\n{}",
                self.failures.len(),
                self.checks,
                self.failures.join("\n")
            );
        }
        eprintln!("{what}: {} differential checks passed", self.checks);
    }
}
