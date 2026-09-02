//! Shared differential-test harness.
//!
//! Both libraries are loaded as shared objects through `libloading` and called
//! only through their exported C symbols — the Rust crate is never linked
//! directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers are under test
//! exactly as an external consumer would exercise them.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

/// The 28-byte `struct tflac`, handled as raw bytes so that padding bytes
/// 21..=23 participate in the comparison. Neither implementation should ever
/// write them.
#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tflac(pub [u8; 28]);

pub const OFF_BLOCKSIZE: usize = 0;
pub const OFF_SAMPLERATE: usize = 4;
pub const OFF_CHANNELS: usize = 8;
pub const OFF_BITDEPTH: usize = 12;
pub const OFF_CHANNEL_MODE: usize = 16;
pub const OFF_MAX_RICE: usize = 17;
pub const OFF_MIN_PO: usize = 18;
pub const OFF_MAX_PO: usize = 19;
pub const OFF_PARTITION_ORDER: usize = 20;
pub const OFF_CUR_BLOCKSIZE: usize = 24;

impl Tflac {
    /// Start from a recognisable poison pattern so that any field (or padding
    /// byte) a callee forgets to write is still compared.
    pub fn poisoned() -> Self {
        Tflac([0xA5; 28])
    }

    pub fn u32_at(&self, off: usize) -> u32 {
        u32::from_ne_bytes(self.0[off..off + 4].try_into().unwrap())
    }
    pub fn set_u32(&mut self, off: usize, v: u32) -> &mut Self {
        self.0[off..off + 4].copy_from_slice(&v.to_ne_bytes());
        self
    }
    pub fn u8_at(&self, off: usize) -> u8 {
        self.0[off]
    }
    pub fn set_u8(&mut self, off: usize, v: u8) -> &mut Self {
        self.0[off] = v;
        self
    }

    pub fn blocksize(&self) -> u32 {
        self.u32_at(OFF_BLOCKSIZE)
    }
    pub fn cur_blocksize(&self) -> u32 {
        self.u32_at(OFF_CUR_BLOCKSIZE)
    }
    pub fn channel_mode(&self) -> u8 {
        self.0[OFF_CHANNEL_MODE]
    }
    pub fn max_rice_value(&self) -> u8 {
        self.0[OFF_MAX_RICE]
    }
    pub fn partition_order(&self) -> u8 {
        self.0[OFF_PARTITION_ORDER]
    }

    /// Convenience builder for a struct that passes every validation check.
    pub fn valid() -> Self {
        let mut t = Tflac::poisoned();
        t.set_u32(OFF_BLOCKSIZE, 4096)
            .set_u32(OFF_SAMPLERATE, 44100)
            .set_u32(OFF_CHANNELS, 2)
            .set_u32(OFF_BITDEPTH, 16)
            .set_u8(OFF_CHANNEL_MODE, 0)
            .set_u8(OFF_MAX_RICE, 0)
            .set_u8(OFF_MIN_PO, 0)
            .set_u8(OFF_MAX_PO, 0);
        t
    }

    pub fn describe(&self) -> String {
        format!(
            "blocksize={} samplerate={} channels={} bitdepth={} channel_mode={} \
             max_rice={} min_po={} max_po={} partition_order={} cur_blocksize={} pad={:02X?}",
            self.u32_at(OFF_BLOCKSIZE),
            self.u32_at(OFF_SAMPLERATE),
            self.u32_at(OFF_CHANNELS),
            self.u32_at(OFF_BITDEPTH),
            self.0[OFF_CHANNEL_MODE],
            self.0[OFF_MAX_RICE],
            self.0[OFF_MIN_PO],
            self.0[OFF_MAX_PO],
            self.0[OFF_PARTITION_ORDER],
            self.u32_at(OFF_CUR_BLOCKSIZE),
            &self.0[21..24],
        )
    }
}

impl std::fmt::Debug for Tflac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {} | bytes={:02X?} }}", self.describe(), self.0)
    }
}

