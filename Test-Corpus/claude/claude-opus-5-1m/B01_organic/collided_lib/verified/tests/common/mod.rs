//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes them behind identical `extern "C"` function
//! pointers. Nothing in the tests ever calls a Rust function directly — every
//! call crosses the FFI boundary through a `dlsym`'d symbol, exactly as an
//! external consumer would, so the `#[no_mangle]` export wrappers and the
//! struct-passing ABI are under test too.

#![allow(non_snake_case, dead_code)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Mirrored C types
// ---------------------------------------------------------------------------

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers (float `==` would make NaN and -0.0 lie)
// ---------------------------------------------------------------------------

pub trait Bits {
    type Repr: PartialEq + std::fmt::Debug;
    fn bits(&self) -> Self::Repr;
}

impl Bits for f32 {
    type Repr = u32;
    fn bits(&self) -> u32 {
        self.to_bits()
    }
}

impl Bits for C2v {
    type Repr = (u32, u32);
    fn bits(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

impl Bits for C2Circle {
    type Repr = (u32, u32, u32);
    fn bits(&self) -> (u32, u32, u32) {
        (self.p.x.to_bits(), self.p.y.to_bits(), self.r.to_bits())
    }
}

impl Bits for C2Aabb {
    type Repr = (u32, u32, u32, u32);
    fn bits(&self) -> (u32, u32, u32, u32) {
        (
            self.min.x.to_bits(),
            self.min.y.to_bits(),
            self.max.x.to_bits(),
            self.max.y.to_bits(),
        )
    }
}

impl Bits for c_int {
    type Repr = c_int;
    fn bits(&self) -> c_int {
        *self
    }
}

/// Assert that a C result and a Rust result are byte-identical.
#[track_caller]
pub fn same<T: Bits + std::fmt::Debug>(what: &str, ctx: impl std::fmt::Debug, c: T, r: T) {
    assert_eq!(
        c.bits(),
        r.bits(),
        "{what} diverged\n  input : {ctx:?}\n  C     : {c:?} (bits {:?})\n  Rust  : {r:?} (bits {:?})",
        c.bits(),
        r.bits()
    );
}

// ---------------------------------------------------------------------------
// The loaded API
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub c2V: extern "C" fn(f32, f32) -> C2v,
    pub c2Maxv: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Minv: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Clampv: extern "C" fn(C2v, C2v, C2v) -> C2v,
    pub c2Sub: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Dot: extern "C" fn(C2v, C2v) -> f32,
    pub c2CircletoCircle: extern "C" fn(C2Circle, C2Circle) -> c_int,
    pub c2CircletoAABB: extern "C" fn(C2Circle, C2Aabb) -> c_int,
    pub c2AABBtoAABB: extern "C" fn(C2Aabb, C2Aabb) -> c_int,
    pub collided: unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
}

impl Api {
    fn load(name: &'static str, path: &PathBuf) -> Api {
        // Leaked so the returned function pointers stay valid for 'static.
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name))
        }));
        macro_rules! sym {
            ($n:literal, $t:ty) => {{
                let s: libloading::Symbol<'static, $t> = unsafe {
                    lib.get(concat!($n, "\0").as_bytes()).unwrap_or_else(|e| {
                        panic!("{} missing symbol {}: {e}", name, $n)
                    })
                };
                *s
            }};
        }
        Api {
            name,
            c2V: sym!("c2V", extern "C" fn(f32, f32) -> C2v),
            c2Maxv: sym!("c2Maxv", extern "C" fn(C2v, C2v) -> C2v),
            c2Minv: sym!("c2Minv", extern "C" fn(C2v, C2v) -> C2v),
            c2Clampv: sym!("c2Clampv", extern "C" fn(C2v, C2v, C2v) -> C2v),
            c2Sub: sym!("c2Sub", extern "C" fn(C2v, C2v) -> C2v),
            c2Dot: sym!("c2Dot", extern "C" fn(C2v, C2v) -> f32),
            c2CircletoCircle: sym!("c2CircletoCircle", extern "C" fn(C2Circle, C2Circle) -> c_int),
            c2CircletoAABB: sym!("c2CircletoAABB", extern "C" fn(C2Circle, C2Aabb) -> c_int),
            c2AABBtoAABB: sym!("c2AABBtoAABB", extern "C" fn(C2Aabb, C2Aabb) -> c_int),
            collided: sym!(
                "collided",
                unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int
            ),
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C shared library, built by `c_src/CMakeLists.txt`.
fn c_so_path() -> PathBuf {
    let base = manifest_dir().join("c_src").join("build");
    for n in ["libtranslated_rust.so", "libc_src.so"] {
        let p = base.join(n);
        if p.exists() {
            return p;
        }
    }
    // Fall back to whatever single .so is in the build dir.
    if let Ok(rd) = std::fs::read_dir(&base) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                return p;
            }
        }
    }
    panic!(
        "C shared library not found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        base.display()
    );
}

