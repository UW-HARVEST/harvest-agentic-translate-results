//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH libraries are loaded as shared objects through `libloading`; nothing in
//! here ever calls a Rust function directly, so the `#[no_mangle]` export
//! wrappers are part of what is under test.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// repr(C) mirrors of the C types (c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
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
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Simplex {
    pub v: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// Bit-level fingerprint. Using raw bits means `NaN != 0.0`, `-0.0 != 0.0`
/// and every distinct NaN payload is distinguished, so "byte-identical" really
/// means byte-identical.
pub trait Bits {
    fn bits(&self, out: &mut Vec<u32>);
    fn fingerprint(&self) -> Vec<u32> {
        let mut v = Vec::new();
        self.bits(&mut v);
        v
    }
}

impl Bits for f32 {
    fn bits(&self, out: &mut Vec<u32>) {
        out.push(self.to_bits());
    }
}
impl Bits for c_int {
    fn bits(&self, out: &mut Vec<u32>) {
        out.push(*self as u32);
    }
}
impl Bits for c2v {
    fn bits(&self, out: &mut Vec<u32>) {
        out.push(self.x.to_bits());
        out.push(self.y.to_bits());
    }
}
impl Bits for c2r {
    fn bits(&self, out: &mut Vec<u32>) {
        out.push(self.c.to_bits());
        out.push(self.s.to_bits());
    }
}
impl Bits for c2x {
    fn bits(&self, out: &mut Vec<u32>) {
        self.p.bits(out);
        self.r.bits(out);
    }
}
impl Bits for c2Circle {
    fn bits(&self, out: &mut Vec<u32>) {
        self.p.bits(out);
        out.push(self.r.to_bits());
    }
}
impl Bits for c2AABB {
    fn bits(&self, out: &mut Vec<u32>) {
        self.min.bits(out);
        self.max.bits(out);
    }
}
impl Bits for c2Capsule {
    fn bits(&self, out: &mut Vec<u32>) {
        self.a.bits(out);
        self.b.bits(out);
        out.push(self.r.to_bits());
    }
}
impl Bits for c2GJKCache {
    fn bits(&self, out: &mut Vec<u32>) {
        out.push(self.metric.to_bits());
        out.push(self.count as u32);
        for i in 0..3 {
            out.push(self.iA[i] as u32);
        }
        for i in 0..3 {
            out.push(self.iB[i] as u32);
        }
        out.push(self.div.to_bits());
    }
}
impl Bits for c2Proxy {
    fn bits(&self, out: &mut Vec<u32>) {
        out.push(self.radius.to_bits());
        out.push(self.count as u32);
        for v in self.verts.iter() {
            v.bits(out);
        }
    }
}
impl Bits for c2sv {
    fn bits(&self, out: &mut Vec<u32>) {
        self.sA.bits(out);
        self.sB.bits(out);
        self.p.bits(out);
        out.push(self.u.to_bits());
        out.push(self.iA as u32);
        out.push(self.iB as u32);
    }
}
impl Bits for c2Simplex {
    fn bits(&self, out: &mut Vec<u32>) {
        for v in self.v.iter() {
            v.bits(out);
        }
        out.push(self.div.to_bits());
        out.push(self.count as u32);
    }
}
impl<T: Bits> Bits for Option<T> {
    fn bits(&self, out: &mut Vec<u32>) {
        match self {
            None => out.push(0xDEAD_BEEF),
            Some(t) => {
                out.push(0);
                t.bits(out);
            }
        }
    }
}
impl<T: Bits, U: Bits> Bits for (T, U) {
    fn bits(&self, out: &mut Vec<u32>) {
        self.0.bits(out);
        self.1.bits(out);
    }
}
impl<T: Bits, U: Bits, V: Bits> Bits for (T, U, V) {
    fn bits(&self, out: &mut Vec<u32>) {
        self.0.bits(out);
        self.1.bits(out);
        self.2.bits(out);
    }
}
impl<T: Bits, U: Bits, V: Bits, W: Bits> Bits for (T, U, V, W) {
    fn bits(&self, out: &mut Vec<u32>) {
        self.0.bits(out);
        self.1.bits(out);
        self.2.bits(out);
        self.3.bits(out);
    }
}
impl<T: Bits> Bits for Vec<T> {
    fn bits(&self, out: &mut Vec<u32>) {
        out.push(self.len() as u32);
        for t in self.iter() {
            t.bits(out);
        }
    }
}
impl<T: Bits, const N: usize> Bits for [T; N] {
    fn bits(&self, out: &mut Vec<u32>) {
        for t in self.iter() {
            t.bits(out);
        }
    }
}

// ---------------------------------------------------------------------------
// The loaded API surface (all 38 exported symbols)
// ---------------------------------------------------------------------------

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

macro_rules! api {
    ( $( $name:ident : $ty:ty ),* $(,)? ) => {
        pub struct Api {
            pub tag: &'static str,
            _lib: libloading::Library,
            $( pub $name : $ty, )*
        }

        impl Api {
            pub fn load(tag: &'static str, path: &Path) -> Api {
                unsafe {
                    let lib = libloading::Library::new(path)
                        .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
                    $(
                        let $name: $ty = {
                            let sym: libloading::Symbol<$ty> = lib
                                .get(concat!(stringify!($name), "\0").as_bytes())
                                .unwrap_or_else(|e| panic!(
                                    "{tag}: missing symbol {}: {e}", stringify!($name)));
                            *sym
                        };
                    )*
                    Api { tag, _lib: lib, $( $name, )* }
                }
            }
        }
    };
}

api! {
    c2V: unsafe extern "C" fn(f32, f32) -> c2v,
    c2Mulvs: unsafe extern "C" fn(c2v, f32) -> c2v,
    c2Maxv: unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Minv: unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Clampv: unsafe extern "C" fn(c2v, c2v, c2v) -> c2v,
    c2Sub: unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Dot: unsafe extern "C" fn(c2v, c2v) -> f32,
    c2RotIdentity: unsafe extern "C" fn() -> c2r,
    c2xIdentity: unsafe extern "C" fn() -> c2x,
    c2BBVerts: unsafe extern "C" fn(*mut c2v, *mut c2AABB),
    c2MakeProxy: unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy),
    c2Len: unsafe extern "C" fn(c2v) -> f32,
    c2Det2: unsafe extern "C" fn(c2v, c2v) -> f32,
    c2GJKSimplexMetric: unsafe extern "C" fn(*mut c2Simplex) -> f32,
    c2Mulrv: unsafe extern "C" fn(c2r, c2v) -> c2v,
    c2Add: unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Mulxv: unsafe extern "C" fn(c2x, c2v) -> c2v,
    c22: unsafe extern "C" fn(*mut c2Simplex),
    c23: unsafe extern "C" fn(*mut c2Simplex),
    c2Neg: unsafe extern "C" fn(c2v) -> c2v,
    c2Skew: unsafe extern "C" fn(c2v) -> c2v,
    c2CCW90: unsafe extern "C" fn(c2v) -> c2v,
    c2D: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    c2Support: unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int,
    c2Witness: unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v),
    c2Div: unsafe extern "C" fn(c2v, f32) -> c2v,
    c2Norm: unsafe extern "C" fn(c2v) -> c2v,
    c2L: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    c2MulrvT: unsafe extern "C" fn(c2r, c2v) -> c2v,
    c2GJK: FnGJK,
    c2AABBtoAABB: unsafe extern "C" fn(c2AABB, c2AABB) -> c_int,
    c2AABBtoCapsule: unsafe extern "C" fn(c2AABB, c2Capsule) -> c_int,
    c2CapsuletoCapsule: unsafe extern "C" fn(c2Capsule, c2Capsule) -> c_int,
    c2CircletoCircle: unsafe extern "C" fn(c2Circle, c2Circle) -> c_int,
    c2CircletoAABB: unsafe extern "C" fn(c2Circle, c2AABB) -> c_int,
    c2CircletoCapsule: unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int,
    c2Collided: unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
    capsule: unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int,
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Option<PathBuf> = None;
    let entries = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("c_src/build missing ({e}) — build the C library first"));
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found = Some(p);
            break;
        }
    }
    found.unwrap_or_else(|| panic!("no .so in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // Allow pointing the harness at a specific build (e.g. the debug cdylib,
    // which keeps slice bounds checks and `panic = "unwind"`, so it proves the
    // raw-pointer paths really are index-safe).
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO={} does not exist", p.display());
        return p;
    }
    let t = workspace_root().join("translation/target");
    for prof in ["release", "debug"] {
        let p = t.join(prof).join("libcapsule_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libcapsule_lib.so not found — run `cargo build --release` first");
}

/// The two implementations under test: `(C, Rust)`.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Api::load("C", &find_c_so()),
        r: Api::load("Rust", &find_rust_so()),
    }
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

