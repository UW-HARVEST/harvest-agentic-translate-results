//! Shared harness for the differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called only through their exported `hdr_bitrate` symbol. The Rust crate is
//! never linked directly, so the `#[no_mangle] extern "C"` wrapper is part of
//! what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use libloading::{Library, Symbol};

/// `unsigned hdr_bitrate(const uint8_t *h)`
pub type HdrBitrateFn = unsafe extern "C" fn(*const u8) -> u32;

/// The crate root (`translation/`).
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The repository root, i.e. the parent of `translation/`.
pub fn work_dir() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn first_so_in(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    found.pop()
}

/// Path of the C shared object, building it with CMake if it is not there yet.
///
/// `CMakeLists.txt` derives the library name from the *parent directory name*,
/// so the file name is not fixed -- glob for it instead of hard-coding.
pub fn c_so_path() -> PathBuf {
    let c_src = work_dir().join("c_src");
    let build = c_src.join("build");

    if let Some(p) = first_so_in(&build) {
        return p;
    }

    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build)
        .output()
        .expect("failed to run cmake (is cmake installed?)");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let out = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(&build)
        .output()
        .expect("failed to run cmake --build");
    assert!(
        out.status.success(),
        "cmake build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    first_so_in(&build).expect("no .so produced in c_src/build")
}

/// Path of the Rust `cdylib`, located relative to the running test binary so
/// that it is found for any profile (`debug`, `release`, ...).
///
/// The crate's only `crate-type` is `cdylib`, and `cargo test` does *not* build
/// a cdylib-only lib target as a side effect of building the integration
/// tests. If the artifact is missing we build it, so a bare `cargo test` works.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");

    let find = || -> Option<PathBuf> {
        // .../target/<profile>/deps/<test>-<hash>
        exe.ancestors()
            .skip(1)
            .map(|d| d.join("libhdr_bitrate_lib.so"))
            .find(|c| c.is_file())
    };

    if let Some(p) = find() {
        return p;
    }

    // `--offline` because the crates.io index is not reachable from here and
    // every dependency is already vendored in the local cargo cache.
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.arg("build").arg("--offline").current_dir(manifest_dir());
    if exe.components().any(|c| c.as_os_str() == "release") {
        cmd.arg("--release");
    }
    let out = cmd.output().expect("failed to spawn cargo build");
    assert!(
        out.status.success(),
        "cargo build of the cdylib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    find().unwrap_or_else(|| {
        panic!(
            "libhdr_bitrate_lib.so still not found near {} after cargo build",
            exe.display()
        )
    })
}

/// A loaded implementation. The `Library` is kept alive alongside the symbol.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    func: HdrBitrateFn,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            let sym: Symbol<HdrBitrateFn> = lib
                .get(b"hdr_bitrate\0")
                .unwrap_or_else(|e| panic!("dlsym hdr_bitrate in {}: {e}", path.display()));
            let func = *sym;
            Impl {
                name,
                path,
                _lib: lib,
                func,
            }
        }
    }

    /// Call `hdr_bitrate` through the `.so`'s exported symbol.
    ///
    /// # Safety
    /// `h` must point at (at least) 3 readable bytes, or be deliberately
    /// invalid in a test that expects a fault.
    pub unsafe fn call(&self, h: *const u8) -> u32 {
        (self.func)(h)
    }
}

/// Both implementations, loaded from their respective shared objects.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("C", c_so_path()),
        rust: Impl::load("Rust", rust_so_path()),
    }
}

impl Pair {
    /// Call both through FFI on the same buffer and assert byte-identical
    /// results. Returns the agreed value.
    #[track_caller]
    pub fn assert_same(&self, buf: &[u8], ctx: &str) -> u32 {
        assert!(buf.len() >= 3, "buffer must hold h[0..=2]");
        let (a, b) = unsafe { (self.c.call(buf.as_ptr()), self.rust.call(buf.as_ptr())) };
        assert_eq!(
            a, b,
            "DIVERGENCE {ctx}: C returned {a}, Rust returned {b} \
             (h[0]={:#04x} h[1]={:#04x} h[2]={:#04x})",
            buf[0], buf[1], buf[2]
        );
        a
    }

    /// As `assert_same`, but for a raw pointer (guard-page / null tests).
    ///
    /// # Safety
    /// See `Impl::call`.
    #[track_caller]
    pub unsafe fn assert_same_ptr(&self, h: *const u8, ctx: &str) -> u32 {
        let (a, b) = (self.c.call(h), self.rust.call(h));
        assert_eq!(a, b, "DIVERGENCE {ctx}: C returned {a}, Rust returned {b}");
        a
    }
}

/// Deterministic, dependency-free PRNG (SplitMix64) so every randomized row is
/// reproducible.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

/// The three fields the C decodes out of `h[1]`/`h[2]`.
pub const PLANES: [u32; 2] = [0, 1];
pub const LAYER_BITS: [u32; 4] = [0, 1, 2, 3];
pub const RATE_NIBBLES: std::ops::Range<u32> = 0..16;

/// Build an `h[1]` byte carrying `plane` and `layer_bits`, with all bits the C
/// ignores (bit 0 and bits 4..7) taken from `noise`.
pub fn make_h1(plane: u32, layer_bits: u32, noise: u8) -> u8 {
    debug_assert!(plane < 2 && layer_bits < 4);
    let ignored = noise & 0xF1; // bits 0, 4, 5, 6, 7
    (((plane as u8) << 3) | ((layer_bits as u8) << 1) | ignored) as u8
}

/// Build an `h[2]` byte carrying `rate`, low nibble taken from `noise`.
pub fn make_h2(rate: u32, noise: u8) -> u8 {
    debug_assert!(rate < 16);
    ((rate as u8) << 4) | (noise & 0x0F)
}

/// Number of randomized iterations per `CONFIGS.md` row.
pub const ITERS: usize = 256;

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;
