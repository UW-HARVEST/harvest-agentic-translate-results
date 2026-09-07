#![allow(dead_code)] // each test binary uses a different subset of the harness

//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes `hdr_bitrate` from each. The Rust implementation is
//! NEVER called directly — always through its `#[no_mangle]` export, exactly as
//! an external C consumer would.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type HdrBitrateFn = unsafe extern "C" fn(*const u8) -> std::ffi::c_uint;

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c: HdrBitrateFn,
    pub rust: HdrBitrateFn,
}

/// Crate root (`translation/`).
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn work_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

/// Locate the C shared object built from `c_src/`.
///
/// The CMake project name is derived from the *parent directory name*, so the
/// file name is not fixed; scan the build dir for any `lib*.so`.
fn find_c_so() -> PathBuf {
    let build = work_root().join("c_src/build");
    assert!(
        build.is_dir(),
        "C build dir {} missing — build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("read c_src/build")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("lib") && name.ends_with(".so") && p.is_file()
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// Locate the Rust `cdylib`. Tests can be run from `target/debug` or
/// `target/release`; prefer whichever matches the current test profile, then
/// fall back to the other.
fn find_rust_so() -> PathBuf {
    let target = crate_root().join("target");
    // `current_exe` lives in e.g. target/debug/deps/<test>-<hash>
    let profile_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().and_then(Path::parent).map(Path::to_path_buf));

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(d) = profile_dir {
        candidates.push(d.join("libhdr_bitrate_lib.so"));
    }
    candidates.push(target.join("release/libhdr_bitrate_lib.so"));
    candidates.push(target.join("debug/libhdr_bitrate_lib.so"));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found; looked at {:?}. Build it with `cargo build` / `cargo build --release`.",
        candidates
    );
}

fn load() -> Libs {
    let c_path = find_c_so();
    let rust_path = find_rust_so();

    unsafe {
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust_lib = Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

        let c_sym: Symbol<HdrBitrateFn> = c_lib
            .get(b"hdr_bitrate\0")
            .expect("C .so does not export hdr_bitrate");
        let rust_sym: Symbol<HdrBitrateFn> = rust_lib
            .get(b"hdr_bitrate\0")
            .expect("Rust .so does not export hdr_bitrate");

        let c = *c_sym;
        let rust = *rust_sym;
        Libs { _c: c_lib, _rust: rust_lib, c, rust }
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(load)
}

// SAFETY: both handles stay alive for the process lifetime and the functions are
// pure reads of read-only data.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

/// Call both implementations on the same 3+ byte buffer and assert byte-identical
/// results.
#[track_caller]
pub fn assert_same(buf: &[u8], ctx: &str) -> u32 {
    assert!(buf.len() >= 3, "buffer must hold h[0..2]");
    let l = libs();
    let (c, r) = unsafe { ((l.c)(buf.as_ptr()), (l.rust)(buf.as_ptr())) };
    assert_eq!(
        c, r,
        "divergence for {ctx}: h = {buf:02x?} -> C = {c}, Rust = {r}"
    );
    c
}

/// Call both implementations on a raw pointer (used for guard-page / alignment
/// shapes where no slice can be formed).
#[track_caller]
pub fn assert_same_ptr(p: *const u8, ctx: &str) -> u32 {
    let l = libs();
    let (c, r) = unsafe { ((l.c)(p), (l.rust)(p)) };
    assert_eq!(c, r, "divergence for {ctx}: C = {c}, Rust = {r}");
    c
}

/// Deterministic xorshift64* PRNG so every property-style row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        (self.next_u64() >> 1) % n
    }
}

/// Build an `h[1]` byte with the given plane bit and raw 2-bit layer field,
/// filling every ignored bit (bit 0, bits 4..7) from `rng`.
pub fn make_h1(plane: u8, raw_layer: u8, rng: &mut Rng) -> u8 {
    let noise = rng.next_u8();
    let mut b = 0u8;
    b |= (plane & 1) << 3;
    b |= (raw_layer & 3) << 1;
    b |= noise & 0x01; // ignored bit 0
    b |= noise & 0xF0; // ignored bits 4..7
    b
}

/// Build an `h[2]` byte with the given bitrate nibble and random ignored low
/// nibble.
pub fn make_h2(nibble: u8, rng: &mut Rng) -> u8 {
    ((nibble & 0x0F) << 4) | (rng.next_u8() & 0x0F)
}

/// All eight `(plane, raw_layer)` row selectors.
pub const ROWS: [(u8, u8); 8] = [
    (0, 0),
    (0, 1),
    (0, 2),
    (0, 3),
    (1, 0),
    (1, 1),
    (1, 2),
    (1, 3),
];
