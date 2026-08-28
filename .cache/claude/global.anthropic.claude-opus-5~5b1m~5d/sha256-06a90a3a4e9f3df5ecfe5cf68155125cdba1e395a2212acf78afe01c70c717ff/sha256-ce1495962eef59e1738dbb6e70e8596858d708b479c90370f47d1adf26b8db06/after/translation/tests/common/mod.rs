//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` — the C reference built from
//! `c_src/` and the Rust `cdylib` built from this crate — and calls every symbol
//! through the FFI boundary, exactly as an external C consumer would.  No Rust
//! function is ever called directly, so the `#[no_mangle] extern "C"` wrappers
//! and the whole SysV struct-passing ABI are part of what is under test.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_void;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/* -------------------------------------------------------------------------- */
/*                       C types (mirrors of c_src/src/lib.c)                 */
/* -------------------------------------------------------------------------- */

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

pub fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

/// Poison pattern used to pre-fill `c2Raycast` out-parameters so that a path
/// which does *not* write a field is distinguishable from one that does.
pub const POISON: C2Raycast = C2Raycast {
    t: f32::from_bits(0xDEAD_BEEF),
    n: C2v {
        x: f32::from_bits(0xCAFE_BABE),
        y: f32::from_bits(0xFEED_FACE),
    },
};

/* -------------------------------------------------------------------------- */
/*                              the loaded API                                */
/* -------------------------------------------------------------------------- */

pub type FnV = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnDot = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnLen = unsafe extern "C" fn(C2v) -> f32;
pub type FnVV = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnVS = unsafe extern "C" fn(C2v, f32) -> C2v;
pub type FnV1 = unsafe extern "C" fn(C2v) -> C2v;
pub type FnMulmvT = unsafe extern "C" fn(C2m, C2v) -> C2v;
pub type FnRayCircle = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
pub type FnAABBtoAABB = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
pub type FnRayAABB = unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int;
pub type FnAABBtoPoint = unsafe extern "C" fn(C2AABB, C2v) -> c_int;
pub type FnCircleToPoint = unsafe extern "C" fn(C2Circle, C2v) -> c_int;
pub type FnRayCapsule = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
pub type FnCastRay = unsafe extern "C" fn(C2Ray, *const c_void, c_int, *mut C2Raycast) -> c_int;
pub type FnSpecRay =
    unsafe extern "C" fn(*mut C2Raycast, f32, f32, f32, f32, f32, f32, f32) -> c_int;

pub struct Api {
    pub which: &'static str,
    pub path: PathBuf,
    pub c2V: FnV,
    pub c2Dot: FnDot,
    pub c2Len: FnLen,
    pub c2Add: FnVV,
    pub c2Sub: FnVV,
    pub c2Mulvs: FnVS,
    pub c2Div: FnVS,
    pub c2Norm: FnV1,
    pub c2Minv: FnVV,
    pub c2Maxv: FnVV,
    pub c2Skew: FnV1,
    pub c2Absv: FnV1,
    pub c2CCW90: FnV1,
    pub c2MulmvT: FnMulmvT,
    pub c2RaytoCircle: FnRayCircle,
    pub c2AABBtoAABB: FnAABBtoAABB,
    pub c2RaytoAABB: FnRayAABB,
    pub c2AABBtoPoint: FnAABBtoPoint,
    pub c2CircleToPoint: FnCircleToPoint,
    pub c2RaytoCapsule: FnRayCapsule,
    pub c2CastRay: FnCastRay,
    pub spec_ray: FnSpecRay,
}

/// All 22 exported symbols, in `SYMBOLS.md` order.
pub const ALL_SYMBOLS: [&str; 22] = [
    "c2V",
    "c2Dot",
    "c2Len",
    "c2Add",
    "c2Sub",
    "c2Mulvs",
    "c2Div",
    "c2Norm",
    "c2Minv",
    "c2Maxv",
    "c2Skew",
    "c2Absv",
    "c2CCW90",
    "c2MulmvT",
    "c2RaytoCircle",
    "c2AABBtoAABB",
    "c2RaytoAABB",
    "c2AABBtoPoint",
    "c2CircleToPoint",
    "c2RaytoCapsule",
    "c2CastRay",
    "spec_ray",
];

