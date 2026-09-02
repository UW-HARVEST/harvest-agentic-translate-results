// Shared differential-test harness.
//
// Every test loads BOTH the C `.so` and the Rust `.so` through `libloading`
// and calls only exported symbols via `dlsym`. Rust functions are never called
// directly, so the `#[no_mangle] extern "C"` wrappers are under test too.
//
// The library keeps hidden, never-reset `static` state (accumulator /
// multiplier / operation_count), so *call-sequence position is part of the
// input*. To give each test private state, `LibPair::fresh` copies both
// shared objects to a unique path before `dlopen`ing them: glibc keys its
// already-loaded table on (st_dev, st_ino), so a distinct copy yields a
// distinct, freshly initialised set of statics.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — no external dev-dependency, fixed seed.
// ---------------------------------------------------------------------------

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// An `i32` biased toward interesting boundary values, but still covering
    /// the full range.
    pub fn interesting_i32(&mut self) -> i32 {
        const BOUNDS: [i32; 24] = [
            0,
            1,
            -1,
            2,
            -2,
            63,
            64,
            65,
            103,
            104,
            105,
            510,
            511,
            512,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
            83,
            0o150,
            0o100,
            0o777,
            0o10,
            256,
        ];
        match self.below(4) {
            0 | 1 => BOUNDS[self.below(BOUNDS.len() as u64) as usize],
            2 => {
                // small magnitude, both signs
                let v = (self.below(2048) as i32) - 1024;
                v
            }
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C shared library produced by `c_src/build`.
pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// The Rust `cdylib`. Prefers the release artifact (that is what an external
/// consumer links against); falls back to debug. `FINDREP_RUST_SO` overrides.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("FINDREP_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "FINDREP_RUST_SO={} is not a file", p.display());
        return p;
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("target"));
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libfindrep_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libfindrep_lib.so not found under {}; run `cargo build --release` first",
        target.display()
    );
}

static COPY_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_copy(src: &Path, tag: &str, which: &str) -> PathBuf {
    let n = COPY_SEQ.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("findrep_diff_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let dst = dir.join(format!("{tag}_{n}_{which}.so"));
    std::fs::copy(src, &dst).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    dst
}

// ---------------------------------------------------------------------------
// The loaded pair.
// ---------------------------------------------------------------------------

pub type FnII = unsafe extern "C" fn(i32, i32) -> i32;
pub type FnI = unsafe extern "C" fn(i32) -> i32;
pub type FnPtrI = unsafe extern "C" fn(*mut c_char, i32);
pub type Fn4 = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

/// The 8 exported symbols of one implementation.
pub struct Api<'a> {
    pub add_to_accumulator: Symbol<'a, FnII>,
    pub multiply_with_multiplier: Symbol<'a, FnII>,
    pub subtract_from_accumulator: Symbol<'a, FnII>,
    pub divide_multiplier: Symbol<'a, FnII>,
    pub process_octal_string: Symbol<'a, FnPtrI>,
    pub find_and_replace_char: Symbol<'a, FnPtrI>,
    pub validate_and_normalize: Symbol<'a, FnI>,
    pub findrep: Symbol<'a, Fn4>,
}

impl<'a> Api<'a> {
    fn bind(lib: &'a Library, who: &str) -> Api<'a> {
        macro_rules! g {
            ($n:literal, $t:ty) => {
                unsafe {
                    lib.get::<$t>(concat!($n, "\0").as_bytes())
                        .unwrap_or_else(|e| panic!("{who}: missing symbol {}: {e}", $n))
                }
            };
        }
        Api {
            add_to_accumulator: g!("add_to_accumulator", FnII),
            multiply_with_multiplier: g!("multiply_with_multiplier", FnII),
            subtract_from_accumulator: g!("subtract_from_accumulator", FnII),
            divide_multiplier: g!("divide_multiplier", FnII),
            process_octal_string: g!("process_octal_string", FnPtrI),
            find_and_replace_char: g!("find_and_replace_char", FnPtrI),
            validate_and_normalize: g!("validate_and_normalize", FnI),
            findrep: g!("findrep", Fn4),
        }
    }
}

/// Owns a private copy of each `.so`, so the hidden statics start fresh.
pub struct LibPair {
    pub c: Library,
    pub r: Library,
    _paths: (PathBuf, PathBuf),
}

impl LibPair {
    /// Load a brand-new, independent instance of each library.
    pub fn fresh(tag: &str) -> LibPair {
        let cp = unique_copy(&c_so_path(), tag, "c");
        let rp = unique_copy(&rust_so_path(), tag, "r");
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("dlopen C {}: {e}", cp.display()));
        let r = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("dlopen Rust {}: {e}", rp.display()));
        LibPair {
            c,
            r,
            _paths: (cp, rp),
        }
    }

    pub fn apis(&self) -> (Api<'_>, Api<'_>) {
        (Api::bind(&self.c, "C"), Api::bind(&self.r, "Rust"))
    }
}

impl Drop for LibPair {
    fn drop(&mut self) {
        // Unlinking a mapped file is safe on Linux; keeps /tmp from filling up
        // when a test allocates hundreds of fresh pairs.
        let _ = std::fs::remove_file(&self._paths.0);
        let _ = std::fs::remove_file(&self._paths.1);
    }
}

// ---------------------------------------------------------------------------
// Buffer helper for the two pointer-taking exports.
// ---------------------------------------------------------------------------

pub const BUF: usize = 256;

/// A 256-byte scratch buffer pre-filled with a recognisable sentinel so that
/// *any* difference in how far the callee writes is visible.
pub fn scratch(fill: u8) -> Vec<c_char> {
    vec![fill as c_char; BUF]
}

pub fn set_cstr(buf: &mut [c_char], bytes: &[u8]) {
    assert!(bytes.len() + 1 <= buf.len());
    for (i, b) in bytes.iter().enumerate() {
        buf[i] = *b as c_char;
    }
    buf[bytes.len()] = 0;
}

pub fn as_u8(buf: &[c_char]) -> Vec<u8> {
    buf.iter().map(|c| *c as u8).collect()
}

pub fn show(buf: &[c_char]) -> String {
    let b = as_u8(buf);
    let end = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    format!(
        "{:?} (+{} trailing)",
        String::from_utf8_lossy(&b[..end]),
        b.len() - end
    )
}
