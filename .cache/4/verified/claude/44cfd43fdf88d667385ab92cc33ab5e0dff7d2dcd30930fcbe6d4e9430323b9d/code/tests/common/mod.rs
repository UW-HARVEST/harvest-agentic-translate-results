//! Differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` (`dlopen`/`dlsym`) and
//! exposes their exported symbols as raw `extern "C"` function pointers.  The
//! Rust implementation is *never* called directly — it is always reached
//! through `target/<profile>/libreverse_collide_lib.so`, exactly like an
//! external C consumer would, so the `#[no_mangle]` wrappers and the C ABI of
//! every struct-by-value parameter are part of what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI-compatible struct definitions (mirrors of the C typedefs)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
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
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
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

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
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

pub const FLT_MAX: f32 = 3.402_823_5e38;
pub const FLT_MIN_POS: f32 = 1.175_494_4e-38;
pub const FLT_EPSILON: f32 = 1.192_092_9e-7;

// ---------------------------------------------------------------------------
// The loaded API
// ---------------------------------------------------------------------------

macro_rules! define_api {
    ($( $name:ident : $ty:ty ),* $(,)?) => {
        pub struct Api {
            pub tag: &'static str,
            $( pub $name : $ty, )*
            _lib: ::libloading::Library,
        }

        impl Api {
            pub fn load(tag: &'static str, path: &PathBuf) -> Api {
                unsafe {
                    let lib = ::libloading::Library::new(path).unwrap_or_else(|e| {
                        panic!("dlopen({}) failed: {e}", path.display())
                    });
                    $(
                        let $name : $ty = *lib
                            .get::<$ty>(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| {
                                panic!("dlsym({}, {}) failed: {e}",
                                       path.display(), stringify!($name))
                            });
                    )*
                    Api { tag, $( $name, )* _lib: lib }
                }
            }

            /// Every symbol name this harness resolves (used by the symbol-parity test).
            pub const SYMBOLS: &'static [&'static str] = &[ $( stringify!($name), )* ];
        }
    };
}

define_api! {
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
    c2GJK: unsafe extern "C" fn(
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
    ) -> f32,
    c2AABBtoAABB: unsafe extern "C" fn(c2AABB, c2AABB) -> c_int,
    c2AABBtoCapsule: unsafe extern "C" fn(c2AABB, c2Capsule) -> c_int,
    c2CapsuletoCapsule: unsafe extern "C" fn(c2Capsule, c2Capsule) -> c_int,
    c2CircletoCircle: unsafe extern "C" fn(c2Circle, c2Circle) -> c_int,
    c2CircletoAABB: unsafe extern "C" fn(c2Circle, c2AABB) -> c_int,
    c2CircletoCapsule: unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int,
    c2Collided: unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
    reverse_collide: unsafe extern "C" fn(f32, f32, f32) -> c_int,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

pub fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target dir")
        .to_path_buf();
    dir.join("libreverse_collide_lib.so")
}

static PAIR: OnceLock<(Api, Api)> = OnceLock::new();

/// `c_src/CMakeLists.txt` never links `-lm`, so `libtranslated_rust.so` has an
/// unresolved `sqrtf` that the (Rust) test executable does not pull in either.
/// Publish libm into the global lookup scope before `dlopen`-ing it.  This is a
/// property of the *test harness*, not of the library under test, so nothing in
/// `c_src/` needs to change.
fn preload_libm() {
    use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        for name in ["libm.so.6", "libm.so"] {
            if let Ok(lib) = unsafe { Library::open(Some(name), RTLD_NOW | RTLD_GLOBAL) } {
                std::mem::forget(lib); // keep it resident for the whole process
                return;
            }
        }
        // glibc >= 2.34 folds libm into libc; in that case sqrtf is already in
        // the global scope and nothing needs to be done.
    });
}

/// Refuse to test a shared object that is older than its source: a stale
/// artifact would silently "pass" and hide a real divergence.
fn assert_fresh(so: &std::path::Path, src_rel: &str) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(src_rel);
    let mtime = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    assert!(
        mtime(so) >= mtime(&src),
        "{} is OLDER than {} — rebuild before testing (stale artifacts hide \
         divergences)",
        so.display(),
        src.display()
    );
}

