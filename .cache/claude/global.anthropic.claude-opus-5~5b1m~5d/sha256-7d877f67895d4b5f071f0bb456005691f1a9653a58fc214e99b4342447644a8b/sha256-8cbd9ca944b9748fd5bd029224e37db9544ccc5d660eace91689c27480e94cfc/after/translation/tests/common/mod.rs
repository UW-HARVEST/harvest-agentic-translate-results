//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading`:
//!   * the C reference  : `c_src/build/lib<project>.so`
//!   * the Rust crate   : `translation/target/<profile>/libreverse_collide_lib.so`
//!
//! Rust functions are NEVER called directly — every call goes through the
//! `.so` export, so the `#[no_mangle] extern "C"` wrappers are under test too.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI-identical type definitions (mirrors of the C typedefs)
// ---------------------------------------------------------------------------

pub type C2_TYPE = u32;
pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
pub const C2_TYPE_AABB: C2_TYPE = 1;
pub const C2_TYPE_CAPSULE: C2_TYPE = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// Mirror of the C `c2Simplex { c2sv a, b, c, d; float div; int count; }`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: i32,
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub trait BitEq {
    fn bit_eq(&self, other: &Self) -> bool;
    fn show(&self) -> String;
}

impl BitEq for f32 {
    fn bit_eq(&self, other: &Self) -> bool {
        // Byte-identical, with the one concession that any two NaNs are
        // considered equal (the NaN payload produced by `sqrtf` is not part of
        // the observable contract of either implementation).
        self.to_bits() == other.to_bits() || (self.is_nan() && other.is_nan())
    }
    fn show(&self) -> String {
        format!("{:?} (0x{:08x})", self, self.to_bits())
    }
}

impl BitEq for i32 {
    fn bit_eq(&self, other: &Self) -> bool {
        self == other
    }
    fn show(&self) -> String {
        format!("{}", self)
    }
}

impl BitEq for c2v {
    fn bit_eq(&self, other: &Self) -> bool {
        self.x.bit_eq(&other.x) && self.y.bit_eq(&other.y)
    }
    fn show(&self) -> String {
        format!("c2v {{ x: {}, y: {} }}", self.x.show(), self.y.show())
    }
}

impl BitEq for c2r {
    fn bit_eq(&self, other: &Self) -> bool {
        self.c.bit_eq(&other.c) && self.s.bit_eq(&other.s)
    }
    fn show(&self) -> String {
        format!("c2r {{ c: {}, s: {} }}", self.c.show(), self.s.show())
    }
}

impl BitEq for c2x {
    fn bit_eq(&self, other: &Self) -> bool {
        self.p.bit_eq(&other.p) && self.r.bit_eq(&other.r)
    }
    fn show(&self) -> String {
        format!("c2x {{ p: {}, r: {} }}", self.p.show(), self.r.show())
    }
}

impl BitEq for c2Proxy {
    fn bit_eq(&self, other: &Self) -> bool {
        self.radius.bit_eq(&other.radius)
            && self.count == other.count
            && (0..8).all(|i| self.verts[i].bit_eq(&other.verts[i]))
    }
    fn show(&self) -> String {
        let v: Vec<String> = self.verts.iter().map(|v| v.show()).collect();
        format!(
            "c2Proxy {{ radius: {}, count: {}, verts: [{}] }}",
            self.radius.show(),
            self.count,
            v.join(", ")
        )
    }
}

impl BitEq for c2sv {
    fn bit_eq(&self, other: &Self) -> bool {
        self.sA.bit_eq(&other.sA)
            && self.sB.bit_eq(&other.sB)
            && self.p.bit_eq(&other.p)
            && self.u.bit_eq(&other.u)
            && self.iA == other.iA
            && self.iB == other.iB
    }
    fn show(&self) -> String {
        format!(
            "c2sv {{ sA: {}, sB: {}, p: {}, u: {}, iA: {}, iB: {} }}",
            self.sA.show(),
            self.sB.show(),
            self.p.show(),
            self.u.show(),
            self.iA,
            self.iB
        )
    }
}

