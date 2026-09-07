//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes them behind an identical call interface.
//!
//! Neither side is ever called as a normal Rust function — both go through
//! `dlopen`/`dlsym`, so the `#[no_mangle] extern "C"` export wrapper is under
//! test too.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// The C ABI under test: `float half2float(uint16_t h);`
pub type Half2FloatFn = unsafe extern "C" fn(u16) -> f32;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Find the first file in `dir` whose name starts with `lib` and ends with `.so`.
fn find_so(dir: &Path, exclude_substr: Option<&str>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            if !name.starts_with("lib") || !name.ends_with(".so") {
                return false;
            }
            if let Some(x) = exclude_substr {
                if name.contains(x) {
                    return false;
                }
            }
            true
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Locate the C shared library built by CMake into `c_src/build/`.
pub fn c_so_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    find_so(&build_dir, None).unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}.\nBuild it first:\n  \
             cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && \
             cmake --build c_src/build",
            build_dir.display()
        )
    })
}

/// Locate the Rust `cdylib`.
///
/// `HALF2FLOAT_RUST_SO` overrides the path outright, and
/// `HALF2FLOAT_PROFILE` (`debug`/`release`) selects which profile's artifact to
/// load. This matters because the `debug` profile enables integer overflow
/// checks while the C relies on unsigned wrap-around, so the two artifacts must
/// be verified separately.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HALF2FLOAT_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HALF2FLOAT_RUST_SO points at a missing file: {}", p.display());
        return p;
    }

    let target = workspace_root().join("translation").join("target");
    let profiles: Vec<&str> = match std::env::var("HALF2FLOAT_PROFILE") {
        Ok(p) if p == "debug" => vec!["debug"],
        Ok(p) if p == "release" => vec!["release"],
        _ => vec!["release", "debug"],
    };
    for profile in profiles {
        let dir = target.join(profile);
        // Exclude the test harness's own artifacts; our cdylib is
        // `libhalf2float_lib.so` (see [lib] name in Cargo.toml).
        let direct = dir.join("libhalf2float_lib.so");
        if direct.is_file() {
            return direct;
        }
        if let Some(p) = find_so(&dir, None) {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}.\nBuild it first:\n  cargo build --release",
        target.display()
    )
}

/// A loaded implementation. Holds the `Library` so the symbol stays valid.
pub struct Impl {
    _lib: Library,
    func: Half2FloatFn,
    pub name: &'static str,
}

impl Impl {
    pub fn load(path: &Path, name: &'static str) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen failed for {} ({name}): {e}", path.display()));
            let sym: Symbol<Half2FloatFn> = lib.get(b"half2float\0").unwrap_or_else(|e| {
                panic!("dlsym `half2float` failed in {} ({name}): {e}", path.display())
            });
            let func = *sym;
            Impl {
                _lib: lib,
                func,
                name,
            }
        }
    }

    /// Call through the FFI boundary and return the RAW bit pattern, so that
    /// NaN payloads and -0.0 vs +0.0 are compared byte-for-byte rather than
    /// with float `==`.
    #[inline]
    pub fn call_bits(&self, h: u16) -> u32 {
        unsafe { (self.func)(h) }.to_bits()
    }

    #[inline]
    pub fn call(&self, h: u16) -> f32 {
        unsafe { (self.func)(h) }
    }
}

/// Load both implementations.
pub fn both() -> (Impl, Impl) {
    let c = Impl::load(&c_so_path(), "C");
    let r = Impl::load(&rust_so_path(), "Rust");
    (c, r)
}

/// Deterministic PRNG (fixed seed) so randomized sweeps are reproducible.
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Lcg {
        Lcg(seed)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }
    #[inline]
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u32() & 0xFFFF) as u16
    }
}

/// Assert C and Rust agree on `h`, with a diagnostic naming the configuration.
#[track_caller]
pub fn assert_same(c: &Impl, r: &Impl, h: u16, ctx: &str) {
    let cb = c.call_bits(h);
    let rb = r.call_bits(h);
    assert_eq!(
        cb, rb,
        "DIVERGENCE [{ctx}] h=0x{h:04X} (n={}, low=0x{:03X}): \
         C=0x{cb:08X} ({}) vs Rust=0x{rb:08X} ({})",
        h >> 10,
        h & 0x3FF,
        f32::from_bits(cb),
        f32::from_bits(rb),
    );
}
