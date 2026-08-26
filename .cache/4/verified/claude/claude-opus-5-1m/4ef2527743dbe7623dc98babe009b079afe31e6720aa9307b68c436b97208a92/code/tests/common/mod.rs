//! Shared differential-testing harness.
//!
//! Both the original C shared library and the Rust `cdylib` are loaded at run
//! time with `libloading` and driven **only** through their exported symbols, so
//! the `#[no_mangle]`/`extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Repr(C) types mirroring the C declarations in c_src/src/lib.c
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
pub struct CnRnd {
    pub state: [u64; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;

// ---------------------------------------------------------------------------
// Function-pointer types
// ---------------------------------------------------------------------------

pub type FnC2V = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnBin2 = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnTri2 = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
pub type FnDot = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnCirCir = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
pub type FnCirAabb = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
pub type FnAabbAabb = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
pub type FnF2 = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
pub type FnF3 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnF4 = unsafe extern "C" fn(*mut CnRnd) -> f64;
pub type FnF5 = unsafe extern "C" fn(u32) -> u32;
pub type FnF7 = unsafe extern "C" fn(u32, u32, u32) -> u32;
pub type FnF9 = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
pub type FnF10 = unsafe extern "C" fn(u16) -> f32;
pub type FnColor = unsafe extern "C" fn(*mut f32, *const f32);
#[rustfmt::skip]
pub type FnAgglom = unsafe extern "C" fn(
    f32, f32, f32, f32, f32, f32, f32,
    c_int, c_int,
    u64, u64,
    u32,
    u32, u32, u32,
    f32, f32, f32, f32, f32, f32, f32, f32,
    u16,
    f32, f32, f32,
    f32, f32, f32,
    f32, f32, f32,
) -> f64;

/// All 33 `agglom` arguments, so both libraries can be fed the identical tuple.
#[derive(Copy, Clone, Debug)]
#[rustfmt::skip]
pub struct AgglomArgs {
    pub f2_1: f32, pub f2_2: f32, pub f2_3: f32,
    pub f2_7: f32, pub f2_8: f32, pub f2_9: f32, pub f2_10: f32,
    pub f3_1: c_int, pub f3_2: c_int,
    pub f4_1: u64, pub f4_2: u64,
    pub f5_1: u32,
    pub f7_1: u32, pub f7_2: u32, pub f7_3: u32,
    pub f9_1: f32, pub f9_2: f32, pub f9_4: f32, pub f9_5: f32,
    pub f9_7: f32, pub f9_8: f32, pub f9_10: f32, pub f9_11: f32,
    pub f10_1: u16,
    pub f11_2: f32, pub f11_3: f32, pub f11_4: f32,
    pub f12_2: f32, pub f12_3: f32, pub f12_4: f32,
    pub f13_2: f32, pub f13_3: f32, pub f13_4: f32,
}

// ---------------------------------------------------------------------------
// Library wrapper
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub c2V: FnC2V,
    pub c2Maxv: FnBin2,
    pub c2Minv: FnBin2,
    pub c2Clampv: FnTri2,
    pub c2Sub: FnBin2,
    pub c2Dot: FnDot,
    pub c2CircletoCircle: FnCirCir,
    pub c2CircletoAABB: FnCirAabb,
    pub c2AABBtoAABB: FnAabbAabb,
    pub f2: FnF2,
    pub f3: FnF3,
    pub f4: FnF4,
    pub f5: FnF5,
    pub f7: FnF7,
    pub f9: FnF9,
    pub f10: FnF10,
    pub f11: FnColor,
    pub f12: FnColor,
    pub f13: FnColor,
    pub agglom: FnAgglom,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol `{}`: {e}", $name));
        *s
    }};
}

