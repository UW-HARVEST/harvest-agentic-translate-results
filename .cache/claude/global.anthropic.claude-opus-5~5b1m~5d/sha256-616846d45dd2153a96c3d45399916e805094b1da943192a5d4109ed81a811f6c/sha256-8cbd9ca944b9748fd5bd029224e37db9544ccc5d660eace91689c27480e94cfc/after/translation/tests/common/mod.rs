//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes one `Api` struct per
//! object holding raw `extern "C"` function pointers, so every call — including
//! the ones to the Rust crate — crosses a real FFI boundary through the
//! `#[no_mangle]` exports, exactly as an external C consumer would.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI-compatible mirrors of the C types (c_src/src/lib.c:9-28)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnC2V = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnC2Mulvs = unsafe extern "C" fn(C2v, f32) -> C2v;
pub type FnC2Binv = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnC2Clampv = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
pub type FnC2Dot = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnCircleCircle = unsafe extern "C" fn(C2Circle, C2Circle) -> i32;
pub type FnCircleAabb = unsafe extern "C" fn(C2Circle, C2Aabb) -> i32;
pub type FnCircleCapsule = unsafe extern "C" fn(C2Circle, C2Capsule) -> i32;
pub type FnCollided =
    unsafe extern "C" fn(*const std::ffi::c_void, *const std::ffi::c_void, i32) -> i32;
pub type FnCircleCollide = unsafe extern "C" fn(f32, f32, f32) -> i32;

/// All 12 exported symbols of one shared object.
pub struct Api {
    pub name: &'static str,
    pub c2V: FnC2V,
    pub c2Mulvs: FnC2Mulvs,
    pub c2Maxv: FnC2Binv,
    pub c2Minv: FnC2Binv,
    pub c2Clampv: FnC2Clampv,
    pub c2Sub: FnC2Binv,
    pub c2Dot: FnC2Dot,
    pub c2CircletoCircle: FnCircleCircle,
    pub c2CircletoAABB: FnCircleAabb,
    pub c2CircletoCapsule: FnCircleCapsule,
    pub c2Collided: FnCollided,
    pub circle_collide: FnCircleCollide,
}

unsafe fn sym<T: Copy + 'static>(lib: &'static Library, name: &str) -> T {
    let s: Symbol<'static, T> = unsafe {
        lib.get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"))
    };
    *s
}

