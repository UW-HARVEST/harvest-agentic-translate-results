//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries with `libloading` and calls every function
//! through raw `extern "C"` function pointers, so the Rust `#[no_mangle]`
//! export wrappers and the C ABI (struct-by-value classification, in
//! particular) are exercised exactly like an external C consumer would.
//!
//! Nothing in the Rust crate is ever called directly.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/* -------------------------------------------------------------------------- */
/* C-compatible types (mirror c_src/include/lib.h and c_src/src/lib.c)        */
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

/* -------------------------------------------------------------------------- */
/* The exported API surface, as raw function pointers                         */
/* -------------------------------------------------------------------------- */

#[derive(Copy, Clone)]
pub struct Api {
    pub name: &'static str,
    pub c2V: unsafe extern "C" fn(f32, f32) -> C2v,
    pub c2Dot: unsafe extern "C" fn(C2v, C2v) -> f32,
    pub c2Len: unsafe extern "C" fn(C2v) -> f32,
    pub c2Add: unsafe extern "C" fn(C2v, C2v) -> C2v,
    pub c2Sub: unsafe extern "C" fn(C2v, C2v) -> C2v,
    pub c2Mulvs: unsafe extern "C" fn(C2v, f32) -> C2v,
    pub c2Div: unsafe extern "C" fn(C2v, f32) -> C2v,
    pub c2Norm: unsafe extern "C" fn(C2v) -> C2v,
    pub c2Minv: unsafe extern "C" fn(C2v, C2v) -> C2v,
    pub c2Maxv: unsafe extern "C" fn(C2v, C2v) -> C2v,
    pub c2Skew: unsafe extern "C" fn(C2v) -> C2v,
    pub c2Absv: unsafe extern "C" fn(C2v) -> C2v,
    pub c2CCW90: unsafe extern "C" fn(C2v) -> C2v,
    pub c2MulmvT: unsafe extern "C" fn(C2m, C2v) -> C2v,
    pub c2AABBtoAABB: unsafe extern "C" fn(C2AABB, C2AABB) -> c_int,
    pub c2AABBtoPoint: unsafe extern "C" fn(C2AABB, C2v) -> c_int,
    pub c2CircleToPoint: unsafe extern "C" fn(C2Circle, C2v) -> c_int,
    pub c2RaytoCircle: unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int,
    pub c2RaytoAABB: unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int,
    pub c2RaytoCapsule: unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int,
    pub c2CastRay: unsafe extern "C" fn(C2Ray, *const c_void, c_int, *mut C2Raycast) -> c_int,
    pub spec_ray: unsafe extern "C" fn(*mut C2Raycast, f32, f32, f32, f32, f32, f32, f32) -> c_int,
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol `{}` missing from shared library: {e}",
            String::from_utf8_lossy(name)
        )
    });
    *s
}

