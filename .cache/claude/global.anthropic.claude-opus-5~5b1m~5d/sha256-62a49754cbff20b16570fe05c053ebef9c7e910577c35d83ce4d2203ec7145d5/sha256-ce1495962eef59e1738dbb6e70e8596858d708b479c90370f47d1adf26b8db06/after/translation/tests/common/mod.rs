//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes every one of the
//! 39 exported symbols as a plain `extern "C"` function pointer, so the Rust
//! implementation is always reached through its `#[no_mangle]` export wrapper
//! exactly like an external C consumer would reach it. Nothing in this file
//! links against the crate directly.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C-ABI types (mirrors of the ones in c_src/src/lib.c)
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
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
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

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;

/// The three valid `C2_TYPE` values, in enum-declaration order.
pub const VALID_TYPES: [c_int; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

/// `int`s that are *not* a valid `C2_TYPE` variant but are perfectly legal to
/// pass through the FFI boundary (C enums accept any `int`).
pub const INVALID_TYPES: [c_int; 10] = [
    3,
    4,
    -1,
    -2,
    255,
    256,
    1 << 16,
    -(1 << 16),
    c_int::MAX,
    c_int::MIN,
];

pub const FLT_EPSILON: f32 = 1.192_092_9e-7;
pub const FLT_MAX: f32 = f32::MAX;

// ---------------------------------------------------------------------------
// The loaded API surface
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,

    pub c2V: extern "C" fn(f32, f32) -> c2v,
    pub c2Mulvs: extern "C" fn(c2v, f32) -> c2v,
    pub c2Maxv: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Minv: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Clampv: extern "C" fn(c2v, c2v, c2v) -> c2v,
    pub c2Sub: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Dot: extern "C" fn(c2v, c2v) -> f32,
    pub c2RotIdentity: extern "C" fn() -> c2r,
    pub c2xIdentity: extern "C" fn() -> c2x,
    pub c2BBVerts: unsafe extern "C" fn(*mut c2v, *mut c2AABB),
    pub c2MakeProxy: unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy),
    pub c2Len: extern "C" fn(c2v) -> f32,
    pub c2Det2: extern "C" fn(c2v, c2v) -> f32,
    pub c2GJKSimplexMetric: unsafe extern "C" fn(*mut c2Simplex) -> f32,
    pub c2Mulrv: extern "C" fn(c2r, c2v) -> c2v,
    pub c2Add: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Mulxv: extern "C" fn(c2x, c2v) -> c2v,
    pub c22: unsafe extern "C" fn(*mut c2Simplex),
    pub c23: unsafe extern "C" fn(*mut c2Simplex),
    pub c2Neg: extern "C" fn(c2v) -> c2v,
    pub c2Skew: extern "C" fn(c2v) -> c2v,
    pub c2CCW90: extern "C" fn(c2v) -> c2v,
    pub c2D: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    pub c2Support: unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int,
    pub c2Witness: unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v),
    pub c2Div: extern "C" fn(c2v, f32) -> c2v,
    pub c2Norm: extern "C" fn(c2v) -> c2v,
    pub c2L: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    pub c2MulrvT: extern "C" fn(c2r, c2v) -> c2v,
    #[allow(clippy::type_complexity)]
    pub c2GJK: unsafe extern "C" fn(
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
    pub c2AABBtoAABB: extern "C" fn(c2AABB, c2AABB) -> c_int,
    pub c2AABBtoCapsule: extern "C" fn(c2AABB, c2Capsule) -> c_int,
    pub c2CapsuletoCapsule: extern "C" fn(c2Capsule, c2Capsule) -> c_int,
    pub c2CircletoCircle: extern "C" fn(c2Circle, c2Circle) -> c_int,
    pub c2CircletoAABB: extern "C" fn(c2Circle, c2AABB) -> c_int,
    pub c2CircletoCapsule: extern "C" fn(c2Circle, c2Capsule) -> c_int,
    pub c2Collided: unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
    pub ptr_from_parts: unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void,
    #[allow(clippy::type_complexity)]
    pub omni_collide: unsafe extern "C" fn(
        c_int,
        f32,
        f32,
        f32,
        f32,
        f32,
        c_int,
        f32,
        f32,
        f32,
        f32,
        f32,
    ) -> c_int,
}

