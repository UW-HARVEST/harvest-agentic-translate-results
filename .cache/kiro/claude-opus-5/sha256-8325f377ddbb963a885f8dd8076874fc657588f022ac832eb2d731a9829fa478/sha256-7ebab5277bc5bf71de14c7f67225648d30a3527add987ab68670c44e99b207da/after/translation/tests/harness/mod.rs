//! Shared plumbing for the differential tests.
//!
//! Both the C reference library and the Rust `cdylib` are opened with
//! `libloading` and every function is reached through its exported symbol, so
//! the `#[no_mangle]` wrappers are part of what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Mirrored C types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct V {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct R {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct X {
    pub p: V,
    pub r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Circle {
    pub p: V,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AABB {
    pub min: V,
    pub max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Capsule {
    pub a: V,
    pub b: V,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [V; 8],
}

impl Default for Proxy {
    fn default() -> Self {
        Proxy {
            radius: 0.0,
            count: 0,
            verts: [V::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Sv {
    pub sA: V,
    pub sB: V,
    pub p: V,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Simplex {
    pub verts: [Sv; 4],
    pub div: f32,
    pub count: i32,
}

impl Default for Simplex {
    fn default() -> Self {
        Simplex {
            verts: [Sv::default(); 4],
            div: 0.0,
            count: 0,
        }
    }
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn rust_so_path() -> PathBuf {
    // current_exe() == <target>/<profile>/deps/<test binary>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let p = profile_dir.join("libgjk_lib.so");

    // Integration tests cannot link a `cdylib`, so cargo does not build the
    // library target on their behalf and would happily leave a stale artifact
    // in place. Always rebuild; this runs during the test phase, when the outer
    // cargo invocation holds no build lock.
    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug");
    let mut cmd =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.arg("build").arg("--lib").arg("--quiet");
    if profile == "release" {
        cmd.arg("--release");
    }
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    // Inherited cargo/rustc env from the outer invocation confuses the nested
    // build; strip the ones that matter.
    for k in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_MAKEFLAGS"] {
        cmd.env_remove(k);
    }
    let status = cmd.status().expect("spawn cargo build --lib");
    assert!(status.success(), "nested `cargo build --lib` failed");

    assert!(
        p.exists(),
        "Rust cdylib not found at {} after building",
        p.display()
    );
    p
}

static C_LIB: OnceLock<Library> = OnceLock::new();
static RS_LIB: OnceLock<Library> = OnceLock::new();

pub fn c_lib() -> &'static Library {
    C_LIB.get_or_init(|| unsafe { Library::new(c_so_path()).expect("load C .so") })
}

pub fn rs_lib() -> &'static Library {
    RS_LIB.get_or_init(|| unsafe { Library::new(rust_so_path()).expect("load Rust .so") })
}

/// Look up `name` in both libraries and hand back the two function pointers.
pub fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let c = unsafe {
        c_lib()
            .get::<T>(name.as_bytes())
            .unwrap_or_else(|e| panic!("C .so is missing `{name}`: {e}"))
    };
    let r = unsafe {
        rs_lib()
            .get::<T>(name.as_bytes())
            .unwrap_or_else(|e| panic!("Rust .so is missing `{name}`: {e}"))
    };
    (c, r)
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-identical, with the single concession that any NaN matches any NaN
/// (the sign/payload of a NaN produced by `sqrtf` is not architecturally
/// pinned down, and neither implementation inspects it).
pub fn feq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[track_caller]
pub fn assert_f(label: &str, ctx: &dyn std::fmt::Debug, a: f32, b: f32) {
    assert!(
        feq(a, b),
        "{label}: C={a:?} ({:#010x}) vs Rust={b:?} ({:#010x})\n  input: {ctx:?}",
        a.to_bits(),
        b.to_bits()
    );
}

#[track_caller]
pub fn assert_v(label: &str, ctx: &dyn std::fmt::Debug, a: V, b: V) {
    assert!(
        feq(a.x, b.x) && feq(a.y, b.y),
        "{label}: C={a:?} vs Rust={b:?}\n  input: {ctx:?}",
    );
}

#[track_caller]
pub fn assert_r(label: &str, ctx: &dyn std::fmt::Debug, a: R, b: R) {
    assert!(
        feq(a.c, b.c) && feq(a.s, b.s),
        "{label}: C={a:?} vs Rust={b:?}\n  input: {ctx:?}",
    );
}

#[track_caller]
pub fn assert_x(label: &str, ctx: &dyn std::fmt::Debug, a: X, b: X) {
    assert!(
        feq(a.p.x, b.p.x) && feq(a.p.y, b.p.y) && feq(a.r.c, b.r.c) && feq(a.r.s, b.r.s),
        "{label}: C={a:?} vs Rust={b:?}\n  input: {ctx:?}",
    );
}

pub fn sv_eq(a: &Sv, b: &Sv) -> bool {
    feq(a.sA.x, b.sA.x)
        && feq(a.sA.y, b.sA.y)
        && feq(a.sB.x, b.sB.x)
        && feq(a.sB.y, b.sB.y)
        && feq(a.p.x, b.p.x)
        && feq(a.p.y, b.p.y)
        && feq(a.u, b.u)
        && a.iA == b.iA
        && a.iB == b.iB
}

pub fn simplex_eq(a: &Simplex, b: &Simplex) -> bool {
    a.count == b.count
        && feq(a.div, b.div)
        && (0..4).all(|i| sv_eq(&a.verts[i], &b.verts[i]))
}

#[track_caller]
pub fn assert_simplex(label: &str, ctx: &dyn std::fmt::Debug, a: &Simplex, b: &Simplex) {
    assert!(
        simplex_eq(a, b),
        "{label}:\n  C   ={a:?}\n  Rust={b:?}\n  input: {ctx:?}",
    );
}

pub fn proxy_eq(a: &Proxy, b: &Proxy) -> bool {
    a.count == b.count
        && feq(a.radius, b.radius)
        && (0..8).all(|i| feq(a.verts[i].x, b.verts[i].x) && feq(a.verts[i].y, b.verts[i].y))
}

pub fn cache_eq(a: &GJKCache, b: &GJKCache) -> bool {
    feq(a.metric, b.metric) && a.count == b.count && a.iA == b.iA && a.iB == b.iB
        && feq(a.div, b.div)
}

// ---------------------------------------------------------------------------
// Deterministic value generation
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
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

    /// Uniform in [-1, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f64 / u32::MAX as f64) as f32 * 2.0 - 1.0
    }

    /// A float drawn from a distribution that mixes "nice" values (zeros,
    /// signed zeros, small integers, halves) with wide-range magnitudes and
    /// the occasional special value.
    pub fn float(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => -1.0,
            4 => (self.below(11) as f32) - 5.0,
            5 => ((self.below(41) as f32) - 20.0) * 0.5,
            6 => self.unit() * 1.0e-7,
            7 => self.unit() * 1.0e-30,
            8 => self.unit() * 1.0e30,
            9 => self.unit() * 1.0e8,
            10 => f32::from_bits(self.next_u32()),
            _ => self.unit() * 10.0,
        }
    }

    /// Same as `float` but never NaN/inf - useful where a single non-finite
    /// input would swamp a whole struct comparison with NaNs.
    pub fn finite(&mut self) -> f32 {
        for _ in 0..64 {
            let v = self.float();
            if v.is_finite() {
                return v;
            }
        }
        0.0
    }

    pub fn v(&mut self) -> V {
        V {
            x: self.float(),
            y: self.float(),
        }
    }

    pub fn v_finite(&mut self) -> V {
        V {
            x: self.finite(),
            y: self.finite(),
        }
    }
}

/// Iteration count for the randomised sweeps. Multiply the built-in volume via
/// `GJK_FUZZ_SCALE=<n>` to run a much longer pass without editing the tests.
pub fn volume(base: u32) -> u32 {
    let scale: u32 = std::env::var("GJK_FUZZ_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);
    base.saturating_mul(scale)
}