impl BitEq for c2Simplex {
    fn bit_eq(&self, other: &Self) -> bool {
        self.div.bit_eq(&other.div)
            && self.count == other.count
            && (0..4).all(|i| self.verts[i].bit_eq(&other.verts[i]))
    }
    fn show(&self) -> String {
        let v: Vec<String> = self.verts.iter().map(|v| v.show()).collect();
        format!(
            "c2Simplex {{ verts: [{}], div: {}, count: {} }}",
            v.join(", "),
            self.div.show(),
            self.count
        )
    }
}

impl BitEq for c2GJKCache {
    fn bit_eq(&self, other: &Self) -> bool {
        self.metric.bit_eq(&other.metric)
            && self.count == other.count
            && self.iA == other.iA
            && self.iB == other.iB
            && self.div.bit_eq(&other.div)
    }
    fn show(&self) -> String {
        format!(
            "c2GJKCache {{ metric: {}, count: {}, iA: {:?}, iB: {:?}, div: {} }}",
            self.metric.show(),
            self.count,
            self.iA,
            self.iB,
            self.div.show()
        )
    }
}

impl<T: BitEq> BitEq for Option<T> {
    fn bit_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (None, None) => true,
            (Some(a), Some(b)) => a.bit_eq(b),
            _ => false,
        }
    }
    fn show(&self) -> String {
        match self {
            None => "None".to_string(),
            Some(v) => format!("Some({})", v.show()),
        }
    }
}

impl<A: BitEq, B: BitEq> BitEq for (A, B) {
    fn bit_eq(&self, other: &Self) -> bool {
        self.0.bit_eq(&other.0) && self.1.bit_eq(&other.1)
    }
    fn show(&self) -> String {
        format!("({}, {})", self.0.show(), self.1.show())
    }
}

impl<A: BitEq, B: BitEq, C: BitEq> BitEq for (A, B, C) {
    fn bit_eq(&self, other: &Self) -> bool {
        self.0.bit_eq(&other.0) && self.1.bit_eq(&other.1) && self.2.bit_eq(&other.2)
    }
    fn show(&self) -> String {
        format!("({}, {}, {})", self.0.show(), self.1.show(), self.2.show())
    }
}

impl<A: BitEq, B: BitEq, C: BitEq, D: BitEq> BitEq for (A, B, C, D) {
    fn bit_eq(&self, other: &Self) -> bool {
        self.0.bit_eq(&other.0)
            && self.1.bit_eq(&other.1)
            && self.2.bit_eq(&other.2)
            && self.3.bit_eq(&other.3)
    }
    fn show(&self) -> String {
        format!(
            "({}, {}, {}, {})",
            self.0.show(),
            self.1.show(),
            self.2.show(),
            self.3.show()
        )
    }
}

impl<A: BitEq, B: BitEq, C: BitEq, D: BitEq, E: BitEq> BitEq for (A, B, C, D, E) {
    fn bit_eq(&self, other: &Self) -> bool {
        self.0.bit_eq(&other.0)
            && self.1.bit_eq(&other.1)
            && self.2.bit_eq(&other.2)
            && self.3.bit_eq(&other.3)
            && self.4.bit_eq(&other.4)
    }
    fn show(&self) -> String {
        format!(
            "({}, {}, {}, {}, {})",
            self.0.show(),
            self.1.show(),
            self.2.show(),
            self.3.show(),
            self.4.show()
        )
    }
}

/// Assert the C and Rust results are byte-identical, printing the input on
/// failure.
#[track_caller]
pub fn same<T: BitEq>(what: &str, input: &str, c: T, r: T) {
    if !c.bit_eq(&r) {
        panic!(
            "DIVERGENCE in {what}\n  input: {input}\n  C    : {}\n  Rust : {}",
            c.show(),
            r.show()
        );
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = crate_root().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|e| e == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // The test binary lives in target/<profile>/deps/, so the cdylib built by
    // the same `cargo test` invocation is one directory up.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                let p = profile.join("libreverse_collide_lib.so");
                if p.exists() {
                    return p;
                }
            }
        }
    }
    for prof in ["debug", "release"] {
        let p = crate_root()
            .join("target")
            .join(prof)
            .join("libreverse_collide_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust .so found; run `cargo build` first");
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// Refuse to test a `.so` that is older than its source: a stale artifact would
/// silently "prove" a translation that is no longer the one on disk.
fn assert_fresh(so: &Path, src: &Path) {
    if let (Some(a), Some(b)) = (mtime(so), mtime(src)) {
        if b > a {
            panic!(
                "STALE ARTIFACT: {} is older than {}.\n  Rebuild before testing:\n  \
                 cd translation && cargo build --release\n  cd c_src/build && cmake --build .",
                so.display(),
                src.display()
            );
        }
    }
}

fn load(path: &Path) -> Library {
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()))
}

