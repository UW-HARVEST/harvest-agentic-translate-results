//! Shared test harness: loads the C reference .so and the Rust .so side by
//! side and provides helpers to call the same exported symbol in both and
//! compare results byte-for-byte.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_uchar, c_ulonglong, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type Uchar = c_uchar;

/// A pair of loaded libraries: `.0` = C reference, `.1` = Rust translation.
pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libsodium.so")
}

/// Locate the Rust cdylib. The test executable lives in
/// `target/<profile>/deps/`, so the cdylib is normally one directory up. If it
/// has not been produced for this profile (a plain `cargo test` does not always
/// emit the cdylib), fall back to any other profile directory under `target/`.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for cand in [
        profile.join("liblibsodium.so"),
        deps.join("liblibsodium.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    // Fall back to a sibling profile directory.
    if let Some(target) = profile.parent() {
        for p in ["release", "debug"] {
            let cand = target.join(p).join("liblibsodium.so");
            if cand.exists() {
                eprintln!(
                    "note: using {} (no cdylib for this profile)",
                    cand.display()
                );
                return cand;
            }
        }
    }
    panic!(
        "could not locate liblibsodium.so near {}.\nBuild it first: cd translation && cargo build --release",
        profile.display()
    );
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Load (once) and return both libraries, with `sodium_init` called on each
/// and a deterministic randombytes implementation installed in both.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        assert!(
            c_path.exists(),
            "C reference library not built: {:?}\nBuild it with: cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path
        );
        let rs_path = rust_so_path();
        // RTLD_LOCAL (libloading's default) keeps the two symbol namespaces
        // separate so `sodium_memzero` in one cannot bind to the other.
        let c = unsafe { Library::new(&c_path) }.expect("load C .so");
        let rs = unsafe { Library::new(&rs_path) }.expect("load Rust .so");
        let libs = Libs { c, rs };
        unsafe {
            let ci: Symbol<unsafe extern "C" fn() -> c_int> =
                libs.c.get(b"sodium_init\0").unwrap();
            let ri: Symbol<unsafe extern "C" fn() -> c_int> =
                libs.rs.get(b"sodium_init\0").unwrap();
            assert!(ci() >= 0, "C sodium_init failed");
            assert!(ri() >= 0, "Rust sodium_init failed");
        }
        install_det_rng(&libs);
        libs
    })
}

// ---------------------------------------------------------------------------
// Deterministic randombytes implementation, installed into both libraries so
// that functions which consume randomness (keypair generation, sealed boxes,
// ...) produce comparable output.
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct RandombytesImplementation {
    pub implementation_name: Option<unsafe extern "C" fn() -> *const c_char>,
    pub random: Option<unsafe extern "C" fn() -> u32>,
    pub stir: Option<unsafe extern "C" fn()>,
    pub uniform: Option<unsafe extern "C" fn(u32) -> u32>,
    pub buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    pub close: Option<unsafe extern "C" fn() -> c_int>,
}
unsafe impl Sync for RandombytesImplementation {}

// The deterministic counter is thread-local: `cargo test` runs test functions
// on separate threads, and each must be able to rewind "its" random stream
// without disturbing the others.
std::thread_local! {
    static DET_CTR: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Reset the deterministic RNG stream for the current thread. Call this
/// immediately before invoking the C function and again immediately before
/// invoking the Rust function so both observe an identical byte stream.
pub fn det_reset() {
    DET_CTR.with(|c| c.set(0));
}

/// xorshift-style keystream: a pure function of the call offset.
fn det_byte(i: u64) -> u8 {
    let mut x = i.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x1234_5678_9ABC_DEF0);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x & 0xff) as u8
}

unsafe extern "C" fn det_buf(buf: *mut c_void, size: usize) {
    let p = buf as *mut u8;
    DET_CTR.with(|ctr| {
        let mut n = ctr.get();
        for i in 0..size {
            *p.add(i) = det_byte(n);
            n = n.wrapping_add(1);
        }
        ctr.set(n);
    });
}

