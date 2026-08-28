//! Shared harness: locates and dynamically loads both the C reference `.so`
//! and the Rust `cdylib`, then exposes their exported symbols uniformly.
//!
//! Nothing here calls into the Rust crate directly — every invocation goes
//! through `libloading`, so the `#[no_mangle] extern "C"` wrappers are under
//! test just as they would be for any external consumer.

// Each test binary gets its own copy of this module and uses only part of it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// `void hsl_to_rgb(float *dest, const float *src)`
pub type HslToRgbFn = unsafe extern "C" fn(*mut f32, *const f32);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The C reference library, built by CMake into `c_src/build/`.
///
/// The CMake project name is derived from the parent directory name, so the
/// exact file name is not known ahead of time; scan for any `lib*.so`.
fn find_c_library() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C reference first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("lib"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build_dir.display()))
}

/// The Rust `cdylib` (`name = "hsl_to_rgb_lib"`).
///
/// `cargo test` does **not** rebuild a `cdylib`-only lib target (integration
/// tests cannot link it), so the artifact on disk can easily be stale. The
/// profile directory is picked to match the profile this test binary was built
/// with, and a mtime check turns a stale artifact into a loud failure instead of
/// a silently-passing test run.
fn find_rust_library() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let name = format!("{}hsl_to_rgb_lib.so", std::env::consts::DLL_PREFIX);
    let path = manifest_dir.join("target").join(profile).join(&name);

    let build_hint = format!(
        "run `cargo build{}` in translation/ before `cargo test`",
        if profile == "release" { " --release" } else { "" }
    );

    let so_mtime = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("{} is missing ({e}); {build_hint}", path.display()));

    // Guard against testing a stale library.
    for src in ["src/lib.rs", "Cargo.toml"] {
        let src_path = manifest_dir.join(src);
        if let Ok(src_mtime) = std::fs::metadata(&src_path).and_then(|m| m.modified())
            && src_mtime > so_mtime
        {
            panic!(
                "{} is older than {} — {build_hint}",
                path.display(),
                src_path.display()
            );
        }
    }

    path
}

/// Public accessor for the discovered C reference library path.
pub fn c_library_path() -> PathBuf {
    find_c_library()
}

/// Public accessor for the discovered Rust `cdylib` path.
pub fn rust_library_path() -> PathBuf {
    find_rust_library()
}

/// Both implementations, kept alive for the duration of a test.
pub struct Pair {
    c_lib: Library,
    rust_lib: Library,
}

impl Pair {
    pub fn load() -> Self {
        let c_path = find_c_library();
        let rust_path = find_rust_library();
        // SAFETY: both paths point at libraries we just built from the sources
        // in this repository; loading them runs only their normal init code.
        unsafe {
            Self {
                c_lib: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("loading C lib {}: {e}", c_path.display())),
                rust_lib: Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("loading Rust lib {}: {e}", rust_path.display())),
            }
        }
    }

    fn sym<'a, T>(lib: &'a Library, which: &str, name: &[u8]) -> Symbol<'a, T> {
        // SAFETY: the caller states the correct signature for `name`; a missing
        // symbol is reported as a panic rather than being dereferenced.
        unsafe { lib.get(name) }.unwrap_or_else(|e| {
            panic!(
                "{which} library does not export `{}`: {e}",
                String::from_utf8_lossy(name)
            )
        })
    }

    pub fn c_hsl_to_rgb(&self) -> Symbol<'_, HslToRgbFn> {
        Self::sym(&self.c_lib, "C", b"hsl_to_rgb\0")
    }

    pub fn rust_hsl_to_rgb(&self) -> Symbol<'_, HslToRgbFn> {
        Self::sym(&self.rust_lib, "Rust", b"hsl_to_rgb\0")
    }
}

/// Sentinel written into `dest` before each call so that we also detect
/// out-of-bounds writes and elements the implementation failed to set.
const GUARD: [u32; 8] = [
    0xDEAD_BEEF,
    0xDEAD_BEEF,
    0xDEAD_BEEF,
    0xDEAD_BEEF,
    0xDEAD_BEEF,
    0xDEAD_BEEF,
    0xDEAD_BEEF,
    0xDEAD_BEEF,
];

