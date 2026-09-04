//! Shared differential-test harness.
//!
//! Both libraries are loaded through `libloading` and called **only** through
//! their exported `extern "C"` symbols, so the `#[no_mangle]` wrappers, the ABI
//! and the argument/return marshalling are all under test. Nothing in the crate
//! under test is called directly.

#![allow(dead_code)]

use std::ffi::{c_int, OsStr};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::Library;

pub type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, c_int, f64) -> c_int;
pub type ScFn = unsafe extern "C" fn(*mut f32, *mut f32, c_int) -> f64;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub r#match: MatchFn,
    pub spectral_contrast: ScFn,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// The C shared object is named after the *parent directory* of `c_src`
/// (`cmake_path(GET parent FILENAME project_name)`), so it is discovered rather
/// than hard-coded.
fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}\nbuild the C library first", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension() == Some(OsStr::new("so")))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // Escape hatch used by `tests/nan_payload_search.rs` to A/B a deliberately
    // mutated build against the C oracle.
    if let Some(p) = std::env::var_os("RUST_SO_OVERRIDE") {
        return PathBuf::from(p);
    }
    let p = workspace_root().join("translation/target/release/libunderhanded_c_nuke_lib.so");
    assert!(
        p.exists(),
        "{} missing -- run `cargo build --release` first",
        p.display()
    );

    // `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` target, so it is
    // very easy to test a stale shared object. Fail loudly instead.
    let so_mtime = std::fs::metadata(&p).and_then(|m| m.modified()).unwrap();
    let src = workspace_root().join("translation/src");
    let mut newest = so_mtime;
    let mut newest_name = String::new();
    for e in std::fs::read_dir(&src).unwrap().filter_map(|e| e.ok()) {
        if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
            if m > newest {
                newest = m;
                newest_name = e.file_name().to_string_lossy().into_owned();
            }
        }
    }
    assert!(
        newest <= so_mtime,
        "{} is STALE: src/{newest_name} is newer.\n\
         `cargo test` does not rebuild a cdylib -- run `cargo build --release` first \
         (or use ./run_tests.sh).",
        p.display()
    );
    p
}

fn load(name: &'static str, path: PathBuf) -> Lib {
    // Leaked on purpose: the function pointers below must outlive the `Library`
    // borrow for the whole test binary.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
    }));
    let m: libloading::Symbol<MatchFn> = unsafe {
        lib.get(b"match\0")
            .unwrap_or_else(|e| panic!("{name}: no `match` symbol: {e}"))
    };
    let s: libloading::Symbol<ScFn> = unsafe {
        lib.get(b"spectral_contrast\0")
            .unwrap_or_else(|e| panic!("{name}: no `spectral_contrast` symbol: {e}"))
    };
    Lib {
        name,
        path,
        r#match: *m,
        spectral_contrast: *s,
    }
}

/// `(c, rust)`
pub fn libs() -> &'static (Lib, Lib) {
    static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();
    LIBS.get_or_init(|| {
        (
            load("C", find_c_so()),
            load("Rust", find_rust_so()),
        )
    })
}

// ---------------------------------------------------------------- RNG --------

/// SplitMix64 -- deterministic, seedable, no dependencies.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
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
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bool_(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

pub const SEED: u64 = 0x5DEE_CE66_D;

// ------------------------------------------------------------- fixtures ------

/// The threshold set used by every "T ∈ full set" row of `CONFIGS.md`.
pub const THRESHOLDS: &[f64] = &[
    f64::NEG_INFINITY,
    -1e300,
    -1.0,
    -0.5,
    -0.0,
    0.0,
    f64::MIN_POSITIVE,
    1e-9,
    0.25,
    0.5,
    0.75,
    0.999_999_999_999_999_9,
    1.0,
    1.000_000_000_000_000_2,
    2.0,
    1e300,
    f64::INFINITY,
];

/// NaN thresholds, kept separate so a row can note that they never reject.
pub fn nan_thresholds() -> Vec<f64> {
    vec![
        f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001),
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF4_0000_0000_0000), // signalling
        f64::from_bits(0xFFF0_0000_0000_0001), // signalling, negative
    ]
}

