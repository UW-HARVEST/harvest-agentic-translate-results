//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and exposes them through identical raw FFI function pointers, so every test
//! goes through the real export wrappers exactly like an external caller.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::path::{Path, PathBuf};

/// Mirror of `struct tflac_md5` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Md5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

/// `void md5_digest(const tflac_md5 *m, tflac_u8 out[16])`
pub type DigestFn = unsafe extern "C" fn(*const Md5, *mut u8);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// Locate the C shared object produced by `c_src/CMakeLists.txt`. The library
/// name is derived from the parent directory name, so glob for it.
pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}. Build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// Locate the Rust cdylib. Prefers the profile the tests were built with.
pub fn rust_so_path() -> PathBuf {
    let target = workspace_root().join("translation").join("target");
    let name = "libmd5_digest_lib.so";
    let profiles: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for p in profiles {
        let cand = target.join(p).join(name);
        if cand.is_file() {
            assert_not_stale(&cand);
            return cand;
        }
    }
    panic!(
        "could not find {} under {} (build it with `cargo build` / `cargo build --release`)",
        name,
        target.display()
    );
}

/// Guard against silently testing a `.so` that predates the current `src/`.
/// (`cargo test` on a `crate-type = ["cdylib"]` crate does not necessarily
/// refresh the shared object, so this must be checked explicitly.)
fn assert_not_stale(so: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs");
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    assert!(
        mtime(so) >= mtime(&src),
        "STALE ARTIFACT: {} is older than {}. Re-run `cargo build --release` \
         (or use ./run_all.sh) before testing.",
        so.display(),
        src.display()
    );
}

fn load(path: &Path) -> DigestFn {
    unsafe {
        // Leaked on purpose: keeps the library (and therefore the code the raw
        // function pointer points at) alive for the whole test process.
        let lib: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display())),
        ));
        let sym: libloading::Symbol<'static, DigestFn> = lib
            .get(b"md5_digest\0")
            .unwrap_or_else(|e| panic!("dlsym md5_digest in {} failed: {e}", path.display()));
        *sym
    }
}

/// Both implementations, obtained purely through `dlopen`/`dlsym`.
pub struct Pair {
    pub c: DigestFn,
    pub rs: DigestFn,
}

pub fn pair() -> Pair {
    Pair {
        c: load(&c_so_path()),
        rs: load(&rust_so_path()),
    }
}

/// Deterministic xorshift64* PRNG so every run is reproducible.
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

    pub fn new() -> Self {
        Rng(Self::SEED)
    }

    pub fn with_seed(seed: u64) -> Self {
        Rng(if seed == 0 { Self::SEED } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    pub fn next_md5(&mut self) -> Md5 {
        Md5 {
            a: self.next_u32(),
            b: self.next_u32(),
            c: self.next_u32(),
            d: self.next_u32(),
        }
    }

    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }

    /// Occasionally return an "interesting" value instead of a uniform one.
    pub fn next_u32_biased(&mut self) -> u32 {
        const SPECIAL: [u32; 12] = [
            0x0000_0000,
            0xFFFF_FFFF,
            0x0000_0001,
            0xFFFF_FFFE,
            0x7FFF_FFFF,
            0x8000_0000,
            0x0000_00FF,
            0x0000_FF00,
            0x00FF_0000,
            0xFF00_0000,
            0x0000_FFFF,
            0xFFFF_0000,
        ];
        let r = self.next_u64();
        if r % 4 == 0 {
            SPECIAL[(r >> 8) as usize % SPECIAL.len()]
        } else {
            self.next_u32()
        }
    }

    pub fn next_md5_biased(&mut self) -> Md5 {
        Md5 {
            a: self.next_u32_biased(),
            b: self.next_u32_biased(),
            c: self.next_u32_biased(),
            d: self.next_u32_biased(),
        }
    }
}

/// 128-byte arena with 16-byte alignment, so tests can place `m` and `out` at
/// arbitrary sub-alignments and at arbitrary overlaps.
#[repr(C, align(16))]
pub struct Arena(pub [u8; 128]);

impl Arena {
    pub fn new(fill: u8) -> Self {
        Arena([fill; 128])
    }
    pub fn ptr(&mut self, off: usize) -> *mut u8 {
        assert!(off < 128);
        unsafe { self.0.as_mut_ptr().add(off) }
    }
}

/// Render a byte slice as a stable hex string for assertion messages.
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
