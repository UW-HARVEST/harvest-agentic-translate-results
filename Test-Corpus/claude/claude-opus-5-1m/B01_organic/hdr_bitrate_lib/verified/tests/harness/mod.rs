//! Shared harness: locates and `dlopen`s BOTH shared objects and exposes the
//! `hdr_bitrate` export of each through the FFI boundary.
//!
//! Nothing in this file ever calls the Rust implementation directly. The Rust
//! side is always reached via `dlsym("hdr_bitrate")` on
//! `target/<profile>/libhdr_bitrate_lib.so`, exactly as an external C consumer
//! would, so the `#[no_mangle] extern "C"` wrapper is under test too.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The C ABI of the function under test: `unsigned hdr_bitrate(const uint8_t *h);`
pub type HdrBitrateFn = unsafe extern "C" fn(*const u8) -> u32;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build the C shared library with CMake if it is not already present.
fn ensure_c_so() -> PathBuf {
    // Optional override so the same suite can be run against C shared objects
    // built at other optimisation levels / by other compilers, to confirm the
    // C's out-of-bounds table reads (see ERRORS.md E2/E3) observe the same
    // values there too.
    if let Some(p) = std::env::var_os("HDR_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HDR_C_SO={} is not a file", p.display());
        return p;
    }

    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");

    if let Some(p) = find_so_in(&build) {
        return p;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");

    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("failed to run `cmake` — is CMake installed?");
    assert!(st.success(), "cmake configure failed");

    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("failed to run `cmake --build`");
    assert!(st.success(), "cmake build failed");

    find_so_in(&build).expect("C .so not produced in c_src/build")
}

fn find_so_in(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n.starts_with("lib"))
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Build the Rust `cdylib` FRESH and return its path.
///
/// This must not simply *locate* an existing `target/debug/libhdr_bitrate_lib.so`:
/// `cargo test` does **not** rebuild the `cdylib` artifact (the integration test
/// binaries do not link against it), so a previously-built `.so` can be
/// arbitrarily stale. Loading a stale `.so` silently makes every differential
/// assertion below test old code — i.e. vacuously pass. This was observed in
/// practice: nine deliberate mutations of `src/lib.rs` all survived until this
/// function was introduced.
///
/// We therefore invoke `cargo build --lib` explicitly, into a *separate*
/// `--target-dir` so we take a different build lock than the outer `cargo test`
/// and cannot deadlock. Building only `--lib` also means no test targets are
/// compiled, so there is no recursion.
fn ensure_rust_so() -> PathBuf {
    let name = "libhdr_bitrate_lib.so";
    let target_dir = manifest_dir().join("target").join("difftest");

    // Mirror the profile of the test binary itself, so `cargo test --release`
    // compares against a release-built `.so` (different codegen, and
    // `[profile.release] panic = "abort"`).
    let release = !cfg!(debug_assertions);

    let mut cmd = Command::new(env!("CARGO"));
    cmd.current_dir(manifest_dir())
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target_dir);
    if release {
        cmd.arg("--release");
    }

    // Mirror the feature selection of the outer test run so that each feature
    // combination is verified against a matching `.so`. `HDR_FEATURES` is set
    // by `scripts/run_all_features.sh`.
    if std::env::var_os("HDR_NO_DEFAULT_FEATURES").is_some() {
        cmd.arg("--no-default-features");
    }
    if let Ok(f) = std::env::var("HDR_FEATURES") {
        if !f.trim().is_empty() {
            cmd.arg("--features").arg(f);
        }
    }

    let out = cmd.output().expect("failed to run `cargo build --lib`");
    assert!(
        out.status.success(),
        "`cargo build --lib` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let so = target_dir
        .join(if release { "release" } else { "debug" })
        .join(name);
    assert!(
        so.is_file(),
        "cargo build --lib did not produce {}",
        so.display()
    );

    // Freshness guard: the .so must be at least as new as the sources it is
    // built from. If this ever trips, the tests would be running against stale
    // code and must not be trusted.
    let src = manifest_dir().join("src").join("lib.rs");
    if let (Ok(a), Ok(b)) = (so.metadata(), src.metadata()) {
        if let (Ok(ta), Ok(tb)) = (a.modified(), b.modified()) {
            assert!(
                ta >= tb,
                "STALE ARTIFACT: {} ({ta:?}) is older than {} ({tb:?})",
                so.display(),
                src.display()
            );
        }
    }

    so
}

