//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `max_size_frame`
//! only through its exported C symbol. The Rust implementation is never called
//! directly as a Rust function, so the `#[unsafe(no_mangle)] extern "C"` export
//! wrapper and the C ABI are part of what is under test.

// Each test crate includes this module and uses a different subset of it.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The FFI signature from `c_src/include/lib.h`:
/// `tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth);`
pub type MaxSizeFrameFn = unsafe extern "C" fn(u32, u32, u32) -> u32;

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: MaxSizeFrameFn,
    rust_fn: MaxSizeFrameFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Impls {
    /// Call the C `.so` export.
    #[inline]
    pub fn c(&self, blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
        unsafe { (self.c_fn)(blocksize, channels, bitdepth) }
    }

    /// Call the Rust `.so` export.
    #[inline]
    pub fn rust(&self, blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
        unsafe { (self.rust_fn)(blocksize, channels, bitdepth) }
    }

    /// Differential assertion: both `.so`s must agree byte-for-byte.
    #[track_caller]
    pub fn assert_eq(&self, blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
        let c = self.c(blocksize, channels, bitdepth);
        let r = self.rust(blocksize, channels, bitdepth);
        assert_eq!(
            c, r,
            "DIVERGENCE max_size_frame(blocksize={blocksize} (0x{blocksize:08x}), \
             channels={channels} (0x{channels:08x}), bitdepth={bitdepth} (0x{bitdepth:08x})): \
             C returned {c} (0x{c:08x}), Rust returned {r} (0x{r:08x})"
        );
        c
    }

    /// Differential assertion plus an independently derived expected value.
    #[track_caller]
    pub fn assert_eq_expect(&self, blocksize: u32, channels: u32, bitdepth: u32, expected: u32) {
        let c = self.assert_eq(blocksize, channels, bitdepth);
        assert_eq!(
            c, expected,
            "C result for max_size_frame({blocksize}, {channels}, {bitdepth}) was {c}, \
             but the value derived from the C source semantics is {expected}"
        );
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Newest mtime among the given paths (recursing into directories).
fn newest_mtime(paths: &[PathBuf]) -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    for p in paths {
        if let Ok(md) = std::fs::metadata(p) {
            if md.is_dir() {
                if let Ok(rd) = std::fs::read_dir(p) {
                    let kids: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
                    let t = newest_mtime(&kids);
                    if t > newest {
                        newest = t;
                    }
                }
            } else if let Ok(t) = md.modified() {
                if t > newest {
                    newest = t;
                }
            }
        }
    }
    newest
}

/// `cargo test` does **not** build `crate-type = ["cdylib"]` artifacts: it only
/// builds the rlib it links into the test binaries. Without this step the tests
/// would silently `dlopen` a **stale** `.so` left over from an earlier
/// `cargo build` and pass no matter what `src/lib.rs` says. So build the cdylib
/// explicitly for the profile this test binary was compiled with, then verify
/// the artifact really is newer than the sources.
fn build_rust_cdylib(profile_dir: &Path) {
    let profile = profile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .expect("target/<profile> dir name");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");

    let mut cmd = Command::new(&cargo);
    cmd.arg("build").arg("--manifest-path").arg(&manifest);
    match profile {
        "debug" => {}
        "release" => {
            cmd.arg("--release");
        }
        other => {
            cmd.arg("--profile").arg(other);
        }
    }
    // Preserve the feature selection the test binary itself was built with, so
    // that `cargo test --no-default-features --features X` builds a matching .so.
    if let Ok(feats) = std::env::var("DIFFTEST_CARGO_FEATURE_ARGS") {
        for a in feats.split_whitespace() {
            cmd.arg(a);
        }
    }

    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run `{cargo} build`: {e}"));
    assert!(
        out.status.success(),
        "`cargo build` for the cdylib failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The C `.so`. `c_src/CMakeLists.txt` names the library after the *parent*
/// directory, so the filename is environment-dependent: glob for `lib*.so`.
fn find_c_so() -> PathBuf {
    // Allow pointing at an alternative C build (e.g. a copy compiled at a
    // different optimization level) to prove the translation matches the C
    // *semantics* rather than one particular codegen.
    if let Ok(p) = std::env::var("DIFFTEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DIFFTEST_C_SO does not exist: {}", p.display());
        return p;
    }
    let build_dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with("lib") && name.ends_with(".so")
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build_dir.display(),
        candidates
    );
    let so = candidates.pop().unwrap();

    // Guard against comparing against a stale C build.
    let root = workspace_root();
    let src_time = newest_mtime(&[
        root.join("c_src").join("src"),
        root.join("c_src").join("include"),
        root.join("c_src").join("CMakeLists.txt"),
    ]);
    let so_time = std::fs::metadata(&so).and_then(|m| m.modified()).unwrap();
    assert!(
        so_time >= src_time,
        "STALE C .so: {} is older than the C sources. Rebuild it:\n  \
         cd c_src/build && cmake --build .",
        so.display()
    );
    so
}

/// The Rust cdylib for the profile the test binary itself was built with
/// (`target/debug/` or `target/release/`), derived from `current_exe()`:
/// `target/<profile>/deps/<test>-<hash>` -> `target/<profile>/`.
fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test exe should live in target/<profile>/deps/")
        .to_path_buf();

    // Build it: `cargo test` alone does not produce cdylib artifacts.
    build_rust_cdylib(&profile_dir);

    let so = profile_dir.join("libmax_size_frame_lib.so");
    assert!(
        so.is_file(),
        "Rust cdylib not found at {}\n(`[lib] name = \"max_size_frame_lib\"`, \
         crate-type = [\"cdylib\"]; run `cargo build` for this profile)",
        so.display()
    );

    // Freshness guard: a stale .so would make every differential test vacuous.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_time = newest_mtime(&[manifest_dir.join("src"), manifest_dir.join("Cargo.toml")]);
    let so_time = std::fs::metadata(&so).and_then(|m| m.modified()).unwrap();
    assert!(
        so_time >= src_time,
        "STALE Rust .so: {} is older than src/. The differential tests would be \
         vacuous (they would test an old build). Run `cargo build` for this profile.",
        so.display()
    );
    so
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

/// Load both `.so`s once per test binary.
pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

            // Resolve by exact symbol name in both -- this is the export test.
            let c_sym: Symbol<MaxSizeFrameFn> = c_lib
                .get(b"max_size_frame\0")
                .expect("C .so must export `max_size_frame`");
            let rust_sym: Symbol<MaxSizeFrameFn> = rust_lib
                .get(b"max_size_frame\0")
                .expect("Rust .so must export `max_size_frame` (check #[no_mangle])");

            let c_fn = *c_sym;
            let rust_fn = *rust_sym;

            Impls {
                c_fn,
                rust_fn,
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_path,
                rust_path,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed for reproducible property tests
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5F37_59DF_C0FF_EE01;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo.wrapping_add((self.next_u64() % span) as u32)
    }
    pub fn pick(&mut self, xs: &[u32]) -> u32 {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Number of randomized draws per `CONFIGS.md` row.
pub const DRAWS: usize = 20_000;

/// Value sets used across rows, taken from the constants the C branches on
/// (`2`, `32`, `8`, `7`, `18`) plus the documented FLAC ranges and one-past values.
pub const TYPICAL_BITDEPTHS: &[u32] = &[4, 8, 12, 16, 20, 24, 32];
pub const TYPICAL_BLOCKSIZES: &[u32] = &[192, 576, 1152, 2304, 4608, 4096, 8192, 16384, 65535];

/// Reference model transcribed directly from `c_src/src/lib.c`, used only as a
/// third opinion in the error-path tests (the C `.so` remains ground truth).
pub fn model(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let b = |c: bool| if c { 1u32 } else { 0u32 };
    let t1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(b(channels != 2)));
    let t2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(b(channels == 2));
    let t3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(b(bitdepth != 32)))
        .wrapping_mul(b(channels == 2));
    let sum = t1.wrapping_add(t2).wrapping_add(t3).wrapping_add(7);
    18u32.wrapping_add(channels).wrapping_add(sum / 8)
}
