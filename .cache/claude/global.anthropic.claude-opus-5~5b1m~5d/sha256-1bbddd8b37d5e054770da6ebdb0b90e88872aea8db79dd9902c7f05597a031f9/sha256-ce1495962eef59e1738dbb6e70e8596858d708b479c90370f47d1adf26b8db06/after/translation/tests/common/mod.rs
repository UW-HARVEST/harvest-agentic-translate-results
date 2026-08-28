//! Shared support code for the differential test suite.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called through their exported `md5_digest` symbol. The Rust implementation is
//! *never* called directly as a Rust function — always via the `.so` export, so
//! the `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `void md5_digest(const tflac_md5 *m, tflac_u8 out[16])`
pub type Md5DigestFn = unsafe extern "C" fn(*const Md5, *mut u8);

/// Layout-compatible mirror of `struct tflac_md5`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Md5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

/// Which implementation is under the microscope.
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

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object built by CMake.
///
/// The CMake project name is derived from the *parent directory name*
/// (`cmake_path(GET parent FILENAME project_name)`), so the file name is not
/// fixed — glob for it instead of hard-coding.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n.starts_with("lib"))
            {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        ),
        _ => found.remove(0),
    }
}

/// Locate the Rust cdylib **for the profile this test binary was built with**.
///
/// Derived from `current_exe()` (`target/<profile>/deps/<test>-<hash>`) so a
/// stale `.so` from another profile can never be picked up by accident.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test-bin>  ->  target/<profile>
    let profile_dir: &Path = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary should live in target/<profile>/deps/");
    let candidates = [
        profile_dir.join("libmd5_digest_lib.so"),
        profile_dir.join("deps/libmd5_digest_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libmd5_digest_lib.so not found next to {}. Run `cargo build --offline` first.",
        exe.display()
    );
}

/// Both libraries, kept alive for the lifetime of a test.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            Libs {
                c: Library::new(c_so_path()).expect("load C .so"),
                rust: Library::new(rust_so_path()).expect("load Rust .so"),
            }
        }
    }

    pub fn digest(&self, which: Impl) -> Symbol<'_, Md5DigestFn> {
        let lib = match which {
            Impl::C => &self.c,
            Impl::Rust => &self.rust,
        };
        unsafe { lib.get(b"md5_digest\0").expect("md5_digest symbol") }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — property-style testing with a fixed seed.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

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
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// A `u32` assembled from boundary bytes — puts extreme values in every
    /// byte position, which plain uniform randomness rarely does.
    pub fn sparse_u32(&mut self) -> u32 {
        const POOL: [u8; 6] = [0x00, 0x01, 0x7F, 0x80, 0xFE, 0xFF];
        let mut v = 0u32;
        for i in 0..4 {
            let b = POOL[self.below(POOL.len())] as u32;
            v |= b << (8 * i);
        }
        v
    }
    pub fn md5(&mut self) -> Md5 {
        Md5 {
            a: self.next_u32(),
            b: self.next_u32(),
            c: self.next_u32(),
            d: self.next_u32(),
        }
    }
    pub fn md5_sparse(&mut self) -> Md5 {
        Md5 {
            a: self.sparse_u32(),
            b: self.sparse_u32(),
            c: self.sparse_u32(),
            d: self.sparse_u32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Aligned scratch region — lets a test place `m` and `out` at exact offsets,
// including overlapping ones, with byte-identical geometry for both impls.
// ---------------------------------------------------------------------------

pub struct Region {
    ptr: *mut u8,
    len: usize,
    layout: std::alloc::Layout,
}

impl Region {
    /// 64-byte-aligned allocation of `len` bytes, so offsets used by a test
    /// translate to identical relative alignment in both runs.
    pub fn new(len: usize) -> Region {
        let layout = std::alloc::Layout::from_size_align(len, 64).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        assert!(!ptr.is_null(), "allocation failed");
        Region { ptr, len, layout }
    }
    pub fn fill(&mut self, b: u8) {
        unsafe { std::ptr::write_bytes(self.ptr, b, self.len) }
    }
    pub fn write_at(&mut self, off: usize, bytes: &[u8]) {
        assert!(off + bytes.len() <= self.len);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.add(off), bytes.len()) }
    }
    pub fn at(&self, off: usize) -> *mut u8 {
        assert!(off <= self.len);
        unsafe { self.ptr.add(off) }
    }
    pub fn snapshot(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len).to_vec() }
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) }
    }
}

/// A fully-specified call geometry: where the source struct lives, where `out`
/// lives, and what the surrounding memory looks like. Comparing the *entire*
/// buffer afterwards simultaneously checks the output bytes, the write extent
/// (guard bytes must be untouched) and the aliasing semantics.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub buf_len: usize,
    pub m_off: usize,
    pub out_off: usize,
    pub fill: u8,
    pub src: [u8; 16],
}

