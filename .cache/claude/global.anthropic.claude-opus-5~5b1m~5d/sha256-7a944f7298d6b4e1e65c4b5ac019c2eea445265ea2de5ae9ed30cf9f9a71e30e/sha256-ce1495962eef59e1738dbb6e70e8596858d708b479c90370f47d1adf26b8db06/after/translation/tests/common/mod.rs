//! Shared differential-test harness.
//!
//! Both the C shared object (built by `c_src/CMakeLists.txt`) and the Rust
//! shared object (`libaabb_lib.so`) are loaded with `libloading` and every call
//! goes through the dynamic symbol table, exactly like an external consumer.
//! No Rust function is ever called directly.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// FFI types (mirrors of the C declarations in c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

pub const ALL_TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// `FLT_EPSILON`
pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// Compile-time layout checks (must match what gcc computes for c_src).
const _: () = {
    use std::mem::size_of;
    assert!(size_of::<c2v>() == 8);
    assert!(size_of::<c2r>() == 8);
    assert!(size_of::<c2x>() == 16);
    assert!(size_of::<c2Circle>() == 12);
    assert!(size_of::<c2AABB>() == 16);
    assert!(size_of::<c2Capsule>() == 20);
    assert!(size_of::<c2GJKCache>() == 36);
    assert!(size_of::<c2Proxy>() == 72);
    assert!(size_of::<c2sv>() == 36);
    assert!(size_of::<c2Simplex>() == 152);
};

// ---------------------------------------------------------------------------
// Function-pointer signatures
// ---------------------------------------------------------------------------

pub type FnV2 = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVecScalar = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVecVec = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVecVecVec = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVec = unsafe extern "C" fn(c2v) -> c2v;
pub type FnVecVecF = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVecF = unsafe extern "C" fn(c2v) -> f32;
pub type FnRotIdentity = unsafe extern "C" fn() -> c2r;
pub type FnXIdentity = unsafe extern "C" fn() -> c2x;
pub type FnMulrv = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnMulxv = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
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
pub type FnAABBtoAABB = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBtoCapsule = unsafe extern "C" fn(c2AABB, c2Capsule) -> c_int;
pub type FnCapsuletoCapsule = unsafe extern "C" fn(c2Capsule, c2Capsule) -> c_int;
pub type FnCircletoCircle = unsafe extern "C" fn(c2Circle, c2Circle) -> c_int;
pub type FnCircletoAABB = unsafe extern "C" fn(c2Circle, c2AABB) -> c_int;
pub type FnCircletoCapsule = unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int;
pub type FnCollided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
pub type FnAabb = unsafe extern "C" fn(f32, f32, f32, f32) -> c_int;

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn find_c_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("workspace root").to_path_buf();
    let build = root.join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    assert!(
        !candidates.is_empty(),
        "no C shared object found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \\\n    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    candidates.sort();
    candidates.remove(0)
}

fn find_rust_so() -> PathBuf {
    // target/<profile>/deps/<test-bin> -> target/<profile>/libaabb_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir: &Path = exe.parent().expect("deps dir");
    for _ in 0..3 {
        let cand = dir.join("libaabb_lib.so");
        if cand.is_file() {
            return cand;
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => break,
        }
    }
    // Fallback: look in the well-known profile directories.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in ["debug", "release"] {
        let cand = manifest.join("target").join(profile).join("libaabb_lib.so");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("libaabb_lib.so not found; run `cargo build` first");
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Libs {
    /// Resolve `name` in both libraries and return the pair of function
    /// pointers `(c, rust)`. Panics with a clear message when the Rust `.so`
    /// does not export a symbol the C `.so` does.
    pub fn pair<T: Copy>(&self, name: &str) -> (T, T) {
        let cn = format!("{name}\0");
        let c: Symbol<T> = unsafe { self.c.get(cn.as_bytes()) }
            .unwrap_or_else(|e| panic!("C .so is missing `{name}`: {e}"));
        let r: Symbol<T> = unsafe { self.rust.get(cn.as_bytes()) }
            .unwrap_or_else(|e| panic!("Rust .so is missing `{name}`: {e}"));
        (*c, *r)
    }
}

/// `cargo test` compiles the *test* targets but does **not** re-link the
/// `cdylib` artifact, so a plain `cargo test` after editing `src/lib.rs` would
/// silently exercise a stale `.so`. Refuse to run in that case.
fn assert_so_is_fresh(rust_so: &Path) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src").join("lib.rs");
    let (Ok(a), Ok(b)) = (std::fs::metadata(&src), std::fs::metadata(rust_so)) else {
        return;
    };
    let (Ok(ta), Ok(tb)) = (a.modified(), b.modified()) else {
        return;
    };
    assert!(
        tb >= ta,
        "{} is OLDER than src/lib.rs — run `cargo build` (or ./run_all.sh) first;\n\
         `cargo test` alone does not re-link the cdylib.",
        rust_so.display()
    );
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        assert_so_is_fresh(&rust_path);
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", rust_path.display()));
        Libs { c, rust, c_path, rust_path }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn feq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn veq(a: c2v, b: c2v) -> bool {
    feq(a.x, b.x) && feq(a.y, b.y)
}

pub fn req(a: c2r, b: c2r) -> bool {
    feq(a.c, b.c) && feq(a.s, b.s)
}

pub fn xeq(a: c2x, b: c2x) -> bool {
    veq(a.p, b.p) && req(a.r, b.r)
}

pub fn fdesc(v: f32) -> String {
    format!("{v:?} (0x{:08x})", v.to_bits())
}

pub fn vdesc(v: c2v) -> String {
    format!("({}, {})", fdesc(v.x), fdesc(v.y))
}

/// Raw byte image of any `Copy` POD — used to compare whole structs including
/// padding-free field-by-field equality of `NaN` payloads.
pub fn bytes_of<T: Copy>(v: &T) -> Vec<u8> {
    let p = v as *const T as *const u8;
    unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<T>()) }.to_vec()
}

