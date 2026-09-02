//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes their exported symbols side by side.
//!
//! Nothing here ever calls a Rust function directly — every Rust call goes
//! through `dlsym` on `libarrayfunc_lib.so`, exactly like an external C caller,
//! so the `#[no_mangle]` / `extern "C"` wrappers are under test too.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_double, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C ABI types (mirrors of the C structs in c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Result {
    pub value: c_int,
    pub scaled: c_double,
    pub rank: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ResultArray {
    pub data: [Result; 10],
    pub count: c_int,
}

impl ResultArray {
    /// Deterministic non-zero fill so that untouched slots are still compared.
    pub fn poisoned() -> Self {
        ResultArray {
            data: [Result {
                value: -559038737, // 0xDEADBEEF
                scaled: f64::from_bits(0xDEAD_BEEF_DEAD_BEEF),
                rank: -1,
            }; 10],
            count: 0,
        }
    }

    pub fn zeroed() -> Self {
        ResultArray {
            data: [Result {
                value: 0,
                scaled: 0.0,
                rank: 0,
            }; 10],
            count: 0,
        }
    }

    /// Observable content of the struct, padding excluded, as a byte string.
    /// `scaled` goes in as its raw IEEE-754 bit pattern so NaN payloads and
    /// signed zeros are compared bit-for-bit rather than by `==`.
    pub fn observable_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(10 * 16 + 4);
        for r in self.data.iter() {
            out.extend_from_slice(&r.value.to_le_bytes());
            out.extend_from_slice(&r.scaled.to_bits().to_le_bytes());
            out.extend_from_slice(&r.rank.to_le_bytes());
        }
        out.extend_from_slice(&self.count.to_le_bytes());
        out
    }

    /// Raw 248-byte image, padding included.
    pub fn raw_bytes(&self) -> Vec<u8> {
        let p = self as *const ResultArray as *const u8;
        unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<ResultArray>()) }.to_vec()
    }
}

// ---------------------------------------------------------------------------
// Loaded library handles
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub add_operation: OperationFunc,
    pub multiply_operation: OperationFunc,
    pub subtract_operation: OperationFunc,
    pub modulo_operation: OperationFunc,
    pub safe_double_to_int: extern "C" fn(c_double) -> c_int,
    pub compute_scaled_value: extern "C" fn(c_int, c_double) -> c_int,
    pub compare_results_in_array: extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int,
    pub init_result_array: extern "C" fn(*mut ResultArray, *mut c_int, c_int),
    pub process_with_foreach: extern "C" fn(*mut ResultArray, OperationFunc) -> c_int,
    pub compute_weighted_sum: extern "C" fn(*mut ResultArray) -> c_int,
    pub arrayfunc: extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

/// The four `operations[]` entries, in the order `arrayfunc` uses them.
impl Lib {
    pub fn operations(&self) -> [OperationFunc; 4] {
        [
            self.add_operation,
            self.multiply_operation,
            self.subtract_operation,
            self.modulo_operation,
        ]
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_so(dir: &PathBuf, must_contain: Option<&str>) -> PathBuf {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .filter(|p| match must_contain {
            None => true,
            Some(s) => p.file_name().unwrap().to_string_lossy().contains(s),
        })
        .collect();
    candidates.sort();
    match candidates.len() {
        0 => panic!(
            "no .so found in {} (build it first: see README of this test suite)",
            dir.display()
        ),
        _ => candidates.remove(0),
    }
}

pub fn c_so_path() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    find_so(&dir, None)
}

/// Locates the Rust `cdylib` belonging to the SAME profile as this test binary.
///
/// The profile is derived from `current_exe()` (`<target>/<profile>/deps/<bin>`)
/// rather than from `cfg!(debug_assertions)`, because this crate deliberately
/// disables debug-assertions in every profile (see Cargo.toml), which would make
/// a `cfg`-based guess silently pick the wrong artifact.
/// Overridable with `HARVEST_RUST_SO`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    const LIB: &str = "libarrayfunc_lib.so";

    // <target>/<profile>/deps/<test-bin>  ->  <target>/<profile>/<LIB>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            let cand = profile_dir.join(LIB);
            if cand.exists() {
                return cand;
            }
        }
    }
    // Fallback for unusual layouts.
    let target = workspace_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let cand = target.join(profile).join(LIB);
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "{LIB} not found next to {:?} nor under {} — run `cargo build` first",
        std::env::current_exe(),
        target.display()
    );
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol `{}`: {e}", $name));
        *s
    }};
}

