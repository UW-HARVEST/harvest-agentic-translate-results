//! Shared harness: loads the C `.so` and the Rust `.so` side by side and
//! exposes the mirrored `#[repr(C)]` types plus a deterministic PRNG.
//!
//! Every call in the test-suite goes through `libloading`, including the Rust
//! side, so the `#[no_mangle]` export wrappers are what actually gets exercised.

#![allow(non_camel_case_types, non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Mirrored C types
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy { radius: 0.0, count: 0, verts: [c2v::default(); 8] }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// `struct c2Simplex { c2sv a, b, c, d; float div; int count; }`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: i32,
}

// ---------------------------------------------------------------------------
// Byte-exact comparison helpers
// ---------------------------------------------------------------------------

/// Raw object representation of a `Copy` value. All mirrored structs above are
/// padding-free on x86-64, so this is a well-defined byte-for-byte comparison.
pub fn raw<T: Copy>(v: &T) -> Vec<u8> {
    let p = v as *const T as *const u8;
    unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<T>()) }.to_vec()
}

/// Bit-identical float equality (so `NaN == NaN` and `0.0 != -0.0`).
pub fn f32_bits_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn assert_f32_eq(c: f32, r: f32, ctx: &str) {
    assert!(
        f32_bits_eq(c, r),
        "{ctx}: C returned {c:?} (bits {:#010x}), Rust returned {r:?} (bits {:#010x})",
        c.to_bits(),
        r.to_bits()
    );
}

pub fn assert_bytes_eq<T: Copy + std::fmt::Debug>(c: &T, r: &T, ctx: &str) {
    assert!(raw(c) == raw(r), "{ctx}:\n  C    = {c:?}\n  Rust = {r:?}");
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn find_one(dir: &Path, pred: impl Fn(&str) -> bool) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".so") && pred(n))
                .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop()
}

fn c_lib_path() -> PathBuf {
    let build = repo_root().join("c_src/build");
    find_one(&build, |n| !n.contains("capsule_lib")).unwrap_or_else(|| {
        panic!(
            "no C .so under {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// `cargo test` does not build the `cdylib` target on its own (no test artifact
/// links against it), so build it here on demand. A dedicated `--target-dir`
/// keeps this out of the cargo lock currently held by the running `cargo test`.
fn build_rust_cdylib() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = manifest.join("target/ffi-so");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(manifest)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&out);

    // Reproduce the feature selection the test binary itself was built with.
    if let Some(args) = feature_args() {
        cmd.args(args);
    }

    let status = cmd.status().expect("failed to spawn cargo to build the cdylib");
    assert!(status.success(), "building the Rust cdylib failed");

    find_one(&out.join("debug"), |n| n.contains("capsule_lib"))
        .unwrap_or_else(|| panic!("cdylib missing from {}", out.join("debug").display()))
}

/// Feature flags recovered from the compile-time `cfg`s of this test binary, so
/// the freshly built cdylib matches the configuration under test.
///
/// `Cargo.toml` currently declares no `[features]`, so there is exactly one
/// configuration and nothing to forward. If features are added later, list them
/// here as `#[cfg(feature = "...")] enabled.push("...")`.
fn feature_args() -> Option<Vec<String>> {
    let enabled: Vec<&str> = Vec::new();
    if enabled.is_empty() {
        None
    } else {
        Some(vec![
            "--no-default-features".to_string(),
            "--features".to_string(),
            enabled.join(","),
        ])
    }
}

fn rust_lib_path() -> PathBuf {
    // Explicit override, used to re-run the whole suite against the optimized
    // `--release` cdylib and prove the results are codegen-independent.
    if let Ok(p) = std::env::var("CAPSULE_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "CAPSULE_RUST_SO points at a missing file: {}", p.display());
        return p;
    }
    // If cargo happened to place a cdylib next to the test binary, use it.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for dir in [profile, deps] {
        if let Some(p) = find_one(dir, |n| n.contains("capsule_lib")) {
            return p;
        }
    }
    build_rust_cdylib()
}

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

impl Libs {
    /// Fetch the same symbol name out of both libraries.
    pub fn pair<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let bytes = name.as_bytes();
        let c = unsafe { self.c.get::<T>(bytes) }
            .unwrap_or_else(|e| panic!("C .so is missing `{name}`: {e}"));
        let rs = unsafe { self.rs.get::<T>(bytes) }
            .unwrap_or_else(|e| panic!("Rust .so is missing `{name}`: {e}"));
        (c, rs)
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", cp.display()));
        let rs = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", rp.display()));
        Libs { c, rs }
    })
}

// ---------------------------------------------------------------------------
// Deterministic input generation
// ---------------------------------------------------------------------------

