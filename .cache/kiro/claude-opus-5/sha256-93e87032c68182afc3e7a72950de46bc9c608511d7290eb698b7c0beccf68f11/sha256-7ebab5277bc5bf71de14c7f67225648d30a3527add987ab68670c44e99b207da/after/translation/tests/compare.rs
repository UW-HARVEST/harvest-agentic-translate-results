//! Differential tests: load BOTH the C shared object and the Rust `cdylib`
//! through `libloading` and compare every exported function's output
//! byte-for-byte.
//!
//! Nothing is ever called directly on the Rust crate -- everything goes through
//! the `.so` exports, so the `#[no_mangle]` wrappers are exercised too.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::field_reassign_with_default)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirror of the C structs
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/ -> parent is the working directory holding c_src/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    static SO: OnceLock<PathBuf> = OnceLock::new();
    SO.get_or_init(|| {
        // Escape hatch used to cross-check against a differently-compiled C
        // library (e.g. a `-O2` build).
        if let Ok(p) = std::env::var("C2_C_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "C2_C_SO points at {} which does not exist", p.display());
            return p;
        }
        let build = workspace_root().join("c_src").join("build");
        if let Some(p) = scan_for_so(&build) {
            return p;
        }
        // Not built yet: configure + build with cmake.
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let ok = Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            && Command::new("cmake")
                .arg("--build")
                .arg(".")
                .current_dir(&build)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        assert!(ok, "cmake build of the C library failed");
        scan_for_so(&build).unwrap_or_else(|| {
            panic!("C shared library not found in {}", build.display());
        })
    })
    .clone()
}

fn scan_for_so(dir: &PathBuf) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if name.starts_with("lib") && name.ends_with(".so") {
            return Some(p);
        }
    }
    None
}

fn find_rust_so() -> PathBuf {
    // `cargo test` does not build the `cdylib` artifact (an integration test
    // cannot link against a cdylib-only crate), so build it explicitly into a
    // dedicated target directory. Done once per test binary; the same feature
    // set the test was compiled with is propagated automatically because the
    // nested invocation reuses this crate's manifest.
    static SO: OnceLock<PathBuf> = OnceLock::new();
    SO.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target = manifest.join("target").join("dylib-under-test");
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build")
            .arg("--release")
            .arg("--lib")
            .arg("--manifest-path")
            .arg(manifest.join("Cargo.toml"));
        for f in FEATURES {
            cmd.arg("--features").arg(f);
        }
        if !DEFAULT_FEATURES {
            cmd.arg("--no-default-features");
        }
        let status = cmd
            .env("CARGO_TARGET_DIR", &target)
            .env_remove("RUSTFLAGS")
            .status()
            .expect("failed to spawn cargo to build the cdylib");
        assert!(status.success(), "building the Rust cdylib failed");
        let so = target.join("release").join("libomni_collide_lib.so");
        assert!(so.exists(), "cdylib missing at {}", so.display());
        so
    })
    .clone()
}

/// The crate currently declares no `[features]`, so there is exactly one
/// build configuration. These constants exist so that adding a feature only
/// requires editing this list.
const FEATURES: &[&str] = &[];
const DEFAULT_FEATURES: bool = true;

struct Pair {
    c: Library,
    r: Library,
}

impl Pair {
    fn load() -> Pair {
        unsafe {
            Pair {
                c: Library::new(find_c_so()).expect("load C .so"),
                r: Library::new(find_rust_so()).expect("load Rust .so"),
            }
        }
    }

    fn get<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        unsafe {
            let c = self.c.get::<T>(name).expect("symbol in C .so");
            let r = self.r.get::<T>(name).expect("symbol in Rust .so");
            (c, r)
        }
    }
}

// ---------------------------------------------------------------------------
// Byte-exact comparison helpers
// ---------------------------------------------------------------------------

fn bytes_of<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

