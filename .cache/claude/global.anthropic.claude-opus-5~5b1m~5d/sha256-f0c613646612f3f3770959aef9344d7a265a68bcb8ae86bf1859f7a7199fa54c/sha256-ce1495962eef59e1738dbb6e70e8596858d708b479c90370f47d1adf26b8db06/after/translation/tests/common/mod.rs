//! Shared differential-test harness.
//!
//! Both implementations are loaded as *shared objects* through `libloading`
//! and called only through their exported `next_double` symbol. The Rust
//! functions are never called directly, so the `#[no_mangle] extern "C"`
//! wrapper is part of what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::os::unix::Symbol as RawSymbol;
use libloading::Library;

// ---------------------------------------------------------------------------
// ABI mirror of `cn_rnd_t` from c_src/include/lib.h
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct cn_rnd_t { uint64_t state[2]; } cn_rnd_t;
/// ```
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct CnRnd {
    pub state: [u64; 2],
}

impl CnRnd {
    pub fn new(s0: u64, s1: u64) -> Self {
        CnRnd { state: [s0, s1] }
    }
    /// Raw 16 bytes of the struct, for byte-exact comparison.
    pub fn bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&self.state[0].to_ne_bytes());
        out[8..].copy_from_slice(&self.state[1].to_ne_bytes());
        out
    }
}

/// `double next_double(cn_rnd_t *rnd)`
pub type NextDoubleFn = unsafe extern "C" fn(*mut CnRnd) -> f64;

// ---------------------------------------------------------------------------
// Loaded library handle
// ---------------------------------------------------------------------------

pub struct Lib {
    _lib: Library,
    next_double: RawSymbol<NextDoubleFn>,
    pub label: String,
    pub path: PathBuf,
}

impl Lib {
    fn open(path: &Path, label: &str) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let sym: libloading::Symbol<NextDoubleFn> = unsafe { lib.get(b"next_double\0") }
            .unwrap_or_else(|e| panic!("dlsym(next_double) in {} failed: {e}", path.display()));
        let raw = unsafe { sym.into_raw() };
        Lib {
            _lib: lib,
            next_double: raw,
            label: label.to_string(),
            path: path.to_path_buf(),
        }
    }

    /// Call through the `.so`'s exported symbol with an arbitrary raw pointer.
    pub unsafe fn call_ptr(&self, p: *mut CnRnd) -> f64 {
        (*self.next_double)(p)
    }

    /// Call through the `.so`'s exported symbol on a live struct.
    pub fn call(&self, rnd: &mut CnRnd) -> f64 {
        unsafe { (*self.next_double)(rnd as *mut CnRnd) }
    }
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn repo_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Path to the C shared object, building it with CMake if necessary.
pub fn c_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        if let Some(p) = find_so(&build) {
            return p;
        }
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let st = Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .expect("run cmake");
        assert!(st.success(), "cmake configure failed");
        let st = Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .status()
            .expect("run cmake --build");
        assert!(st.success(), "cmake --build failed");
        find_so(&build).expect("no .so produced in c_src/build")
    })
    .clone()
}

