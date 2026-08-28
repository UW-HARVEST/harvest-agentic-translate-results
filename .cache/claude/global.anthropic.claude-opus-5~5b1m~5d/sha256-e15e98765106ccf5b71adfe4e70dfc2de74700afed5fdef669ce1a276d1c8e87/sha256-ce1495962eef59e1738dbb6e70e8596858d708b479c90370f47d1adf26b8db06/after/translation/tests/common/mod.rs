//! Shared differential-test harness.
//!
//! Both the C and the Rust implementation are loaded **as shared libraries** via
//! `libloading`; no Rust function is ever called directly, so the `#[no_mangle]`
//! export wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_double, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI types — redeclared here exactly as an external C consumer would.
// ---------------------------------------------------------------------------

/// `typedef int (*operation_func)(int, int, int, int);`
pub type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// ```c
/// typedef struct { int value; double scaled; int rank; } Result;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Result_ {
    pub value: c_int,
    pub scaled: c_double,
    pub rank: c_int,
}

/// ```c
/// typedef struct { Result data[10]; int count; } ResultArray;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ResultArray {
    pub data: [Result_; 10],
    pub count: c_int,
}

impl ResultArray {
    /// A deterministic, fully-defined starting state. Using non-zero garbage
    /// everywhere means "the callee left this byte alone" is observable.
    pub fn dirty(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut data = [Result_ {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }; 10];
        for (i, slot) in data.iter_mut().enumerate() {
            slot.value = rng.next_i32();
            slot.scaled = f64::from_bits(rng.next_u64());
            slot.rank = (i as c_int).wrapping_mul(7).wrapping_sub(3);
        }
        ResultArray {
            data,
            count: -12345,
        }
    }

    /// Byte-for-byte serialisation of every **defined** byte of the struct: all
    /// ten elements (`value`, the raw bit pattern of `scaled`, `rank`) plus
    /// `count`. The four padding holes per element and the trailing hole are
    /// deliberately excluded — C leaves them indeterminate, so comparing them
    /// would test the compilers' scratch memory rather than the translation.
    /// Everything the C language actually defines is covered.
    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(10 * 16 + 4);
        for e in &self.data {
            out.extend_from_slice(&e.value.to_le_bytes());
            out.extend_from_slice(&e.scaled.to_bits().to_le_bytes());
            out.extend_from_slice(&e.rank.to_le_bytes());
        }
        out.extend_from_slice(&self.count.to_le_bytes());
        out
    }

    pub fn describe(&self) -> String {
        let mut s = format!("count={}", self.count);
        for (i, e) in self.data.iter().enumerate() {
            s += &format!(
                "\n  [{}] value={} scaled=0x{:016x} ({:?}) rank={}",
                i,
                e.value,
                e.scaled.to_bits(),
                e.scaled,
                e.rank
            );
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style tests)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid the zero fixed point of xorshift64*.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// An `int` biased towards the interesting magnitudes (0, ±1, extremes,
    /// powers of two) rather than uniform over 2^32.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.next_u64() % 10 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => self.range_i32(-16, 16),
            6 => 1i32 << (self.next_u64() % 31),
            7 => -(1i32 << (self.next_u64() % 31)),
            8 => self.range_i32(-100_000, 100_000),
            _ => self.next_i32(),
        }
    }

    /// A `double` biased towards the classes `safe_double_to_int` branches on.
    pub fn interesting_f64(&mut self) -> f64 {
        match self.next_u64() % 16 {
            0 => 0.0,
            1 => -0.0,
            2 => f64::NAN,
            3 => -f64::NAN,
            4 => f64::INFINITY,
            5 => f64::NEG_INFINITY,
            6 => i32::MAX as f64,
            7 => i32::MIN as f64,
            8 => i32::MAX as f64 + 1.0,
            9 => i32::MIN as f64 - 1.0,
            10 => i32::MAX as f64 - 0.5,
            11 => i32::MIN as f64 + 0.5,
            12 => f64::from_bits(self.next_u64()),
            13 => self.next_i32() as f64 + 0.5,
            14 => (self.next_i32() as f64) / 3.0,
            _ => f64::MIN_POSITIVE * (self.next_u32() as f64),
        }
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn so_files_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v
}

/// Locates the C `.so`, building it with cmake on first use if necessary, so
/// that a bare `cargo test` works from a clean checkout.
pub fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");

    // Always (re)configure and rebuild: cmake is a no-op when nothing changed,
    // and this guarantees the C `.so` under test is never a stale artifact.
    // Nothing in `c_src/` other than its own `build/` output directory is
    // touched.
    let _ = std::fs::create_dir_all(&build);
    let configure = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .output();
    let built = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .output();

    so_files_in(&build).pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {} and the automatic cmake build failed.\n\
             Build it manually:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             configure: {:?}\nbuild: {:?}",
            build.display(),
            configure.map(|o| String::from_utf8_lossy(&o.stderr).into_owned()),
            built.map(|o| String::from_utf8_lossy(&o.stderr).into_owned()),
        )
    })
}

/// Builds the cdylib for the given profile (always — `cargo build` is a no-op
/// when nothing changed, and this guarantees the `.so` under test is never a
/// stale artifact from an earlier edit) and returns its path.
///
/// The `.so` is then loaded through `dlopen` like any other C library; the Rust
/// code is never called directly, so the `#[no_mangle]` export wrappers are part
/// of what is being tested.
fn build_cdylib(target_dir: &Path, release: bool) -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = target_dir.join(if release { "release" } else { "debug" });
    let names = ["libarrayfunc_lib.so", "libtranslation.so"];
    let probe = || names.iter().map(|n| out_dir.join(n)).find(|p| p.exists());

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(&cargo);
    cmd.current_dir(manifest)
        .arg("build")
        .arg("--offline")
        .arg("--target-dir")
        .arg(target_dir);
    if release {
        cmd.arg("--release");
    }
    let out = cmd.output();

    probe().unwrap_or_else(|| {
        panic!(
            "cdylib not found in {} and `cargo build{}` did not produce it.\n{:?}",
            out_dir.display(),
            if release { " --release" } else { "" },
            out.map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
        )
    })
}