/// Byte-identical comparison. Floats are compared through their bit pattern so
/// `-0.0 != 0.0` and NaN payloads must match too.
fn assert_same<T: std::fmt::Debug>(what: &str, args: &dyn std::fmt::Debug, c: &T, r: &T) {
    if bytes_of(c) != bytes_of(r) {
        panic!(
            "{what} mismatch\n  args: {args:?}\n  C   : {c:?} {:02x?}\n  Rust: {r:?} {:02x?}",
            bytes_of(c),
            bytes_of(r)
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic input generation
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Coarse grid value: lots of exact ties / degenerate configurations.
    fn coarse(&mut self) -> f32 {
        (self.below(33) as f32) * 0.5 - 8.0
    }
    /// Wide-range value including sub-normal-ish and huge magnitudes.
    fn wide(&mut self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => -1.0,
            4 => f32::from_bits(self.next_u32() & 0x7F7F_FFFF), // finite, positive
            5 => -f32::from_bits(self.next_u32() & 0x7F7F_FFFF),
            6 => (self.next_u32() as f32) / 1.0e6,
            7 => -(self.next_u32() as f32) / 1.0e6,
            8 => self.coarse(),
            _ => (self.below(2001) as f32 - 1000.0) * 0.125,
        }
    }
    fn radius(&mut self) -> f32 {
        match self.below(6) {
            0 => 0.0,
            1 => 0.5,
            2 => 1.0,
            3 => 2.0,
            4 => (self.below(64) as f32) * 0.25,
            _ => (self.below(1000) as f32) / 97.0,
        }
    }
    fn v_coarse(&mut self) -> c2v {
        c2v {
            x: self.coarse(),
            y: self.coarse(),
        }
    }
    fn v_wide(&mut self) -> c2v {
        c2v {
            x: self.wide(),
            y: self.wide(),
        }
    }
    fn rot(&mut self) -> c2r {
        match self.below(4) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 1.0 },
            2 => {
                let a = (self.below(360) as f32).to_radians();
                c2r {
                    c: a.cos(),
                    s: a.sin(),
                }
            }
            _ => c2r {
                c: self.coarse(),
                s: self.coarse(),
            },
        }
    }
    fn xform(&mut self) -> c2x {
        c2x {
            p: self.v_coarse(),
            r: self.rot(),
        }
    }
    fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.v_coarse(),
            r: self.radius(),
        }
    }
    fn aabb(&mut self) -> c2AABB {
        // Deliberately not normalised: inverted boxes are legal C inputs.
        c2AABB {
            min: self.v_coarse(),
            max: self.v_coarse(),
        }
    }
    fn capsule(&mut self) -> c2Capsule {
        c2Capsule {
            a: self.v_coarse(),
            b: self.v_coarse(),
            r: self.radius(),
        }
    }
    fn ty(&mut self) -> c_int {
        self.below(3) as c_int
    }
}

// ===========================================================================
// Level 1: scalar / vector primitives
// ===========================================================================

type FnFF_V = unsafe extern "C" fn(f32, f32) -> c2v;
type FnV_V = unsafe extern "C" fn(c2v) -> c2v;
type FnVF_V = unsafe extern "C" fn(c2v, f32) -> c2v;
type FnVV_V = unsafe extern "C" fn(c2v, c2v) -> c2v;
type FnVVV_V = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
type FnVV_F = unsafe extern "C" fn(c2v, c2v) -> f32;
type FnV_F = unsafe extern "C" fn(c2v) -> f32;
type FnRV_V = unsafe extern "C" fn(c2r, c2v) -> c2v;

#[test]
fn t01_c2V() {
    let p = Pair::load();
    let (c, r) = p.get::<FnFF_V>(b"c2V\0");
    let mut rng = Rng::new(1);
    for _ in 0..80_000 {
        let (x, y) = (rng.wide(), rng.wide());
        unsafe { assert_same("c2V", &(x, y), &c(x, y), &r(x, y)) };
    }
}

#[test]
fn t02_unary_v() {
    let p = Pair::load();
    let mut rng = Rng::new(2);
    for name in [&b"c2Neg\0"[..], b"c2Skew\0", b"c2CCW90\0", b"c2Norm\0"] {
        let (c, r) = p.get::<FnV_V>(name);
        for _ in 0..80_000 {
            let a = rng.v_wide();
            unsafe { assert_same("unary", &(name, a), &c(a), &r(a)) };
        }
    }
}

#[test]
fn t03_v_f_to_v() {
    let p = Pair::load();
    let mut rng = Rng::new(3);
    for name in [&b"c2Mulvs\0"[..], b"c2Div\0"] {
        let (c, r) = p.get::<FnVF_V>(name);
        for _ in 0..80_000 {
            let a = rng.v_wide();
            let b = rng.wide();
            unsafe { assert_same("v,f->v", &(name, a, b), &c(a, b), &r(a, b)) };
        }
    }
}

#[test]
fn t04_vv_to_v() {
    let p = Pair::load();
    let mut rng = Rng::new(4);
    for name in [&b"c2Maxv\0"[..], b"c2Minv\0", b"c2Sub\0", b"c2Add\0"] {
        let (c, r) = p.get::<FnVV_V>(name);
        for _ in 0..80_000 {
            let a = rng.v_wide();
            let b = if rng.below(4) == 0 { a } else { rng.v_wide() };
            unsafe { assert_same("v,v->v", &(name, a, b), &c(a, b), &r(a, b)) };
        }
    }
}

#[test]
fn t05_c2Clampv() {
    let p = Pair::load();
    let (c, r) = p.get::<FnVVV_V>(b"c2Clampv\0");
    let mut rng = Rng::new(5);
    for _ in 0..120_000 {
        let a = rng.v_wide();
        let lo = rng.v_wide();
        let hi = if rng.below(3) == 0 { lo } else { rng.v_wide() };
        unsafe { assert_same("c2Clampv", &(a, lo, hi), &c(a, lo, hi), &r(a, lo, hi)) };
    }
}

