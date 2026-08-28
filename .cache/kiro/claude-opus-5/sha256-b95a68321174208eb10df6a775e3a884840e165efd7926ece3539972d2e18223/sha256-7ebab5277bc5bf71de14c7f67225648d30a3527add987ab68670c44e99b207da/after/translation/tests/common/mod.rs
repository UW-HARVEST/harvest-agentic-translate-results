//! Shared harness: loads the C `.so` and the Rust `.so` and exposes both
//! through identical `libloading` bindings so every comparison goes across the
//! real FFI boundary.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub const MAX_RESULTS: usize = 10;

/// `typedef struct { int value; double scaled; int rank; } Result;`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CResult {
    pub value: i32,
    pub scaled: f64,
    pub rank: i32,
}

/// `typedef struct { Result data[10]; int count; } ResultArray;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CResultArray {
    pub data: [CResult; MAX_RESULTS],
    pub count: i32,
}

impl Default for CResultArray {
    fn default() -> Self {
        Self {
            data: [CResult::default(); MAX_RESULTS],
            count: 0,
        }
    }
}

pub type OpFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

/// Field-wise equality that treats `scaled` as a raw bit pattern, so NaN
/// payloads and -0.0 vs 0.0 are distinguished. Struct padding is deliberately
/// ignored (C leaves it indeterminate).
pub fn arrays_bit_equal(a: &CResultArray, b: &CResultArray) -> bool {
    if a.count != b.count {
        return false;
    }
    for i in 0..MAX_RESULTS {
        if a.data[i].value != b.data[i].value
            || a.data[i].rank != b.data[i].rank
            || a.data[i].scaled.to_bits() != b.data[i].scaled.to_bits()
        {
            return false;
        }
    }
    true
}

pub fn describe(a: &CResultArray) -> String {
    let mut s = format!("count={}", a.count);
    for i in 0..MAX_RESULTS {
        s += &format!(
            "\n  [{}] value={} rank={} scaled=0x{:016x} ({:?})",
            i,
            a.data[i].value,
            a.data[i].rank,
            a.data[i].scaled.to_bits(),
            a.data[i].scaled
        );
    }
    s
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// `cargo test` builds the lib target for unit tests but does **not**
/// necessarily refresh the `cdylib` artifact, since no integration test links
/// against it. Loading whatever `.so` happens to be lying in the profile
/// directory would silently test stale code, so build it explicitly, once per
/// test binary, before any load.
fn ensure_rust_so_built(profile_dir: &std::path::Path) {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");

        let mut cmd = std::process::Command::new(&cargo);
        cmd.arg("build")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(&manifest)
            .arg("--lib");

        if profile_dir.file_name().and_then(|s| s.to_str()) == Some("release") {
            cmd.arg("--release");
        }

        // Reproduce the feature selection this test binary was compiled with,
        // so the .so under test matches the harness's expectations.
        cmd.arg("--no-default-features");
        let feats = enabled_features();
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats.join(","));
        }

        // Keep the artifact in the same target directory we are going to read.
        if let Some(target_dir) = profile_dir.parent() {
            cmd.env("CARGO_TARGET_DIR", target_dir);
        }
        // Avoid inheriting the parent cargo's build-plan environment.
        cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");

        let out = cmd
            .output()
            .unwrap_or_else(|e| panic!("failed to spawn `{cargo} build`: {e}"));
        if !out.status.success() {
            panic!(
                "`cargo build --lib` failed while refreshing the cdylib:\n{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    });
}

/// Features this test binary was compiled with. Kept in sync with
/// `[features]` in Cargo.toml; the crate currently declares none, so this is
/// empty and `--no-default-features` alone describes the only configuration.
fn enabled_features() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut v: Vec<&'static str> = Vec::new();
    v
}

fn find_rust_so() -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so walk up
    // to the profile directory and look for the cdylib next to it.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();

    ensure_rust_so_built(&profile_dir);

    let direct = profile_dir.join("libarrayfunc_lib.so");
    if direct.exists() {
        assert_rust_so_fresh(&direct);
        return direct;
    }

    panic!(
        "libarrayfunc_lib.so not found in {} after `cargo build --lib`",
        profile_dir.display()
    );
}

/// Belt-and-braces: the artifact must be at least as new as every Rust source
/// file it is built from. Catches any build that silently did not happen.
fn assert_rust_so_fresh(so: &std::path::Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat cdylib");

    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
                    if newest.as_ref().is_none_or(|(_, best)| t > *best) {
                        newest = Some((p, t));
                    }
                }
            }
        }
    }

    if let Some((path, t)) = newest {
        assert!(
            so_mtime >= t,
            "{} is older than {} — the tests would be comparing stale code. \
             Run `cargo build` (add --release when testing the release profile).",
            so.display(),
            path.display()
        );
    }
}

