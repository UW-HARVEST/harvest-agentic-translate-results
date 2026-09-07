//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through the exported C symbols — the Rust crate is never linked
//! or called directly, so the `#[no_mangle] extern "C"` wrappers are under
//! test too.

#![allow(dead_code)]

use libloading::Library;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

pub struct Api {
    pub which: &'static str,
    _lib: Library,
    pub pack_u64le: unsafe extern "C" fn(*mut u8, u64),
    pub md5_addsample: unsafe extern "C" fn(*mut u8, u32, u64),
    pub update_md5: unsafe extern "C" fn(*mut u8, *const i32) -> u32,
}

fn workdir() -> PathBuf {
    // tests/ lives in the crate root; the crate root's parent holds c_src/.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    // The CMake target name is derived from the parent directory name, so glob.
    let dir = workdir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {}, got {found:?}", dir.display());
    found.pop().unwrap()
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let rel = workdir().join("target/release/libupdate_md5_lib.so");
    if rel.exists() {
        return rel;
    }
    workdir().join("target/debug/libupdate_md5_lib.so")
}

impl Api {
    fn load(which: &'static str, path: &PathBuf) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            let pack_u64le = *lib
                .get::<unsafe extern "C" fn(*mut u8, u64)>(b"tflac_pack_u64le\0")
                .expect("tflac_pack_u64le not exported");
            let md5_addsample = *lib
                .get::<unsafe extern "C" fn(*mut u8, u32, u64)>(b"tflac_md5_addsample\0")
                .expect("tflac_md5_addsample not exported");
            let update_md5 = *lib
                .get::<unsafe extern "C" fn(*mut u8, *const i32) -> u32>(b"update_md5\0")
                .expect("update_md5 not exported");
            Api { which, _lib: lib, pack_u64le, md5_addsample, update_md5 }
        }
    }

    pub fn load_c() -> Api {
        Api::load("C", &c_so_path())
    }
    pub fn load_rust() -> Api {
        Api::load("RUST", &rust_so_path())
    }
}

// `Library` is Send+Sync and the rest are plain fn pointers.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

static APIS: OnceLock<(Api, Api)> = OnceLock::new();

pub fn apis() -> &'static (Api, Api) {
    APIS.get_or_init(|| (Api::load_c(), Api::load_rust()))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seeds, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
        self.next_u64() as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }
    pub fn fill(&mut self, b: &mut [u8]) {
        for x in b.iter_mut() {
            *x = self.next_u64() as u8;
        }
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u32) as usize]
    }
}

// ---------------------------------------------------------------------------
// ABI constants, verified against the C build
// ---------------------------------------------------------------------------

pub const SIZEOF_TFLAC_MD5: usize = 88;
pub const SIZEOF_TFLAC: usize = 96;
pub const OFF_POS: usize = 0;
pub const OFF_TOTAL: usize = 8;
pub const OFF_BUFFER: usize = 16;
pub const OFF_CUR_BLOCKSIZE: usize = 88;
pub const OFF_CHANNELS: usize = 92;
pub const BUFFER_LEN: usize = 64 + 8;

/// Backing region for the state. Larger than `sizeof(tflac)` so that the C
/// code's deliberate out-of-bounds reads (`buffer[64 + bytes]`, up to
/// `buffer[126]` ⇒ byte offset 142) land in memory the test controls and
/// initialises identically for both libraries.
pub const STATE_LEN: usize = 512;

/// `update_md5` reads elements `[0,8) [32,40) [64,72) [96,104) [128,136)`
/// relative to the pointer it is given, i.e. it touches up to element 135.
///
/// Tests that pass an offset pointer must keep `offset + 136 <= SAMPLES_ELEMS`,
/// otherwise the library reads past the region and the two libraries see
/// different (uninitialised) heap bytes — a test artefact, not a divergence.
/// The largest offset used anywhere is `7 * 136 = 952` (row 24), so 2048
/// elements leaves ample headroom. Enforced by `check_samples_window`.
pub const SAMPLES_ELEMS: usize = 2048;
pub const SAMPLES_LEN: usize = SAMPLES_ELEMS * 4;

/// Number of `tflac_s32` elements `update_md5` reads past its argument pointer.
pub const UPDATE_MD5_WINDOW: usize = 136;

/// Guards against the test itself reading uninitialised memory.
pub fn check_samples_window(offset_elems: usize) {
    assert!(
        offset_elems + UPDATE_MD5_WINDOW <= SAMPLES_ELEMS,
        "test bug: update_md5 at element offset {offset_elems} would read to \
         {} but the region holds only {SAMPLES_ELEMS} elements",
        offset_elems + UPDATE_MD5_WINDOW
    );
}

// ---------------------------------------------------------------------------
// 8-aligned byte region
// ---------------------------------------------------------------------------

pub struct Region {
    words: Vec<u64>,
    len: usize,
}

impl Region {
    pub fn from_bytes(src: &[u8]) -> Region {
        let len = src.len();
        let mut words = vec![0u64; (len + 7) / 8];
        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), words.as_mut_ptr() as *mut u8, len);
        }
        Region { words, len }
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.words.as_mut_ptr() as *mut u8
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.words.as_ptr() as *const u8, self.len) }
    }
}