#[test]
fn t06_vv_to_f() {
    let p = Pair::load();
    let mut rng = Rng::new(6);
    for name in [&b"c2Dot\0"[..], b"c2Det2\0"] {
        let (c, r) = p.get::<FnVV_F>(name);
        for _ in 0..80_000 {
            let a = rng.v_wide();
            let b = if rng.below(4) == 0 { a } else { rng.v_wide() };
            unsafe { assert_same("v,v->f", &(name, a, b), &c(a, b), &r(a, b)) };
        }
    }
}

#[test]
fn t07_c2Len() {
    let p = Pair::load();
    let (c, r) = p.get::<FnV_F>(b"c2Len\0");
    let mut rng = Rng::new(7);
    for _ in 0..120_000 {
        let a = rng.v_wide();
        unsafe { assert_same("c2Len", &a, &c(a), &r(a)) };
    }
}

#[test]
fn t08_rot_identity() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn() -> c2r>(b"c2RotIdentity\0");
    unsafe { assert_same("c2RotIdentity", &(), &c(), &r()) };
    let (c, r) = p.get::<unsafe extern "C" fn() -> c2x>(b"c2xIdentity\0");
    unsafe { assert_same("c2xIdentity", &(), &c(), &r()) };
}

#[test]
fn t09_rv_to_v() {
    let p = Pair::load();
    let mut rng = Rng::new(9);
    for name in [&b"c2Mulrv\0"[..], b"c2MulrvT\0"] {
        let (c, r) = p.get::<FnRV_V>(name);
        for _ in 0..80_000 {
            let a = rng.rot();
            let b = rng.v_wide();
            unsafe { assert_same("r,v->v", &(name, a, b), &c(a, b), &r(a, b)) };
        }
    }
}

#[test]
fn t10_c2Mulxv() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(c2x, c2v) -> c2v>(b"c2Mulxv\0");
    let mut rng = Rng::new(10);
    for _ in 0..120_000 {
        let a = rng.xform();
        let b = rng.v_wide();
        unsafe { assert_same("c2Mulxv", &(a, b), &c(a, b), &r(a, b)) };
    }
}

// ===========================================================================
// Level 2: proxy construction
// ===========================================================================

#[test]
fn t11_c2BBVerts() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(*mut c2v, *mut c2AABB)>(b"c2BBVerts\0");
    let mut rng = Rng::new(11);
    for _ in 0..80_000 {
        let mut bb = c2AABB {
            min: rng.v_wide(),
            max: rng.v_wide(),
        };
        let mut oc = [c2v::default(); 4];
        let mut or_ = [c2v::default(); 4];
        unsafe {
            c(oc.as_mut_ptr(), &mut bb);
            r(or_.as_mut_ptr(), &mut bb);
        }
        assert_same("c2BBVerts", &bb, &oc, &or_);
    }
}

#[test]
fn t12_c2MakeProxy() {
    let p = Pair::load();
    let (c, r) =
        p.get::<unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy)>(b"c2MakeProxy\0");
    let mut rng = Rng::new(12);
    for _ in 0..60_000 {
        // Fill both proxies with the same non-zero garbage so that untouched
        // fields are compared as well.
        let seed = rng.next_u32();
        let mk = || {
            let mut pr = c2Proxy::default();
            pr.radius = f32::from_bits(seed);
            pr.count = seed as c_int;
            for (i, v) in pr.verts.iter_mut().enumerate() {
                v.x = (seed as f32) + i as f32;
                v.y = (seed as f32) - i as f32;
            }
            pr
        };
        let (mut pc, mut pr) = (mk(), mk());
        match rng.ty() {
            C2_TYPE_CIRCLE => {
                let s = rng.circle();
                unsafe {
                    c(&s as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pc);
                    r(&s as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pr);
                }
                assert_same("c2MakeProxy(circle)", &s, &pc, &pr);
            }
            C2_TYPE_AABB => {
                let s = rng.aabb();
                unsafe {
                    c(&s as *const _ as *const c_void, C2_TYPE_AABB, &mut pc);
                    r(&s as *const _ as *const c_void, C2_TYPE_AABB, &mut pr);
                }
                assert_same("c2MakeProxy(aabb)", &s, &pc, &pr);
            }
            _ => {
                let s = rng.capsule();
                unsafe {
                    c(&s as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pc);
                    r(&s as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pr);
                }
                assert_same("c2MakeProxy(capsule)", &s, &pc, &pr);
            }
        }
    }
    // Out-of-range tag: the C switch has no default, so the proxy is untouched.
    let s = c2Circle::default();
    for bad in [3 as c_int, 7, -1, 12345] {
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        pc.radius = 12.5;
        pr.radius = 12.5;
        pc.count = 99;
        pr.count = 99;
        unsafe {
            c(&s as *const _ as *const c_void, bad, &mut pc);
            r(&s as *const _ as *const c_void, bad, &mut pr);
        }
        assert_same("c2MakeProxy(bad tag)", &bad, &pc, &pr);
    }
}

