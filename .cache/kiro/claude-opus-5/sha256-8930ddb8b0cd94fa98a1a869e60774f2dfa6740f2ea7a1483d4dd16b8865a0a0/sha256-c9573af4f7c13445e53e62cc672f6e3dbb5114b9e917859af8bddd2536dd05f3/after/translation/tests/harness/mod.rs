//! Shared harness: loads BOTH shared objects (the C one and the Rust one) via
//! `libloading` and exposes them behind an identical `extern "C"` signature.
//!
//! Nothing here calls the Rust implementation directly — the Rust side is
//! always reached through `dlopen`/`dlsym` on `libcrc16_lib.so`, exactly as an
//! external C consumer would, so the `#[no_mangle] extern "C"` wrapper is part
//! of what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);`
pub type Crc16Fn = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

pub struct Impls {
    // Libraries must outlive the function pointers; keep them alive.
    _c_lib: Library,
    _rust_lib: Library,
    pub c: Crc16Fn,
    pub rust: Crc16Fn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path, must_contain: Option<&str>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("so") {
            continue;
        }
        let name = p.file_name()?.to_str()?.to_string();
        if let Some(frag) = must_contain {
            if !name.contains(frag) {
                continue;
            }
        }
        found.push(p);
    }
    found.sort();
    found.into_iter().next()
}

fn newest_mtime(dir: &Path) -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let m = newest_mtime(&p);
                if m > newest {
                    newest = m;
                }
            } else if let Ok(md) = e.metadata() {
                if let Ok(m) = md.modified() {
                    if m > newest {
                        newest = m;
                    }
                }
            }
        }
    }
    newest
}

// ---------------------------------------------------------------------------
// Building the artifacts under test.
//
// IMPORTANT: `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` target
// (it compiles src/lib.rs as a test harness instead), so simply globbing
// target/<profile>/*.so silently loads a STALE .so from some earlier
// `cargo build`. That makes the whole differential suite vacuous - verified by
// mutation testing, where every injected bug survived.
//
// So the harness builds the cdylib itself, into a DEDICATED target dir. A
// separate --target-dir is required: cargo's build lock is per target dir, and
// reusing the one `cargo test` already holds would deadlock.
// ---------------------------------------------------------------------------

fn profile_name() -> &'static str {
    // current_exe = <target-dir>/<profile>/deps/<test-bin>
    match std::env::current_exe() {
        Ok(p) if p.to_string_lossy().contains("/release/") => "release",
        _ => "debug",
    }
}

fn build_rust_so() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let profile = profile_name();
            let target_dir = crate_dir().join("target").join("so-under-test");
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

            let mut cmd = std::process::Command::new(&cargo);
            cmd.current_dir(crate_dir())
                .arg("build")
                .arg("--lib")
                .arg("--target-dir")
                .arg(&target_dir);
            if profile == "release" {
                cmd.arg("--release");
            }
            // Don't inherit cargo-test-injected env that could confuse the
            // nested invocation.
            cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");
            cmd.env_remove("CARGO_MAKEFLAGS");

            let out = cmd
                .output()
                .unwrap_or_else(|e| panic!("failed to spawn `{cargo} build --lib`: {e}"));
            assert!(
                out.status.success(),
                "building the Rust cdylib under test failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );

            let dir = target_dir.join(profile);
            let so = find_so(&dir, Some("crc16_lib")).unwrap_or_else(|| {
                panic!(
                    "libcrc16_lib.so not found in {} after building it",
                    dir.display()
                )
            });

            // Staleness guard: the .so must be at least as new as every source
            // file. If this ever trips, the suite is testing the wrong binary.
            let so_mtime = std::fs::metadata(&so)
                .and_then(|m| m.modified())
                .expect("stat the built .so");
            let src_mtime = newest_mtime(&crate_dir().join("src"));
            assert!(
                so_mtime >= src_mtime,
                "STALE .so: {} is older than the newest file in src/. \
                 The differential tests would be vacuous.",
                so.display()
            );

            so
        })
        .clone()
}

fn build_c_so() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let c_src = workspace_root().join("c_src");
            let build = c_src.join("build");
            if find_so(&build, None).is_none() {
                std::fs::create_dir_all(&build).expect("mkdir c_src/build");
                let conf = std::process::Command::new("cmake")
                    .current_dir(&build)
                    .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                    .output()
                    .expect("spawn cmake configure");
                assert!(
                    conf.status.success(),
                    "cmake configure failed:\n{}",
                    String::from_utf8_lossy(&conf.stderr)
                );
                let bld = std::process::Command::new("cmake")
                    .current_dir(&build)
                    .args(["--build", "."])
                    .output()
                    .expect("spawn cmake build");
                assert!(
                    bld.status.success(),
                    "cmake build failed:\n{}",
                    String::from_utf8_lossy(&bld.stderr)
                );
            }
            find_so(&build, None).unwrap_or_else(|| {
                panic!(
                    "no C .so found in {}.\nBuild it:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                    build.display()
                )
            })
        })
        .clone()
}

fn c_so_path() -> PathBuf {
    build_c_so()
}

fn rust_so_path() -> PathBuf {
    build_rust_so()
}

impl Impls {
    pub fn load() -> Impls {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

            let c_sym: Symbol<Crc16Fn> = c_lib
                .get(b"crc16\0")
                .unwrap_or_else(|e| panic!("dlsym crc16 in C .so failed: {e}"));
            let rust_sym: Symbol<Crc16Fn> = rust_lib.get(b"crc16\0").unwrap_or_else(|e| {
                panic!("dlsym crc16 in Rust .so failed (missing #[no_mangle] export?): {e}")
            });

            let c = *c_sym;
            let rust = *rust_sym;

            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    }

    /// Call both implementations and assert byte-identical results.
    #[inline]
    pub fn check(&self, data: &[u8], len: u32, seed: u16, ctx: &str) -> u16 {
        let p = data.as_ptr();
        let cv = unsafe { (self.c)(p, len, seed) };
        let rv = unsafe { (self.rust)(p, len, seed) };
        assert_eq!(
            cv, rv,
            "DIVERGENCE [{ctx}]: len={len} seed=0x{seed:04x} C=0x{cv:04x} Rust=0x{rv:04x}\n  \
             first bytes={:02x?}",
            &data[..data.len().min(32)]
        );
        cv
    }

    /// Call both with a raw pointer (for null / bogus-pointer cases).
    #[inline]
    pub fn check_raw(&self, p: *const u8, len: u32, seed: u16, ctx: &str) -> u16 {
        let cv = unsafe { (self.c)(p, len, seed) };
        let rv = unsafe { (self.rust)(p, len, seed) };
        assert_eq!(
            cv, rv,
            "DIVERGENCE [{ctx}]: ptr={p:?} len={len} seed=0x{seed:04x} C=0x{cv:04x} Rust=0x{rv:04x}"
        );
        cv
    }

    #[inline]
    pub fn c_call(&self, data: &[u8], len: u32, seed: u16) -> u16 {
        unsafe { (self.c)(data.as_ptr(), len, seed) }
    }

    #[inline]
    pub fn rust_call(&self, data: &[u8], len: u32, seed: u16) -> u16 {
        unsafe { (self.rust)(data.as_ptr(), len, seed) }
    }
}

/// xorshift64* — deterministic, fixed seed, so every run is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x0BAD_C0DE_D15E_A5E5;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 1 } else { seed })
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    #[inline]
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish in `0..n`.
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
}
