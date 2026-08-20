//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading`:
//!   * the C reference  -> `c_src/build/libtranslated_rust.so`
//!   * the Rust port     -> `target/<profile>/libaabb_lib.so`
//!
//! Nothing in the crate under test is ever called directly; every call goes
//! through the `.so` export, exactly like an external C consumer.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI types (mirrors of the C declarations in c_src/src/lib.c)
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
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
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

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
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

/// Raw bit pattern of every float in a value, so `-0.0 != 0.0` and NaN
/// payloads/signs are compared too.
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
impl Bits for c2Proxy {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.radius.to_bits(), self.count as u32];
        for e in self.verts.iter() {
            v.extend(e.bits());
        }
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
        for e in self.verts.iter() {
            v.extend(e.bits());
        }
        v.push(self.div.to_bits());
        v.push(self.count as u32);
        v
    }
}
impl Bits for c2GJKCache {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.metric.to_bits(), self.count as u32];
        for e in self.iA.iter() {
            v.push(*e as u32);
        }
        for e in self.iB.iter() {
            v.push(*e as u32);
        }
        v.push(self.div.to_bits());
        v
    }
}
impl Bits for () {
    fn bits(&self) -> Vec<u32> {
        Vec::new()
    }
}
impl Bits for c2Circle {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.p.bits();
        v.push(self.r.to_bits());
        v
    }
}
impl Bits for c2AABB {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.min.bits();
        v.extend(self.max.bits());
        v
    }
}
impl Bits for c2Capsule {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.a.bits();
        v.extend(self.b.bits());
        v.push(self.r.to_bits());
        v
    }
}
impl Bits for u32 {
    fn bits(&self) -> Vec<u32> {
        vec![*self]
    }
}
impl<T: Bits> Bits for Option<T> {
    fn bits(&self) -> Vec<u32> {
        match self {
            Some(x) => {
                let mut v = vec![1];
                v.extend(x.bits());
                v
            }
            None => vec![0],
        }
    }
}
impl<T: Bits> Bits for Vec<T> {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.len() as u32];
        for e in self.iter() {
            v.extend(e.bits());
        }
        v
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
impl<A: Bits, B: Bits, C: Bits, D: Bits, E: Bits> Bits for (A, B, C, D, E) {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.0.bits();
        v.extend(self.1.bits());
        v.extend(self.2.bits());
        v.extend(self.3.bits());
        v.extend(self.4.bits());
        v
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

/// `target/<profile>/libaabb_lib.so` — derived from the running test binary,
/// which lives in `target/<profile>/deps/`.
///
/// IMPORTANT: no test target *links* the crate under test (it is a pure
/// `cdylib` that we only ever `dlopen`), so cargo has no dependency edge from
/// the tests to the library and **`cargo test` alone will happily run against a
/// stale `.so`**.  The staleness guard below turns that silent-false-pass into a
/// hard failure; always run `cargo build` before `cargo test`
/// (`./run_verification.sh` does).
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|f| f == "deps").unwrap_or(false) {
        dir.pop();
    }
    rebuild_cdylib();
    let p = dir.join("libaabb_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {p:?} — run `cargo build` first"
    );
    assert_fresh(&p);
    p
}