/// Both implementations, loaded side by side.
pub struct Pair {
    pub c: Library,
    pub rs: Library,
}

static mut PAIR: Option<Pair> = None;
static ONCE: std::sync::Once = std::sync::Once::new();

/// Process-wide handle to the two libraries (dlopen'd exactly once).
pub fn pair() -> &'static Pair {
    unsafe {
        ONCE.call_once(|| {
            let c_so = find_c_so();
            let rust_so = find_rust_so();
            assert_fresh(&c_so, &crate_root().join("../c_src/src/lib.c"));
            assert_fresh(&rust_so, &crate_root().join("src/lib.rs"));
            let c = load(&c_so);
            let rs = load(&rust_so);
            #[allow(static_mut_refs)]
            {
                PAIR = Some(Pair { c, rs });
            }
        });
        #[allow(static_mut_refs)]
        PAIR.as_ref().unwrap()
    }
}

/// Fetch the same symbol from both libraries.
pub fn sym<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let p = pair();
    let mut cn = name.as_bytes().to_vec();
    cn.push(0);
    let a: Symbol<'static, T> = unsafe { p.c.get(&cn) }
        .unwrap_or_else(|e| panic!("C .so is missing symbol `{name}`: {e}"));
    let b: Symbol<'static, T> = unsafe { p.rs.get(&cn) }
        .unwrap_or_else(|e| panic!("Rust .so is missing symbol `{name}`: {e}"));
    (a, b)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible test runs)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo,hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// A "well behaved" coordinate: small integers, halves and general floats.
    pub fn coord(&mut self) -> f32 {
        match self.below(6) {
            0 => self.below(21) as f32 - 10.0,          // small integer
            1 => (self.below(41) as f32 - 20.0) * 0.5,  // half integer
            2 => self.range(-100.0, 100.0),
            3 => self.range(-1.0, 1.0),
            4 => self.range(-1e4, 1e4),
            _ => self.range(-50.0, 50.0),
        }
    }
    /// A radius: often 0, often integral, sometimes negative or huge.
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => self.below(20) as f32,
            2 => self.range(0.0, 1.0),
            3 => self.range(0.0, 50.0),
            4 => -self.range(0.0, 10.0),
            5 => self.range(0.0, 1e5),
            6 => 1.0,
            _ => self.range(0.0, 10.0),
        }
    }
    /// A float that also covers the nasty IEEE-754 corners.
    pub fn wild(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::MIN_POSITIVE,
            6 => -f32::MIN_POSITIVE,
            7 => f32::from_bits(1), // subnormal
            8 => f32::MAX,
            9 => f32::MIN,
            10 => 1.0,
            11 => -1.0,
            12 => f32::EPSILON,
            _ => self.coord(),
        }
    }
    pub fn vec(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn wild_vec(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.vec(),
            r: self.radius(),
        }
    }
    /// Random AABB. 1/6 of the time deliberately inverted, 1/6 zero extent.
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.vec();
        match self.below(6) {
            0 => c2AABB { min: a, max: a }, // zero extent
            1 => c2AABB {
                min: a,
                max: c2v {
                    x: a.x - self.range(0.0, 20.0),
                    y: a.y - self.range(0.0, 20.0),
                },
            }, // inverted
            _ => {
                let b = self.vec();
                c2AABB {
                    min: c2v {
                        x: a.x.min(b.x),
                        y: a.y.min(b.y),
                    },
                    max: c2v {
                        x: a.x.max(b.x),
                        y: a.y.max(b.y),
                    },
                }
            }
        }
    }
    /// Random capsule; 1/6 of the time degenerate (`a == b`).
    pub fn capsule(&mut self) -> c2Capsule {
        let a = self.vec();
        let b = if self.below(6) == 0 { a } else { self.vec() };
        c2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }
    /// Random rotation: identity, axis aligned, or arbitrary angle. The `c2r`
    /// is not required to be normalised by the C code, so un-normalised values
    /// are generated too.
    pub fn rot(&mut self) -> c2r {
        match self.below(6) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 1.0 },
            2 => c2r { c: -1.0, s: 0.0 },
            3 => c2r {
                c: self.range(-2.0, 2.0),
                s: self.range(-2.0, 2.0),
            },
            _ => {
                let t = self.range(-3.15, 3.15);
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
        }
    }
    pub fn xform(&mut self) -> c2x {
        c2x {
            p: if self.below(4) == 0 {
                c2v { x: 0.0, y: 0.0 }
            } else {
                self.vec()
            },
            r: self.rot(),
        }
    }
    pub fn simplex(&mut self, count: i32) -> c2Simplex {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i] = c2sv {
                sA: self.vec(),
                sB: self.vec(),
                p: self.vec(),
                u: self.range(-2.0, 2.0),
                iA: self.below(8) as i32,
                iB: self.below(8) as i32,
            };
        }
        s.div = match self.below(5) {
            0 => 1.0,
            1 => self.range(0.001, 10.0),
            2 => self.range(-10.0, -0.001),
            3 => self.range(0.0, 100.0),
            _ => 1.0,
        };
        s.count = count;
        s
    }
}

