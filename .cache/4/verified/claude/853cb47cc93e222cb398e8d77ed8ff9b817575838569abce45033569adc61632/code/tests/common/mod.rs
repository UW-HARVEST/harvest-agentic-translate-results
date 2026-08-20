//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called through their exported `encode_quant` symbol. The Rust side is never
//! called directly as a Rust function -- this way the `#[no_mangle]`
//! `extern "C"` wrapper and the C ABI are part of what is under test.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// The one and only exported entry point.
pub type EncodeQuantFn =
    unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

pub struct Libs {
    pub c: EncodeQuantFn,
    pub rust: EncodeQuantFn,
    // Keep the handles alive for the whole process lifetime.
    _c_lib: Library,
    _rust_lib: Library,
}

// The extracted function pointers are plain `fn` pointers, and `libloading::Library`
// is itself `Send + Sync`.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Candidate directories that may hold the freshly built Rust cdylib.
fn rust_so_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let name = "libencode_quant_lib.so";

    if let Ok(explicit) = std::env::var("RUST_SO_PATH") {
        out.push(PathBuf::from(explicit));
    }

    // .../target/<profile>/deps/<test-binary>  ->  .../target/<profile>/
    if let Ok(exe) = std::env::current_exe() {
        let mut dir: Option<&Path> = exe.parent();
        for _ in 0..3 {
            if let Some(d) = dir {
                out.push(d.join(name));
                dir = d.parent();
            }
        }
    }

    let md = manifest_dir();
    out.push(md.join("target/debug").join(name));
    out.push(md.join("target/release").join(name));
    out
}

fn c_so_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(explicit) = std::env::var("C_SO_PATH") {
        out.push(PathBuf::from(explicit));
    }
    let md = manifest_dir();
    out.push(md.join("c_src/build/libtranslated_rust.so"));
    out.push(md.join("c_src/build/libc_src.so"));
    // Fall back to whatever single .so the cmake build produced.
    if let Ok(entries) = std::fs::read_dir(md.join("c_src/build")) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                out.push(p);
            }
        }
    }
    out
}

fn pick(candidates: Vec<PathBuf>, what: &str) -> PathBuf {
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not locate the {what} shared object; tried:\n{}",
        candidates
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn newest_mtime(dir: &Path, exts: &[&str]) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().and_then(|s| s.to_str()) != Some("build") {
                    stack.push(p);
                }
                continue;
            }
            let is_src = p
                .extension()
                .and_then(|s| s.to_str())
                .map(|x| exts.contains(&x))
                .unwrap_or(false);
            if !is_src {
                continue;
            }
            if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                if newest.map(|n| m > n).unwrap_or(true) {
                    newest = Some(m);
                }
            }
        }
    }
    newest
}

/// `cargo test` does **not** rebuild a `cdylib`-only lib target, so a `.so` left
/// over from an earlier `cargo build` would silently be tested instead of the
/// current sources. Refuse to run in that case rather than reporting a false
/// pass.
fn assert_fresh(so: &Path, src_dir: &Path, exts: &[&str], what: &str, rebuild_hint: &str) {
    let Ok(so_m) = std::fs::metadata(so).and_then(|m| m.modified()) else {
        return;
    };
    if let Some(src_m) = newest_mtime(src_dir, exts) {
        assert!(
            so_m >= src_m,
            "STALE {what} SHARED OBJECT\n  {}\nis older than the newest source in\n  {}\n\
             Rebuild first:  {rebuild_hint}\n\
             (a `cdylib`-only lib target is NOT rebuilt by `cargo test`)",
            so.display(),
            src_dir.display()
        );
    }
}

fn load() -> Libs {
    let c_path = pick(c_so_candidates(), "C");
    let rust_path = pick(rust_so_candidates(), "Rust");

    let md = manifest_dir();
    assert_fresh(
        &rust_path,
        &md.join("src"),
        &["rs"],
        "RUST",
        "cargo build --no-default-features",
    );
    assert_fresh(
        &c_path,
        &md.join("c_src"),
        &["c", "h"],
        "C",
        "cmake --build c_src/build",
    );

    unsafe {
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let rust_lib = Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));

        let c_sym: Symbol<EncodeQuantFn> = c_lib
            .get(b"encode_quant\0")
            .expect("C .so does not export `encode_quant`");
        let rust_sym: Symbol<EncodeQuantFn> = rust_lib
            .get(b"encode_quant\0")
            .expect("Rust .so does not export `encode_quant`");

        let c = *c_sym;
        let rust = *rust_sym;

        Libs {
            c,
            rust,
            _c_lib: c_lib,
            _rust_lib: rust_lib,
        }
    }
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(load)
}

/// One differential call. Returns the shared result on agreement, panics with the
/// full input vector on divergence.
#[track_caller]
pub fn diff(uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32, lsbit: i32) -> i32 {
    let l = libs();
    let c = unsafe { (l.c)(uni, step, pred, tgt, tgt2, lsbit) };
    let r = unsafe { (l.rust)(uni, step, pred, tgt, tgt2, lsbit) };
    assert_eq!(
        c, r,
        "DIVERGENCE encode_quant(uni={uni}, step={step}, pred={pred}, \
         tgt={tgt}, tgt2={tgt2}, lsbit={lsbit}): C={c} Rust={r}"
    );
    c
}

/// Deterministic SplitMix64 -- fixed seed, reproducible across runs and hosts.
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

    /// Uniform over the entire `i32` range (all bit patterns).
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `lo..=hi` (inclusive), computed in `i64` to avoid overflow.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn pick<T: Copy>(&mut self, pool: &[T]) -> T {
        pool[(self.next_u64() % pool.len() as u64) as usize]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// The four distinct `lsbit` modes the C branches on (axis A of `CONFIGS.md`).
pub const LSBIT_MODES: [i32; 4] = [
    0,  // A0    : no fixup
    4,  // A4    : clear-then-re-OR from bits 1&2
    1,  // AODD  : force bit 0 set
    2,  // AEVEN : force bit 0 clear
];

/// Interesting corner values for the free-form integer arguments.
pub const CORNERS: [i32; 27] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    7,
    -7,
    8,
    -8,
    15,
    -15,
    16,
    31,
    32,
    -32,
    33,
    255,
    -255,
    256,
    0x0FFF_FFFF,
    0x1000_0000,
    -0x1000_0000,
    i32::MAX - 1,
    i32::MAX,
    i32::MIN,
];
