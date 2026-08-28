//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols — the Rust functions are never
//! called directly, so the `#[no_mangle]` wrappers and the C ABI are part of
//! what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type ValidateFn = unsafe extern "C" fn(*mut u8) -> i32;
pub type SizeMemoryFn = unsafe extern "C" fn(u32) -> u32;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub validate: ValidateFn,
    pub size_memory: SizeMemoryFn,
    _lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        let validate: ValidateFn = unsafe {
            let s: Symbol<ValidateFn> = lib
                .get(b"flac_validate\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol flac_validate: {e}"));
            *s
        };
        let size_memory: SizeMemoryFn = unsafe {
            let s: Symbol<SizeMemoryFn> = lib
                .get(b"tflac_size_memory\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol tflac_size_memory: {e}"));
            *s
        };
        Impl { name, path, validate, size_memory, _lib: lib }
    }
}

pub struct Libs {
    pub c: Impl,
    pub rust: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn newest_so(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .filter(|p| p.is_file())
        .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok())
        .cloned()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    newest_so(&found).unwrap_or_else(|| {
        panic!(
            "no C .so found in {} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    let name = "libflac_validate_lib.so";
    // Deterministic preference: the cdylib built for the SAME profile as the
    // running test binary (target/<profile>/), then any other profile.
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join(name));
            }
            candidates.push(deps.join(name));
        }
    }
    for p in &candidates {
        if p.is_file() {
            return p.clone();
        }
    }
    candidates.push(md.join("target/debug").join(name));
    candidates.push(md.join("target/release").join(name));
    newest_so(&candidates).unwrap_or_else(|| {
        panic!(
            "no Rust cdylib found (looked for {name} in {:?}) — build it with `cargo build`",
            candidates
        )
    })
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c = find_c_so();
        let r = find_rust_so();
        assert!(
            !same_file(&c, &r),
            "C and Rust .so resolved to the same file: {}",
            c.display()
        );
        Libs { c: Impl::load("C", c), rust: Impl::load("Rust", r) }
    })
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// struct tflac — kept as a raw 28-byte, 4-byte-aligned image so that padding
// bytes 21..=23 are observable too.
// ---------------------------------------------------------------------------

pub const TFLAC_SIZE: usize = 28;

#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Raw(pub [u8; TFLAC_SIZE]);

impl std::fmt::Debug for Raw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let f2 = Fields::from_raw(*self);
        write!(f, "{f2:?} bytes={:02x?}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fields {
    pub blocksize: u32,
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub max_rice_value: u8,
    pub min_partition_order: u8,
    pub max_partition_order: u8,
    pub partition_order: u8,
    pub padding: [u8; 3],
    pub cur_blocksize: u32,
}

impl Default for Fields {
    /// A valid baseline configuration (CD audio, independent channels).
    fn default() -> Self {
        Fields {
            blocksize: 4096,
            samplerate: 44100,
            channels: 2,
            bitdepth: 16,
            channel_mode: 0,
            max_rice_value: 0,
            min_partition_order: 0,
            max_partition_order: 0,
            partition_order: 0,
            padding: [0, 0, 0],
            cur_blocksize: 0,
        }
    }
}

impl Fields {
    pub fn to_raw(self) -> Raw {
        let mut b = [0u8; TFLAC_SIZE];
        b[0..4].copy_from_slice(&self.blocksize.to_ne_bytes());
        b[4..8].copy_from_slice(&self.samplerate.to_ne_bytes());
        b[8..12].copy_from_slice(&self.channels.to_ne_bytes());
        b[12..16].copy_from_slice(&self.bitdepth.to_ne_bytes());
        b[16] = self.channel_mode;
        b[17] = self.max_rice_value;
        b[18] = self.min_partition_order;
        b[19] = self.max_partition_order;
        b[20] = self.partition_order;
        b[21..24].copy_from_slice(&self.padding);
        b[24..28].copy_from_slice(&self.cur_blocksize.to_ne_bytes());
        Raw(b)
    }