// -------------------------------------------------------- data generators ----

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Data {
    /// Finite `double`s in `[-1, 1]`.
    Finite,
    /// Finite non-negative `double`s in `[0, 1]` (a plausible spectrum).
    Positive,
    AllZeros,
    /// Every element the same random value.
    Constant,
    Ramp,
    /// One random index nonzero.
    Spike,
    /// `|x| ~ 1e300` -- `total` and `Σx²` overflow.
    Huge,
    /// Subnormal / near-zero `double`s.
    Tiny,
    /// Mixture of `+0.0` and `-0.0`.
    SignedZeros,
    /// Finite with a few `±inf` lanes.
    WithInf,
    /// Finite with a few quiet NaNs with random payloads.
    WithQNaN,
    /// Finite with a few signalling NaNs with random payloads.
    WithSNaN,
    /// Completely random bit patterns (every IEEE class).
    RandomBits,
    /// `double`s whose **low 32 bits** are a chosen `float` class.
    LowWordInf,
    LowWordNaN,
    LowWordSubnormal,
}

pub fn gen_f64(rng: &mut Rng, n: usize, d: Data) -> Vec<f64> {
    let mut v = vec![0.0f64; n];
    match d {
        Data::Finite => {
            for x in v.iter_mut() {
                *x = rng.range(-1.0, 1.0);
            }
        }
        Data::Positive => {
            for x in v.iter_mut() {
                *x = rng.unit();
            }
        }
        Data::AllZeros => {}
        Data::Constant => {
            let c = rng.range(-4.0, 4.0);
            for x in v.iter_mut() {
                *x = c;
            }
        }
        Data::Ramp => {
            let step = rng.range(-2.0, 2.0);
            let base = rng.range(-2.0, 2.0);
            for (i, x) in v.iter_mut().enumerate() {
                *x = base + step * i as f64;
            }
        }
        Data::Spike => {
            if n > 0 {
                let i = rng.below(n);
                v[i] = rng.range(-100.0, 100.0);
            }
        }
        Data::Huge => {
            for x in v.iter_mut() {
                *x = rng.range(1e295, 1e308) * if rng.bool_() { 1.0 } else { -1.0 };
            }
        }
        Data::Tiny => {
            for x in v.iter_mut() {
                // Random subnormal / very small doubles.
                let bits = (rng.next_u64() & 0x000F_FFFF_FFFF_FFFF)
                    | if rng.bool_() { 1u64 << 63 } else { 0 };
                *x = f64::from_bits(bits);
            }
        }
        Data::SignedZeros => {
            for x in v.iter_mut() {
                *x = if rng.bool_() { 0.0 } else { -0.0 };
            }
        }
        Data::WithInf => {
            for x in v.iter_mut() {
                *x = rng.range(-1.0, 1.0);
            }
            for _ in 0..(n / 4 + 1).min(n.max(1)) {
                if n > 0 {
                    let i = rng.below(n);
                    v[i] = if rng.bool_() {
                        f64::INFINITY
                    } else {
                        f64::NEG_INFINITY
                    };
                }
            }
        }
        Data::WithQNaN => {
            for x in v.iter_mut() {
                *x = rng.range(-1.0, 1.0);
            }
            for _ in 0..(n / 4 + 1) {
                if n > 0 {
                    let i = rng.below(n);
                    let payload = rng.next_u64() & 0x0007_FFFF_FFFF_FFFF;
                    let sign = if rng.bool_() { 1u64 << 63 } else { 0 };
                    v[i] = f64::from_bits(sign | 0x7FF8_0000_0000_0000 | payload);
                }
            }
        }
        Data::WithSNaN => {
            for x in v.iter_mut() {
                *x = rng.range(-1.0, 1.0);
            }
            for _ in 0..(n / 4 + 1) {
                if n > 0 {
                    let i = rng.below(n);
                    let payload = (rng.next_u64() & 0x0007_FFFF_FFFF_FFFF).max(1);
                    let sign = if rng.bool_() { 1u64 << 63 } else { 0 };
                    // exponent all ones, quiet bit CLEAR, payload nonzero
                    v[i] = f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload);
                }
            }
        }
        Data::RandomBits => {
            for x in v.iter_mut() {
                *x = f64::from_bits(rng.next_u64());
            }
        }
        Data::LowWordInf => {
            for x in v.iter_mut() {
                let hi = (rng.next_u32() as u64) << 32;
                let lo = if rng.bool_() { 0x7F80_0000u64 } else { 0xFF80_0000u64 };
                *x = f64::from_bits(hi | lo);
            }
        }
        Data::LowWordNaN => {
            for x in v.iter_mut() {
                let hi = (rng.next_u32() as u64) << 32;
                let lo = 0x7F80_0000u64 | (rng.next_u32() as u64 & 0x007F_FFFF).max(1);
                *x = f64::from_bits(hi | lo);
            }
        }
        Data::LowWordSubnormal => {
            for x in v.iter_mut() {
                let hi = (rng.next_u32() as u64) << 32;
                let lo = (rng.next_u32() as u64) & 0x007F_FFFF;
                *x = f64::from_bits(hi | lo);
            }
        }
    }
    v
}

