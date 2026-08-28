//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported C symbols. The Rust code is *never*
//! called directly as a Rust function — the `#[no_mangle] extern "C"` wrapper
//! is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type Memchra2Fn = unsafe extern "C" fn(
    ::std::os::raw::c_int,
    ::std::os::raw::c_int,
    ::std::os::raw::c_int,
    ::std::os::raw::c_int,
) -> ::std::os::raw::c_int;

/// Repository root (the directory that holds both `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so_in(dir: &Path, wanted_stem_contains: Option<&str>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Option<PathBuf> = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("so") {
            continue;
        }
        let name = p.file_name()?.to_str()?.to_string();
        if let Some(pat) = wanted_stem_contains {
            if !name.contains(pat) {
                continue;
            }
        }
        // Prefer the newest file if several match.
        let take = match &found {
            None => true,
            Some(prev) => {
                let a = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
                let b = std::fs::metadata(prev).and_then(|m| m.modified()).ok();
                match (a, b) {
                    (Some(a), Some(b)) => a > b,
                    _ => false,
                }
            }
        };
        if take {
            found = Some(p);
        }
    }
    found
}

/// Locate the C shared library built by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent directory name*, so the
/// artifact file name is not fixed; discover it instead of hard-coding it.
pub fn c_so_path() -> PathBuf {
    let root = repo_root();
    let candidates = [
        root.join("c_src/build"),
        root.join("c_src/build/lib"),
        root.join("c_src/build/Debug"),
        root.join("c_src/build/Release"),
    ];
    for dir in &candidates {
        if let Some(p) = find_so_in(dir, None) {
            return p;
        }
    }
    panic!(
        "could not find the C .so. Build it with:\n  cd {} && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        root.join("c_src").display()
    );
}

/// Locate the Rust `cdylib` (`libmemchra2_lib.so`) for the profile the test
/// binary itself was built with.
pub fn rust_so_path() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>
    if let Ok(exe) = std::env::current_exe() {
        let deps = exe.parent().map(Path::to_path_buf);
        if let Some(deps) = deps {
            if let Some(p) = find_so_in(&deps, Some("memchra2_lib")) {
                return p;
            }
            if let Some(profile_dir) = deps.parent() {
                if let Some(p) = find_so_in(profile_dir, Some("memchra2_lib")) {
                    return p;
                }
            }
        }
    }
    let root = repo_root();
    for prof in ["release", "debug"] {
        let dir = root.join("translation/target").join(prof);
        if let Some(p) = find_so_in(&dir, Some("memchra2_lib")) {
            return p;
        }
    }
    panic!("could not find libmemchra2_lib.so — run `cargo build` first");
}

pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_memchra2: Memchra2Fn,
    pub rust_memchra2: Memchra2Fn,
}

// The function pointers are plain C functions with no thread-affine state and
// the `Library` handles stay alive for the whole process.
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

impl Pair {
    fn load() -> Pair {
        unsafe {
            let c_path = c_so_path();
            let rust_path = rust_so_path();
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));

            let c_sym: Symbol<Memchra2Fn> = c_lib
                .get(b"memchra2\0")
                .expect("C .so does not export `memchra2`");
            let rust_sym: Symbol<Memchra2Fn> = rust_lib
                .get(b"memchra2\0")
                .expect("Rust .so does not export `memchra2`");

            let c_memchra2 = *c_sym;
            let rust_memchra2 = *rust_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_memchra2,
                rust_memchra2,
            }
        }
    }
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(Pair::load)
}

/// Call both `.so` exports and assert bit-identical results.
#[track_caller]
pub fn assert_same(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let p = pair();
    let cv = unsafe { (p.c_memchra2)(a, b, c, d) };
    let rv = unsafe { (p.rust_memchra2)(a, b, c, d) };
    assert_eq!(
        cv, rv,
        "divergence for memchra2({a}, {b}, {c}, {d}) \
         [hex a=0x{a:08X} b=0x{b:08X} c=0x{c:08X} d=0x{d:08X}]: C={cv} (0x{cv:08X}) \
         Rust={rv} (0x{rv:08X})"
    );
    cv
}

/// Deterministic xorshift64* PRNG — reproducible across platforms/runs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` (inclusive), works across the full i64 span.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64;
        if span == u64::MAX {
            return self.next_u64() as i64;
        }
        lo + (self.next_u64() % (span + 1)) as i64
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        self.range_i64(lo as i64, hi as i64) as i32
    }
    /// Uniform in `[lo, hi]` over `u32` bit patterns, returned as `i32`.
    pub fn range_u32_as_i32(&mut self, lo: u32, hi: u32) -> i32 {
        let span = (hi - lo) as u64;
        (lo.wrapping_add((self.next_u64() % (span + 1)) as u32)) as i32
    }
}

/// Number of randomized samples per `CONFIGS.md` row.
pub const SAMPLES: usize = 400;

