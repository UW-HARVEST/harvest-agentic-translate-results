//! Shared harness: loads the C reference `.so` and the Rust `.so` and exposes
//! their `md5_digest` exports so both are called strictly through the FFI
//! boundary (never by calling Rust functions directly).

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Mirrors `struct tflac_md5` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TflacMd5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

pub type Md5DigestFn = unsafe extern "C" fn(*const TflacMd5, *mut u8);

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Directory the current test binary lives in, walked up past `deps/`.
/// For `target/<profile>/deps/test-<hash>` this yields `target/<profile>`.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("test exe parent").to_path_buf();
    if deps.file_name().and_then(|n| n.to_str()) == Some("deps") {
        deps.parent().expect("profile dir").to_path_buf()
    } else {
        deps
    }
}

/// Locate the C reference shared library inside `c_src/build`.
fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });

    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("lib"))
        })
        .collect();
    found.sort();

    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no lib*.so found in {}; build the C library first",
            build_dir.display()
        )
    })
}

/// Locate the Rust cdylib produced by this crate.
fn find_rust_so() -> PathBuf {
    let name = "libmd5_digest_lib.so";
    let mut candidates = vec![target_profile_dir().join(name)];
    // Fallbacks for unusual invocations.
    let target = workspace_root().join("translation").join("target");
    candidates.push(target.join("debug").join(name));
    candidates.push(target.join("release").join(name));

    for cand in &candidates {
        if cand.is_file() {
            return cand.clone();
        }
    }
    panic!(
        "could not find {name}; looked in: {}",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Both implementations, loaded as dynamic libraries.
pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: Md5DigestFn,
    pub rust: Md5DigestFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Impls {
    pub fn load() -> Self {
        let c_path = find_c_so();
        let rust_path = find_rust_so();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

            let c_sym: Symbol<Md5DigestFn> = c_lib
                .get(b"md5_digest\0")
                .expect("C .so must export md5_digest");
            let rust_sym: Symbol<Md5DigestFn> = rust_lib
                .get(b"md5_digest\0")
                .expect("Rust .so must export md5_digest");

            let c = *c_sym;
            let rust = *rust_sym;

            Self {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    }

    /// Call both exports with `m` into 32-byte buffers pre-filled with a
    /// sentinel, so writes past the documented 16 bytes are also detected.
    /// Returns `(c_buf, rust_buf)`.
    pub fn digest_both(&self, m: &TflacMd5, sentinel: u8) -> ([u8; 32], [u8; 32]) {
        let mut c_buf = [sentinel; 32];
        let mut rust_buf = [sentinel; 32];
        unsafe {
            (self.c)(m as *const TflacMd5, c_buf.as_mut_ptr());
            (self.rust)(m as *const TflacMd5, rust_buf.as_mut_ptr());
        }
        (c_buf, rust_buf)
    }

    /// Assert C and Rust agree byte-for-byte for `m`.
    pub fn assert_matches(&self, m: &TflacMd5) {
        for &sentinel in &[0x00u8, 0xFF, 0xA5] {
            let (c_buf, rust_buf) = self.digest_both(m, sentinel);
            assert_eq!(
                c_buf, rust_buf,
                "mismatch for {m:?} (sentinel {sentinel:#04x})\n  C   : {c_buf:02x?}\n  Rust: {rust_buf:02x?}"
            );
            // Neither implementation may touch bytes beyond the first 16.
            assert!(
                c_buf[16..].iter().all(|&b| b == sentinel),
                "C wrote past 16 bytes for {m:?}"
            );
            assert!(
                rust_buf[16..].iter().all(|&b| b == sentinel),
                "Rust wrote past 16 bytes for {m:?}"
            );
        }
    }
}

/// Deterministic 64-bit xorshift PRNG for reproducible fuzzing.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}
