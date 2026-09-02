//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and resolves `div_euclid` from
//! each. The Rust implementation is never called directly — it is always
//! reached through `dlsym` on `libdiv_euclid_lib.so`, so the `#[no_mangle]`
//! `extern "C"` export wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub type DivEuclidFn = unsafe extern "C" fn(c_int, c_int) -> c_int;

pub const I32_MIN: c_int = i32::MIN;
pub const I32_MAX: c_int = i32::MAX;

/// `translation/`
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object built by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent directory name* of
/// `c_src`, so the file name is not fixed. Scan the build tree for the single
/// `.so` instead of hard-coding it.
fn find_c_so() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    collect_so(&build, &mut found);
    assert!(
        !found.is_empty(),
        "no .so found under {}. Build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.sort();
    found.swap_remove(0)
}

fn collect_so(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_so(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("so") {
            out.push(p);
        }
    }
}

/// Locate the Rust `cdylib`. Prefer the build profile the test binary itself
/// was built with (`target/<profile>/libdiv_euclid_lib.so`).
///
/// `cargo test` does **not** rebuild the `cdylib` artifact for a `crate-type =
/// ["cdylib"]` package (it only builds the test harnesses), so the `.so` on disk
/// can be stale relative to `src/lib.rs`. That would silently make every
/// differential test validate an old binary. Build it explicitly first, then
/// assert freshness.
fn find_rust_so() -> PathBuf {
    const NAME: &str = "libdiv_euclid_lib.so";

    let release = std::env::current_exe()
        .map(|p| p.components().any(|c| c.as_os_str() == "release"))
        .unwrap_or(false);

    static BUILT: OnceLock<()> = OnceLock::new();
    BUILT.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(cargo);
        cmd.arg("build").arg("--lib").current_dir(manifest_dir());
        if release {
            cmd.arg("--release");
        }
        // Do not inherit the test run's RUSTFLAGS-sensitive env that could
        // change the artifact path.
        cmd.env_remove("CARGO_TARGET_DIR");
        let out = cmd.output().expect("failed to spawn `cargo build --lib`");
        assert!(
            out.status.success(),
            "`cargo build --lib{}` failed:\n{}",
            if release { " --release" } else { "" },
            String::from_utf8_lossy(&out.stderr)
        );
    });

    let profile_dir = manifest_dir()
        .join("target")
        .join(if release { "release" } else { "debug" });
    let so = profile_dir.join(NAME);
    assert!(
        so.is_file(),
        "{} not found after `cargo build --lib`",
        so.display()
    );

    // Freshness gate: the artifact must be at least as new as every source file.
    let so_mtime = std::fs::metadata(&so)
        .and_then(|m| m.modified())
        .expect("stat rust .so");
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest_dir().join(src);
        if let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            assert!(
                so_mtime >= m,
                "{} is STALE (older than {}). The differential tests would be \
                 validating an out-of-date binary.",
                so.display(),
                p.display()
            );
        }
    }
    so
}

/// Both libraries, kept alive for the lifetime of the test.
pub struct Pair {
    _c_lib: Library,
    _r_lib: Library,
    pub c: DivEuclidFn,
    pub r: DivEuclidFn,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let r_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", r_path.display()));

            let c_sym: Symbol<DivEuclidFn> = c_lib
                .get(b"div_euclid\0")
                .expect("C .so does not export div_euclid");
            let r_sym: Symbol<DivEuclidFn> = r_lib
                .get(b"div_euclid\0")
                .expect("Rust .so does not export div_euclid");

            let c = *c_sym;
            let r = *r_sym;
            Pair {
                _c_lib: c_lib,
                _r_lib: r_lib,
                c,
                r,
                c_path,
                r_path,
            }
        }
    }

    /// Call both `.so`s and assert byte-identical results.
    #[track_caller]
    pub fn check(&self, v1: c_int, v2: c_int) {
        let cv = unsafe { (self.c)(v1, v2) };
        let rv = unsafe { (self.r)(v1, v2) };
        assert_eq!(
            cv.to_le_bytes(),
            rv.to_le_bytes(),
            "div_euclid({v1}, {v2}): C returned {cv} (0x{cv:08x}), Rust returned {rv} (0x{rv:08x})"
        );
    }

    /// Assert both agree *and* that the shared value is `expected` — used by the
    /// error-path tests so a row asserts the same specific sentinel/code, not
    /// merely "both did the same thing".
    #[track_caller]
    pub fn check_eq(&self, v1: c_int, v2: c_int, expected: c_int) {
        let cv = unsafe { (self.c)(v1, v2) };
        let rv = unsafe { (self.r)(v1, v2) };
        assert_eq!(cv, expected, "C div_euclid({v1}, {v2}) = {cv}, expected {expected}");
        assert_eq!(rv, cv, "Rust div_euclid({v1}, {v2}) = {rv}, C = {cv}");
    }

    /// Returns the value both agree on, asserting agreement first.
    #[track_caller]
    pub fn agreed(&self, v1: c_int, v2: c_int) -> c_int {
        self.check(v1, v2);
        unsafe { (self.c)(v1, v2) }
    }
}

/// xorshift64* — deterministic, fixed seed, no external crate.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x2545_F491_4F6C_DD1D } else { seed })
    }
    /// The seed mandated by CONFIGS.md for reproducibility.
    pub fn fixed() -> Rng {
        Rng::new(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    /// Uniform in `[lo, hi]` inclusive; works across the full i32 range.
    pub fn range(&mut self, lo: c_int, hi: c_int) -> c_int {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        let v = self.next_u64() % span;
        (lo as i64 + v as i64) as c_int
    }
    /// Uniform in `[1, INT_MAX]`.
    pub fn pos(&mut self) -> c_int {
        self.range(1, I32_MAX)
    }
    /// Uniform in `[INT_MIN + 1, -1]` (excludes `INT_MIN`).
    pub fn neg_non_min(&mut self) -> c_int {
        self.range(I32_MIN + 1, -1)
    }
}

/// The boundary value set used by the cross-product rows.
pub fn boundary_values() -> Vec<c_int> {
    let mut v: Vec<c_int> = Vec::new();
    v.push(0);
    for m in [
        1i64,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        15,
        16,
        17,
        31,
        32,
        33,
        100,
        127,
        128,
        129,
        255,
        256,
        257,
        1000,
        1023,
        1024,
        1025,
        65535,
        65536,
        65537,
        1 << 24,
        (1 << 30) - 1,
        1 << 30,
        (1 << 30) + 1,
        i32::MAX as i64 - 2,
        i32::MAX as i64 - 1,
        i32::MAX as i64,
    ] {
        v.push(m as c_int);
        v.push(-(m as c_int));
    }
    v.push(I32_MIN);
    v.push(I32_MIN + 1);
    v.push(I32_MIN + 2);
    v.sort();
    v.dedup();
    v
}

/// Run `n` randomized draws for one CONFIGS.md row.
pub fn sweep<F>(pair: &Pair, n: usize, mut f: F)
where
    F: FnMut(&mut Rng) -> (c_int, c_int),
{
    let mut rng = Rng::fixed();
    for _ in 0..n {
        let (v1, v2) = f(&mut rng);
        pair.check(v1, v2);
    }
}
