//! (Each test binary uses a subset of this shared harness, hence the allow.)
#![allow(dead_code)]

//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes `jumpnode` from each. The Rust side is *never* called directly —
//! only through its exported `#[no_mangle]` symbol, exactly as an external
//! consumer would.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub type JumpnodeFn = unsafe extern "C" fn(
    std::os::raw::c_int,
    std::os::raw::c_int,
    std::os::raw::c_int,
    std::os::raw::c_int,
) -> std::os::raw::c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let path = e.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(path);
            }
        }
    }
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one lib*.so in {:?}, found {:?}. \
         Build the C library first: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build,
        candidates
    );
    candidates.pop().unwrap()
}

/// `cargo test` builds the test binaries but **not** the `cdylib` artifact,
/// because no test target depends on it at link time. Without this step the
/// differential tests would happily `dlopen` a stale `libjumpnode_lib.so` left
/// over from an earlier `cargo build` and report success against code that is no
/// longer in `src/lib.rs`. So: build it here (once per test binary), then refuse
/// to run if the artifact is still older than the sources.
fn ensure_rust_so_fresh() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let release = std::env::current_exe()
            .map(|p| p.components().any(|c| c.as_os_str() == "release"))
            .unwrap_or(false);
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(&cargo);
        cmd.arg("build").arg("--lib").current_dir(&manifest);
        if release {
            cmd.arg("--release");
        }
        // Inherit the feature selection of the current test run so the cdylib is
        // built with the same cfg as the tests that probe it.
        if let Ok(features) = std::env::var("DIFF_CARGO_FEATURES") {
            if features == "--no-default-features" {
                cmd.arg("--no-default-features");
            } else if !features.is_empty() {
                cmd.arg("--no-default-features");
                cmd.arg("--features").arg(features);
            }
        }
        let out = cmd
            .output()
            .unwrap_or_else(|e| panic!("failed to run `{cargo} build --lib`: {e}"));
        assert!(
            out.status.success(),
            "`cargo build --lib` failed while refreshing the cdylib:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

fn mtime(p: &Path) -> std::time::SystemTime {
    std::fs::metadata(p)
        .unwrap_or_else(|e| panic!("cannot stat {p:?}: {e}"))
        .modified()
        .unwrap_or(std::time::UNIX_EPOCH)
}

fn assert_not_stale(so: &Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs");
    let so_t = mtime(so);
    let src_t = mtime(&src);
    assert!(
        so_t >= src_t,
        "STALE ARTIFACT: {so:?} (mtime {so_t:?}) is older than {src:?} (mtime {src_t:?}). \
         The differential tests would be comparing against out-of-date Rust code. \
         Run `cargo build --release` (or `cargo build`) and re-run the tests."
    );
}

fn find_rust_so() -> PathBuf {
    ensure_rust_so_fresh();
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib is one directory up. Fall back to the well-known locations.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            roots.push(deps.to_path_buf());
            if let Some(profile) = deps.parent() {
                roots.push(profile.to_path_buf());
            }
        }
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    roots.push(target.join("release"));
    roots.push(target.join("debug"));

    for r in &roots {
        let p = r.join("libjumpnode_lib.so");
        if p.is_file() {
            assert_not_stale(&p);
            return p;
        }
    }
    panic!(
        "libjumpnode_lib.so not found; searched {:?}. Run `cargo build` first.",
        roots
    );
}

pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: JumpnodeFn,
    pub rust: JumpnodeFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path).expect("failed to dlopen C .so");
            let rust_lib = Library::new(&rust_path).expect("failed to dlopen Rust .so");
            let c_sym: Symbol<JumpnodeFn> =
                c_lib.get(b"jumpnode\0").expect("C .so does not export `jumpnode`");
            let rust_sym: Symbol<JumpnodeFn> = rust_lib
                .get(b"jumpnode\0")
                .expect("Rust .so does not export `jumpnode` (missing #[no_mangle]?)");
            let c = *c_sym;
            let rust = *rust_sym;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    }

    /// Calls both implementations and returns `(c_result, rust_result)`.
    pub fn call(&self, m: i32, n: i32, d: i32, f: i32) -> (i32, i32) {
        unsafe { ((self.c)(m, n, d, f), (self.rust)(m, n, d, f)) }
    }

    /// Calls both and asserts byte-identical return values.
    #[track_caller]
    pub fn assert_same(&self, m: i32, n: i32, d: i32, f: i32) {
        let (a, b) = self.call(m, n, d, f);
        assert_eq!(
            a, b,
            "DIVERGENCE jumpnode(mode={m}, node_id={n}, depth={d}, flags={f}): \
             C returned {a} (0o{a:o}), Rust returned {b} (0o{b:o})"
        );
    }
}

/// Deterministic xorshift64* PRNG so every run uses the identical input stream.
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
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Random i32 biased towards small magnitudes and boundary values, which is
    /// where the digit-count / mask / overflow branches live.
    pub fn next_i32_biased(&mut self) -> i32 {
        let r = self.next_u64();
        match r % 8 {
            0 => (r >> 8) as u8 as i32,                       // 0..255
            1 => -(((r >> 8) as u8 as i32)),                  // -255..0
            2 => i32::MIN.wrapping_add((r >> 8) as u8 as i32), // near INT_MIN
            3 => i32::MAX.wrapping_sub((r >> 8) as u8 as i32), // near INT_MAX
            4 => {
                // powers of ten +/- 1, exercising %d width transitions
                const POW10: [i64; 10] = [1, 10, 100, 1000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000, 1_000_000_000];
                let p = POW10[((r >> 8) % 10) as usize];
                let delta = ((r >> 16) % 3) as i64 - 1;
                let sign = if (r >> 24) & 1 == 0 { 1i64 } else { -1i64 };
                (sign * (p + delta)) as i32
            }
            5 => 1i32.wrapping_shl(((r >> 8) % 32) as u32),
            _ => (r >> 32) as u32 as i32,
        }
    }
}

/// Modes the C `switch` has explicit `case` labels for.
pub const VALID_MODES: [i32; 4] = [0o1, 0o2, 0o3, 0o4];

/// A spread of interesting i32 values reused by many rows.
pub const EDGE_I32: [i32; 29] = [
    0, 1, -1, 2, -2, 3, 4, 5, 7, 8, 9, 10, -9, -10, 15, 16, 17, 19, 20, 0o177, 0o200, 0o377, 99,
    100, 1_000_000_000, -1_000_000_000, i32::MAX, i32::MIN, i32::MIN + 1,
];

/// Scales randomized-iteration counts. Unoptimized (`dev`) builds run the same
/// code paths but ~20x fewer samples so the suite stays well inside the time
/// budget; `--release` runs the full count. Override with `DIFF_ITERS_DIV`.
pub fn iters(n: usize) -> usize {
    let div: usize = std::env::var("DIFF_ITERS_DIV")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(if cfg!(debug_assertions) { 20 } else { 1 });
    let div = div.max(1);
    (n / div).max(1)
}

/// Path to the freshly-built Rust `cdylib`. Rebuilds it and rejects a stale
/// artifact (see `ensure_rust_so_fresh`).
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

/// Path to the C `.so` produced by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    find_c_so()
}
