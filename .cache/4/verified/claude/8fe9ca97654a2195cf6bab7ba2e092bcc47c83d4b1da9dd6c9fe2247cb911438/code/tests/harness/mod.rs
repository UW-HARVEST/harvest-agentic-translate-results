//! Differential-test harness.
//!
//! Loads THREE shared objects and calls everything through `dlsym`, so the
//! `#[no_mangle] extern "C"` export wrappers are exercised exactly like an
//! external consumer would exercise them:
//!
//! * `cbuild/libcdriver.so` -- the C library (`c_src/src/q_math.c` +
//!   `c_src/src/main.c`, the translation units from `c_src/CMakeLists.txt`).
//! * `cbuild/libcwrap.so`  -- `c_src/src/q_math.c` + `tests/csupport/wrappers.c`,
//!   which adds `w_*` entry points for the macros and `static ID_INLINE`
//!   functions of `c_src/inc/q_shared.h`.
//! * `target/<profile>/libdriver.so` -- the Rust translation, as a cdylib.
//!
//! Run `./build_c.sh && cargo build` before `cargo test`.
//!
//! Both libraries are opened with `RTLD_LOCAL` (libloading's default), so the C
//! library's internal PLT calls (e.g. `PlaneFromPoints` -> `VectorNormalize`)
//! can never be interposed by the Rust library's identically named exports, and
//! vice versa.

#![allow(dead_code)]

