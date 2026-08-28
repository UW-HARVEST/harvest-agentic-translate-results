//! Shared plumbing: locate + dlopen the C and Rust shared libraries and expose
//! their exported symbols behind identical Rust signatures.
//!
//! Neither side is ever called directly as a Rust function: both are reached
//! through `libloading`, so the `#[no_mangle]` export wrappers of the Rust
//! crate are exercised exactly like an external C caller would exercise them.

#![allow(non_camel_case_types, non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C-compatible layouts (mirrors of the anonymous types in c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Signature aliases
// ---------------------------------------------------------------------------

pub type FnC2V = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnMulvs = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnClampv = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnDot = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnCircleCircle = unsafe extern "C" fn(c2Circle, c2Circle) -> c_int;
pub type FnCircleAabb = unsafe extern "C" fn(c2Circle, c2AABB) -> c_int;
pub type FnCircleCapsule = unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int;
pub type FnCollided = unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int;
pub type FnCircleCollide = unsafe extern "C" fn(f32, f32, f32) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `cdylib`).
pub struct Impl {
    pub name: &'static str,
    #[allow(unused)]
    lib: Library,
    pub c2V: FnC2V,
    pub c2Mulvs: FnMulvs,
    pub c2Maxv: FnVV,
    pub c2Minv: FnVV,
    pub c2Clampv: FnClampv,
    pub c2Sub: FnVV,
    pub c2Dot: FnDot,
    pub c2CircletoCircle: FnCircleCircle,
    pub c2CircletoAABB: FnCircleAabb,
    pub c2CircletoCapsule: FnCircleCapsule,
    pub c2Collided: FnCollided,
    pub circle_collide: FnCircleCollide,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing exported symbol `{name}`: {e}"));
        *s
    }
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            Impl {
                name,
                c2V: sym(&lib, "c2V"),
                c2Mulvs: sym(&lib, "c2Mulvs"),
                c2Maxv: sym(&lib, "c2Maxv"),
                c2Minv: sym(&lib, "c2Minv"),
                c2Clampv: sym(&lib, "c2Clampv"),
                c2Sub: sym(&lib, "c2Sub"),
                c2Dot: sym(&lib, "c2Dot"),
                c2CircletoCircle: sym(&lib, "c2CircletoCircle"),
                c2CircletoAABB: sym(&lib, "c2CircletoAABB"),
                c2CircletoCapsule: sym(&lib, "c2CircletoCapsule"),
                c2Collided: sym(&lib, "c2Collided"),
                circle_collide: sym(&lib, "circle_collide"),
                lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/lib<something>.so` — the CMake project name is derived
/// from the parent directory name, so the file name is not fixed.
fn c_library_path() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    match found.len() {
        0 => panic!("no lib*.so found in {}", build.display()),
        _ => found.remove(0),
    }
}

/// The Rust `cdylib` for the profile the test binary itself was built with.
///
/// `cargo test` builds the test harnesses but *not* the `cdylib` artifact, so a
/// stale `.so` left over from an earlier `cargo build` would silently be tested
/// instead of the current sources. Reject that explicitly rather than reporting
/// bogus results.
fn rust_library_path() -> PathBuf {
    // .../target/<profile>/deps/<test-bin> -> .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let path = profile_dir.join("libcircle_collide_lib.so");
    assert!(
        path.exists(),
        "{} not found — `cargo test` does not build the cdylib; \
         run `cargo build` for the same profile first (see ./verify.sh)",
        path.display()
    );

    let so_mtime = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    for source in ["src/lib.rs", "Cargo.toml"] {
        let src = manifest_dir().join(source);
        let src_mtime = std::fs::metadata(&src)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("cannot stat {}: {e}", src.display()));
        assert!(
            so_mtime >= src_mtime,
            "{} is older than {} — the cdylib is stale. \
             `cargo test` does not rebuild it; run `cargo build` first (see ./verify.sh)",
            path.display(),
            src.display()
        );
    }
    path
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

/// Loads both libraries once per test binary.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &c_library_path()),
        rs: Impl::load("Rust", &rust_library_path()),
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Byte-for-byte float comparison (NaN payload and signed zero sensitive).
pub fn eqf(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn eqv(a: c2v, b: c2v) -> bool {
    eqf(a.x, b.x) && eqf(a.y, b.y)
}

/// Byte-for-byte float comparison (NaN payload and signed zero sensitive).
#[track_caller]
pub fn assert_f32_bits(what: &str, ctx: &str, c: f32, rs: f32) {
    if c.to_bits() != rs.to_bits() {
        panic!(
            "{what} mismatch for {ctx}:\n  C    = {c:?} (bits 0x{:08x})\n  Rust = {rs:?} (bits 0x{:08x})",
            c.to_bits(),
            rs.to_bits()
        );
    }
}

#[track_caller]
pub fn assert_v_bits(what: &str, ctx: &str, c: c2v, rs: c2v) {
    assert_f32_bits(what, &format!("{ctx} [.x]"), c.x, rs.x);
    assert_f32_bits(what, &format!("{ctx} [.y]"), c.y, rs.y);
}

#[track_caller]
pub fn assert_int(what: &str, ctx: &str, c: c_int, rs: c_int) {
    assert_eq!(c, rs, "{what} mismatch for {ctx}: C = {c}, Rust = {rs}");
}

// ---------------------------------------------------------------------------
// Input corpora
// ---------------------------------------------------------------------------

/// Scalars that hit every interesting IEEE-754 class plus the magnitudes used
/// by the hard-coded shapes inside `circle_collide`.
pub const SCALARS: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    10.0,
    -10.0,
    15.0,
    -15.0,
    20.0,
    -20.0,
    40.0,
    -40.0,
    70.0,
    -70.0,
    100.0,
    -100.0,
    1e-30,
    -1e-30,
    1e30,
    -1e30,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-45,  // smallest positive subnormal
    -1e-45,
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
];

/// Deterministic xorshift so both sides see identical bit patterns.
pub struct Rng(pub u32);

impl Rng {
    pub fn new() -> Rng {
        Rng(0x1234_5678)
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Uniform in `[-lo_hi, lo_hi]`, with exact representability of the bounds.
    pub fn range(&mut self, lo_hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * lo_hi
    }
    /// Any bit pattern at all, including NaNs and infinities.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    pub fn v(&mut self, lo_hi: f32) -> c2v {
        c2v {
            x: self.range(lo_hi),
            y: self.range(lo_hi),
        }
    }
    pub fn any_v(&mut self) -> c2v {
        c2v {
            x: self.any_f32(),
            y: self.any_f32(),
        }
    }
}
