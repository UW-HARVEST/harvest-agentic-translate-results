//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `md5_digest` symbol — the Rust side is
//! never called directly as a Rust function, so the `#[no_mangle] extern "C"`
//! wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// FFI mirror of the C API (`c_src/include/lib.h`)
// ---------------------------------------------------------------------------

/// Mirrors `struct tflac_md5`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    /// The struct's 16 bytes as the host lays them out.
    pub fn to_bytes(self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.a.to_ne_bytes());
        out[4..8].copy_from_slice(&self.b.to_ne_bytes());
        out[8..12].copy_from_slice(&self.c.to_ne_bytes());
        out[12..16].copy_from_slice(&self.d.to_ne_bytes());
        out
    }
    pub fn from_bytes(b: &[u8]) -> Md5 {
        Md5 {
            a: u32::from_ne_bytes([b[0], b[1], b[2], b[3]]),
            b: u32::from_ne_bytes([b[4], b[5], b[6], b[7]]),
            c: u32::from_ne_bytes([b[8], b[9], b[10], b[11]]),
            d: u32::from_ne_bytes([b[12], b[13], b[14], b[15]]),
        }
    }
}

/// `void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);`
pub type Md5DigestFn = unsafe extern "C" fn(*const Md5, *mut u8);

// ---------------------------------------------------------------------------
// Loading both shared objects
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub md5_digest: Md5DigestFn,
    // Kept alive so the function pointer stays valid.
    _lib: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// First file in `dir` whose name starts with `prefix` and ends with `.so`.
fn find_so(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            name.starts_with(prefix) && name.ends_with(".so") && p.is_file()
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

fn c_so_path() -> PathBuf {
    // Escape hatch used by verify.sh to cross-check against a differently
    // optimised C build; defaults to the canonical c_src/build output.
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HARVEST_C_SO={} is not a file", p.display());
        return p;
    }
    let build = manifest_dir().join("c_src/build");
    find_so(&build, "lib").unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    // target/<profile>/deps/<test-exe>  ->  target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(deps) = exe.parent() {
        candidates.push(deps.to_path_buf());
        if let Some(profile) = deps.parent() {
            candidates.push(profile.to_path_buf());
        }
    }
    let target = manifest_dir().join("target");
    candidates.push(target.join("debug"));
    candidates.push(target.join("release"));

    for dir in &candidates {
        if let Some(p) = find_so(dir, "libmd5_digest_lib") {
            return p;
        }
    }
    panic!(
        "Rust cdylib libmd5_digest_lib.so not found (looked in {:?}).\nBuild it with: cargo build",
        candidates
    );
}

fn load(name: &'static str, path: &Path) -> Impl {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    let md5_digest = unsafe {
        let sym: Symbol<Md5DigestFn> = lib
            .get(b"md5_digest\0")
            .unwrap_or_else(|e| panic!("dlsym(md5_digest) in {} failed: {e}", path.display()));
        *sym
    };
    Impl {
        name,
        md5_digest,
        _lib: lib,
    }
}

/// Loads (C implementation, Rust implementation).
pub fn both() -> (Impl, Impl) {
    (load("C", &c_so_path()), load("Rust", &rust_so_path()))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEE_CE66_D15E_A5E5;

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
        (self.next_u64() % n as u64) as usize
    }
    /// Random state, biased towards interesting byte patterns.
    pub fn state(&mut self) -> Md5 {
        let mut w = [0u32; 4];
        for slot in w.iter_mut() {
            *slot = match self.below(8) {
                0 => 0,
                1 => u32::MAX,
                2 => 1 << self.below(32),
                3 => u32::MAX ^ (1 << self.below(32)),
                4 => (self.next_u32() & 0xFF) * 0x0101_0101,
                _ => self.next_u32(),
            };
        }
        Md5::new(w[0], w[1], w[2], w[3])
    }
}

/// Reference little-endian serialisation (used only for extra sanity checks;
/// the authoritative comparison is always C-`.so` vs Rust-`.so`).
pub fn expected_le(m: &Md5) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&m.a.to_le_bytes());
    out[4..8].copy_from_slice(&m.b.to_le_bytes());
    out[8..12].copy_from_slice(&m.c.to_le_bytes());
    out[12..16].copy_from_slice(&m.d.to_le_bytes());
    out
}

// ---------------------------------------------------------------------------
// Guarded 32-byte scratch buffer: sentinel bytes around the 16-byte output
// ---------------------------------------------------------------------------

pub const GUARD: u8 = 0xA5;
pub const PAD: usize = 32;

/// Runs `f` with an `out` pointer at `offset` inside a sentinel-filled buffer.
/// Returns (16 output bytes, whole buffer) so callers can assert the guards.
pub fn call_with_guards(f: Md5DigestFn, m: &Md5, offset: usize) -> ([u8; 16], Vec<u8>) {
    let mut buf = vec![GUARD; PAD * 2 + 16];
    let base = PAD + offset;
    unsafe { f(m as *const Md5, buf.as_mut_ptr().add(base)) };
    let mut out = [0u8; 16];
    out.copy_from_slice(&buf[base..base + 16]);
    (out, buf)
}

