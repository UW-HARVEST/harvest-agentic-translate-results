//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes every exported
//! symbol as a C-ABI function pointer. Nothing in this file calls the Rust
//! crate directly: the Rust side is always reached through
//! `libspec_ray_lib.so`, exactly as an external C consumer would, so the
//! `#[no_mangle]` wrappers and the struct-passing ABI are under test too.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C ABI types (mirrors of include/lib.h + the private structs in src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;
pub const C2_TYPE_CAPSULE: c_uint = 2;

/// A properly aligned 12-byte scratch buffer for the `c2Raycast *out`
/// parameter. Kept as raw bytes so that "the callee did not write this field"
/// is distinguishable from "the callee wrote the same value".
#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OutBuf(pub [u8; 12]);

/// Arbitrary non-float-looking fill so an untouched field is obvious.
pub const OUT_FILL: OutBuf = OutBuf([0xA5; 12]);

impl OutBuf {
    pub fn filled() -> Self {
        OUT_FILL
    }
    pub fn as_ptr(&mut self) -> *mut c2Raycast {
        self.0.as_mut_ptr() as *mut c2Raycast
    }
    pub fn words(&self) -> [u32; 3] {
        [
            u32::from_ne_bytes([self.0[0], self.0[1], self.0[2], self.0[3]]),
            u32::from_ne_bytes([self.0[4], self.0[5], self.0[6], self.0[7]]),
            u32::from_ne_bytes([self.0[8], self.0[9], self.0[10], self.0[11]]),
        ]
    }
}

impl std::fmt::Debug for OutBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let w = self.words();
        write!(
            f,
            "{{t:{:#010x} n.x:{:#010x} n.y:{:#010x}}}",
            w[0], w[1], w[2]
        )
    }
}

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVf = unsafe extern "C" fn(c2v) -> f32;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVfV = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnMV = unsafe extern "C" fn(c2m, c2v) -> c2v;
pub type FnBBi = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnBVi = unsafe extern "C" fn(c2AABB, c2v) -> c_int;
pub type FnCVi = unsafe extern "C" fn(c2Circle, c2v) -> c_int;
pub type FnRayCircle = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
pub type FnRayAABB = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
pub type FnRayCapsule = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
pub type FnCastRay = unsafe extern "C" fn(c2Ray, *const c_void, c_uint, *mut c2Raycast) -> c_int;
pub type FnSpecRay =
    unsafe extern "C" fn(*mut c2Raycast, f32, f32, f32, f32, f32, f32, f32) -> c_int;

