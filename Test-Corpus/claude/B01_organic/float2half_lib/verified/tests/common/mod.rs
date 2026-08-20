//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes them behind the
//! same raw C function pointer type, so every comparison crosses a real FFI
//! boundary in both directions:
//!
//!   * C reference:      `c_src/build/libtranslated_rust.so`
//!   * Rust translation: `target/<dylib-profile>/libfloat2half_lib.so`
//!
//! The Rust side is **never** called directly as a Rust function — always
//! through the `.so` export, so the `#[no_mangle] extern "C"` wrapper and its
//! ABI are part of what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The one exported C ABI symbol (see `c_src/include/lib.h`):
/// `uint16_t float2half(float flt);`
pub type Float2HalfFn = unsafe extern "C" fn(f32) -> u16;

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn is_release() -> bool {
    // Integration-test executables live in target/<profile>/deps/<name>-<hash>.
    std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.parent()
                .and_then(Path::parent)
                .and_then(|d| d.file_name())
                .map(|n| n.to_string_lossy().into_owned())
        })
        .map(|profile_dir| profile_dir == "release")
        .unwrap_or(false)
}

/// Build the C reference shared library with CMake, exactly as documented.
/// `c_src/` itself is never modified — only the out-of-tree `c_src/build` dir.
fn build_c_so() -> PathBuf {
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    let so = build.join("libtranslated_rust.so");
    if so.is_file() {
        return so;
    }
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake configure");
    assert!(st.success(), "cmake configure failed");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake build");
    assert!(st.success(), "cmake build failed");
    assert!(so.is_file(), "expected C shared object at {}", so.display());
    so
}

/// Build the Rust `cdylib`. `cargo test` does not build a cdylib-only lib
/// target, so this nested `cargo build` produces it. It uses a *separate*
/// target directory so it cannot deadlock against the lock the outer
/// `cargo test` holds on `target/`.
fn build_rust_so() -> PathBuf {
    let root = manifest_dir();
    let release = is_release();
    let target_dir = root
        .join("target")
        .join(if release { "dylib-release" } else { "dylib-debug" });

    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(&root)
        .arg("build")
        .arg("--offline")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target_dir);
    if release {
        cmd.arg("--release");
    }
    // Don't let the parent test run's env leak a conflicting profile/target.
    cmd.env_remove("CARGO_BUILD_TARGET_DIR")
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTFLAGS");

    let st = cmd.status().expect("run cargo build --lib");
    assert!(st.success(), "cargo build --lib failed");

    let so = target_dir
        .join(if release { "release" } else { "debug" })
        .join("libfloat2half_lib.so");
    assert!(
        so.is_file(),
        "expected Rust shared object at {}",
        so.display()
    );
    so
}

/// Both libraries plus the resolved raw function pointers.
pub struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c_float2half: Float2HalfFn,
    pub rust_float2half: Float2HalfFn,
}

impl Libs {
    /// Result of the C implementation, called through `libloading`.
    #[inline(always)]
    pub fn c(&self, x: f32) -> u16 {
        unsafe { (self.c_float2half)(x) }
    }

    /// Result of the Rust implementation, called through `libloading`.
    #[inline(always)]
    pub fn rust(&self, x: f32) -> u16 {
        unsafe { (self.rust_float2half)(x) }
    }