/// Runs `f` against both implementations and asserts the observable results are
/// bit-identical. `ctx` should identify the CONFIGS.md/ERRORS.md row and input.
pub fn diff<T, F>(ctx: &str, p: &Pair, f: F)
where
    T: Bits + std::fmt::Debug,
    F: Fn(&Api) -> T,
{
    let got_c = f(&p.c);
    let got_r = f(&p.r);
    let bc = got_c.fingerprint();
    let br = got_r.fingerprint();
    if bc != br {
        panic!(
            "DIVERGENCE [{ctx}]\n  C    = {got_c:?}\n  Rust = {got_r:?}\n  \
             C bits    = {bc:08x?}\n  Rust bits = {br:08x?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    /// Uniform in `[0,1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[lo,hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// A "normal" coordinate: moderate magnitude, mixture of grid-aligned and
    /// arbitrary values so exact-equality branches (`u <= 0`, `d2 == r2`) get
    /// hit as well as generic ones.
    pub fn coord(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => -0.0,
            2 => self.below(21) as f32 - 10.0,             // integers -10..10
            3 => (self.below(41) as f32 - 20.0) * 0.5,     // halves
            4 => self.range(-1.0, 1.0),
            5 => self.range(-100.0, 100.0),
            6 => self.range(-1.0e4, 1.0e4),
            _ => self.range(-10.0, 10.0),
        }
    }

    /// A coordinate spanning the whole float range, including specials.
    pub fn coord_wide(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::MAX,
            6 => f32::MIN,
            7 => f32::MIN_POSITIVE,
            8 => -f32::MIN_POSITIVE,
            9 => f32::from_bits(1),  // smallest denormal
            10 => f32::from_bits(0x8000_0001),
            11 => 1.192_092_895_507_812_5e-7, // FLT_EPSILON
            12 => -1.192_092_895_507_812_5e-7,
            13 => f32::from_bits(self.next_u32()), // any bit pattern at all
            14 => self.range(-1.0e18, 1.0e18),
            _ => self.coord(),
        }
    }

    pub fn vec(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn vec_wide(&mut self) -> c2v {
        c2v {
            x: self.coord_wide(),
            y: self.coord_wide(),
        }
    }
    pub fn radius(&mut self) -> f32 {
        match self.below(6) {
            0 => 0.0,
            1 => -0.0,
            2 => self.range(-5.0, 0.0), // negative radii are accepted by the C
            3 => self.below(20) as f32,
            _ => self.range(0.0, 30.0),
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.vec(),
            r: self.radius(),
        }
    }
    /// AABB; sometimes well-ordered, sometimes inverted, sometimes degenerate.
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.vec();
        let b = self.vec();
        match self.below(4) {
            0 => c2AABB { min: a, max: b }, // possibly inverted
            1 => c2AABB { min: a, max: a }, // degenerate point
            _ => c2AABB {
                min: c2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
                max: c2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
            },
        }
    }
    pub fn capsule(&mut self) -> c2Capsule {
        let a = self.vec();
        let b = if self.below(5) == 0 { a } else { self.vec() };
        c2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }
    /// A `c2r`: identity, a true unit rotation, or a non-normalised one.
    pub fn rot(&mut self) -> c2r {
        match self.below(5) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 0.0 },
            2 => {
                let t = self.range(-3.141_592_7, 3.141_592_7);
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
            3 => c2r {
                c: self.range(-2.0, 2.0),
                s: self.range(-2.0, 2.0),
            },
            _ => {
                let t = self.range(0.0, 6.283_185_5);
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
        }
    }
    pub fn xform(&mut self) -> c2x {
        c2x {
            p: self.vec(),
            r: self.rot(),
        }
    }
    /// A random simplex with the given count; `p`/`sA`/`sB`/`u` all randomized.
    pub fn simplex(&mut self, count: c_int) -> c2Simplex {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.v[i].sA = self.vec();
            s.v[i].sB = self.vec();
            s.v[i].p = self.vec();
            s.v[i].u = self.coord();
            s.v[i].iA = self.below(8) as c_int;
            s.v[i].iB = self.below(8) as c_int;
        }
        s.div = match self.below(6) {
            0 => 0.0,
            1 => 1.0,
            _ => self.coord(),
        };
        s.count = count;
        s
    }
}

