//! Shared differential-test harness.
//!
//! Loads BOTH shared objects — the C one built by `c_src/CMakeLists.txt` and the
//! Rust `cdylib` — through `libloading`, and exposes them behind an identical
//! `Api` struct. Tests never call the Rust functions directly; every call goes
//! through the `.so`'s exported symbol, so the `#[no_mangle]` / `extern "C"`
//! wrappers and the struct-passing ABI are under test too.
#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI-compatible mirrors of the C types (c_src/src/lib.c:3-16)
// ---------------------------------------------------------------------------

/// `typedef struct c2v { float x; float y; } c2v;` — 8 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;` — 12 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;` — 16 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;

impl C2v {
    pub fn bits(self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

/// Bit-exact equality: `==` on floats would call NaN != NaN and would also
/// conflate `+0.0` with `-0.0`. The C returns raw bit patterns, so compare bits.
pub fn same_v(a: C2v, b: C2v) -> bool {
    a.bits() == b.bits()
}

pub fn same_f(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn fmt_v(v: C2v) -> String {
    format!("({:e}/{:#010x}, {:e}/{:#010x})", v.x, v.x.to_bits(), v.y, v.y.to_bits())
}

pub fn fmt_c(c: C2Circle) -> String {
    format!("Circle{{p:{}, r:{:e}/{:#010x}}}", fmt_v(c.p), c.r, c.r.to_bits())
}

pub fn fmt_b(b: C2Aabb) -> String {
    format!("AABB{{min:{}, max:{}}}", fmt_v(b.min), fmt_v(b.max))
}

// ---------------------------------------------------------------------------
// The loaded API
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
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

/// `Library` is leaked so the extracted function pointers stay valid for the
/// whole test process (they are only valid while the library is mapped).
fn load(name: &'static str, path: PathBuf) -> Api {
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen {} ({:?}) failed: {e}", name, path))
    }));
    macro_rules! sym {
        ($t:ty, $n:literal) => {{
            let s: Symbol<$t> = unsafe { lib.get(concat!($n, "\0").as_bytes()) }
                .unwrap_or_else(|e| panic!("{} does not export `{}`: {e}", name, $n));
            *s
        }};
    }
    Api {
        name,
        path,
        c2V: sym!(extern "C" fn(f32, f32) -> C2v, "c2V"),
        c2Maxv: sym!(extern "C" fn(C2v, C2v) -> C2v, "c2Maxv"),
        c2Minv: sym!(extern "C" fn(C2v, C2v) -> C2v, "c2Minv"),
        c2Clampv: sym!(extern "C" fn(C2v, C2v, C2v) -> C2v, "c2Clampv"),
        c2Sub: sym!(extern "C" fn(C2v, C2v) -> C2v, "c2Sub"),
        c2Dot: sym!(extern "C" fn(C2v, C2v) -> f32, "c2Dot"),
        c2CircletoCircle: sym!(extern "C" fn(C2Circle, C2Circle) -> c_int, "c2CircletoCircle"),
        c2CircletoAABB: sym!(extern "C" fn(C2Circle, C2Aabb) -> c_int, "c2CircletoAABB"),
        c2AABBtoAABB: sym!(extern "C" fn(C2Aabb, C2Aabb) -> c_int, "c2AABBtoAABB"),
        collided: sym!(
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
            "collided"
        ),
    }
}

/// `<workspace>/c_src/build/lib<parent-dir-name>.so`. The CMake project name is
/// derived from the parent directory name, so the file is located by globbing
/// rather than hard-coding it.
fn c_so_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent dir")
        .to_path_buf();
    let build = root.join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {:?}: {e}\nBuild the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one C .so in {:?}, got {:?}", build, found);
    found.pop().unwrap()
}

/// The Rust `cdylib` for the profile the test binary was built with
/// (`target/<profile>/libcollided_lib.so`).
///
/// `cargo test` does **not** build a `cdylib`-only crate's shared object, so the
/// runner script must `cargo build` first; if the matching profile's artifact is
/// absent we fall back to any other profile rather than silently passing.
fn rust_so_path() -> PathBuf {
    const LIB: &str = "libcollided_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test-bin>  ->  target/<profile>/
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test binary lives in target/<profile>/deps/")
        .to_path_buf();

    let primary = profile_dir.join(LIB);
    if primary.is_file() {
        return primary;
    }

    let target_dir = profile_dir.parent().unwrap_or(&profile_dir).to_path_buf();
    for alt in ["release", "debug"] {
        let p = target_dir.join(alt).join(LIB);
        if p.is_file() {
            eprintln!("warning: using {:?} ({} not present)", p, primary.display());
            return p;
        }
    }
    panic!(
        "{LIB} not found in {:?} — build the cdylib first:\n  cd translation && cargo build",
        profile_dir
    );
}

