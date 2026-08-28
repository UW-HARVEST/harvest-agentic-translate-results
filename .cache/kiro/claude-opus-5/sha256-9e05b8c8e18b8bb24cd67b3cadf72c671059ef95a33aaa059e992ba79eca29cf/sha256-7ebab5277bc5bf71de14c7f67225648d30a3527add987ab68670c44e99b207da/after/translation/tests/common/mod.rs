//! Shared harness for the differential tests.
//!
//! Both the C reference `.so` and the Rust `.so` are opened with `libloading`
//! and every function is reached only through its exported symbol, so the
//! `#[no_mangle]` wrappers are part of what is under test.
#![allow(dead_code, non_snake_case)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Types mirroring the C declarations in c_src/include/lib.h and c_src/src/lib.c
// ---------------------------------------------------------------------------

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

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

fn find_c_so() -> PathBuf {
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // target/<profile>/deps/<test binary> -> target/<profile>/libgen_ray_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary lives in target/<profile>/deps");
    let so = profile_dir.join("libgen_ray_lib.so");
    if so.exists() {
        return so;
    }

    // `cargo test` builds the integration tests but not the `cdylib` artifact,
    // so build it on demand.  A separate target directory keeps this out of the
    // build lock held by the outer `cargo test`.
    let release = profile_dir
        .file_name()
        .map(|n| n == "release")
        .unwrap_or(false);
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target/difftest");
    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.arg("build")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir);
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status().expect("spawning cargo to build the cdylib");
    assert!(status.success(), "cargo build of the cdylib failed");

    let built = target_dir
        .join(if release { "release" } else { "debug" })
        .join("libgen_ray_lib.so");
    assert!(
        built.exists(),
        "{} was not produced; build the cdylib with `cargo build` first",
        built.display()
    );
    built
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        Libs {
            c: Library::new(&c_path)
                .unwrap_or_else(|e| panic!("loading {}: {e}", c_path.display())),
            rs: Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("loading {}: {e}", rs_path.display())),
        }
    })
}

/// Fetch the same symbol from both libraries, typed as `T` (an
/// `extern "C" fn(..)` pointer).  `name` must be nul-terminated.
pub fn syms<T: Copy>(name: &[u8]) -> (T, T) {
    let l = libs();
    assert_eq!(name.last(), Some(&0), "symbol name must be nul-terminated");
    unsafe {
        let c: Symbol<T> = l
            .c
            .get(name)
            .unwrap_or_else(|e| panic!("C .so is missing {:?}: {e}", pretty(name)));
        let r: Symbol<T> = l
            .rs
            .get(name)
            .unwrap_or_else(|e| panic!("Rust .so is missing {:?}: {e}", pretty(name)));
        (*c, *r)
    }
}

fn pretty(name: &[u8]) -> String {
    String::from_utf8_lossy(&name[..name.len() - 1]).into_owned()
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// Bit-identical, except that any NaN is considered equal to any other NaN:
/// NaN payload propagation through SSE arithmetic is not part of the
/// observable contract and depends on operand ordering chosen by each
/// compiler.
pub fn f32_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

pub fn v_eq(a: C2v, b: C2v) -> bool {
    f32_eq(a.x, b.x) && f32_eq(a.y, b.y)
}

pub fn cast_eq(a: C2Raycast, b: C2Raycast) -> bool {
    f32_eq(a.t, b.t) && v_eq(a.n, b.n)
}

pub fn show_f32(v: f32) -> String {
    format!("{v:?} [{:#010x}]", v.to_bits())
}

pub fn show_v(v: C2v) -> String {
    format!("({}, {})", show_f32(v.x), show_f32(v.y))
}

pub fn show_cast(c: C2Raycast) -> String {
    format!("{{ t: {}, n: {} }}", show_f32(c.t), show_v(c.n))
}

/// Sentinel written into `out` before each call.  If one implementation
/// writes the struct and the other leaves it untouched, the comparison fails.
pub const SENTINEL: C2Raycast = C2Raycast {
    t: -12345.678,
    n: C2v {
        x: -98765.43,
        y: 54321.125,
    },
};

// ---------------------------------------------------------------------------
// Deterministic input generation
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A float drawn from a distribution tuned to hit the branch boundaries in
    /// the C code: exact zeros and signed zeros, quarter-integer grid values
    /// (so `<=` / `>=` boundaries are actually reached), plain uniform values,
    /// and occasional extreme magnitudes.
    pub fn f32v(&mut self) -> f32 {
        match self.below(100) {
            0..=9 => {
                const SPECIALS: [f32; 10] = [
                    0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 2.0, -2.0, f32::MIN_POSITIVE, 1e-30,
                ];
                SPECIALS[self.below(SPECIALS.len() as u64) as usize]
            }
            10..=49 => {
                // quarter-integer grid in [-8, 8]
                let n = self.below(65) as i64 - 32;
                n as f32 * 0.25
            }
            50..=94 => (self.unit() - 0.5) * 20.0,
            95..=98 => (self.unit() - 0.5) * 2.0e6,
            _ => (self.unit() - 0.5) * 1.0e-6,
        }
    }

    /// Non-negative radius-like value; occasionally zero or negative, since
    /// the C code never validates its inputs.
    pub fn radius(&mut self) -> f32 {
        match self.below(20) {
            0 => 0.0,
            1 => -0.0,
            2 => -(self.unit() * 4.0),
            3..=11 => (self.below(33) as f32) * 0.25,
            _ => self.unit() * 8.0,
        }
    }

    pub fn vec(&mut self) -> C2v {
        C2v {
            x: self.f32v(),
            y: self.f32v(),
        }
    }

    pub fn aabb(&mut self) -> C2AABB {
        let a = self.vec();
        let b = self.vec();
        // Mostly well-formed (min <= max), sometimes deliberately inverted.
        if self.below(10) == 0 {
            C2AABB { min: a, max: b }
        } else {
            C2AABB {
                min: C2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
                max: C2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
            }
        }
    }

    pub fn circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.vec(),
            r: self.radius(),
        }
    }

    pub fn capsule(&mut self) -> C2Capsule {
        let a = self.vec();
        // Degenerate capsules (a == b) make c2Norm divide by zero; the C code
        // does it too, so keep a few.
        let b = if self.below(25) == 0 { a } else { self.vec() };
        C2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }

    pub fn ray(&mut self) -> C2Ray {
        let p = self.vec();
        let d = match self.below(8) {
            0 => self.vec(),               // unnormalised
            1 => C2v { x: 0.0, y: 0.0 },    // degenerate
            _ => {
                let v = self.vec();
                let len = (v.x * v.x + v.y * v.y).sqrt();
                if len == 0.0 || !len.is_finite() {
                    C2v { x: 1.0, y: 0.0 }
                } else {
                    C2v {
                        x: v.x / len,
                        y: v.y / len,
                    }
                }
            }
        };
        let t = match self.below(10) {
            0 => 0.0,
            1 => -(self.unit() * 5.0),
            2 => (self.below(33) as f32) * 0.25,
            _ => self.unit() * 30.0,
        };
        C2Ray { p, d, t }
    }
}

/// Number of random cases per function; override with `DIFF_ITERS`.
pub fn iters(default: usize) -> usize {
    std::env::var("DIFF_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