    /// Compare for one input given as raw bits (so NaN payloads and signed
    /// zero are compared exactly, not by float equality).
    #[track_caller]
    pub fn assert_same_bits(&self, bits: u32, ctx: &str) {
        let x = f32::from_bits(bits);
        let c = self.c(x);
        let r = self.rust(x);
        assert_eq!(
            c, r,
            "MISMATCH [{ctx}] input bits=0x{bits:08X} (j={}, mant=0x{:06X}): \
             C=0x{c:04X} Rust=0x{r:04X}",
            (bits >> 23) & 0x1ff,
            bits & 0x007f_ffff,
        );
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Load (building if needed) both shared objects. Cached per test binary.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = build_c_so();
        let rust_path = build_rust_so();
        unsafe {
            let c = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
            let c_sym: libloading::Symbol<Float2HalfFn> = c
                .get(b"float2half\0")
                .expect("C .so must export float2half");
            let r_sym: libloading::Symbol<Float2HalfFn> = rust
                .get(b"float2half\0")
                .expect("Rust .so must export float2half");
            let c_float2half = *c_sym;
            let rust_float2half = *r_sym;
            drop(c_sym);
            drop(r_sym);
            Libs {
                _c: c,
                _rust: rust,
                c_float2half,
                rust_float2half,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed => reproducible property-style testing).
// splitmix64; no external crate needed.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

// ---------------------------------------------------------------------------
// Input-shape helpers, mirroring the C's two data axes:
//   j    = (bits >> 23) & 0x1ff      (sign bit + 8 exponent bits)
//   mant = bits & 0x007fffff
// ---------------------------------------------------------------------------

/// Reassemble a float's bits from the exact axes the C code reads.
#[inline]
pub fn bits_from(j: u32, mant: u32) -> u32 {
    debug_assert!(j < 512);
    debug_assert!(mant <= 0x007f_ffff);
    (j << 23) | mant
}

/// Boundary mantissa shapes: zero, min nonzero, MSB only, max-1, max.
pub const MANT_SHAPES: [u32; 8] = [
    0x00_0000, 0x00_0001, 0x00_0002, 0x40_0000, 0x3F_FFFF, 0x7F_FFFE, 0x7F_FFFF, 0x2A_AAAA,
];

/// The 86 maximal `(m__base, m__shift)` runs over `j`, as generated in
/// `CONFIGS.md`: `(row_number, j_lo, j_hi)`. Row N here == row N there.
pub const CONFIG_ROWS: [(u32, u32, u32); 86] = [
    (1, 0, 102),
    (2, 103, 103),
    (3, 104, 104),
    (4, 105, 105),
    (5, 106, 106),
    (6, 107, 107),
    (7, 108, 108),
    (8, 109, 109),
    (9, 110, 110),
    (10, 111, 111),
    (11, 112, 112),
    (12, 113, 113),
    (13, 114, 114),
    (14, 115, 115),
    (15, 116, 116),
    (16, 117, 117),
    (17, 118, 118),
    (18, 119, 119),
    (19, 120, 120),
    (20, 121, 121),
    (21, 122, 122),
    (22, 123, 123),
    (23, 124, 124),
    (24, 125, 125),
    (25, 126, 126),
    (26, 127, 127),
    (27, 128, 128),
    (28, 129, 129),
    (29, 130, 130),
    (30, 131, 131),
    (31, 132, 132),
    (32, 133, 133),
    (33, 134, 134),
    (34, 135, 135),
    (35, 136, 136),
    (36, 137, 137),
    (37, 138, 138),
    (38, 139, 139),
    (39, 140, 140),
    (40, 141, 141),
    (41, 142, 142),
    (42, 143, 254),
    (43, 255, 255),
    (44, 256, 358),
    (45, 359, 359),
    (46, 360, 360),
    (47, 361, 361),
    (48, 362, 362),
    (49, 363, 363),
    (50, 364, 364),
    (51, 365, 365),
    (52, 366, 366),
    (53, 367, 367),
    (54, 368, 368),
    (55, 369, 369),
    (56, 370, 370),
    (57, 371, 371),
    (58, 372, 372),
    (59, 373, 373),
    (60, 374, 374),
    (61, 375, 375),
    (62, 376, 376),
    (63, 377, 377),
    (64, 378, 378),
    (65, 379, 379),
    (66, 380, 380),
    (67, 381, 381),
    (68, 382, 382),
    (69, 383, 383),
    (70, 384, 384),
    (71, 385, 385),
    (72, 386, 386),
    (73, 387, 387),
    (74, 388, 388),
    (75, 389, 389),
    (76, 390, 390),
    (77, 391, 391),
    (78, 392, 392),
    (79, 393, 393),
    (80, 394, 394),
    (81, 395, 395),
    (82, 396, 396),
    (83, 397, 397),
    (84, 398, 398),
    (85, 399, 510),
    (86, 511, 511),
];