/// Rebuild the cdylib before loading it.
///
/// Cargo's own fingerprinting is mtime-based, so a *content* change that leaves
/// the mtime alone (e.g. restoring a backup with `mv`, `git checkout`, a
/// mutation harness) does **not** trigger a rebuild — and because no test target
/// links the cdylib, `cargo test` would then dlopen an artifact built from
/// different source than the one on disk and pass vacuously.  Forcing the source
/// mtime forward and re-running `cargo build` here makes that impossible.
///
/// Set `HARNESS_NO_REBUILD=1` to skip (e.g. when the caller already built).
fn rebuild_cdylib() {
    use std::sync::OnceLock;
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        if std::env::var_os("HARNESS_NO_REBUILD").is_some() {
            return;
        }
        let dir = manifest_dir();
        // bump the mtime so cargo cannot mistake a content rollback for "fresh"
        let src = dir.join("src/lib.rs");
        let _ = std::fs::OpenOptions::new().append(true).open(&src).and_then(|f| {
            f.set_modified(std::time::SystemTime::now())
        });
        let out = std::process::Command::new(env!("CARGO"))
            .args(["build", "--offline"])
            .current_dir(&dir)
            .output();
        let out = match out {
            Ok(o) if o.status.success() => o,
            Ok(_) => std::process::Command::new(env!("CARGO"))
                .args(["build"])
                .current_dir(&dir)
                .output()
                .expect("cargo build"),
            Err(e) => panic!("could not run cargo build: {e}"),
        };
        assert!(
            out.status.success(),
            "cargo build of the cdylib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

/// Panic if the `.so` is older than any source file it is built from.
fn assert_fresh(so: &std::path::Path) {
    let so_t = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat .so");
    for rel in ["src/lib.rs", "Cargo.toml"] {
        let src = manifest_dir().join(rel);
        let src_t = match std::fs::metadata(&src).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        assert!(
            so_t >= src_t,
            "STALE ARTIFACT: {so:?} is older than {src:?}.\n\
             `cargo test` does not rebuild a cdylib that no test target links \
             against — run `cargo build` (or ./run_verification.sh) first, \
             otherwise the differential tests silently compare against an old \
             build of the Rust library."
        );
    }
}

pub struct Libs {
    pub c: Library,
    pub r: Library,
}

impl Libs {
    pub fn load() -> Libs {
        let cp = c_so_path();
        assert!(
            cp.exists(),
            "C shared object not found at {cp:?} — build it with cmake first"
        );
        unsafe {
            Libs {
                c: Library::new(&cp).expect("load C .so"),
                r: Library::new(rust_so_path()).expect("load Rust .so"),
            }
        }
    }

    pub fn sym<T>(&self, which: Side, name: &str) -> Symbol<'_, T> {
        let lib = match which {
            Side::C => &self.c,
            Side::R => &self.r,
        };
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        unsafe { lib.get(&bytes).unwrap_or_else(|e| panic!("symbol {name}: {e}")) }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    C,
    R,
}

pub const SIDES: [Side; 2] = [Side::C, Side::R];

/// Global, lazily-initialised handle pair (loading twice per process is fine
/// but wasteful).
pub fn libs() -> &'static Libs {
    use std::sync::OnceLock;
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(Libs::load)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xoshiro-ish; fixed seed => reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[-mag, mag]`.
    pub fn f32_range(&mut self, mag: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * mag
    }
    /// Small integer-valued float — makes exact ties / degenerate simplices
    /// (`u == 0`, `area == 0`, duplicate support points) actually happen.
    pub fn f32_grid(&mut self, mag: i32) -> f32 {
        let m = (2 * mag + 1) as u32;
        (self.below(m) as i32 - mag) as f32
    }
    /// Fully arbitrary bit pattern: NaNs (any payload/sign), infinities,
    /// denormals, huge magnitudes.
    pub fn f32_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// One of the classic hand-picked boundary values.
    pub fn f32_special(&mut self) -> f32 {
        const S: [f32; 18] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::EPSILON,
            -f32::EPSILON,
            1.0e8,
            -1.0e8,
            1.0e-30,
        ];
        let i = self.below(S.len() as u32) as usize;
        let v = S[i];
        if i == 8 && self.next_u32() & 1 == 1 {
            -v // exercise the sign bit of NaN as well
        } else {
            v
        }
    }

    /// Mixed generator: mostly well-behaved, but regularly hits grid values,
    /// specials and raw bit patterns.
    pub fn f32_mixed(&mut self, mag: f32) -> f32 {
        match self.below(10) {
            0..=4 => self.f32_range(mag),
            5..=6 => self.f32_grid(3),
            7..=8 => self.f32_special(),
            _ => self.f32_bits(),
        }
    }

    /// Well-behaved geometry only (finite, moderate) — used where NaN/Inf would
    /// merely drown the interesting geometric branches.
    pub fn f32_geom(&mut self, mag: f32) -> f32 {
        match self.below(4) {
            0 => self.f32_grid(4),
            _ => self.f32_range(mag),
        }
    }

    pub fn v_mixed(&mut self, mag: f32) -> c2v {
        c2v {
            x: self.f32_mixed(mag),
            y: self.f32_mixed(mag),
        }
    }
    pub fn v_geom(&mut self, mag: f32) -> c2v {
        c2v {
            x: self.f32_geom(mag),
            y: self.f32_geom(mag),
        }
    }
    pub fn r_geom(&mut self) -> c2r {
        // Real rotations plus deliberately non-normalised ones.
        match self.below(4) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => {
                let a = self.f32_range(3.2);
                c2r {
                    c: a.cos(),
                    s: a.sin(),
                }
            }
            2 => c2r {
                c: self.f32_grid(2),
                s: self.f32_grid(2),
            },
            _ => c2r {
                c: self.f32_range(4.0),
                s: self.f32_range(4.0),
            },
        }
    }
    pub fn x_geom(&mut self) -> c2x {
        c2x {
            p: self.v_geom(60.0),
            r: self.r_geom(),
        }
    }
    pub fn circle(&mut self, mag: f32) -> c2Circle {
        c2Circle {
            p: self.v_geom(mag),
            r: self.f32_geom(mag * 0.4).abs(),
        }
    }
    pub fn aabb(&mut self, mag: f32) -> c2AABB {
        // Half of the time a properly ordered box, half of the time whatever
        // came out of the generator (inverted / degenerate boxes included).
        let a = self.v_geom(mag);
        let b = self.v_geom(mag);
        if self.next_u32() & 1 == 0 {
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
        } else {
            c2AABB { min: a, max: b }
        }
    }
    pub fn capsule(&mut self, mag: f32) -> c2Capsule {
        let a = self.v_geom(mag);
        // Frequently make a degenerate capsule (a == b) — that is the `n == 0`
        // branch inside c2CircletoCapsule and a duplicated support point in GJK.
        let b = if self.below(8) == 0 { a } else { self.v_geom(mag) };
        c2Capsule {
            a,
            b,
            r: self.f32_geom(mag * 0.4).abs(),
        }
    }
    pub fn sv(&mut self, mag: f32) -> c2sv {
        c2sv {
            sA: self.v_geom(mag),
            sB: self.v_geom(mag),
            p: self.v_geom(mag),
            u: self.f32_geom(mag),
            iA: self.below(4) as i32,
            iB: self.below(4) as i32,
        }
    }
    /// A fully populated simplex with the requested `count`.
    pub fn simplex(&mut self, count: i32, mag: f32) -> c2Simplex {
        let mut s = c2Simplex {
            verts: [self.sv(mag), self.sv(mag), self.sv(mag), self.sv(mag)],
            div: 0.0,
            count,
        };
        s.div = match self.below(6) {
            0 => 0.0,
            1 => 1.0,
            _ => self.f32_geom(mag),
        };
        s
    }
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_same<T: Bits + std::fmt::Debug>(what: &str, case: &dyn std::fmt::Debug, c: T, r: T) {
    if c.bits() != r.bits() {
        panic!(
            "DIVERGENCE in {what}\n  input : {case:?}\n  C     : {c:?}\n  rust  : {r:?}\n  C bits: {:08x?}\n  R bits: {:08x?}",
            c.bits(),
            r.bits()
        );
    }
}

pub const C2_TYPES: [C2_TYPE; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// Enum values with no valid variant that a C caller can legally pass.
pub const C2_BAD_TYPES: [C2_TYPE; 6] = [3, 4, 7, 100, 0x7FFF_FFFF, 0xFFFF_FFFF];

pub type VoidP = *const c_void;

// ---------------------------------------------------------------------------
// Typed view of every exported symbol, resolved once per side.
// ---------------------------------------------------------------------------

pub type FnV_f = unsafe extern "C" fn(c2v) -> f32;
pub type FnV_v = unsafe extern "C" fn(c2v) -> c2v;
pub type FnVV_v = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVV_f = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnSimplex_v = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplex_f = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplex = unsafe extern "C" fn(*mut c2Simplex);

pub struct Api {
    pub c2V: Symbol<'static, unsafe extern "C" fn(f32, f32) -> c2v>,
    pub c2Mulvs: Symbol<'static, unsafe extern "C" fn(c2v, f32) -> c2v>,
    pub c2Maxv: Symbol<'static, FnVV_v>,
    pub c2Minv: Symbol<'static, FnVV_v>,
    pub c2Clampv: Symbol<'static, unsafe extern "C" fn(c2v, c2v, c2v) -> c2v>,
    pub c2Sub: Symbol<'static, FnVV_v>,
    pub c2Add: Symbol<'static, FnVV_v>,
    pub c2Dot: Symbol<'static, FnVV_f>,
    pub c2Det2: Symbol<'static, FnVV_f>,
    pub c2Len: Symbol<'static, FnV_f>,
    pub c2Neg: Symbol<'static, FnV_v>,
    pub c2Skew: Symbol<'static, FnV_v>,
    pub c2CCW90: Symbol<'static, FnV_v>,
    pub c2Norm: Symbol<'static, FnV_v>,
    pub c2Div: Symbol<'static, unsafe extern "C" fn(c2v, f32) -> c2v>,
    pub c2RotIdentity: Symbol<'static, unsafe extern "C" fn() -> c2r>,
    pub c2xIdentity: Symbol<'static, unsafe extern "C" fn() -> c2x>,
    pub c2Mulrv: Symbol<'static, unsafe extern "C" fn(c2r, c2v) -> c2v>,
    pub c2MulrvT: Symbol<'static, unsafe extern "C" fn(c2r, c2v) -> c2v>,
    pub c2Mulxv: Symbol<'static, unsafe extern "C" fn(c2x, c2v) -> c2v>,
    pub c2BBVerts: Symbol<'static, unsafe extern "C" fn(*mut c2v, *mut c2AABB)>,
    pub c2MakeProxy: Symbol<'static, unsafe extern "C" fn(*const c_void, C2_TYPE, *mut c2Proxy)>,
    pub c2GJKSimplexMetric: Symbol<'static, FnSimplex_f>,
    pub c22: Symbol<'static, FnSimplex>,
    pub c23: Symbol<'static, FnSimplex>,
    pub c2D: Symbol<'static, FnSimplex_v>,
    pub c2L: Symbol<'static, FnSimplex_v>,
    pub c2Support: Symbol<'static, unsafe extern "C" fn(*const c2v, i32, c2v) -> i32>,
    pub c2Witness: Symbol<'static, unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)>,
    pub c2GJK: Symbol<
        'static,
        unsafe extern "C" fn(
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
        ) -> f32,
    >,
    pub c2AABBtoAABB: Symbol<'static, unsafe extern "C" fn(c2AABB, c2AABB) -> i32>,
    pub c2AABBtoCapsule: Symbol<'static, unsafe extern "C" fn(c2AABB, c2Capsule) -> i32>,
    pub c2CapsuletoCapsule: Symbol<'static, unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32>,
    pub c2CircletoCircle: Symbol<'static, unsafe extern "C" fn(c2Circle, c2Circle) -> i32>,
    pub c2CircletoAABB: Symbol<'static, unsafe extern "C" fn(c2Circle, c2AABB) -> i32>,
    pub c2CircletoCapsule: Symbol<'static, unsafe extern "C" fn(c2Circle, c2Capsule) -> i32>,
    pub c2Collided:
        Symbol<'static, unsafe extern "C" fn(*const c_void, C2_TYPE, *const c_void, C2_TYPE) -> i32>,
    pub aabb: Symbol<'static, unsafe extern "C" fn(f32, f32, f32, f32) -> i32>,
}

impl Api {
    fn build(side: Side) -> Api {
        let l = libs();
        Api {
            c2V: l.sym(side, "c2V"),
            c2Mulvs: l.sym(side, "c2Mulvs"),
            c2Maxv: l.sym(side, "c2Maxv"),
            c2Minv: l.sym(side, "c2Minv"),
            c2Clampv: l.sym(side, "c2Clampv"),
            c2Sub: l.sym(side, "c2Sub"),
            c2Add: l.sym(side, "c2Add"),
            c2Dot: l.sym(side, "c2Dot"),
            c2Det2: l.sym(side, "c2Det2"),
            c2Len: l.sym(side, "c2Len"),
            c2Neg: l.sym(side, "c2Neg"),
            c2Skew: l.sym(side, "c2Skew"),
            c2CCW90: l.sym(side, "c2CCW90"),
            c2Norm: l.sym(side, "c2Norm"),
            c2Div: l.sym(side, "c2Div"),
            c2RotIdentity: l.sym(side, "c2RotIdentity"),
            c2xIdentity: l.sym(side, "c2xIdentity"),
            c2Mulrv: l.sym(side, "c2Mulrv"),
            c2MulrvT: l.sym(side, "c2MulrvT"),
            c2Mulxv: l.sym(side, "c2Mulxv"),
            c2BBVerts: l.sym(side, "c2BBVerts"),
            c2MakeProxy: l.sym(side, "c2MakeProxy"),
            c2GJKSimplexMetric: l.sym(side, "c2GJKSimplexMetric"),
            c22: l.sym(side, "c22"),
            c23: l.sym(side, "c23"),
            c2D: l.sym(side, "c2D"),
            c2L: l.sym(side, "c2L"),
            c2Support: l.sym(side, "c2Support"),
            c2Witness: l.sym(side, "c2Witness"),
            c2GJK: l.sym(side, "c2GJK"),
            c2AABBtoAABB: l.sym(side, "c2AABBtoAABB"),
            c2AABBtoCapsule: l.sym(side, "c2AABBtoCapsule"),
            c2CapsuletoCapsule: l.sym(side, "c2CapsuletoCapsule"),
            c2CircletoCircle: l.sym(side, "c2CircletoCircle"),
            c2CircletoAABB: l.sym(side, "c2CircletoAABB"),
            c2CircletoCapsule: l.sym(side, "c2CircletoCapsule"),
            c2Collided: l.sym(side, "c2Collided"),
            aabb: l.sym(side, "aabb"),
        }
    }
}

/// `(C api, Rust api)`
pub fn apis() -> &'static (Api, Api) {
    use std::sync::OnceLock;
    static A: OnceLock<(Api, Api)> = OnceLock::new();
    A.get_or_init(|| (Api::build(Side::C), Api::build(Side::R)))
}

unsafe impl Sync for Api {}
unsafe impl Send for Api {}

// ---------------------------------------------------------------------------
// c2GJK invocation record: everything the call can observe or mutate.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct GjkOut {
    pub dist: f32,
    pub outA: Option<c2v>,
    pub outB: Option<c2v>,
    pub iterations: Option<i32>,
    pub cache: Option<c2GJKCache>,
}

impl Bits for GjkOut {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.dist.bits();
        v.extend(self.outA.bits());
        v.extend(self.outB.bits());
        v.extend(self.iterations.bits());
        v.extend(self.cache.bits());
        v
    }
}

/// Which optional out-parameters the caller supplies.
#[derive(Copy, Clone, Debug)]
pub struct OutSel {
    pub a: bool,
    pub b: bool,
    pub iters: bool,
}

impl OutSel {
    pub const ALL: OutSel = OutSel {
        a: true,
        b: true,
        iters: true,
    };
    pub const NONE: OutSel = OutSel {
        a: false,
        b: false,
        iters: false,
    };
}

/// Any of the three shapes, kept as raw bytes so it can be handed to the
/// `const void *` parameter exactly like a C caller would.
#[derive(Copy, Clone, Debug)]
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
    pub fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn call_gjk(
    api: &Api,
    a: &Shape,
    ax: Option<&c2x>,
    b: &Shape,
    bx: Option<&c2x>,
    use_radius: i32,
    sel: OutSel,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    call_gjk_ty(api, a, a.ty(), ax, b, b.ty(), bx, use_radius, sel, cache_in)
}

#[allow(clippy::too_many_arguments)]
pub fn call_gjk_ty(
    api: &Api,
    a: &Shape,
    tya: C2_TYPE,
    ax: Option<&c2x>,
    b: &Shape,
    tyb: C2_TYPE,
    bx: Option<&c2x>,
    use_radius: i32,
    sel: OutSel,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    // Poison the out-params so "not written" is distinguishable from "written".
    let mut oa = c2v {
        x: f32::from_bits(0xDEAD_BEEF),
        y: f32::from_bits(0xDEAD_BEEE),
    };
    let mut ob = c2v {
        x: f32::from_bits(0xDEAD_BEED),
        y: f32::from_bits(0xDEAD_BEEC),
    };
    let mut it: i32 = -12345;
    let mut cache = cache_in;
    let dist = unsafe {
        (api.c2GJK)(
            a.ptr(),
            tya,
            ax.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            b.ptr(),
            tyb,
            bx.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            if sel.a { &mut oa } else { std::ptr::null_mut() },
            if sel.b { &mut ob } else { std::ptr::null_mut() },
            use_radius,
            if sel.iters {
                &mut it
            } else {
                std::ptr::null_mut()
            },
            cache
                .as_mut()
                .map(|c| c as *mut c2GJKCache)
                .unwrap_or(std::ptr::null_mut()),
        )
    };
    GjkOut {
        dist,
        outA: Some(oa),
        outB: Some(ob),
        iterations: Some(it),
        cache,
    }
}