impl Lib {
    pub fn open(name: &'static str, path: &Path) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
        let out = Lib {
            name,
            path: path.to_path_buf(),
            c2V: sym!(lib, "c2V", FnC2V),
            c2Maxv: sym!(lib, "c2Maxv", FnBin2),
            c2Minv: sym!(lib, "c2Minv", FnBin2),
            c2Clampv: sym!(lib, "c2Clampv", FnTri2),
            c2Sub: sym!(lib, "c2Sub", FnBin2),
            c2Dot: sym!(lib, "c2Dot", FnDot),
            c2CircletoCircle: sym!(lib, "c2CircletoCircle", FnCirCir),
            c2CircletoAABB: sym!(lib, "c2CircletoAABB", FnCirAabb),
            c2AABBtoAABB: sym!(lib, "c2AABBtoAABB", FnAabbAabb),
            f2: sym!(lib, "f2", FnF2),
            f3: sym!(lib, "f3", FnF3),
            f4: sym!(lib, "f4", FnF4),
            f5: sym!(lib, "f5", FnF5),
            f7: sym!(lib, "f7", FnF7),
            f9: sym!(lib, "f9", FnF9),
            f10: sym!(lib, "f10", FnF10),
            f11: sym!(lib, "f11", FnColor),
            f12: sym!(lib, "f12", FnColor),
            f13: sym!(lib, "f13", FnColor),
            agglom: sym!(lib, "agglom", FnAgglom),
            _lib: lib,
        };
        out
    }

    /// Convenience: run one of the three `void (*)(float*, const float*)`
    /// colour conversions on a 3-element input and return the 3 outputs.
    pub fn color(&self, which: Which, src: [f32; 3]) -> [f32; 3] {
        let f = match which {
            Which::F11 => self.f11,
            Which::F12 => self.f12,
            Which::F13 => self.f13,
        };
        let mut dest = [0.0f32; 3];
        unsafe { f(dest.as_mut_ptr(), src.as_ptr()) };
        dest
    }

    /// In-place variant: `dest == src` (aliasing).
    pub fn color_inplace(&self, which: Which, src: [f32; 3]) -> [f32; 3] {
        let f = match which {
            Which::F11 => self.f11,
            Which::F12 => self.f12,
            Which::F13 => self.f13,
        };
        let mut buf = src;
        unsafe { f(buf.as_mut_ptr(), buf.as_ptr()) };
        buf
    }