// ===========================================================================
// Level 3: simplex helpers
// ===========================================================================

impl Rng {
    fn sv(&mut self) -> c2sv {
        c2sv {
            sA: self.v_coarse(),
            sB: self.v_coarse(),
            p: self.v_coarse(),
            u: self.wide(),
            iA: self.below(8) as c_int,
            iB: self.below(8) as c_int,
        }
    }
    /// Fully populated simplex -- every byte is defined so both libraries read
    /// exactly the same input.
    fn simplex(&mut self, count: c_int) -> c2Simplex {
        let mut s = c2Simplex {
            verts: [self.sv(), self.sv(), self.sv(), self.sv()],
            div: match self.below(5) {
                0 => 1.0,
                1 => 0.0,
                2 => -1.0,
                3 => self.coarse(),
                _ => self.wide(),
            },
            count,
        };
        // Occasionally force duplicate / collinear points to hit the
        // degenerate branches of c22 / c23.
        match self.below(6) {
            0 => s.verts[1].p = s.verts[0].p,
            1 => s.verts[2].p = s.verts[0].p,
            2 => s.verts[2].p = s.verts[1].p,
            3 => {
                s.verts[1].p = c2v {
                    x: s.verts[0].p.x * 2.0,
                    y: s.verts[0].p.y * 2.0,
                };
                s.verts[2].p = c2v {
                    x: s.verts[0].p.x * 3.0,
                    y: s.verts[0].p.y * 3.0,
                };
            }
            _ => {}
        }
        s
    }
}

const COUNTS: [c_int; 7] = [0, 1, 2, 3, 4, -1, 9];

#[test]
fn t13_c2GJKSimplexMetric() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(*mut c2Simplex) -> f32>(b"c2GJKSimplexMetric\0");
    let mut rng = Rng::new(13);
    for i in 0..160_000 {
        let count = COUNTS[i % COUNTS.len()];
        let s = rng.simplex(count);
        let (mut sc, mut sr) = (s, s);
        let (rc, rr) = unsafe { (c(&mut sc), r(&mut sr)) };
        assert_same("c2GJKSimplexMetric ret", &s, &rc, &rr);
        assert_same("c2GJKSimplexMetric state", &s, &sc, &sr);
    }
}

#[test]
fn t14_c22() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(*mut c2Simplex)>(b"c22\0");
    let mut rng = Rng::new(14);
    for i in 0..160_000 {
        let s = rng.simplex(COUNTS[i % COUNTS.len()]);
        let (mut sc, mut sr) = (s, s);
        unsafe {
            c(&mut sc);
            r(&mut sr);
        }
        assert_same("c22", &s, &sc, &sr);
    }
}

#[test]
fn t15_c23() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(*mut c2Simplex)>(b"c23\0");
    let mut rng = Rng::new(15);
    for i in 0..240_000 {
        let s = rng.simplex(COUNTS[i % COUNTS.len()]);
        let (mut sc, mut sr) = (s, s);
        unsafe {
            c(&mut sc);
            r(&mut sr);
        }
        assert_same("c23", &s, &sc, &sr);
    }
}

#[test]
fn t16_c2D() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>(b"c2D\0");
    let mut rng = Rng::new(16);
    for i in 0..160_000 {
        let s = rng.simplex(COUNTS[i % COUNTS.len()]);
        let (mut sc, mut sr) = (s, s);
        let (rc, rr) = unsafe { (c(&mut sc), r(&mut sr)) };
        assert_same("c2D ret", &s, &rc, &rr);
        assert_same("c2D state", &s, &sc, &sr);
    }
}

#[test]
fn t17_c2L() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>(b"c2L\0");
    let mut rng = Rng::new(17);
    for i in 0..160_000 {
        let s = rng.simplex(COUNTS[i % COUNTS.len()]);
        let (mut sc, mut sr) = (s, s);
        let (rc, rr) = unsafe { (c(&mut sc), r(&mut sr)) };
        assert_same("c2L ret", &s, &rc, &rr);
        assert_same("c2L state", &s, &sc, &sr);
    }
}

#[test]
fn t18_c2Witness() {
    let p = Pair::load();
    let (c, r) =
        p.get::<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)>(b"c2Witness\0");
    let mut rng = Rng::new(18);
    for i in 0..160_000 {
        let s = rng.simplex(COUNTS[i % COUNTS.len()]);
        let (mut sc, mut sr) = (s, s);
        let mut ac = c2v { x: 7.5, y: -3.25 };
        let mut bc = c2v { x: 1.5, y: 9.75 };
        let mut ar = ac;
        let mut br = bc;
        unsafe {
            c(&mut sc, &mut ac, &mut bc);
            r(&mut sr, &mut ar, &mut br);
        }
        assert_same("c2Witness a", &s, &ac, &ar);
        assert_same("c2Witness b", &s, &bc, &br);
        assert_same("c2Witness state", &s, &sc, &sr);
    }
}

