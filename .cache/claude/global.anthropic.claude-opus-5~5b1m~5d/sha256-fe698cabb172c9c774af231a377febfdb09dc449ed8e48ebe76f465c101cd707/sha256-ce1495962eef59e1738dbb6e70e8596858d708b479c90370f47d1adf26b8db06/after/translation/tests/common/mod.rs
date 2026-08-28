//! Shared differential-test harness.
//!
//! Both the C library and the Rust library are loaded **as shared objects via
//! `libloading`** and every call goes through `dlsym`. The Rust crate is never
//! called directly, so the `#[unsafe(no_mangle)] extern "C"` export wrapper is
//! itself under test, exactly as an external C consumer would exercise it.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// The C ABI of the one exported function: `uint32_t rev16(uint32_t a)`.
pub type Rev16Fn = unsafe extern "C" fn(u32) -> u32;

/// The same symbol viewed with 64-bit-wide argument/return registers, used only
/// by the ABI-width test (row E10). Per the x86-64 SysV ABI the upper half of
/// both the argument and the return register is unspecified for a 32-bit type,
/// so callers of this alias must mask before comparing.
pub type Rev16FnWide = unsafe extern "C" fn(u64) -> u64;

/// Workspace root: the directory that contains both `c_src/` and `translation/`.
pub fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .to_path_buf()
    })
}

/// Directory the CMake build writes into.
fn c_build_dir() -> PathBuf {
    repo_root().join("c_src").join("build")
}

/// Locate the C shared object.
///
/// `c_src/CMakeLists.txt` derives the project name (and therefore the library
/// file name) from the *name of the parent directory* of `c_src`, which is not
/// fixed. The `.so` is therefore discovered by globbing rather than hardcoded.
fn find_c_so() -> PathBuf {
    let dir = c_build_dir();
    if let Some(found) = scan_for_so(&dir) {
        return found;
    }

    // Not built yet - build it the documented way, then look again.
    build_c_library();
    scan_for_so(&dir).unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

fn scan_for_so(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("lib") && n.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

fn build_c_library() {
    let c_src = repo_root().join("c_src");
    let build = c_build_dir();
    let _ = std::fs::create_dir_all(&build);

    let configure = Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output();
    if let Ok(out) = configure {
        if !out.status.success() {
            eprintln!(
                "cmake configure in {} failed:\n{}",
                c_src.display(),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    let compile = Command::new("cmake")
        .current_dir(&build)
        .arg("--build")
        .arg(".")
        .output();
    if let Ok(out) = compile {
        if !out.status.success() {
            eprintln!(
                "cmake --build failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
}

/// Locate the Rust `cdylib`. Always the release artifact, so the object under
/// test is the one an external consumer would link against.
fn find_rust_so() -> PathBuf {
    let path = repo_root()
        .join("translation")
        .join("target")
        .join("release")
        .join("librev16_lib.so");
    if path.is_file() {
        return path;
    }

    // Build it if the caller forgot. `cargo test` has already released the
    // build lock by the time test binaries run, so this cannot self-deadlock.
    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(repo_root().join("translation"))
        .args(["build", "--release"])
        .output();
    if let Ok(out) = out {
        if !out.status.success() {
            eprintln!(
                "cargo build --release failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    assert!(
        path.is_file(),
        "Rust shared library missing at {}. Build it with: \
         cd translation && cargo build --release",
        path.display()
    );
    path
}

pub fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(find_c_so)
}

pub fn rust_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(find_rust_so)
}

fn load(path: &Path) -> &'static Library {
    // SAFETY: loading a shared object runs its initialisers; both libraries are
    // plain leaf libraries we just built ourselves.
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    // Leaked so symbols can be handed out with a 'static lifetime; the process
    // is a short-lived test binary.
    Box::leak(Box::new(lib))
}

pub fn c_lib() -> &'static Library {
    static L: OnceLock<&'static Library> = OnceLock::new();
    *L.get_or_init(|| load(c_so_path()))
}

pub fn rust_lib() -> &'static Library {
    static L: OnceLock<&'static Library> = OnceLock::new();
    *L.get_or_init(|| load(rust_so_path()))
}

fn sym<T: Copy + 'static>(lib: &'static Library, name: &[u8]) -> T {
    // SAFETY: the signature is taken from `c_src/include/lib.h`.
    let s: Symbol<'static, T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol {} not found: {e}",
            String::from_utf8_lossy(name).trim_end_matches('\0')
        )
    });
    *s
}

/// `rev16` from the C shared object.
pub fn c_rev16() -> Rev16Fn {
    static F: OnceLock<Rev16Fn> = OnceLock::new();
    *F.get_or_init(|| sym(c_lib(), b"rev16\0"))
}

/// `rev16` from the Rust shared object (through its `no_mangle` export).
pub fn rust_rev16() -> Rev16Fn {
    static F: OnceLock<Rev16Fn> = OnceLock::new();
    *F.get_or_init(|| sym(rust_lib(), b"rev16\0"))
}

pub fn c_rev16_wide() -> Rev16FnWide {
    static F: OnceLock<Rev16FnWide> = OnceLock::new();
    *F.get_or_init(|| sym(c_lib(), b"rev16\0"))
}

pub fn rust_rev16_wide() -> Rev16FnWide {
    static F: OnceLock<Rev16FnWide> = OnceLock::new();
    *F.get_or_init(|| sym(rust_lib(), b"rev16\0"))
}

/// Call both `.so`s with `arg` and assert byte-identical results.
///
/// Returns the (agreed) value so callers can chain it.
#[track_caller]
pub fn assert_same(row: &str, arg: u32) -> u32 {
    let c = unsafe { c_rev16()(arg) };
    let r = unsafe { rust_rev16()(arg) };
    assert_eq!(
        c, r,
        "[{row}] divergence for rev16(0x{arg:08X}): C returned 0x{c:08X} \
         (0b{c:032b}) but Rust returned 0x{r:08X} (0b{r:032b})"
    );
    c
}

/// Sweep a whole row of arguments through both libraries.
#[track_caller]
pub fn assert_same_all<I: IntoIterator<Item = u32>>(row: &str, args: I) -> usize {
    let mut n = 0usize;
    for a in args {
        assert_same(row, a);
        n += 1;
    }
    assert!(n > 0, "[{row}] generated no inputs - the row tested nothing");
    n
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64). Fixed seeds keep every row reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
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

    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }

    /// A 32-bit value with roughly `weight` bits set, for sparse/dense shapes.
    pub fn with_weight(&mut self, weight: u32) -> u32 {
        let mut v = 0u32;
        let mut set = 0;
        while set < weight {
            let bit = 1u32 << self.below(32);
            if v & bit == 0 {
                v |= bit;
                set += 1;
            }
        }
        v
    }
}

/// Reference 16-bit bit reversal, used only to *construct* interesting inputs
/// (e.g. palindromes). It is never used as the oracle - the C library is.
pub fn reverse16(x: u16) -> u16 {
    let mut out = 0u16;
    for i in 0..16 {
        if x & (1 << i) != 0 {
            out |= 1 << (15 - i);
        }
    }
    out
}
