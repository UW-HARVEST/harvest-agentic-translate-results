//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! every exported symbol as a typed function pointer. Rust functions are never
//! called directly — always through the `.so`'s exported symbol — so the
//! `#[no_mangle]` / `extern "C"` wrappers and the struct-passing ABI are part
//! of what is under test.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// FFI types — must be layout-identical to the C structs.
//   c2v       ->  8 bytes  (1 SSE eightbyte)
//   c2Circle  -> 12 bytes  (2 SSE eightbytes)
//   c2AABB    -> 16 bytes  (2 SSE eightbytes)
//   c2Capsule -> 20 bytes  (> 16 => MEMORY class, passed on the stack)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

pub fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

// ---------------------------------------------------------------------------
// Bitwise comparison helpers. Floats are compared by raw bits so that
// +0.0 / -0.0 and differing NaN payloads are caught.
// ---------------------------------------------------------------------------

pub fn bits_f32(a: f32) -> u32 {
    a.to_bits()
}

pub fn bits_v(a: C2v) -> (u32, u32) {
    (a.x.to_bits(), a.y.to_bits())
}

// ---------------------------------------------------------------------------
// Library location + loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so` — the project name (and hence the file name) is derived from the
/// parent directory name by `c_src/CMakeLists.txt`, so glob for it.
fn c_so_path() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {:?} — build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build
    );
    assert_eq!(found.len(), 1, "expected exactly one C .so, got {:?}", found);
    found.pop().unwrap()
}

/// The Rust `cdylib`. The test binary lives at `target/<profile>/deps/<name>`,
/// so the `.so` is two directories up. Fall back to the other profile dir.
///
/// The crate declares `crate-type = ["cdylib"]` only, so integration tests do
/// NOT link it and `cargo test` will happily run against a **stale** `.so` left
/// behind by an earlier `cargo build`. That silently invalidates every result,
/// so the file's mtime is checked against the sources here and the test fails
/// loudly rather than reporting a false pass.
pub fn rust_so_path_checked() -> PathBuf {
    const NAME: &str = "libcircle_collide_lib.so";
    let mut found: Option<PathBuf> = None;
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            let p = profile_dir.join(NAME);
            if p.exists() {
                found = Some(p);
            }
        }
    }
    if found.is_none() {
        for profile in ["release", "debug"] {
            let p = manifest_dir().join("target").join(profile).join(NAME);
            if p.exists() {
                found = Some(p);
                break;
            }
        }
    }
    let path = found
        .unwrap_or_else(|| panic!("Rust {NAME} not found — run `cargo build` first"));

    let so_time = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .expect("cannot stat the Rust .so");
    for src in ["src/lib.rs", "Cargo.toml"] {
        let sp = manifest_dir().join(src);
        if let Ok(t) = std::fs::metadata(&sp).and_then(|m| m.modified()) {
            assert!(
                t <= so_time,
                "STALE Rust .so: {path:?} is older than {sp:?}.\n\
                 `cargo test` does not rebuild a cdylib-only lib target — run\n\
                 `cargo build` (same profile) before `cargo test`, or use\n\
                 ./run_all_combos.sh which does both."
            );
        }
    }
    path
}

/// Every exported entry point of one library, resolved through `dlsym`.
pub struct Api {
    _lib: Library,
    pub c2V: unsafe extern "C" fn(f32, f32) -> C2v,
    pub c2Mulvs: unsafe extern "C" fn(C2v, f32) -> C2v,
    pub c2Maxv: unsafe extern "C" fn(C2v, C2v) -> C2v,
    pub c2Minv: unsafe extern "C" fn(C2v, C2v) -> C2v,
    pub c2Clampv: unsafe extern "C" fn(C2v, C2v, C2v) -> C2v,
    pub c2Sub: unsafe extern "C" fn(C2v, C2v) -> C2v,
    pub c2Dot: unsafe extern "C" fn(C2v, C2v) -> f32,
    pub c2CircletoCircle: unsafe extern "C" fn(C2Circle, C2Circle) -> c_int,
    pub c2CircletoAABB: unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int,
    pub c2CircletoCapsule: unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int,
    pub c2Collided: unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int,
    pub circle_collide: unsafe extern "C" fn(f32, f32, f32) -> c_int,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name));
        *s
    }};
}