type FnValidate = unsafe extern "C" fn(*mut Tflac) -> std::ffi::c_int;
type FnSizeMemory = unsafe extern "C" fn(u32) -> u32;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    validate: FnValidate,
    size_memory: FnSizeMemory,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
            let validate: Symbol<FnValidate> = lib
                .get(b"flac_validate\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol flac_validate: {e}"));
            let size_memory: Symbol<FnSizeMemory> = lib
                .get(b"tflac_size_memory\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol tflac_size_memory: {e}"));
            let validate = *validate;
            let size_memory = *size_memory;
            Impl { name, path, _lib: lib, validate, size_memory }
        }
    }

    pub fn flac_validate(&self, t: &mut Tflac) -> i32 {
        unsafe { (self.validate)(t as *mut Tflac) as i32 }
    }

    /// Raw-pointer entry point, so the null-pointer probe can pass an actual
    /// null across the FFI boundary without first forming an invalid `&mut`.
    pub unsafe fn flac_validate_raw(&self, t: *mut Tflac) -> i32 {
        unsafe { (self.validate)(t) as i32 }
    }

    pub fn tflac_size_memory(&self, blocksize: u32) -> u32 {
        unsafe { (self.size_memory)(blocksize) }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {} (build the C lib first): {e}", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let target = workspace_root().join("translation/target");
    // Prefer the debug build (overflow checks + unwinding) when present, since
    // it is the stricter of the two; fall back to release.
    for profile in ["debug", "release"] {
        let p = target.join(profile).join("libflac_validate_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libflac_validate_lib.so not found under {} — run `cargo build` / `cargo build --release`",
        target.display()
    );
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let p = Pair {
            c: Impl::load("C", find_c_so()),
            rs: Impl::load("Rust", find_rust_so()),
        };
        eprintln!("[harness]    C .so = {}", p.c.path.display());
        eprintln!("[harness] Rust .so = {}", p.rs.path.display());
        p
    })
}

/// Run `flac_validate` on identical copies of `input` in both libraries and
/// assert the return value AND all 28 resulting struct bytes agree.
#[track_caller]
pub fn diff_validate(row: &str, input: &Tflac) -> (i32, Tflac) {
    let p = pair();
    let mut tc = *input;
    let mut tr = *input;
    let rc = p.c.flac_validate(&mut tc);
    let rr = p.rs.flac_validate(&mut tr);
    assert_eq!(
        rc, rr,
        "[{row}] return value diverged for input {}\n  C returned {rc}, Rust returned {rr}",
        input.describe()
    );
    assert_eq!(
        tc.0, tr.0,
        "[{row}] struct state diverged for input {}\n  C   -> {}\n  Rust-> {}",
        input.describe(),
        tc.describe(),
        tr.describe()
    );
    (rc, tc)
}

/// Run `tflac_size_memory` in both libraries and assert equality.
#[track_caller]
pub fn diff_size_memory(row: &str, blocksize: u32) -> u32 {
    let p = pair();
    let c = p.c.tflac_size_memory(blocksize);
    let r = p.rs.tflac_size_memory(blocksize);
    assert_eq!(
        c, r,
        "[{row}] tflac_size_memory({blocksize} / {blocksize:#010X}) diverged: C={c:#010X} Rust={r:#010X}"
    );
    c
}

/// Deterministic xorshift64* PRNG — fixed seed per row for reproducibility,
/// no external dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `lo..=hi`.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        self.range_u32(lo as u32, hi as u32) as u8
    }
    /// A struct with every axis randomized inside its *valid* range.
    pub fn valid_struct(&mut self) -> Tflac {
        let mut t = Tflac::poisoned();
        let max_po = self.range_u8(0, 15);
        t.set_u32(OFF_BLOCKSIZE, self.range_u32(16, 65535))
            .set_u32(OFF_SAMPLERATE, self.range_u32(1, 655350))
            .set_u32(OFF_CHANNELS, self.range_u32(1, 8))
            .set_u32(OFF_BITDEPTH, self.range_u32(1, 32))
            .set_u8(OFF_CHANNEL_MODE, self.range_u8(0, 3))
            .set_u8(OFF_MAX_RICE, self.range_u8(0, 30))
            .set_u8(OFF_MAX_PO, max_po)
            .set_u8(OFF_MIN_PO, self.range_u8(0, max_po));
        t
    }
}

/// Iterations per randomized row.
pub const ITERS: usize = 4000;