// ---------------------------------------------------------------------------
// State builder
// ---------------------------------------------------------------------------

/// Builds a `STATE_LEN`-byte region holding a `struct tflac` at offset 0.
pub struct StateBuilder(pub Vec<u8>);

impl StateBuilder {
    /// Fully randomized backing region (including the tail past `sizeof(tflac)`,
    /// which the OOB reads touch).
    pub fn random(rng: &mut Rng) -> StateBuilder {
        let mut v = vec![0u8; STATE_LEN];
        rng.fill(&mut v);
        StateBuilder(v)
    }
    pub fn zeroed() -> StateBuilder {
        StateBuilder(vec![0u8; STATE_LEN])
    }
    pub fn pos(mut self, p: u32) -> Self {
        self.0[OFF_POS..OFF_POS + 4].copy_from_slice(&p.to_le_bytes());
        self
    }
    pub fn total(mut self, t: u64) -> Self {
        self.0[OFF_TOTAL..OFF_TOTAL + 8].copy_from_slice(&t.to_le_bytes());
        self
    }
    pub fn cur_blocksize(mut self, n: u32) -> Self {
        self.0[OFF_CUR_BLOCKSIZE..OFF_CUR_BLOCKSIZE + 4].copy_from_slice(&n.to_le_bytes());
        self
    }
    pub fn channels(mut self, n: u32) -> Self {
        self.0[OFF_CHANNELS..OFF_CHANNELS + 4].copy_from_slice(&n.to_le_bytes());
        self
    }
    pub fn build(self) -> Vec<u8> {
        self.0
    }
}

pub fn random_samples(rng: &mut Rng) -> Vec<u8> {
    let mut v = vec![0u8; SAMPLES_LEN];
    rng.fill(&mut v);
    v
}

pub fn samples_from_i32(xs: &[i32]) -> Vec<u8> {
    let mut v = vec![0u8; SAMPLES_LEN];
    for (i, x) in xs.iter().enumerate().take(SAMPLES_ELEMS) {
        v[i * 4..i * 4 + 4].copy_from_slice(&x.to_le_bytes());
    }
    v
}

// ---------------------------------------------------------------------------
// The differential driver
// ---------------------------------------------------------------------------

/// Runs `op` against both libraries over identically-initialised memory and
/// asserts that the returned value, the whole state region and the whole
/// samples region are byte-identical afterwards.
pub fn run_diff<T: PartialEq + Debug>(
    label: &str,
    state_init: &[u8],
    samples_init: &[u8],
    op: &dyn Fn(&Api, *mut u8, *const i32) -> T,
) {
    let (c, r) = apis();

    let mut c_state = Region::from_bytes(state_init);
    let mut c_samples = Region::from_bytes(samples_init);
    let c_ret = op(c, c_state.as_mut_ptr(), c_samples.as_mut_ptr() as *const i32);

    let mut r_state = Region::from_bytes(state_init);
    let mut r_samples = Region::from_bytes(samples_init);
    let r_ret = op(r, r_state.as_mut_ptr(), r_samples.as_mut_ptr() as *const i32);

    if c_ret != r_ret {
        panic!("[{label}] RETURN VALUE DIVERGED\n  C   = {c_ret:?}\n  RUST= {r_ret:?}\n{}", dump(&c_state, &r_state));
    }
    if c_state.bytes() != r_state.bytes() {
        panic!("[{label}] STATE DIVERGED\n{}", dump(&c_state, &r_state));
    }
    if c_samples.bytes() != r_samples.bytes() {
        let first = c_samples
            .bytes()
            .iter()
            .zip(r_samples.bytes())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!("[{label}] SAMPLES REGION DIVERGED at byte {first}");
    }
}

fn dump(c: &Region, r: &Region) -> String {
    let cb = c.bytes();
    let rb = r.bytes();
    let mut s = String::new();
    for (i, (a, b)) in cb.iter().zip(rb.iter()).enumerate() {
        if a != b {
            let where_ = match i {
                0..=3 => "pos".to_string(),
                4..=7 => "pad".to_string(),
                8..=15 => "total".to_string(),
                16..=87 => format!("buffer[{}]", i - 16),
                88..=91 => "cur_blocksize".to_string(),
                92..=95 => "channels".to_string(),
                _ => format!("past-tflac[+{}]", i - SIZEOF_TFLAC),
            };
            s.push_str(&format!("  byte {i:>3} ({where_}): C=0x{a:02x} RUST=0x{b:02x}\n"));
        }
    }
    if s.is_empty() {
        s.push_str("  (state regions identical)\n");
    }
    s
}

/// Reads back the observable `tflac_md5` scalar fields, for readable assertions.
pub fn get_pos(b: &[u8]) -> u32 {
    u32::from_le_bytes(b[OFF_POS..OFF_POS + 4].try_into().unwrap())
}
pub fn get_total(b: &[u8]) -> u64 {
    u64::from_le_bytes(b[OFF_TOTAL..OFF_TOTAL + 8].try_into().unwrap())
}
