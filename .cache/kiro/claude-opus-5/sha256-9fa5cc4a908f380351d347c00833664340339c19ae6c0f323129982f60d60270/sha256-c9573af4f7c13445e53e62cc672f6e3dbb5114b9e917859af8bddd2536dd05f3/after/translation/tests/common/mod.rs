//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! one `Lib` handle per implementation. Rust functions are NEVER called
//! directly — every call goes through `dlsym` on the built `cdylib`, exactly as
//! an external C consumer would, so the `#[no_mangle]` / `extern "C"` export
//! wrappers and the SysV struct-passing ABI are under test too.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI-mirrored types (must match c_src/src/lib.c byte for byte)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

// Function-pointer types for every one of the 12 exported symbols.
pub type FnC2V = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnC2Mulvs = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnC2Maxv = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnC2Minv = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnC2Clampv = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnC2Sub = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnC2Dot = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnCircleCircle = unsafe extern "C" fn(c2Circle, c2Circle) -> i32;
pub type FnCircleAABB = unsafe extern "C" fn(c2Circle, c2AABB) -> i32;
pub type FnCircleCapsule = unsafe extern "C" fn(c2Circle, c2Capsule) -> i32;
pub type FnCollided = unsafe extern "C" fn(*const u8, *const u8, i32) -> i32;
pub type FnCircleCollide = unsafe extern "C" fn(f32, f32, f32) -> i32;

/// One loaded implementation, with all 12 symbols resolved up front.
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub c2V: FnC2V,
    pub c2Mulvs: FnC2Mulvs,
    pub c2Maxv: FnC2Maxv,
    pub c2Minv: FnC2Minv,
    pub c2Clampv: FnC2Clampv,
    pub c2Sub: FnC2Sub,
    pub c2Dot: FnC2Dot,
    pub c2CircletoCircle: FnCircleCircle,
    pub c2CircletoAABB: FnCircleAABB,
    pub c2CircletoCapsule: FnCircleCapsule,
    pub c2Collided: FnCollided,
    pub circle_collide: FnCircleCollide,
}

impl Lib {
    fn load(name: &'static str, path: PathBuf) -> Lib {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({:?}): {e}", name, path));
            macro_rules! sym {
                ($n:literal, $t:ty) => {{
                    let s: Symbol<$t> = lib.get(concat!($n, "\0").as_bytes()).unwrap_or_else(|e| {
                        panic!("{} .so is missing symbol `{}`: {e}", name, $n)
                    });
                    *s
                }};
            }
            let l = Lib {
                name,
                c2V: sym!("c2V", FnC2V),
                c2Mulvs: sym!("c2Mulvs", FnC2Mulvs),
                c2Maxv: sym!("c2Maxv", FnC2Maxv),
                c2Minv: sym!("c2Minv", FnC2Minv),
                c2Clampv: sym!("c2Clampv", FnC2Clampv),
                c2Sub: sym!("c2Sub", FnC2Sub),
                c2Dot: sym!("c2Dot", FnC2Dot),
                c2CircletoCircle: sym!("c2CircletoCircle", FnCircleCircle),
                c2CircletoAABB: sym!("c2CircletoAABB", FnCircleAABB),
                c2CircletoCapsule: sym!("c2CircletoCapsule", FnCircleCapsule),
                c2Collided: sym!("c2Collided", FnCollided),
                circle_collide: sym!("circle_collide", FnCircleCollide),
                path,
                _lib: lib,
            };
            l
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so found in {:?} — build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build
        )
    })
}

fn find_rust_so() -> PathBuf {
    let candidates: Vec<PathBuf> = if let Ok(p) = std::env::var("RUST_SO_PATH") {
        vec![PathBuf::from(p)]
    } else {
        // NOTE: `cargo test` does *not* build the cdylib (the integration tests
        // do not link it, since the crate only produces a cdylib), so the .so on
        // disk can easily be stale. Prefer the artifact next to this test binary,
        // then fall back to the two profile dirs, and hard-fail on staleness
        // below rather than silently verifying an old build.
        let exe = std::env::current_exe().expect("current_exe");
        let mut v = Vec::new();
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            v.push(profile_dir.join("libcircle_collide_lib.so"));
        }
        let t = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        v.push(t.join("debug/libcircle_collide_lib.so"));
        v.push(t.join("release/libcircle_collide_lib.so"));
        v
    };

    let so = candidates
        .iter()
        .find(|c| c.is_file())
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found. Tried {:?}.\n\
                 `cargo test` does not build the cdylib -- run `./run_tests.sh`, \
                 or `cargo build && cargo build --release` first.",
                candidates
            )
        })
        .clone();

    assert_so_fresh(&so);
    so
}

/// Refuse to run against a `.so` older than the Rust source it should contain.
fn assert_so_fresh(so: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let m = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or_else(|e| panic!("cannot stat {p:?}: {e}"))
    };
    let (so_t, src_t) = (m(so), m(&src));
    if so_t < src_t {
        panic!(
            "STALE ARTIFACT: {so:?} was built before {src:?} was last modified.\n\
             `cargo test` does not rebuild the cdylib. Run `./run_tests.sh` \
             (or `cargo build --release`) and re-run the tests."
        );
    }
}