/// `(c, rust)` — both libraries, loaded once per test binary.
pub fn libs() -> &'static (Api, Api) {
    PAIR.get_or_init(|| {
        preload_libm();
        let c = c_so_path();
        let r = rust_so_path();
        assert!(
            c.exists(),
            "C shared library not built: {} (run cmake in c_src/build)",
            c.display()
        );
        assert!(
            r.exists(),
            "Rust cdylib not built: {} — `cargo test` alone does NOT build a \
             cdylib-only lib target; run `cargo build` (same profile/features) \
             first, or use ./verify_all_features.sh",
            r.display()
        );
        assert_fresh(&r, "src/lib.rs");
        assert_fresh(&c, "c_src/src/lib.c");
        (Api::load("C", &c), Api::load("RUST", &r))
    })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64* — fixed seed, fully reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Splitmix the seed so that adjacent seeds give unrelated streams, and
        // guarantee the xorshift state is never 0.
        let mut z = seed
            .wrapping_add(0x9E37_79B9_7F4A_7C15)
            .wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z ^= z >> 27;
        z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Rng(if z == 0 { 0x853C_49E6_748F_EA9B } else { z })
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

    /// Uniform in `[lo, hi]`.
    pub fn uniform(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u32() >> 8) as f32 / ((1u32 << 24) as f32);
        lo + (hi - lo) * t
    }

    /// A "nice" coordinate: uniform in `[-range, range]`, occasionally an
    /// exact small integer or exact zero so that ties / `==` branches fire.
    pub fn coord(&mut self, range: f32) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => -0.0,
            2 => (self.next_u32() % 21) as f32 - 10.0,
            _ => self.uniform(-range, range),
        }
    }

    /// A radius-like value: mostly positive, sometimes exactly 0 or negative.
    pub fn radius(&mut self, range: f32) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => -self.uniform(0.0, range),
            2 => (self.next_u32() % 11) as f32,
            _ => self.uniform(0.0, range),
        }
    }

    /// A value drawn from the full IEEE-754 `f32` space, biased towards the
    /// interesting classes (zeros, denormals, huge, inf, NaN).
    pub fn wild_f32(&mut self) -> f32 {
        const SPECIALS: [f32; 16] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            FLT_MIN_POS,
            -FLT_MIN_POS,
            1.0e-45, // smallest denormal
            -1.0e-45,
            FLT_MAX,
            -FLT_MAX,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            FLT_EPSILON,
            1.0e30,
        ];
        match self.below(4) {
            0 => SPECIALS[self.below(16) as usize],
            1 => f32::from_bits(self.next_u32()),
            2 => self.uniform(-1.0e18, 1.0e18),
            _ => self.uniform(-1000.0, 1000.0),
        }
    }

    pub fn wild_v(&mut self) -> c2v {
        c2v {
            x: self.wild_f32(),
            y: self.wild_f32(),
        }
    }

    pub fn v(&mut self, range: f32) -> c2v {
        c2v {
            x: self.coord(range),
            y: self.coord(range),
        }
    }

    pub fn rot(&mut self) -> c2r {
        match self.below(6) {
            0 => c2r { c: 1.0, s: 0.0 }, // identity
            1 => c2r {
                c: self.wild_f32(),
                s: self.wild_f32(),
            },
            2 => c2r {
                c: self.uniform(-3.0, 3.0),
                s: self.uniform(-3.0, 3.0),
            }, // unnormalized
            _ => {
                let a = self.uniform(-3.15, 3.15);
                c2r {
                    c: a.cos(),
                    s: a.sin(),
                }
            }
        }
    }

    pub fn xform(&mut self, range: f32) -> c2x {
        c2x {
            p: self.v(range),
            r: self.rot(),
        }
    }

    pub fn circle(&mut self, range: f32) -> c2Circle {
        c2Circle {
            p: self.v(range),
            r: self.radius(range * 0.5),
        }
    }

    pub fn aabb(&mut self, range: f32) -> c2AABB {
        let a = self.v(range);
        let b = self.v(range);
        match self.below(6) {
            0 => c2AABB { min: a, max: a }, // zero extent
            1 => c2AABB { min: b, max: a }, // possibly inverted
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

    pub fn capsule(&mut self, range: f32) -> c2Capsule {
        let a = self.v(range);
        let b = if self.below(6) == 0 { a } else { self.v(range) };
        c2Capsule {
            a,
            b,
            r: self.radius(range * 0.5),
        }
    }

    /// A fully random simplex: every byte (including `u`, `iA`, `iB` and the
    /// unused 4th vertex) is randomized so that a byte-wise comparison of the
    /// mutated struct is meaningful.
    pub fn simplex(&mut self, count: c_int, range: f32) -> c2Simplex {
        let mut s = c2Simplex::default();
        for v in s.verts.iter_mut() {
            v.sA = self.v(range);
            v.sB = self.v(range);
            v.p = self.v(range);
            v.u = self.coord(range);
            v.iA = (self.next_u32() % 8) as c_int;
            v.iB = (self.next_u32() % 8) as c_int;
        }
        s.div = match self.below(6) {
            0 => 1.0,
            1 => 0.0,
            2 => self.uniform(-100.0, 100.0),
            _ => self.uniform(0.001, 100.0),
        };
        s.count = count;
        s
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(v: f32) -> u32 {
    v.to_bits()
}

#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    assert!(
        c.to_bits() == r.to_bits(),
        "{ctx}: float mismatch  C={c:?} (0x{:08x})  RUST={r:?} (0x{:08x})",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_v(ctx: &str, c: c2v, r: c2v) {
    eq_f32(&format!("{ctx}.x"), c.x, r.x);
    eq_f32(&format!("{ctx}.y"), c.y, r.y);
}

#[track_caller]
pub fn eq_r(ctx: &str, c: c2r, r: c2r) {
    eq_f32(&format!("{ctx}.c"), c.c, r.c);
    eq_f32(&format!("{ctx}.s"), c.s, r.s);
}

#[track_caller]
pub fn eq_x(ctx: &str, c: c2x, r: c2x) {
    eq_v(&format!("{ctx}.p"), c.p, r.p);
    eq_r(&format!("{ctx}.r"), c.r, r.r);
}

#[track_caller]
pub fn eq_int(ctx: &str, c: c_int, r: c_int) {
    assert!(c == r, "{ctx}: int mismatch  C={c}  RUST={r}");
}

/// Byte-for-byte comparison of any POD value (all the structs here are
/// padding-free: every member is 4-byte sized and 4-byte aligned).
#[track_caller]
pub fn eq_bytes<T: Copy>(ctx: &str, c: &T, r: &T) {
    let n = std::mem::size_of::<T>();
    let cb = unsafe { std::slice::from_raw_parts(c as *const T as *const u8, n) };
    let rb = unsafe { std::slice::from_raw_parts(r as *const T as *const u8, n) };
    if cb != rb {
        let first = cb.iter().zip(rb).position(|(a, b)| a != b).unwrap();
        panic!(
            "{ctx}: {n}-byte struct mismatch at offset {first}\n  C   = {:02x?}\n  RUST= {:02x?}",
            cb, rb
        );
    }
}

#[track_caller]
pub fn eq_simplex(ctx: &str, c: &c2Simplex, r: &c2Simplex) {
    eq_bytes(ctx, c, r);
}

#[track_caller]
pub fn eq_proxy(ctx: &str, c: &c2Proxy, r: &c2Proxy) {
    eq_bytes(ctx, c, r);
}

#[track_caller]
pub fn eq_cache(ctx: &str, c: &c2GJKCache, r: &c2GJKCache) {
    eq_bytes(ctx, c, r);
}

/// The boundary grid used for exhaustive small cross-products.
pub const GRID: [f32; 12] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    FLT_MIN_POS,
    1.0e-45,
    FLT_EPSILON,
    FLT_MAX,
    -FLT_MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

/// Out-of-range `C2_TYPE` values that a C caller can legally pass (a C enum
/// has the range of `int`).
pub const BAD_TYPES: [c_int; 8] = [3, 4, 5, -1, -2, 100, c_int::MIN, c_int::MAX];
