//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes symbol pairs.
//! Nothing here calls a Rust function directly — every call goes through the
//! `.so` export table, exactly as an external C consumer would.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::Library;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirror types (must match c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

pub type C2_TYPE = u32;
pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
pub const C2_TYPE_AABB: C2_TYPE = 1;
pub const C2_TYPE_CAPSULE: C2_TYPE = 2;

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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// C declares `c2sv a, b, c, d; float div; int count;` — layout-identical.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: i32,
}

pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;
pub const FLT_MAX: f32 = f32::MAX;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn find_c_so() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let build = root.join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|s| s == "so").unwrap_or(false) {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    assert!(
        !candidates.is_empty(),
        "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    candidates.remove(0)
}

fn find_rust_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the profile the test binary itself was built with.
    let order: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for prof in order {
        let p = manifest
            .join("target")
            .join(prof)
            .join("libreverse_collide_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libreverse_collide_lib.so not found under target/{{debug,release}}");
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        // The C .so references `sqrtf` but CMakeLists.txt does not link libm,
        // so it must be made available in the global namespace first.
        preload_libm();
        let c_path = find_c_so();
        let r_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("loading {}: {e}", c_path.display()));
        let r = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("loading {}: {e}", r_path.display()));
        Libs {
            c,
            r,
            c_path,
            r_path,
        }
    })
}

static LIBM: OnceLock<Option<Library>> = OnceLock::new();

fn preload_libm() {
    LIBM.get_or_init(|| {
        use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
        for name in ["libm.so.6", "libm.so", "libc.so.6"] {
            if let Ok(l) = unsafe { UnixLibrary::open(Some(name), RTLD_NOW | RTLD_GLOBAL) } {
                let lib: Library = l.into();
                if unsafe { lib.get::<extern "C" fn(f32) -> f32>(b"sqrtf") }.is_ok() {
                    return Some(lib);
                }
            }
        }
        None
    });
}

/// Fetch the same symbol from both libraries, typed as `T` (a fn pointer).
pub fn pair<T: Copy>(name: &str) -> (T, T) {
    let l = libs();
    let cf = unsafe {
        *l.c.get::<T>(name.as_bytes())
            .unwrap_or_else(|e| panic!("C .so missing symbol `{name}`: {e}"))
    };
    let rf = unsafe {
        *l.r.get::<T>(name.as_bytes())
            .unwrap_or_else(|e| panic!("Rust .so missing symbol `{name}`: {e}"))
    };
    (cf, rf)
}

// ---------------------------------------------------------------------------
// Bitwise comparison helpers (NO epsilons — byte-identical or bust)
// ---------------------------------------------------------------------------

pub trait Bits {
    fn bits(&self) -> Vec<u32>;
}

impl Bits for f32 {
    fn bits(&self) -> Vec<u32> {
        vec![self.to_bits()]
    }
}
impl Bits for i32 {
    fn bits(&self) -> Vec<u32> {
        vec![*self as u32]
    }
}
impl Bits for c2v {
    fn bits(&self) -> Vec<u32> {
        vec![self.x.to_bits(), self.y.to_bits()]
    }
}
impl Bits for c2r {
    fn bits(&self) -> Vec<u32> {
        vec![self.c.to_bits(), self.s.to_bits()]
    }
}
impl Bits for c2x {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.p.bits();
        v.extend(self.r.bits());
        v
    }
}
impl Bits for c2sv {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.sA.bits();
        v.extend(self.sB.bits());
        v.extend(self.p.bits());
        v.push(self.u.to_bits());
        v.push(self.iA as u32);
        v.push(self.iB as u32);
        v
    }
}
impl Bits for c2Simplex {
    fn bits(&self) -> Vec<u32> {
        let mut v = Vec::new();
        for s in &self.verts {
            v.extend(s.bits());
        }
        v.push(self.div.to_bits());
        v.push(self.count as u32);
        v
    }
}
impl Bits for c2GJKCache {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.metric.to_bits(), self.count as u32];
        v.extend(self.iA.iter().map(|x| *x as u32));
        v.extend(self.iB.iter().map(|x| *x as u32));
        v.push(self.div.to_bits());
        v
    }
}
impl Bits for c2Proxy {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.radius.to_bits(), self.count as u32];
        for p in &self.verts {
            v.extend(p.bits());
        }
        v
    }
}
impl<T: Bits> Bits for Option<T> {
    fn bits(&self) -> Vec<u32> {
        match self {
            None => vec![0xDEAD_BEEF],
            Some(t) => {
                let mut v = vec![0x0000_0001];
                v.extend(t.bits());
                v
            }
        }
    }
}
impl<A: Bits, B: Bits> Bits for (A, B) {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.0.bits();
        v.extend(self.1.bits());
        v
    }
}
impl<A: Bits, B: Bits, C: Bits> Bits for (A, B, C) {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.0.bits();
        v.extend(self.1.bits());
        v.extend(self.2.bits());
        v
    }
}
impl<A: Bits, B: Bits, C: Bits, D: Bits> Bits for (A, B, C, D) {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.0.bits();
        v.extend(self.1.bits());
        v.extend(self.2.bits());
        v.extend(self.3.bits());
        v
    }
}
impl<T: Bits> Bits for Vec<T> {
    fn bits(&self) -> Vec<u32> {
        let mut v = Vec::new();
        for t in self {
            v.extend(t.bits());
        }
        v
    }
}
impl<T: Bits, const N: usize> Bits for [T; N] {
    fn bits(&self) -> Vec<u32> {
        let mut v = Vec::new();
        for t in self {
            v.extend(t.bits());
        }
        v
    }
}