// ---------------------------------------------------------------------------
// Typed wrappers around the FFI exports (used by every test file)
// ---------------------------------------------------------------------------

pub type FnVV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVfV = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVf = unsafe extern "C" fn(c2v) -> f32;
pub type FnffV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnRVV = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnXVV = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnR = unsafe extern "C" fn() -> c2r;
pub type FnX = unsafe extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, C2_TYPE, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSupport = unsafe extern "C" fn(*const c2v, i32, c2v) -> i32;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    C2_TYPE,
    *const c2x,
    *const c_void,
    C2_TYPE,
    *const c2x,
    *mut c2v,
    *mut c2v,
    i32,
    *mut i32,
    *mut c2GJKCache,
) -> f32;
pub type FnAABBtoAABB = unsafe extern "C" fn(c2AABB, c2AABB) -> i32;
pub type FnAABBtoCapsule = unsafe extern "C" fn(c2AABB, c2Capsule) -> i32;
pub type FnCapsuletoCapsule = unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32;
pub type FnCircletoCircle = unsafe extern "C" fn(c2Circle, c2Circle) -> i32;
pub type FnCircletoAABB = unsafe extern "C" fn(c2Circle, c2AABB) -> i32;
pub type FnCircletoCapsule = unsafe extern "C" fn(c2Circle, c2Capsule) -> i32;
pub type FnCollided = unsafe extern "C" fn(*const c_void, C2_TYPE, *const c_void, C2_TYPE) -> i32;
pub type FnReverseCollide = unsafe extern "C" fn(f32, f32, f32) -> i32;

/// A shape usable as the `const void *` argument of `c2GJK` / `c2Collided`.
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    pub fn ty(&self) -> C2_TYPE {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    pub fn as_ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(a) => a as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
    pub fn show(&self) -> String {
        match self {
            Shape::Circle(c) => format!("Circle{{p:({},{}),r:{}}}", c.p.x, c.p.y, c.r),
            Shape::Aabb(a) => format!(
                "AABB{{min:({},{}),max:({},{})}}",
                a.min.x, a.min.y, a.max.x, a.max.y
            ),
            Shape::Capsule(c) => format!(
                "Capsule{{a:({},{}),b:({},{}),r:{}}}",
                c.a.x, c.a.y, c.b.x, c.b.y, c.r
            ),
        }
    }
}

pub fn rand_shape(rng: &mut Rng, ty: C2_TYPE) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(rng.circle()),
        C2_TYPE_AABB => Shape::Aabb(rng.aabb()),
        _ => Shape::Capsule(rng.capsule()),
    }
}

