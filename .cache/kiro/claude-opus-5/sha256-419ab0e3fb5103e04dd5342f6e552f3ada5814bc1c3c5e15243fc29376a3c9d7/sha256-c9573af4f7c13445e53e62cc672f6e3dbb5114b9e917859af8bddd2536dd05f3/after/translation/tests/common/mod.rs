//! Shared differential-test harness.
//!
//! Both libraries are loaded as shared objects with `libloading` and invoked
//! only through their exported `md5_digest` symbol. The Rust implementation is
//! NEVER called directly as a Rust function — every call goes through the
//! `cdylib`'s `#[no_mangle]` export, exactly as an external C caller would, so
//! the export wrapper itself is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Mirror of `struct tflac_md5` from `c_src/include/lib.h`.
///
/// Declared `#[repr(C)]` here so the test harness itself agrees with the C ABI;
/// `layout_matches_c_abi` asserts size/align/offsets independently.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Md5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

impl Md5 {
    pub fn new(a: u32, b: u32, c: u32, d: u32) -> Self {
        Md5 { a, b, c, d }
    }

    /// Build from the raw 16-byte memory image, i.e. what a C caller that
    /// `memcpy`s into the struct would produce (little-endian host).
    pub fn from_image(img: &[u8; 16]) -> Self {
        Md5 {
            a: u32::from_le_bytes(img[0..4].try_into().unwrap()),
            b: u32::from_le_bytes(img[4..8].try_into().unwrap()),
            c: u32::from_le_bytes(img[8..12].try_into().unwrap()),
            d: u32::from_le_bytes(img[12..16].try_into().unwrap()),
        }
    }
}

/// `void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);`
pub type DigestFn = unsafe extern "C" fn(*const Md5, *mut u8);

/// Which of the two shared objects to talk to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

/// Locate the C shared object produced by the CMake build.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name().map(|n| n.to_string_lossy().starts_with("lib")).unwrap_or(false)
        })
        .collect();
    found.sort();
    assert!(
        !found.is_empty(),
        "no lib*.so found in {} — build the C library first",
        build_dir.display()
    );
    found.remove(0)
}

/// Locate the Rust `cdylib`. Prefers the release artifact (the deliverable,
/// built with `panic = "abort"`), falls back to debug.
///
/// Also refuses to run against a STALE artifact: `cargo test` does not rebuild
/// the `cdylib`, so without this check a passing run could be verifying an old
/// `.so` rather than the current `src/lib.rs`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libmd5_digest_lib.so");
        if p.exists() {
            assert_fresh(&p, &root.join("src/lib.rs"));
            return p;
        }
    }
    panic!("libmd5_digest_lib.so not found; run `cargo build --release` first");
}

fn assert_fresh(artifact: &PathBuf, source: &PathBuf) {
    let mtime = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    if let (Some(a), Some(s)) = (mtime(artifact), mtime(source)) {
        assert!(
            a >= s,
            "STALE ARTIFACT: {} is older than {}. `cargo test` does not rebuild the cdylib — \
             run `cargo build --release` first, otherwise the tests verify an outdated .so.",
            artifact.display(),
            source.display()
        );
    }
}

fn load(path: &PathBuf) -> DigestFn {
    // The Library is intentionally leaked so the resolved function pointer is
    // valid for the whole process lifetime.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
    }));
    let sym: Symbol<DigestFn> = unsafe {
        lib.get(b"md5_digest\0")
            .unwrap_or_else(|e| panic!("md5_digest not exported by {}: {e}", path.display()))
    };
    *sym
}

/// The `md5_digest` symbol from the requested shared object.
pub fn digest(which: Impl) -> DigestFn {
    static C_FN: OnceLock<DigestFn> = OnceLock::new();
    static RUST_FN: OnceLock<DigestFn> = OnceLock::new();
    match which {
        Impl::C => *C_FN.get_or_init(|| load(&c_so_path())),
        Impl::Rust => *RUST_FN.get_or_init(|| load(&rust_so_path())),
    }
}

/// Call one implementation into a freshly poisoned 16-byte buffer.
pub fn call(which: Impl, m: &Md5) -> [u8; 16] {
    call_poisoned(which, m, 0xAA)
}

/// Call one implementation into a buffer pre-filled with `poison`, so any byte
/// the implementation fails to write is visible.
pub fn call_poisoned(which: Impl, m: &Md5, poison: u8) -> [u8; 16] {
    let f = digest(which);
    let mut out = [poison; 16];
    unsafe { f(m as *const Md5, out.as_mut_ptr()) };
    out
}

/// Run both implementations on the same input and return (C, Rust).
pub fn both(m: &Md5) -> ([u8; 16], [u8; 16]) {
    (call(Impl::C, m), call(Impl::Rust, m))
}

/// Differential assertion for one input: C and Rust must agree byte-for-byte.
#[track_caller]
pub fn assert_same(row: &str, m: &Md5) {
    let (c, r) = both(m);
    assert_eq!(
        c, r,
        "[{row}] divergence for m = {{a:{:#010x}, b:{:#010x}, c:{:#010x}, d:{:#010x}}}\n  C   : {:02x?}\n  Rust: {:02x?}",
        m.a, m.b, m.c, m.d, c, r
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — property-style inputs, fixed seed.
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
    pub fn next_md5(&mut self) -> Md5 {
        Md5::new(self.next_u32(), self.next_u32(), self.next_u32(), self.next_u32())
    }
    pub fn next_image(&mut self) -> [u8; 16] {
        let mut img = [0u8; 16];
        for chunk in img.chunks_mut(8) {
            chunk.copy_from_slice(&self.next_u64().to_le_bytes()[..chunk.len()]);
        }
        img
    }
    pub fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

/// Word values the C's shifts make interesting, plus signed/unsigned edges.
pub const BOUNDARY_WORDS: [u32; 14] = [
    0x0000_0000,
    0x0000_0001,
    0x0000_007F,
    0x0000_0080,
    0x0000_00FF,
    0x0000_0100,
    0x0000_7FFF,
    0x0000_8000,
    0x0000_FFFF,
    0x0001_0000,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
];

/// Number of randomized inputs used per property-style row.
pub const ITERS: usize = 4096;