#[track_caller]
pub fn same<T: Bits + std::fmt::Debug>(what: &str, c: T, r: T) {
    let cb = c.bits();
    let rb = r.bits();
    assert_eq!(
        cb, rb,
        "\nDIVERGENCE in {what}\n  C    = {c:?}\n  Rust = {r:?}\n  C bits    = {cb:08x?}\n  Rust bits = {rb:08x?}\n"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo, hi].
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// A "nasty" float: mostly ordinary magnitudes, occasionally exact
    /// boundary values, subnormals, huge values and signed zeros.
    pub fn nasty(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => -1.0,
            4 => FLT_EPSILON,
            5 => -FLT_EPSILON,
            6 => f32::MIN_POSITIVE,
            7 => f32::from_bits(1), // smallest subnormal
            8 => 1e20,
            9 => -1e20,
            10 => f32::MAX,
            11 => f32::MIN,
            12 => self.range(-1.0, 1.0),
            13 => self.range(-1e-18, 1e-18),
            14 => self.range(-1e18, 1e18),
            _ => self.range(-200.0, 200.0),
        }
    }

    /// Ordinary geometry-scale coordinate.
    pub fn coord(&mut self) -> f32 {
        self.range(-100.0, 100.0)
    }
    pub fn vec(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn nasty_vec(&mut self) -> c2v {
        c2v {
            x: self.nasty(),
            y: self.nasty(),
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.vec(),
            r: self.range(0.0, 30.0),
        }
    }
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.vec();
        let w = self.range(0.0, 40.0);
        let h = self.range(0.0, 40.0);
        c2AABB {
            min: a,
            max: c2v {
                x: a.x + w,
                y: a.y + h,
            },
        }
    }
    pub fn capsule(&mut self) -> c2Capsule {
        c2Capsule {
            a: self.vec(),
            b: self.vec(),
            r: self.range(0.0, 25.0),
        }
    }
    /// Rotor: identity, normalised, or deliberately unnormalised.
    pub fn rot(&mut self) -> c2r {
        match self.below(4) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => {
                let t = self.range(-6.3, 6.3);
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
            2 => c2r {
                c: self.range(-3.0, 3.0),
                s: self.range(-3.0, 3.0),
            },
            _ => c2r { c: 0.0, s: 0.0 },
        }
    }
    pub fn xform(&mut self) -> c2x {
        c2x {
            p: self.vec(),
            r: self.rot(),
        }
    }
    pub fn sv(&mut self) -> c2sv {
        c2sv {
            sA: self.vec(),
            sB: self.vec(),
            p: self.vec(),
            u: self.range(-2.0, 2.0),
            iA: self.below(8) as i32,
            iB: self.below(8) as i32,
        }
    }
    pub fn simplex(&mut self, count: i32) -> c2Simplex {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i] = self.sv();
        }
        s.div = self.range(-3.0, 3.0);
        s.count = count;
        s
    }
}

// ---------------------------------------------------------------------------
// Shape blobs: pass a shape through the `const void*` parameter of
// c2GJK / c2Collided.  Sized to the largest shape so an out-of-range enum
// never reads past the allocation.
// ---------------------------------------------------------------------------

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct Blob(pub [u8; 32]);

impl Blob {
    pub fn of_circle(c: c2Circle) -> Blob {
        let mut b = Blob([0u8; 32]);
        unsafe {
            std::ptr::write_unaligned(b.0.as_mut_ptr() as *mut c2Circle, c);
        }
        b
    }
    pub fn of_aabb(a: c2AABB) -> Blob {
        let mut b = Blob([0u8; 32]);
        unsafe {
            std::ptr::write_unaligned(b.0.as_mut_ptr() as *mut c2AABB, a);
        }
        b
    }
    pub fn of_capsule(c: c2Capsule) -> Blob {
        let mut b = Blob([0u8; 32]);
        unsafe {
            std::ptr::write_unaligned(b.0.as_mut_ptr() as *mut c2Capsule, c);
        }
        b
    }
    pub fn ptr(&self) -> *const std::ffi::c_void {
        self.0.as_ptr() as *const std::ffi::c_void
    }
}

