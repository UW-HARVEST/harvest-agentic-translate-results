//! Shared differential-test harness.
//!
//! Both implementations are loaded *as shared objects* through `libloading` and
//! called only through their exported C symbols — the Rust crate is never
//! linked directly, so the `#[no_mangle] extern "C"` wrappers are part of what
//! is under test.
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use libloading::Library;

// ---------------------------------------------------------------------------
// C types (mirrors of the anonymous typedefs in c_src/src/lib.c)
// ---------------------------------------------------------------------------

/// `typedef struct { int id; char name[32]; uint8_t flags; } DataBlock;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: u8,
}

/// `typedef struct { int *data; size_t size; } MemoryBlock;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

pub type CreateBlockFn = unsafe extern "C" fn(c_int, *const c_char, u8) -> DataBlock;
/// Same symbol, but declared with a 32-bit `flags` slot so we can push
/// out-of-range values through the narrow parameter (ERRORS.md #16).
pub type CreateBlockWideFn = unsafe extern "C" fn(c_int, *const c_char, c_int) -> DataBlock;
pub type AllocateBlockFn = unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock;
pub type FreeBlockFn = unsafe extern "C" fn(*mut MemoryBlock);
pub type ComputeHashFn = unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int;
pub type BetagammaFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn c_lib_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

pub fn rust_lib_path() -> PathBuf {
    // <target>/<profile>/deps/<test-exe>  ->  <target>/<profile>/libbetagamma_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let direct = dir.join("libbetagamma_lib.so");
    if direct.exists() {
        return direct;
    }
    // fall back to a sibling profile directory if the test exe lives elsewhere
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("libbetagamma_lib.so")
}

/// `cargo test` does **not** rebuild a `cdylib`-only lib target (an integration
/// test cannot link it), so a stale `.so` would silently be tested.  Guard
/// against that by comparing modification times with `src/lib.rs`.
fn assert_so_fresh(so: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs");
    if let (Ok(a), Ok(b)) = (std::fs::metadata(so), std::fs::metadata(&src)) {
        if let (Ok(ta), Ok(tb)) = (a.modified(), b.modified()) {
            assert!(
                ta >= tb,
                "STALE {} (older than src/lib.rs) — run `cargo build` before `cargo test`",
                so.display()
            );
        }
    }
}

/// One loaded implementation: all five exported entry points.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub create_block: CreateBlockFn,
    pub create_block_wide: CreateBlockWideFn,
    pub allocate_block: AllocateBlockFn,
    pub free_block: FreeBlockFn,
    pub compute_hash: ComputeHashFn,
    pub betagamma: BetagammaFn,
}

impl Impl {
    pub fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
            let create_block: CreateBlockFn = *lib
                .get(b"create_block\0")
                .unwrap_or_else(|e| panic!("{name}: create_block: {e}"));
            let allocate_block: AllocateBlockFn = *lib
                .get(b"allocate_block\0")
                .unwrap_or_else(|e| panic!("{name}: allocate_block: {e}"));
            let free_block: FreeBlockFn = *lib
                .get(b"free_block\0")
                .unwrap_or_else(|e| panic!("{name}: free_block: {e}"));
            let compute_hash: ComputeHashFn = *lib
                .get(b"compute_hash\0")
                .unwrap_or_else(|e| panic!("{name}: compute_hash: {e}"));
            let betagamma: BetagammaFn = *lib
                .get(b"betagamma\0")
                .unwrap_or_else(|e| panic!("{name}: betagamma: {e}"));
            let create_block_wide: CreateBlockWideFn =
                core::mem::transmute::<CreateBlockFn, CreateBlockWideFn>(create_block);
            Impl {
                name,
                _lib: lib,
                create_block,
                create_block_wide,
                allocate_block,
                free_block,
                compute_hash,
                betagamma,
            }
        }
    }
}

/// `(c_implementation, rust_implementation)`
pub fn both() -> (Impl, Impl) {
    let rp = rust_lib_path();
    assert_so_fresh(&rp);
    let c = Impl::load("C", &c_lib_path());
    let r = Impl::load("Rust", &rp);
    (c, r)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — reproducible property-style inputs
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
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish in `0..n`
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// An `i32` biased towards "interesting" magnitudes.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(8) {
            0 => 0,
            1 => (self.below(21) as i64 - 10) as i32,
            2 => i32::MAX,
            3 => i32::MIN,
            4 => (self.below(2000) as i64 - 1000) as i32,
            5 => self.next_i32() / 2,
            6 => -(self.below(1_000_000_000) as i64) as i32,
            _ => self.next_i32(),
        }
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// libc bits used by the harness
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct Rlimit {
    pub cur: u64,
    pub max: u64,
}

