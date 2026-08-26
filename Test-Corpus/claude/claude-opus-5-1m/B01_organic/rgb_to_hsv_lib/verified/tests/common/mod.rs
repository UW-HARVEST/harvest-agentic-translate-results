//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading`
//! and called only through their exported `rgb_to_hsv` symbol, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// ABI of the single exported entry point.
pub type RgbToHsvFn = unsafe extern "C" fn(*mut f32, *const f32);

pub struct DynLib {
    // Keep the library alive for as long as the function pointer is used.
    _lib: Library,
    pub rgb_to_hsv: RgbToHsvFn,
    pub name: &'static str,
}

impl DynLib {
    fn open(path: &PathBuf, name: &'static str) -> DynLib {
        assert!(
            path.exists(),
            "shared object {} not found. Build it first:\n  \
             C:    cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
             Rust: cargo build",
            path.display()
        );
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let f = unsafe {
            let sym: Symbol<RgbToHsvFn> = lib
                .get(b"rgb_to_hsv\0")
                .unwrap_or_else(|e| panic!("dlsym(rgb_to_hsv) in {} failed: {e}", path.display()));
            *sym
        };
        DynLib {
            _lib: lib,
            rgb_to_hsv: f,
            name,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test executable.
fn target_profile_dir() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // test binary file name
    if p.file_name().map(|n| n == "deps").unwrap_or(false) {
        p.pop();
    }
    p
}

/// Path of the reference C shared object. `HARVEST_C_SO` allows pointing the
/// whole suite at a differently-compiled C build (e.g. `-O2`) to prove the Rust
/// matches the C at any optimisation level.
pub fn c_lib_path() -> PathBuf {
    match std::env::var_os("HARVEST_C_SO") {
        Some(p) => PathBuf::from(p),
        None => manifest_dir().join("c_src/build/libtranslated_rust.so"),
    }
}

/// The `.so` for the *profile this test binary was built with*. There is
/// deliberately no cross-profile fallback: loading a `target/release` object
/// from a debug test run would silently test a stale/foreign artifact.
pub fn rust_lib_path() -> PathBuf {
    target_profile_dir().join("librgb_to_hsv_lib.so")
}

fn mtime(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// `cargo test` does **not** emit the `cdylib` artifact (integration tests do
/// not link it), so a plain `cargo test` would happily run against a stale
/// `.so`. Build it explicitly (once per process) and then verify it is newer
/// than the sources it was built from.
fn ensure_rust_lib() {
    use std::sync::OnceLock;
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let profile_dir = target_profile_dir();
        let profile = profile_dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "debug".to_string());

        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.arg("build").arg("--lib").arg("--quiet");
        match profile.as_str() {
            "debug" => {}
            "release" => {
                cmd.arg("--release");
            }
            other => {
                cmd.arg("--profile").arg(other);
            }
        }
        cmd.current_dir(manifest_dir());
        let _ = cmd.status();
    });

    let so = rust_lib_path();
    let so_t = mtime(&so).unwrap_or_else(|| {
        panic!(
            "Rust shared object {} does not exist.\nBuild it with `cargo build` \
             (or `cargo build --release`) before running the tests.",
            so.display()
        )
    });
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest_dir().join(src);
        if let Some(t) = mtime(&p) {
            assert!(
                so_t >= t,
                "{} is OLDER than {} — the tests would run against a stale library.\n\
                 Run `cargo build` (matching the test profile) first.",
                so.display(),
                p.display()
            );
        }
    }
}

/// Build the C shared object on demand (once per process) and reject a stale one.
fn ensure_c_lib() {
    use std::sync::OnceLock;
    if std::env::var_os("HARVEST_C_SO").is_some() {
        // Externally supplied build: use it as-is.
        assert!(
            c_lib_path().exists(),
            "HARVEST_C_SO={} does not exist",
            c_lib_path().display()
        );
        return;
    }
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let c_dir = manifest_dir().join("c_src");
        let build_dir = c_dir.join("build");
        let so = c_lib_path();
        let stale = match (mtime(&so), mtime(&c_dir.join("src/lib.c"))) {
            (Some(a), Some(b)) => a < b,
            (None, _) => true,
            _ => false,
        };
        if stale {
            let _ = std::fs::create_dir_all(&build_dir);
            let _ = std::process::Command::new("cmake")
                .args([".."])
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .current_dir(&build_dir)
                .status();
            let _ = std::process::Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .status();
        }
    });

    let so = c_lib_path();
    let so_t = mtime(&so).unwrap_or_else(|| {
        panic!(
            "C shared object {} does not exist. Build it with:\n  cd c_src && mkdir -p build \
             && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            so.display()
        )
    });
    let src = manifest_dir().join("c_src/src/lib.c");
    if let Some(t) = mtime(&src) {
        assert!(
            so_t >= t,
            "{} is OLDER than {} — rebuild the C library first.",
            so.display(),
            src.display()
        );
    }
}