pub fn simplex_eq(a: &c2Simplex, b: &c2Simplex) -> bool {
    if a.count != b.count || !feq(a.div, b.div) {
        return false;
    }
    for i in 0..4 {
        let (x, y) = (&a.verts[i], &b.verts[i]);
        if !veq(x.sA, y.sA)
            || !veq(x.sB, y.sB)
            || !veq(x.p, y.p)
            || !feq(x.u, y.u)
            || x.iA != y.iA
            || x.iB != y.iB
        {
            return false;
        }
    }
    true
}

pub fn simplex_desc(s: &c2Simplex) -> String {
    let mut out = format!("count={} div={}", s.count, fdesc(s.div));
    for (i, v) in s.verts.iter().enumerate() {
        out += &format!(
            "\n  v{i}: sA={} sB={} p={} u={} iA={} iB={}",
            vdesc(v.sA),
            vdesc(v.sB),
            vdesc(v.p),
            fdesc(v.u),
            v.iA,
            v.iB
        );
    }
    out
}

pub fn proxy_eq(a: &c2Proxy, b: &c2Proxy) -> bool {
    if a.count != b.count || !feq(a.radius, b.radius) {
        return false;
    }
    (0..8).all(|i| veq(a.verts[i], b.verts[i]))
}

pub fn proxy_desc(p: &c2Proxy) -> String {
    let mut out = format!("radius={} count={}", fdesc(p.radius), p.count);
    for (i, v) in p.verts.iter().enumerate() {
        out += &format!("\n  vert{i}={}", vdesc(*v));
    }
    out
}

pub fn cache_eq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    feq(a.metric, b.metric)
        && a.count == b.count
        && a.iA == b.iA
        && a.iB == b.iB
        && feq(a.div, b.div)
}

pub fn cache_desc(c: &c2GJKCache) -> String {
    format!(
        "metric={} count={} iA={:?} iB={:?} div={}",
        fdesc(c.metric),
        c.count,
        c.iA,
        c.iB,
        fdesc(c.div)
    )
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed => reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

/// Extra entropy mixed into every `Rng::new` seed. `0` (the default) keeps the
/// suite perfectly reproducible; set `C2_DIFF_SEED=<n>` to soak the same rows
/// with a completely different set of random inputs.
fn env_seed() -> u64 {
    static ENV_SEED: OnceLock<u64> = OnceLock::new();
    *ENV_SEED.get_or_init(|| {
        std::env::var("C2_DIFF_SEED")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    })
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mixed = seed ^ env_seed().wrapping_mul(0x9E37_79B9_7F4A_7C15);
        Rng(mixed | 1)
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

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n
    }

    pub fn boolean(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// A "nice" coordinate: quantised to 1/16 so that exact ties, exact
    /// touching and exact zero cases occur often.
    pub fn coord(&mut self, scale: f32) -> f32 {
        let n = (self.next_u32() % 641) as i32 - 320;
        (n as f32) * scale / 16.0
    }

    /// A coordinate drawn from a mixture of magnitude classes plus the
    /// interesting special values.
    pub fn wild(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => -f32::NAN,
            6 => f32::from_bits(0x7fc0_1234), // NaN with payload
            7 => f32::MIN_POSITIVE,
            8 => -f32::MIN_POSITIVE,
            9 => f32::from_bits(1), // smallest subnormal
            10 => f32::MAX,
            11 => f32::MIN,
            12 => self.range(-1.0e-4, 1.0e-4),
            13 => self.range(-1.0e6, 1.0e6),
            _ => self.coord(64.0),
        }
    }

    /// Finite-only wild value (no NaN/inf) for tests that need determinism of
    /// control flow but still want extreme magnitudes.
    pub fn wild_finite(&mut self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::MIN_POSITIVE,
            3 => f32::from_bits(1),
            4 => self.range(-1.0e-4, 1.0e-4),
            5 => self.range(-1.0e6, 1.0e6),
            6 => self.range(-1.0e12, 1.0e12),
            _ => self.coord(64.0),
        }
    }

    pub fn vec_wild(&mut self) -> c2v {
        c2v { x: self.wild(), y: self.wild() }
    }

    pub fn vec_finite(&mut self) -> c2v {
        c2v { x: self.wild_finite(), y: self.wild_finite() }
    }

    pub fn vec_coord(&mut self, scale: f32) -> c2v {
        c2v { x: self.coord(scale), y: self.coord(scale) }
    }
}

