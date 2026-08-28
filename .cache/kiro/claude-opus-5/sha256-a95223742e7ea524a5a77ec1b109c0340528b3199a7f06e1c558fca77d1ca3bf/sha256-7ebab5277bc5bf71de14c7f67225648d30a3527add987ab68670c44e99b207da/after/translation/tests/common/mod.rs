//! Shared harness: loads the C and Rust shared libraries through `libloading`
//! and exposes a differential comparison helper for `decode_base64`.

// Not every test binary uses every helper in here.
#![allow(dead_code)]

use std::ffi::{c_char, c_void};
use std::path::PathBuf;

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

pub type DecodeBase64Fn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

/// The tests `dlopen` the cdylib rather than linking it, so cargo has no
/// dependency edge that would rebuild `libdriver.so` when `src/lib.rs`
/// changes. Shell out to `cargo build` once per test binary to guarantee the
/// loaded `.so` matches the current sources.
pub fn ensure_cdylib_fresh() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var_os("XLAT_SKIP_REBUILD").is_some() {
            return;
        }
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let mut cmd = std::process::Command::new(cargo);
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
            .arg("build")
            .arg("--lib")
            .env("XLAT_SKIP_REBUILD", "1")
            // Don't inherit the parent cargo's jobserver / target-dir plumbing.
            .env_remove("CARGO_MAKEFLAGS")
            .env_remove("RUSTC_WORKSPACE_WRAPPER");

        if cfg!(not(debug_assertions)) {
            cmd.arg("--release");
        }
        // Reproduce the feature selection this test binary was compiled with.
        cmd.arg("--no-default-features");
        let feats = active_features();
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats.join(","));
        }

        match cmd.output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => panic!(
                "`cargo build --lib` failed while refreshing the cdylib:\n{}",
                String::from_utf8_lossy(&out.stderr)
            ),
            Err(e) => panic!("could not run cargo to refresh the cdylib: {e}"),
        }
    });
}

/// Features enabled for this test binary, derived from `CARGO_FEATURE_*`-style
/// `cfg(feature = ...)` checks. Kept in one place so new features only need to
/// be listed here.
fn active_features() -> Vec<&'static str> {
    // No `[features]` are declared in Cargo.toml today. When features are
    // added, append a `#[cfg(feature = "x")] v.push("x");` line for each.
    #[allow(unused_mut)]
    let mut v: Vec<&'static str> = Vec::new();
    v
}

/// Workspace root (the directory containing `c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    let root = workspace_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\nLooked in: {candidates:?}"
    );
}

fn rust_library_path() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");

    let mut candidates = vec![
        profile_dir.join("libdriver.so"),
        deps.join("libdriver.so"),
    ];
    // Fall back to the well-known cargo output directories.
    let root = workspace_root();
    for p in ["debug", "release"] {
        candidates.push(root.join("translation/target").join(p).join("libdriver.so"));
    }

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found; build it with `cargo build` inside translation/.\nLooked in: \
         {candidates:?}"
    );
}

/// Both implementations, loaded purely through their exported C ABI symbols.
pub struct Harness {
    _c_lib: Library,
    _rust_lib: Library,
    c_decode: DecodeBase64Fn,
    rust_decode: DecodeBase64Fn,
}

impl Harness {
    pub fn load() -> Self {
        ensure_cdylib_fresh();
        unsafe {
            let c_lib = Library::new(c_library_path()).expect("load C libdriver.so");
            let rust_lib = Library::new(rust_library_path()).expect("load Rust libdriver.so");

            let c_sym: Symbol<DecodeBase64Fn> =
                c_lib.get(b"decode_base64\0").expect("C decode_base64");
            let rust_sym: Symbol<DecodeBase64Fn> = rust_lib
                .get(b"decode_base64\0")
                .expect("Rust decode_base64 (no_mangle export missing?)");

            let c_decode = *c_sym;
            let rust_decode = *rust_sym;

            Harness {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_decode,
                rust_decode,
            }
        }
    }

    /// Call `decode_base64` in both libraries with the raw bytes `input`
    /// (a NUL terminator is appended) and assert the results are identical.
    ///
    /// The comparison covers the *entire* destination allocation, which the C
    /// code sizes as `strlen(src) + 1 + 13`, so trailing zero-fill from
    /// `calloc` is verified too — not just the NUL-terminated prefix.
    pub fn assert_same(&self, input: &[u8]) {
        assert!(
            !input.contains(&0),
            "test inputs must not contain embedded NUL bytes"
        );
        let mut src: Vec<u8> = input.to_vec();
        src.push(0);
        let ptr = src.as_ptr() as *const c_char;

        let (c_out, rust_out) = unsafe { ((self.c_decode)(ptr), (self.rust_decode)(ptr)) };

        assert_eq!(
            c_out.is_null(),
            rust_out.is_null(),
            "NULL-ness mismatch for input {:?}: C null={} rust null={}",
            String::from_utf8_lossy(input),
            c_out.is_null(),
            rust_out.is_null()
        );

        if c_out.is_null() {
            return;
        }

        // Full destination buffer size as computed by the C implementation.
        let total = input.len() + 1 + 13;
        let c_bytes = unsafe { std::slice::from_raw_parts(c_out as *const u8, total) }.to_vec();
        let rust_bytes =
            unsafe { std::slice::from_raw_parts(rust_out as *const u8, total) }.to_vec();

        unsafe {
            free(c_out as *mut c_void);
            free(rust_out as *mut c_void);
        }

        assert_eq!(
            c_bytes,
            rust_bytes,
            "output mismatch for input {:?} ({} bytes)\n  C   : {:02x?}\n  rust: {:02x?}",
            String::from_utf8_lossy(input),
            input.len(),
            c_bytes,
            rust_bytes
        );
    }

    /// Compare behaviour for a NULL `src` pointer.
    pub fn assert_same_null_input(&self) {
        let (c_out, rust_out) = unsafe {
            (
                (self.c_decode)(std::ptr::null()),
                (self.rust_decode)(std::ptr::null()),
            )
        };
        assert!(c_out.is_null(), "C should return NULL for NULL input");
        assert!(rust_out.is_null(), "Rust should return NULL for NULL input");
    }
}

/// Small deterministic PRNG (xorshift64*) so fuzz inputs are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }

    /// A random non-NUL byte in `1..=255`.
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
}
