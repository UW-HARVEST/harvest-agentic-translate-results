//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compares their exported `get_predict_func` through the FFI
//! boundary. The Rust code is never called directly, so the `#[no_mangle]`
//! export wrapper is under test too.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

pub type GetPredictFunc = unsafe extern "C" fn(std::ffi::c_int) -> std::ffi::c_int;

/// Crate root (`translation/`).
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Working-directory root (the parent of `translation/` and `c_src/`).
fn work_root() -> PathBuf {
    crate_root()
        .parent()
        .expect("crate root has a parent")
        .to_path_buf()
}

fn find_so(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            name.starts_with(prefix) && name.ends_with(".so")
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Path to the C shared library. The CMake project name is derived from the
/// working directory's name, so the file name is not fixed — glob for it.
pub fn c_lib_path() -> PathBuf {
    let build = work_root().join("c_src").join("build");
    find_so(&build, "lib").unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path to the Rust `cdylib`. Located relative to the running test binary
/// (`target/<profile>/deps/<test>`), so it picks up whatever profile and
/// feature set was last built.
///
/// IMPORTANT: with `crate-type = ["cdylib"]` only, `cargo test` does *not*
/// refresh the `.so` — it builds a separate unit-test harness instead. A stale
/// `.so` would make every differential test pass vacuously, so
/// [`assert_rust_lib_fresh`] cross-checks mtimes and this function calls it.
pub fn rust_lib_path() -> PathBuf {
    let p = rust_lib_path_raw();
    assert_rust_lib_fresh(&p);
    p
}

fn rust_lib_path_raw() -> PathBuf {
    let exe = std::env::current_exe().expect("test exe path");
    // .../target/<profile>/deps/<test-bin>
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    for _ in 0..2 {
        if let Some(p) = find_so(&dir, "libget_predict_func_lib") {
            return p;
        }
        dir = match dir.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
    }
    panic!(
        "could not locate libget_predict_func_lib.so near {}",
        exe.display()
    );
}

fn mtime(p: &Path) -> std::time::SystemTime {
    std::fs::metadata(p)
        .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
        .modified()
        .expect("mtime")
}

/// Refuse to run against a `.so` older than the Rust sources it is built from.
/// This is what turns a silently-stale mutation into a hard failure.
pub fn assert_rust_lib_fresh(so: &Path) {
    let so_t = mtime(so);
    let src_dir = crate_root().join("src");
    for e in std::fs::read_dir(&src_dir).expect("read src/").flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        assert!(
            mtime(&p) <= so_t,
            "STALE ARTIFACT: {} is newer than {}.\n\
             `cargo test` does not rebuild a cdylib-only lib target. Run \
             `cargo build [--release] [feature flags]` first (or use \
             ./verify_all.sh); otherwise the differential tests compare against \
             an out-of-date library and pass vacuously.",
            p.display(),
            so.display()
        );
    }
}

/// Both implementations, loaded through `dlopen`.
pub struct Pair {
    _c: Library,
    _r: Library,
    c_fn: GetPredictFunc,
    r_fn: GetPredictFunc,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c = Library::new(c_lib_path()).expect("dlopen C .so");
            let r = Library::new(rust_lib_path()).expect("dlopen Rust .so");
            let c_sym: Symbol<GetPredictFunc> = c
                .get(b"get_predict_func\0")
                .expect("C .so exports get_predict_func");
            let r_sym: Symbol<GetPredictFunc> = r
                .get(b"get_predict_func\0")
                .expect("Rust .so exports get_predict_func");
            let c_fn = *c_sym;
            let r_fn = *r_sym;
            Pair {
                _c: c,
                _r: r,
                c_fn,
                r_fn,
            }
        }
    }

    #[inline]
    pub fn c(&self, pfcn: i32) -> i32 {
        unsafe { (self.c_fn)(pfcn) }
    }

    #[inline]
    pub fn rust(&self, pfcn: i32) -> i32 {
        unsafe { (self.r_fn)(pfcn) }
    }

    /// Assert byte-identical results, returning the shared value.
    #[track_caller]
    pub fn assert_same(&self, pfcn: i32) -> i32 {
        let c = self.c(pfcn);
        let r = self.rust(pfcn);
        assert_eq!(
            c, r,
            "divergence for get_predict_func({pfcn}) [0x{pfcn:08x}]: C={c} Rust={r}"
        );
        c
    }
}

/// Deterministic 64-bit LCG (fixed seed) so every property run is reproducible.
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Lcg {
        Lcg(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        // Numerical Recipes constants.
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