// ---------------------------------------------------------------------------
// Shape generators
// ---------------------------------------------------------------------------

/// Coordinate scale classes used across Phase B.
pub const SCALES: [f32; 4] = [1.0e-4, 1.0, 64.0, 1.0e6];

pub fn gen_circle(rng: &mut Rng, scale: f32) -> c2Circle {
    let r = match rng.below(6) {
        0 => 0.0,
        1 => scale / 16.0,
        2 => -scale / 4.0, // negative radius: the C never validates it
        _ => rng.range(0.0, scale),
    };
    c2Circle { p: rng.vec_coord(scale), r }
}

pub fn gen_aabb(rng: &mut Rng, scale: f32) -> c2AABB {
    let a = rng.vec_coord(scale);
    match rng.below(6) {
        0 => c2AABB { min: a, max: a }, // degenerate point box
        1 => c2AABB {
            // inverted box
            min: c2v { x: a.x + scale, y: a.y + scale },
            max: a,
        },
        _ => {
            let w = rng.range(0.0, scale);
            let h = rng.range(0.0, scale);
            c2AABB { min: a, max: c2v { x: a.x + w, y: a.y + h } }
        }
    }
}

pub fn gen_capsule(rng: &mut Rng, scale: f32) -> c2Capsule {
    let a = rng.vec_coord(scale);
    let b = match rng.below(6) {
        0 => a, // degenerate: a == b
        1 => c2v { x: a.x, y: a.y + rng.range(0.0, scale) }, // vertical
        2 => c2v { x: a.x + rng.range(0.0, scale), y: a.y }, // horizontal
        _ => rng.vec_coord(scale),
    };
    let r = match rng.below(5) {
        0 => 0.0,
        1 => -scale / 8.0,
        _ => rng.range(0.0, scale),
    };
    c2Capsule { a, b, r }
}

/// Storage for a shape of any of the three types, plus the `const void *` and
/// `C2_TYPE` needed to feed `c2GJK` / `c2Collided`.
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    pub fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    pub fn as_ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
}

pub fn gen_shape(rng: &mut Rng, ty: c_int, scale: f32) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(gen_circle(rng, scale)),
        C2_TYPE_AABB => Shape::Aabb(gen_aabb(rng, scale)),
        _ => Shape::Capsule(gen_capsule(rng, scale)),
    }
}

/// Random `c2x`, covering identity / rotation-only / translation-only /
/// rotation+translation / non-unit rotation.
pub fn gen_x(rng: &mut Rng, scale: f32) -> c2x {
    let r = match rng.below(5) {
        0 => c2r { c: 1.0, s: 0.0 },
        1 => {
            let a = rng.range(-3.15, 3.15);
            c2r { c: a.cos(), s: a.sin() }
        }
        2 => c2r { c: 0.0, s: 1.0 },
        3 => c2r { c: rng.range(-2.0, 2.0), s: rng.range(-2.0, 2.0) }, // non-unit
        _ => c2r { c: 0.0, s: 0.0 },                                   // annihilating
    };
    let p = match rng.below(3) {
        0 => c2v { x: 0.0, y: 0.0 },
        _ => rng.vec_coord(scale),
    };
    c2x { p, r }
}

/// Random simplex with `count` vertices and randomized `sA`/`sB`/`u`/`div`.
pub fn gen_simplex(rng: &mut Rng, count: c_int, scale: f32, wild: bool) -> c2Simplex {
    let mut s = c2Simplex::default();
    for i in 0..4 {
        let sA = if wild { rng.vec_wild() } else { rng.vec_coord(scale) };
        let sB = if wild { rng.vec_wild() } else { rng.vec_coord(scale) };
        s.verts[i].sA = sA;
        s.verts[i].sB = sB;
        // `p` mirrors what c2GJK stores: sB - sA. Randomised independently in
        // some rows so the helpers are also probed with inconsistent state.
        s.verts[i].p = if rng.boolean() {
            c2v { x: sB.x - sA.x, y: sB.y - sA.y }
        } else if wild {
            rng.vec_wild()
        } else {
            rng.vec_coord(scale)
        };
        s.verts[i].u = if wild { rng.wild() } else { rng.range(-2.0, 2.0) };
        s.verts[i].iA = (rng.next_u32() % 8) as c_int;
        s.verts[i].iB = (rng.next_u32() % 8) as c_int;
    }
    s.div = match rng.below(6) {
        0 => 1.0,
        1 => 0.0,
        2 => -1.0,
        _ => rng.range(-4.0, 4.0),
    };
    s.count = count;
    s
}
