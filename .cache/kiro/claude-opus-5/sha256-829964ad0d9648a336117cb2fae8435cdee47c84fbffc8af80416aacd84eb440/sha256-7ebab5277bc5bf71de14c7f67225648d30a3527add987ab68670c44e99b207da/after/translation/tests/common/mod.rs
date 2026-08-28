//! Shared harness: loads the C reference `.so` and the Rust `.so` through
//! `libloading` and exposes both under identical, mangled-free signatures.
//!
//! Nothing in here calls into the Rust crate directly -- every Rust function is
//! reached via `dlsym` on the freshly built `cdylib`, exactly like an external
//! C caller would, so the `#[no_mangle]` wrappers are part of what is tested.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C-compatible types (mirrors of the structs in c_src/src/lib.c)
// ---------------------------------------------------------------------------

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
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// Layout-compatible with the C `c2Simplex` (`c2sv a, b, c, d; float div; int count;`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// function pointer types
// ---------------------------------------------------------------------------

pub type FnVff = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVvf = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVvv = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVvvv = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnFvv = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnFv = unsafe extern "C" fn(c2v) -> f32;
pub type FnVv = unsafe extern "C" fn(c2v) -> c2v;
pub type FnR = unsafe extern "C" fn() -> c2r;
pub type FnX = unsafe extern "C" fn() -> c2x;
pub type FnVrv = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnVxv = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnGJK = unsafe extern "C" fn(
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
pub type FnGjkCache = unsafe extern "C" fn(
    c_char,
    *mut c2v,
    *mut c2v,
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
// library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/ -> parent is the working directory holding c_src/ too.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}; build it with cmake first",
            build.display()
        )
    })
}

fn rust_library_path() -> PathBuf {
    // Escape hatch used by the tooling to point the "Rust side" of the harness at
    // an alternative shared object (e.g. a differently optimised C build) in
    // order to establish which behaviours are compiler artifacts.
    if let Ok(p) = std::env::var("HARVEST_RS_SO") {
        return PathBuf::from(p);
    }
    // current_exe is <target>/<profile>/deps/<test-bin>; the cdylib sits in
    // <target>/<profile>/, so we get the artifact for the profile under test.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let direct = profile_dir.join("libgjk_cache_lib.so");
    if direct.exists() {
        return direct;
    }
    // `cargo test` builds the integration-test binaries but not necessarily the
    // `cdylib` artifact, so build it on demand. The build lock is already
    // released by the time test binaries run.
    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();
    let target_dir = profile_dir.parent().expect("target dir").to_path_buf();
    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()));
    cmd.arg("build")
        .arg("--lib")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CARGO_TARGET_DIR", &target_dir);
    match profile.as_str() {
        "debug" => {}
        "release" => {
            cmd.arg("--release");
        }
        other => {
            cmd.arg("--profile").arg(other);
        }
    }
    // Propagate the feature selection the test binary itself was built with, so
    // the cdylib under test matches the configuration being exercised.
    cmd.arg("--no-default-features");
    let feats = enabled_features();
    if !feats.is_empty() {
        cmd.arg("--features").arg(feats.join(","));
    }
    let status = cmd.status();
    if direct.exists() {
        return direct;
    }
    panic!(
        "Rust cdylib not found at {} (on-demand `cargo build` returned {:?}); \
         run `cargo build` first",
        direct.display(),
        status
    );
}

/// Cargo features active in this test binary, mirrored so the on-demand cdylib
/// build uses the same configuration. The crate currently declares none, but
/// this keeps the harness correct if any are added.
fn enabled_features() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut v: Vec<&'static str> = Vec::new();
    v
}

/// One loaded implementation (either the C reference or the Rust translation).
pub struct Impl {
    lib: Library,
    pub name: &'static str,
}

impl Impl {
    pub fn get<T>(&self, sym: &str) -> Symbol<'_, T> {
        unsafe {
            self.lib
                .get(sym.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, sym, e))
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

/// Loads both shared objects. Uses a leaked `Box` so the `Library` handles stay
/// alive for the whole process and returned function pointers remain valid.
pub fn load() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<&'static Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let cp = c_library_path();
        let rp = rust_library_path();
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {}", cp.display(), e));
        let rs = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {}", rp.display(), e));
        Box::leak(Box::new(Pair {
            c: Impl { lib: c, name: "C" },
            rs: Impl {
                lib: rs,
                name: "Rust",
            },
        }))
    })
}