fn load(name: &'static str, path: PathBuf) -> Lib {
    // Leaked on purpose: the function pointers must stay valid for the whole
    // test process. RTLD_LOCAL (libloading's default) keeps the two libraries'
    // identically-named symbols from interposing on each other.
    let lib: &'static Library = Box::leak(Box::new(
        unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display())),
    ));

    Lib {
        name,
        path: path.clone(),
        add_operation: sym!(lib, OperationFunc, "add_operation"),
        multiply_operation: sym!(lib, OperationFunc, "multiply_operation"),
        subtract_operation: sym!(lib, OperationFunc, "subtract_operation"),
        modulo_operation: sym!(lib, OperationFunc, "modulo_operation"),
        safe_double_to_int: sym!(lib, extern "C" fn(c_double) -> c_int, "safe_double_to_int"),
        compute_scaled_value: sym!(
            lib,
            extern "C" fn(c_int, c_double) -> c_int,
            "compute_scaled_value"
        ),
        compare_results_in_array: sym!(
            lib,
            extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int,
            "compare_results_in_array"
        ),
        init_result_array: sym!(
            lib,
            extern "C" fn(*mut ResultArray, *mut c_int, c_int),
            "init_result_array"
        ),
        process_with_foreach: sym!(
            lib,
            extern "C" fn(*mut ResultArray, OperationFunc) -> c_int,
            "process_with_foreach"
        ),
        compute_weighted_sum: sym!(
            lib,
            extern "C" fn(*mut ResultArray) -> c_int,
            "compute_weighted_sum"
        ),
        arrayfunc: sym!(lib, extern "C" fn(c_int, c_int, c_int, c_int) -> c_int, "arrayfunc"),
    }
}

static FRESHNESS: OnceLock<()> = OnceLock::new();

/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact, so a
/// stale `libarrayfunc_lib.so` would silently make every differential test
/// vacuous (it would compare C against an old Rust build). Refuse to run if the
/// `.so` is older than the sources.
fn assert_so_is_fresh() {
    FRESHNESS.get_or_init(|| {
        let so = rust_so_path();
        let so_mtime = std::fs::metadata(&so)
            .and_then(|m| m.modified())
            .expect("stat rust .so");
        let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
        for e in std::fs::read_dir(&src_dir).expect("read src/").flatten() {
            if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().map(|(_, t)| m > *t).unwrap_or(true) {
                    newest = Some((e.path(), m));
                }
            }
        }
        if let Some((p, t)) = newest {
            assert!(
                so_mtime >= t,
                "STALE ARTIFACT: {} is older than {}.\n\
                 `cargo test` does not rebuild a cdylib — run \
                 `cargo build --release` (or use ./run_verification.sh) first, \
                 otherwise the differential tests compare C against an old Rust build.",
                so.display(),
                p.display()
            );
        }
    });
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c() -> &'static Lib {
    C_LIB.get_or_init(|| load("C", c_so_path()))
}

pub fn r() -> &'static Lib {
    assert_so_is_fresh();
    RUST_LIB.get_or_init(|| load("RUST", rust_so_path()))
}

/// Both libraries, C first.
pub fn both() -> (&'static Lib, &'static Lib) {
    (c(), r())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

pub struct Rng(u64);

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
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as i32
    }
    /// Biased toward interesting small values / boundaries a third of the time.
    pub fn next_i32_spicy(&mut self) -> c_int {
        match self.next_u64() % 12 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => i32::MAX / 2,
            6 => i32::MIN / 2,
            7 => (self.next_u64() % 21) as i32 - 10,
            _ => self.next_i32(),
        }
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A finite double strictly inside the i32 range.
    pub fn f64_in_int_range(&mut self) -> f64 {
        let frac = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let mag = frac * 2147483646.0;
        sign * mag
    }
    pub fn f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}

// ---------------------------------------------------------------------------
// Interesting constants
// ---------------------------------------------------------------------------

pub const INT_MAX_D: f64 = 2147483647.0;
pub const INT_MIN_D: f64 = -2147483648.0;

pub fn boundary_i32() -> [c_int; 5] {
    [i32::MIN, -1, 0, 1, i32::MAX]
}

pub fn extreme_scales() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.5,
        0.333,
        0.75,
        0.8,
        1e10,
        -1e10,
        1e300,
        -1e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MIN_POSITIVE,
        5e-324, // smallest subnormal
        -5e-324,
        f64::MAX,
        f64::MIN,
        2147483647.0,
        -2147483648.0,
    ]
}

// ---------------------------------------------------------------------------
// Differential assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_int(ctx: &str, cv: c_int, rv: c_int) {
    assert_eq!(
        cv, rv,
        "return value diverged [{ctx}]: C={cv} ({cv:#010x}) RUST={rv} ({rv:#010x})"
    );
}

#[track_caller]
pub fn eq_struct(ctx: &str, ca: &ResultArray, ra: &ResultArray) {
    let cb = ca.observable_bytes();
    let rb = ra.observable_bytes();
    if cb != rb {
        let mut msg = format!("ResultArray diverged [{ctx}]\n  count: C={} RUST={}\n", ca.count, ra.count);
        for i in 0..10 {
            let (x, y) = (&ca.data[i], &ra.data[i]);
            if x.value != y.value || x.scaled.to_bits() != y.scaled.to_bits() || x.rank != y.rank {
                msg += &format!(
                    "  data[{i}]: C={{value:{}, scaled_bits:{:#018x} ({}), rank:{}}} \
                     RUST={{value:{}, scaled_bits:{:#018x} ({}), rank:{}}}\n",
                    x.value, x.scaled.to_bits(), x.scaled, x.rank,
                    y.value, y.scaled.to_bits(), y.scaled, y.rank
                );
            }
        }
        panic!("{msg}");
    }
}