unsafe extern "C" fn det_random() -> u32 {
    let mut b = [0u8; 4];
    det_buf(b.as_mut_ptr() as *mut c_void, 4);
    u32::from_le_bytes(b)
}

unsafe extern "C" fn det_stir() {}

unsafe extern "C" fn det_close() -> c_int {
    0
}

static DET_NAME: &[u8] = b"det_test\0";

unsafe extern "C" fn det_name() -> *const c_char {
    DET_NAME.as_ptr() as *const c_char
}

static DET_IMPL: RandombytesImplementation = RandombytesImplementation {
    implementation_name: Some(det_name),
    random: Some(det_random),
    stir: Some(det_stir),
    uniform: None, // exercise each library's own default uniform()
    buf: Some(det_buf),
    close: Some(det_close),
};

fn install_det_rng(libs: &Libs) {
    unsafe {
        let cs: Symbol<unsafe extern "C" fn(*const RandombytesImplementation) -> c_int> =
            libs.c.get(b"randombytes_set_implementation\0").unwrap();
        let rt: Symbol<unsafe extern "C" fn(*const RandombytesImplementation) -> c_int> =
            libs.rs.get(b"randombytes_set_implementation\0").unwrap();
        assert_eq!(cs(&DET_IMPL as *const _), 0);
        assert_eq!(rt(&DET_IMPL as *const _), 0);
    }
}

/// Re-install the deterministic randombytes implementation in both libraries.
/// Any test that swaps in a different implementation must call this afterwards,
/// because the implementation pointer is global library state.
pub fn restore_det_rng() {
    install_det_rng(libs());
    det_reset();
}

// ---------------------------------------------------------------------------
// Symbol lookup helpers
// ---------------------------------------------------------------------------

/// Fetch symbol `name` from both libraries as function pointers of type `F`.
///
/// # Safety
/// `F` must be the correct `extern "C" fn` signature for `name`.
pub unsafe fn pair<F: Copy>(name: &str) -> (F, F) {
    let l = libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    let c: Symbol<F> = l
        .c
        .get(&n)
        .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
    let r: Symbol<F> = l
        .rs
        .get(&n)
        .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
    (*c, *r)
}

/// True when both libraries export `name`.
pub fn has(name: &str) -> bool {
    let l = libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe {
        l.c.get::<*const c_void>(&n).is_ok() && l.rs.get::<*const c_void>(&n).is_ok()
    }
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-random test data
// ---------------------------------------------------------------------------

/// Small, fast, deterministic byte generator for test vectors (splitmix64).
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0xDEAD_BEEF_CAFE_BABE)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() & 0xff) as u8
    }
    pub fn fill(&mut self, b: &mut [u8]) {
        for x in b.iter_mut() {
            *x = self.byte();
        }
    }
    pub fn vec(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
}

/// Assert two byte slices are equal, with a hex diff on failure.
pub fn assert_bytes_eq(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let idx = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c.len().min(r.len()));
        panic!(
            "{what}: output mismatch (C len {}, Rust len {}) first diff at byte {idx}\n  C   : {}\n  Rust: {}",
            c.len(),
            r.len(),
            hex(&c[..c.len().min(idx + 32)]),
            hex(&r[..r.len().min(idx + 32)]),
        );
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Common `extern "C"` signatures used across the tests.
pub type FnBufLen = unsafe extern "C" fn(*mut c_uchar, usize) -> c_int;
pub type FnSize = unsafe extern "C" fn() -> usize;
pub type FnInt = unsafe extern "C" fn() -> c_int;
pub type FnCStr = unsafe extern "C" fn() -> *const c_char;
pub type FnVerify = unsafe extern "C" fn(*const c_uchar, *const c_uchar) -> c_int;

/// Hash-style: (out, in, inlen) -> int
pub type FnHash = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, c_ulonglong) -> c_int;