pub const ALL_TYPES: [C2_TYPE; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// Full observable result of one `c2GJK` call.
#[derive(Clone, Copy, Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub a: Option<c2v>,
    pub b: Option<c2v>,
    pub iters: Option<i32>,
    pub cache: Option<c2GJKCache>,
}

impl BitEq for GjkOut {
    fn bit_eq(&self, o: &Self) -> bool {
        self.dist.bit_eq(&o.dist)
            && self.a.bit_eq(&o.a)
            && self.b.bit_eq(&o.b)
            && self.iters.bit_eq(&o.iters)
            && self.cache.bit_eq(&o.cache)
    }
    fn show(&self) -> String {
        format!(
            "GjkOut {{ dist: {}, a: {}, b: {}, iters: {}, cache: {} }}",
            self.dist.show(),
            self.a.show(),
            self.b.show(),
            self.iters.show(),
            self.cache.show()
        )
    }
}

/// Options for one `c2GJK` invocation.
#[derive(Clone, Copy, Debug)]
pub struct GjkOpts {
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub use_radius: i32,
    pub want_a: bool,
    pub want_b: bool,
    pub want_iters: bool,
    /// `None` = pass NULL; `Some(cache)` = pass this cache in (and read it back).
    pub cache: Option<c2GJKCache>,
}

impl Default for GjkOpts {
    fn default() -> Self {
        GjkOpts {
            ax: None,
            bx: None,
            use_radius: 1,
            want_a: true,
            want_b: true,
            want_iters: true,
            cache: None,
        }
    }
}

impl GjkOpts {
    pub fn show(&self) -> String {
        format!(
            "ax:{} bx:{} use_radius:{} outA:{} outB:{} iters:{} cache:{}",
            self.ax
                .map(|x| format!("({},{},{},{})", x.p.x, x.p.y, x.r.c, x.r.s))
                .unwrap_or_else(|| "NULL".into()),
            self.bx
                .map(|x| format!("({},{},{},{})", x.p.x, x.p.y, x.r.c, x.r.s))
                .unwrap_or_else(|| "NULL".into()),
            self.use_radius,
            self.want_a,
            self.want_b,
            self.want_iters,
            self.cache
                .map(|c| format!(
                    "{{metric:{},count:{},iA:{:?},iB:{:?},div:{}}}",
                    c.metric, c.count, c.iA, c.iB, c.div
                ))
                .unwrap_or_else(|| "NULL".into()),
        )
    }
}

/// Call one `c2GJK` implementation and collect every observable output.
pub unsafe fn call_gjk(f: FnGJK, a: &Shape, b: &Shape, o: &GjkOpts) -> GjkOut {
    let ax = o.ax;
    let bx = o.bx;
    let mut outa = c2v { x: 12.5, y: -3.25 };
    let mut outb = c2v { x: -7.75, y: 9.5 };
    let mut iters: i32 = -12345;
    let mut cache = o.cache;

    let dist = f(
        a.as_ptr(),
        a.ty(),
        ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
        b.as_ptr(),
        b.ty(),
        bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
        if o.want_a {
            &mut outa as *mut c2v
        } else {
            std::ptr::null_mut()
        },
        if o.want_b {
            &mut outb as *mut c2v
        } else {
            std::ptr::null_mut()
        },
        o.use_radius,
        if o.want_iters {
            &mut iters as *mut i32
        } else {
            std::ptr::null_mut()
        },
        cache
            .as_mut()
            .map_or(std::ptr::null_mut(), |c| c as *mut c2GJKCache),
    );

    GjkOut {
        dist,
        a: if o.want_a { Some(outa) } else { None },
        b: if o.want_b { Some(outb) } else { None },
        iters: if o.want_iters { Some(iters) } else { None },
        cache,
    }
}

/// Differential `c2GJK` call: run both libraries, compare everything.
#[track_caller]
pub fn gjk_same(row: &str, a: &Shape, b: &Shape, o: &GjkOpts) {
    let (cf, rf) = sym::<FnGJK>("c2GJK");
    let (rc, rr) = unsafe { (call_gjk(*cf, a, b, o), call_gjk(*rf, a, b, o)) };
    same(
        row,
        &format!("A={} B={} [{}]", a.show(), b.show(), o.show()),
        rc,
        rr,
    );
}