impl Api {
    unsafe fn load(path: &Path, name: &'static str) -> Api {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()))
        }));
        unsafe {
            Api {
                name,
                c2V: sym(lib, "c2V"),
                c2Mulvs: sym(lib, "c2Mulvs"),
                c2Maxv: sym(lib, "c2Maxv"),
                c2Minv: sym(lib, "c2Minv"),
                c2Clampv: sym(lib, "c2Clampv"),
                c2Sub: sym(lib, "c2Sub"),
                c2Dot: sym(lib, "c2Dot"),
                c2CircletoCircle: sym(lib, "c2CircletoCircle"),
                c2CircletoAABB: sym(lib, "c2CircletoAABB"),
                c2CircletoCapsule: sym(lib, "c2CircletoCapsule"),
                c2Collided: sym(lib, "c2Collided"),
                circle_collide: sym(lib, "circle_collide"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path, must_contain: &[&str]) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for e in std::fs::read_dir(dir).ok()? {
        let p = e.ok()?.path();
        let f = p.file_name()?.to_string_lossy().to_string();
        if !f.ends_with(".so") || !f.starts_with("lib") {
            continue;
        }
        if must_contain.iter().any(|m| !f.contains(m)) {
            continue;
        }
        best = Some(p);
    }
    best
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = crate_root().parent().unwrap().join("c_src/build");
    find_so(&build, &[]).unwrap_or_else(|| {
        panic!(
            "no C .so found in {} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    // Prefer the release artifact (the one that ships); fall back to debug.
    for profile in ["release", "debug"] {
        let d = crate_root().join("target").join(profile);
        if let Some(p) = find_so(&d, &["circle_collide_lib"]) {
            return p;
        }
    }
    panic!("no Rust cdylib found — run `cargo build --release` first");
}

/// The two APIs under test. Loaded once per test binary.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn apis() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| unsafe {
        let cp = c_so_path();
        let rp = rust_so_path();
        eprintln!("  C   .so: {}", cp.display());
        eprintln!("  Rust.so: {}", rp.display());
        Pair {
            c: Api::load(&cp, "C"),
            r: Api::load(&rp, "Rust"),
        }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// Raw-bit view of an `f32`, so `-0.0` vs `+0.0` and NaN payloads differ.
pub fn fb(x: f32) -> u32 {
    x.to_bits()
}
pub fn vb(v: C2v) -> (u32, u32) {
    (v.x.to_bits(), v.y.to_bits())
}

pub struct Case {
    pub row: &'static str,
    pub fails: Vec<String>,
    pub n: usize,
}

impl Case {
    pub fn new(row: &'static str) -> Case {
        Case {
            row,
            fails: Vec::new(),
            n: 0,
        }
    }

    /// Record one comparison. `desc` is only formatted when it diverges.
    pub fn eq<T: PartialEq + std::fmt::Debug>(
        &mut self,
        c: T,
        r: T,
        desc: impl FnOnce() -> String,
    ) {
        self.n += 1;
        if c != r && self.fails.len() < 8 {
            self.fails
                .push(format!("{}\n     C={c:?}\n  Rust={r:?}", desc()));
        } else if c != r {
            self.fails.push(String::from("..."));
        }
    }

    pub fn finish(self) {
        assert!(self.n > 0, "row {} ran zero comparisons", self.row);
        if !self.fails.is_empty() {
            panic!(
                "CONFIG/ERROR row `{}`: {} of {} cases diverged:\n  {}",
                self.row,
                self.fails.len(),
                self.n,
                self.fails.join("\n  ")
            );
        }
        eprintln!("  row {:<42} OK ({} cases)", self.row, self.n);
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2024_C0DE_CAFE_F00D;

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }
    pub fn seeded(s: u64) -> Rng {
        Rng(SEED ^ s.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-m, m]`.
    pub fn sym(&mut self, m: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * m
    }
    /// An `f32` from completely random bits: hits every class, including
    /// signalling NaN, subnormals, ±0 and ±inf.
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "spicy" `f32`: 50% an edge value, 50% random bits, so structured
    /// specials appear far more often than uniform bit sampling gives.
    pub fn spicy(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 {
            EDGES[self.below(EDGES.len() as u32) as usize]
        } else {
            self.any_bits()
        }
    }
    pub fn v_finite(&mut self, m: f32) -> C2v {
        C2v {
            x: self.sym(m),
            y: self.sym(m),
        }
    }
    pub fn v_any(&mut self) -> C2v {
        C2v {
            x: self.any_bits(),
            y: self.any_bits(),
        }
    }
    pub fn v_spicy(&mut self) -> C2v {
        C2v {
            x: self.spicy(),
            y: self.spicy(),
        }
    }
}

/// The 12 special values called out in `CONFIGS.md`.
pub const EDGES: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    f32::MIN_POSITIVE,         // smallest normal
    f32::from_bits(1),         // smallest subnormal
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::from_bits(0x7fc0_0000), // default qNaN
    f32::from_bits(0xffc0_dead), // negative qNaN, odd payload
];

/// NaN-focused edge list (payload / sign / signalling variants).
pub const NANS: &[f32] = &[
    f32::from_bits(0x7fc0_0000),
    f32::from_bits(0xffc0_0000),
    f32::from_bits(0x7fc0_dead),
    f32::from_bits(0xffff_ffff),
    f32::from_bits(0x7f80_0001), // signalling
    f32::from_bits(0xff80_0001), // signalling, negative
    1.0,
    f32::INFINITY,
];