impl Scenario {
    pub fn disjoint(m: Md5) -> Scenario {
        Scenario {
            buf_len: 128,
            m_off: 0,
            out_off: 64,
            fill: 0xAA,
            src: md5_to_le_bytes(m),
        }
    }
    /// Run this scenario against one implementation, returning the whole buffer.
    pub fn run(&self, libs: &Libs, which: Impl) -> Vec<u8> {
        let f = libs.digest(which);
        let mut r = Region::new(self.buf_len);
        r.fill(self.fill);
        r.write_at(self.m_off, &self.src);
        unsafe { f(r.at(self.m_off) as *const Md5, r.at(self.out_off)) };
        r.snapshot()
    }
    /// Run against both and assert byte-identical *whole-buffer* results.
    pub fn assert_match(&self, libs: &Libs, label: &str) {
        let got_c = self.run(libs, Impl::C);
        let got_r = self.run(libs, Impl::Rust);
        if got_c != got_r {
            let diff: Vec<usize> = (0..got_c.len()).filter(|&i| got_c[i] != got_r[i]).collect();
            panic!(
                "{label}: C and Rust disagree\n  scenario: buf_len={} m_off={} out_off={} fill={:#04x}\n  src   = {:02x?}\n  C     = {:02x?}\n  Rust  = {:02x?}\n  differing byte offsets: {:?}",
                self.buf_len, self.m_off, self.out_off, self.fill, self.src, got_c, got_r, diff
            );
        }
    }
}

/// The little-endian in-memory image of a `tflac_md5`.
pub fn md5_to_le_bytes(m: Md5) -> [u8; 16] {
    let mut o = [0u8; 16];
    o[0..4].copy_from_slice(&m.a.to_ne_bytes());
    o[4..8].copy_from_slice(&m.b.to_ne_bytes());
    o[8..12].copy_from_slice(&m.c.to_ne_bytes());
    o[12..16].copy_from_slice(&m.d.to_ne_bytes());
    o
}

/// Simple disjoint call: returns the 16 output bytes.
pub fn digest16(libs: &Libs, which: Impl, m: Md5) -> [u8; 16] {
    let f = libs.digest(which);
    let mut out = [0u8; 16];
    unsafe { f(&m as *const Md5, out.as_mut_ptr()) };
    out
}

/// Disjoint call with a caller-chosen pre-fill of `out`, so "byte never stored"
/// is distinguishable from "byte stored as 0".
pub fn digest16_prefill(libs: &Libs, which: Impl, m: Md5, prefill: u8) -> [u8; 16] {
    let f = libs.digest(which);
    let mut out = [prefill; 16];
    unsafe { f(&m as *const Md5, out.as_mut_ptr()) };
    out
}

// ---------------------------------------------------------------------------
// Raw mmap bindings (no libc crate needed) for guard-page tests.
// ---------------------------------------------------------------------------

pub const PROT_NONE: i32 = 0;
pub const PROT_READ: i32 = 1;
pub const PROT_WRITE: i32 = 2;
pub const MAP_PRIVATE: i32 = 0x02;
pub const MAP_ANONYMOUS: i32 = 0x20;
pub const PAGE: usize = 4096;

unsafe extern "C" {
    pub unsafe fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut core::ffi::c_void;
    pub unsafe fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
    pub unsafe fn mprotect(addr: *mut core::ffi::c_void, len: usize, prot: i32) -> i32;
}

/// Two pages: page 0 readable+writable, page 1 `PROT_NONE`. An access one byte
/// past the end of page 0 is therefore a deterministic `SIGSEGV` instead of
/// silent corruption.
pub struct GuardedPage {
    base: *mut u8,
}

impl GuardedPage {
    pub fn new() -> GuardedPage {
        unsafe {
            let base = mmap(
                core::ptr::null_mut(),
                2 * PAGE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(base as isize != -1, "mmap failed");
            let rc = mprotect(base.add(PAGE), PAGE, PROT_NONE);
            assert_eq!(rc, 0, "mprotect failed");
            GuardedPage { base: base as *mut u8 }
        }
    }
    /// Pointer such that exactly `n` writable bytes remain before the guard.
    pub fn end_minus(&self, n: usize) -> *mut u8 {
        unsafe { self.base.add(PAGE - n) }
    }
    pub fn write_at_end(&self, n: usize, bytes: &[u8]) {
        assert!(bytes.len() <= n);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.end_minus(n), bytes.len()) }
    }
    pub fn read_end(&self, n: usize) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.end_minus(n) as *const u8, n).to_vec() }
    }
    pub fn fill_end(&self, n: usize, b: u8) {
        unsafe { std::ptr::write_bytes(self.end_minus(n), b, n) }
    }
}

impl Drop for GuardedPage {
    fn drop(&mut self) {
        unsafe {
            munmap(self.base as *mut core::ffi::c_void, 2 * PAGE);
        }
    }
}