fn load(path: &Path, name: &'static str) -> Api {
    let lib: &'static Library = Box::leak(Box::new(
        unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display())),
    ));
    unsafe {
        Api {
            name,
            c2V: sym(lib, b"c2V\0"),
            c2Dot: sym(lib, b"c2Dot\0"),
            c2Len: sym(lib, b"c2Len\0"),
            c2Add: sym(lib, b"c2Add\0"),
            c2Sub: sym(lib, b"c2Sub\0"),
            c2Mulvs: sym(lib, b"c2Mulvs\0"),
            c2Div: sym(lib, b"c2Div\0"),
            c2Norm: sym(lib, b"c2Norm\0"),
            c2Minv: sym(lib, b"c2Minv\0"),
            c2Maxv: sym(lib, b"c2Maxv\0"),
            c2Skew: sym(lib, b"c2Skew\0"),
            c2Absv: sym(lib, b"c2Absv\0"),
            c2CCW90: sym(lib, b"c2CCW90\0"),
            c2MulmvT: sym(lib, b"c2MulmvT\0"),
            c2AABBtoAABB: sym(lib, b"c2AABBtoAABB\0"),
            c2AABBtoPoint: sym(lib, b"c2AABBtoPoint\0"),
            c2CircleToPoint: sym(lib, b"c2CircleToPoint\0"),
            c2RaytoCircle: sym(lib, b"c2RaytoCircle\0"),
            c2RaytoAABB: sym(lib, b"c2RaytoAABB\0"),
            c2RaytoCapsule: sym(lib, b"c2RaytoCapsule\0"),
            c2CastRay: sym(lib, b"c2CastRay\0"),
            spec_ray: sym(lib, b"spec_ray\0"),
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — the name is derived from the parent
/// directory by CMakeLists.txt, so search for any `.so` in the build dir.
pub fn c_so_path() -> PathBuf {
    // `DIFF_C_SO` allows pointing the harness at an alternative build of the C
    // reference (e.g. an -O2 build) without touching c_src/.
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no .so found in {} — build the C reference first:\n  cd c_src && mkdir -p build && cd build && \\\n  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// `target/<profile>/libspec_ray_lib.so`, found relative to the test binary
/// (`target/<profile>/deps/<test>-<hash>`).
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        let cand = d.join("libspec_ray_lib.so");
        if cand.is_file() {
            return cand;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    panic!("libspec_ray_lib.so not found next to {} — run `cargo build` first", exe.display());
}

fn newest_mtime(dir: &Path) -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if p.file_name().and_then(|s| s.to_str()) != Some("build") {
                        stack.push(p);
                    }
                } else if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    if m > newest {
                        newest = m;
                    }
                }
            }
        }
    }
    newest
}

/// `cargo test` compiles the library for the *test* target but does **not**
/// re-link the `cdylib`, so a stale `.so` would silently be tested.  Fail loudly
/// instead.
fn assert_fresh(so: &Path, src: &Path, how_to_rebuild: &str) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let src_time = newest_mtime(src);
    assert!(
        so_time >= src_time,
        "{} is OLDER than the newest file in {} — the differential tests would \
         run against a stale library.\nRebuild first:\n  {how_to_rebuild}",
        so.display(),
        src.display()
    );
}

static PAIR: OnceLock<(Api, Api)> = OnceLock::new();

/// `(c_api, rust_api)`
pub fn apis() -> &'static (Api, Api) {
    PAIR.get_or_init(|| {
        let c_so = c_so_path();
        let r_so = rust_so_path();
        if std::env::var("DIFF_C_SO").is_err() {
            assert_fresh(
                &c_so,
                &manifest_dir().join("c_src"),
                "cd c_src/build && cmake --build .",
            );
        }
        assert_fresh(
            &r_so,
            &manifest_dir().join("src"),
            "cargo build            # cargo test alone does NOT re-link the cdylib",
        );
        let c = load(&c_so, "C");
        let r = load(&r_so, "RUST");
        (c, r)
    })
}

pub fn c_api() -> &'static Api {
    &apis().0
}
pub fn rust_api() -> &'static Api {
    &apis().1
}

/* -------------------------------------------------------------------------- */
/* Bit-exact comparison helpers                                               */
/* -------------------------------------------------------------------------- */

/// Sentinel pattern written into the out-parameter before every call, so that a
/// missing store or a spurious store is detected.
pub const SENT_T: u32 = 0xDEAD_BEEF;
pub const SENT_NX: u32 = 0xCAFE_BABE;
pub const SENT_NY: u32 = 0x1234_5678;

pub fn sentinel() -> C2Raycast {
    C2Raycast {
        t: f32::from_bits(SENT_T),
        n: C2v {
            x: f32::from_bits(SENT_NX),
            y: f32::from_bits(SENT_NY),
        },
    }
}

pub fn fbits(x: f32) -> u32 {
    x.to_bits()
}

pub fn fshow(x: f32) -> String {
    format!("{:e}[{:#010x}]", x, x.to_bits())
}