unsafe fn sym<T: Copy + 'static>(lib: &'static Library, name: &str) -> T {
    let mut owned = name.as_bytes().to_vec();
    owned.push(0);
    let s: Symbol<'static, T> = unsafe {
        lib.get(&owned)
            .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"))
    };
    *s
}

fn load_lib(path: &Path) -> &'static Library {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    Box::leak(Box::new(lib))
}

impl Api {
    /// Load an arbitrary shared library that exports the 22 `c2*` symbols.
    pub fn open(which: &'static str, path: PathBuf) -> Api {
        Api::new(which, path)
    }

    fn new(which: &'static str, path: PathBuf) -> Api {
        let lib = load_lib(&path);
        unsafe {
            Api {
                which,
                path,
                c2V: sym(lib, "c2V"),
                c2Dot: sym(lib, "c2Dot"),
                c2Len: sym(lib, "c2Len"),
                c2Add: sym(lib, "c2Add"),
                c2Sub: sym(lib, "c2Sub"),
                c2Mulvs: sym(lib, "c2Mulvs"),
                c2Div: sym(lib, "c2Div"),
                c2Norm: sym(lib, "c2Norm"),
                c2Minv: sym(lib, "c2Minv"),
                c2Maxv: sym(lib, "c2Maxv"),
                c2Skew: sym(lib, "c2Skew"),
                c2Absv: sym(lib, "c2Absv"),
                c2CCW90: sym(lib, "c2CCW90"),
                c2MulmvT: sym(lib, "c2MulmvT"),
                c2RaytoCircle: sym(lib, "c2RaytoCircle"),
                c2AABBtoAABB: sym(lib, "c2AABBtoAABB"),
                c2RaytoAABB: sym(lib, "c2RaytoAABB"),
                c2AABBtoPoint: sym(lib, "c2AABBtoPoint"),
                c2CircleToPoint: sym(lib, "c2CircleToPoint"),
                c2RaytoCapsule: sym(lib, "c2RaytoCapsule"),
                c2CastRay: sym(lib, "c2CastRay"),
                spec_ray: sym(lib, "spec_ray"),
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                            library discovery                               */
/* -------------------------------------------------------------------------- */

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn workspace_dir() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

/// `c_src/build/lib<workdir>.so`, or `$SPEC_RAY_C_SO`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("SPEC_RAY_C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_dir().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

/// `target/<profile>/libspec_ray_lib.so`, or `$SPEC_RAY_RUST_SO`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("SPEC_RAY_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>  ->  .../target/<profile>
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let candidates = [
        profile_dir.join("libspec_ray_lib.so"),
        manifest_dir().join("target/release/libspec_ray_lib.so"),
        manifest_dir().join("target/debug/libspec_ray_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found (looked in {:?}).\n\
         `cargo test` does not build a cdylib-only lib target, so build it first:\n  \
         cargo build --offline   # and/or --release",
        candidates
    );
}

fn mtime(p: &Path) -> std::time::SystemTime {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::UNIX_EPOCH)
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn apis() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        // Guard against testing a stale cdylib (skipped when the path was
        // pinned explicitly, e.g. when cross-checking two C builds).
        let src = manifest_dir().join("src/lib.rs");
        if std::env::var_os("SPEC_RAY_RUST_SO").is_none() && mtime(&r_path) < mtime(&src) {
            panic!(
                "{} is older than {} — rebuild with `cargo build --offline [--release]` \
                 before running the differential tests",
                r_path.display(),
                src.display()
            );
        }
        eprintln!("[harness]   C .so: {}", c_path.display());
        eprintln!("[harness] rust .so: {}", r_path.display());
        Pair {
            c: Api::new("C", c_path),
            r: Api::new("rust", r_path),
        }
    })
}

/* -------------------------------------------------------------------------- */
/*                          deterministic PRNG                                */
/* -------------------------------------------------------------------------- */

/// splitmix64 — tiny, deterministic, no external crate.
pub struct Rng(u64);

pub const SPECIALS: [f32; 26] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    f32::MIN_POSITIVE,               // smallest normal
    -f32::MIN_POSITIVE,              //
    1.0e-45,                         // smallest denormal
    -1.0e-45,                        //
    1.1754942e-38,                   // largest denormal
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    16777216.0,                      // 2^24, f32 integer precision limit
    16777217.0,                      // rounds to 2^24
    1.0e30,
    1.0e-30,
    -1.0e30,
    1.0e-7,
];

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + (hi - lo) * u
    }

    /// A "nice" finite coordinate in [-100, 100), sometimes an exact integer or
    /// a half-integer so that boundary comparisons (`t == A.t`, ties) are hit.
    pub fn coord(&mut self) -> f32 {
        match self.below(8) {
            0 => self.below(21) as f32 - 10.0,
            1 => (self.below(41) as f32 - 20.0) * 0.5,
            2 => self.range(-1.0, 1.0),
            _ => self.range(-100.0, 100.0),
        }
    }

    /// A non-negative radius-like value, sometimes 0.
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => self.below(11) as f32,
            2 => self.range(0.0, 0.001),
            _ => self.range(0.0, 50.0),
        }
    }

    /// A completely random bit pattern reinterpreted as `f32`.
    pub fn bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    pub fn special(&mut self) -> f32 {
        SPECIALS[self.below(SPECIALS.len() as u32) as usize]
    }

    /// The "wild" generator: mixes specials, raw bit patterns, denormals and
    /// ordinary coordinates.  Covers axis L of `CONFIGS.md`.
    pub fn wild(&mut self) -> f32 {
        match self.below(10) {
            0 | 1 => self.special(),
            2 | 3 | 4 => self.bits_f32(),
            5 => f32::from_bits(self.next_u32() & 0x807F_FFFF), // denormal / zero
            6 => f32::from_bits((self.next_u32() & 0x807F_FFFF) | 0x7F80_0000), // NaN/inf
            _ => self.coord(),
        }
    }

    pub fn wild_v(&mut self) -> C2v {
        C2v {
            x: self.wild(),
            y: self.wild(),
        }
    }

    pub fn coord_v(&mut self) -> C2v {
        C2v {
            x: self.coord(),
            y: self.coord(),
        }
    }

    pub fn wild_ray(&mut self) -> C2Ray {
        C2Ray {
            p: self.wild_v(),
            d: self.wild_v(),
            t: self.wild(),
        }
    }

    pub fn wild_aabb(&mut self) -> C2AABB {
        C2AABB {
            min: self.wild_v(),
            max: self.wild_v(),
        }
    }

    pub fn wild_circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.wild_v(),
            r: self.wild(),
        }
    }

    pub fn wild_capsule(&mut self) -> C2Capsule {
        C2Capsule {
            a: self.wild_v(),
            b: self.wild_v(),
            r: self.wild(),
        }
    }

    /// A proper (min <= max) finite box.
    pub fn proper_aabb(&mut self) -> C2AABB {
        let (x0, x1) = (self.coord(), self.coord());
        let (y0, y1) = (self.coord(), self.coord());
        C2AABB {
            min: v(x0.min(x1), y0.min(y1)),
            max: v(x0.max(x1), y0.max(y1)),
        }
    }

    /// A finite ray with a (mostly) normalized direction.
    pub fn nice_ray(&mut self) -> C2Ray {
        let ang = self.range(-3.15, 3.15);
        let (mut dx, mut dy) = (ang.cos(), ang.sin());
        match self.below(6) {
            0 => {
                dx = 1.0;
                dy = 0.0;
            }
            1 => {
                dx = 0.0;
                dy = 1.0;
            }
            2 => {
                dx = -1.0;
                dy = 0.0;
            }
            3 => {
                dx = 0.0;
                dy = -1.0;
            }
            _ => {}
        }
        let scale = match self.below(8) {
            0 => 0.0,
            1 => 2.0,
            2 => 0.001,
            _ => 1.0,
        };
        C2Ray {
            p: self.coord_v(),
            d: v(dx * scale, dy * scale),
            t: match self.below(8) {
                0 => 0.0,
                1 => -self.range(0.0, 10.0),
                2 => self.range(0.0, 1.0e9),
                _ => self.range(0.0, 200.0),
            },
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                              comparison                                    */
/* -------------------------------------------------------------------------- */

pub struct Checker {
    pub row: String,
    pub checked: usize,
    pub hard: usize,
    pub nan_payload: usize,
    pub reports: Vec<String>,
    pub nan_reports: Vec<String>,
}

impl Checker {
    pub fn new(row: &str) -> Checker {
        Checker {
            row: row.to_string(),
            checked: 0,
            hard: 0,
            nan_payload: 0,
            reports: Vec::new(),
            nan_reports: Vec::new(),
        }
    }

    pub fn f32<F: Fn() -> String>(&mut self, field: &str, c: f32, r: f32, ctx: F) {
        self.checked += 1;
        if c.to_bits() == r.to_bits() {
            return;
        }
        if c.is_nan() && r.is_nan() {
            self.nan_payload += 1;
            if self.nan_reports.len() < 3 {
                self.nan_reports.push(format!(
                    "  {field}: C=NaN(0x{:08x}) rust=NaN(0x{:08x})  input {}",
                    c.to_bits(),
                    r.to_bits(),
                    ctx()
                ));
            }
            return;
        }
        self.hard += 1;
        if self.reports.len() < 8 {
            self.reports.push(format!(
                "  {field}: C=0x{:08x} ({:e})  rust=0x{:08x} ({:e})  input {}",
                c.to_bits(),
                c,
                r.to_bits(),
                r,
                ctx()
            ));
        }
    }

    pub fn int<F: Fn() -> String>(&mut self, field: &str, c: c_int, r: c_int, ctx: F) {
        self.checked += 1;
        if c == r {
            return;
        }
        self.hard += 1;
        if self.reports.len() < 8 {
            self.reports.push(format!(
                "  {field}: C={c} rust={r}  input {}",
                ctx()
            ));
        }
    }

    pub fn vec<F: Fn() -> String>(&mut self, field: &str, c: C2v, r: C2v, ctx: F) {
        self.f32(&format!("{field}.x"), c.x, r.x, &ctx);
        self.f32(&format!("{field}.y"), c.y, r.y, &ctx);
    }

    pub fn cast<F: Fn() -> String>(&mut self, field: &str, c: C2Raycast, r: C2Raycast, ctx: F) {
        self.f32(&format!("{field}.t"), c.t, r.t, &ctx);
        self.f32(&format!("{field}.n.x"), c.n.x, r.n.x, &ctx);
        self.f32(&format!("{field}.n.y"), c.n.y, r.n.y, &ctx);
    }

    /// Panics with a full report if any non-NaN-payload difference was seen.
    pub fn finish(self) {
        eprintln!(
            "[{}] {} comparisons, {} hard mismatches, {} NaN-payload-only diffs",
            self.row, self.checked, self.hard, self.nan_payload
        );
        if !self.nan_reports.is_empty() {
            for l in &self.nan_reports {
                eprintln!("[{}] nan-payload example:{}", self.row, l);
            }
        }
        assert!(self.checked > 0, "[{}] nothing was compared", self.row);
        if self.hard > 0 {
            panic!(
                "[{}] {} of {} comparisons differ between the C and the Rust .so:\n{}",
                self.row,
                self.hard,
                self.checked,
                self.reports.join("\n")
            );
        }
        // The *payload* of a NaN produced from two NaN operands is unspecified
        // by IEEE-754: on SSE it comes from whichever operand the compiler put
        // in the destination register.  The C reference disagrees with its own
        // -O2 build on ~8300 of these (more than the Rust build differs from
        // it), see `nan_payload_policy.rs`, so a payload-only difference cannot
        // be a correctness failure.  It is counted and reported, and can be
        // turned into a failure with SPEC_RAY_STRICT_NAN=1.
        if self.nan_payload > 0 && std::env::var_os("SPEC_RAY_STRICT_NAN").is_some() {
            panic!(
                "[{}] {} NaN-payload-only differences (SPEC_RAY_STRICT_NAN is set):\n{}",
                self.row,
                self.nan_payload,
                self.nan_reports.join("\n")
            );
        }
    }
}

/* ---------------------------- formatting helpers -------------------------- */

pub fn fv(a: C2v) -> String {
    format!("({:e}/0x{:08x}, {:e}/0x{:08x})", a.x, a.x.to_bits(), a.y, a.y.to_bits())
}
pub fn fray(a: C2Ray) -> String {
    format!("ray{{p={} d={} t={:e}/0x{:08x}}}", fv(a.p), fv(a.d), a.t, a.t.to_bits())
}
pub fn fcircle(a: C2Circle) -> String {
    format!("circle{{p={} r={:e}/0x{:08x}}}", fv(a.p), a.r, a.r.to_bits())
}
pub fn faabb(a: C2AABB) -> String {
    format!("aabb{{min={} max={}}}", fv(a.min), fv(a.max))
}
pub fn fcap(a: C2Capsule) -> String {
    format!("capsule{{a={} b={} r={:e}/0x{:08x}}}", fv(a.a), fv(a.b), a.r, a.r.to_bits())
}
pub fn fm(a: C2m) -> String {
    format!("m{{x={} y={}}}", fv(a.x), fv(a.y))
}

/// Number of randomized inputs per `CONFIGS.md` row (override with `SPEC_RAY_N`).
pub fn n_iters() -> usize {
    std::env::var("SPEC_RAY_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000)
}

/* -------------------------------------------------------------------------- */
/*        path classifiers — built from the C library's OWN exports           */
/* -------------------------------------------------------------------------- */
//
// These reproduce only the *branch conditions* of `c_src/src/lib.c`; every
// arithmetic step is delegated to the C `.so` itself (`c2Norm`, `c2MulmvT`,
// `c2AABBtoPoint`, ...), so a classifier cannot silently disagree with the
// reference implementation.  They exist purely to prove that a `CONFIGS.md` row
// really exercises the sub-path it claims to (coverage histograms below).

pub mod paths {
    use super::*;

    /// `c2RaytoCircle` sub-paths (axis D).
    pub const CIRCLE_DISC_NEG: usize = 0;
    pub const CIRCLE_T_NEG: usize = 1;
    pub const CIRCLE_T_BEYOND: usize = 2;
    pub const CIRCLE_HIT: usize = 3;
    pub const CIRCLE_NAN: usize = 4;
    pub const CIRCLE_NPATH: usize = 5;
    pub const CIRCLE_NAMES: [&str; CIRCLE_NPATH] =
        ["disc<0", "t<0", "t>A.t", "HIT", "nan"];

    pub fn circle_path(ray: C2Ray, c: C2Circle) -> usize {
        let p = apis();
        unsafe {
            let m = (p.c.c2Sub)(ray.p, c.p);
            let cc = (p.c.c2Dot)(m, m) - c.r * c.r;
            let b = (p.c.c2Dot)(m, ray.d);
            let disc = b * b - cc;
            if disc < 0.0 {
                return CIRCLE_DISC_NEG;
            }
            let t = -b - disc.sqrt();
            if t.is_nan() || ray.t.is_nan() {
                return CIRCLE_NAN;
            }
            if t >= 0.0 && t <= ray.t {
                CIRCLE_HIT
            } else if t < 0.0 {
                CIRCLE_T_NEG
            } else {
                CIRCLE_T_BEYOND
            }
        }
    }

    /// `c2RaytoAABB` sub-paths (axes E+F).
    pub const AABB_BROAD_REJECT: usize = 0;
    pub const AABB_SAT_REJECT: usize = 1;
    pub const AABB_NO_HIT: usize = 2;
    pub const AABB_WIN_T0: usize = 3;
    pub const AABB_WIN_T1: usize = 4;
    pub const AABB_WIN_T2: usize = 5;
    pub const AABB_WIN_T3: usize = 6;
    pub const AABB_NPATH: usize = 7;
    pub const AABB_NAMES: [&str; AABB_NPATH] = [
        "broadphase reject",
        "SAT reject",
        "no plane hit",
        "win t0 (-x)",
        "win t1 (+x)",
        "win t2 (-y)",
        "win t3 (+y)",
    ];

    fn plane1d(p: f32, n: f32, d: f32) -> f32 {
        p * n - d * n
    }
    fn raytoplane1d(da: f32, db: f32) -> f32 {
        if da < 0.0 {
            0.0
        } else if da * db > 0.0 {
            1.0
        } else {
            let d = da - db;
            if d != 0.0 {
                da / d
            } else {
                0.0
            }
        }
    }

    pub fn aabb_path(ray: C2Ray, b: C2AABB) -> usize {
        let p = apis();
        unsafe {
            let p0 = ray.p;
            let p1 = (p.c.c2Add)(ray.p, (p.c.c2Mulvs)(ray.d, ray.t));
            let a_box = C2AABB {
                min: (p.c.c2Minv)(p0, p1),
                max: (p.c.c2Maxv)(p0, p1),
            };
            if (p.c.c2AABBtoAABB)(a_box, b) == 0 {
                return AABB_BROAD_REJECT;
            }
            let ab = (p.c.c2Sub)(p1, p0);
            let n = (p.c.c2Skew)(ab);
            let abs_n = (p.c.c2Absv)(n);
            let half = (p.c.c2Mulvs)((p.c.c2Sub)(b.max, b.min), 0.5);
            let center = (p.c.c2Mulvs)((p.c.c2Add)(b.min, b.max), 0.5);
            let dot = (p.c.c2Dot)(n, (p.c.c2Sub)(p0, center));
            let d = (if dot < 0.0 { -dot } else { dot }) - (p.c.c2Dot)(abs_n, half);
            if d > 0.0 {
                return AABB_SAT_REJECT;
            }
            let mut t0 = raytoplane1d(plane1d(p0.x, -1.0, b.min.x), plane1d(p1.x, -1.0, b.min.x));
            let mut t1 = raytoplane1d(plane1d(p0.x, 1.0, b.max.x), plane1d(p1.x, 1.0, b.max.x));
            let mut t2 = raytoplane1d(plane1d(p0.y, -1.0, b.min.y), plane1d(p1.y, -1.0, b.min.y));
            let mut t3 = raytoplane1d(plane1d(p0.y, 1.0, b.max.y), plane1d(p1.y, 1.0, b.max.y));
            let h0 = (t0 <= 1.0) as i32;
            let h1 = (t1 <= 1.0) as i32;
            let h2 = (t2 <= 1.0) as i32;
            let h3 = (t3 <= 1.0) as i32;
            if (h0 | h1 | h2 | h3) == 0 {
                return AABB_NO_HIT;
            }
            t0 *= h0 as f32;
            t1 *= h1 as f32;
            t2 *= h2 as f32;
            t3 *= h3 as f32;
            if t0 >= t1 && t0 >= t2 && t0 >= t3 {
                AABB_WIN_T0
            } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
                AABB_WIN_T1
            } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
                AABB_WIN_T2
            } else {
                AABB_WIN_T3
            }
        }
    }

    /// `c2RaytoCapsule` sub-paths (axis G).
    pub const CAP_IN_BB: usize = 0;
    pub const CAP_IN_CAP_A: usize = 1;
    pub const CAP_IN_CAP_B: usize = 2;
    pub const CAP_DELEG_A_BY_X: usize = 3;
    pub const CAP_DELEG_B_BY_X: usize = 4;
    pub const CAP_DELEG_A_BY_Y: usize = 5;
    pub const CAP_DELEG_B_BY_Y: usize = 6;
    pub const CAP_SIDE_POS: usize = 7;
    pub const CAP_SIDE_NEG: usize = 8;
    pub const CAP_FALLTHROUGH: usize = 9;
    pub const CAP_NPATH: usize = 10;
    pub const CAP_NAMES: [&str; CAP_NPATH] = [
        "origin in rotated bb",
        "origin in cap a",
        "origin in cap b",
        "|yAp.x|<r -> circle a",
        "|yAp.x|<r -> circle b",
        "side plane, y<=0 -> circle a",
        "side plane, y>=yBb.y -> circle b",
        "side hit c>0 (n=M.x)",
        "side hit c<=0 (n=skew(M.y))",
        "fall through (return 0)",
    ];

    pub fn capsule_path(ray: C2Ray, b: C2Capsule) -> usize {
        let p = apis();
        unsafe {
            let my = (p.c.c2Norm)((p.c.c2Sub)(b.b, b.a));
            let mx = (p.c.c2CCW90)(my);
            let m = C2m { x: mx, y: my };
            let cap_n = (p.c.c2Sub)(b.b, b.a);
            let ybb = (p.c.c2MulmvT)(m, cap_n);
            let yap = (p.c.c2MulmvT)(m, (p.c.c2Sub)(ray.p, b.a));
            let yad = (p.c.c2MulmvT)(m, ray.d);
            let yae = (p.c.c2Add)(yap, (p.c.c2Mulvs)(yad, ray.t));
            let bb = C2AABB {
                min: (p.c.c2V)(-b.r, 0.0),
                max: (p.c.c2V)(b.r, ybb.y),
            };
            if (p.c.c2AABBtoPoint)(bb, yap) != 0 {
                return CAP_IN_BB;
            }
            let ca = C2Circle { p: b.a, r: b.r };
            let cb = C2Circle { p: b.b, r: b.r };
            if (p.c.c2CircleToPoint)(ca, ray.p) != 0 {
                return CAP_IN_CAP_A;
            }
            if (p.c.c2CircleToPoint)(cb, ray.p) != 0 {
                return CAP_IN_CAP_B;
            }
            let absx = |x: f32| if x < 0.0 { -x } else { x };
            let mn = {
                let (u, w) = (absx(yae.x), absx(yap.x));
                if u < w {
                    u
                } else {
                    w
                }
            };
            if yae.x * yap.x < 0.0 || mn < b.r {
                if absx(yap.x) < b.r {
                    if yap.y < 0.0 {
                        CAP_DELEG_A_BY_X
                    } else {
                        CAP_DELEG_B_BY_X
                    }
                } else {
                    let c = if yap.x > 0.0 { b.r } else { -b.r };
                    let d = yae.x - yap.x;
                    let t = (c - yap.x) / d;
                    let y = yap.y + (yae.y - yap.y) * t;
                    if y <= 0.0 {
                        CAP_DELEG_A_BY_Y
                    } else if y >= ybb.y {
                        CAP_DELEG_B_BY_Y
                    } else if c > 0.0 {
                        CAP_SIDE_POS
                    } else {
                        CAP_SIDE_NEG
                    }
                }
            } else {
                CAP_FALLTHROUGH
            }
        }
    }
}

/// Coverage histogram helper: counts how often each sub-path was taken and can
/// assert that the required ones were reached.
pub struct Cover {
    pub row: String,
    pub names: &'static [&'static str],
    pub hits: Vec<usize>,
}

impl Cover {
    pub fn new(row: &str, names: &'static [&'static str]) -> Cover {
        Cover {
            row: row.to_string(),
            names,
            hits: vec![0; names.len()],
        }
    }
    pub fn hit(&mut self, path: usize) {
        self.hits[path] += 1;
    }
    pub fn report(&self) {
        let parts: Vec<String> = self
            .names
            .iter()
            .zip(&self.hits)
            .map(|(n, c)| format!("{n}={c}"))
            .collect();
        eprintln!("[{}] path coverage: {}", self.row, parts.join(", "));
    }
    /// Assert the listed sub-paths were exercised at least `min` times.
    pub fn require(&self, required: &[usize], min: usize) {
        self.report();
        for &p in required {
            assert!(
                self.hits[p] >= min,
                "[{}] sub-path `{}` was exercised {} times, needed >= {}",
                self.row,
                self.names[p],
                self.hits[p],
                min
            );
        }
    }
}
