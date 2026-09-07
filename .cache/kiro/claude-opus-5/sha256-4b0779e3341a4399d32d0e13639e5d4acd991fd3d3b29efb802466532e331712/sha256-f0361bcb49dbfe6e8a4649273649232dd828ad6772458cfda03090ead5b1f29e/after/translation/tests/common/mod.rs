//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! matching symbol pairs. The Rust crate is NEVER called directly — every call
//! goes through the `cdylib`'s exported symbols, exactly like an external C
//! caller, so the `#[no_mangle]`/`extern "C"` wrappers are under test too.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI-compatible type mirrors (must match c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: i32,
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = repo_root().join("c_src/build");
    let mut found: Option<PathBuf> = None;
    for e in std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    }) {
        let p = e.expect("dir entry").path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    // Allow the verification script to point the harness at a specific build
    // (release vs debug) so the same tests run against every configuration.
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO_PATH={} does not exist", p.display());
        return p;
    }
    let root = repo_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libreverse_collide_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libreverse_collide_lib.so not found under {}. Run `cargo build --release` first.",
        root.display()
    )
}

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        // `c_src/CMakeLists.txt` has no `target_link_libraries`, so the C `.so`
        // leaves `sqrtf` (used by `c2Len`) undefined and expects the process's
        // global scope to supply it. A Rust test binary does not necessarily pull
        // in libm (LLVM lowers `f32::sqrt` to the `sqrtss` instruction), so load
        // libm with RTLD_GLOBAL first. Test-harness-only; c_src is untouched.
        for cand in ["libm.so.6", "libm.so"] {
            let lib = libloading::os::unix::Library::open(
                Some(cand),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
            );
            if lib.is_ok() {
                // Leak it: it must stay resident for the whole process.
                std::mem::forget(lib);
                break;
            }
        }
        let c = Library::new(find_c_so()).expect("load C .so");
        let rs = Library::new(find_rust_so()).expect("load Rust .so");
        Libs { c, rs }
    })
}

/// Fetch the same symbol from both libraries.
pub fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    unsafe {
        let c = l
            .c
            .get::<T>(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("C .so missing symbol `{name}`: {e}"));
        let r = l
            .rs
            .get::<T>(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("Rust .so missing symbol `{name}`: {e}"));
        (c, r)
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-exact float comparison with one documented relaxation.
///
/// Requires byte-identical bits (so `+0.0 != -0.0`, and inf/finite results must
/// match exactly) EXCEPT when both sides are NaN, in which case the payload and
/// sign bit are not compared.
///
/// Why: IEEE 754 §6.2 leaves the sign and payload of a NaN *result* unspecified
/// when an operand is NaN. Which NaN survives `a + b` is decided purely by which
/// register the compiler chose as the SSE destination (`addss` returns the
/// destination operand when it is NaN). gcc at `-O0` is not even internally
/// consistent about this: in `c2Add` the destination holds `b`, in `c2Dot`'s
/// multiply it holds `a`, and in `c2Mulrv` it differs between the two `addss`
/// sites. LLVM's instcombine canonicalizes commutative operand order, so
/// `b.x + a.x` and `a.x + b.x` compile to the *same* instruction (verified: LLVM
/// identical-code-folds the two variants into one symbol). The choice therefore
/// cannot be steered from Rust source, and matching it would mean fitting to one
/// particular gcc invocation rather than translating the C.
///
/// This relaxation only applies when BOTH results are NaN. A NaN on one side and
/// a number on the other is still a hard failure, which is the semantically
/// meaningful class of divergence.
pub fn f32_bits_eq(a: f32, b: f32) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    a.to_bits() == b.to_bits()
}

pub fn v_bits_eq(a: c2v, b: c2v) -> bool {
    f32_bits_eq(a.x, b.x) && f32_bits_eq(a.y, b.y)
}

pub fn aabb_bits_eq(a: c2AABB, b: c2AABB) -> bool {
    v_bits_eq(a.min, b.min) && v_bits_eq(a.max, b.max)
}

#[macro_export]
macro_rules! assert_f32_bits {
    ($c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv) = ($c, $r);
        assert!(
            $crate::common::f32_bits_eq(cv, rv),
            "float divergence: C={cv:?} (0x{cb:08x}) vs Rust={rv:?} (0x{rb:08x}) :: {}",
            format_args!($($ctx)*), cb = cv.to_bits(), rb = rv.to_bits()
        );
    }};
}

#[macro_export]
macro_rules! assert_v_bits {
    ($c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv) = ($c, $r);
        assert!(
            $crate::common::v_bits_eq(cv, rv),
            "c2v divergence: C=({:?},{:?}) [0x{:08x},0x{:08x}] vs Rust=({:?},{:?}) \
             [0x{:08x},0x{:08x}] :: {}",
            cv.x, cv.y, cv.x.to_bits(), cv.y.to_bits(),
            rv.x, rv.y, rv.x.to_bits(), rv.y.to_bits(),
            format_args!($($ctx)*)
        );
    }};
}