/// Multiplier for the randomised case counts, from `CAPSULE_FUZZ_SCALE`.
/// Defaults to 1 so `cargo test` stays fast; set it higher for a deep run:
/// `CAPSULE_FUZZ_SCALE=50 cargo test`.
pub fn scale(n: usize) -> usize {
    static S: OnceLock<usize> = OnceLock::new();
    let k = *S.get_or_init(|| {
        std::env::var("CAPSULE_FUZZ_SCALE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(1)
    });
    n.saturating_mul(k)
}

/// xorshift64* — reproducible across runs and platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() >> 32) as u32 % n
    }

    /// Uniform in `[-mag, mag)`.
    pub fn uniform(&mut self, mag: f32) -> f32 {
        let u = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * mag
    }

    /// A "coordinate-like" float: mostly ordinary magnitudes, sometimes exact
    /// integers or zero so that ties in `<`/`>` comparisons get hit, and
    /// occasionally a hostile value (denormal, huge, inf, NaN, -0.0).
    ///
    /// The NaN cases deliberately use several distinct payloads, including
    /// negative ones, because NaN payload selection is observable through the
    /// FFI boundary and differs between naive codegen and the reference build.
    pub fn coord(&mut self) -> f32 {
        match self.below(100) {
            0..=52 => self.uniform(100.0),
            53..=67 => self.uniform(100.0).trunc(),
            68..=77 => (self.below(11) as f32) - 5.0,
            78..=82 => self.uniform(1.0e18),
            83..=87 => self.uniform(1.0e-18),
            88..=90 => 0.0,
            91..=92 => -0.0,
            93 => f32::INFINITY,
            94 => f32::NEG_INFINITY,
            95 => f32::NAN,                       // 0x7fc00000
            96 => f32::from_bits(0xffc0_0000),    // x86 "indefinite" QNaN
            97 => f32::from_bits(0x7fc0_dead),    // positive, odd payload
            98 => f32::from_bits(0xffc0_beef),    // negative, odd payload
            _ => f32::MAX,
        }
    }

    /// Like `coord`, but never NaN/inf — for paths where the C code's own
    /// control flow would otherwise diverge only through UB.
    pub fn finite_coord(&mut self) -> f32 {
        loop {
            let v = self.coord();
            if v.is_finite() {
                return v;
            }
        }
    }

    /// Radius-like: non-negative most of the time, occasionally odd.
    pub fn radius(&mut self) -> f32 {
        match self.below(20) {
            0..=11 => self.uniform(50.0).abs(),
            12..=14 => self.below(30) as f32,
            15 => 0.0,
            16 => -0.0,
            17 => self.uniform(10.0), // may be negative
            18 => 1.0e20,
            _ => self.finite_coord(),
        }
    }

    pub fn vec(&mut self) -> c2v {
        c2v { x: self.coord(), y: self.coord() }
    }

    pub fn finite_vec(&mut self) -> c2v {
        c2v { x: self.finite_coord(), y: self.finite_coord() }
    }

    pub fn rot(&mut self) -> c2r {
        // Mostly a genuine rotation, sometimes arbitrary.
        if self.below(4) == 0 {
            c2r { c: self.finite_coord(), s: self.finite_coord() }
        } else {
            let t = self.uniform(4.0);
            c2r { c: t.cos(), s: t.sin() }
        }
    }

    pub fn xform(&mut self) -> c2x {
        c2x { p: self.finite_vec(), r: self.rot() }
    }

    pub fn circle(&mut self) -> c2Circle {
        c2Circle { p: self.finite_vec(), r: self.radius() }
    }

    pub fn aabb(&mut self) -> c2AABB {
        let a = self.finite_vec();
        let b = self.finite_vec();
        if self.below(8) == 0 {
            // Degenerate / inverted boxes are legal inputs to these routines.
            c2AABB { min: a, max: b }
        } else {
            c2AABB {
                min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
            }
        }
    }

    pub fn capsule(&mut self) -> c2Capsule {
        c2Capsule { a: self.finite_vec(), b: self.finite_vec(), r: self.radius() }
    }

    /// Fully initialised simplex (no indeterminate bytes on either side).
    pub fn simplex(&mut self, count: i32) -> c2Simplex {
        let mut s = c2Simplex::default();
        for v in s.verts.iter_mut() {
            v.sA = self.finite_vec();
            v.sB = self.finite_vec();
            v.p = c2v { x: self.uniform(50.0), y: self.uniform(50.0) };
            v.u = self.uniform(10.0).abs();
            v.iA = self.below(4) as i32;
            v.iB = self.below(4) as i32;
        }
        // Occasionally make two support points coincide, which is what drives
        // the degenerate branches of c22/c23.
        if self.below(6) == 0 {
            s.verts[1].p = s.verts[0].p;
        }
        if self.below(6) == 0 {
            s.verts[2].p = s.verts[1].p;
        }
        s.div = match self.below(10) {
            0 => 0.0,
            1 => 1.0,
            _ => self.uniform(20.0),
        };
        s.count = count;
        s
    }
}