pub fn load_c() -> DynLib {
    ensure_c_lib();
    DynLib::open(&c_lib_path(), "C")
}

pub fn load_rust() -> DynLib {
    ensure_rust_lib();
    DynLib::open(&rust_lib_path(), "Rust")
}

/// Both libraries, ready for differential calls.
pub struct Pair {
    pub c: DynLib,
    pub rs: DynLib,
}

pub fn load_pair() -> Pair {
    Pair {
        c: load_c(),
        rs: load_rust(),
    }
}

/// Poison value written into the output buffers before each call so that a
/// missing store is detected instead of silently reading a stale/zero value.
pub const POISON: u32 = 0xDEAD_BEEF;

fn poisoned() -> [f32; 3] {
    [f32::from_bits(POISON); 3]
}

impl Pair {
    /// Call both implementations with disjoint buffers and return
    /// `(c_out_bits, rust_out_bits)`.
    pub fn call_both(&self, src: [f32; 3]) -> ([u32; 3], [u32; 3]) {
        let mut c_out = poisoned();
        let mut r_out = poisoned();
        let c_src = src;
        let r_src = src;
        unsafe {
            (self.c.rgb_to_hsv)(c_out.as_mut_ptr(), c_src.as_ptr());
            (self.rs.rgb_to_hsv)(r_out.as_mut_ptr(), r_src.as_ptr());
        }
        (bits3(&c_out), bits3(&r_out))
    }

    /// Differential assertion for disjoint buffers.
    pub fn assert_same(&self, row: &str, src: [f32; 3]) {
        let (c, r) = self.call_both(src);
        assert_bits_eq(row, src, c, r);
    }

    /// In-place call: `dest == src` (row 22 of CONFIGS.md).
    pub fn assert_same_in_place(&self, row: &str, src: [f32; 3]) {
        let mut c_buf = src;
        let mut r_buf = src;
        unsafe {
            let p = c_buf.as_mut_ptr();
            (self.c.rgb_to_hsv)(p, p as *const f32);
            let p = r_buf.as_mut_ptr();
            (self.rs.rgb_to_hsv)(p, p as *const f32);
        }
        assert_bits_eq(
            &format!("{row} [in-place dest==src]"),
            src,
            bits3(&c_buf),
            bits3(&r_buf),
        );
        // In-place must agree with the disjoint-buffer result as well.
        let (c_disjoint, _) = self.call_both(src);
        assert_bits_eq(
            &format!("{row} [in-place vs disjoint]"),
            src,
            c_disjoint,
            bits3(&r_buf),
        );
    }
}

pub fn bits3(v: &[f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

pub fn show(bits: [u32; 3]) -> String {
    format!(
        "[{} (0x{:08x}), {} (0x{:08x}), {} (0x{:08x})]",
        f32::from_bits(bits[0]),
        bits[0],
        f32::from_bits(bits[1]),
        bits[1],
        f32::from_bits(bits[2]),
        bits[2]
    )
}

pub fn assert_bits_eq(row: &str, src: [f32; 3], c: [u32; 3], r: [u32; 3]) {
    assert_ne!(
        c,
        [POISON; 3],
        "{row}: C wrote nothing for src={src:?} (output still poisoned)"
    );
    assert_eq!(
        c,
        r,
        "\n{row}\n  src   = [{} (0x{:08x}), {} (0x{:08x}), {} (0x{:08x})]\n  C     = {}\n  Rust  = {}\n",
        src[0], src[0].to_bits(), src[1], src[1].to_bits(), src[2], src[2].to_bits(),
        show(c),
        show(r)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    /// A random bit pattern reinterpreted as `f32` (any class).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// `n` in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}