/// One loaded implementation. All calls go through `libloading`, so the Rust
/// side is exercised exactly like an external C caller would: via the
/// `#[no_mangle]` exports in the shared object.
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: PathBuf) -> Self {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        Self { name, lib }
    }

    fn sym<T>(&self, name: &str) -> Symbol<'_, T> {
        unsafe { self.lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol `{name}`: {e}", self.name))
    }

    // ---- operations -------------------------------------------------------

    pub fn op_ptr(&self, which: &str) -> OpFn {
        let s: Symbol<OpFn> = self.sym(which);
        *s
    }

    pub fn add_operation(&self, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        unsafe { (self.op_ptr("add_operation"))(a, b, u1, u2) }
    }
    pub fn multiply_operation(&self, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        unsafe { (self.op_ptr("multiply_operation"))(a, b, u1, u2) }
    }
    pub fn subtract_operation(&self, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        unsafe { (self.op_ptr("subtract_operation"))(a, b, u1, u2) }
    }
    pub fn modulo_operation(&self, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        unsafe { (self.op_ptr("modulo_operation"))(a, b, u1, u2) }
    }

    // ---- scalar helpers ---------------------------------------------------

    pub fn safe_double_to_int(&self, d: f64) -> i32 {
        let f: Symbol<unsafe extern "C" fn(f64) -> i32> = self.sym("safe_double_to_int");
        unsafe { f(d) }
    }

    pub fn compute_scaled_value(&self, base: i32, scale: f64) -> i32 {
        let f: Symbol<unsafe extern "C" fn(i32, f64) -> i32> = self.sym("compute_scaled_value");
        unsafe { f(base, scale) }
    }

    // ---- array helpers ----------------------------------------------------

    pub fn init_result_array(&self, arr: &mut CResultArray, values: &mut [i32], count: i32) {
        let f: Symbol<unsafe extern "C" fn(*mut CResultArray, *mut i32, i32)> =
            self.sym("init_result_array");
        unsafe { f(arr as *mut CResultArray, values.as_mut_ptr(), count) }
    }

    pub fn compare_results_in_array(&self, arr: &mut CResultArray, i1: i32, i2: i32) -> i32 {
        let f: Symbol<unsafe extern "C" fn(*mut CResultArray, i32, i32) -> i32> =
            self.sym("compare_results_in_array");
        unsafe { f(arr as *mut CResultArray, i1, i2) }
    }

    pub fn process_with_foreach(&self, arr: &mut CResultArray, op: OpFn) -> i32 {
        let f: Symbol<unsafe extern "C" fn(*mut CResultArray, OpFn) -> i32> =
            self.sym("process_with_foreach");
        unsafe { f(arr as *mut CResultArray, op) }
    }

    pub fn compute_weighted_sum(&self, arr: &mut CResultArray) -> i32 {
        let f: Symbol<unsafe extern "C" fn(*mut CResultArray) -> i32> =
            self.sym("compute_weighted_sum");
        unsafe { f(arr as *mut CResultArray) }
    }

    // ---- entry point ------------------------------------------------------

    pub fn arrayfunc(&self, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
        let f: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> = self.sym("arrayfunc");
        unsafe { f(p1, p2, p3, p4) }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn load() -> Pair {
    Pair {
        c: Impl::open("C", find_c_so()),
        rs: Impl::open("Rust", find_rust_so()),
    }
}

// ---------------------------------------------------------------------------
// Input corpora
// ---------------------------------------------------------------------------

/// Deterministic xorshift so runs are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Small magnitudes, where the interesting non-saturating behaviour lives.
    pub fn small_i32(&mut self) -> i32 {
        (self.next_u64() % 2001) as i32 - 1000
    }
}

/// Integers worth trying: identities, boundaries, and overflow triggers.
pub fn interesting_i32() -> Vec<i32> {
    vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        10,
        -10,
        100,
        -100,
        1000,
        -1000,
        65535,
        65536,
        -65536,
        1 << 20,
        -(1 << 20),
        1 << 30,
        -(1 << 30),
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        46341,
        -46341,
        2147483,
        -2147483,
    ]
}

/// Doubles worth trying against `safe_double_to_int`: the exact clamp
/// boundaries, the neighbouring representable values, NaNs, and infinities.
pub fn interesting_f64() -> Vec<f64> {
    let imax = i32::MAX as f64; // 2147483647.0, exactly representable
    let imin = i32::MIN as f64; // -2147483648.0, exactly representable
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        1.5,
        -1.5,
        2.5,
        -2.5,
        -0.75,
        0.75,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        imax,
        imin,
        imax - 1.0,
        imin + 1.0,
        imax + 1.0,
        imin - 1.0,
        imax - 0.5,
        imin + 0.5,
        f64::from_bits(imax.to_bits() - 1),
        f64::from_bits(imax.to_bits() + 1),
        f64::from_bits((-imin).to_bits() - 1),
        2147483646.9999997,
        -2147483647.9999995,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_dead_beef), // quiet NaN, custom payload
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::MAX,
        f64::MIN,
        1e300,
        -1e300,
        1e-300,
        -1e-300,
        123.456,
        -123.456,
        1e9,
        -1e9,
        3.0 / 7.0,
    ];
    // A few multiples of the scale factors the library itself uses.
    for k in [0.75f64, 1.5, 0.8, 0.333] {
        for n in [1.0f64, 3.0, 7.0, 1e9, -1e9, imax, imin] {
            v.push(k * n);
        }
    }
    v
}
