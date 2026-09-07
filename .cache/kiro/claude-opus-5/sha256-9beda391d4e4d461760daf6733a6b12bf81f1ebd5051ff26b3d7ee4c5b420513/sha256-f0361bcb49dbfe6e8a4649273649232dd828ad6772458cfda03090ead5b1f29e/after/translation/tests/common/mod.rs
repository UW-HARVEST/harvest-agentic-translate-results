//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes their exported symbols. Rust functions are NEVER called
//! directly — every call goes through the `cdylib`'s `#[no_mangle]` exports,
//! exactly as an external C consumer would.

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;

pub type HashBytesFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type SipHashFn = unsafe extern "C" fn(c_int);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Prefer the profile the currently running test binary was built with, so
    // `cargo test` (debug) and `cargo test --release` each load a fresh .so.
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test>-<hash>
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            let p = profile_dir.join("libsiphash_lib.so");
            if p.exists() {
                return p;
            }
        }
    }
    let target = workspace_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libsiphash_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libsiphash_lib.so not found under {}; build it with: cd translation && cargo build --release",
        target.display()
    )
}

/// `cargo test` does **not** rebuild a `cdylib`-only lib target when the
/// integration test does not `use` the crate — so the `.so` on disk can silently
/// be stale, which would make every differential test vacuous. Fail loudly
/// instead.
fn assert_so_fresh(so: &std::path::Path) {
    let src = workspace_root()
        .join("translation")
        .join("src")
        .join("lib.rs");
    let mtime = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
    };
    let so_t = mtime(so);
    let src_t = mtime(&src);
    assert!(
        so_t >= src_t,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib-only lib target. Run:\n\
         \x20 cd translation && cargo build --release --lib --example siphash_driver && cargo test --release",
        so.display(),
        src.display()
    );
}

/// Path to the C `.so` under test.
pub fn c_so_path() -> PathBuf {
    find_c_so()
}

/// Path to the Rust `.so` under test (freshness-checked).
pub fn rust_so_path() -> PathBuf {
    let p = find_rust_so();
    assert_so_fresh(&p);
    p
}

#[allow(dead_code)]
pub struct Libs {
    // Kept alive for the lifetime of the loaded symbols.
    _c: Library,
    _rust: Library,
    pub c_hash_bytes: HashBytesFn,
    pub rust_hash_bytes: HashBytesFn,
    pub c_siphash: SipHashFn,
    pub rust_siphash: SipHashFn,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c_path = find_c_so();
            let r_path = find_rust_so();
            assert_so_fresh(&r_path);
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));

            let c_hash: Symbol<HashBytesFn> = c
                .get(b"stbds_hash_bytes\0")
                .expect("C .so must export stbds_hash_bytes");
            let r_hash: Symbol<HashBytesFn> = rust
                .get(b"stbds_hash_bytes\0")
                .expect("Rust .so must export stbds_hash_bytes");
            let c_sip: Symbol<SipHashFn> =
                c.get(b"siphash\0").expect("C .so must export siphash");
            let r_sip: Symbol<SipHashFn> = rust
                .get(b"siphash\0")
                .expect("Rust .so must export siphash");

            let c_hash_bytes = *c_hash;
            let rust_hash_bytes = *r_hash;
            let c_siphash = *c_sip;
            let rust_siphash = *r_sip;

            Libs {
                _c: c,
                _rust: rust,
                c_hash_bytes,
                rust_hash_bytes,
                c_siphash,
                rust_siphash,
            }
        }
    }

    /// Call `stbds_hash_bytes` in both libraries on the same bytes.
    pub fn both_hash(&self, buf: &mut [u8], len: usize, seed: usize) -> (usize, usize) {
        let p = buf.as_mut_ptr() as *mut c_void;
        unsafe {
            let c = (self.c_hash_bytes)(p, len, seed);
            let r = (self.rust_hash_bytes)(p, len, seed);
            (c, r)
        }
    }

    pub fn assert_hash_eq(&self, buf: &mut [u8], len: usize, seed: usize, ctx: &str) {
        let (c, r) = self.both_hash(buf, len, seed);
        assert_eq!(
            c,
            r,
            "{ctx}: C=0x{c:016x} Rust=0x{r:016x} len={len} seed=0x{seed:016x} data={:02x?}",
            &buf[..len.min(buf.len())]
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

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
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = (self.next_u64() & 0xff) as u8;
        }
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

pub const FIXED_SEED: u64 = 0x5eed_1234_5678_9abc;

// ---------------------------------------------------------------------------
// stdout capture for `siphash()` — done in a child process so that no
// test-harness output can interleave with the library's `printf` output.
// ---------------------------------------------------------------------------

fn driver_path() -> PathBuf {
    // current_exe() == <root>/translation/target/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let p = profile_dir.join("examples").join("siphash_driver");
    if p.exists() {
        return p;
    }
    // Fall back to the other profile directory.
    let target = workspace_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let q = target.join(profile).join("examples").join("siphash_driver");
        if q.exists() {
            return q;
        }
    }
    panic!(
        "siphash_driver example not built; expected {} (build with: cargo build --release --example siphash_driver)",
        p.display()
    )
}

/// Run the driver against one `.so` and return its raw stdout bytes.
fn run_driver(so: &std::path::Path, init: i32) -> Vec<u8> {
    if so.file_name().and_then(|n| n.to_str()) == Some("libsiphash_lib.so") {
        assert_so_fresh(so);
    }
    let out = std::process::Command::new(driver_path())
        .arg(so)
        .arg(init.to_string())
        .output()
        .unwrap_or_else(|e| panic!("spawn siphash_driver: {e}"));
    assert!(
        out.status.success(),
        "siphash_driver {} {init} failed: status={:?} stderr={}",
        so.display(),
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// `siphash(init)` stdout from the C `.so`.
pub fn c_siphash_stdout(init: i32) -> Vec<u8> {
    run_driver(&find_c_so(), init)
}

/// `siphash(init)` stdout from the Rust `.so`.
pub fn rust_siphash_stdout(init: i32) -> Vec<u8> {
    run_driver(&find_rust_so(), init)
}

