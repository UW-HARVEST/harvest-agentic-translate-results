//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through `dlsym`-resolved `extern "C"` pointers. The Rust crate is
//! *never* linked directly, so the `#[no_mangle]` export wrappers are part of
//! what is under test.

#![allow(dead_code)]

use libloading::Library;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C-compatible types (mirrors of the private types in c_src/src/lib.c)
// ---------------------------------------------------------------------------

/// `typedef struct { int value; double scaled; int rank; } Result;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CResult {
    pub value: i32,
    pub scaled: f64,
    pub rank: i32,
}

impl Default for CResult {
    fn default() -> Self {
        CResult {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }
    }
}

/// `typedef struct { Result data[10]; int count; } ResultArray;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CResultArray {
    pub data: [CResult; 10],
    pub count: i32,
}

impl Default for CResultArray {
    fn default() -> Self {
        CResultArray {
            data: [CResult::default(); 10],
            count: 0,
        }
    }
}

impl CResultArray {
    /// All-zero array with `count == 0`.
    pub fn zeroed() -> Self {
        Self::default()
    }

    /// Array whose every byte is `0xAA`, so any element the library leaves
    /// untouched is instantly recognisable (and must be left untouched by both
    /// implementations alike).
    pub fn poisoned(count: i32) -> Self {
        let mut u = std::mem::MaybeUninit::<CResultArray>::uninit();
        unsafe {
            std::ptr::write_bytes(u.as_mut_ptr() as *mut u8, 0xAA, SIZE_OF_RESULT_ARRAY);
            let mut a = u.assume_init();
            a.count = count;
            a
        }
    }

    /// Build an array directly (bypassing `init_result_array`) so that
    /// `process_with_foreach` / `compute_weighted_sum` can be driven with
    /// arbitrary state.
    pub fn from_values(values: &[i32]) -> Self {
        let mut a = Self::zeroed();
        a.count = values.len() as i32;
        for (i, v) in values.iter().enumerate().take(10) {
            a.data[i] = CResult {
                value: *v,
                scaled: *v as f64 * 1.5,
                rank: i as i32,
            };
        }
        a
    }

    /// Comparable projection: every *named* field, with `scaled` compared by its
    /// exact IEEE-754 bit pattern (so `-0.0 != 0.0` and NaN payloads matter).
    /// Struct padding is deliberately excluded — C leaves it indeterminate.
    pub fn snapshot(&self) -> (i32, Vec<(i32, u64, i32)>) {
        (
            self.count,
            self.data
                .iter()
                .map(|r| (r.value, r.scaled.to_bits(), r.rank))
                .collect(),
        )
    }
}

pub const SIZE_OF_RESULT_ARRAY: usize = 248;
pub const SIZE_OF_RESULT: usize = 24;
pub const OFFSET_OF_COUNT: usize = 240;

pub type OpFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
pub type OpFnOpt = Option<OpFn>;

/// A `ResultArray` embedded at the start of a 512-byte, 8-byte-aligned buffer.
///
/// Used by CONFIGS.md row C24, where the `FOREACH` macro is driven with
/// `count > 10` and therefore walks one `Result` past `data[10]`. The extra
/// slack means both libraries touch real, comparable memory instead of
/// scribbling on the stack.
#[repr(C, align(8))]
pub struct PaddedArray {
    pub bytes: [u8; 512],
}

impl PaddedArray {
    pub fn new_filled(byte: u8) -> Box<Self> {
        Box::new(PaddedArray { bytes: [byte; 512] })
    }
    pub fn as_arr_ptr(&mut self) -> *mut CResultArray {
        self.bytes.as_mut_ptr() as *mut CResultArray
    }
    /// Field-wise element write (never touches struct padding, exactly like the
    /// C code's `item->value = …` / `item->scaled = …` assignments).
    pub fn set_elem(&mut self, i: usize, value: i32, scaled: f64, rank: i32) {
        let base = i * SIZE_OF_RESULT;
        self.bytes[base..base + 4].copy_from_slice(&value.to_ne_bytes());
        self.bytes[base + 8..base + 16].copy_from_slice(&scaled.to_bits().to_ne_bytes());
        self.bytes[base + 16..base + 20].copy_from_slice(&rank.to_ne_bytes());
    }
    pub fn set_count(&mut self, count: i32) {
        self.bytes[OFFSET_OF_COUNT..OFFSET_OF_COUNT + 4].copy_from_slice(&count.to_ne_bytes());
    }
}