/// Convenience: fetch the same symbol from both libraries.
pub fn both<T: Copy>(sym: &str) -> (T, T) {
    let p = load();
    let a: Symbol<T> = p.c.get(sym);
    let b: Symbol<T> = p.rs.get(sym);
    (*a, *b)
}

/// Paths of the two shared objects under comparison: `(c, rust)`.
pub fn library_paths() -> (PathBuf, PathBuf) {
    // Ensure the Rust cdylib exists (rust_library_path builds it on demand).
    let rs = rust_library_path();
    (c_library_path(), rs)
}

// ---------------------------------------------------------------------------
// bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn f32_same(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

pub fn v_same(a: c2v, b: c2v) -> bool {
    f32_same(a.x, b.x) && f32_same(a.y, b.y)
}

pub fn assert_f32(ctx: &str, c: f32, r: f32) {
    assert!(
        f32_same(c, r),
        "{ctx}: C={c:?} (0x{:08x}) != Rust={r:?} (0x{:08x})",
        c.to_bits(),
        r.to_bits()
    );
}

pub fn assert_v(ctx: &str, c: c2v, r: c2v) {
    assert!(
        v_same(c, r),
        "{ctx}: C=({:?},{:?}) [0x{:08x},0x{:08x}] != Rust=({:?},{:?}) [0x{:08x},0x{:08x}]",
        c.x,
        c.y,
        c.x.to_bits(),
        c.y.to_bits(),
        r.x,
        r.y,
        r.x.to_bits(),
        r.y.to_bits()
    );
}

/// Byte-for-byte comparison of two `#[repr(C)]` values that contain no padding.
pub fn assert_bytes<T>(ctx: &str, c: &T, r: &T) {
    let n = std::mem::size_of::<T>();
    let cb = unsafe { std::slice::from_raw_parts(c as *const T as *const u8, n) };
    let rb = unsafe { std::slice::from_raw_parts(r as *const T as *const u8, n) };
    if cb == rb {
        return;
    }
    // Allow NaN payload differences only when the differing words are both NaN.
    let mut ok = true;
    let mut detail = String::new();
    for i in (0..n).step_by(4) {
        let cw = u32::from_ne_bytes([cb[i], cb[i + 1], cb[i + 2], cb[i + 3]]);
        let rw = u32::from_ne_bytes([rb[i], rb[i + 1], rb[i + 2], rb[i + 3]]);
        if cw != rw {
            let cf = f32::from_bits(cw);
            let rf = f32::from_bits(rw);
            if cf.is_nan() && rf.is_nan() {
                continue;
            }
            ok = false;
            detail.push_str(&format!(
                "\n  @+{i:3}: C=0x{cw:08x} ({cf:?})  Rust=0x{rw:08x} ({rf:?})"
            ));
        }
    }
    assert!(ok, "{ctx}: struct bytes differ:{detail}");
}

// ---------------------------------------------------------------------------
// deterministic pseudo-random inputs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u32(&mut self) -> u32 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Well-behaved float in [-range, range] with ~4 decimal digits.
    pub fn f(&mut self, range: f32) -> f32 {
        let u = (self.next_u32() % 2_000_001) as f32 / 1_000_000.0 - 1.0;
        u * range
    }
    pub fn v(&mut self, range: f32) -> c2v {
        c2v {
            x: self.f(range),
            y: self.f(range),
        }
    }
    /// Occasionally returns an interesting edge-case float instead of a plain one.
    pub fn f_spicy(&mut self, range: f32) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => -1.0,
            4 => f32::MIN_POSITIVE,
            5 => -f32::MIN_POSITIVE,
            6 => f32::EPSILON,
            7 => 1.1920929e-7,
            _ => self.f(range),
        }
    }
    pub fn v_spicy(&mut self, range: f32) -> c2v {
        c2v {
            x: self.f_spicy(range),
            y: self.f_spicy(range),
        }
    }
}

/// A fixed set of nasty float values used to complement the random sweeps.
pub const EDGE_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    1e-30,
    -1e-30,
    1e30,
    -1e30,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::EPSILON,
    1.1920929e-7,
    f32::MAX,
    f32::MIN,
    16777216.0,
    16777217.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];