/// The target directory and profile this test binary was compiled into.
fn own_target_dir_and_profile() -> (PathBuf, bool) {
    // The test binary lives in <target>/<profile>/deps/.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let release = dir.file_name().map(|n| n == "release").unwrap_or(false);
    let target_dir = dir.parent().unwrap_or(&dir).to_path_buf();
    (target_dir, release)
}

/// The cdylib matching this test binary's own profile.
pub fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let (target_dir, release) = own_target_dir_and_profile();
    build_cdylib(&target_dir, release)
}

// NOTE on feature combinations: the bootstrap build above uses the crate's
// default feature set. `check_features.sh` drives the per-combination runs by
// building the cdylib for each combination itself and exporting `RUST_SO_PATH`,
// so the bootstrap path never picks the wrong `.so`.

/// The full set of exports of one implementation, resolved through `dlsym`.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,

    pub add_operation: OperationFunc,
    pub multiply_operation: OperationFunc,
    pub subtract_operation: OperationFunc,
    pub modulo_operation: OperationFunc,
    pub safe_double_to_int: unsafe extern "C" fn(c_double) -> c_int,
    pub compute_scaled_value: unsafe extern "C" fn(c_int, c_double) -> c_int,
    pub compare_results_in_array: unsafe extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int,
    pub init_result_array: unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int),
    pub process_with_foreach:
        unsafe extern "C" fn(*mut ResultArray, Option<OperationFunc>) -> c_int,
    pub compute_weighted_sum: unsafe extern "C" fn(*mut ResultArray) -> c_int,
    pub arrayfunc: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

/// Every symbol `nm -D` reports on the C `.so`, in declaration order.
pub const EXPECTED_SYMBOLS: &[&str] = &[
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

unsafe fn get<T: Copy>(lib: &Library, name: &str, path: &Path) -> T {
    let sym: Symbol<T> = lib
        .get(format!("{name}\0").as_bytes())
        .unwrap_or_else(|e| panic!("{} does not export `{name}`: {e}", path.display()));
    *sym
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Self {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
            let me = Impl {
                name,
                add_operation: get(&lib, "add_operation", &path),
                multiply_operation: get(&lib, "multiply_operation", &path),
                subtract_operation: get(&lib, "subtract_operation", &path),
                modulo_operation: get(&lib, "modulo_operation", &path),
                safe_double_to_int: get(&lib, "safe_double_to_int", &path),
                compute_scaled_value: get(&lib, "compute_scaled_value", &path),
                compare_results_in_array: get(&lib, "compare_results_in_array", &path),
                init_result_array: get(&lib, "init_result_array", &path),
                process_with_foreach: get(&lib, "process_with_foreach", &path),
                compute_weighted_sum: get(&lib, "compute_weighted_sum", &path),
                arrayfunc: get(&lib, "arrayfunc", &path),
                path,
                _lib: lib,
            };
            me
        }
    }

    /// The four built-in operations of *this* library, in `arrayfunc` order.
    pub fn ops(&self) -> [(&'static str, OperationFunc); 4] {
        [
            ("add", self.add_operation),
            ("multiply", self.multiply_operation),
            ("subtract", self.subtract_operation),
            ("modulo", self.modulo_operation),
        ]
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static PAIR_RELEASE: OnceLock<Pair> = OnceLock::new();

/// Loads (once per test binary) both shared libraries.
pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", find_c_so()),
        rust: Impl::load("Rust", find_rust_so()),
    })
}

/// Same pair, but pinned to the **release** Rust cdylib.
///
/// Needed by the signal-parity probes: a debug cdylib carries rustc's optional
/// UB instrumentation (`-C debug-assertions`), which converts a null-pointer
/// dereference into `abort()` (SIGABRT) before the hardware fault (SIGSEGV) can
/// happen.  CMake builds the C library without any comparable instrumentation,
/// so the uninstrumented release artifact — the one an external consumer
/// actually links against — is the correct counterpart when comparing *how* the
/// process dies.  Every defined-behaviour comparison runs against both profiles.
pub fn libs_release() -> &'static Pair {
    PAIR_RELEASE.get_or_init(|| Pair {
        c: Impl::load("C", find_c_so()),
        rust: Impl::load("Rust", find_rust_so_release()),
    })
}

/// The **release** cdylib, rebuilt if out of date.
pub fn find_rust_so_release() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_RELEASE_PATH") {
        return PathBuf::from(p);
    }
    let (target_dir, _) = own_target_dir_and_profile();
    build_cdylib(&target_dir, true)
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_int(ctx: &str, c: c_int, rust: c_int) {
    assert_eq!(
        c, rust,
        "return value mismatch in {ctx}\n  C    = {c}\n  Rust = {rust}"
    );
}

#[track_caller]
pub fn eq_array(ctx: &str, c: &ResultArray, rust: &ResultArray) {
    if c.snapshot() != rust.snapshot() {
        panic!(
            "ResultArray mismatch in {ctx}\n--- C ---\n{}\n--- Rust ---\n{}",
            c.describe(),
            rust.describe()
        );
    }
}