/// Both libraries, kept alive for the duration of a test.
pub struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c: HdrBitrateFn,
    pub rust: HdrBitrateFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Libs {
    pub fn load() -> Libs {
        let c_path = ensure_c_so();
        let rust_path = ensure_rust_so();

        unsafe {
            let c_lib = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

            let c_sym: libloading::Symbol<HdrBitrateFn> = c_lib
                .get(b"hdr_bitrate\0")
                .expect("C .so does not export `hdr_bitrate`");
            let rust_sym: libloading::Symbol<HdrBitrateFn> = rust_lib
                .get(b"hdr_bitrate\0")
                .expect("Rust .so does not export `hdr_bitrate`");

            let c = *c_sym;
            let rust = *rust_sym;

            Libs {
                _c: c_lib,
                _rust: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    }

    /// Call both implementations on the same buffer and return `(c, rust)`.
    pub fn both(&self, h: &[u8]) -> (u32, u32) {
        assert!(h.len() >= 3, "hdr_bitrate reads h[1] and h[2]");
        unsafe { ((self.c)(h.as_ptr()), (self.rust)(h.as_ptr())) }
    }

    /// Call both on a raw pointer (for pointer-shape / short-buffer tests).
    pub unsafe fn both_raw(&self, h: *const u8) -> (u32, u32) {
        unsafe { ((self.c)(h), (self.rust)(h)) }
    }

    /// Assert byte-for-byte agreement, with a descriptive failure message.
    pub fn assert_eq_on(&self, h: &[u8], ctx: &str) -> u32 {
        let (c, r) = self.both(h);
        assert_eq!(
            c, r,
            "DIVERGENCE [{ctx}] h={:02x?}: C returned {c}, Rust returned {r}",
            &h[..3.min(h.len())]
        );
        c
    }
}

/// Build an `h` buffer for a given index triple, filling every bit the C never
/// reads with pseudo-random noise so that don't-care bits are exercised.
///
/// * `i` selects bit 3 of `h[1]`      (`!!(h[1] & 0x8)`)
/// * `layer` is the raw 2-bit field   (`(h[1] >> 1) & 3`), so `j = layer - 1`
/// * `k` is the bitrate index         (`h[2] >> 4`)
pub fn make_header(i: u32, layer: u32, k: u32, rng: &mut Rng) -> [u8; 4] {
    assert!(i <= 1 && layer <= 3 && k <= 15);
    let noise = rng.next_u32();

    let h0 = (noise & 0xff) as u8;
    // bit0 and bits4..7 of h[1] are never read -> randomize them.
    let h1 = ((noise >> 8) as u8 & 0xF1) | ((i as u8) << 3) | ((layer as u8) << 1);
    // low nibble of h[2] is never read -> randomize it.
    let h2 = ((k as u8) << 4) | ((noise >> 16) as u8 & 0x0F);
    let h3 = (noise >> 24) as u8;

    // Sanity: the noise must not have perturbed the fields the C reads.
    debug_assert_eq!(u32::from(h1 & 0x8 != 0), i);
    debug_assert_eq!(((h1 >> 1) & 3) as u32, layer);
    debug_assert_eq!((h2 >> 4) as u32, k);

    [h0, h1, h2, h3]
}

/// The flat byte offset the C computes into its 90-byte `halfrate` table.
pub fn flat_offset(i: i32, layer: i32, k: i32) -> i32 {
    i * 45 + (layer - 1) * 15 + k
}

/// Deterministic SplitMix64 PRNG — fixed seed for reproducibility, no crates.
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// Number of randomized inputs used per `CONFIGS.md` row.
pub const ITERS: usize = 500;