#[test]
fn t19_c2Support() {
    let p = Pair::load();
    let (c, r) =
        p.get::<unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int>(b"c2Support\0");
    let mut rng = Rng::new(19);
    for _ in 0..160_000 {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.v_wide();
        }
        // Duplicates matter: `>` (not `>=`) decides ties.
        if rng.below(3) == 0 {
            let a = rng.below(8) as usize;
            let b = rng.below(8) as usize;
            verts[b] = verts[a];
        }
        let count = rng.below(9) as c_int;
        let d = rng.v_wide();
        let (rc, rr) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
        assert_same("c2Support", &(verts, count, d), &rc, &rr);
    }
}

// ===========================================================================
// Level 4: c2GJK
// ===========================================================================

type FnGJK = unsafe extern "C" fn(
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

/// One of the three concrete shapes, kept in a fixed-size buffer so a raw
/// pointer to it can be handed to both libraries.
#[derive(Clone, Copy, Debug)]
enum AnyShape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl AnyShape {
    fn ty(&self) -> c_int {
        match self {
            AnyShape::Circle(_) => C2_TYPE_CIRCLE,
            AnyShape::Aabb(_) => C2_TYPE_AABB,
            AnyShape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    fn ptr(&self) -> *const c_void {
        match self {
            AnyShape::Circle(s) => s as *const _ as *const c_void,
            AnyShape::Aabb(s) => s as *const _ as *const c_void,
            AnyShape::Capsule(s) => s as *const _ as *const c_void,
        }
    }
    /// Number of proxy vertices, i.e. the valid range for cache indices.
    fn nverts(&self) -> u32 {
        match self {
            AnyShape::Circle(_) => 1,
            AnyShape::Aabb(_) => 4,
            AnyShape::Capsule(_) => 2,
        }
    }
}

impl Rng {
    fn any_shape(&mut self) -> AnyShape {
        match self.ty() {
            C2_TYPE_CIRCLE => AnyShape::Circle(self.circle()),
            C2_TYPE_AABB => AnyShape::Aabb(self.aabb()),
            _ => AnyShape::Capsule(self.capsule()),
        }
    }
}

/// Runs c2GJK on both libraries with identical inputs and compares the return
/// value plus every out-parameter byte-for-byte.
#[allow(clippy::too_many_arguments)]
fn gjk_case(
    c: &FnGJK,
    r: &FnGJK,
    a: &AnyShape,
    ax: Option<c2x>,
    b: &AnyShape,
    bx: Option<c2x>,
    use_radius: c_int,
    cache: Option<c2GJKCache>,
    label: &str,
) {
    let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

    let mut oac = c2v { x: 1.25, y: -6.5 };
    let mut obc = c2v { x: -2.5, y: 3.75 };
    let mut oar = oac;
    let mut obr = obc;
    let mut itc: c_int = -999;
    let mut itr: c_int = -999;
    let mut cc = cache;
    let mut cr = cache;

    let (dc, dr) = unsafe {
        (
            c(
                a.ptr(),
                a.ty(),
                axp,
                b.ptr(),
                b.ty(),
                bxp,
                &mut oac,
                &mut obc,
                use_radius,
                &mut itc,
                cc.as_mut().map_or(std::ptr::null_mut(), |x| x as *mut _),
            ),
            r(
                a.ptr(),
                a.ty(),
                axp,
                b.ptr(),
                b.ty(),
                bxp,
                &mut oar,
                &mut obr,
                use_radius,
                &mut itr,
                cr.as_mut().map_or(std::ptr::null_mut(), |x| x as *mut _),
            ),
        )
    };
    let args = format!(
        "{label} A={a:?} ax={ax:?} B={b:?} bx={bx:?} use_radius={use_radius} cache={cache:?}"
    );
    assert_same("c2GJK dist", &args, &dc, &dr);
    assert_same("c2GJK outA", &args, &oac, &oar);
    assert_same("c2GJK outB", &args, &obc, &obr);
    assert_same("c2GJK iterations", &args, &itc, &itr);
    assert_same("c2GJK cache", &args, &cc, &cr);
}

#[test]
fn t20_c2GJK_no_cache() {
    let p = Pair::load();
    let (c, r) = p.get::<FnGJK>(b"c2GJK\0");
    let mut rng = Rng::new(20);
    for i in 0..400_000 {
        let a = rng.any_shape();
        let b = rng.any_shape();
        let ax = if i % 3 == 0 { None } else { Some(rng.xform()) };
        let bx = if i % 5 == 0 { None } else { Some(rng.xform()) };
        let ur = (i % 2) as c_int;
        gjk_case(&c, &r, &a, ax, &b, bx, ur, None, "no-cache");
    }
}

#[test]
fn t21_c2GJK_null_outputs() {
    // Exercise every combination of null out-parameters.
    let p = Pair::load();
    let (c, r) = p.get::<FnGJK>(b"c2GJK\0");
    let mut rng = Rng::new(21);
    for i in 0..80_000u32 {
        let a = rng.any_shape();
        let b = rng.any_shape();
        let mut oac = c2v { x: 5.0, y: 5.0 };
        let mut obc = oac;
        let mut oar = oac;
        let mut obr = oac;
        let mut itc: c_int = 42;
        let mut itr: c_int = 42;
        let use_a = i & 1 != 0;
        let use_b = i & 2 != 0;
        let use_it = i & 4 != 0;
        let ur = ((i >> 3) & 1) as c_int;
        let pa_c = if use_a {
            &mut oac as *mut c2v
        } else {
            std::ptr::null_mut()
        };
        let pa_r = if use_a {
            &mut oar as *mut c2v
        } else {
            std::ptr::null_mut()
        };
        let pb_c = if use_b {
            &mut obc as *mut c2v
        } else {
            std::ptr::null_mut()
        };
        let pb_r = if use_b {
            &mut obr as *mut c2v
        } else {
            std::ptr::null_mut()
        };
        let pi_c = if use_it {
            &mut itc as *mut c_int
        } else {
            std::ptr::null_mut()
        };
        let pi_r = if use_it {
            &mut itr as *mut c_int
        } else {
            std::ptr::null_mut()
        };
        let (dc, dr) = unsafe {
            (
                c(
                    a.ptr(),
                    a.ty(),
                    std::ptr::null(),
                    b.ptr(),
                    b.ty(),
                    std::ptr::null(),
                    pa_c,
                    pb_c,
                    ur,
                    pi_c,
                    std::ptr::null_mut(),
                ),
                r(
                    a.ptr(),
                    a.ty(),
                    std::ptr::null(),
                    b.ptr(),
                    b.ty(),
                    std::ptr::null(),
                    pa_r,
                    pb_r,
                    ur,
                    pi_r,
                    std::ptr::null_mut(),
                ),
            )
        };
        let args = format!("{a:?} {b:?} mask={i:#x}");
        assert_same("c2GJK(null) dist", &args, &dc, &dr);
        assert_same("c2GJK(null) outA", &args, &oac, &oar);
        assert_same("c2GJK(null) outB", &args, &obc, &obr);
        assert_same("c2GJK(null) iter", &args, &itc, &itr);
    }
}

#[test]
fn t22_c2GJK_with_cache() {
    let p = Pair::load();
    let (c, r) = p.get::<FnGJK>(b"c2GJK\0");
    let mut rng = Rng::new(22);

    // Synthetic caches, including count == 0 (cache treated as unusable) and
    // counts 1..3 with in-range vertex indices.
    for _ in 0..240_000 {
        let a = rng.any_shape();
        let b = rng.any_shape();
        let count = rng.below(4) as c_int;
        let mut cache = c2GJKCache {
            metric: match rng.below(5) {
                0 => 0.0,
                1 => -1.0e9,
                2 => 1.0e9,
                3 => rng.coarse(),
                _ => rng.wide(),
            },
            count,
            iA: [0; 3],
            iB: [0; 3],
            div: match rng.below(3) {
                0 => 1.0,
                1 => 0.0,
                _ => rng.coarse(),
            },
        };
        for k in 0..3 {
            cache.iA[k] = rng.below(a.nverts()) as c_int;
            cache.iB[k] = rng.below(b.nverts()) as c_int;
        }
        let ax = if rng.below(2) == 0 {
            None
        } else {
            Some(rng.xform())
        };
        let bx = if rng.below(2) == 0 {
            None
        } else {
            Some(rng.xform())
        };
        let ur = rng.below(2) as c_int;
        gjk_case(&c, &r, &a, ax, &b, bx, ur, Some(cache), "synthetic-cache");
    }

    // Warm-start chains: feed the cache produced by one call into the next,
    // which is how the cache is meant to be used.
    for _ in 0..60_000 {
        let a = rng.any_shape();
        let b = rng.any_shape();
        let mut cache = c2GJKCache::default();
        for step in 0..4 {
            let ax = Some(rng.xform());
            let bx = Some(rng.xform());
            let ur = (step % 2) as c_int;
            let axp = ax.as_ref().unwrap() as *const c2x;
            let bxp = bx.as_ref().unwrap() as *const c2x;
            let mut cc = cache;
            let mut cr = cache;
            let mut oac = c2v::default();
            let mut obc = c2v::default();
            let mut oar = c2v::default();
            let mut obr = c2v::default();
            let mut itc: c_int = 0;
            let mut itr: c_int = 0;
            let (dc, dr) = unsafe {
                (
                    c(
                        a.ptr(),
                        a.ty(),
                        axp,
                        b.ptr(),
                        b.ty(),
                        bxp,
                        &mut oac,
                        &mut obc,
                        ur,
                        &mut itc,
                        &mut cc,
                    ),
                    r(
                        a.ptr(),
                        a.ty(),
                        axp,
                        b.ptr(),
                        b.ty(),
                        bxp,
                        &mut oar,
                        &mut obr,
                        ur,
                        &mut itr,
                        &mut cr,
                    ),
                )
            };
            let args = format!("warm step={step} A={a:?} B={b:?} cache_in={cache:?}");
            assert_same("c2GJK warm dist", &args, &dc, &dr);
            assert_same("c2GJK warm outA", &args, &oac, &oar);
            assert_same("c2GJK warm outB", &args, &obc, &obr);
            assert_same("c2GJK warm iter", &args, &itc, &itr);
            assert_same("c2GJK warm cache", &args, &cc, &cr);
            cache = cc;
        }
    }
}

// ===========================================================================
// Level 5: shape-vs-shape predicates
// ===========================================================================

#[test]
fn t23_c2AABBtoAABB() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(c2AABB, c2AABB) -> c_int>(b"c2AABBtoAABB\0");
    let mut rng = Rng::new(23);
    for _ in 0..400_000 {
        let a = rng.aabb();
        let b = if rng.below(5) == 0 { a } else { rng.aabb() };
        unsafe { assert_same("c2AABBtoAABB", &(a, b), &c(a, b), &r(a, b)) };
    }
}

#[test]
fn t24_c2CircletoCircle() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(c2Circle, c2Circle) -> c_int>(b"c2CircletoCircle\0");
    let mut rng = Rng::new(24);
    for _ in 0..400_000 {
        let a = rng.circle();
        let b = if rng.below(5) == 0 { a } else { rng.circle() };
        unsafe { assert_same("c2CircletoCircle", &(a, b), &c(a, b), &r(a, b)) };
    }
}

#[test]
fn t25_c2CircletoAABB() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(c2Circle, c2AABB) -> c_int>(b"c2CircletoAABB\0");
    let mut rng = Rng::new(25);
    for _ in 0..400_000 {
        let a = rng.circle();
        let b = rng.aabb();
        unsafe { assert_same("c2CircletoAABB", &(a, b), &c(a, b), &r(a, b)) };
    }
}

