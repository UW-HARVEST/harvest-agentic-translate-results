//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls `tfm` through the FFI
//! boundary in each. The Rust implementation is never called directly — it is
//! resolved from `libtfm_lib.so` by symbol name, exactly as an external C
//! consumer would, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

/// The C ABI of the one public entry point.
pub type TfmFn = unsafe extern "C" fn(*mut f32, *const f32, i32);

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_tfm: TfmFn,
    pub rust_tfm: TfmFn,
}

// SAFETY: both libraries are leaked for the process lifetime and `tfm` is a
// pure, stateless function, so the pointers stay valid and are safe to share.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let path = e.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Explicit override, so the same test suite can be pointed at the debug and
    // the release cdylib (they are different codegen and must both match C).
    if let Ok(p) = std::env::var("TFM_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "TFM_RUST_SO points at a missing file: {}", p.display());
        return p;
    }
    // The test binary lives in target/<profile>/deps/, so walk up to the
    // profile dir and look for the cdylib there.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().unwrap().to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let direct = dir.join("libtfm_lib.so");
    if direct.exists() {
        return direct;
    }
    for prof in ["debug", "release"] {
        let p = repo_root()
            .join("translation")
            .join("target")
            .join(prof)
            .join("libtfm_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libtfm_lib.so not found near {}. Build it with `cargo build` / `cargo build --release`.",
        dir.display()
    )
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));

            let c_sym: Symbol<TfmFn> = c
                .get(b"tfm\0")
                .unwrap_or_else(|e| panic!("symbol `tfm` missing from C .so: {e}"));
            let r_sym: Symbol<TfmFn> = rust
                .get(b"tfm\0")
                .unwrap_or_else(|e| panic!("symbol `tfm` missing from Rust .so: {e}"));

            let c_tfm = *c_sym;
            let rust_tfm = *r_sym;
            Libs {
                _c: c,
                _rust: rust,
                c_tfm,
                rust_tfm,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Byte-for-byte float comparison: distinguishes `+0.0`/`-0.0` and compares
/// NaN sign bits and payloads.
pub fn bits_eq(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

pub fn show(v: &[f32]) -> String {
    let parts: Vec<String> = v
        .iter()
        .map(|x| format!("{:#010x}({})", x.to_bits(), x))
        .collect();
    format!("[{}]", parts.join(", "))
}

/// Sentinel that must never appear as a legitimate output, used to detect
/// untouched destination slots.
pub const FILL: f32 = -123.456e7;

/// Run both implementations on the same input with freshly-filled destination
/// buffers and assert the results are bit-identical.
///
/// Returns the (identical) C output.
#[track_caller]
pub fn diff_call(src: &[f32], count: i32, ctx: &str) -> Vec<f32> {
    let out_len = if count > 0 { 2 * count as usize } else { 0 };
    // Extra guard slots after the live region to catch over-writes.
    let guards = 4usize;
    let mut c_dest = vec![FILL; out_len + guards];
    let mut r_dest = vec![FILL; out_len + guards];

    let l = libs();
    unsafe {
        (l.c_tfm)(c_dest.as_mut_ptr(), src.as_ptr(), count);
        (l.rust_tfm)(r_dest.as_mut_ptr(), src.as_ptr(), count);
    }

    assert!(
        bits_eq(&c_dest, &r_dest),
        "MISMATCH [{ctx}]\n  count = {count}\n  src   = {}\n  C     = {}\n  Rust  = {}",
        show(src),
        show(&c_dest),
        show(&r_dest),
    );

    // No writes past 2*count.
    for (i, g) in c_dest[out_len..].iter().enumerate() {
        assert_eq!(
            g.to_bits(),
            FILL.to_bits(),
            "[{ctx}] C wrote past 2*count at guard {i}"
        );
    }
    for (i, g) in r_dest[out_len..].iter().enumerate() {
        assert_eq!(
            g.to_bits(),
            FILL.to_bits(),
            "[{ctx}] Rust wrote past 2*count at guard {i}"
        );
    }

    c_dest.truncate(out_len);
    c_dest
}

/// Differential call on a single triple (`count == 1`).
#[track_caller]
pub fn diff_one(a: f32, b: f32, c: f32, ctx: &str) -> Vec<f32> {
    diff_call(&[a, b, c], 1, ctx)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed → reproducible property tests)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid the zero fixed-point of xorshift.
        Rng(seed | 1)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, n)`.
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }

    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Random *finite normal* float with magnitude in `[lo, hi]` and random sign.
    pub fn normal_in(&mut self, lo: f32, hi: f32) -> f32 {
        let t = self.unit();
        // Log-uniform magnitude spread.
        let mag = (lo.ln() + t * (hi.ln() - lo.ln())).exp();
        let mag = if mag.is_finite() && mag != 0.0 { mag } else { lo };
        if self.next_u64() & 1 == 0 {
            mag
        } else {
            -mag
        }
    }

    /// Random moderate finite float, the workhorse for happy-path rows.
    pub fn finite(&mut self) -> f32 {
        self.normal_in(1e-6, 1e6)
    }

    /// Arbitrary 32-bit pattern reinterpreted as `f32` (any class, incl. sNaN
    /// and non-canonical NaN payloads).
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

/// The interesting-value set: every IEEE-754 class boundary the C can meet.
pub const SPECIALS: &[f32] = &[
    0.0,
    -0.0,
    f32::MIN_POSITIVE,               // smallest normal
    -f32::MIN_POSITIVE,
    1.0e-45,                         // subnormal (~FLT_TRUE_MIN)
    -1.0e-45,
    1.0,
    -1.0,
    2.0,
    -2.0,
    0.5,
    -0.5,
    f32::MAX,
    f32::MIN,                        // -FLT_MAX
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

/// NaN bit patterns with distinct payloads and both sign bits, plus sNaNs.
pub fn nan_patterns() -> Vec<f32> {
    vec![
        f32::from_bits(0x7FC0_0000), // canonical qNaN, +
        f32::from_bits(0xFFC0_0000), // x86 indefinite, -
        f32::from_bits(0x7FC0_1234), // qNaN, + , payload
        f32::from_bits(0xFFDE_AD01), // qNaN, - , payload
        f32::from_bits(0x7F80_0001), // sNaN, +
        f32::from_bits(0xFF80_0001), // sNaN, -
        f32::from_bits(0x7FFF_FFFF), // qNaN all payload bits
        f32::from_bits(0xFFBF_FFFF), // sNaN max payload, -
    ]
}