    pub fn from_raw(r: Raw) -> Fields {
        let b = r.0;
        let u32at = |i: usize| u32::from_ne_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        Fields {
            blocksize: u32at(0),
            samplerate: u32at(4),
            channels: u32at(8),
            bitdepth: u32at(12),
            channel_mode: b[16],
            max_rice_value: b[17],
            min_partition_order: b[18],
            max_partition_order: b[19],
            partition_order: b[20],
            padding: [b[21], b[22], b[23]],
            cur_blocksize: u32at(24),
        }
    }
}

/// Result of one differential `flac_validate` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub ret: i32,
    pub out: Raw,
}

fn call_validate(f: ValidateFn, input: Raw) -> Outcome {
    let mut r = input;
    let ret = unsafe { f(r.0.as_mut_ptr()) };
    Outcome { ret, out: r }
}

/// Runs `flac_validate` in BOTH shared objects on the same input and asserts
/// that the return value and all 28 struct bytes agree.
#[track_caller]
pub fn diff_validate_raw(input: Raw) -> Outcome {
    let l = libs();
    let c = call_validate(l.c.validate, input);
    let r = call_validate(l.rust.validate, input);
    if c.ret != r.ret {
        panic!(
            "flac_validate return mismatch for input {input:?}\n  C    = {}\n  Rust = {}",
            c.ret, r.ret
        );
    }
    if c.out != r.out {
        panic!(
            "flac_validate struct mismatch for input {input:?}\n  ret  = {}\n  C    = {:?}\n  Rust = {:?}\n  C bytes    = {:02x?}\n  Rust bytes = {:02x?}",
            c.ret,
            Fields::from_raw(c.out),
            Fields::from_raw(r.out),
            c.out.0,
            r.out.0
        );
    }
    c
}

#[track_caller]
pub fn diff_validate(f: Fields) -> Outcome {
    diff_validate_raw(f.to_raw())
}

/// Runs `tflac_size_memory` in BOTH shared objects and asserts equality.
#[track_caller]
pub fn diff_size_memory(blocksize: u32) -> u32 {
    let l = libs();
    let c = unsafe { (l.c.size_memory)(blocksize) };
    let r = unsafe { (l.rust.size_memory)(blocksize) };
    assert_eq!(
        c, r,
        "tflac_size_memory({blocksize}) mismatch: C = {c} (0x{c:08x}), Rust = {r} (0x{r:08x})"
    );
    c
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds keep every run reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    /// A random struct image where every byte is random (valid *and* invalid).
    pub fn raw(&mut self) -> Raw {
        let mut b = [0u8; TFLAC_SIZE];
        for x in b.iter_mut() {
            *x = self.next_u8();
        }
        Raw(b)
    }
    /// A random but always-valid configuration.
    pub fn valid_fields(&mut self) -> Fields {
        let max_po = self.range_u8(0, 15);
        Fields {
            blocksize: self.range_u32(16, 65535),
            samplerate: self.range_u32(1, 655350),
            channels: self.range_u32(1, 8),
            bitdepth: self.range_u32(1, 32),
            channel_mode: self.next_u8(),
            max_rice_value: self.range_u8(0, 30),
            min_partition_order: self.range_u8(0, max_po),
            max_partition_order: max_po,
            partition_order: self.next_u8(),
            padding: [self.next_u8(), self.next_u8(), self.next_u8()],
            cur_blocksize: self.next_u32(),
        }
    }
}

/// Reference model of the C partition-order loop, used as a sanity check that
/// the randomized rows really do reach the intended code paths.
pub fn expected_partition_order(blocksize: u32, min_po: u8, max_po: u8) -> u8 {
    let mut po = min_po;
    while blocksize % (1u32 << (po as u32 + 1)) == 0 && po < max_po {
        po += 1;
    }
    po
}