// ---------------------------------------------------------------------------
// Loaded API surface — all 11 exported symbols
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub add_operation: OpFn,
    pub multiply_operation: OpFn,
    pub subtract_operation: OpFn,
    pub modulo_operation: OpFn,
    pub safe_double_to_int: unsafe extern "C" fn(f64) -> i32,
    pub compute_scaled_value: unsafe extern "C" fn(i32, f64) -> i32,
    pub compare_results_in_array: unsafe extern "C" fn(*mut CResultArray, i32, i32) -> i32,
    pub init_result_array: unsafe extern "C" fn(*mut CResultArray, *const i32, i32),
    pub process_with_foreach: unsafe extern "C" fn(*mut CResultArray, OpFnOpt) -> i32,
    pub compute_weighted_sum: unsafe extern "C" fn(*mut CResultArray) -> i32,
    pub arrayfunc: unsafe extern "C" fn(i32, i32, i32, i32) -> i32,
}

/// The 11 symbol names the C `.so` exports, in `nm -D` order.
pub const C_SYMBOLS: &[&str] = &[
    "add_operation",
    "multiply_operation",
    "subtract_operation",
    "modulo_operation",
    "safe_double_to_int",
    "compute_scaled_value",
    "compare_results_in_array",
    "init_result_array",
    "process_with_foreach",
    "compute_weighted_sum",
    "arrayfunc",
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libtranslated_rust.so` — CMake names the target after the
/// directory *above* `c_src`, which happens to be `translated_rust`.
pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().join("c_src").join("build");
    for name in [
        "libtranslated_rust.so",
        "libc_src.so",
        "libtranslated_rust.dylib",
    ] {
        let p = build.join(name);
        if p.is_file() {
            return p;
        }
    }
    // Fall back to whatever single .so is in the build directory.
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                return p;
            }
        }
    }
    panic!(
        "C shared library not found in {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
}

/// True when the running test binary came out of a `release`-profile build.
fn running_release_profile() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|e| {
            e.ancestors()
                .find(|a| {
                    matches!(
                        a.file_name().and_then(|f| f.to_str()),
                        Some("debug") | Some("release")
                    )
                })
                .and_then(|a| a.file_name().and_then(|f| f.to_str()).map(str::to_owned))
        })
        .map(|p| p == "release")
        .unwrap_or(false)
}

/// Newest mtime across everything that can change the cdylib's behaviour.
fn newest_source_mtime() -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut bump = |p: PathBuf| {
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if newest.is_none_or(|n| t > n) {
                newest = Some(t);
            }
        }
    };
    bump(manifest_dir().join("Cargo.toml"));
    let mut stack = vec![manifest_dir().join("src")];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    bump(p);
                }
            }
        }
    }
    newest
}

/// An artifact is only trustworthy if it is at least as new as every source file.
fn is_fresh(so: &Path) -> bool {
    let Ok(so_t) = std::fs::metadata(so).and_then(|m| m.modified()) else {
        return false;
    };
    match newest_source_mtime() {
        Some(src_t) => so_t >= src_t,
        None => true,
    }
}

/// Compile the cdylib into a private target directory so that the `.so` under
/// test is *always* built from the current sources.
///
/// `cargo test` does **not** build `crate-type = ["cdylib"]` artifacts, so a
/// plain `cargo test` would otherwise silently `dlopen` whatever stale `.so` an
/// earlier `cargo build` happened to leave in `target/`. A private
/// `CARGO_TARGET_DIR` keeps this nested build from contending for the lock the
/// outer cargo holds on the main target directory.
fn build_fresh_cdylib() -> Option<PathBuf> {
    let manifest = manifest_dir();
    let target_dir = manifest.join("target").join("dylib-under-test");
    let release = running_release_profile();
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    // Try offline first (fast, and the common case in sandboxes), then online.
    for offline in [true, false] {
        let mut cmd = std::process::Command::new(&cargo);
        cmd.arg("build")
            .arg("--lib")
            .arg("--manifest-path")
            .arg(manifest.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &target_dir)
            .env_remove("RUSTC_WRAPPER")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        if release {
            cmd.arg("--release");
        }
        if offline {
            cmd.arg("--offline");
        }
        if let Ok(out) = cmd.output() {
            if out.status.success() {
                let p = target_dir
                    .join(if release { "release" } else { "debug" })
                    .join("libarrayfunc_lib.so");
                if p.is_file() {
                    return Some(p);
                }
            } else if !offline {
                eprintln!(
                    "nested `cargo build --lib` failed:\n{}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
    }
    None
}

/// Path to the Rust cdylib under test.
///
/// Prefers the canonical `target/<profile>/libarrayfunc_lib.so` when it is newer
/// than every source file; otherwise builds a guaranteed-fresh copy. A stale
/// artifact is never used, because testing a stale `.so` silently reports
/// success for code that is no longer there.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut dir: &Path = exe.parent().expect("test exe has a parent");
    for _ in 0..3 {
        candidates.push(dir.join("libarrayfunc_lib.so"));
        match dir.parent() {
            Some(d) => dir = d,
            None => break,
        }
    }

    for p in &candidates {
        if p.is_file() && is_fresh(p) {
            return p.clone();
        }
    }

    if let Some(p) = build_fresh_cdylib() {
        return p;
    }

    let stale: Vec<&PathBuf> = candidates.iter().filter(|p| p.is_file()).collect();
    panic!(
        "could not obtain an up-to-date Rust cdylib.\n\
         Stale artifacts found: {stale:?}\n\
         Searched near: {}\n\
         Build it with `cargo build` (note that `cargo test` alone does NOT build \
         cdylib targets), or run ./verify_all.sh",
        exe.display()
    );
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol `{}`: {e}", $name));
        *s
    }};
}

fn load(path: &Path, name: &'static str) -> Api {
    // Leak the handle: the resolved function pointers must outlive it.
    let lib: &'static Library = Box::leak(Box::new(
        unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display())),
    ));

    Api {
        name,
        add_operation: sym!(lib, OpFn, "add_operation"),
        multiply_operation: sym!(lib, OpFn, "multiply_operation"),
        subtract_operation: sym!(lib, OpFn, "subtract_operation"),
        modulo_operation: sym!(lib, OpFn, "modulo_operation"),
        safe_double_to_int: sym!(lib, unsafe extern "C" fn(f64) -> i32, "safe_double_to_int"),
        compute_scaled_value: sym!(
            lib,
            unsafe extern "C" fn(i32, f64) -> i32,
            "compute_scaled_value"
        ),
        compare_results_in_array: sym!(
            lib,
            unsafe extern "C" fn(*mut CResultArray, i32, i32) -> i32,
            "compare_results_in_array"
        ),
        init_result_array: sym!(
            lib,
            unsafe extern "C" fn(*mut CResultArray, *const i32, i32),
            "init_result_array"
        ),
        process_with_foreach: sym!(
            lib,
            unsafe extern "C" fn(*mut CResultArray, OpFnOpt) -> i32,
            "process_with_foreach"
        ),
        compute_weighted_sum: sym!(
            lib,
            unsafe extern "C" fn(*mut CResultArray) -> i32,
            "compute_weighted_sum"
        ),
        arrayfunc: sym!(lib, unsafe extern "C" fn(i32, i32, i32, i32) -> i32, "arrayfunc"),
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c() -> &'static Api {
    C_API.get_or_init(|| load(&c_so_path(), "C"))
}

pub fn rs() -> &'static Api {
    RUST_API.get_or_init(|| load(&rust_so_path(), "Rust"))
}

/// `(c_api, rust_api)`
pub fn both() -> (&'static Api, &'static Api) {
    (c(), rs())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible everywhere
// ---------------------------------------------------------------------------

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// An `i32` drawn from a distribution that favours interesting magnitudes.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => self.range_i32(-3, 3),
            3 => self.range_i32(-1000, 1000),
            4 => self.range_i32(-(1 << 20), 1 << 20),
            5 => self.range_i32(i32::MIN / 2, i32::MIN / 2 + 64),
            6 => self.range_i32(i32::MAX / 2 - 64, i32::MAX / 2),
            _ => self.next_i32(),
        }
    }
    /// Any `f64`, including NaNs and infinities (raw bit pattern).
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A finite `f64` roughly inside / around the `i32` range.
    pub fn nearby_f64(&mut self) -> f64 {
        match self.next_u64() % 6 {
            0 => self.next_i32() as f64,
            1 => self.next_i32() as f64 + 0.5,
            2 => self.next_i32() as f64 - 0.5,
            3 => (self.next_i32() as f64) * 1.0000001,
            4 => (self.next_u32() as f64) / 7.0 - 3e9,
            _ => (self.next_i32() as f64) / 3.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_i32(what: &str, ctx: impl std::fmt::Debug, cv: i32, rv: i32) {
    assert_eq!(
        cv, rv,
        "{what}{ctx:?}: C returned {cv} but Rust returned {rv}"
    );
}

#[track_caller]
pub fn eq_arrays(what: &str, ctx: impl std::fmt::Debug, ca: &CResultArray, ra: &CResultArray) {
    let (cc, cd) = ca.snapshot();
    let (rc, rd) = ra.snapshot();
    assert_eq!(cc, rc, "{what}{ctx:?}: count C={cc} Rust={rc}");
    for i in 0..10 {
        assert_eq!(
            cd[i], rd[i],
            "{what}{ctx:?}: element {i} differs: C=(value={}, scaled_bits={:#018x}, rank={}) \
             Rust=(value={}, scaled_bits={:#018x}, rank={})",
            cd[i].0, cd[i].1, cd[i].2, rd[i].0, rd[i].1, rd[i].2
        );
    }
}
