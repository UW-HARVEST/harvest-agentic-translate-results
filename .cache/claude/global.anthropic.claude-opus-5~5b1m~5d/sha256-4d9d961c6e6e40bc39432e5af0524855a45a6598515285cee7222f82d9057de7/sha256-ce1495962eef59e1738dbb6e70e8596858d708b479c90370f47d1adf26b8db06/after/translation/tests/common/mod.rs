//! Shared differential-test harness.
//!
//! Both the C library and the Rust library are loaded as shared objects with
//! `libloading` and driven **only** through their exported C symbols, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test. No Rust
//! function is ever called directly.

#![allow(dead_code)]

use libloading::os::unix::Symbol as RawSymbol;
use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub type JumpnodeFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type InitFn = unsafe extern "C" fn();

// ---------------------------------------------------------------------------
// Paths / builds
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Working-directory root that holds both `c_src/` and `translation/`.
pub fn root_dir() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

fn scratch_dir() -> PathBuf {
    let d = manifest_dir().join("target").join("ctest");
    std::fs::create_dir_all(&d).expect("create target/ctest");
    d
}

fn run(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    if !out.status.success() {
        panic!(
            "{what} failed ({})\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

fn newer_than(a: &Path, b: &Path) -> bool {
    let ma = match std::fs::metadata(a).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mb = match std::fs::metadata(b).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return true,
    };
    ma > mb
}

/// The C shared library produced by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent* directory name via
/// `cmake_path(GET ... PARENT_PATH)`, so the artifact name is not fixed; it is
/// discovered rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let c_src = root_dir().join("c_src");
        let build = c_src.join("build");
        if find_so(&build).is_none() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            run(
                Command::new("cmake")
                    .current_dir(&build)
                    .arg("..")
                    .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON"),
                "cmake configure",
            );
            run(
                Command::new("cmake")
                    .current_dir(&build)
                    .args(["--build", "."]),
                "cmake build",
            );
        }
        find_so(&build).unwrap_or_else(|| panic!("no .so found in {}", build.display()))
    })
    .clone()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop()
}

/// The test-only C shim `.so`: pristine `c_src/src/lib.c` plus an exported
/// wrapper for the file-local `initialize_test_data()`.
pub fn c_shim_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = manifest_dir().join("tests/csupport/init_shim.c");
        let c_lib = root_dir().join("c_src/src/lib.c");
        let out = scratch_dir().join("libjumpnode_c_shim.so");
        if !out.exists() || newer_than(&src, &out) || newer_than(&c_lib, &out) {
            run(
                Command::new("cc")
                    .arg("-shared")
                    .arg("-fPIC")
                    .arg("-o")
                    .arg(&out)
                    .arg(&src)
                    .arg("-lm"),
                "cc (C init shim)",
            );
        }
        out
    })
    .clone()
}

/// The Rust cdylib, built with exactly the feature set this test binary was
/// compiled with, into a private target dir so it cannot deadlock on the
/// enclosing `cargo test`'s build lock.
pub fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Ok(p) = std::env::var("JUMPNODE_RUST_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "JUMPNODE_RUST_SO does not exist: {}", p.display());
            return p;
        }

        let feat_init = cfg!(feature = "expose_init_test_data");
        let tag = if feat_init { "feat_init" } else { "default" };
        let target_dir = manifest_dir().join("target").join("sotest").join(tag);

        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.current_dir(manifest_dir())
            .env("CARGO_TARGET_DIR", &target_dir)
            .env_remove("RUSTFLAGS")
            .args(["build", "--release", "--lib", "--offline", "--no-default-features"]);
        if feat_init {
            cmd.args(["--features", "expose_init_test_data"]);
        }
        run(&mut cmd, "cargo build --release --lib (Rust cdylib)");

        let p = target_dir.join("release").join("libjumpnode_lib.so");
        assert!(p.exists(), "Rust cdylib not found at {}", p.display());
        p
    })
    .clone()
}

// ---------------------------------------------------------------------------
// Loaded pair
// ---------------------------------------------------------------------------

/// A loaded (C, Rust) pair, each reached only through `dlsym`.
pub struct Pair {
    _c: Library,
    _r: Library,
    pub c_jumpnode: RawSymbol<JumpnodeFn>,
    pub r_jumpnode: RawSymbol<JumpnodeFn>,
    pub c_init: Option<RawSymbol<InitFn>>,
    pub r_init: Option<RawSymbol<InitFn>>,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

fn load(path: &Path) -> Library {
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
}

fn sym<T>(lib: &Library, name: &[u8]) -> RawSymbol<T> {
    let s = unsafe { lib.get::<T>(name) }.unwrap_or_else(|e| {
        panic!("dlsym({}) failed: {e}", String::from_utf8_lossy(name))
    });
    unsafe { s.into_raw() }
}

fn opt_sym<T>(lib: &Library, name: &[u8]) -> Option<RawSymbol<T>> {
    unsafe { lib.get::<T>(name) }
        .ok()
        .map(|s| unsafe { s.into_raw() })
}

impl Pair {
    /// Loads the plain C library (as shipped by CMake) alongside the Rust cdylib.
    pub fn shipped() -> &'static Pair {
        static P: OnceLock<Pair> = OnceLock::new();
        P.get_or_init(|| Pair::new(c_so_path(), rust_so_path()))
    }