/// The number of randomized cases per CONFIGS.md row.
pub const N: usize = 3000;

// ---------------------------------------------------------------------------
// Shape plumbing shared by the c2GJK / c2Collided rows in both phases
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Shape plumbing for the c2GJK / c2Collided rows
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
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
}

/// Materialises the shape in a stack local and hands its address to `f`,
/// exactly the way the C callers (`c2AABBtoCapsule`, `capsule`) do.
pub unsafe fn with_shape<R>(s: &Shape, f: impl FnOnce(*const c_void) -> R) -> R {
    match *s {
        Shape::Circle(v) => {
            let mut v = v;
            f(&mut v as *mut c2Circle as *const c_void)
        }
        Shape::Aabb(v) => {
            let mut v = v;
            f(&mut v as *mut c2AABB as *const c_void)
        }
        Shape::Capsule(v) => {
            let mut v = v;
            f(&mut v as *mut c2Capsule as *const c_void)
        }
    }
}

pub fn rand_shape(g: &mut Rng, ty: c_int) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(g.circle()),
        C2_TYPE_AABB => Shape::Aabb(g.aabb()),
        _ => Shape::Capsule(g.capsule()),
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GjkOpts {
    pub use_radius: c_int,
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub want_a: bool,
    pub want_b: bool,
    pub want_iters: bool,
    /// `None` = pass a NULL cache pointer.
    pub cache: Option<c2GJKCache>,
}

