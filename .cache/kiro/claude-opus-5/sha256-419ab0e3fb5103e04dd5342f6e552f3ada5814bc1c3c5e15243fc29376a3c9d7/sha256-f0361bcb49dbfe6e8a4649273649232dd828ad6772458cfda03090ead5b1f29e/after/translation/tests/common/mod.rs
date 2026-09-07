//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! only through their exported `md5_digest` symbol. The Rust implementation is
//! never called directly, so the `#[no_mangle] extern "C"` wrapper is under
//! test too.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `struct tflac_md5` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TflacMd5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

impl TflacMd5 {
    pub fn new(a: u32, b: u32, c: u32, d: u32) -> Self {
        Self { a, b, c, d }
    }
}

pub type Md5DigestFn = unsafe extern "C" fn(*const TflacMd5, *mut u8);

/// Workspace root (parent of the `translation` crate directory).
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

/// Locate the C shared library built by CMake under `c_src/build/`.
pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build.display()))
}

/// Locate the Rust `cdylib`. `CARGO_BIN_EXE_*` is unavailable for cdylibs, so
/// walk up from the integration-test binary's own directory
/// (`target/<profile>/deps/`) to `target/<profile>/`.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<test-bin>")
        .to_path_buf();
    let p = profile_dir.join("libmd5_digest_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {}. Run `cargo build` (same profile) first.",
        p.display()
    );
    p
}

/// A loaded implementation, addressed only through its exported symbol.
pub struct Impl {
    _lib: Library,
    digest: Md5DigestFn,
    pub name: &'static str,
}

impl Impl {
    fn load(path: &Path, name: &'static str) -> Self {
        // SAFETY: loading a shared object; the symbol signature is taken from
        // the C header.
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            let sym: Symbol<Md5DigestFn> = lib
                .get(b"md5_digest\0")
                .unwrap_or_else(|e| panic!("dlsym md5_digest in {} failed: {e}", path.display()));
            let digest = *sym;
            Impl { _lib: lib, digest, name }
        }
    }

    /// Raw FFI call. Caller upholds the C contract.
    ///
    /// # Safety
    /// `m` must be readable for 16 bytes and `out` writable for 16 bytes,
    /// unless the test is deliberately probing undefined behavior.
    pub unsafe fn call(&self, m: *const TflacMd5, out: *mut u8) {
        unsafe { (self.digest)(m, out) }
    }

    /// Convenience wrapper for the common disjoint, aligned case.
    pub fn digest(&self, m: &TflacMd5) -> [u8; 16] {
        let mut out = [0u8; 16];
        unsafe { self.call(m as *const TflacMd5, out.as_mut_ptr()) };
        out
    }
}

/// The C and Rust implementations, both loaded through `dlopen`.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load(&c_so_path(), "C"),
        rust: Impl::load(&rust_so_path(), "Rust"),
    }
}

impl Pair {
    /// Assert both implementations produce byte-identical output for `m`.
    pub fn assert_same(&self, label: &str, m: &TflacMd5) {
        let got_c = self.c.digest(m);
        let got_r = self.rust.digest(m);
        assert_eq!(
            got_c, got_r,
            "[{label}] divergence for {m:?}\n  C    = {got_c:02x?}\n  Rust = {got_r:02x?}"
        );
    }
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_md5(&mut self) -> TflacMd5 {
        TflacMd5::new(self.next_u32(), self.next_u32(), self.next_u32(), self.next_u32())
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// Fixed seed used by every property-style row, per `CONFIGS.md`.
pub const SEED: u64 = 0x5EED_1234;

/// A byte buffer with 8-byte alignment, backed by `Vec<u64>`, so that a
/// `tflac_md5` placed at offset 0 / 4 / 8 is naturally aligned and offsets
/// 1 / 2 / 3 are deliberately misaligned.
pub struct AlignedBuf {
    words: Vec<u64>,
    len: usize,
}

impl AlignedBuf {
    pub fn new(len: usize, fill: u8) -> Self {
        let words = vec![u64::from_ne_bytes([fill; 8]); len.div_ceil(8) + 1];
        AlignedBuf { words, len }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.words.as_ptr() as *const u8
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.words.as_mut_ptr() as *mut u8
    }
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: `len` <= allocated byte capacity.
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len) }
    }
    /// Write the 16 little-endian bytes of `m` at `off`.
    pub fn put_md5(&mut self, off: usize, m: &TflacMd5) {
        assert!(off + 16 <= self.len);
        let mut raw = [0u8; 16];
        raw[0..4].copy_from_slice(&m.a.to_le_bytes());
        raw[4..8].copy_from_slice(&m.b.to_le_bytes());
        raw[8..12].copy_from_slice(&m.c.to_le_bytes());
        raw[12..16].copy_from_slice(&m.d.to_le_bytes());
        // SAFETY: bounds asserted above.
        unsafe { std::ptr::copy_nonoverlapping(raw.as_ptr(), self.as_mut_ptr().add(off), 16) };
    }
}

/// Run one aliasing/offset case against both implementations and compare the
/// **entire** buffer afterwards (writes may land inside `m`'s own storage).
///
/// `m_off` is where the `tflac_md5` lives, `out_off` where `out` points.
pub fn assert_same_in_buffer(
    pair: &Pair,
    label: &str,
    buf_len: usize,
    m_off: usize,
    out_off: usize,
    m: &TflacMd5,
    fill: u8,
) {
    assert!(m_off + 16 <= buf_len && out_off + 16 <= buf_len, "case fits in buffer");

    let run = |imp: &Impl| -> Vec<u8> {
        let mut buf = AlignedBuf::new(buf_len, fill);
        buf.put_md5(m_off, m);
        // SAFETY: both regions lie inside the buffer; overlap is legal because
        // neither C parameter is `restrict`-qualified.
        unsafe {
            let mp = buf.as_mut_ptr().add(m_off) as *const TflacMd5;
            let op = buf.as_mut_ptr().add(out_off);
            imp.call(mp, op);
        }
        buf.bytes().to_vec()
    };

    let got_c = run(&pair.c);
    let got_r = run(&pair.rust);
    assert_eq!(
        got_c, got_r,
        "[{label}] buffer divergence (buf_len={buf_len} m_off={m_off} out_off={out_off} {m:?})\n\
         \x20 C    = {got_c:02x?}\n  Rust = {got_r:02x?}"
    );
}