// Every field is a bare `fn` pointer, which is `Send + Sync`.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        // Leaked on purpose: the function pointers we hand out must stay valid
        // for the whole test-binary lifetime.
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()))
        }));

        macro_rules! g {
            ($sym:literal) => {{
                let s: libloading::Symbol<'static, _> = unsafe {
                    lib.get(concat!($sym, "\0").as_bytes()).unwrap_or_else(|e| {
                        panic!("{} is missing symbol {}: {e}", stringify!($sym), $sym)
                    })
                };
                *s
            }};
        }

        Api {
            name,
            path: path.to_path_buf(),
            c2V: g!("c2V"),
            c2Mulvs: g!("c2Mulvs"),
            c2Maxv: g!("c2Maxv"),
            c2Minv: g!("c2Minv"),
            c2Clampv: g!("c2Clampv"),
            c2Sub: g!("c2Sub"),
            c2Dot: g!("c2Dot"),
            c2RotIdentity: g!("c2RotIdentity"),
            c2xIdentity: g!("c2xIdentity"),
            c2BBVerts: g!("c2BBVerts"),
            c2MakeProxy: g!("c2MakeProxy"),
            c2Len: g!("c2Len"),
            c2Det2: g!("c2Det2"),
            c2GJKSimplexMetric: g!("c2GJKSimplexMetric"),
            c2Mulrv: g!("c2Mulrv"),
            c2Add: g!("c2Add"),
            c2Mulxv: g!("c2Mulxv"),
            c22: g!("c22"),
            c23: g!("c23"),
            c2Neg: g!("c2Neg"),
            c2Skew: g!("c2Skew"),
            c2CCW90: g!("c2CCW90"),
            c2D: g!("c2D"),
            c2Support: g!("c2Support"),
            c2Witness: g!("c2Witness"),
            c2Div: g!("c2Div"),
            c2Norm: g!("c2Norm"),
            c2L: g!("c2L"),
            c2MulrvT: g!("c2MulrvT"),
            c2GJK: g!("c2GJK"),
            c2AABBtoAABB: g!("c2AABBtoAABB"),
            c2AABBtoCapsule: g!("c2AABBtoCapsule"),
            c2CapsuletoCapsule: g!("c2CapsuletoCapsule"),
            c2CircletoCircle: g!("c2CircletoCircle"),
            c2CircletoAABB: g!("c2CircletoAABB"),
            c2CircletoCapsule: g!("c2CircletoCapsule"),
            c2Collided: g!("c2Collided"),
            ptr_from_parts: g!("ptr_from_parts"),
            omni_collide: g!("omni_collide"),
        }
    }
}