/// `f32` generator mirroring `Data` for the low-level entry point.
pub fn gen_f32(rng: &mut Rng, n: usize, d: Data) -> Vec<f32> {
    let mut v = vec![0.0f32; n];
    match d {
        Data::Finite => {
            for x in v.iter_mut() {
                *x = rng.range(-1.0, 1.0) as f32;
            }
        }
        Data::Positive => {
            for x in v.iter_mut() {
                *x = rng.unit() as f32;
            }
        }
        Data::AllZeros => {}
        Data::Constant => {
            let c = rng.range(-4.0, 4.0) as f32;
            for x in v.iter_mut() {
                *x = c;
            }
        }
        Data::Ramp => {
            let step = rng.range(-2.0, 2.0) as f32;
            let base = rng.range(-2.0, 2.0) as f32;
            for (i, x) in v.iter_mut().enumerate() {
                *x = base + step * i as f32;
            }
        }
        Data::Spike => {
            if n > 0 {
                let i = rng.below(n);
                v[i] = rng.range(-100.0, 100.0) as f32;
            }
        }
        Data::Huge => {
            for x in v.iter_mut() {
                *x = (rng.range(1e30, 3.4e38) * if rng.bool_() { 1.0 } else { -1.0 }) as f32;
            }
        }
        Data::Tiny => {
            for x in v.iter_mut() {
                let bits = (rng.next_u32() & 0x007F_FFFF) | if rng.bool_() { 1 << 31 } else { 0 };
                *x = f32::from_bits(bits);
            }
        }
        Data::SignedZeros => {
            for x in v.iter_mut() {
                *x = if rng.bool_() { 0.0 } else { -0.0 };
            }
        }
        Data::WithInf => {
            for x in v.iter_mut() {
                *x = rng.range(-1.0, 1.0) as f32;
            }
            for _ in 0..(n / 4 + 1) {
                if n > 0 {
                    let i = rng.below(n);
                    v[i] = if rng.bool_() {
                        f32::INFINITY
                    } else {
                        f32::NEG_INFINITY
                    };
                }
            }
        }
        Data::WithQNaN | Data::LowWordNaN => {
            for x in v.iter_mut() {
                *x = rng.range(-1.0, 1.0) as f32;
            }
            for _ in 0..(n / 4 + 1) {
                if n > 0 {
                    let i = rng.below(n);
                    let payload = rng.next_u32() & 0x003F_FFFF;
                    let sign = if rng.bool_() { 1u32 << 31 } else { 0 };
                    v[i] = f32::from_bits(sign | 0x7FC0_0000 | payload);
                }
            }
        }
        Data::WithSNaN => {
            for x in v.iter_mut() {
                *x = rng.range(-1.0, 1.0) as f32;
            }
            for _ in 0..(n / 4 + 1) {
                if n > 0 {
                    let i = rng.below(n);
                    let payload = (rng.next_u32() & 0x003F_FFFF).max(1);
                    let sign = if rng.bool_() { 1u32 << 31 } else { 0 };
                    v[i] = f32::from_bits(sign | 0x7F80_0000 | payload);
                }
            }
        }
        Data::RandomBits => {
            for x in v.iter_mut() {
                *x = f32::from_bits(rng.next_u32());
            }
        }
        Data::LowWordInf => {
            for x in v.iter_mut() {
                *x = if rng.bool_() {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                };
            }
        }
        Data::LowWordSubnormal => {
            for x in v.iter_mut() {
                let bits = (rng.next_u32() & 0x007F_FFFF).max(1)
                    | if rng.bool_() { 1 << 31 } else { 0 };
                *x = f32::from_bits(bits);
            }
        }
    }
    v
}