use libloading::Library;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Libs {
    /// the C shared object (symbol-parity reference)
    pub c: Library,
    /// the C shared object + `w_*` wrappers around q_shared.h
    pub cw: Library,
    /// the Rust cdylib
    pub r: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/libdriver.so`, derived from the test binary's own location
/// (`target/<profile>/deps/<test>-<hash>`).
fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cdriver = manifest_dir().join("cbuild/libcdriver.so");
        let cwrap = manifest_dir().join("cbuild/libcwrap.so");
        let rust = rust_so();
        for p in [&cdriver, &cwrap, &rust] {
            assert!(
                p.exists(),
                "missing {}: run ./build_c.sh && cargo build first",
                p.display()
            );
        }
        // `cargo test` builds the rlib but NOT the cdylib, so a source change
        // that has not been followed by `cargo build` would be tested against a
        // stale `.so` and could "pass" without ever running the new code.
        assert_not_stale(&rust, &["src"], "cargo build");
        assert_not_stale(&cdriver, &["c_src/src", "c_src/inc"], "./build_c.sh");
        assert_not_stale(
            &cwrap,
            &["c_src/src", "c_src/inc", "tests/csupport"],
            "./build_c.sh",
        );
        unsafe {
            Libs {
                c: Library::new(&cdriver).expect("dlopen libcdriver.so"),
                cw: Library::new(&cwrap).expect("dlopen libcwrap.so"),
                r: Library::new(&rust).expect("dlopen libdriver.so"),
            }
        }
    })
}

/// Panics when `artifact` is older than any source file under `dirs`.
fn assert_not_stale(artifact: &std::path::Path, dirs: &[&str], rebuild_with: &str) {
    let built = artifact
        .metadata()
        .and_then(|m| m.modified())
        .expect("artifact mtime");
    for dir in dirs {
        let d = manifest_dir().join(dir);
        let entries = match std::fs::read_dir(&d) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Ok(src) = entry.metadata().and_then(|m| m.modified()) {
                assert!(
                    src <= built,
                    "{} is older than {}: run `{rebuild_with}` first (a stale \
                     shared object would make this suite test the wrong code)",
                    artifact.display(),
                    path.display()
                );
            }
        }
    }
}

/// The C driver executable and the Rust driver executable.
pub fn drivers() -> (PathBuf, PathBuf) {
    let c = manifest_dir().join("cbuild/cdriver");
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().and_then(|p| p.parent()).unwrap().to_path_buf();
    let r = profile_dir.join("driver");
    (c, r)
}

fn lookup<T: Copy>(lib: &Library, name: &str) -> T {
    let mut key: Vec<u8> = name.as_bytes().to_vec();
    key.push(0);
    unsafe {
        *lib.get::<T>(&key)
            .unwrap_or_else(|e| panic!("dlsym({name}): {e}"))
    }
}

/// A function pointer from the C library.
pub fn cfn<F: Copy>(name: &str) -> F {
    lookup(&libs().c, name)
}

/// A function pointer from the Rust library.
pub fn rfn<F: Copy>(name: &str) -> F {
    lookup(&libs().r, name)
}

/// A `w_*` function pointer from the C wrapper library.
pub fn cwfn<F: Copy>(name: &str) -> F {
    lookup(&libs().cw, name)
}

/// The address of a data symbol in the C library.
pub fn cdata<T>(name: &str) -> *mut T {
    lookup::<*mut T>(&libs().c, name)
}

/// The address of a data symbol in the Rust library.
pub fn rdata<T>(name: &str) -> *mut T {
    lookup::<*mut T>(&libs().r, name)
}

/// Both implementations of the same symbol, C first.
pub fn both<F: Copy>(name: &str) -> (F, F) {
    (cfn(name), rfn(name))
}

/// Both implementations of a `w_*` wrapper, C first.
pub fn both_w<F: Copy>(name: &str) -> (F, F) {
    (cwfn(name), rfn(name))
}

// ---------------------------------------------------------------------------
// bit-exact comparisons
// ---------------------------------------------------------------------------

pub fn f32_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn f64_eq(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

#[track_caller]
pub fn assert_f32(ctx: &str, c: f32, r: f32) {
    assert!(
        f32_eq(c, r),
        "{ctx}: C returned {c:?} (0x{:08x}) but Rust returned {r:?} (0x{:08x})",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn assert_f64(ctx: &str, c: f64, r: f64) {
    assert!(
        f64_eq(c, r),
        "{ctx}: C returned {c:?} (0x{:016x}) but Rust returned {r:?} (0x{:016x})",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn assert_vec(ctx: &str, c: &[f32], r: &[f32]) {
    assert_eq!(c.len(), r.len());
    for i in 0..c.len() {
        assert!(
            f32_eq(c[i], r[i]),
            "{ctx}: element {i} differs: C {:?} (0x{:08x}) vs Rust {:?} (0x{:08x})\n  C   = {:?}\n  Rust= {:?}",
            c[i],
            c[i].to_bits(),
            r[i],
            r[i].to_bits(),
            c,
            r
        );
    }
}

#[track_caller]
pub fn assert_int<T: PartialEq + std::fmt::Debug>(ctx: &str, c: T, r: T) {
    assert_eq!(c, r, "{ctx}: C vs Rust");
}

// ---------------------------------------------------------------------------
// NaN payload ambiguity
// ---------------------------------------------------------------------------
//
// IEEE 754 and ISO C both leave it open which NaN a binary operation returns
// when *both* operands are NaN.  On x86-64 `addss`/`mulss` propagate the first
// source operand, and gcc -O0 does not choose the source operands in a
// consistent order (in `_DotProduct` the first product ends up as
// `mulss %xmm0,%xmm1` -- left operand first -- and the second as
// `mulss %xmm2,%xmm0` -- right operand first).  Reproducing that would mean
// encoding gcc's register allocation into the translation, so this suite
// compares NaN *payloads* only where the C result cannot be ambiguous.
//
// Two different NaN patterns can meet in two ways, both of which need a
// non-finite (or overflowing) input: an invalid operation manufactures the x86
// "default" NaN 0xffc00000 (`inf - inf`, `0 * inf`, `sin(inf)`, ...), and a
// negation of a NaN (`-forward[0]`, `-d`, `-sp`, ...) flips its sign bit into
// another distinct pattern.  See NOTES.md.

/// `|x| > 1.8e19` squares to +inf in `f32`.
const OVERFLOWS_WHEN_SQUARED: f32 = 1.8e19;

/// Set `DIFF_STRICT_NAN=1` to compare NaN payloads too, i.e. to disable the one
/// documented tolerance of this suite.  Used to measure exactly which inputs
/// still depend on gcc's operand ordering; see NOTES.md.
pub fn strict_nan() -> bool {
    static STRICT: OnceLock<bool> = OnceLock::new();
    *STRICT.get_or_init(|| std::env::var_os("DIFF_STRICT_NAN").is_some())
}

/// Bit-exact, except that two NaNs with different payloads compare equal.
#[track_caller]
pub fn assert_f32_soft(ctx: &str, c: f32, r: f32) {
    if c.is_nan() && r.is_nan() {
        return;
    }
    assert_f32(ctx, c, r);
}

/// True when a *second* NaN bit pattern can appear in the computation, so that
/// the surviving payload is decided by gcc's operand ordering:
///
/// * a non-finite or overflowing input can manufacture the x86 default NaN
///   `0xffc00000` (`inf - inf`, `0 * inf`, `sin(inf)`, ...);
/// * a NaN input can be negated (`-forward[0]`, `-d`, `-sp`, `-1*cr*-sy`, ...),
///   which flips its sign bit and produces a second, different pattern.
///
/// Only finite, non-overflowing inputs can therefore be guaranteed to keep NaN
/// payloads reproducible -- and for those no NaN exists in the first place.
pub fn nan_payload_ambiguous(vals: &[f32]) -> bool {
    vals.iter()
        .any(|v| !v.is_finite() || v.abs() > OVERFLOWS_WHEN_SQUARED)
}

/// Bit-exact comparison of a result computed from `inputs`.
///
/// The ONLY tolerated difference is the payload/sign of a NaN when both sides
/// produced a NaN *and* the inputs allow two different NaN patterns to meet in
/// one expression (see [`nan_payload_ambiguous`] and NOTES.md "Deviation 1").
/// Everything else -- the NaN-ness itself, every finite result, every sign of
/// zero, every infinity -- is compared bit for bit.
#[track_caller]
pub fn check_f32(ctx: &str, inputs: &[f32], c: f32, r: f32) {
    if c.is_nan() && r.is_nan() && nan_payload_ambiguous(inputs) && !strict_nan() {
        return;
    }
    assert_f32(ctx, c, r);
}

/// Vector flavour of [`check_f32`].
#[track_caller]
pub fn check_vec(ctx: &str, inputs: &[f32], c: &[f32], r: &[f32]) {
    assert_eq!(c.len(), r.len());
    let ambiguous = nan_payload_ambiguous(inputs) && !strict_nan();
    for i in 0..c.len() {
        if ambiguous && c[i].is_nan() && r[i].is_nan() {
            continue;
        }
        assert!(
            f32_eq(c[i], r[i]),
            "{ctx}: element {i} differs: C {:?} (0x{:08x}) vs Rust {:?} (0x{:08x})\n  C   = {:?}\n  Rust= {:?}",
            c[i],
            c[i].to_bits(),
            r[i],
            r[i].to_bits(),
            c,
            r
        );
    }
}

// ---------------------------------------------------------------------------
// non-reentrant C functions
// ---------------------------------------------------------------------------

/// `AngleVectors` keeps its six intermediate sines and cosines in
/// **`static float`** variables (q_math.c:946: "static to help MS compiler fp
/// bugs"), so the C implementation is not reentrant: two threads calling it --
/// directly or through `AnglesToAxis` -- overwrite each other's `sy`/`cy`/... and
/// then read the other thread's values.  `cargo test` runs the tests of one
/// binary on parallel threads, so every test that reaches `AngleVectors` must
/// hold this lock to make the C side well defined.
///
/// (The six statics are always assigned before they are read, so for any single
/// call they behave exactly like locals, which is what the Rust translation
/// uses -- see NOTES.md.)
pub fn angle_vectors_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// deterministic randomness
// ---------------------------------------------------------------------------

/// xorshift64* -- small, fast, fully reproducible from a fixed seed.
pub struct Rng(u64);

/// Values that are interesting to every routine in the library: the identity
/// elements, both zeroes, both infinities, a canonical NaN, denormals, the
/// extremes of the exponent range, and the angle constants the code compares
/// against.
///
/// Only ONE NaN bit pattern appears here on purpose: when two NaNs with
/// different payloads meet in a single expression, which payload survives is
/// decided by the operand order the compiler happened to pick for `addss` /
/// `mulss`, and gcc -O0 does not even pick it consistently within one
/// expression (see `NOTES.md`).  Single-NaN propagation, which is what real
/// callers hit, is order independent and is covered here and by the dedicated
/// NaN-payload tests.
pub const INTERESTING: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    255.0,
    1.0 / 255.0,
    90.0,
    -90.0,
    180.0,
    -180.0,
    270.0,
    360.0,
    -360.0,
    99999.0,
    -99999.0,
    65536.0,
    32768.0,
    0.999_999_94,
    1.000_000_1,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-45, // smallest denormal
    -1e-45,
    1e-30,
    1e30,
    -1e30,
    f32::MAX,
    -f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// uniform in `0..n`
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 != 0
    }

    /// A float from [`INTERESTING`], or a random bit pattern with a random
    /// exponent (so denormals, huge and tiny values all show up), never a
    /// non-canonical NaN.
    pub fn f32_any(&mut self) -> f32 {
        match self.below(3) {
            0 => INTERESTING[self.below(INTERESTING.len() as u32) as usize],
            1 => {
                let bits = self.next_u32();
                let v = f32::from_bits(bits);
                if v.is_nan() {
                    f32::NAN
                } else {
                    v
                }
            }
            _ => self.f32_finite(),
        }
    }

    /// A finite float with a random sign, a random exponent in a "sane" range
    /// and a random mantissa.
    pub fn f32_finite(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        // biased exponent 1..=253 -> no infinities, no NaN
        let exp = 1 + self.below(253);
        let mant = self.next_u32() & 0x7f_ffff;
        f32::from_bits(sign | (exp << 23) | mant)
    }

    /// A finite float in `[-mag, mag]`, uniform in the mantissa.
    pub fn f32_mag(&mut self, mag: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        let v = mag * (2.0 * u - 1.0);
        if self.below(16) == 0 {
            // hit the exact boundaries now and then
            if self.bool() {
                mag
            } else {
                -mag
            }
        } else {
            v
        }
    }

    /// A float in `[-1, 1]` (typical normalized-vector component).
    pub fn f32_unit(&mut self) -> f32 {
        self.f32_mag(1.0)
    }

    pub fn vec3_any(&mut self) -> [f32; 3] {
        [self.f32_any(), self.f32_any(), self.f32_any()]
    }

    pub fn vec3_finite(&mut self) -> [f32; 3] {
        [self.f32_finite(), self.f32_finite(), self.f32_finite()]
    }

    pub fn vec3_mag(&mut self, mag: f32) -> [f32; 3] {
        [self.f32_mag(mag), self.f32_mag(mag), self.f32_mag(mag)]
    }

    pub fn vec4_any(&mut self) -> [f32; 4] {
        [
            self.f32_any(),
            self.f32_any(),
            self.f32_any(),
            self.f32_any(),
        ]
    }

    /// A vec3 that contains AT MOST one NaN (see [`INTERESTING`]).
    pub fn vec3_any_1nan(&mut self) -> [f32; 3] {
        let mut v = self.vec3_any();
        let mut seen = false;
        for x in v.iter_mut() {
            if x.is_nan() {
                if seen {
                    *x = 1.0;
                } else {
                    seen = true;
                }
            }
        }
        v
    }

    /// A "unit-ish" direction vector, occasionally exactly axial or degenerate.
    pub fn dir(&mut self) -> [f32; 3] {
        match self.below(8) {
            0 => [1.0, 0.0, 0.0],
            1 => [0.0, 1.0, 0.0],
            2 => [0.0, 0.0, 1.0],
            3 => [0.0, 0.0, 0.0],
            4 => [-1.0, 0.0, 0.0],
            _ => {
                let v = [self.f32_unit(), self.f32_unit(), self.f32_unit()];
                v
            }
        }
    }

    pub fn i32_any(&mut self) -> i32 {
        match self.below(4) {
            0 => {
                const POOL: &[i32] = &[
                    0,
                    1,
                    -1,
                    2,
                    -2,
                    3,
                    7,
                    8,
                    127,
                    128,
                    -128,
                    -129,
                    255,
                    256,
                    32767,
                    32768,
                    -32768,
                    -32769,
                    65535,
                    65536,
                    i32::MAX,
                    i32::MIN,
                    i32::MAX - 1,
                    i32::MIN + 1,
                ];
                POOL[self.below(POOL.len() as u32) as usize]
            }
            1 => self.next_u32() as i32,
            2 => (self.next_u32() % 512) as i32 - 256,
            _ => (self.next_u32() % 65536) as i32,
        }
    }
}