/// The C reference implementation, loaded from `c_src/build/*.so`.
pub fn c_lib() -> &'static Lib {
    static ONCE: std::sync::OnceLock<Lib> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| Lib::load("C", find_c_so()))
}

/// The Rust translation, loaded from its `cdylib` — never called directly.
pub fn rs_lib() -> &'static Lib {
    static ONCE: std::sync::OnceLock<Lib> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| Lib::load("RUST", find_rust_so()))
}

/// Convenience: both handles at once.
pub fn libs() -> (&'static Lib, &'static Lib) {
    (c_lib(), rs_lib())
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Strict bit-for-bit `f32` comparison — NaN payloads and sign bits included.
///
/// The translation reproduces the C's x86 SSE operand ordering exactly (see the
/// `mul_ss` / `add_ss` models in `src/lib.rs`), so no NaN-payload leniency is
/// needed or granted: `probe_dot`/`probe_order` demonstrate 0 mismatches over
/// millions of raw-bit-pattern inputs.
pub fn f32_eq_bits(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn v_eq(a: c2v, b: c2v) -> bool {
    f32_eq_bits(a.x, b.x) && f32_eq_bits(a.y, b.y)
}

pub fn show(v: f32) -> String {
    format!("{:e}(0x{:08x})", v, v.to_bits())
}

pub fn show_v(v: c2v) -> String {
    format!("({}, {})", show(v.x), show(v.y))
}

// ---------------------------------------------------------------------------
// Deterministic RNG — SplitMix64, fixed seed, no external crates
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    /// Fixed project-wide seed, salted per test so rows do not share streams.
    pub fn seeded(salt: u64) -> Rng {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-mag, mag).
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    /// A "normal" coordinate: mostly in [-200,200], plus a spread of exponents.
    pub fn coord(&mut self) -> f32 {
        match self.next_u32() % 8 {
            0..=4 => self.sym(200.0),
            5 => self.sym(1.0),
            6 => self.sym(1.0e6),
            _ => self.sym(1.0e-6),
        }
    }
    /// A non-negative radius with a spread of magnitudes.
    pub fn radius(&mut self) -> f32 {
        match self.next_u32() % 6 {
            0..=3 => self.unit() * 100.0,
            4 => self.unit(),
            _ => self.unit() * 1.0e5,
        }
    }
    /// An arbitrary bit pattern reinterpreted as f32 (may be NaN/inf/subnormal).
    pub fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    pub fn vec_coord(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn vec_raw(&mut self) -> c2v {
        c2v {
            x: self.raw_f32(),
            y: self.raw_f32(),
        }
    }
    /// A coordinate pair, occasionally an exact-boundary special value.
    pub fn vec_proper_or_coord(&mut self) -> c2v {
        if self.next_u32().is_multiple_of(8) {
            let sp = SPECIAL_F32[(self.next_u32() as usize) % SPECIAL_F32.len()];
            c2v { x: sp, y: self.coord() }
        } else {
            self.vec_coord()
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.vec_coord(),
            r: self.radius(),
        }
    }
    /// A proper AABB (min <= max on both axes).
    pub fn aabb_proper(&mut self) -> c2AABB {
        let a = self.vec_coord();
        let b = self.vec_coord();
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
    }
    pub fn capsule(&mut self) -> c2Capsule {
        c2Capsule {
            a: self.vec_coord(),
            b: self.vec_coord(),
            r: self.radius(),
        }
    }
}

/// The 26 special / boundary `f32` values used by the "special float" rows.
pub const SPECIAL_F32: [f32; 26] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    -f32::EPSILON,
    0.5,
    -0.5,
    2.0,
    -2.0,
    1.0e-30,
    1.0e30,
    -1.0e30,
    20.0,
    -70.0,
    -40.0,
    -15.0,
    100.0,
];

/// Subnormals and NaNs with explicit payloads — raw bit patterns.
pub const SPECIAL_BITS: [u32; 12] = [
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007F_FFFF, // largest subnormal
    0x7F80_0001, // signalling NaN, payload 1
    0xFF80_0001, // negative signalling NaN
    0x7FC0_0000, // quiet NaN (default)
    0xFFC0_0000, // negative quiet NaN
    0x7FFF_FFFF, // quiet NaN, all payload bits set
    0xFFFF_FFFF, // negative quiet NaN, all payload bits set
    0x7F7F_FFFF, // f32::MAX
    0x0080_0000, // f32::MIN_POSITIVE
    0x3F80_0000, // 1.0
];

/// Assertion helper: fail with a full dump of the diverging inputs.
#[macro_export]
macro_rules! diff_assert {
    ($cond:expr, $($arg:tt)*) => {
        if !($cond) {
            panic!("C/Rust divergence: {}", format!($($arg)*));
        }
    };
}
