//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes a symmetric view of the exported C ABI so tests can compare
//! them symbol-for-symbol.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::env;
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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

impl c2v {
    pub fn bits(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;

pub type FnC2V = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnDot = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnCC = unsafe extern "C" fn(c2Circle, c2Circle) -> i32;
pub type FnCA = unsafe extern "C" fn(c2Circle, c2AABB) -> i32;
pub type FnAA = unsafe extern "C" fn(c2AABB, c2AABB) -> i32;
pub type FnCollided =
    unsafe extern "C" fn(*const std::ffi::c_void, i32, *const std::ffi::c_void, i32) -> i32;

/// One loaded implementation (either the C or the Rust shared object).
pub struct Impl {
    _lib: Library,
    pub name: &'static str,
    pub c2V: FnC2V,
    pub c2Maxv: FnVV,
    pub c2Minv: FnVV,
    pub c2Clampv: FnVVV,
    pub c2Sub: FnVV,
    pub c2Dot: FnDot,
    pub c2CircletoCircle: FnCC,
    pub c2CircletoAABB: FnCA,
    pub c2AABBtoAABB: FnAA,
    pub collided: FnCollided,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib.get(name).unwrap_or_else(|e| {
            panic!(
                "missing exported symbol {}: {e}",
                String::from_utf8_lossy(name)
            )
        });
        *s
    }
}

impl Impl {
    pub fn load(path: &Path, name: &'static str) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        unsafe {
            Impl {
                name,
                c2V: sym(&lib, b"c2V"),
                c2Maxv: sym(&lib, b"c2Maxv"),
                c2Minv: sym(&lib, b"c2Minv"),
                c2Clampv: sym(&lib, b"c2Clampv"),
                c2Sub: sym(&lib, b"c2Sub"),
                c2Dot: sym(&lib, b"c2Dot"),
                c2CircletoCircle: sym(&lib, b"c2CircletoCircle"),
                c2CircletoAABB: sym(&lib, b"c2CircletoAABB"),
                c2AABBtoAABB: sym(&lib, b"c2AABBtoAABB"),
                collided: sym(&lib, b"collided"),
                _lib: lib,
            }
        }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so(dir: &Path, stem_contains: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            let f = p.file_name()?.to_string_lossy().to_string();
            if f.starts_with("lib") && f.contains(stem_contains) {
                return Some(p);
            }
        }
    }
    None
}

pub fn c_lib_path() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    // The CMake project name is derived from the parent directory name, so
    // discover the artifact rather than hard-coding it.
    find_so(&build, "").unwrap_or_else(|| {
        panic!(
            "no C .so found in {} - build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locate the Rust cdylib next to the running test binary
/// (`target/<profile>/deps/<test>` -> `target/<profile>/`).
///
/// `cargo test` does not emit the `cdylib` artifact for a cdylib-only crate,
/// so build it on demand with the same profile before looking for it. This
/// keeps `cargo test` a single, self-contained command.
pub fn rust_lib_path() -> PathBuf {
    static ONCE: std::sync::Once = std::sync::Once::new();

    let exe = env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().and_then(|s| s.to_str()) == Some("deps") {
        dir.pop();
    }
    let release = dir.file_name().and_then(|s| s.to_str()) == Some("release");

    ONCE.call_once(|| {
        let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.current_dir(env!("CARGO_MANIFEST_DIR")).arg("build");
        if release {
            cmd.arg("--release");
        }
        // Propagate the feature selection the test binary itself was built
        // with, so the .so under test matches this configuration.
        if let Ok(feats) = env::var("HARNESS_CARGO_FEATURES") {
            cmd.arg("--no-default-features");
            if !feats.is_empty() {
                cmd.arg("--features").arg(feats);
            }
        }
        let status = cmd.status().expect("failed to spawn cargo build");
        assert!(status.success(), "cargo build of the cdylib failed");
    });

    if let Some(p) = find_so(&dir, "collided_lib") {
        return p;
    }
    panic!(
        "no Rust cdylib (libcollided_lib.so) found in {}",
        dir.display()
    );
}

pub fn both() -> (Impl, Impl) {
    (
        Impl::load(&c_lib_path(), "C"),
        Impl::load(&rust_lib_path(), "Rust"),
    )
}

/// Interesting f32 values: normal, zero signs, subnormals, extremes, NaNs.
pub fn interesting_f32() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        3.0,
        -3.0,
        1e-30,
        -1e-30,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),  // smallest subnormal
        f32::from_bits(0x8000_0001),
        1e30,
        -1e30,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234), // quiet NaN, custom payload
        f32::from_bits(0x7F80_0001), // signalling NaN
        16777216.0,                  // 2^24, precision boundary
        16777217.0,
        0.1,
        -0.1,
        123.456,
        -987.654,
    ]
}

/// Small deterministic PRNG (xorshift64*) for randomised sweeps.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
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
    /// Any f32 bit pattern, including NaNs/infinities.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "reasonable" geometric coordinate in [-100, 100).
    pub fn coord(&mut self) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        u * 200.0 - 100.0
    }
}

pub fn assert_f32_bits_eq(what: &str, c: f32, r: f32, ctx: &str) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{what} mismatch: C={c:?} (0x{:08X}) vs Rust={r:?} (0x{:08X}) for {ctx}",
        c.to_bits(),
        r.to_bits()
    );
}

pub fn assert_c2v_eq(what: &str, c: c2v, r: c2v, ctx: &str) {
    assert!(
        c.bits() == r.bits(),
        "{what} mismatch: C={{x:{:?} (0x{:08X}), y:{:?} (0x{:08X})}} vs \
         Rust={{x:{:?} (0x{:08X}), y:{:?} (0x{:08X})}} for {ctx}",
        c.x,
        c.x.to_bits(),
        c.y,
        c.y.to_bits(),
        r.x,
        r.x.to_bits(),
        r.y,
        r.y.to_bits()
    );
}
