//! Shared plumbing for the differential test-suite.
//!
//! Both implementations are always reached through `dlopen`/`dlsym`
//! (`libloading`), never by calling Rust functions directly, so the
//! `#[unsafe(no_mangle)] extern "C"` export wrappers are part of what is being
//! tested.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::Library;

pub const SEED: u64 = 0x5EED_1234;

// ---------------------------------------------------------------------------
// Library discovery / loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` -- derived from the running test executable
/// (`target/<profile>/deps/<test>-<hash>`).
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

/// The C library built from `c_src` by CMake.
pub fn c_lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let path = manifest_dir().join("c_src/build/libtranslated_rust.so");
        assert!(
            path.exists(),
            "C shared library missing at {}\n\
             build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            path.display()
        );
        unsafe { Library::new(&path) }.expect("dlopen C library")
    })
}

/// The Rust cdylib (`crate-type = ["cdylib"]`).
pub fn rust_lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let path = target_profile_dir().join("libmemchra2_lib.so");
        assert!(
            path.exists(),
            "Rust shared library missing at {} -- run `cargo build` first",
            path.display()
        );
        unsafe { Library::new(&path) }.expect("dlopen Rust library")
    })
}

/// The C shim (`tests/cshim/shim.c`, which `#include`s `c_src/src/lib.c`) that
/// exposes the `static` helpers under `itest_*` names.  Compiled on demand with
/// the same (default, unoptimised) flags CMake uses for the main library.
pub fn c_shim() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let src = manifest_dir().join("tests/cshim/shim.c");
        let dir = target_profile_dir().join("cshim");
        std::fs::create_dir_all(&dir).expect("mkdir cshim");

        // `CSHIM_CFLAGS=-O2 cargo test ...` re-runs the whole differential suite
        // against an optimised C build (informational: the reference build is
        // CMake's default, unoptimised one).
        let extra: Vec<String> = std::env::var("CSHIM_CFLAGS")
            .unwrap_or_default()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let tag: String = extra
            .join("_")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let out = if tag.is_empty() {
            dir.join("libcshim.so")
        } else {
            dir.join(format!("libcshim_{tag}.so"))
        };

        if needs_rebuild(&out, &src) {
            // Compile to a unique name, then rename: concurrently running test
            // binaries can then never observe a half-written library.
            let tmp = dir.join(format!("libcshim.{}.so", std::process::id()));
            let status = Command::new("cc")
                .args(["-shared", "-fPIC"])
                .args(&extra)
                .arg("-o")
                .arg(&tmp)
                .arg(&src)
                .status()
                .expect("failed to spawn cc -- is a C compiler installed?");
            assert!(status.success(), "compiling {} failed", src.display());
            std::fs::rename(&tmp, &out).expect("rename cshim");
        }

        unsafe { Library::new(&out) }.expect("dlopen C shim")
    })
}

fn needs_rebuild(out: &Path, src: &Path) -> bool {
    let (o, s) = match (out.metadata(), src.metadata()) {
        (Ok(o), Ok(s)) => (o, s),
        _ => return true,
    };
    match (o.modified(), s.modified()) {
        (Ok(o), Ok(s)) => o < s,
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// Symbol helpers
// ---------------------------------------------------------------------------

/// Looks up `name` in `lib` and hands back a plain function pointer.
///
/// # Safety
/// `F` must be the exact `extern "C" fn` type of the symbol.
pub unsafe fn sym<F: Copy>(lib: &'static Library, name: &str) -> F {
    let mut owned = Vec::with_capacity(name.len() + 1);
    owned.extend_from_slice(name.as_bytes());
    owned.push(0);
    let s: libloading::Symbol<F> = lib
        .get(&owned)
        .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"));
    *s
}

/// A pair of function pointers -- `(c, rust)` -- for the same symbol name.
pub unsafe fn pair<F: Copy>(lib_c: &'static Library, name: &str) -> (F, F) {
    (sym::<F>(lib_c, name), sym::<F>(rust_lib(), name))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seed, reproducible
// ---------------------------------------------------------------------------

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
        x.wrapping_mul(0x2545F491_4F6CDD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 40) as u8
    }

    /// Uniform-ish value in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    /// `i32` biased towards "interesting" values (boundaries, small magnitudes,
    /// float-relevant bit patterns) but still covering the whole range.
    pub fn next_i32_interesting(&mut self) -> i32 {
        const SPECIAL: [i32; 20] = [
            0,
            1,
            -1,
            2,
            -2,
            9,
            10,
            -10,
            99,
            127,
            128,
            255,
            256,
            -255,
            -256,
            i32::MAX,
            i32::MIN,
            0x3F80_0000,          // 1.0f
            0x447A_0000,          // 1000.0f
            0x7F80_0000u32 as i32, // +inf
        ];
        match self.below(4) {
            0 => SPECIAL[self.below(SPECIAL.len())],
            1 => (self.next_u32() % 2000) as i32 - 1000,
            2 => {
                // bit patterns that land in the interesting float window
                let lo = 0x3F80_0000u32;
                let hi = 0x447A_0000u32;
                (lo + self.next_u32() % (hi - lo + 4096)) as i32
            }
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

/// Builds a NUL-terminated `Vec<c_char>` from bytes (the bytes must not contain
/// a NUL unless that is intended).
pub fn cstring(bytes: &[u8]) -> Vec<std::ffi::c_char> {
    let mut v: Vec<std::ffi::c_char> = bytes.iter().map(|&b| b as std::ffi::c_char).collect();
    v.push(0);
    v
}