/// Compare a `() -> size_t` constant getter in both libraries.
pub fn cmp_size(name: &str) {
    unsafe {
        let (c, r): (FnSize, FnSize) = pair(name);
        assert_eq!(c(), r(), "{name}() differs");
    }
}

/// Compare a `() -> int` constant getter in both libraries.
pub fn cmp_int(name: &str) {
    unsafe {
        let (c, r): (FnInt, FnInt) = pair(name);
        assert_eq!(c(), r(), "{name}() differs");
    }
}

/// Compare a `() -> const char *` getter in both libraries.
pub fn cmp_cstr(name: &str) {
    unsafe {
        let (c, r): (FnCStr, FnCStr) = pair(name);
        let cs = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let rs = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        assert_eq!(
            String::from_utf8_lossy(&cs),
            String::from_utf8_lossy(&rs),
            "{name}() differs"
        );
    }
}

// ---------------------------------------------------------------------------
// Reusable streaming-primitive harness
// ---------------------------------------------------------------------------

/// Heap buffer whose payload is guaranteed 16-byte aligned, for opaque
/// `CRYPTO_ALIGN(16)` state structs.
pub struct AlignedBuf {
    ptr: *mut u8,
    len: usize,
}

impl AlignedBuf {
    pub fn new(len: usize, fill: u8) -> Self {
        let cap = ((len + 15) / 16 + 1) * 16;
        let layout = std::alloc::Layout::from_size_align(cap, 16).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        assert!(!ptr.is_null());
        unsafe { std::ptr::write_bytes(ptr, fill, cap) };
        assert_eq!(ptr as usize % 16, 0);
        AlignedBuf { ptr, len: cap }
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::from_size_align(self.len, 16).unwrap();
        unsafe { std::alloc::dealloc(self.ptr, layout) };
    }
}

/// Message lengths covering empty input, sub-block, block boundaries and
/// multi-block inputs for every block size libsodium uses (16/64/128/136/168).
pub fn msg_lens() -> Vec<usize> {
    let mut v: Vec<usize> = (0..200).collect();
    v.extend([
        255, 256, 257, 271, 272, 273, 335, 336, 337, 511, 512, 513, 1000, 1023, 1024, 1025, 2048,
        4096, 5000,
    ]);
    v.sort_unstable();
    v.dedup();
    v
}

/// A smaller length set, for expensive primitives.
pub fn msg_lens_small() -> Vec<usize> {
    let mut v: Vec<usize> = vec![
        0, 1, 2, 3, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 135, 136, 137, 143,
        144, 145, 167, 168, 169, 199, 200, 201, 255, 256, 257, 512, 1000, 1024, 2048,
    ];
    v.sort_unstable();
    v.dedup();
    v
}

/// Chunk splits used to exercise streaming update() buffering.
pub fn chunkings(total: usize) -> Vec<Vec<usize>> {
    if total == 0 {
        return vec![vec![], vec![0], vec![0, 0]];
    }
    let mut out: Vec<Vec<usize>> = Vec::new();
    out.push(vec![total]);
    out.push(vec![0, total, 0]);
    // byte at a time (bounded)
    let n1 = total.min(80);
    let mut acc: Vec<usize> = std::iter::repeat(1).take(n1).collect();
    if total > n1 {
        acc.push(total - n1);
    }
    out.push(acc);
    let h = total / 2;
    out.push(vec![h, total - h]);
    for step in [63usize, 137, 65, 129, 17] {
        let mut acc = Vec::new();
        let mut rem = total;
        while rem > 0 {
            let n = step.min(rem);
            acc.push(n);
            rem -= n;
        }
        out.push(acc);
    }
    // geometrically growing chunks
    let mut acc = Vec::new();
    let mut rem = total;
    let mut step = 1usize;
    while rem > 0 {
        let n = step.min(rem);
        acc.push(n);
        rem -= n;
        step = step * 2 + 1;
    }
    out.push(acc);
    out
}