/// Core differential assertion for the disjoint case: identical 16 output bytes
/// *and* identical surrounding memory (no over/under-write).
pub fn assert_same(c: &Impl, r: &Impl, m: &Md5, offset: usize, ctx: &str) {
    let (c_out, c_buf) = call_with_guards(c.md5_digest, m, offset);
    let (r_out, r_buf) = call_with_guards(r.md5_digest, m, offset);
    assert_eq!(
        c_out, r_out,
        "output mismatch [{ctx}] m={m:08x?} out_offset={offset}\n  C   : {c_out:02x?}\n  Rust: {r_out:02x?}"
    );
    assert_eq!(
        c_buf, r_buf,
        "surrounding-memory mismatch [{ctx}] m={m:08x?} out_offset={offset}"
    );
    // The C is ground truth; this only documents what that truth is.
    assert_eq!(c_out, expected_le(m), "C reference sanity [{ctx}]");
}

// ---------------------------------------------------------------------------
// libc bits for the page-protection / crash-equivalence harness
// ---------------------------------------------------------------------------

pub const PROT_NONE: i32 = 0;
pub const PROT_READ: i32 = 1;
pub const PROT_WRITE: i32 = 2;
pub const MAP_SHARED: i32 = 0x01;
pub const MAP_ANONYMOUS: i32 = 0x20;
pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;

unsafe extern "C" {
    fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
    fn sysconf(name: i32) -> i64;
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

pub fn page_size() -> usize {
    let v = unsafe { sysconf(30) }; // _SC_PAGESIZE
    if v > 0 { v as usize } else { 4096 }
}

/// Three shared anonymous pages: `[PROT_NONE guard][RW page][PROT_NONE guard]`.
///
/// `MAP_SHARED` so that writes performed by a forked child are visible to the
/// parent — that is how partial output before a fault is compared.
pub struct Guarded {
    base: *mut u8,
    total: usize,
    page: usize,
}

impl Guarded {
    pub fn new() -> Guarded {
        let page = page_size();
        let total = page * 3;
        let base = unsafe {
            mmap(
                std::ptr::null_mut(),
                total,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(base as isize != -1, "mmap failed");
        let base = base as *mut u8;
        unsafe {
            assert_eq!(
                mprotect(base as *mut c_void, page, PROT_NONE),
                0,
                "mprotect low guard"
            );
            assert_eq!(
                mprotect(base.add(page * 2) as *mut c_void, page, PROT_NONE),
                0,
                "mprotect high guard"
            );
        }
        Guarded { base, total, page }
    }

    /// Start of the read/write page.
    pub fn rw(&self) -> *mut u8 {
        unsafe { self.base.add(self.page) }
    }
    /// One past the last writable byte.
    pub fn rw_end(&self) -> *mut u8 {
        unsafe { self.base.add(self.page * 2) }
    }
    pub fn page(&self) -> usize {
        self.page
    }
    /// Make the RW page read-only (for the "unwritable `out`" error row).
    pub fn make_readonly(&self) {
        unsafe {
            assert_eq!(
                mprotect(self.rw() as *mut c_void, self.page, PROT_READ),
                0,
                "mprotect ro"
            )
        };
    }
    /// Make the RW page completely inaccessible (for the "unreadable `m`" row).
    pub fn make_none(&self) {
        unsafe {
            assert_eq!(
                mprotect(self.rw() as *mut c_void, self.page, PROT_NONE),
                0,
                "mprotect none"
            )
        };
    }
    pub fn fill(&self, byte: u8) {
        unsafe { std::ptr::write_bytes(self.rw(), byte, self.page) };
    }
    pub fn read(&self, offset: usize, len: usize) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.rw().add(offset), len).to_vec() }
    }
    pub fn write_at(&self, offset: usize, bytes: &[u8]) {
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.rw().add(offset), bytes.len()) };
    }
}

impl Drop for Guarded {
    fn drop(&mut self) {
        unsafe { munmap(self.base as *mut c_void, self.total) };
    }
}

/// How a forked child terminated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Terminating signal, or 0 if it exited normally.
    pub signal: i32,
    /// Exit status if it exited normally, else -1.
    pub code: i32,
}

impl Outcome {
    pub fn ok() -> Outcome {
        Outcome { signal: 0, code: 0 }
    }
    pub fn segv() -> Outcome {
        Outcome {
            signal: SIGSEGV,
            code: -1,
        }
    }
}

/// Runs `f` in a forked child and reports how the child terminated.
///
/// The child performs no allocation and no I/O: it just calls the loaded
/// function and `_exit(0)`, so a fault is attributable to the call alone.
pub fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: i32 = 0;
    let got = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(got, pid, "waitpid failed");
    let sig = status & 0x7f;
    if sig != 0 {
        Outcome {
            signal: sig,
            code: -1,
        }
    } else {
        Outcome {
            signal: 0,
            code: (status >> 8) & 0xff,
        }
    }
}