// ----------------------------------------------------------- comparators -----

pub fn bits64(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}
pub fn bits32(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Differential call of `spectral_contrast` on **distinct** buffers.
///
/// Returns nothing; panics with a full bit-level dump on divergence. Both the
/// return value *and* the mutated buffers are compared, because the C
/// normalises in place.
pub fn diff_sc(ctx: &str, a: &[f32], b: &[f32], length: c_int) {
    let (c, rs) = libs();

    let mut ac = a.to_vec();
    let mut bc = b.to_vec();
    let mut ar = a.to_vec();
    let mut br = b.to_vec();

    let rc = unsafe { (c.spectral_contrast)(ac.as_mut_ptr(), bc.as_mut_ptr(), length) };
    let rr = unsafe { (rs.spectral_contrast)(ar.as_mut_ptr(), br.as_mut_ptr(), length) };

    assert_eq!(
        rc.to_bits(),
        rr.to_bits(),
        "{ctx}: spectral_contrast return diverged\n  C    = {rc:?} ({:#018x})\n  Rust = {rr:?} ({:#018x})\n  a = {:x?}\n  b = {:x?}",
        rc.to_bits(),
        rr.to_bits(),
        bits32(a),
        bits32(b)
    );
    assert_eq!(
        bits32(&ac),
        bits32(&ar),
        "{ctx}: spectral_contrast mutated `a` differently\n  in   = {:x?}\n  C    = {:x?}\n  Rust = {:x?}",
        bits32(a),
        bits32(&ac),
        bits32(&ar)
    );
    assert_eq!(
        bits32(&bc),
        bits32(&br),
        "{ctx}: spectral_contrast mutated `b` differently\n  in   = {:x?}\n  C    = {:x?}\n  Rust = {:x?}",
        bits32(b),
        bits32(&bc),
        bits32(&br)
    );
}

/// Differential call of `spectral_contrast` with **aliased** arguments
/// (`a == b`), which `include/match.h` permits (no `restrict`).
pub fn diff_sc_aliased(ctx: &str, a: &[f32], length: c_int) {
    let (c, rs) = libs();

    let mut ac = a.to_vec();
    let mut ar = a.to_vec();

    let rc = unsafe {
        let p = ac.as_mut_ptr();
        (c.spectral_contrast)(p, p, length)
    };
    let rr = unsafe {
        let p = ar.as_mut_ptr();
        (rs.spectral_contrast)(p, p, length)
    };

    assert_eq!(
        rc.to_bits(),
        rr.to_bits(),
        "{ctx}: aliased spectral_contrast return diverged\n  C = {rc:?}, Rust = {rr:?}\n  a = {:x?}",
        bits32(a)
    );
    assert_eq!(
        bits32(&ac),
        bits32(&ar),
        "{ctx}: aliased spectral_contrast mutated buffer differently\n  in   = {:x?}\n  C    = {:x?}\n  Rust = {:x?}",
        bits32(a),
        bits32(&ac),
        bits32(&ar)
    );
}

/// Differential call of `match` on **distinct** buffers. Also asserts neither
/// implementation writes through its (nominally mutable) input pointers.
pub fn diff_match(ctx: &str, test: &[f64], reference: &[f64], bins: c_int, threshold: f64) {
    let (c, rs) = libs();

    let mut tc = test.to_vec();
    let mut rfc = reference.to_vec();
    let mut tr = test.to_vec();
    let mut rfr = reference.to_vec();

    let vc = unsafe { (c.r#match)(tc.as_mut_ptr(), rfc.as_mut_ptr(), bins, threshold) };
    let vr = unsafe { (rs.r#match)(tr.as_mut_ptr(), rfr.as_mut_ptr(), bins, threshold) };

    assert_eq!(
        vc, vr,
        "{ctx}: match return diverged (C={vc}, Rust={vr})\n  bins={bins} threshold={threshold:?} ({:#018x})\n  test      = {:x?}\n  reference = {:x?}",
        threshold.to_bits(),
        bits64(test),
        bits64(reference)
    );
    assert_eq!(
        bits64(&tc),
        bits64(&tr),
        "{ctx}: match perturbed `test` differently"
    );
    assert_eq!(
        bits64(&rfc),
        bits64(&rfr),
        "{ctx}: match perturbed `reference` differently"
    );
    // `match` copies into VLAs; neither side may touch the caller's arrays.
    assert_eq!(bits64(&tc), bits64(test), "{ctx}: C match wrote to `test`");
    assert_eq!(
        bits64(&rfc),
        bits64(reference),
        "{ctx}: C match wrote to `reference`"
    );
}

/// Differential call of `match` with `test == reference` (aliased).
pub fn diff_match_aliased(ctx: &str, buf: &[f64], bins: c_int, threshold: f64) {
    let (c, rs) = libs();

    let mut bc = buf.to_vec();
    let mut br = buf.to_vec();

    let vc = unsafe {
        let p = bc.as_mut_ptr();
        (c.r#match)(p, p, bins, threshold)
    };
    let vr = unsafe {
        let p = br.as_mut_ptr();
        (rs.r#match)(p, p, bins, threshold)
    };

    assert_eq!(
        vc, vr,
        "{ctx}: aliased match return diverged (C={vc}, Rust={vr})\n  bins={bins} threshold={threshold:?}\n  buf = {:x?}",
        bits64(buf)
    );
    assert_eq!(
        bits64(&bc),
        bits64(&br),
        "{ctx}: aliased match perturbed buffer differently"
    );
}

/// Raw differential `match` call that keeps the exact pointers given -- used for
/// null-pointer and degenerate-length rows.
pub fn diff_match_raw(
    ctx: &str,
    tc: *mut f64,
    rc: *mut f64,
    tr: *mut f64,
    rr: *mut f64,
    bins: c_int,
    threshold: f64,
) {
    let (c, rs) = libs();
    let vc = unsafe { (c.r#match)(tc, rc, bins, threshold) };
    let vr = unsafe { (rs.r#match)(tr, rr, bins, threshold) };
    assert_eq!(
        vc, vr,
        "{ctx}: match return diverged (C={vc}, Rust={vr}) bins={bins} threshold={threshold:?}"
    );
}

/// Raw differential `spectral_contrast` call keeping the exact pointers given.
pub fn diff_sc_raw(ctx: &str, ac: *mut f32, bc: *mut f32, ar: *mut f32, br: *mut f32, n: c_int) {
    let (c, rs) = libs();
    let rc = unsafe { (c.spectral_contrast)(ac, bc, n) };
    let rr = unsafe { (rs.spectral_contrast)(ar, br, n) };
    assert_eq!(
        rc.to_bits(),
        rr.to_bits(),
        "{ctx}: spectral_contrast return diverged (C={rc:?}, Rust={rr:?}) n={n}"
    );
}

/// Run `body` in a forked-off copy of this test binary and report how it died.
/// Used for the rows of `ERRORS.md` whose "expected C result" is `SIGSEGV`.
///
/// The child is this same test executable, re-invoked with `--exact <test>` and
/// `CRASH_CHILD=1`; the guarded test body only performs the faulting call when
/// that variable is set.
pub fn run_isolated(test_name: &str) -> std::process::Output {
    use std::process::Command;
    let exe = std::env::current_exe().expect("current_exe");
    Command::new(exe)
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env("CRASH_CHILD", "1")
        .output()
        .expect("spawn isolated child")
}

pub fn is_crash_child() -> bool {
    std::env::var_os("CRASH_CHILD").is_some()
}

pub fn signal_of(out: &std::process::Output) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    out.status.signal()
}