/// One randomized shape of the given type, as a blob.
pub fn rand_shape(rng: &mut Rng, ty: C2_TYPE) -> Blob {
    match ty {
        C2_TYPE_CIRCLE => Blob::of_circle(rng.circle()),
        C2_TYPE_AABB => Blob::of_aabb(rng.aabb()),
        _ => Blob::of_capsule(rng.capsule()),
    }
}

pub const ALL_TYPES: [C2_TYPE; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// Enum values with no valid variant — a C enum accepts any `int`.
pub const BAD_TYPES: [C2_TYPE; 8] = [3, 4, 7, 100, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0xdead_beef];

// ---------------------------------------------------------------------------
// Typed fn-pointer aliases for every exported symbol
// ---------------------------------------------------------------------------

pub type FnV_ff = extern "C" fn(f32, f32) -> c2v;
pub type FnV_vf = extern "C" fn(c2v, f32) -> c2v;
pub type FnV_vv = extern "C" fn(c2v, c2v) -> c2v;
pub type FnV_vvv = extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnV_v = extern "C" fn(c2v) -> c2v;
pub type FnF_vv = extern "C" fn(c2v, c2v) -> f32;
pub type FnF_v = extern "C" fn(c2v) -> f32;
pub type FnR_void = extern "C" fn() -> c2r;
pub type FnX_void = extern "C" fn() -> c2x;
pub type FnV_rv = extern "C" fn(c2r, c2v) -> c2v;
pub type FnV_xv = extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const std::ffi::c_void, C2_TYPE, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSupport = unsafe extern "C" fn(*const c2v, i32, c2v) -> i32;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
#[allow(clippy::type_complexity)]
pub type FnGJK = unsafe extern "C" fn(
    *const std::ffi::c_void,
    C2_TYPE,
    *const c2x,
    *const std::ffi::c_void,
    C2_TYPE,
    *const c2x,
    *mut c2v,
    *mut c2v,
    i32,
    *mut i32,
    *mut c2GJKCache,
) -> f32;
pub type FnI_AABB_AABB = extern "C" fn(c2AABB, c2AABB) -> i32;
pub type FnI_AABB_Cap = extern "C" fn(c2AABB, c2Capsule) -> i32;
pub type FnI_Cap_Cap = extern "C" fn(c2Capsule, c2Capsule) -> i32;
pub type FnI_Cir_Cir = extern "C" fn(c2Circle, c2Circle) -> i32;
pub type FnI_Cir_AABB = extern "C" fn(c2Circle, c2AABB) -> i32;
pub type FnI_Cir_Cap = extern "C" fn(c2Circle, c2Capsule) -> i32;
pub type FnCollided = unsafe extern "C" fn(
    *const std::ffi::c_void,
    C2_TYPE,
    *const std::ffi::c_void,
    C2_TYPE,
) -> i32;
pub type FnReverseCollide = extern "C" fn(f32, f32, f32) -> i32;

// ---------------------------------------------------------------------------
// Convenience wrappers around c2GJK that return the full observable result.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: i32,
    pub cache: Option<c2GJKCache>,
}

impl Bits for GjkOut {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.dist.to_bits()];
        v.extend(self.a.bits());
        v.extend(self.b.bits());
        v.push(self.iters as u32);
        v.extend(self.cache.bits());
        v
    }
}

/// Call `c2GJK` capturing every output. `cache_in` of `None` passes NULL.
#[allow(clippy::too_many_arguments)]
pub fn call_gjk(
    f: FnGJK,
    a: &Blob,
    ta: C2_TYPE,
    ax: Option<&c2x>,
    b: &Blob,
    tb: C2_TYPE,
    bx: Option<&c2x>,
    use_radius: i32,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    let mut oa = c2v {
        x: f32::from_bits(0xCAFE_BABE),
        y: f32::from_bits(0xCAFE_BABE),
    };
    let mut ob = oa;
    let mut it: i32 = -12345;
    let mut cache = cache_in;
    let cache_ptr = match cache.as_mut() {
        Some(c) => c as *mut c2GJKCache,
        None => std::ptr::null_mut(),
    };
    let dist = unsafe {
        f(
            a.ptr(),
            ta,
            ax.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            b.ptr(),
            tb,
            bx.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            &mut oa,
            &mut ob,
            use_radius,
            &mut it,
            cache_ptr,
        )
    };
    GjkOut {
        dist,
        a: oa,
        b: ob,
        iters: it,
        cache,
    }
}

pub fn gjk_pair() -> (FnGJK, FnGJK) {
    pair::<FnGJK>("c2GJK")
}