/// One loaded implementation. `_lib` must outlive every function pointer, so
/// the whole thing is leaked into a `'static` via `OnceLock`.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub c2V: FnV,
    pub c2Dot: FnVVf,
    pub c2Len: FnVf,
    pub c2Add: FnVVV,
    pub c2Sub: FnVVV,
    pub c2Mulvs: FnVfV,
    pub c2Div: FnVfV,
    pub c2Norm: FnVV,
    pub c2Minv: FnVVV,
    pub c2Maxv: FnVVV,
    pub c2Skew: FnVV,
    pub c2Absv: FnVV,
    pub c2CCW90: FnVV,
    pub c2MulmvT: FnMV,
    pub c2AABBtoAABB: FnBBi,
    pub c2AABBtoPoint: FnBVi,
    pub c2CircleToPoint: FnCVi,
    pub c2RaytoCircle: FnRayCircle,
    pub c2RaytoAABB: FnRayAABB,
    pub c2RaytoCapsule: FnRayCapsule,
    pub c2CastRay: FnCastRay,
    pub spec_ray: FnSpecRay,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> T {
    let s: Symbol<T> = lib
        .get(name.as_bytes())
        .unwrap_or_else(|e| panic!("symbol {name} not found: {e}"));
    *s
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
        unsafe {
            Impl {
                name,
                c2V: sym(&lib, "c2V"),
                c2Dot: sym(&lib, "c2Dot"),
                c2Len: sym(&lib, "c2Len"),
                c2Add: sym(&lib, "c2Add"),
                c2Sub: sym(&lib, "c2Sub"),
                c2Mulvs: sym(&lib, "c2Mulvs"),
                c2Div: sym(&lib, "c2Div"),
                c2Norm: sym(&lib, "c2Norm"),
                c2Minv: sym(&lib, "c2Minv"),
                c2Maxv: sym(&lib, "c2Maxv"),
                c2Skew: sym(&lib, "c2Skew"),
                c2Absv: sym(&lib, "c2Absv"),
                c2CCW90: sym(&lib, "c2CCW90"),
                c2MulmvT: sym(&lib, "c2MulmvT"),
                c2AABBtoAABB: sym(&lib, "c2AABBtoAABB"),
                c2AABBtoPoint: sym(&lib, "c2AABBtoPoint"),
                c2CircleToPoint: sym(&lib, "c2CircleToPoint"),
                c2RaytoCircle: sym(&lib, "c2RaytoCircle"),
                c2RaytoAABB: sym(&lib, "c2RaytoAABB"),
                c2RaytoCapsule: sym(&lib, "c2RaytoCapsule"),
                c2CastRay: sym(&lib, "c2CastRay"),
                spec_ray: sym(&lib, "spec_ray"),
                _lib: lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two .so files
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                best = Some(p);
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "no lib*.so under {} -- build the C first:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in <target>/<profile>/deps/, and the cdylib in
    // <target>/<profile>/. Prefer that, then fall back to the sibling profile.
    let exe = std::env::current_exe().expect("current_exe");
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(deps) = exe.parent() {
        if let Some(profile) = deps.parent() {
            cands.push(profile.join("libspec_ray_lib.so"));
            if let Some(target) = profile.parent() {
                cands.push(target.join("release/libspec_ray_lib.so"));
                cands.push(target.join("debug/libspec_ray_lib.so"));
            }
        }
    }
    let t = workspace_root().join("translation/target");
    cands.push(t.join("release/libspec_ray_lib.so"));
    cands.push(t.join("debug/libspec_ray_lib.so"));
    for c in &cands {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "libspec_ray_lib.so not found; tried:\n{}",
        cands
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

static C_IMPL: OnceLock<Impl> = OnceLock::new();
static R_IMPL: OnceLock<Impl> = OnceLock::new();

/// `cargo test` builds the integration-test binaries but **not** the `cdylib`
/// (the tests do not link against the crate -- they `dlopen` it), so a plain
/// `cargo test` after editing `src/lib.rs` would silently load a stale `.so`
/// and report a false pass. Refuse to run in that situation.
fn assert_so_is_fresh(so: &Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let so_t = std::fs::metadata(so).and_then(|m| m.modified());
    let src_t = std::fs::metadata(&src).and_then(|m| m.modified());
    if let (Ok(so_t), Ok(src_t)) = (so_t, src_t) {
        assert!(
            so_t >= src_t,
            "STALE LIBRARY: {} is older than {}.\n\
             `cargo test` does not rebuild the cdylib, so this run would have \
             compared against an out-of-date .so and passed for the wrong \
             reason. Rebuild first:\n\
             \n    cargo build --release && cargo test --release\n\
             \nor use ./verify_all.sh, which always builds before testing.",
            so.display(),
            src.display()
        );
    }
}

/// `(c_impl, rust_impl)` — both reached only through `dlopen`/`dlsym`.
pub fn pair() -> (&'static Impl, &'static Impl) {
    let c = C_IMPL.get_or_init(|| Impl::load("C", &find_c_so()));
    let r = R_IMPL.get_or_init(|| {
        let p = find_rust_so();
        assert_so_is_fresh(&p);
        Impl::load("Rust", &p)
    });
    (c, r)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, n)`.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-m, m]`.
    #[inline]
    pub fn sym(&mut self, m: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * m
    }
    /// Any bit pattern at all: normals, subnormals, `±0`, `±inf`, `NaN`
    /// (with a random payload), uniformly over the 2^32 encodings.
    #[inline]
    pub fn any_bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A value drawn from the "interesting classes" table.
    pub fn special_f32(&mut self) -> f32 {
        const S: &[f32] = &[
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            2.0,
            -2.0,
            f32::MIN_POSITIVE,             // smallest normal
            -f32::MIN_POSITIVE,
            f32::MAX,
            f32::MIN,
            f32::EPSILON,
            -f32::EPSILON,
            f32::INFINITY,
            f32::NEG_INFINITY,
            1e-30,
            -1e-30,
            1e30,
            -1e30,
            1e18,
            -1e18,
            16777216.0,  // 2^24, first f32 with a gap
            16777217.0,
        ];
        let n = (S.len() + 6) as u32;
        let i = self.below(n) as usize;
        match i.checked_sub(S.len()) {
            None => S[i],
            Some(0) => f32::from_bits(0x0000_0001),          // +min subnormal
            Some(1) => f32::from_bits(0x8000_0001),          // -min subnormal
            Some(2) => f32::from_bits(0x7FC0_0000),          // +quiet NaN
            Some(3) => f32::from_bits(0xFFC0_0000),          // -quiet NaN
            Some(4) => f32::from_bits(0x7FC0_0000 | (self.next_u32() & 0x3F_FFFF)), // NaN payload
            _ => f32::from_bits(0xFF80_0000 | (self.next_u32() & 0x7F_FFFF)), // -NaN payload
        }
    }
    /// A "mostly reasonable, sometimes hostile" float: 3/4 uniform in
    /// `[-m, m]`, 1/8 from the special table, 1/8 a fully random bit pattern.
    pub fn mixed_f32(&mut self, m: f32) -> f32 {
        match self.below(8) {
            0 => self.special_f32(),
            1 => self.any_bits_f32(),
            _ => self.sym(m),
        }
    }
    pub fn v(&mut self, m: f32) -> c2v {
        c2v {
            x: self.sym(m),
            y: self.sym(m),
        }
    }
    pub fn v_mixed(&mut self, m: f32) -> c2v {
        c2v {
            x: self.mixed_f32(m),
            y: self.mixed_f32(m),
        }
    }
    pub fn v_special(&mut self) -> c2v {
        c2v {
            x: self.special_f32(),
            y: self.special_f32(),
        }
    }
    /// A unit vector at a random angle.
    pub fn dir(&mut self) -> c2v {
        let a = self.unit() * std::f32::consts::TAU;
        c2v {
            x: a.cos(),
            y: a.sin(),
        }
    }
    /// A box with `min <= max`.
    pub fn aabb(&mut self, m: f32) -> c2AABB {
        let a = self.v(m);
        let b = self.v(m);
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

// ---------------------------------------------------------------------------
// Divergence accounting
// ---------------------------------------------------------------------------

/// Collects mismatches so a failing row reports a representative sample rather
/// than only the first bad input.
pub struct Diff {
    row: String,
    checked: usize,
    failures: Vec<String>,
}

const MAX_REPORTED: usize = 12;

impl Diff {
    pub fn new(row: impl Into<String>) -> Diff {
        Diff {
            row: row.into(),
            checked: 0,
            failures: Vec::new(),
        }
    }

    pub fn eq<T: PartialEq + std::fmt::Debug>(&mut self, ctx: impl FnOnce() -> String, c: T, r: T) {
        self.checked += 1;
        if c != r {
            if self.failures.len() < MAX_REPORTED {
                self.failures
                    .push(format!("  {}\n      C={:?}\n   Rust={:?}", ctx(), c, r));
            } else {
                self.failures.truncate(MAX_REPORTED);
            }
        }
    }

    /// Bit-exact `f32` comparison (so `NaN != NaN` unless the payloads match,
    /// and `+0.0 != -0.0`).
    pub fn f32_bits(&mut self, ctx: impl FnOnce() -> String, c: f32, r: f32) {
        self.checked += 1;
        if c.to_bits() != r.to_bits() {
            if self.failures.len() < MAX_REPORTED {
                self.failures.push(format!(
                    "  {}\n      C={:#010x} ({c:e})\n   Rust={:#010x} ({r:e})",
                    ctx(),
                    c.to_bits(),
                    r.to_bits()
                ));
            }
        }
    }

    pub fn v_bits(&mut self, ctx: impl Fn() -> String, c: c2v, r: c2v) {
        self.f32_bits(|| format!("{}.x", ctx()), c.x, r.x);
        self.f32_bits(|| format!("{}.y", ctx()), c.y, r.y);
    }

    pub fn checked(&self) -> usize {
        self.checked
    }

    #[track_caller]
    pub fn finish(self) {
        assert!(
            self.checked > 0,
            "row `{}` performed no comparisons",
            self.row
        );
        if !self.failures.is_empty() {
            panic!(
                "row `{}`: {} of {} comparisons diverged (first {} shown)\n{}",
                self.row,
                self.failures.len(),
                self.checked,
                self.failures.len().min(MAX_REPORTED),
                self.failures.join("\n")
            );
        }
    }
}

/// Formatting helper used in a lot of contexts.
pub fn fv(v: c2v) -> String {
    format!("({:#010x},{:#010x})", v.x.to_bits(), v.y.to_bits())
}
pub fn fray(r: c2Ray) -> String {
    format!("ray{{p:{} d:{} t:{:#010x}}}", fv(r.p), fv(r.d), r.t.to_bits())
}
pub fn fcircle(c: c2Circle) -> String {
    format!("circle{{p:{} r:{:#010x}}}", fv(c.p), c.r.to_bits())
}
pub fn fbox(b: c2AABB) -> String {
    format!("aabb{{min:{} max:{}}}", fv(b.min), fv(b.max))
}
pub fn fcap(c: c2Capsule) -> String {
    format!(
        "capsule{{a:{} b:{} r:{:#010x}}}",
        fv(c.a),
        fv(c.b),
        c.r.to_bits()
    )
}

/// Run both raycast-shaped implementations on a pre-filled out-buffer and
/// compare the returned int **and** all 12 out bytes.
pub struct RayResult {
    pub ret: c_int,
    pub out: OutBuf,
}

impl PartialEq for RayResult {
    fn eq(&self, other: &Self) -> bool {
        self.ret == other.ret && self.out == other.out
    }
}

impl std::fmt::Debug for RayResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ret={} out={:?}", self.ret, self.out)
    }
}

// ---------------------------------------------------------------------------
// Differential drivers for the raycast-shaped entry points.
//
// Each one pre-fills a 12-byte out-buffer with `OUT_FILL`, calls the C and the
// Rust export, and compares the returned `int` together with all 12 out bytes.
// Comparing the raw bytes (rather than reading `t`/`n` as floats) is what makes
// "the callee left this field alone" observable, which matters because
// `c2RaytoCircle` / `c2RaytoAABB` only write on a hit while `c2RaytoCapsule`
// always writes `n` and `t` up front.
// ---------------------------------------------------------------------------

pub fn cmp_ray_circle(d: &mut Diff, c: &Impl, r: &Impl, ray: c2Ray, s: c2Circle) {
    let mut cb = OutBuf::filled();
    let mut rb = OutBuf::filled();
    let cres = RayResult {
        ret: unsafe { (c.c2RaytoCircle)(ray, s, cb.as_ptr()) },
        out: cb,
    };
    let rres = RayResult {
        ret: unsafe { (r.c2RaytoCircle)(ray, s, rb.as_ptr()) },
        out: rb,
    };
    d.eq(
        || format!("c2RaytoCircle({}, {})", fray(ray), fcircle(s)),
        cres,
        rres,
    );
}

pub fn cmp_ray_aabb(d: &mut Diff, c: &Impl, r: &Impl, ray: c2Ray, s: c2AABB) {
    let mut cb = OutBuf::filled();
    let mut rb = OutBuf::filled();
    let cres = RayResult {
        ret: unsafe { (c.c2RaytoAABB)(ray, s, cb.as_ptr()) },
        out: cb,
    };
    let rres = RayResult {
        ret: unsafe { (r.c2RaytoAABB)(ray, s, rb.as_ptr()) },
        out: rb,
    };
    d.eq(
        || format!("c2RaytoAABB({}, {})", fray(ray), fbox(s)),
        cres,
        rres,
    );
}

pub fn cmp_ray_capsule(d: &mut Diff, c: &Impl, r: &Impl, ray: c2Ray, s: c2Capsule) {
    let mut cb = OutBuf::filled();
    let mut rb = OutBuf::filled();
    let cres = RayResult {
        ret: unsafe { (c.c2RaytoCapsule)(ray, s, cb.as_ptr()) },
        out: cb,
    };
    let rres = RayResult {
        ret: unsafe { (r.c2RaytoCapsule)(ray, s, rb.as_ptr()) },
        out: rb,
    };
    d.eq(
        || format!("c2RaytoCapsule({}, {})", fray(ray), fcap(s)),
        cres,
        rres,
    );
}

/// `c2CastRay` with an arbitrary tag and an arbitrary payload buffer. The
/// payload is copied into a 32-byte aligned scratch area so that over-reads by
/// a mismatched tag stay in bounds and read *identical* bytes in both calls.
pub fn cmp_cast_ray(
    d: &mut Diff,
    c: &Impl,
    r: &Impl,
    ray: c2Ray,
    payload: &[u8],
    tag: c_uint,
    label: &str,
) {
    #[repr(C, align(16))]
    struct Pad([u8; 32]);
    let mut buf = Pad([0x3C; 32]);
    buf.0[..payload.len()].copy_from_slice(payload);

    let mut cb = OutBuf::filled();
    let mut rb = OutBuf::filled();
    let p = buf.0.as_ptr() as *const c_void;
    let cres = RayResult {
        ret: unsafe { (c.c2CastRay)(ray, p, tag, cb.as_ptr()) },
        out: cb,
    };
    let rres = RayResult {
        ret: unsafe { (r.c2CastRay)(ray, p, tag, rb.as_ptr()) },
        out: rb,
    };
    d.eq(
        || format!("c2CastRay[{label}] tag={tag} {}", fray(ray)),
        cres,
        rres,
    );
}

pub fn as_bytes<T: Copy>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

#[allow(clippy::too_many_arguments)]
pub fn cmp_spec_ray(
    d: &mut Diff,
    c: &Impl,
    r: &Impl,
    mp: c2v,
    cp: c2v,
    cr: f32,
    rp: c2v,
) {
    let mut cb = OutBuf::filled();
    let mut rb = OutBuf::filled();
    let cres = RayResult {
        ret: unsafe { (c.spec_ray)(cb.as_ptr(), mp.x, mp.y, cp.x, cp.y, cr, rp.x, rp.y) },
        out: cb,
    };
    let rres = RayResult {
        ret: unsafe { (r.spec_ray)(rb.as_ptr(), mp.x, mp.y, cp.x, cp.y, cr, rp.x, rp.y) },
        out: rb,
    };
    d.eq(
        || {
            format!(
                "spec_ray(mp={} c.p={} c.r={:#010x} r.p={})",
                fv(mp),
                fv(cp),
                cr.to_bits(),
                fv(rp)
            )
        },
        cres,
        rres,
    );
}

// ---------------------------------------------------------------------------
// Controlled-`%eax` trampoline for `c2CastRay`'s undefined-behaviour edge.
//
// The C's out-of-range-`C2_TYPE` edge branches to the epilogue without writing
// `%eax`, so it returns whatever the *caller* left there. That makes a plain
// `(c.c2CastRay)(..)` vs `(r.c2CastRay)(..)` comparison ill-posed: an optimised
// caller happens to leave the same value before both calls (`ray.t`), but an
// unoptimised one loads each callee's address into `rax` immediately before
// `call rax`, so the two call sites genuinely differ in their input and no
// implementation could make the outputs agree.
//
// `cast_ray_with_eax` removes that ambiguity by setting `%eax` to a caller-
// chosen value immediately before the branch, so both libraries are given
// *identical* register state and the comparison tests the implementation rather
// than the harness's register allocator.
// ---------------------------------------------------------------------------

/// Naked thunk with the same leading ABI as `c2CastRay` plus two extra
/// arguments: `eax_in` (`ecx`) and `target` (`r8`).
///
/// `A` is 20 bytes and therefore MEMORY-classed, so it sits on the stack; the
/// tail `jmp` leaves the frame -- and hence `A`'s location -- untouched, and
/// `rdi`/`esi`/`rdx` already hold `B`/`typeB`/`out`. The extra `ecx`/`r8`
/// arguments are simply ignored by the target.
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
unsafe extern "C" fn cast_ray_eax_thunk(
    _A: c2Ray,
    _B: *const c_void,
    _typeB: c_uint,
    _out: *mut c2Raycast,
    _eax_in: c_uint,
    _target: usize,
) -> c_int {
    core::arch::naked_asm!("mov eax, ecx", "jmp r8")
}

/// Call `f` with `%eax` set to exactly `eax_in` at the call boundary.
#[cfg(target_arch = "x86_64")]
pub unsafe fn cast_ray_with_eax(
    f: FnCastRay,
    eax_in: u32,
    ray: c2Ray,
    b: *const c_void,
    tag: c_uint,
    out: *mut c2Raycast,
) -> c_int {
    unsafe { cast_ray_eax_thunk(ray, b, tag, out, eax_in, f as usize) }
}

/// Compare the two `c2CastRay` exports under a *controlled* incoming `%eax`.
pub fn cmp_cast_ray_eax(
    d: &mut Diff,
    c: &Impl,
    r: &Impl,
    eax_in: u32,
    ray: c2Ray,
    payload: &[u8],
    tag: c_uint,
) {
    #[repr(C, align(16))]
    struct Pad([u8; 32]);
    let mut buf = Pad([0x3C; 32]);
    buf.0[..payload.len()].copy_from_slice(payload);
    let p = buf.0.as_ptr() as *const c_void;

    let mut cb = OutBuf::filled();
    let mut rb = OutBuf::filled();
    let cres = RayResult {
        ret: unsafe { cast_ray_with_eax(c.c2CastRay, eax_in, ray, p, tag, cb.as_ptr()) },
        out: cb,
    };
    let rres = RayResult {
        ret: unsafe { cast_ray_with_eax(r.c2CastRay, eax_in, ray, p, tag, rb.as_ptr()) },
        out: rb,
    };
    d.eq(
        || format!("c2CastRay eax={eax_in:#010x} tag={tag} {}", fray(ray)),
        cres,
        rres,
    );
}