pub const RLIMIT_AS: c_int = 9;

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    fn fork() -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    pub fn setrlimit(resource: c_int, rlim: *const Rlimit) -> c_int;
}

pub const MAX_PAYLOAD: usize = 16384;

/// What a forked child produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildResult {
    pub bytes: Vec<u8>,
    pub raw_status: i32,
}

impl ChildResult {
    pub fn signal(&self) -> Option<i32> {
        let s = self.raw_status & 0x7f;
        if s != 0 && s != 0x7f { Some(s) } else { None }
    }
    pub fn exit_code(&self) -> Option<i32> {
        if self.raw_status & 0x7f == 0 {
            Some((self.raw_status >> 8) & 0xff)
        } else {
            None
        }
    }
    pub fn i32s(&self) -> Vec<i32> {
        self.bytes
            .chunks_exact(4)
            .map(|c| i32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }
    /// Short description used in assertion messages.
    pub fn describe(&self) -> String {
        match (self.exit_code(), self.signal()) {
            (Some(c), _) => format!("exit({c}) payload={:?}", self.bytes),
            (_, Some(s)) => format!("signal({s}) payload={:?}", self.bytes),
            _ => format!("status(0x{:x}) payload={:?}", self.raw_status, self.bytes),
        }
    }
}

/// Run `f(which, buf)` in a freshly forked child and collect what it wrote.
///
/// Performs **no heap allocation** between `fork()` calls (see `CONFIGS.md`
/// note N2): the output buffer is supplied by the caller, so two children
/// forked back to back observe byte-identical allocator state.
fn run_child<F>(f: &F, which: bool, out: &mut [u8]) -> (usize, i32)
where
    F: Fn(bool, &mut [u8]) -> usize,
{
    let mut fds = [-1 as c_int; 2];
    let rc = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(rc, 0, "pipe() failed");

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // ---------------- child ----------------
        unsafe { close(fds[0]) };
        let mut cbuf = [0u8; MAX_PAYLOAD];
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(which, &mut cbuf)));
        match res {
            Ok(n) => {
                let n = n.min(MAX_PAYLOAD);
                let mut off = 0usize;
                while off < n {
                    let w = unsafe {
                        write(
                            fds[1],
                            cbuf.as_ptr().add(off) as *const c_void,
                            n - off,
                        )
                    };
                    if w <= 0 {
                        break;
                    }
                    off += w as usize;
                }
                unsafe {
                    close(fds[1]);
                    _exit(0)
                }
            }
            Err(_) => unsafe {
                close(fds[1]);
                _exit(101)
            },
        }
    }

    // ---------------- parent ----------------
    unsafe { close(fds[1]) };
    let cap = out.len();
    let mut got = 0usize;
    while got < cap {
        let r = unsafe { read(fds[0], out.as_mut_ptr().add(got) as *mut c_void, cap - got) };
        if r <= 0 {
            break;
        }
        got += r as usize;
    }
    unsafe { close(fds[0]) };
    let mut status: c_int = 0;
    unsafe { waitpid(pid, &mut status, 0) };
    (got, status)
}

/// Fork twice from the *same* parent image and run `f(false, ..)` (C) in the
/// first child and `f(true, ..)` (Rust) in the second.  Both children therefore
/// start from an identical heap, which is what makes the allocator-address
/// dependent behaviour of `compute_hash`/`betagamma` comparable at all.
pub fn fork_pair<F>(f: F) -> (ChildResult, ChildResult)
where
    F: Fn(bool, &mut [u8]) -> usize,
{
    // Allocate *everything* up front so nothing touches the heap between forks.
    let mut buf_a = vec![0u8; MAX_PAYLOAD];
    let mut buf_b = vec![0u8; MAX_PAYLOAD];
    let mut res_a = ChildResult {
        bytes: Vec::with_capacity(MAX_PAYLOAD),
        raw_status: 0,
    };
    let mut res_b = ChildResult {
        bytes: Vec::with_capacity(MAX_PAYLOAD),
        raw_status: 0,
    };
    use std::io::Write as _;
    let _ = std::io::stdout().flush();

    let (na, sa) = run_child(&f, false, &mut buf_a);
    let (nb, sb) = run_child(&f, true, &mut buf_b);

    res_a.bytes.extend_from_slice(&buf_a[..na]);
    res_a.raw_status = sa;
    res_b.bytes.extend_from_slice(&buf_b[..nb]);
    res_b.raw_status = sb;
    (res_a, res_b)
}

/// Current total virtual size of this process, in bytes (from `/proc/self/statm`).
pub fn vm_size_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = s
        .split_whitespace()
        .next()
        .and_then(|t| t.parse().ok())
        .unwrap_or(0);
    pages * 4096
}
