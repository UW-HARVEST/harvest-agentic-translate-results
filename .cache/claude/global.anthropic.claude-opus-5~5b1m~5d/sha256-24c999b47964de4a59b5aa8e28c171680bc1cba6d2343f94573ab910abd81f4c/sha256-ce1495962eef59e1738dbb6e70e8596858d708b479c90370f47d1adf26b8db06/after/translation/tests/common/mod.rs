//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every test
//! calls the *exported* symbol on both sides and compares the results
//! bit-for-bit.  Nothing is ever called directly on the Rust crate, so the
//! `#[no_mangle] extern "C"` wrappers and the C ABI struct passing are part of
//! what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Struct definitions — mirror c_src/src/lib.c exactly.
// Verified against gcc 11.5 / x86-64 SysV:
//   c2v 8, c2r 8, c2x 16, c2Circle 12, c2AABB 16, c2Capsule 20,
//   c2GJKCache 36 (metric@0 count@4 iA@8 iB@20 div@32),
//   c2Proxy 72 (radius@0 count@4 verts@8),
//   c2sv 36 (sA@0 sB@8 p@16 u@24 iA@28 iB@32),
//   c2Simplex 152 (a@0 b@36 c@72 d@108 div@144 count@148)
// None of these types contain padding, so raw byte comparison is meaningful.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [C2v; 8],
}

