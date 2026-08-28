//! Shared harness: loads the C `.so` and the Rust `.so` via `libloading` and
//! compares `tfm` outputs bit-for-bit.
//!
//! Nothing here calls into the Rust crate directly; both implementations are
//! reached only through their exported C ABI symbols.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Signature of the exported function under test.
pub type TfmFn = unsafe extern "C" fn(*mut f32, *const f32, std::ffi::c_int);

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locates the C shared library produced by CMake.
///
/// The CMake project name is derived from the parent directory name, so the
/// file name is not fixed; scan `c_src/build` for the single `.so`. Override
/// with `C_TFM_SO` to compare against a differently-configured C build.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_TFM_SO") {
        return PathBuf::from(p);
    }
    let build = root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {}; build it with cmake first",
        build.display()
    );
    found.remove(0)
}

/// Locates the Rust `cdylib` for the profile the tests were built with.
///
/// `cargo test` does not itself emit the `cdylib` artifact, so the tests rely on
/// a preceding `cargo build` (see `run_all_tests.sh`). The search prefers the
/// profile directory of the running test binary and then falls back to the other
/// profiles.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_TFM_SO") {
        return PathBuf::from(p);
    }
    // The test executable lives in `target/<profile>/deps/`, so the cdylib
    // built for the same profile sits one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>/deps/<exe>")
        .to_path_buf();

    let target_dir = profile_dir.parent().unwrap_or(&profile_dir).to_path_buf();
    let candidates = [
        profile_dir.join("libtfm_lib.so"),
        target_dir.join("release/libtfm_lib.so"),
        target_dir.join("debug/libtfm_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found. Run `cargo build` (and/or `cargo build --release`) \
         first, or set RUST_TFM_SO. Looked in: {:?}",
        candidates
    );
}

/// Both libraries, kept alive for the duration of a test.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_tfm: TfmFn,
    pub rust_tfm: TfmFn,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("load C .so");
            let rust_lib = Library::new(rust_so_path()).expect("load Rust .so");
            let c_sym: Symbol<TfmFn> = c_lib.get(b"tfm\0").expect("C exports tfm");
            let rust_sym: Symbol<TfmFn> = rust_lib.get(b"tfm\0").expect("Rust exports tfm");
            let c_tfm = *c_sym;
            let rust_tfm = *rust_sym;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_tfm,
                rust_tfm,
            }
        }
    }
}

/// Sentinel written into destination slots that `tfm` must not touch.
const SENTINEL: u32 = 0xDEAD_BEEF;

/// Runs both implementations over `src` with the given `count` and asserts the
/// destination buffers are bit-identical.
///
/// `dest_len` is the number of `f32` slots actually allocated; slots at or past
/// `2 * count` are pre-filled with a sentinel so out-of-range writes are caught.
pub fn check_with(pair: &Pair, label: &str, src: &[f32], count: i32, dest_len: usize) {
    let mut c_dest = vec![f32::from_bits(SENTINEL); dest_len];
    let mut rust_dest = vec![f32::from_bits(SENTINEL); dest_len];

    unsafe {
        (pair.c_tfm)(c_dest.as_mut_ptr(), src.as_ptr(), count);
        (pair.rust_tfm)(rust_dest.as_mut_ptr(), src.as_ptr(), count);
    }

    compare(label, src, count, &c_dest, &rust_dest);
}

/// Same as [`check_with`] but sizes the destination from `count` (plus a couple
/// of guard slots).
pub fn check(pair: &Pair, label: &str, src: &[f32], count: i32) {
    let needed = if count > 0 { count as usize * 2 } else { 0 };
    check_with(pair, label, src, count, needed + 2);
}

/// Convenience wrapper for a single 3-float entry.
pub fn check_one(pair: &Pair, label: &str, entry: [f32; 3]) {
    check(pair, label, &entry, 1);
}

fn compare(label: &str, src: &[f32], count: i32, c_dest: &[f32], rust_dest: &[f32]) {
    assert_eq!(c_dest.len(), rust_dest.len());
    for i in 0..c_dest.len() {
        let cb = c_dest[i].to_bits();
        let rb = rust_dest[i].to_bits();
        if cb != rb {
            panic!(
                "{label}: mismatch at dest[{i}] (count={count})\n  \
                 C    = {:?} (0x{cb:08x})\n  \
                 Rust = {:?} (0x{rb:08x})\n  \
                 src  = {}",
                c_dest[i],
                rust_dest[i],
                fmt_src(src),
            );
        }
    }
}

fn fmt_src(src: &[f32]) -> String {
    let shown: Vec<String> = src
        .iter()
        .take(24)
        .map(|v| format!("{v:?}/0x{:08x}", v.to_bits()))
        .collect();
    let mut s = shown.join(", ");
    if src.len() > 24 {
        s.push_str(", ...");
    }
    s
}

/// Small deterministic PRNG so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u32(&mut self) -> u32 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 16) as u32
    }

    /// Uniform random bit pattern reinterpreted as `f32` (yields NaNs, infs,
    /// subnormals and huge magnitudes).
    pub fn next_f32_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// "Reasonable" magnitude float in `[-scale, scale)`.
    pub fn next_f32_scaled(&mut self, scale: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        (u * 2.0 - 1.0) * scale
    }
}

/// Interesting single-float values that stress the branch and the arithmetic.
pub const EDGE_VALUES: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    1e-30,
    -1e-30,
    1e30,
    -1e30,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::from_bits(1),  // smallest subnormal
    f32::from_bits(0x8000_0001),
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::from_bits(0x7F80_0001), // signalling NaN
    f32::from_bits(0x7FC0_1234), // quiet NaN with payload
];