#[test]
fn t26_c2CircletoCapsule() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int>(b"c2CircletoCapsule\0");
    let mut rng = Rng::new(26);
    for _ in 0..400_000 {
        let a = rng.circle();
        let mut b = rng.capsule();
        // Degenerate capsule (a == b) makes c2Dot(n, n) zero -> division by 0.
        if rng.below(6) == 0 {
            b.b = b.a;
        }
        unsafe { assert_same("c2CircletoCapsule", &(a, b), &c(a, b), &r(a, b)) };
    }
}

#[test]
fn t27_c2AABBtoCapsule() {
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(c2AABB, c2Capsule) -> c_int>(b"c2AABBtoCapsule\0");
    let mut rng = Rng::new(27);
    for _ in 0..400_000 {
        let a = rng.aabb();
        let mut b = rng.capsule();
        if rng.below(6) == 0 {
            b.b = b.a;
        }
        unsafe { assert_same("c2AABBtoCapsule", &(a, b), &c(a, b), &r(a, b)) };
    }
}

#[test]
fn t28_c2CapsuletoCapsule() {
    let p = Pair::load();
    let (c, r) =
        p.get::<unsafe extern "C" fn(c2Capsule, c2Capsule) -> c_int>(b"c2CapsuletoCapsule\0");
    let mut rng = Rng::new(28);
    for _ in 0..400_000 {
        let mut a = rng.capsule();
        let mut b = if rng.below(5) == 0 { a } else { rng.capsule() };
        if rng.below(6) == 0 {
            a.b = a.a;
        }
        if rng.below(6) == 0 {
            b.b = b.a;
        }
        unsafe { assert_same("c2CapsuletoCapsule", &(a, b), &c(a, b), &r(a, b)) };
    }
}

