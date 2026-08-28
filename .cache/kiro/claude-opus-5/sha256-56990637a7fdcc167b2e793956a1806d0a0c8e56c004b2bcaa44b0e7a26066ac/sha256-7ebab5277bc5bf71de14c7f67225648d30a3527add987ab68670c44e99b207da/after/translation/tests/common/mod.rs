//! Shared harness: locates and loads both the C reference `.so` and the Rust
//! `cdylib`, then exposes their exported symbols through identical FFI
//! signatures so the two can be driven with byte-identical inputs.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Layout constants, mirrored from `c_src/include/lib.h`.
pub const MD5_BUFFER_LEN: usize = 64 + 8; // 72
pub const SIZEOF_TFLAC_MD5: usize = 88; // {u32 pos; pad; u64 total; u8 buf[72]}
pub const SIZEOF_TFLAC: usize = 96; // {tflac_md5; u32; u32}

pub const MD5_POS_OFF: usize = 0;
pub const MD5_TOTAL_OFF: usize = 8;
pub const MD5_BUFFER_OFF: usize = 16;

pub const TFLAC_MD5_OFF: usize = 0;
pub const TFLAC_CUR_BLOCKSIZE_OFF: usize = 88;
pub const TFLAC_CHANNELS_OFF: usize = 92;

/// `tflac_md5_addsample` can read `buffer[64 + bytes]` for `bytes` up to 63,
/// i.e. up to `buffer[126]`, which is past the end of the 72-byte buffer. The
/// C code does that deliberately, so both implementations are handed an arena
/// with plenty of initialised slack behind the struct. Reads therefore land on
/// identical, defined bytes on both sides and the results stay comparable.
pub const ARENA: usize = 512;

type FnPackU64Le = unsafe extern "C" fn(*mut u8, u64);
type FnAddSample = unsafe extern "C" fn(*mut u8, u32, u64);
type FnUpdateMd5 = unsafe extern "C" fn(*mut u8, *const i32) -> u32;

/// One loaded implementation (either the C reference or the Rust translation).
pub struct Impl {
    _lib: Library,
    pack_u64le: FnPackU64Le,
    md5_addsample: FnAddSample,
    update_md5: FnUpdateMd5,
    pub name: &'static str,
}

impl Impl {
    fn load(path: &Path, name: &'static str) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        unsafe {
            let pack: Symbol<FnPackU64Le> = lib
                .get(b"tflac_pack_u64le\0")
                .expect("missing symbol tflac_pack_u64le");
            let add: Symbol<FnAddSample> = lib
                .get(b"tflac_md5_addsample\0")
                .expect("missing symbol tflac_md5_addsample");
            let upd: Symbol<FnUpdateMd5> =
                lib.get(b"update_md5\0").expect("missing symbol update_md5");
            let (pack_u64le, md5_addsample, update_md5) = (*pack, *add, *upd);
            Impl {
                _lib: lib,
                pack_u64le,
                md5_addsample,
                update_md5,
                name,
            }
        }
    }

    /// `void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n)`
    pub fn pack_u64le(&self, d: &mut [u8], n: u64) {
        assert!(d.len() >= 8);
        unsafe { (self.pack_u64le)(d.as_mut_ptr(), n) }
    }

    /// `void tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_uint val)`
    pub fn md5_addsample(&self, arena: &mut [u8], bits: u32, val: u64) {
        assert!(arena.len() >= ARENA);
        unsafe { (self.md5_addsample)(arena.as_mut_ptr(), bits, val) }
    }

    /// `tflac_u32 update_md5(tflac *t, const tflac_s32 *samples)`
    pub fn update_md5(&self, arena: &mut [u8], samples: &[i32]) -> u32 {
        assert!(arena.len() >= ARENA);
        assert!(samples.len() >= UPDATE_MD5_SAMPLES_READ);
        unsafe { (self.update_md5)(arena.as_mut_ptr(), samples.as_ptr()) }
    }
}

/// `update_md5` runs 5 iterations; iteration `i` reads `samples[32*i .. 32*i+8]`
/// because the stride is `8 * sizeof(tflac_s32) == 32` elements. The highest
/// index touched is therefore `32*4 + 7 == 135`.
pub const UPDATE_MD5_SAMPLES_READ: usize = 136;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} -- build the C library first", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    found.sort();
    match found.len() {
        0 => panic!("no .so found in {}", dir.display()),
        _ => found.remove(0),
    }
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("PARITY_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "PARITY_RUST_SO={} is not a file", p.display());
        return p;
    }
    // `cargo test` does not necessarily emit the cdylib, so accept whichever
    // profile directory has it; `run_tests.sh` builds it first.
    let root = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libupdate_md5_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libupdate_md5_lib.so not found under {} -- run `cargo build --release` first",
        root.display()
    );
}

/// Loads the C reference and the Rust translation, both via `dlopen`.
pub fn load_pair() -> (Impl, Impl) {
    (
        Impl::load(&find_c_so(), "C"),
        Impl::load(&find_rust_so(), "Rust"),
    )
}

/// Deterministic xorshift64* so both implementations see the exact same inputs.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// Builds a fresh arena. Slack bytes get a deterministic, non-zero fill so an
/// out-of-bounds read that differs between the two builds cannot hide behind
/// coincidental zeroes.
pub fn fresh_arena(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    (0..ARENA).map(|_| (rng.next_u32() & 0xFF) as u8).collect()
}

pub fn write_u32(a: &mut [u8], off: usize, v: u32) {
    a[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

pub fn write_u64(a: &mut [u8], off: usize, v: u64) {
    a[off..off + 8].copy_from_slice(&v.to_le_bytes());
}

pub fn read_u32(a: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(a[off..off + 4].try_into().unwrap())
}

pub fn read_u64(a: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(a[off..off + 8].try_into().unwrap())
}

/// Asserts two arenas are byte-identical, reporting the first divergence.
pub fn assert_arenas_eq(c: &[u8], r: &[u8], case: &str) {
    if c == r {
        return;
    }
    let idx = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .expect("lengths differ");
    panic!(
        "{case}: memory mismatch at byte {idx}: C=0x{:02x} Rust=0x{:02x}\n  C   : {:02x?}\n  Rust: {:02x?}",
        c[idx],
        r[idx],
        &c[idx.saturating_sub(8)..(idx + 8).min(c.len())],
        &r[idx.saturating_sub(8)..(idx + 8).min(r.len())],
    );
}