impl Default for C2Proxy {
    fn default() -> Self {
        C2Proxy {
            radius: 0.0,
            count: 0,
            verts: [C2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2sv {
    pub sA: C2v,
    pub sB: C2v,
    pub p: C2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// C spells this `c2sv a, b, c, d; float div; int count;`.  `c2GJK` walks the
/// four members with `c2sv *verts = &s.a;`, so an array is layout-identical.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Simplex {
    pub verts: [C2sv; 4],
    pub div: f32,
    pub count: i32,
}

pub const C2_TYPE_CIRCLE: u32 = 0;
pub const C2_TYPE_AABB: u32 = 1;
pub const C2_TYPE_CAPSULE: u32 = 2;

pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;
pub const FLT_MAX: f32 = 3.402_823_466_385_288_6e38;
pub const FLT_MIN: f32 = 1.175_494_35e-38;

// ---------------------------------------------------------------------------
// Function-pointer types for every exported symbol
// ---------------------------------------------------------------------------

pub type FnV2 = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnVvf = unsafe extern "C" fn(C2v, f32) -> C2v;
pub type FnVvv = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnVvvv = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
pub type FnFvv = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnFv = unsafe extern "C" fn(C2v) -> f32;
pub type FnVv = unsafe extern "C" fn(C2v) -> C2v;
pub type FnR = unsafe extern "C" fn() -> C2r;
pub type FnX = unsafe extern "C" fn() -> C2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut C2v, *mut C2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const core::ffi::c_void, u32, *mut C2Proxy);
pub type FnFSimplex = unsafe extern "C" fn(*mut C2Simplex) -> f32;
pub type FnVSimplex = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
pub type FnSimplex = unsafe extern "C" fn(*mut C2Simplex);
pub type FnMulrv = unsafe extern "C" fn(C2r, C2v) -> C2v;
pub type FnMulxv = unsafe extern "C" fn(C2x, C2v) -> C2v;
pub type FnSupport = unsafe extern "C" fn(*const C2v, i32, C2v) -> i32;
pub type FnWitness = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);
pub type FnGJK = unsafe extern "C" fn(
    *const core::ffi::c_void,
    u32,
    *const C2x,
    *const core::ffi::c_void,
    u32,
    *const C2x,
    *mut C2v,
    *mut C2v,
    i32,
    *mut i32,
    *mut C2GJKCache,
) -> f32;
pub type FnGjkCache = unsafe extern "C" fn(
    core::ffi::c_char,
    *mut C2v,
    *mut C2v,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
);

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Duo {
    pub c: libloading::Library,
    pub r: libloading::Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
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
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no C shared object found in {}\n\
             build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_RUST_SO") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    let name = "libgjk_cache_lib.so";
    // Prefer the profile the test binary itself was built with.  `cfg!` is not
    // usable here: the crate deliberately turns `debug-assertions` off in the
    // dev profile, so the profile is read off the executable's own path instead.
    let exe = std::env::current_exe().unwrap_or_default();
    let exe_s = exe.to_string_lossy().to_string();
    let (first, second) = if exe_s.contains("/release/") {
        ("release", "debug")
    } else {
        ("debug", "release")
    };
    for prof in [first, second] {
        let p = md.join("target").join(prof).join(name);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust cdylib found under {}/target/{{debug,release}}/{name}\n\
         build it with: cargo build --release",
        md.display()
    )
}

impl Duo {
    fn load() -> Duo {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        assert!(
            Path::new(&c_path).exists(),
            "C .so does not exist: {}",
            c_path.display()
        );
        let c = unsafe { libloading::Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let r = unsafe { libloading::Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", r_path.display()));
        Duo {
            c,
            r,
            c_path,
            r_path,
        }
    }
}

static DUO: OnceLock<Duo> = OnceLock::new();

pub fn duo() -> &'static Duo {
    DUO.get_or_init(Duo::load)
}

/// Resolve one symbol name in BOTH libraries and return the two raw function
/// pointers as `(c, rust)`.  Panics if either library lacks the symbol, which
/// is itself part of the parity check.
pub fn sym<T: Copy>(name: &[u8]) -> (T, T) {
    let d = duo();
    let cs: libloading::Symbol<T> = unsafe { d.c.get(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol {:?} missing from C .so {}: {e}",
            String::from_utf8_lossy(name),
            d.c_path.display()
        )
    });
    let rs: libloading::Symbol<T> = unsafe { d.r.get(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol {:?} missing from Rust .so {}: {e}",
            String::from_utf8_lossy(name),
            d.r_path.display()
        )
    });
    (*cs, *rs)
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(x: f32) -> u32 {
    x.to_bits()
}

pub fn f32_same(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn v_same(a: C2v, b: C2v) -> bool {
    f32_same(a.x, b.x) && f32_same(a.y, b.y)
}

pub fn r_same(a: C2r, b: C2r) -> bool {
    f32_same(a.c, b.c) && f32_same(a.s, b.s)
}

pub fn x_same(a: C2x, b: C2x) -> bool {
    v_same(a.p, b.p) && r_same(a.r, b.r)
}

pub fn raw<T>(t: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(t as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub fn raw_same<T>(a: &T, b: &T) -> bool {
    raw(a) == raw(b)
}

pub fn fmt_f32(x: f32) -> String {
    format!("{x:e}(0x{:08x})", x.to_bits())
}

pub fn fmt_v(v: C2v) -> String {
    format!("({}, {})", fmt_f32(v.x), fmt_f32(v.y))
}

pub fn fmt_simplex(s: &C2Simplex) -> String {
    let mut out = format!("count={} div={}", s.count, fmt_f32(s.div));
    for (i, v) in s.verts.iter().enumerate() {
        out += &format!(
            "\n  v[{i}] sA={} sB={} p={} u={} iA={} iB={}",
            fmt_v(v.sA),
            fmt_v(v.sB),
            fmt_v(v.p),
            fmt_f32(v.u),
            v.iA,
            v.iB
        );
    }
    out
}

pub fn fmt_cache(c: &C2GJKCache) -> String {
    format!(
        "metric={} count={} iA={:?} iB={:?} div={}",
        fmt_f32(c.metric),
        c.count,
        c.iA,
        c.iB,
        fmt_f32(c.div)
    )
}

pub fn fmt_bytes(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Assert two `f32`s are bit-identical.
#[track_caller]
pub fn assert_f32(c: f32, r: f32, ctx: &str) {
    assert!(
        f32_same(c, r),
        "float mismatch [{ctx}]\n  C    = {}\n  Rust = {}",
        fmt_f32(c),
        fmt_f32(r)
    );
}

#[track_caller]
pub fn assert_v(c: C2v, r: C2v, ctx: &str) {
    assert!(
        v_same(c, r),
        "c2v mismatch [{ctx}]\n  C    = {}\n  Rust = {}",
        fmt_v(c),
        fmt_v(r)
    );
}

#[track_caller]
pub fn assert_raw<T>(c: &T, r: &T, ctx: &str) {
    assert!(
        raw_same(c, r),
        "byte-image mismatch [{ctx}]\n  C    = {}\n  Rust = {}",
        fmt_bytes(raw(c)),
        fmt_bytes(raw(r))
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// Uniform in `[lo, hi)`, always finite.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + (hi - lo) * u
    }

    /// A "nice" finite value with a wide but sane exponent range.
    pub fn finite(&mut self) -> f32 {
        let m = self.range(-1.0, 1.0);
        let e = self.below(40) as i32 - 20; // 2^-20 .. 2^19
        m * (2.0f32).powi(e)
    }

    /// Completely arbitrary bit pattern — includes NaNs (both signs, random
    /// payloads), infinities, signed zeros and denormals.
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A value drawn from the special/boundary table, or a random finite one.
    pub fn spicy(&mut self) -> f32 {
        const SPECIALS: [f32; 22] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            2.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            FLT_MAX,
            -FLT_MAX,
            FLT_MIN,
            -FLT_MIN,
            FLT_EPSILON,
            -FLT_EPSILON,
            1e-45,  // smallest positive denormal
            -1e-45, // smallest negative denormal
            1e8,
            -1e8,
            1e-8,
            16_777_216.0, // 2^24, first integer that is not exactly representable+1
        ];
        match self.below(4) {
            0 => SPECIALS[self.below(SPECIALS.len() as u32) as usize],
            1 => self.any_bits(),
            _ => self.finite(),
        }
    }

    pub fn v_finite(&mut self) -> C2v {
        C2v {
            x: self.finite(),
            y: self.finite(),
        }
    }

    pub fn v_range(&mut self, lo: f32, hi: f32) -> C2v {
        C2v {
            x: self.range(lo, hi),
            y: self.range(lo, hi),
        }
    }

    pub fn v_spicy(&mut self) -> C2v {
        C2v {
            x: self.spicy(),
            y: self.spicy(),
        }
    }

    pub fn v_bits(&mut self) -> C2v {
        C2v {
            x: self.any_bits(),
            y: self.any_bits(),
        }
    }

    /// A unit rotation (cos, sin) for a random angle.
    pub fn rot_unit(&mut self) -> C2r {
        let a = self.range(-7.0, 7.0);
        C2r {
            c: a.cos(),
            s: a.sin(),
        }
    }

    pub fn rot_spicy(&mut self) -> C2r {
        match self.below(4) {
            0 => C2r { c: 1.0, s: 0.0 },
            1 => C2r {
                c: self.finite(),
                s: self.finite(),
            },
            2 => C2r {
                c: self.spicy(),
                s: self.spicy(),
            },
            _ => self.rot_unit(),
        }
    }

    pub fn x_unit(&mut self, span: f32) -> C2x {
        C2x {
            p: self.v_range(-span, span),
            r: self.rot_unit(),
        }
    }

    pub fn x_spicy(&mut self) -> C2x {
        C2x {
            p: self.v_spicy(),
            r: self.rot_spicy(),
        }
    }
}

// ---------------------------------------------------------------------------
// Shape generators
// ---------------------------------------------------------------------------

pub fn rand_circle(rng: &mut Rng, span: f32) -> C2Circle {
    C2Circle {
        p: rng.v_range(-span, span),
        r: rng.range(0.0, span * 0.5),
    }
}

pub fn rand_aabb(rng: &mut Rng, span: f32) -> C2AABB {
    let a = rng.v_range(-span, span);
    let b = rng.v_range(-span, span);
    C2AABB {
        min: C2v {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
        },
        max: C2v {
            x: a.x.max(b.x),
            y: a.y.max(b.y),
        },
    }
}

pub fn rand_capsule(rng: &mut Rng, span: f32) -> C2Capsule {
    C2Capsule {
        a: rng.v_range(-span, span),
        b: rng.v_range(-span, span),
        r: rng.range(0.0, span * 0.5),
    }
}

/// An opaque shape blob big enough for any of the three shape types, so that a
/// single buffer can be handed to `c2MakeProxy`/`c2GJK` for every `C2_TYPE`.
#[repr(C, align(4))]
#[derive(Copy, Clone)]
pub struct ShapeBlob(pub [u8; 32]);

impl ShapeBlob {
    pub fn circle(c: C2Circle) -> ShapeBlob {
        let mut b = ShapeBlob([0xAB; 32]);
        unsafe { std::ptr::write_unaligned(b.0.as_mut_ptr() as *mut C2Circle, c) };
        b
    }
    pub fn aabb(a: C2AABB) -> ShapeBlob {
        let mut b = ShapeBlob([0xAB; 32]);
        unsafe { std::ptr::write_unaligned(b.0.as_mut_ptr() as *mut C2AABB, a) };
        b
    }
    pub fn capsule(c: C2Capsule) -> ShapeBlob {
        let mut b = ShapeBlob([0xAB; 32]);
        unsafe { std::ptr::write_unaligned(b.0.as_mut_ptr() as *mut C2Capsule, c) };
        b
    }
    pub fn as_ptr(&self) -> *const core::ffi::c_void {
        self.0.as_ptr() as *const core::ffi::c_void
    }
}

/// Build a random shape of the requested type, returned as an opaque blob.
pub fn rand_shape(rng: &mut Rng, ty: u32, span: f32) -> ShapeBlob {
    match ty {
        C2_TYPE_CIRCLE => ShapeBlob::circle(rand_circle(rng, span)),
        C2_TYPE_AABB => ShapeBlob::aabb(rand_aabb(rng, span)),
        C2_TYPE_CAPSULE => ShapeBlob::capsule(rand_capsule(rng, span)),
        _ => ShapeBlob([0xAB; 32]),
    }
}

pub fn type_name(ty: u32) -> &'static str {
    match ty {
        C2_TYPE_CIRCLE => "CIRCLE",
        C2_TYPE_AABB => "AABB",
        C2_TYPE_CAPSULE => "CAPSULE",
        _ => "INVALID",
    }
}

pub const ALL_TYPES: [u32; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

/// Build a simplex whose `verts[i]` have distinctive, easily-traced contents so
/// that a mis-copied `c2sv` (only `.u` copied instead of the whole struct, say)
/// shows up immediately.
pub fn simplex_from_points(rng: &mut Rng, pts: &[C2v], count: i32, div: f32) -> C2Simplex {
    let mut s = C2Simplex {
        verts: [C2sv::default(); 4],
        div,
        count,
    };
    for i in 0..4 {
        s.verts[i] = C2sv {
            sA: rng.v_finite(),
            sB: rng.v_finite(),
            p: if i < pts.len() {
                pts[i]
            } else {
                rng.v_finite()
            },
            u: rng.finite(),
            iA: (i as i32) * 7 + 1,
            iB: (i as i32) * 11 + 2,
        };
    }
    s
}