    #[rustfmt::skip]
    pub fn call_agglom(&self, a: &AgglomArgs) -> f64 {
        unsafe {
            (self.agglom)(
                a.f2_1, a.f2_2, a.f2_3, a.f2_7, a.f2_8, a.f2_9, a.f2_10,
                a.f3_1, a.f3_2,
                a.f4_1, a.f4_2,
                a.f5_1,
                a.f7_1, a.f7_2, a.f7_3,
                a.f9_1, a.f9_2, a.f9_4, a.f9_5, a.f9_7, a.f9_8, a.f9_10, a.f9_11,
                a.f10_1,
                a.f11_2, a.f11_3, a.f11_4,
                a.f12_2, a.f12_3, a.f12_4,
                a.f13_2, a.f13_3, a.f13_4,
            )
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Which {
    F11,
    F12,
    F13,
}

impl Which {
    pub fn name(self) -> &'static str {
        match self {
            Which::F11 => "f11",
            Which::F12 => "f12",
            Which::F13 => "f13",
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two .so files
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test executable
/// (`target/<profile>/deps/<test>`), so it is correct for every profile.
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not built: {}\n\
         build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    // Guard against comparing against a stale C build.
    let src = manifest_dir().join("c_src/src/lib.c");
    if let (Some(so_t), Some(src_t)) = (mtime(&p), mtime(&src)) {
        assert!(
            so_t >= src_t,
            "STALE C shared library: {} is older than {}. Rebuild the C .so.",
            p.display(),
            src.display()
        );
    }
    p
}

/// The Rust `cdylib`.
///
/// `cargo build` uplifts it to `target/<profile>/`, while `cargo test` leaves it
/// in `target/<profile>/deps/`. Both locations are searched and the NEWEST
/// existing artifact wins, so the suite can never silently test a stale `.so`.
pub fn rust_so_path() -> PathBuf {
    let dir = target_profile_dir();
    let candidates = [dir.join("libagglom_lib.so"), dir.join("deps/libagglom_lib.so")];
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for c in candidates.iter() {
        if let Some(t) = mtime(c) {
            if best.as_ref().map(|(_, bt)| t > *bt).unwrap_or(true) {
                best = Some((c.clone(), t));
            }
        }
    }
    let (path, so_t) = best.unwrap_or_else(|| {
        panic!(
            "Rust cdylib not built; looked for {:?}. Run `cargo build` first.",
            candidates
        )
    });

    // Staleness guard: the `.so` must be at least as new as every Rust source
    // file it is built from. Without this, a `cargo test` invocation that fails
    // to rebuild the cdylib would silently compare against an old library.
    for src in ["src/lib.rs", "src/tables.rs"] {
        let sp = manifest_dir().join(src);
        if let Some(src_t) = mtime(&sp) {
            assert!(
                so_t >= src_t,
                "STALE Rust cdylib: {} is older than {}.\n\
                 Rebuild with `cargo build` (or `cargo build --release`) before testing.",
                path.display(),
                sp.display()
            );
        }
    }
    path
}

/// Both libraries, freshly `dlopen`ed.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

pub fn both() -> Pair {
    Pair {
        c: Lib::open("C", &c_so_path()),
        r: Lib::open("Rust", &rust_so_path()),
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn f32b(v: f32) -> u32 {
    v.to_bits()
}
pub fn f64b(v: f64) -> u64 {
    v.to_bits()
}

#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{ctx}: C = {c:?} (0x{:08x}) vs Rust = {r:?} (0x{:08x})",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_f64(ctx: &str, c: f64, r: f64) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{ctx}: C = {c:?} (0x{:016x}) vs Rust = {r:?} (0x{:016x})",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_i32(ctx: &str, c: c_int, r: c_int) {
    assert_eq!(c, r, "{ctx}: C = {c} vs Rust = {r}");
}

#[track_caller]
pub fn eq_u32(ctx: &str, c: u32, r: u32) {
    assert_eq!(c, r, "{ctx}: C = {c} (0x{c:08x}) vs Rust = {r} (0x{r:08x})");
}

#[track_caller]
pub fn eq_v2(ctx: &str, c: C2v, r: C2v) {
    eq_f32(&format!("{ctx}.x"), c.x, r.x);
    eq_f32(&format!("{ctx}.y"), c.y, r.y);
}

#[track_caller]
pub fn eq_lm(ctx: &str, c: LmVec2, r: LmVec2) {
    eq_f32(&format!("{ctx}.x"), c.x, r.x);
    eq_f32(&format!("{ctx}.y"), c.y, r.y);
}

#[track_caller]
pub fn eq_arr3(ctx: &str, c: [f32; 3], r: [f32; 3]) {
    for i in 0..3 {
        eq_f32(&format!("{ctx}[{i}]"), c[i], r[i]);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seeds ⇒ reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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

    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        ((self.next_u32() >> 8) as f32) / ((1u32 << 24) as f32)
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    /// A "reasonable" finite float: mostly small magnitudes, sometimes large.
    pub fn finite(&mut self) -> f32 {
        match self.below(6) {
            0 => self.range(-1.0, 1.0),
            1 => self.range(-100.0, 100.0),
            2 => self.range(-1.0e6, 1.0e6),
            3 => self.range(-1.0e30, 1.0e30),
            4 => self.range(-1.0e-30, 1.0e-30),
            _ => self.range(-10.0, 10.0),
        }
    }

    /// Any `f32` bit pattern, biased towards interesting classes: `±0`,
    /// `±inf`, subnormals, quiet/signalling `NaN`s with *distinct payloads*,
    /// and plain finite values.
    pub fn wild_f32(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            // quiet NaN, random payload & sign
            4 | 5 => {
                let sign = (self.next_u32() & 1) << 31;
                let payload = self.next_u32() & 0x003f_ffff;
                f32::from_bits(sign | 0x7fc0_0000 | payload)
            }
            // signalling NaN (payload must be non-zero)
            6 => {
                let sign = (self.next_u32() & 1) << 31;
                let payload = (self.next_u32() & 0x003f_ffff) | 1;
                f32::from_bits(sign | 0x7f80_0000 | payload)
            }
            // subnormal
            7 => {
                let sign = (self.next_u32() & 1) << 31;
                f32::from_bits(sign | (self.next_u32() & 0x007f_ffff))
            }
            // completely random bits
            8 | 9 => f32::from_bits(self.next_u32()),
            _ => self.finite(),
        }
    }

    /// A quiet NaN with a distinct, non-zero payload (used to pin down which
    /// operand's NaN survives an SSE op).
    pub fn nan_payload(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let payload = (self.next_u32() & 0x003f_ffff) | 1;
        f32::from_bits(sign | 0x7fc0_0000 | payload)
    }

    pub fn wild_v2(&mut self) -> C2v {
        C2v {
            x: self.wild_f32(),
            y: self.wild_f32(),
        }
    }

    pub fn wild_lm(&mut self) -> LmVec2 {
        LmVec2 {
            x: self.wild_f32(),
            y: self.wild_f32(),
        }
    }

    pub fn finite_v2(&mut self) -> C2v {
        C2v {
            x: self.finite(),
            y: self.finite(),
        }
    }

    pub fn wild_circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.wild_v2(),
            r: self.wild_f32(),
        }
    }

    pub fn wild_aabb(&mut self) -> C2Aabb {
        C2Aabb {
            min: self.wild_v2(),
            max: self.wild_v2(),
        }
    }

    pub fn finite_circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.finite_v2(),
            r: self.finite(),
        }
    }

    pub fn finite_aabb(&mut self) -> C2Aabb {
        // Deliberately *not* normalised: inverted boxes are valid inputs.
        C2Aabb {
            min: self.finite_v2(),
            max: self.finite_v2(),
        }
    }

    #[rustfmt::skip]
    pub fn wild_agglom(&mut self) -> AgglomArgs {
        AgglomArgs {
            f2_1: self.wild_f32(), f2_2: self.wild_f32(), f2_3: self.wild_f32(),
            f2_7: self.wild_f32(), f2_8: self.wild_f32(),
            f2_9: self.wild_f32(), f2_10: self.wild_f32(),
            f3_1: self.next_i32(), f3_2: self.next_i32(),
            f4_1: self.next_u64(), f4_2: self.next_u64(),
            f5_1: self.next_u32(),
            f7_1: self.next_u32(), f7_2: self.next_u32(), f7_3: self.next_u32(),
            f9_1: self.wild_f32(), f9_2: self.wild_f32(),
            f9_4: self.wild_f32(), f9_5: self.wild_f32(),
            f9_7: self.wild_f32(), f9_8: self.wild_f32(),
            f9_10: self.wild_f32(), f9_11: self.wild_f32(),
            f10_1: self.next_u16(),
            f11_2: self.wild_f32(), f11_3: self.wild_f32(), f11_4: self.wild_f32(),
            f12_2: self.wild_f32(), f12_3: self.wild_f32(), f12_4: self.wild_f32(),
            f13_2: self.wild_f32(), f13_3: self.wild_f32(), f13_4: self.wild_f32(),
        }
    }

    #[rustfmt::skip]
    pub fn sane_agglom(&mut self) -> AgglomArgs {
        AgglomArgs {
            f2_1: self.range(-10.0, 10.0), f2_2: self.range(-10.0, 10.0),
            f2_3: self.range(0.0, 5.0),
            f2_7: self.range(-10.0, 0.0), f2_8: self.range(-10.0, 0.0),
            f2_9: self.range(0.0, 10.0), f2_10: self.range(0.0, 10.0),
            f3_1: (self.next_i32() % 100_000), f3_2: 1 + (self.next_u32() % 999) as i32,
            f4_1: self.next_u64() | 1, f4_2: self.next_u64() | 1,
            f5_1: self.next_u32(),
            f7_1: 1 + self.next_u32() % 65_536,
            f7_2: 1 + self.next_u32() % 9,
            f7_3: [8u32, 12, 16, 20, 24, 32][self.below(6) as usize],
            f9_1: self.range(-4.0, 4.0), f9_2: self.range(-4.0, 4.0),
            f9_4: self.range(-4.0, 4.0), f9_5: self.range(-4.0, 4.0),
            f9_7: self.range(-4.0, 4.0), f9_8: self.range(-4.0, 4.0),
            f9_10: self.range(-4.0, 4.0), f9_11: self.range(-4.0, 4.0),
            f10_1: self.next_u16(),
            f11_2: self.range(0.0, 360.0), f11_3: self.unit(), f11_4: self.unit(),
            f12_2: self.range(0.0, 360.0), f12_3: self.unit(), f12_4: self.unit(),
            f13_2: self.unit(), f13_3: self.unit(), f13_4: self.unit(),
        }
    }
}

/// Interesting scalar `f32` values used for exhaustive small cross-products.
pub const SPECIAL_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1.0e-45, // smallest subnormal
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

/// Two quiet NaNs with clearly distinct payloads (and one negative NaN).
pub const NAN_A: f32 = f32::from_bits(0x7fc0_0001);
pub const NAN_B: f32 = f32::from_bits(0x7fd2_3456);
pub const NAN_C: f32 = f32::from_bits(0xffe0_0abc);
/// Signalling NaN.
pub const SNAN: f32 = f32::from_bits(0x7f80_0001);