// ===========================================================================
// Level 6: dispatch + public API
// ===========================================================================

#[test]
fn t29_c2Collided() {
    let p = Pair::load();
    let (c, r) = p
        .get::<unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int>(
            b"c2Collided\0",
        );
    let mut rng = Rng::new(29);
    for _ in 0..600_000 {
        let a = rng.any_shape();
        let b = rng.any_shape();
        let (rc, rr) = unsafe {
            (
                c(a.ptr(), a.ty(), b.ptr(), b.ty()),
                r(a.ptr(), a.ty(), b.ptr(), b.ty()),
            )
        };
        assert_same("c2Collided", &(a, b), &rc, &rr);
    }
    // Out-of-range tags take the `default: return 0;` paths without ever
    // dereferencing the corresponding pointer.
    let s = AnyShape::Circle(c2Circle::default());
    for bad in [3 as c_int, -1, 77] {
        for good in [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB] {
            let (rc, rr) = unsafe {
                (
                    c(s.ptr(), bad, s.ptr(), good),
                    r(s.ptr(), bad, s.ptr(), good),
                )
            };
            assert_same("c2Collided(bad A)", &(bad, good), &rc, &rr);
            let (rc, rr) = unsafe {
                (
                    c(s.ptr(), good, std::ptr::null(), bad),
                    r(s.ptr(), good, std::ptr::null(), bad),
                )
            };
            assert_same("c2Collided(bad B)", &(good, bad), &rc, &rr);
        }
    }
}