/// One (C, Rust) pair to run every differential assertion against.
pub struct Pair {
    pub label: String,
    pub c: Api,
    pub r: Api,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so` name is derived from the *parent* directory name by
/// `c_src/CMakeLists.txt`, so glob instead of hard-coding it.
fn find_c_so() -> PathBuf {
    // `$C_SO` overrides the search. Used to point the suite at a
    // coverage-instrumented (`--coverage`) build of the same C sources, so the
    // C's own line/branch coverage under this test suite can be measured.
    if let Some(p) = std::env::var_os("C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "$C_SO does not exist: {}", p.display());
        return p;
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") && p.is_file() {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.into_iter().next() {
        Some(p) => p,
        None => panic!(
            "no C shared library found in {}.\n\
             Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        ),
    }
}

fn find_rust_sos() -> Vec<(&'static str, PathBuf)> {
    let md = manifest_dir();
    let mut out = Vec::new();
    for profile in ["debug", "release"] {
        let p = md.join("target").join(profile).join("libomni_collide_lib.so");
        if p.is_file() {
            let name: &'static str = if profile == "debug" {
                "rust(debug)"
            } else {
                "rust(release)"
            };
            out.push((name, p));
        }
    }
    if out.is_empty() {
        panic!(
            "no Rust cdylib found under {}/target/{{debug,release}}/libomni_collide_lib.so.\n\
             Build it first: cargo build && cargo build --release",
            md.display()
        );
    }
    out
}

static PAIRS: OnceLock<Vec<Pair>> = OnceLock::new();

/// Every (C, Rust) pair to compare. Contains one entry per Rust `.so` found
/// (debug and/or release), so optimisation-dependent divergence is caught too.
pub fn pairs() -> &'static Vec<Pair> {
    PAIRS.get_or_init(|| {
        let c_so = find_c_so();
        find_rust_sos()
            .into_iter()
            .map(|(name, path)| Pair {
                label: format!("C({}) vs {}", c_so.display(), name),
                c: Api::load("c", &c_so),
                r: Api::load(name, &path),
            })
            .collect()
    })
}

/// Run `f` for every (C, Rust) pair.
pub fn for_each_pair(mut f: impl FnMut(&Api, &Api, &str)) {
    for p in pairs() {
        f(&p.c, &p.r, &p.label);
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Strict bit equality for `f32`, except that two NaNs of *any* payload compare
/// equal. Everything else -- including the `+0.0` / `-0.0` distinction,
/// infinities and subnormals -- must match exactly, bit for bit.
///
/// # Why NaN payloads are exempt
///
/// On x86-64, `mulss`/`addss` propagate whichever *source operand* the compiler
/// placed first when both are NaN. The C source writes `a.x * b.x`, and the
/// operand order the back end picks for that is not part of the language
/// semantics, so GCC -O0 and LLVM -O0/-O3 can legitimately disagree about the
/// NaN sign bit. Requiring identical payloads would be testing the register
/// allocator, not the translation.
///
/// Set `STRICT_NAN_BITS=1` to disable the exemption and see exactly which cases
/// rely on it. As of this verification run the answer is:
///
/// | test | case | C | Rust |
/// |------|------|---|------|
/// | `row04_binary_extreme` | `c2Dot((NaN⁺,NaN⁻),(-0.5,-FLT_MAX))` | `0xffc00000` | `0x7fc00000` |
/// | `row06_scale_extreme`  | `c2Mulvs((-inf,NaN⁻), NaN⁺)`         | `0xffc00000` | `0x7fc00000` |
/// | `row13_len_norm_extreme` | `c2Len((NaN⁻,NaN⁺))`               | `0xffc00000` | `0x7fc00000` |
///
/// i.e. **only** the NaN sign bit, and **only** when at least two operands are
/// already NaN. Every other input -- 500k+ comparisons including `±0.0`,
/// subnormals, `±inf` and `±FLT_MAX` -- matches bit-exactly.
pub fn f32_same(a: f32, b: f32) -> bool {
    if a.is_nan() && b.is_nan() && std::env::var_os("STRICT_NAN_BITS").is_none() {
        return true;
    }
    a.to_bits() == b.to_bits()
}

pub fn v_same(a: c2v, b: c2v) -> bool {
    f32_same(a.x, b.x) && f32_same(a.y, b.y)
}

pub fn r_same(a: c2r, b: c2r) -> bool {
    f32_same(a.c, b.c) && f32_same(a.s, b.s)
}

pub fn x_same(a: c2x, b: c2x) -> bool {
    v_same(a.p, b.p) && r_same(a.r, b.r)
}

pub fn sv_same(a: &c2sv, b: &c2sv) -> bool {
    v_same(a.sA, b.sA)
        && v_same(a.sB, b.sB)
        && v_same(a.p, b.p)
        && f32_same(a.u, b.u)
        && a.iA == b.iA
        && a.iB == b.iB
}

pub fn simplex_same(a: &c2Simplex, b: &c2Simplex) -> bool {
    (0..4).all(|i| sv_same(&a.verts[i], &b.verts[i]))
        && f32_same(a.div, b.div)
        && a.count == b.count
}

pub fn proxy_same(a: &c2Proxy, b: &c2Proxy) -> bool {
    f32_same(a.radius, b.radius)
        && a.count == b.count
        && (0..8).all(|i| v_same(a.verts[i], b.verts[i]))
}

pub fn cache_same(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    f32_same(a.metric, b.metric) && a.count == b.count && a.iA == b.iA && a.iB == b.iB
        && f32_same(a.div, b.div)
}

pub fn fmt_f32(v: f32) -> String {
    format!("{v:?}[{:#010x}]", v.to_bits())
}

pub fn fmt_v(v: c2v) -> String {
    format!("({}, {})", fmt_f32(v.x), fmt_f32(v.y))
}

pub fn fmt_simplex(s: &c2Simplex) -> String {
    let mut out = format!("count={} div={}", s.count, fmt_f32(s.div));
    for (i, v) in s.verts.iter().enumerate() {
        out.push_str(&format!(
            "\n    [{i}] sA={} sB={} p={} u={} iA={} iB={}",
            fmt_v(v.sA),
            fmt_v(v.sB),
            fmt_v(v.p),
            fmt_f32(v.u),
            v.iA,
            v.iB
        ));
    }
    out
}

pub fn fmt_proxy(p: &c2Proxy) -> String {
    let mut out = format!("radius={} count={}", fmt_f32(p.radius), p.count);
    for (i, v) in p.verts.iter().enumerate() {
        out.push_str(&format!("\n    v[{i}]={}", fmt_v(*v)));
    }
    out
}

pub fn fmt_cache(c: &c2GJKCache) -> String {
    format!(
        "metric={} count={} iA={:?} iB={:?} div={}",
        fmt_f32(c.metric),
        c.count,
        c.iA,
        c.iB,
        fmt_f32(c.div)
    )
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) + value generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

/// Global seed shift, from `$SEED_OFFSET`. Every test uses a *fixed* per-row
/// seed so failures are reproducible, but the whole suite can be re-run over a
/// completely different sample of the input space with
/// `SEED_OFFSET=<n> cargo test`. Used to confirm the fixed seeds did not just
/// get lucky.
fn seed_offset() -> u64 {
    static OFF: OnceLock<u64> = OnceLock::new();
    *OFF.get_or_init(|| {
        std::env::var("SEED_OFFSET")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
    })
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed
            .wrapping_add(seed_offset().wrapping_mul(0x1000_0000_0000_0001))
            ^ 0x9E37_79B9_7F4A_7C15)
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

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Uniform in `[-1, 1]`.
    pub fn unit(&mut self) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        u * 2.0 - 1.0
    }

    /// "Ordinary" float: uniform in `[-scale, scale]`, occasionally snapped to a
    /// round value so that exact ties / boundary comparisons get hit too.
    pub fn ordinary(&mut self, scale: f32) -> f32 {
        let v = self.unit() * scale;
        match self.below(8) {
            0 => v.round(),
            1 => (v * 4.0).round() / 4.0,
            2 => 0.0,
            _ => v,
        }
    }

    /// Small non-negative radius, sometimes exactly 0.
    pub fn radius(&mut self, scale: f32) -> f32 {
        match self.below(6) {
            0 => 0.0,
            1 => (self.unit().abs() * scale).round(),
            _ => self.unit().abs() * scale,
        }
    }

    /// A float drawn from the pathological set, or a fully random bit pattern.
    pub fn special(&mut self) -> f32 {
        const SPECIALS: [f32; 26] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            1e-45,  // smallest subnormal
            -1e-45,
            f32::MAX,
            f32::MIN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            FLT_EPSILON,
            -FLT_EPSILON,
            FLT_EPSILON * FLT_EPSILON,
            1.0e8,
            -1.0e8,
            1.0e20,
            -1.0e20,
            16_777_216.0, // 2^24, first integer that is not exactly representable +1
            3.0,
            -3.0,
        ];
        match self.below(4) {
            0..=2 => SPECIALS[self.below(SPECIALS.len() as u32) as usize],
            // Fully random 32-bit pattern: covers arbitrary NaN payloads,
            // subnormals and huge exponents.
            _ => f32::from_bits(self.next_u32()),
        }
    }

    /// Like [`Rng::special`] but never produces NaN (for cases where NaN would
    /// only differ in payload).
    pub fn special_no_nan(&mut self) -> f32 {
        loop {
            let v = self.special();
            if !v.is_nan() {
                return v;
            }
        }
    }

    pub fn v_ordinary(&mut self, scale: f32) -> c2v {
        c2v {
            x: self.ordinary(scale),
            y: self.ordinary(scale),
        }
    }

    pub fn v_special(&mut self) -> c2v {
        c2v {
            x: self.special(),
            y: self.special(),
        }
    }

    pub fn v_special_no_nan(&mut self) -> c2v {
        c2v {
            x: self.special_no_nan(),
            y: self.special_no_nan(),
        }
    }

    pub fn circle(&mut self, scale: f32) -> c2Circle {
        c2Circle {
            p: self.v_ordinary(scale),
            r: self.radius(scale * 0.5),
        }
    }

    /// Well-formed AABB (`min <= max` componentwise), possibly degenerate.
    pub fn aabb(&mut self, scale: f32) -> c2AABB {
        let a = self.v_ordinary(scale);
        let b = self.v_ordinary(scale);
        let mut min = c2v {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
        };
        let max = c2v {
            x: a.x.max(b.x),
            y: a.y.max(b.y),
        };
        if self.below(8) == 0 {
            // degenerate: zero-area box
            min = max;
        }
        c2AABB { min, max }
    }

    pub fn capsule(&mut self, scale: f32) -> c2Capsule {
        let a = self.v_ordinary(scale);
        let b = if self.below(8) == 0 {
            a // degenerate capsule
        } else {
            self.v_ordinary(scale)
        };
        c2Capsule {
            a,
            b,
            r: self.radius(scale * 0.5),
        }
    }

    /// Unit rotor `(cos t, sin t)` for a random `t`.
    pub fn rot_unit(&mut self) -> c2r {
        let t = self.unit() * std::f32::consts::PI;
        c2r {
            c: t.cos(),
            s: t.sin(),
        }
    }

    pub fn xform_translation(&mut self, scale: f32) -> c2x {
        c2x {
            p: self.v_ordinary(scale),
            r: c2r { c: 1.0, s: 0.0 },
        }
    }

    pub fn xform_rot_trans(&mut self, scale: f32) -> c2x {
        c2x {
            p: self.v_ordinary(scale),
            r: self.rot_unit(),
        }
    }

    /// Deliberately non-unit / degenerate rotor: the C code never normalises.
    pub fn xform_weird(&mut self, scale: f32) -> c2x {
        let r = match self.below(4) {
            0 => c2r { c: 0.0, s: 0.0 },
            1 => c2r {
                c: self.ordinary(3.0),
                s: self.ordinary(3.0),
            },
            2 => {
                let u = self.rot_unit();
                let k = self.ordinary(4.0);
                c2r { c: u.c * k, s: u.s * k }
            }
            _ => c2r {
                c: self.special_no_nan(),
                s: self.special_no_nan(),
            },
        };
        c2x {
            p: self.v_ordinary(scale),
            r,
        }
    }
}

// ---------------------------------------------------------------------------
// Shape <-> flat-parameter conversion (the `omni_collide` / `ptr_from_parts`
// packing: circle = (x, y, r, _, _), aabb = (minx, miny, maxx, maxy, _),
// capsule = (ax, ay, bx, by, r))
// ---------------------------------------------------------------------------

/// A shape plus its `C2_TYPE` tag, kept in a caller-owned buffer big enough for
/// any of the three (`c2Capsule` is the largest at 20 bytes).
#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Capsule(c2Capsule),
    Circle(c2Circle),
    Aabb(c2AABB),
}

impl Shape {
    pub fn ty(&self) -> c_int {
        match self {
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
        }
    }

    /// The five `float` parameters `ptr_from_parts` would use to rebuild it.
    /// Unused slots are filled with a fixed poison value to prove they are
    /// ignored identically by both implementations.
    pub fn parts(&self) -> [f32; 5] {
        const POISON: f32 = -12345.678;
        match self {
            Shape::Circle(c) => [c.p.x, c.p.y, c.r, POISON, POISON],
            Shape::Aabb(b) => [b.min.x, b.min.y, b.max.x, b.max.y, POISON],
            Shape::Capsule(c) => [c.a.x, c.a.y, c.b.x, c.b.y, c.r],
        }
    }

    /// Raw bytes of the shape struct, in a 20-byte buffer.
    pub fn bytes(&self) -> [u8; 20] {
        let mut out = [0u8; 20];
        match self {
            Shape::Circle(c) => {
                let src: [u8; 12] = unsafe { std::mem::transmute(*c) };
                out[..12].copy_from_slice(&src);
            }
            Shape::Aabb(b) => {
                let src: [u8; 16] = unsafe { std::mem::transmute(*b) };
                out[..16].copy_from_slice(&src);
            }
            Shape::Capsule(c) => {
                let src: [u8; 20] = unsafe { std::mem::transmute(*c) };
                out.copy_from_slice(&src);
            }
        }
        out
    }

    pub fn random(rng: &mut Rng, ty: c_int, scale: f32) -> Shape {
        match ty {
            C2_TYPE_CIRCLE => Shape::Circle(rng.circle(scale)),
            C2_TYPE_AABB => Shape::Aabb(rng.aabb(scale)),
            _ => Shape::Capsule(rng.capsule(scale)),
        }
    }

    /// Degenerate / hostile-but-valid variants: zero and negative radii,
    /// `a == b` capsules, inverted and zero-area AABBs.
    pub fn random_degenerate(rng: &mut Rng, ty: c_int, scale: f32) -> Shape {
        match ty {
            C2_TYPE_CIRCLE => {
                let p = rng.v_ordinary(scale);
                let r = match rng.below(4) {
                    0 => 0.0,
                    1 => -rng.radius(scale),
                    2 => -0.0,
                    _ => rng.radius(scale),
                };
                Shape::Circle(c2Circle { p, r })
            }
            C2_TYPE_AABB => {
                let a = rng.v_ordinary(scale);
                let b = rng.v_ordinary(scale);
                let bb = match rng.below(3) {
                    // inverted
                    0 => c2AABB {
                        min: c2v {
                            x: a.x.max(b.x),
                            y: a.y.max(b.y),
                        },
                        max: c2v {
                            x: a.x.min(b.x),
                            y: a.y.min(b.y),
                        },
                    },
                    // zero-area
                    1 => c2AABB { min: a, max: a },
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
                };
                Shape::Aabb(bb)
            }
            _ => {
                let a = rng.v_ordinary(scale);
                let b = match rng.below(3) {
                    0 => a,
                    _ => rng.v_ordinary(scale),
                };
                let r = match rng.below(4) {
                    0 => 0.0,
                    1 => -rng.radius(scale),
                    2 => -0.0,
                    _ => rng.radius(scale),
                };
                Shape::Capsule(c2Capsule { a, b, r })
            }
        }
    }

    /// Extreme-magnitude variants (no NaN: NaN would only differ in payload).
    pub fn random_extreme(rng: &mut Rng, ty: c_int) -> Shape {
        match ty {
            C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                p: rng.v_special_no_nan(),
                r: rng.special_no_nan(),
            }),
            C2_TYPE_AABB => Shape::Aabb(c2AABB {
                min: rng.v_special_no_nan(),
                max: rng.v_special_no_nan(),
            }),
            _ => Shape::Capsule(c2Capsule {
                a: rng.v_special_no_nan(),
                b: rng.v_special_no_nan(),
                r: rng.special_no_nan(),
            }),
        }
    }
}

/// Calls `c2Collided` on both libraries with `A`/`B` in caller-owned buffers.
pub fn collided_both(c: &Api, r: &Api, a: &Shape, b: &Shape) -> (c_int, c_int) {
    // 4-byte aligned storage, as C would have for a real struct.
    #[repr(align(4))]
    struct Buf([u8; 20]);
    let ba = Buf(a.bytes());
    let bb = Buf(b.bytes());
    let pa = ba.0.as_ptr() as *const c_void;
    let pb = bb.0.as_ptr() as *const c_void;
    unsafe {
        (
            (c.c2Collided)(pa, a.ty(), pb, b.ty()),
            (r.c2Collided)(pa, a.ty(), pb, b.ty()),
        )
    }
}

extern "C" {
    /// Same allocator both `.so`s use (glibc `malloc`), so we can free what
    /// `ptr_from_parts` hands back.
    pub fn free(p: *mut c_void);
}

/// Number of randomized iterations per `CONFIGS.md` row.
pub const N: usize = 4000;
/// Smaller iteration count for rows whose per-iteration cost is higher.
pub const N_SLOW: usize = 800;