pub fn vshow(a: C2v) -> String {
    format!("({}, {})", fshow(a.x), fshow(a.y))
}

pub fn castshow(c: &C2Raycast) -> String {
    format!("{{t: {}, n: {}}}", fshow(c.t), vshow(c.n))
}

pub fn f_eq_bits(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn v_eq_bits(a: C2v, b: C2v) -> bool {
    f_eq_bits(a.x, b.x) && f_eq_bits(a.y, b.y)
}

pub fn cast_eq_bits(a: &C2Raycast, b: &C2Raycast) -> bool {
    f_eq_bits(a.t, b.t) && v_eq_bits(a.n, b.n)
}

/// Accumulates mismatches so a whole configuration row is reported at once.
pub struct Diff {
    pub row: String,
    pub checked: usize,
    pub failures: Vec<String>,
    pub n_failed: usize,
    pub hits: usize,
    pub misses: usize,
    pub tags: std::collections::BTreeMap<String, usize>,
}

impl Diff {
    pub fn new(row: &str) -> Self {
        Diff {
            row: row.to_string(),
            checked: 0,
            failures: Vec::new(),
            n_failed: 0,
            hits: 0,
            misses: 0,
            tags: std::collections::BTreeMap::new(),
        }
    }

    pub fn tag(&mut self, t: impl Into<String>) {
        *self.tags.entry(t.into()).or_insert(0) += 1;
    }

    /// Assert that the generator really exercised the branch this row is about.
    pub fn require_tag(&mut self, t: &str, min: usize) {
        let got = self.tags.get(t).copied().unwrap_or(0);
        assert!(
            got >= min,
            "row `{}`: expected >= {min} cases tagged `{t}`, got {got}. tags = {:?}",
            self.row,
            self.tags
        );
    }

    pub fn require_hits(&mut self, min: usize) {
        assert!(
            self.hits >= min,
            "row `{}`: expected >= {min} hits (rc==1), got {} of {}",
            self.row,
            self.hits,
            self.checked
        );
    }

    pub fn require_misses(&mut self, min: usize) {
        assert!(
            self.misses >= min,
            "row `{}`: expected >= {min} misses (rc==0), got {} of {}",
            self.row,
            self.misses,
            self.checked
        );
    }

    pub fn check(&mut self, ok: bool, detail: impl FnOnce() -> String) {
        self.checked += 1;
        if !ok {
            self.n_failed += 1;
            if self.failures.len() < 8 {
                self.failures.push(detail());
            }
        }
    }

    /// int + out-parameter comparison in one shot.
    pub fn check_call(
        &mut self,
        inputs: impl FnOnce() -> String,
        (rc_c, out_c): (c_int, C2Raycast),
        (rc_r, out_r): (c_int, C2Raycast),
    ) {
        if rc_c == 1 {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        let ok = rc_c == rc_r && cast_eq_bits(&out_c, &out_r);
        self.check(ok, || {
            format!(
                "inputs {}\n      C   -> rc={rc_c} out={}\n      RUST-> rc={rc_r} out={}",
                inputs(),
                castshow(&out_c),
                castshow(&out_r)
            )
        });
    }

    pub fn finish(self) {
        assert!(self.checked > 0, "row `{}` checked nothing", self.row);
        if !self.failures.is_empty() {
            panic!(
                "row `{}`: {} of {} cases diverged:\n  - {}",
                self.row,
                self.n_failed,
                self.checked,
                self.failures.join("\n  - ")
            );
        }
        println!("row `{}` OK ({} randomized cases)", self.row, self.checked);
    }
}

/* -------------------------------------------------------------------------- */
/* Deterministic PRNG (xorshift64* — fixed seed, reproducible)                */
/* -------------------------------------------------------------------------- */

pub struct Rng(u64);

impl Rng {
    /// Fixed seed per row (reproducible).  `DIFF_SEED_OFFSET=<n>` re-runs the
    /// whole suite with a different but equally reproducible input stream, which
    /// is used to confirm that no row passes by seed luck.
    pub fn new(seed: u64) -> Self {
        let seed = seed.wrapping_add(
            std::env::var("DIFF_SEED_OFFSET")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
                .wrapping_mul(0x9E37_79B9),
        );
        // splitmix the seed so that neighbouring seeds are not correlated
        let mut z = seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0x1234_5678_9ABC_DEF0);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng(z ^ (z >> 31) | 1)
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

    pub fn chance(&mut self, one_in: u32) -> bool {
        self.below(one_in) == 0
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + (hi - lo) * u
    }

    /// Any bit pattern at all: finite, denormal, ±inf, quiet/signalling NaN.
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// One of the hand-picked corner values.
    pub fn special(&mut self) -> f32 {
        const S: [u32; 20] = [
            0x0000_0000, // +0.0
            0x8000_0000, // -0.0
            0x0000_0001, // smallest denormal
            0x8000_0001, // -smallest denormal
            0x007F_FFFF, // largest denormal
            0x0080_0000, // f32::MIN_POSITIVE
            0x3F80_0000, // 1.0
            0xBF80_0000, // -1.0
            0x3F00_0000, // 0.5
            0x4000_0000, // 2.0
            0x7F7F_FFFF, // f32::MAX
            0xFF7F_FFFF, // -f32::MAX
            0x7F80_0000, // +inf
            0xFF80_0000, // -inf
            0x7FC0_0000, // +qNaN
            0xFFC0_0000, // -qNaN ("real indefinite")
            0x7FA0_0000, // +sNaN
            0xFFA0_0000, // -sNaN
            0x4B7F_FFFF, // 16777215.0 (last exact integer)
            0x3333_3333, // 4.17e-8
        ];
        f32::from_bits(S[self.below(S.len() as u32) as usize])
    }

    /// Mostly ordinary values, sometimes a corner value, rarely pure noise.
    pub fn mixed(&mut self) -> f32 {
        match self.below(16) {
            0 | 1 => self.special(),
            2 => self.any_bits(),
            3 => self.range(-1e30, 1e30),
            4 => self.range(-1e-20, 1e-20),
            _ => self.range(-100.0, 100.0),
        }
    }

    pub fn ordinary(&mut self) -> f32 {
        self.range(-100.0, 100.0)
    }

    pub fn v_ordinary(&mut self) -> C2v {
        v(self.ordinary(), self.ordinary())
    }

    pub fn v_mixed(&mut self) -> C2v {
        v(self.mixed(), self.mixed())
    }

    pub fn v_any_bits(&mut self) -> C2v {
        v(self.any_bits(), self.any_bits())
    }

    pub fn v_special(&mut self) -> C2v {
        v(self.special(), self.special())
    }

    /// Uniformly distributed unit-ish direction (computed in f32, like the C).
    pub fn dir(&mut self) -> C2v {
        let a = self.range(-3.141_592_7, 3.141_592_7);
        v(a.cos(), a.sin())
    }
}

/* -------------------------------------------------------------------------- */
/* One-line differential drivers                                             */
/* -------------------------------------------------------------------------- */

pub fn call_raytocircle(api: &Api, ray: C2Ray, c: C2Circle) -> (c_int, C2Raycast) {
    let mut out = sentinel();
    let rc = unsafe { (api.c2RaytoCircle)(ray, c, &mut out) };
    (rc, out)
}

pub fn call_raytoaabb(api: &Api, ray: C2Ray, b: C2AABB) -> (c_int, C2Raycast) {
    let mut out = sentinel();
    let rc = unsafe { (api.c2RaytoAABB)(ray, b, &mut out) };
    (rc, out)
}

pub fn call_raytocapsule(api: &Api, ray: C2Ray, b: C2Capsule) -> (c_int, C2Raycast) {
    let mut out = sentinel();
    let rc = unsafe { (api.c2RaytoCapsule)(ray, b, &mut out) };
    (rc, out)
}

pub fn call_castray_circle(api: &Api, ray: C2Ray, c: C2Circle, ty: c_int) -> (c_int, C2Raycast) {
    let mut out = sentinel();
    let rc = unsafe { (api.c2CastRay)(ray, &c as *const C2Circle as *const c_void, ty, &mut out) };
    (rc, out)
}

pub fn call_castray_aabb(api: &Api, ray: C2Ray, b: C2AABB, ty: c_int) -> (c_int, C2Raycast) {
    let mut out = sentinel();
    let rc = unsafe { (api.c2CastRay)(ray, &b as *const C2AABB as *const c_void, ty, &mut out) };
    (rc, out)
}

pub fn call_castray_capsule(api: &Api, ray: C2Ray, b: C2Capsule, ty: c_int) -> (c_int, C2Raycast) {
    let mut out = sentinel();
    let rc = unsafe { (api.c2CastRay)(ray, &b as *const C2Capsule as *const c_void, ty, &mut out) };
    (rc, out)
}

#[allow(clippy::too_many_arguments)]
pub fn call_spec_ray(
    api: &Api,
    mp_x: f32,
    mp_y: f32,
    c_p_x: f32,
    c_p_y: f32,
    c_r: f32,
    r_p_x: f32,
    r_p_y: f32,
) -> (c_int, C2Raycast) {
    let mut out = sentinel();
    let rc = unsafe { (api.spec_ray)(&mut out, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };
    (rc, out)
}

pub fn rayshow(r: &C2Ray) -> String {
    format!("Ray{{p: {}, d: {}, t: {}}}", vshow(r.p), vshow(r.d), fshow(r.t))
}

pub fn circshow(c: &C2Circle) -> String {
    format!("Circle{{p: {}, r: {}}}", vshow(c.p), fshow(c.r))
}

pub fn aabbshow(b: &C2AABB) -> String {
    format!("AABB{{min: {}, max: {}}}", vshow(b.min), vshow(b.max))
}

pub fn capshow(c: &C2Capsule) -> String {
    format!("Capsule{{a: {}, b: {}, r: {}}}", vshow(c.a), vshow(c.b), fshow(c.r))
}

/// Next representable f32 towards +inf (used for one-ulp boundary probing).
pub fn next_up(x: f32) -> f32 {
    if x.is_nan() || x == f32::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let b = x.to_bits();
    if x > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

/// Next representable f32 towards -inf.
pub fn next_down(x: f32) -> f32 {
    if x.is_nan() || x == f32::NEG_INFINITY {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(0x8000_0001);
    }
    let b = x.to_bits();
    if x > 0.0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

/* -------------------------------------------------------------------------- */
/* Exact branch classification, computed with the C library's OWN primitives  */
/* (so the classification cannot drift from the C arithmetic).                */
/* -------------------------------------------------------------------------- */

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AabbBranch {
    /// swept-bbox test rejected (`lib.c:145`)
    BboxReject,
    /// separating axis rejected (`lib.c:156`)
    SepAxisReject,
    /// `hit == 0` (`lib.c:194`)
    NoPlaneHit,
    /// hit, `out->n == (-1, 0)`
    FaceNegX,
    /// hit, `out->n == (1, 0)`
    FacePosX,
    /// hit, `out->n == (0, -1)`
    FaceNegY,
    /// hit, `out->n == (0, 1)`
    FacePosY,
}

impl AabbBranch {
    pub fn name(self) -> &'static str {
        match self {
            AabbBranch::BboxReject => "bbox_reject",
            AabbBranch::SepAxisReject => "sep_axis_reject",
            AabbBranch::NoPlaneHit => "no_plane_hit",
            AabbBranch::FaceNegX => "face_-x",
            AabbBranch::FacePosX => "face_+x",
            AabbBranch::FaceNegY => "face_-y",
            AabbBranch::FacePosY => "face_+y",
        }
    }
}

fn tmin(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
fn tabs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

/// Mirrors `c2RaytoAABB`'s control flow using the C library's exported helpers.
pub fn classify_aabb(api: &Api, ray: C2Ray, b: C2AABB, rc: c_int, out: &C2Raycast) -> AabbBranch {
    unsafe {
        let p0 = ray.p;
        let p1 = (api.c2Add)(ray.p, (api.c2Mulvs)(ray.d, ray.t));
        let a_box = C2AABB {
            min: (api.c2Minv)(p0, p1),
            max: (api.c2Maxv)(p0, p1),
        };
        if (api.c2AABBtoAABB)(a_box, b) == 0 {
            return AabbBranch::BboxReject;
        }
        let ab = (api.c2Sub)(p1, p0);
        let n = (api.c2Skew)(ab);
        let abs_n = (api.c2Absv)(n);
        let half = (api.c2Mulvs)((api.c2Sub)(b.max, b.min), 0.5);
        let center = (api.c2Mulvs)((api.c2Add)(b.min, b.max), 0.5);
        let d = tabs((api.c2Dot)(n, (api.c2Sub)(p0, center))) - (api.c2Dot)(abs_n, half);
        if d > 0.0 {
            return AabbBranch::SepAxisReject;
        }
        if rc == 0 {
            return AabbBranch::NoPlaneHit;
        }
        let nn = out.n;
        if nn.x.to_bits() == (-1.0f32).to_bits() {
            AabbBranch::FaceNegX
        } else if nn.x.to_bits() == 1.0f32.to_bits() {
            AabbBranch::FacePosX
        } else if nn.y.to_bits() == (-1.0f32).to_bits() {
            AabbBranch::FaceNegY
        } else {
            AabbBranch::FacePosY
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CapBranch {
    /// `c2AABBtoPoint(capsule_bb, yAp)` (`lib.c:245`)
    InSlabBox,
    /// origin inside cap circle `a` (`lib.c:254`)
    InCapA,
    /// origin inside cap circle `b` (`lib.c:256`)
    InCapB,
    /// `|yAp.x| < r`, `yAp.y < 0` ⇒ `c2RaytoCircle(Ca)` (`lib.c:271-272`)
    NearAxisCa,
    /// `|yAp.x| < r`, `yAp.y >= 0` ⇒ `c2RaytoCircle(Cb)` (`lib.c:273-274`)
    NearAxisCb,
    /// slab crossing with `y <= 0` ⇒ `c2RaytoCircle(Ca)` (`lib.c:280-281`)
    CrossCa,
    /// slab crossing with `y >= yBb.y` ⇒ `c2RaytoCircle(Cb)` (`lib.c:282-283`)
    CrossCb,
    /// flat side, `c > 0` ⇒ `out->n = M.x` (`lib.c:285`)
    SidePos,
    /// flat side, `c <= 0` ⇒ `out->n = c2Skew(M.y)` (`lib.c:285`)
    SideNeg,
    /// final `return 0` (`lib.c:291`)
    Outside,
}

impl CapBranch {
    pub fn name(self) -> &'static str {
        match self {
            CapBranch::InSlabBox => "in_slab_box",
            CapBranch::InCapA => "in_cap_a",
            CapBranch::InCapB => "in_cap_b",
            CapBranch::NearAxisCa => "near_axis_Ca",
            CapBranch::NearAxisCb => "near_axis_Cb",
            CapBranch::CrossCa => "cross_Ca",
            CapBranch::CrossCb => "cross_Cb",
            CapBranch::SidePos => "side_+",
            CapBranch::SideNeg => "side_-",
            CapBranch::Outside => "outside",
        }
    }
}

/// Mirrors `c2RaytoCapsule`'s control flow using the C library's exported helpers.
pub fn classify_capsule(api: &Api, ray: C2Ray, cap: C2Capsule) -> CapBranch {
    unsafe {
        let my = (api.c2Norm)((api.c2Sub)(cap.b, cap.a));
        let mx = (api.c2CCW90)(my);
        let m = C2m { x: mx, y: my };
        let cap_n = (api.c2Sub)(cap.b, cap.a);
        let ybb = (api.c2MulmvT)(m, cap_n);
        let yap = (api.c2MulmvT)(m, (api.c2Sub)(ray.p, cap.a));
        let yad = (api.c2MulmvT)(m, ray.d);
        let yae = (api.c2Add)(yap, (api.c2Mulvs)(yad, ray.t));
        let bb = C2AABB {
            min: (api.c2V)(-cap.r, 0.0),
            max: (api.c2V)(cap.r, ybb.y),
        };
        if (api.c2AABBtoPoint)(bb, yap) != 0 {
            return CapBranch::InSlabBox;
        }
        if (api.c2CircleToPoint)(C2Circle { p: cap.a, r: cap.r }, ray.p) != 0 {
            return CapBranch::InCapA;
        }
        if (api.c2CircleToPoint)(C2Circle { p: cap.b, r: cap.r }, ray.p) != 0 {
            return CapBranch::InCapB;
        }
        if yae.x * yap.x < 0.0 || tmin(tabs(yae.x), tabs(yap.x)) < cap.r {
            if tabs(yap.x) < cap.r {
                if yap.y < 0.0 {
                    return CapBranch::NearAxisCa;
                }
                return CapBranch::NearAxisCb;
            }
            let c = if yap.x > 0.0 { cap.r } else { -cap.r };
            let d = yae.x - yap.x;
            let t = (c - yap.x) / d;
            let y = yap.y + (yae.y - yap.y) * t;
            if y <= 0.0 {
                return CapBranch::CrossCa;
            }
            if y >= ybb.y {
                return CapBranch::CrossCb;
            }
            if c > 0.0 {
                return CapBranch::SidePos;
            }
            return CapBranch::SideNeg;
        }
        CapBranch::Outside
    }
}

/* -------------------------------------------------------------------------- */
/* Geometry construction helpers (only used to build inputs)                  */
/* -------------------------------------------------------------------------- */

pub fn vadd(a: C2v, b: C2v) -> C2v {
    v(a.x + b.x, a.y + b.y)
}
pub fn vsub(a: C2v, b: C2v) -> C2v {
    v(a.x - b.x, a.y - b.y)
}
pub fn vscale(a: C2v, s: f32) -> C2v {
    v(a.x * s, a.y * s)
}
pub fn vnorm(a: C2v) -> C2v {
    let l = (a.x * a.x + a.y * a.y).sqrt();
    vscale(a, 1.0 / l)
}

/// Local orthonormal frame of a capsule, matching `c2RaytoCapsule`'s `M`.
pub fn cap_frame(cap: &C2Capsule) -> (C2v, C2v) {
    let my = vnorm(vsub(cap.b, cap.a));
    let mx = v(my.y, -my.x);
    (mx, my)
}

/// Point at capsule-local coordinates `(lx, ly)` (x across, y along the axis).
pub fn cap_local_point(cap: &C2Capsule, lx: f32, ly: f32) -> C2v {
    let (mx, my) = cap_frame(cap);
    vadd(cap.a, vadd(vscale(mx, lx), vscale(my, ly)))
}

/// Direction with capsule-local components `(dx, dy)`.
pub fn cap_local_dir(cap: &C2Capsule, dx: f32, dy: f32) -> C2v {
    let (mx, my) = cap_frame(cap);
    vadd(vscale(mx, dx), vscale(my, dy))
}

pub fn cap_len(cap: &C2Capsule) -> f32 {
    let d = vsub(cap.b, cap.a);
    (d.x * d.x + d.y * d.y).sqrt()
}
