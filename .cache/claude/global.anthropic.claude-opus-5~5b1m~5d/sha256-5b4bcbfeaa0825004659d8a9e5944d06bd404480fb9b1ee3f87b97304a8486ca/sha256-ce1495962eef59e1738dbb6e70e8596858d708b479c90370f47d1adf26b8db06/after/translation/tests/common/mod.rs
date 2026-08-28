//! Shared differential-test scaffolding.
//!
//! Both libraries are loaded as shared objects with `libloading` and driven
//! exclusively through their exported symbols, so the `#[no_mangle]`/`extern
//! "C"` wrappers are on the critical path exactly as they are for an external
//! consumer. No Rust function is ever called directly.
#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

pub type ScFn = unsafe extern "C" fn(*mut f32, *mut f32, c_int) -> f64;
pub type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, c_int, f64) -> c_int;

/// One loaded shared object plus the two exported entry points.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    // Field order matters: symbols must be dropped before the library.
    pub spectral_contrast: ScFn,
    pub matchfn: MatchFn,
    _lib: Library,
}

impl Api {
    pub fn load(name: &'static str, path: &Path) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} ({}): {e}", path.display(), name));
            let sc: Symbol<ScFn> = lib
                .get(b"spectral_contrast\0")
                .unwrap_or_else(|e| panic!("{name}: missing `spectral_contrast`: {e}"));
            let mt: Symbol<MatchFn> = lib
                .get(b"match\0")
                .unwrap_or_else(|e| panic!("{name}: missing `match`: {e}"));
            let api = Api {
                name,
                path: path.to_path_buf(),
                spectral_contrast: *sc,
                matchfn: *mt,
                _lib: lib,
            };
            api
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so` — the project name is the parent directory's
/// name (see `c_src/CMakeLists.txt`), so it is discovered rather than hardcoded.
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C shared object in {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        ),
        _ => found.remove(0),
    }
}

/// `target/release/libunderhanded_c_nuke_lib.so`, falling back to `target/debug`.
///
/// `cargo test` builds test harnesses but does **not** re-emit a `cdylib`, so the
/// `.so` may be missing or stale even though the tests themselves just compiled.
/// Rather than fail confusingly, shell out to `cargo build --release` once. (The
/// build-directory lock has already been released by the time tests run.)
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    static ENSURED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    let base = manifest_dir().join("target");
    let release = base.join("release").join("libunderhanded_c_nuke_lib.so");
    let src = manifest_dir().join("src/lib.rs");
    let stale = || match (std::fs::metadata(&release), std::fs::metadata(&src)) {
        (Ok(a), Ok(b)) => match (a.modified(), b.modified()) {
            (Ok(a), Ok(b)) => a < b,
            _ => false,
        },
        _ => true,
    };
    if stale() {
        ENSURED.get_or_init(|| {
            for extra in [&["--offline"][..], &[][..]] {
                let ok = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()))
                    .arg("build")
                    .arg("--release")
                    .args(extra)
                    .current_dir(manifest_dir())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if ok {
                    return;
                }
            }
        });
    }
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libunderhanded_c_nuke_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust shared object under {}. Build it with `cargo build --release` \
         (note that `cargo test` alone does NOT emit a cdylib).",
        base.display()
    )
}

pub fn rust_lib_path_for(profile: &str) -> Option<PathBuf> {
    let p = manifest_dir()
        .join("target")
        .join(profile)
        .join("libunderhanded_c_nuke_lib.so");
    if p.exists() { Some(p) } else { None }
}

pub fn c_api() -> Api {
    let p = c_lib_path();
    assert_fresher_than(&p, &manifest_dir().join("../c_src/src/match.c"));
    assert_fresher_than(&p, &manifest_dir().join("../c_src/src/spectral_contrast.c"));
    Api::load("C", &p)
}

pub fn rust_api() -> Api {
    let p = rust_lib_path();
    // `cargo test` builds test harnesses but does NOT re-emit a `cdylib`
    // artifact, so it is very easy to end up differentially testing a stale
    // `.so`. Refuse to run instead.
    assert_fresher_than(&p, &manifest_dir().join("src/lib.rs"));
    Api::load("Rust", &p)
}

fn assert_fresher_than(artifact: &Path, source: &Path) {
    let m = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
    };
    if !source.exists() {
        return;
    }
    assert!(
        m(artifact) >= m(source),
        "STALE ARTIFACT: {} is older than {}. Rebuild it \
         (`cargo build --release` for the Rust .so — note that `cargo test` does \
         NOT re-emit a cdylib — or re-run `cmake --build .` for the C .so).",
        artifact.display(),
        source.display()
    );
}

/// The pair every differential test drives.
pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

pub fn pair() -> Pair {
    Pair {
        c: c_api(),
        rust: rust_api(),
    }
}

// ===========================================================================
// Deterministic PRNG (SplitMix64) — fixed seed, so every failure reproduces.
// ===========================================================================

pub struct Rng(pub u64);

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Extra seed offset, from `$DIFF_SEED` (default 0). Each test hard-codes its
/// own per-row seed so a failure is reproducible; setting `DIFF_SEED=k` shifts
/// every stream to a fresh, still fully reproducible corpus. `stress.sh` loops
/// over many values of it.
pub fn seed_offset() -> u64 {
    std::env::var("DIFF_SEED")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed
            .wrapping_mul(0x1000_0000_0000_01B3)
            .wrapping_add(SEED)
            .wrapping_add(seed_offset().wrapping_mul(0xD1B5_4A32_D192_ED03)))
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
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    /// A "realistic" positive spectrum bin: log-uniform over `[2^-40, 2^40]`.
    pub fn pos_normal_f64(&mut self) -> f64 {
        let e = (self.unit() * 80.0 - 40.0) as i32;
        (1.0 + self.unit()) * (2.0f64).powi(e)
    }
    pub fn signed_normal_f64(&mut self) -> f64 {
        let m = self.pos_normal_f64();
        if self.next_u64() & 1 == 0 { m } else { -m }
    }
    /// Log-uniform positive f32 in `[2^-40, 2^40]`.
    pub fn pos_normal_f32(&mut self) -> f32 {
        let e = (self.unit() * 80.0 - 40.0) as i32;
        ((1.0 + self.unit()) as f32) * (2.0f32).powi(e)
    }
    pub fn signed_normal_f32(&mut self) -> f32 {
        let m = self.pos_normal_f32();
        if self.next_u64() & 1 == 0 { m } else { -m }
    }
    /// Fully arbitrary f32 bit pattern: hits +/-0, subnormals, +/-inf, qNaN,
    /// sNaN and every exponent, in their natural proportions.
    pub fn any_f32_bits(&mut self) -> u32 {
        self.next_u32()
    }
    pub fn any_f64_bits(&mut self) -> u64 {
        self.next_u64()
    }
    /// f32 with a uniformly random exponent — magnitudes from 1e-45 to 1e38, so
    /// dot products both underflow and overflow.
    pub fn exp_biased_f32_bits(&mut self) -> u32 {
        let sign = (self.next_u64() as u32 & 1) << 31;
        let exp = (self.next_u32() % 255) << 23;
        let mant = self.next_u32() & 0x007F_FFFF;
        sign | exp | mant
    }
    pub fn exp_biased_f64_bits(&mut self) -> u64 {
        let sign = (self.next_u64() & 1) << 63;
        let exp = ((self.next_u64() % 2047) as u64) << 52;
        let mant = self.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        sign | exp | mant
    }
    /// A quiet NaN f32 with a random non-zero payload.
    pub fn qnan_f32_bits(&mut self) -> u32 {
        let sign = (self.next_u64() as u32 & 1) << 31;
        let payload = self.next_u32() & 0x003F_FFFF;
        sign | 0x7FC0_0000 | payload
    }
    /// A signalling NaN f32 with a random non-zero payload.
    pub fn snan_f32_bits(&mut self) -> u32 {
        let sign = (self.next_u64() as u32 & 1) << 31;
        let mut payload = self.next_u32() & 0x003F_FFFF;
        if payload == 0 {
            payload = 1;
        }
        sign | 0x7F80_0000 | payload
    }
    pub fn qnan_f64_bits(&mut self) -> u64 {
        let sign = (self.next_u64() & 1) << 63;
        let payload = self.next_u64() & 0x0007_FFFF_FFFF_FFFF;
        sign | 0x7FF8_0000_0000_0000 | payload
    }
}

// ===========================================================================
// Uniform call wrappers
// ===========================================================================

/// Result of one `spectral_contrast` invocation: the returned double's raw bits
/// plus the raw bits of the whole scratch buffer afterwards (the function
/// mutates its arguments in place, so the buffer is part of the observable
/// output).
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ScOut {
    pub ret: u64,
    pub buf: Vec<u32>,
}

/// Call `spectral_contrast(&buf[a_off], &buf[b_off], length)`.
///
/// A single flat buffer covers every pointer relationship from CONFIGS.md axis
/// I at once: `a_off != b_off` far apart = distinct, `a_off == b_off` = fully
/// aliased, small difference = partial overlap.
pub fn sc_call(api: &Api, buf_bits: &[u32], a_off: usize, b_off: usize, length: c_int) -> ScOut {
    let mut buf: Vec<f32> = buf_bits.iter().map(|&b| f32::from_bits(b)).collect();
    let base = buf.as_mut_ptr();
    let ret = unsafe {
        (api.spectral_contrast)(base.add(a_off), base.add(b_off), length).to_bits()
    };
    ScOut {
        ret,
        buf: buf.iter().map(|v| v.to_bits()).collect(),
    }
}

/// Same, but the caller's buffer is an array of `double` — the way `match.h`
/// declares `spectral_contrast`, and the way `match` itself calls it. The
/// offsets are in *f64 elements*; `length` is still in f32 elements.
pub fn sc_call_via_f64(
    api: &Api,
    buf_bits: &[u64],
    a_off: usize,
    b_off: usize,
    length: c_int,
) -> (u64, Vec<u64>) {
    let mut buf: Vec<f64> = buf_bits.iter().map(|&b| f64::from_bits(b)).collect();
    let base = buf.as_mut_ptr();
    let ret = unsafe {
        (api.spectral_contrast)(
            base.add(a_off) as *mut f32,
            base.add(b_off) as *mut f32,
            length,
        )
        .to_bits()
    };
    (ret, buf.iter().map(|v| v.to_bits()).collect())
}

/// Result of one `match` invocation: the int return value plus the raw bits of
/// the whole input buffer afterwards (`match` must leave it untouched).
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct MatchOut {
    pub ret: c_int,
    pub buf: Vec<u64>,
}

/// Call `match(&buf[t_off], &buf[r_off], bins, threshold)`.
pub fn match_call(
    api: &Api,
    buf_bits: &[u64],
    t_off: usize,
    r_off: usize,
    bins: c_int,
    threshold: f64,
) -> MatchOut {
    let mut buf: Vec<f64> = buf_bits.iter().map(|&b| f64::from_bits(b)).collect();
    let base = buf.as_mut_ptr();
    let ret = unsafe { (api.matchfn)(base.add(t_off), base.add(r_off), bins, threshold) };
    MatchOut {
        ret,
        buf: buf.iter().map(|v| v.to_bits()).collect(),
    }
}

// ===========================================================================
// Assertions
// ===========================================================================

fn describe_f32(b: u32) -> String {
    format!("{:#010x}({})", b, f32::from_bits(b))
}
fn describe_f64(b: u64) -> String {
    format!("{:#018x}({})", b, f64::from_bits(b))
}

pub fn assert_sc_eq(row: &str, ctx: &str, input: &[u32], c: &ScOut, r: &ScOut) {
    if c == r {
        return;
    }
    let mut msg = format!("[{row}] spectral_contrast divergence: {ctx}\n");
    msg += &format!(
        "  ret: C={} Rust={}\n",
        describe_f64(c.ret),
        describe_f64(r.ret)
    );
    let n = c.buf.len().max(r.buf.len());
    let mut shown = 0;
    for i in 0..n {
        let cv = c.buf.get(i).copied();
        let rv = r.buf.get(i).copied();
        if cv != rv {
            if shown < 12 {
                msg += &format!(
                    "  buf[{i}]: in={} C={} Rust={}\n",
                    input.get(i).map(|&b| describe_f32(b)).unwrap_or_default(),
                    cv.map(describe_f32).unwrap_or_default(),
                    rv.map(describe_f32).unwrap_or_default()
                );
            }
            shown += 1;
        }
    }
    if shown > 12 {
        msg += &format!("  ... and {} more differing elements\n", shown - 12);
    }
    msg += &format!("  input (f32 bits) = {:#010x?}\n", input);
    panic!("{msg}");
}

pub fn assert_match_eq(row: &str, ctx: &str, input: &[u64], c: &MatchOut, r: &MatchOut) {
    if c == r {
        return;
    }
    let mut msg = format!("[{row}] match divergence: {ctx}\n");
    msg += &format!("  ret: C={} Rust={}\n", c.ret, r.ret);
    for i in 0..c.buf.len().max(r.buf.len()) {
        let cv = c.buf.get(i).copied();
        let rv = r.buf.get(i).copied();
        if cv != rv {
            msg += &format!(
                "  buf[{i}]: in={} C={} Rust={}\n",
                input.get(i).map(|&b| describe_f64(b)).unwrap_or_default(),
                cv.map(describe_f64).unwrap_or_default(),
                rv.map(describe_f64).unwrap_or_default()
            );
        }
    }
    msg += &format!("  input (f64 bits) = {:#018x?}\n", input);
    panic!("{msg}");
}

/// Lengths that straddle every boundary the C distinguishes: `N_SMOOTH == 16`,
/// the `length - 1` loop in `differentiate`, and odd/even (which decides how
/// many `double`s the f32-strided reader touches).
pub const LENGTHS: &[c_int] = &[1, 2, 3, 15, 16, 17, 31, 32, 33, 64, 1000];

/// Every `threshold` class from CONFIGS.md axis G.
pub fn thresholds() -> Vec<f64> {
    vec![
        f64::NEG_INFINITY,
        -f64::MAX,
        -1.0,
        -0.0,
        0.0,
        f64::MIN_POSITIVE,
        0.25,
        0.5,
        1.0,
        2.0,
        f64::MAX,
        f64::INFINITY,
        f64::NAN,
    ]
}