/// Invoke `f` with a guarded 8-float destination buffer and return the raw bit
/// patterns of all 8 slots (3 outputs + 5 canary slots).
fn call_guarded(f: &Symbol<'_, HslToRgbFn>, src: &[f32; 3]) -> [u32; 8] {
    let mut dest: [f32; 8] = GUARD.map(f32::from_bits);
    // SAFETY: `dest` has 8 >= 3 writable floats, `src` exactly 3 readable ones.
    unsafe { f(dest.as_mut_ptr(), src.as_ptr()) };
    dest.map(f32::to_bits)
}

/// Compare C and Rust for one input, using raw bit patterns so that `-0.0`
/// vs `0.0` and differing NaN payloads are both treated as mismatches.
pub fn assert_matches(pair: &Pair, src: [f32; 3], label: &str) {
    let c = call_guarded(&pair.c_hsl_to_rgb(), &src);
    let r = call_guarded(&pair.rust_hsl_to_rgb(), &src);
    assert!(
        c == r,
        "hsl_to_rgb mismatch [{label}]\n  src  = {src:?} (bits {:08x?})\n  C    = {:?} (bits \
         {:08x?})\n  Rust = {:?} (bits {:08x?})",
        src.map(f32::to_bits),
        c.map(f32::from_bits),
        c,
        r.map(f32::from_bits),
        r,
    );
}

/// Same as [`assert_matches`] but with `dest == src` (fully aliased buffers),
/// which the C implementation supports because it copies inputs into locals.
pub fn assert_matches_aliased(pair: &Pair, src: [f32; 3], label: &str) {
    let mut c_buf = src;
    let mut r_buf = src;
    // SAFETY: 3 readable and 3 writable floats; overlap is permitted by the
    // C implementation's read-into-locals-first structure.
    unsafe {
        (pair.c_hsl_to_rgb())(c_buf.as_mut_ptr(), c_buf.as_ptr());
        (pair.rust_hsl_to_rgb())(r_buf.as_mut_ptr(), r_buf.as_ptr());
    }
    let c = c_buf.map(f32::to_bits);
    let r = r_buf.map(f32::to_bits);
    assert!(
        c == r,
        "aliased hsl_to_rgb mismatch [{label}]\n  src  = {src:?}\n  C    = {:08x?}\n  Rust = \
         {:08x?}",
        c,
        r
    );
}

/// Small deterministic xorshift PRNG so fuzz cases are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + u * (hi - lo)
    }

    /// An arbitrary bit pattern reinterpreted as `f32` (may be NaN/inf).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

/// Interesting float values: branch boundaries, specials, extremes.
pub fn special_floats() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        f32::from_bits(1), // smallest positive subnormal
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-30,
        -1e-30,
        0.25,
        0.5,
        -0.5,
        0.75,
        1.0,
        -1.0,
        2.0,
        -2.0,
        59.0,
        59.999_996,
        60.0,
        60.000_004,
        119.999_99,
        120.0,
        120.000_01,
        179.999_98,
        180.0,
        180.000_02,
        239.999_98,
        240.0,
        240.000_02,
        299.999_97,
        300.0,
        300.000_03,
        359.999_97,
        360.0,
        360.000_03,
        720.0,
        -0.000_001,
        -1e-45,
        -60.0,
        -120.0,
        -180.0,
        -360.0,
        1e30,
        -1e30,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_1234), // quiet NaN, non-default payload
        f32::from_bits(0x7f80_0001), // signalling NaN
    ]
}

/// `x` shifted by `steps` ULPs (negative steps move towards `-inf`).
///
/// Walks the ordered float encoding, so it crosses zero and the
/// subnormal/normal boundary correctly and saturates at the infinities.
pub fn ulp_offset(x: f32, steps: i32) -> f32 {
    // Map to a monotonically ordered signed key, offset, and map back.
    let bits = x.to_bits() as i32;
    let ordered = if bits < 0 { i32::MIN - bits } else { bits };
    let moved = ordered.saturating_add(steps);
    let back = if moved < 0 { i32::MIN - moved } else { moved };
    f32::from_bits(back as u32)
}