// ===========================================================================
// Internals harness (Phase C) — only available with `--features test_internals`
// ===========================================================================

use std::ffi::{c_char, c_int, c_uchar};

pub type FnMemchra = unsafe extern "C" fn(*const c_char, c_int, usize) -> c_int;
pub type FnProcessBuffer = unsafe extern "C" fn(*mut c_char, usize) -> c_int;
pub type FnIntToFloatBits = unsafe extern "C" fn(c_int) -> f32;
pub type FnProcessStrings =
    unsafe extern "C" fn(*const *const c_char, c_int, *const c_char) -> c_int;
pub type FnSafeSumArray = unsafe extern "C" fn(*const c_int, usize) -> c_int;
pub type FnInterpretAsInt = unsafe extern "C" fn(*const c_uchar, usize) -> c_int;
pub type FnCountOccurrences = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
pub type FnComplexIteration = unsafe extern "C" fn(*const c_int, usize) -> c_int;
pub type FnSnprintfFmt =
    unsafe extern "C" fn(*mut c_char, usize, c_int, c_int, c_int, c_int) -> c_int;

pub struct Internals {
    pub memchra: (FnMemchra, FnMemchra),
    pub process_buffer: (FnProcessBuffer, FnProcessBuffer),
    pub int_to_float_bits: (FnIntToFloatBits, FnIntToFloatBits),
    pub process_strings: (FnProcessStrings, FnProcessStrings),
    pub safe_sum_array: (FnSafeSumArray, FnSafeSumArray),
    pub interpret_as_int: (FnInterpretAsInt, FnInterpretAsInt),
    pub count_occurrences: (FnCountOccurrences, FnCountOccurrences),
    pub complex_iteration: (FnComplexIteration, FnComplexIteration),
    pub snprintf_fmt: (FnSnprintfFmt, FnSnprintfFmt),
    _c: Library,
    _rust: Library,
}

unsafe impl Send for Internals {}
unsafe impl Sync for Internals {}

/// `target/` directory of the crate under test.
fn target_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        // target/<profile>/deps/<bin>
        if let Some(p) = exe.parent().and_then(Path::parent).and_then(Path::parent) {
            return p.to_path_buf();
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")
}

/// Build (once per process) the C harness `.so` that textually includes the
/// unmodified `c_src/src/lib.c` and re-exports its `static` helpers.
pub fn build_c_harness() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("tests/c_harness/harness.c");
    assert!(src.exists(), "missing {}", src.display());
    let out_dir = target_dir().join("harness");
    std::fs::create_dir_all(&out_dir).expect("mkdir target/harness");
    let out = out_dir.join(format!("libc_harness_{}.so", std::process::id()));

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = std::process::Command::new(&cc)
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&out)
        .arg(&src)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));
    assert!(status.success(), "{cc} failed to build the C harness");
    out
}

macro_rules! load_pair {
    ($c:expr, $r:expr, $ty:ty, $name:literal) => {{
        let cs: Symbol<$ty> = $c
            .get(concat!($name, "\0").as_bytes())
            .expect(concat!("C harness .so lacks ", $name));
        let rs: Symbol<$ty> = $r
            .get(concat!($name, "\0").as_bytes())
            .expect(concat!("Rust .so lacks ", $name, " (build with --features test_internals)"));
        (*cs, *rs)
    }};
}

impl Internals {
    fn load() -> Internals {
        unsafe {
            let c_path = build_c_harness();
            let rust_path = rust_so_path();
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let r = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));

            let memchra = load_pair!(c, r, FnMemchra, "harness_memchra");
            let process_buffer = load_pair!(c, r, FnProcessBuffer, "harness_process_buffer");
            let int_to_float_bits =
                load_pair!(c, r, FnIntToFloatBits, "harness_int_to_float_bits");
            let process_strings = load_pair!(c, r, FnProcessStrings, "harness_process_strings");
            let safe_sum_array = load_pair!(c, r, FnSafeSumArray, "harness_safe_sum_array");
            let interpret_as_int = load_pair!(c, r, FnInterpretAsInt, "harness_interpret_as_int");
            let count_occurrences =
                load_pair!(c, r, FnCountOccurrences, "harness_count_occurrences");
            let complex_iteration =
                load_pair!(c, r, FnComplexIteration, "harness_complex_iteration");
            let snprintf_fmt = load_pair!(c, r, FnSnprintfFmt, "harness_snprintf_fmt");

            Internals {
                memchra,
                process_buffer,
                int_to_float_bits,
                process_strings,
                safe_sum_array,
                interpret_as_int,
                count_occurrences,
                complex_iteration,
                snprintf_fmt,
                _c: c,
                _rust: r,
            }
        }
    }
}

static INTERNALS: OnceLock<Internals> = OnceLock::new();

pub fn internals() -> &'static Internals {
    INTERNALS.get_or_init(Internals::load)
}