#[test]
fn t30_ptr_from_parts() {
    // The returned pointer values differ (different heaps), but the memory it
    // points at must be identical.
    let p = Pair::load();
    let (c, r) = p.get::<unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void>(
        b"ptr_from_parts\0",
    );
    let (fc, _fr) = p.get::<unsafe extern "C" fn(*mut c_void)>(b"free\0");
    let mut rng = Rng::new(30);
    for _ in 0..400_000 {
        let ty = rng.ty();
        let (a, b, cc, d, e) = (
            rng.wide(),
            rng.wide(),
            rng.wide(),
            rng.wide(),
            rng.wide(),
        );
        unsafe {
            let pc = c(ty, a, b, cc, d, e);
            let pr = r(ty, a, b, cc, d, e);
            assert!(!pc.is_null() && !pr.is_null(), "allocation failed");
            let n = match ty {
                C2_TYPE_CIRCLE => std::mem::size_of::<c2Circle>(),
                C2_TYPE_AABB => std::mem::size_of::<c2AABB>(),
                _ => std::mem::size_of::<c2Capsule>(),
            };
            let bc = std::slice::from_raw_parts(pc as *const u8, n);
            let br = std::slice::from_raw_parts(pr as *const u8, n);
            assert_eq!(
                bc, br,
                "ptr_from_parts payload mismatch for ty={ty} args={:?}",
                (a, b, cc, d, e)
            );
            fc(pc);
            fc(pr);
        }
    }
}

#[test]
fn t31_omni_collide() {
    let p = Pair::load();
    type F = unsafe extern "C" fn(
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
    ) -> c_int;
    let (c, r) = p.get::<F>(b"omni_collide\0");
    let mut rng = Rng::new(31);
    for _ in 0..800_000 {
        let ta = rng.ty();
        let tb = rng.ty();
        let a: [f32; 5] = [
            rng.coarse(),
            rng.coarse(),
            rng.coarse(),
            rng.coarse(),
            rng.radius(),
        ];
        let b: [f32; 5] = [
            rng.coarse(),
            rng.coarse(),
            rng.coarse(),
            rng.coarse(),
            rng.radius(),
        ];
        let (rc, rr) = unsafe {
            (
                c(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]),
                r(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]),
            )
        };
        assert_same("omni_collide", &(ta, a, tb, b), &rc, &rr);
    }
    // Wide-range float inputs.
    for _ in 0..800_000 {
        let ta = rng.ty();
        let tb = rng.ty();
        let a: [f32; 5] = [
            rng.wide(),
            rng.wide(),
            rng.wide(),
            rng.wide(),
            rng.wide(),
        ];
        let b: [f32; 5] = [
            rng.wide(),
            rng.wide(),
            rng.wide(),
            rng.wide(),
            rng.wide(),
        ];
        let (rc, rr) = unsafe {
            (
                c(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]),
                r(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]),
            )
        };
        assert_same("omni_collide wide", &(ta, a, tb, b), &rc, &rr);
    }
}

#[test]
fn t32_omni_collide_exhaustive_grid() {
    // Small exhaustive sweep over an integer grid: guarantees the exact
    // touching / tangent boundary cases are hit for every type pair.
    let p = Pair::load();
    type F = unsafe extern "C" fn(
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
    ) -> c_int;
    let (c, r) = p.get::<F>(b"omni_collide\0");
    let grid = [-2.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0];
    let radii = [0.0f32, 0.5, 1.0, 2.0];
    for ta in 0..3 {
        for tb in 0..3 {
            for &x in &grid {
                for &y in &grid {
                    for &ra in &radii {
                        for &rb in &radii {
                            let a = [0.0, 0.0, 1.0, 1.0, ra];
                            let b = [x, y, x + 1.0, y + 1.0, rb];
                            let (rc, rr) = unsafe {
                                (
                                    c(
                                        ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2],
                                        b[3], b[4],
                                    ),
                                    r(
                                        ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2],
                                        b[3], b[4],
                                    ),
                                )
                            };
                            assert_same("omni_collide grid", &(ta, a, tb, b), &rc, &rr);
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Exported-symbol parity
// ===========================================================================

#[test]
fn t33_exported_symbols_match() {
    let c_so = find_c_so();
    let r_so = find_rust_so();

    let syms = |path: &PathBuf| -> Vec<String> {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    };

    let cs = syms(&c_so);
    let rs = syms(&r_so);
    assert!(!cs.is_empty(), "no symbols read from the C .so");

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // Sanity: every symbol must also be dlsym-able from both objects.
    let p = Pair::load();
    for s in &cs {
        let mut name = s.clone().into_bytes();
        name.push(0);
        let _ = p.get::<unsafe extern "C" fn()>(&name);
    }
}