impl Api {
    fn open(path: &PathBuf) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {path:?}: {e}"));
        let api = Api {
            c2V: sym!(lib, "c2V", unsafe extern "C" fn(f32, f32) -> C2v),
            c2Mulvs: sym!(lib, "c2Mulvs", unsafe extern "C" fn(C2v, f32) -> C2v),
            c2Maxv: sym!(lib, "c2Maxv", unsafe extern "C" fn(C2v, C2v) -> C2v),
            c2Minv: sym!(lib, "c2Minv", unsafe extern "C" fn(C2v, C2v) -> C2v),
            c2Clampv: sym!(lib, "c2Clampv", unsafe extern "C" fn(C2v, C2v, C2v) -> C2v),
            c2Sub: sym!(lib, "c2Sub", unsafe extern "C" fn(C2v, C2v) -> C2v),
            c2Dot: sym!(lib, "c2Dot", unsafe extern "C" fn(C2v, C2v) -> f32),
            c2CircletoCircle: sym!(
                lib,
                "c2CircletoCircle",
                unsafe extern "C" fn(C2Circle, C2Circle) -> c_int
            ),
            c2CircletoAABB: sym!(
                lib,
                "c2CircletoAABB",
                unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int
            ),
            c2CircletoCapsule: sym!(
                lib,
                "c2CircletoCapsule",
                unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int
            ),
            c2Collided: sym!(
                lib,
                "c2Collided",
                unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int
            ),
            circle_collide: sym!(lib, "circle_collide", unsafe extern "C" fn(f32, f32, f32) -> c_int),
            _lib: lib,
        };
        api
    }
}

/// The pair of libraries under differential test.
pub struct Pair {
    pub c: Api,
    pub rs: Api,
}

pub fn load() -> Pair {
    Pair {
        c: Api::open(&c_so_path()),
        rs: Api::open(&rust_so_path_checked()),
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const DEFAULT_SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn default_seeded() -> Rng {
        Rng(Self::DEFAULT_SEED)
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

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    /// A "reasonable" finite coordinate, mostly small but occasionally large.
    pub fn coord(&mut self) -> f32 {
        match self.next_u32() % 8 {
            0 => self.range(-1.0e-6, 1.0e-6),
            1 => self.range(-1.0e6, 1.0e6),
            2 => self.range(-1.0, 1.0),
            _ => self.range(-150.0, 150.0),
        }
    }

    /// A radius: usually positive, sometimes zero/negative/huge.
    pub fn radius(&mut self) -> f32 {
        match self.next_u32() % 10 {
            0 => 0.0,
            1 => -self.range(0.0, 50.0),
            2 => self.range(0.0, 1.0e-6),
            3 => self.range(1.0e6, 1.0e12),
            _ => self.range(0.0, 60.0),
        }
    }

    /// Any bit pattern reinterpreted as `f32` — covers inf, NaN, denormals,
    /// signed zeros and both extremes of the exponent range.
    pub fn any_bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    pub fn vec_coord(&mut self) -> C2v {
        C2v {
            x: self.coord(),
            y: self.coord(),
        }
    }

    pub fn vec_any(&mut self) -> C2v {
        C2v {
            x: self.any_bits_f32(),
            y: self.any_bits_f32(),
        }
    }

    pub fn circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.vec_coord(),
            r: self.radius(),
        }
    }

    /// A box whose `min`/`max` are NOT normalised — inverted boxes are a real
    /// input shape the C accepts (see CONFIGS.md row 17).
    pub fn aabb_raw(&mut self) -> C2Aabb {
        C2Aabb {
            min: self.vec_coord(),
            max: self.vec_coord(),
        }
    }

    pub fn aabb_sorted(&mut self) -> C2Aabb {
        let a = self.vec_coord();
        let b = self.vec_coord();
        C2Aabb {
            min: v(a.x.min(b.x), a.y.min(b.y)),
            max: v(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn capsule(&mut self) -> C2Capsule {
        C2Capsule {
            a: self.vec_coord(),
            b: self.vec_coord(),
            r: self.radius(),
        }
    }
}

/// Interesting scalar values every float axis should be probed with.
pub const EDGE_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1.0e-45,  // smallest positive denormal
    -1.0e-45,
    f32::MAX,
    f32::MIN,
    1.0e30,
    -1.0e30,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    20.0,
    -70.0,
    -40.0,
    -15.0,
    100.0,
    10.0,
];

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

pub fn assert_int_eq(what: &str, c: c_int, rs: c_int) {
    assert_eq!(c, rs, "int divergence in {what}: C={c} Rust={rs}");
}

pub fn assert_f32_bits_eq(what: &str, c: f32, rs: f32) {
    assert_eq!(
        c.to_bits(),
        rs.to_bits(),
        "f32 bit divergence in {what}: C={c:?} (0x{:08x}) Rust={rs:?} (0x{:08x})",
        c.to_bits(),
        rs.to_bits()
    );
}

pub fn assert_v_bits_eq(what: &str, c: C2v, rs: C2v) {
    assert_eq!(
        bits_v(c),
        bits_v(rs),
        "c2v bit divergence in {what}: C=({:?},{:?}) [0x{:08x},0x{:08x}] \
         Rust=({:?},{:?}) [0x{:08x},0x{:08x}]",
        c.x,
        c.y,
        c.x.to_bits(),
        c.y.to_bits(),
        rs.x,
        rs.y,
        rs.x.to_bits(),
        rs.y.to_bits()
    );
}