/// Extra cargo args (feature selection) forwarded to the nested cdylib build.
/// `run_all_configs.sh` sets `TRANSLATION_FEATURE_ARGS`.
fn feature_args() -> Vec<String> {
    std::env::var("TRANSLATION_FEATURE_ARGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// The cdylib build configurations that must all match the C library.
///
/// * `dev`     — `overflow-checks = on`, Rust UB checks off (C-ABI semantics)
/// * `release` — the published artifact (`opt-level = 3`, `panic = abort`)
/// * `ubcheck` — like `dev` but with Rust's optional UB checks turned back on
pub const PROFILES: [&str; 3] = ["dev", "release", "ubcheck"];

fn profile_dirs(profile: &str) -> (&'static str, &'static str) {
    match profile {
        "release" => ("so-rel", "release"),
        "ubcheck" => ("so-ubc", "ubcheck"),
        "dev" => ("so-dev", "debug"),
        other => panic!("unknown cdylib profile {other}"),
    }
}

fn build_rust_so(profile: &str) -> PathBuf {
    // A dedicated CARGO_TARGET_DIR keeps this nested build from contending on
    // the lock held by the outer `cargo test` invocation.
    let (target_sub, out_sub) = profile_dirs(profile);
    let target_dir = manifest_dir().join("target").join(target_sub);
    let out_dir = target_dir.join(out_sub);

    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("build")
        .arg("--offline")
        .arg("--lib");
    match profile {
        "release" => {
            cmd.arg("--release");
        }
        "ubcheck" => {
            cmd.args(["--profile", "ubcheck"]);
        }
        _ => {}
    }
    for a in feature_args() {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run nested cargo build");
    assert!(
        out.status.success(),
        "nested `cargo build` ({profile}) failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    find_so(&out_dir).unwrap_or_else(|| panic!("no .so in {}", out_dir.display()))
}

pub fn rust_so_path(profile: &str) -> PathBuf {
    static DEV: OnceLock<PathBuf> = OnceLock::new();
    static REL: OnceLock<PathBuf> = OnceLock::new();
    static UBC: OnceLock<PathBuf> = OnceLock::new();
    match profile {
        "release" => REL.get_or_init(|| build_rust_so("release")).clone(),
        "ubcheck" => UBC.get_or_init(|| build_rust_so("ubcheck")).clone(),
        "dev" => DEV.get_or_init(|| build_rust_so("dev")).clone(),
        other => panic!("unknown cdylib profile {other}"),
    }
}

/// The C library.
pub fn c_lib() -> Lib {
    Lib::open(&c_so_path(), "C")
}

/// The Rust library for a given cdylib profile (`"dev"` / `"release"`).
pub fn rust_lib(profile: &str) -> Lib {
    Lib::open(&rust_so_path(profile), &format!("rust[{profile}]"))
}

/// Every (C, Rust) pair that must agree — one per cdylib profile in
/// [`PROFILES`]. Override with `TRANSLATION_PROFILES=dev,release`.
pub fn pairs() -> Vec<(Lib, Lib)> {
    let profiles: Vec<String> = match std::env::var("TRANSLATION_PROFILES") {
        Ok(v) if !v.trim().is_empty() => v
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
        _ => PROFILES.iter().map(|s| s.to_string()).collect(),
    };
    profiles
        .into_iter()
        .map(|p| (c_lib(), rust_lib(&p)))
        .collect()
}

// ---------------------------------------------------------------------------
// Deterministic input generation (SplitMix64, fixed seed)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        SplitMix64(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_nonzero(&mut self) -> u64 {
        loop {
            let v = self.next_u64();
            if v != 0 {
                return v;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reference model — used ONLY to *construct* interesting inputs, never to
// assert correctness. All assertions are C-`.so` vs Rust-`.so`.
// ---------------------------------------------------------------------------

/// `x ^= x << 23; x ^= x >> 17;` from `cn_rnd_next`.
pub fn h(x: u64) -> u64 {
    let a = x ^ (x << 23);
    a ^ (a >> 17)
}

/// Inverse of [`h`] (both xorshift steps are bijections on `u64`).
pub fn h_inv(g: u64) -> u64 {
    let a = g ^ (g >> 17) ^ (g >> 34) ^ (g >> 51);
    a ^ (a << 23) ^ (a << 46)
}

/// Reference model of the `static` C helper, for input construction.
pub fn ref_next(state: &mut [u64; 2]) -> u64 {
    let x0 = state[0];
    let y = state[1];
    state[0] = y;
    let x = h(x0) ^ y ^ (y >> 26);
    state[1] = x;
    x.wrapping_add(y)
}

/// Build a seed whose **first** `next_double` call produces exactly
/// `desired_value` out of `cn_rnd_next`, with `state[1] == y`.
pub fn seed_for_value(desired_value: u64, y: u64) -> CnRnd {
    let x_final = desired_value.wrapping_sub(y);
    let g = x_final ^ y ^ (y >> 26);
    let seed = CnRnd::new(h_inv(g), y);
    // self-check the construction
    let mut s = seed.state;
    let got = ref_next(&mut s);
    assert_eq!(
        got, desired_value,
        "seed_for_value construction broken (y={y:#018x})"
    );
    seed
}

/// The `double` the C code produces from a raw generator output.
pub fn value_to_double_bits(value: u64) -> u64 {
    let mantissa = value >> 12;
    let bits = (1023u64 << 52) | mantissa;
    (f64::from_bits(bits) - 1.0).to_bits()
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn fail(ctx: &str, step: usize, seed: CnRnd, what: &str, c: String, r: String, rl: &str) -> ! {
    panic!(
        "DIVERGENCE [{ctx}] {what}\n  \
         seed        = {{{:#018x}, {:#018x}}}\n  \
         step        = {step}\n  \
         C           = {c}\n  \
         {rl:<11} = {r}",
        seed.state[0], seed.state[1]
    );
}

/// Run `iters` consecutive `next_double` calls on each library, starting from
/// `seed`, asserting after every single call that
///   * the returned `double`'s raw bits are identical, and
///   * the whole 16-byte `cn_rnd_t` is byte-identical.
pub fn assert_seq(c: &Lib, r: &Lib, seed: CnRnd, iters: usize, ctx: &str) {
    let mut sc = seed;
    let mut sr = seed;
    for step in 0..iters {
        let vc = c.call(&mut sc);
        let vr = r.call(&mut sr);
        if vc.to_bits() != vr.to_bits() {
            fail(
                ctx,
                step,
                seed,
                "returned double differs",
                format!("{:#018x} ({vc:?})", vc.to_bits()),
                format!("{:#018x} ({vr:?})", vr.to_bits()),
                &r.label,
            );
        }
        if sc.bytes() != sr.bytes() {
            fail(
                ctx,
                step,
                seed,
                "post-call cn_rnd_t state differs",
                format!("{:02x?}", sc.bytes()),
                format!("{:02x?}", sr.bytes()),
                &r.label,
            );
        }
        // The returned double must always be a normal value in [0, 1).
        assert!(
            vc >= 0.0 && vc < 1.0,
            "[{ctx}] C returned out-of-range {vc:?}"
        );
    }
}

/// One call, from `seed`.
pub fn assert_one(c: &Lib, r: &Lib, seed: CnRnd, ctx: &str) {
    assert_seq(c, r, seed, 1, ctx);
}

/// Run a closure over every (C, Rust-profile) pair.
pub fn for_each_pair<F: FnMut(&Lib, &Lib)>(mut f: F) {
    let ps = pairs();
    assert!(!ps.is_empty(), "no (C, Rust) library pairs to test");
    for (c, r) in &ps {
        f(c, r);
    }
}