/// The Rust `cdylib`, located relative to the running test executable
/// (`<target>/debug/deps/<test>` → `<target>/debug/libcollided_lib.so`).
fn rust_so_path() -> PathBuf {
    // Allows pointing the suite at e.g. the release artifact:
    //   cargo build --release && RUST_SO_PATH=target/release/libcollided_lib.so cargo test
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO_PATH does not exist: {}", p.display());
        return p;
    }
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for dir in [profile, deps] {
        let p = dir.join("libcollided_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib libcollided_lib.so not found next to {}",
        exe.display()
    );
}

/// `cargo test` builds the `test` harness for `src/lib.rs` but does **not**
/// relink the `cdylib` artifact, so a stale `libcollided_lib.so` would silently
/// be tested instead of the current sources. Refuse to run in that case.
fn assert_fresh(so: &PathBuf, sources: &[PathBuf], how_to_build: &str) {
    let so_time = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    for src in sources {
        if let Ok(t) = std::fs::metadata(src).and_then(|m| m.modified()) {
            assert!(
                t <= so_time,
                "STALE SHARED LIBRARY: {} is older than {}.\nRebuild with: {}",
                so.display(),
                src.display(),
                how_to_build
            );
        }
    }
}

static PAIR: OnceLock<(Api, Api)> = OnceLock::new();

/// `(C api, Rust api)` — both loaded via `dlopen`/`dlsym`.
pub fn apis() -> &'static (Api, Api) {
    PAIR.get_or_init(|| {
        let c_so = c_so_path();
        let rust_so = rust_so_path();
        assert_fresh(
            &c_so,
            &[
                manifest_dir().join("c_src/src/lib.c"),
                manifest_dir().join("c_src/include/lib.h"),
            ],
            "cd c_src/build && cmake --build .",
        );
        assert_fresh(
            &rust_so,
            &[
                manifest_dir().join("src/lib.rs"),
                manifest_dir().join("Cargo.toml"),
            ],
            "cargo build <same --features flags as the test run>",
        );
        (Api::load("C", &c_so), Api::load("Rust", &rust_so))
    })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (PCG32) — fixed seeds, no external dev-dependency
// ---------------------------------------------------------------------------

pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(0x853c49e6748fea9b ^ seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in `[-1, 1)` scaled by `scale`.
    pub fn uniform(&mut self, scale: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * scale
    }

    /// A "small grid" coordinate: multiples of 0.5 in [-4, 4], so that exact
    /// ties (`d2 == r2`, touching edges, equal bounds) are hit often.
    pub fn grid(&mut self) -> f32 {
        (self.below(17) as f32) * 0.5 - 4.0
    }

    /// A float drawn from the full `f32` domain, weighted towards the
    /// interesting classes (±0, ±Inf, NaN payloads, subnormals, extremes).
    pub fn wild(&mut self) -> f32 {
        const SPECIALS: [u32; 20] = [
            0x0000_0000, // +0.0
            0x8000_0000, // -0.0
            0x7f80_0000, // +Inf
            0xff80_0000, // -Inf
            0x7fc0_0000, // qNaN
            0xffc0_0000, // -qNaN
            0x7fc0_1234, // qNaN payload
            0x7f80_0001, // sNaN
            0xff80_0001, // -sNaN
            0x0000_0001, // smallest subnormal
            0x8000_0001, // -smallest subnormal
            0x007f_ffff, // largest subnormal
            0x0080_0000, // smallest normal
            0x7f7f_ffff, // f32::MAX
            0xff7f_ffff, // f32::MIN
            0x3f80_0000, // 1.0
            0xbf80_0000, // -1.0
            0x4b80_0000, // 2^24 (integer precision edge)
            0x3f7f_ffff, // 1.0 - eps
            0x0000_8000, // tiny subnormal
        ];
        match self.below(3) {
            0 => f32::from_bits(SPECIALS[self.below(SPECIALS.len() as u32) as usize]),
            1 => f32::from_bits(self.next_u32()),
            _ => self.uniform(1e3),
        }
    }

    pub fn vec_grid(&mut self) -> C2v {
        C2v {
            x: self.grid(),
            y: self.grid(),
        }
    }

    pub fn vec_wild(&mut self) -> C2v {
        C2v {
            x: self.wild(),
            y: self.wild(),
        }
    }

    /// Circle on the coarse grid with a grid radius (often 0 or negative).
    pub fn circle_grid(&mut self) -> C2Circle {
        C2Circle {
            p: self.vec_grid(),
            r: (self.below(13) as f32) * 0.5 - 1.0,
        }
    }

    pub fn circle_wild(&mut self) -> C2Circle {
        C2Circle {
            p: self.vec_wild(),
            r: self.wild(),
        }
    }

    /// AABB on the coarse grid. 1-in-4 boxes are left inverted/degenerate on
    /// purpose (no `min`/`max` normalisation) because the C code never
    /// validates that.
    pub fn aabb_grid(&mut self) -> C2Aabb {
        let a = self.vec_grid();
        let b = self.vec_grid();
        if self.below(4) == 0 {
            C2Aabb { min: a, max: b }
        } else {
            C2Aabb {
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

    pub fn aabb_wild(&mut self) -> C2Aabb {
        C2Aabb {
            min: self.vec_wild(),
            max: self.vec_wild(),
        }
    }
}

/// Number of randomized cases per configuration row.
pub const N: usize = 4000;

// ---------------------------------------------------------------------------
// `collided` invocation helpers
// ---------------------------------------------------------------------------

/// Call `collided` on both libraries with raw byte buffers and assert equality.
/// `off` shifts the payload inside an over-aligned scratch buffer so that
/// deliberately misaligned pointers can be exercised.
#[track_caller]
pub fn collided_bytes_both(
    a: &[u8],
    type_a: c_int,
    b: &[u8],
    type_b: c_int,
    off_a: usize,
    off_b: usize,
    ctx: impl std::fmt::Debug + Copy,
) {
    let (c, r) = apis();
    let mut buf_a = vec![0u8; a.len() + off_a];
    let mut buf_b = vec![0u8; b.len() + off_b];
    buf_a[off_a..].copy_from_slice(a);
    buf_b[off_b..].copy_from_slice(b);
    let pa = buf_a[off_a..].as_ptr() as *const c_void;
    let pb = buf_b[off_b..].as_ptr() as *const c_void;
    let rc = unsafe { (c.collided)(pa, type_a, pb, type_b) };
    let rr = unsafe { (r.collided)(pa, type_a, pb, type_b) };
    same("collided", (ctx, type_a, type_b, off_a, off_b), rc, rr);
}

pub fn circle_bytes(s: &C2Circle) -> [u8; 12] {
    let mut o = [0u8; 12];
    o[0..4].copy_from_slice(&s.p.x.to_ne_bytes());
    o[4..8].copy_from_slice(&s.p.y.to_ne_bytes());
    o[8..12].copy_from_slice(&s.r.to_ne_bytes());
    o
}

pub fn aabb_bytes(s: &C2Aabb) -> [u8; 16] {
    let mut o = [0u8; 16];
    o[0..4].copy_from_slice(&s.min.x.to_ne_bytes());
    o[4..8].copy_from_slice(&s.min.y.to_ne_bytes());
    o[8..12].copy_from_slice(&s.max.x.to_ne_bytes());
    o[12..16].copy_from_slice(&s.max.y.to_ne_bytes());
    o
}

/// The float values that every "boundary" sweep walks over.
///
/// Includes several *distinguishable* NaN encodings (differing sign bit and
/// payload, quiet and signalling) so that a wrong NaN-propagation order or a
/// missing SNaN-quieting step shows up as a bit-level mismatch instead of
/// hiding behind "both are NaN".
pub const EDGE_FLOATS: [f32; 25] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,                     // 0x7fc00000
    -f32::NAN,                    // 0xffc00000
    f32::from_bits(0x7fc0_dead),  // QNaN, distinctive payload
    f32::from_bits(0xffc0_beef),  // -QNaN, distinctive payload
    f32::from_bits(0x7f80_0001),  // SNaN  -> must be quieted to 0x7fc00001
    f32::from_bits(0xff80_0001),  // -SNaN -> must be quieted to 0xffc00001
    f32::from_bits(0x7fbf_ffff),  // SNaN, max payload
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    16_777_216.0, // 2^24
    1.0e-45,      // smallest subnormal
    -1.0e-45,
];

/// Out-of-range / edge `C2_TYPE` values worth passing across the FFI boundary.
pub const EDGE_TYPES: [c_int; 20] = [
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    -1,
    -2,
    -3,
    -8,
    100,
    255,
    256,
    65_536,
    c_int::MAX,
    c_int::MIN,
    -1, // u32::MAX reinterpreted as c_int
];