    /// Loads the C init-shim library alongside the Rust cdylib. Requires the
    /// `expose_init_test_data` feature on the Rust side.
    pub fn with_init() -> &'static Pair {
        static P: OnceLock<Pair> = OnceLock::new();
        P.get_or_init(|| Pair::new(c_shim_so_path(), rust_so_path()))
    }

    fn new(c_path: PathBuf, r_path: PathBuf) -> Pair {
        let c = load(&c_path);
        let r = load(&r_path);
        let c_jumpnode = sym::<JumpnodeFn>(&c, b"jumpnode\0");
        let r_jumpnode = sym::<JumpnodeFn>(&r, b"jumpnode\0");
        let c_init = opt_sym::<InitFn>(&c, b"jumpnode_initialize_test_data\0");
        let r_init = opt_sym::<InitFn>(&r, b"jumpnode_initialize_test_data\0");
        Pair {
            _c: c,
            _r: r,
            c_jumpnode,
            r_jumpnode,
            c_init,
            r_init,
            c_path,
            r_path,
        }
    }

    #[inline]
    pub fn c(&self, m: c_int, n: c_int, d: c_int, f: c_int) -> c_int {
        unsafe { (self.c_jumpnode)(m, n, d, f) }
    }

    #[inline]
    pub fn r(&self, m: c_int, n: c_int, d: c_int, f: c_int) -> c_int {
        unsafe { (self.r_jumpnode)(m, n, d, f) }
    }

    /// Calls both `.so`s and asserts byte-identical `int` results.
    #[track_caller]
    pub fn assert_same(&self, m: c_int, n: c_int, d: c_int, f: c_int) -> c_int {
        let cv = self.c(m, n, d, f);
        let rv = self.r(m, n, d, f);
        assert_eq!(
            cv, rv,
            "DIVERGENCE jumpnode(mode={m}, node_id={n}, depth={d}, flags={f}): \
             C={cv} (0o{cv:o}) vs Rust={rv} (0o{rv:o})"
        );
        cv
    }

    /// Same, but also asserts the shared value equals `expected`.
    #[track_caller]
    pub fn assert_same_eq(
        &self,
        m: c_int,
        n: c_int,
        d: c_int,
        f: c_int,
        expected: c_int,
    ) -> c_int {
        let got = self.assert_same(m, n, d, f);
        assert_eq!(
            got, expected,
            "jumpnode(mode={m}, node_id={n}, depth={d}, flags={f}): \
             both libs returned {got} but the C source implies {expected}"
        );
        got
    }

    /// Drives `initialize_test_data` in BOTH libraries through their exports.
    pub fn init_both(&self) {
        let c = self
            .c_init
            .as_ref()
            .expect("C shim does not export jumpnode_initialize_test_data");
        let r = self
            .r_init
            .as_ref()
            .expect("Rust .so does not export jumpnode_initialize_test_data");
        unsafe {
            c();
            r();
        }
    }
}

/// Serializes tests that mutate the libraries' shared static state.
pub fn state_lock() -> std::sync::MutexGuard<'static, ()> {
    static L: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    L.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform over the whole `i32` range.
    pub fn i32_any(&mut self) -> c_int {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn i32_range(&mut self, lo: c_int, hi: c_int) -> c_int {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as c_int
    }

    /// Picks from a small slice.
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }

    /// A value biased towards "interesting" magnitudes: tiny values, powers of
    /// ten, boundaries around the `i32` extremes, and fully random values.
    pub fn i32_interesting(&mut self) -> c_int {
        const EDGES: [c_int; 30] = [
            0, 1, -1, 2, -2, 3, -3, 4, -4, 5, -5, 7, 8, 9, 10, -9, -10, 15, 16, 17, 99, 100, -99,
            -100, 127, 128, -128, i32::MAX, i32::MIN, i32::MAX - 1,
        ];
        match self.next_u64() % 4 {
            0 => self.pick(&EDGES),
            1 => {
                let k = (self.next_u64() % 10) as u32;
                let base = 10i64.pow(k);
                let v = base + (self.next_u64() % 3) as i64 - 1;
                let v = if self.next_u64() % 2 == 0 { v } else { -v };
                v.clamp(i32::MIN as i64, i32::MAX as i64) as c_int
            }
            2 => self.i32_range(-1000, 1000),
            _ => self.i32_any(),
        }
    }
}

// ---------------------------------------------------------------------------
// Independent model of the C source (for `assert_same_eq` expectations)
// ---------------------------------------------------------------------------

/// Decimal width that `printf("%d")` produces, used to predict case `0003`.
pub fn decimal_width(v: c_int) -> usize {
    let mut s = String::new();
    use std::fmt::Write;
    write!(&mut s, "{v}").unwrap();
    s.len()
}

/// Expected result of `jumpnode(0003, node_id, depth, flags)` straight from the
/// C source: `sprintf("Node_%d_Depth_%d")`, then `strlen*2 + 010`, then
/// `+ (flags & 0177)`.
pub fn expect_mode3(node_id: c_int, depth: c_int, flags: c_int) -> c_int {
    let len = 5 + decimal_width(node_id) + 7 + decimal_width(depth);
    (len as c_int) * 2 + 0o10 + (flags & 0o177)
}

pub const ERR_MODE1_NOT_FOUND: c_int = 0o2 | 0o20; // 18
pub const ERR_MODE2_NOT_FOUND: c_int = 0o2 | 0o40; // 34
pub const ERR_MODE4_NOT_FOUND: c_int = 0o2 | 0o100; // 66
pub const ERR_UNKNOWN_MODE: c_int = 0o2 | 0o200; // 130