impl Default for GjkOpts {
    fn default() -> Self {
        GjkOpts {
            use_radius: 0,
            ax: None,
            bx: None,
            want_a: true,
            want_b: true,
            want_iters: true,
            cache: None,
        }
    }
}

/// Everything observable from one `c2GJK` call.
pub struct GjkOut {
    pub dist: f32,
    pub a: Option<c2v>,
    pub b: Option<c2v>,
    pub iters: Option<c_int>,
    pub cache: Option<c2GJKCache>,
}

impl Bits for GjkOut {
    fn bits(&self, out: &mut Vec<u32>) {
        out.push(self.dist.to_bits());
        self.a.bits(out);
        self.b.bits(out);
        self.iters.bits(out);
        self.cache.bits(out);
    }
}

impl std::fmt::Debug for GjkOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dist={:?}({:#010x}) a={:?} b={:?} iters={:?} cache={:?}",
            self.dist,
            self.dist.to_bits(),
            self.a,
            self.b,
            self.iters,
            self.cache
        )
    }
}

/// One `c2GJK` invocation through the `.so`.
pub fn call_gjk(x: &Api, sa: &Shape, sb: &Shape, o: &GjkOpts) -> GjkOut {
    unsafe {
        let mut ax = o.ax;
        let mut bx = o.bx;
        let axp = match ax.as_mut() {
            Some(t) => t as *const c2x,
            None => core::ptr::null(),
        };
        let bxp = match bx.as_mut() {
            Some(t) => t as *const c2x,
            None => core::ptr::null(),
        };
        // Sentinels: if the C skips the write, both sides must skip it too.
        let mut oa = c2v { x: -1.5, y: 2.5 };
        let mut ob = c2v { x: 3.5, y: -4.5 };
        let mut it: c_int = -12345;
        let mut cache = o.cache;

        let oap = if o.want_a {
            &mut oa as *mut c2v
        } else {
            core::ptr::null_mut()
        };
        let obp = if o.want_b {
            &mut ob as *mut c2v
        } else {
            core::ptr::null_mut()
        };
        let itp = if o.want_iters {
            &mut it as *mut c_int
        } else {
            core::ptr::null_mut()
        };
        let cp = match cache.as_mut() {
            Some(c) => c as *mut c2GJKCache,
            None => core::ptr::null_mut(),
        };

        let dist = with_shape(sa, |pa| {
            with_shape(sb, |pb| {
                (x.c2GJK)(
                    pa,
                    sa.ty(),
                    axp,
                    pb,
                    sb.ty(),
                    bxp,
                    oap,
                    obp,
                    o.use_radius,
                    itp,
                    cp,
                )
            })
        });

        GjkOut {
            dist,
            a: if o.want_a { Some(oa) } else { None },
            b: if o.want_b { Some(ob) } else { None },
            iters: if o.want_iters { Some(it) } else { None },
            cache,
        }
    }
}

/// Drives `n` randomized `c2GJK` cases for one CONFIGS row.
pub fn gjk_row(
    p: &Pair,
    tag: &str,
    seed: u64,
    n: usize,
    mut make: impl FnMut(&mut Rng) -> (Shape, Shape, GjkOpts),
) {
    let mut g = Rng::new(seed);
    for i in 0..n {
        let (sa, sb, o) = make(&mut g);
        let ctx = format!("{tag} #{i} A={sa:?} B={sb:?} opts={o:?}");
        diff(&ctx, p, |x| call_gjk(x, &sa, &sb, &o));
    }
}

pub const TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