pub fn c_api() -> Api {
    load("C", c_so_path())
}

pub fn rust_api() -> Api {
    load("Rust", rust_so_path())
}

/// Loads both libraries once per test binary.
pub fn both() -> (&'static Api, &'static Api) {
    use std::sync::OnceLock;
    static PAIR: OnceLock<(Api, Api)> = OnceLock::new();
    let (c, r) = PAIR.get_or_init(|| (c_api(), rust_api()));
    (c, r)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible failures, no dev-dep needed)
// ---------------------------------------------------------------------------

/// SplitMix64. Chosen because it needs no state beyond a `u64` and produces
/// well-distributed bits, which matters: we reinterpret raw bits as `f32`, so a
/// weak generator would under-sample the NaN/inf exponent range.
pub struct Rng(u64);

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn seeded() -> Self {
        Rng(SEED)
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Any of the 2^32 bit patterns — includes ±0, subnormals, ±inf, quiet and
    /// **signalling** NaNs with arbitrary payloads. These are all valid inputs
    /// because the C validates nothing.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A finite float in `[-range, range]`, i.e. "plausible geometry" — needed
    /// because fully-random bit patterns are overwhelmingly huge-exponent values
    /// and would almost never produce an interesting overlap/no-overlap mix.
    pub fn finite_f32(&mut self, range: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * range
    }

    pub fn any_v(&mut self) -> C2v {
        C2v { x: self.any_f32(), y: self.any_f32() }
    }
    pub fn finite_v(&mut self, range: f32) -> C2v {
        C2v { x: self.finite_f32(range), y: self.finite_f32(range) }
    }
    pub fn any_circle(&mut self) -> C2Circle {
        C2Circle { p: self.any_v(), r: self.any_f32() }
    }
    pub fn finite_circle(&mut self, range: f32, rmax: f32) -> C2Circle {
        C2Circle { p: self.finite_v(range), r: self.finite_f32(rmax).abs() }
    }
    pub fn any_aabb(&mut self) -> C2Aabb {
        C2Aabb { min: self.any_v(), max: self.any_v() }
    }
    /// Well-ordered box: `min <= max` componentwise.
    pub fn ordered_aabb(&mut self, range: f32) -> C2Aabb {
        let a = self.finite_v(range);
        let b = self.finite_v(range);
        C2Aabb {
            min: C2v { x: a.x.min(b.x), y: a.y.min(b.y) },
            max: C2v { x: a.x.max(b.x), y: a.y.max(b.y) },
        }
    }
    /// Inverted box: `min > max` componentwise — a shape the C never normalises,
    /// so `c2Clampv`'s `max(lo, min(a, hi))` takes a different path.
    pub fn inverted_aabb(&mut self, range: f32) -> C2Aabb {
        let b = self.ordered_aabb(range);
        C2Aabb { min: b.max, max: b.min }
    }
}

// ---------------------------------------------------------------------------
// Boundary corpus of interesting f32 values
// ---------------------------------------------------------------------------

/// Every float class the SSE arithmetic in `lib.c` treats specially, plus NaNs
/// with *distinct payloads* so payload-selection bugs are observable (a wrong
/// operand order in `c2Dot` is invisible if every NaN is the same NaN).
pub const SPECIAL_BITS: &[u32] = &[
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007F_FFFF, // largest positive subnormal
    0x0080_0000, // f32::MIN_POSITIVE (smallest normal)
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x3F80_0001, // 1.0 + 1ulp
    0x4000_0000, // 2.0
    0x4120_0000, // 10.0
    0x7F7F_FFFF, // f32::MAX
    0xFF7F_FFFF, // f32::MIN
    0x5F80_0000, // 2^64 (squares to inf)
    0x1F80_0000, // 2^-64 (squares to 0)
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // canonical quiet NaN
    0xFFC0_0000, // negative quiet NaN == x86 QNaN-indefinite
    0x7FC0_1234, // quiet NaN, payload A
    0x7FDE_ADBE, // quiet NaN, payload B (distinct from A)
    0x7F80_0001, // signalling NaN, tiny payload
    0x7FBF_FFFF, // signalling NaN, max payload
    0xFF81_2345, // negative signalling NaN
];

pub fn specials() -> Vec<f32> {
    SPECIAL_BITS.iter().copied().map(f32::from_bits).collect()
}