/// Compare two `c2Simplex` values field by field with bit-exact floats.
pub fn simplex_bits_eq(a: &c2Simplex, b: &c2Simplex) -> bool {
    if a.count != b.count || !f32_bits_eq(a.div, b.div) {
        return false;
    }
    for i in 0..4 {
        let (x, y) = (&a.verts[i], &b.verts[i]);
        if !v_bits_eq(x.sA, y.sA)
            || !v_bits_eq(x.sB, y.sB)
            || !v_bits_eq(x.p, y.p)
            || !f32_bits_eq(x.u, y.u)
            || x.iA != y.iA
            || x.iB != y.iB
        {
            return false;
        }
    }
    true
}

pub fn proxy_bits_eq(a: &c2Proxy, b: &c2Proxy) -> bool {
    if a.count != b.count || !f32_bits_eq(a.radius, b.radius) {
        return false;
    }
    (0..8).all(|i| v_bits_eq(a.verts[i], b.verts[i]))
}

pub fn cache_bits_eq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    f32_bits_eq(a.metric, b.metric)
        && a.count == b.count
        && a.iA == b.iA
        && a.iB == b.iB
        && f32_bits_eq(a.div, b.div)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*, fixed seed -> reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
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
        lo + self.unit() * (hi - lo)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Coordinate-ish float: mostly modest values, sometimes tiny/huge/zero.
    pub fn coord(&mut self, scale: f32) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => self.range(-1e-6, 1e-6),
            3 => self.range(-scale * 1000.0, scale * 1000.0),
            _ => self.range(-scale, scale),
        }
    }
    pub fn vec(&mut self, scale: f32) -> c2v {
        c2v {
            x: self.coord(scale),
            y: self.coord(scale),
        }
    }
    /// Non-negative radius, occasionally exactly zero.
    pub fn radius(&mut self, scale: f32) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => self.range(0.0, 1e-6),
            _ => self.range(0.0, scale),
        }
    }
    pub fn circle(&mut self, scale: f32) -> c2Circle {
        c2Circle {
            p: self.vec(scale),
            r: self.radius(scale),
        }
    }
    /// AABB with `min <= max` per component (the well-formed shape).
    pub fn aabb(&mut self, scale: f32) -> c2AABB {
        let a = self.vec(scale);
        let b = self.vec(scale);
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
    /// AABB with no ordering guarantee (may be inverted).
    pub fn aabb_raw(&mut self, scale: f32) -> c2AABB {
        c2AABB {
            min: self.vec(scale),
            max: self.vec(scale),
        }
    }
    pub fn capsule(&mut self, scale: f32) -> c2Capsule {
        let a = self.vec(scale);
        // 1-in-8 degenerate capsule (a == b)
        let b = if self.below(8) == 0 { a } else { self.vec(scale) };
        c2Capsule {
            a,
            b,
            r: self.radius(scale),
        }
    }
    pub fn rot(&mut self) -> c2r {
        match self.below(4) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => {
                // arbitrary, non-normalized
                c2r {
                    c: self.range(-2.0, 2.0),
                    s: self.range(-2.0, 2.0),
                }
            }
            _ => {
                let t = self.range(-3.15, 3.15);
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
        }
    }
    pub fn xform(&mut self, scale: f32) -> c2x {
        c2x {
            p: self.vec(scale),
            r: self.rot(),
        }
    }
    pub fn sv(&mut self, scale: f32) -> c2sv {
        c2sv {
            sA: self.vec(scale),
            sB: self.vec(scale),
            p: self.vec(scale),
            u: self.range(-2.0, 2.0),
            iA: self.below(8) as i32,
            iB: self.below(8) as i32,
        }
    }
    pub fn simplex(&mut self, scale: f32, count: i32) -> c2Simplex {
        let mut s = c2Simplex {
            verts: [self.sv(scale), self.sv(scale), self.sv(scale), self.sv(scale)],
            div: match self.below(8) {
                0 => 1.0,
                _ => self.range(0.25, 4.0),
            },
            count,
        };
        // Occasionally make points coincident / collinear to hit degenerate paths.
        match self.below(10) {
            0 => {
                s.verts[1].p = s.verts[0].p;
            }
            1 => {
                s.verts[2].p = s.verts[0].p;
            }
            2 => {
                // collinear: c.p = a.p + 2*(b.p - a.p)
                s.verts[2].p = c2v {
                    x: s.verts[0].p.x + 2.0 * (s.verts[1].p.x - s.verts[0].p.x),
                    y: s.verts[0].p.y + 2.0 * (s.verts[1].p.y - s.verts[0].p.y),
                };
            }
            _ => {}
        }
        s
    }
}

/// Interesting scalar edge values used in exhaustive sweeps.
pub const EDGE_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-30,
    FLT_EPSILON,
    -FLT_EPSILON,
    FLT_EPSILON * FLT_EPSILON,
    1.0 + FLT_EPSILON,
    f32::MAX,
    f32::MIN,
    1e20,
    -1e20,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
];

/// Values with no valid `C2_TYPE` variant, passed across the FFI boundary.
pub const BAD_TYPES: &[i32] = &[
    -1,
    3,
    4,
    7,
    99,
    255,
    256,
    -128,
    1 << 16,
    i32::MAX,
    i32::MIN,
    i32::MIN + 1,
    -2147483647,
];

pub const ALL_TYPES: &[i32] = &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
