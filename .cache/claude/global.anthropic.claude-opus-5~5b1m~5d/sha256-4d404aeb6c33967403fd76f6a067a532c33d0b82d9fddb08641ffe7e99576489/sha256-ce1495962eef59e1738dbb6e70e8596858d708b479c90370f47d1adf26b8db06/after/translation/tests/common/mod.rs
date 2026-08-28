//! Shared support code for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded through `libloading` and driven *only* through
//! their exported symbols, so the `#[no_mangle] extern "C"` wrappers and the
//! x86-64 SysV ABI (notably `create_block`'s 40-byte by-value return) are part
//! of what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C type layouts (must match c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct { int id; char name[32]; uint8_t flags; } DataBlock;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: u8,
}

/// ```c
/// typedef struct { int *data; size_t size; } MemoryBlock;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

impl DataBlock {
    /// `id`, `flags`, and the NUL-terminated prefix of `name`.
    ///
    /// The bytes after the NUL (and the 3 tail padding bytes) are
    /// *indeterminate* in the C — `DataBlock block;` is left uninitialised and
    /// `strcpy` writes only up to and including the terminator. Comparing them
    /// would be comparing stack garbage, so the observable projection stops at
    /// the NUL.
    pub fn observable(&self) -> (c_int, u8, Vec<u8>) {
        let bytes: &[u8] = unsafe { &*(self.name.as_ptr() as *const [u8; 32]) };
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(32);
        (self.id, self.flags, bytes[..=end.min(31)].to_vec())
    }

    /// Only the first `n` bytes of `name`, for the overflow row where the
    /// terminator may land outside the array.
    pub fn observable_prefix(&self, n: usize) -> (c_int, Vec<u8>) {
        let bytes: &[u8] = unsafe { &*(self.name.as_ptr() as *const [u8; 32]) };
        (self.id, bytes[..n.min(32)].to_vec())
    }
}

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type CreateBlockFn = unsafe extern "C" fn(c_int, *const c_char, u8) -> DataBlock;
pub type AllocateBlockFn = unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock;
pub type FreeBlockFn = unsafe extern "C" fn(*mut MemoryBlock);
pub type ComputeHashFn = unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int;
pub type BetagammaFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub create_block: CreateBlockFn,
    pub allocate_block: AllocateBlockFn,
    pub free_block: FreeBlockFn,
    pub compute_hash: ComputeHashFn,
    pub betagamma: BetagammaFn,
}

impl Impl {
    unsafe fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = Library::new(&path)
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        macro_rules! sym {
            ($t:ty, $s:literal) => {{
                let s: Symbol<$t> = lib
                    .get($s)
                    .unwrap_or_else(|e| {
                        panic!(
                            "{} missing symbol {}: {e}",
                            name,
                            String::from_utf8_lossy(&$s[..$s.len() - 1])
                        )
                    });
                *s
            }};
        }
        let create_block = sym!(CreateBlockFn, b"create_block\0");
        let allocate_block = sym!(AllocateBlockFn, b"allocate_block\0");
        let free_block = sym!(FreeBlockFn, b"free_block\0");
        let compute_hash = sym!(ComputeHashFn, b"compute_hash\0");
        let betagamma = sym!(BetagammaFn, b"betagamma\0");
        Impl {
            name,
            path,
            _lib: lib,
            create_block,
            allocate_block,
            free_block,
            compute_hash,
            betagamma,
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("c_src/build not found ({e}); build the C library first"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("lib") && s.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so in {}", build.display()))
}

fn rust_so_path() -> PathBuf {
    // Prefer the profile this test binary was built with; fall back to the other.
    let root = repo_root().join("translation/target");
    let mut candidates = Vec::new();
    if let Ok(p) = std::env::var("BETAGAMMA_RUST_SO") {
        candidates.push(PathBuf::from(p));
    }
    for prof in ["release", "debug"] {
        candidates.push(root.join(prof).join("libbetagamma_lib.so"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("libbetagamma_lib.so not found; run `cargo build` first");
}

static IMPLS: OnceLock<(Impl, Impl)> = OnceLock::new();

/// The `(c, rust)` pair, loaded once per test process.
pub fn impls() -> &'static (Impl, Impl) {
    IMPLS.get_or_init(|| unsafe {
        (
            Impl::load("C", c_so_path()),
            Impl::load("Rust", rust_so_path()),
        )
    })
}

pub fn c() -> &'static Impl {
    &impls().0
}
pub fn rs() -> &'static Impl {
    &impls().1
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed → reproducible property-style testing)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
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
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    /// An `i32` biased toward interesting values (boundaries, small, wide).
    pub fn interesting_i32(&mut self) -> i32 {
        match self.next_u64() % 10 {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => 0,
            3 => -1,
            4 => 1,
            5 => self.range(-20, 20) as i32,
            6 => i32::MIN + self.range(0, 32) as i32,
            7 => i32::MAX - self.range(0, 32) as i32,
            8 => self.range(-100_000, 100_000) as i32,
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// fork() isolation
//
// `compute_hash` compares raw allocator addresses, so `betagamma`'s return
// value depends on the process heap state, not just on its arguments. Calling
// C and then Rust in the same process makes them disagree *even when C is
// compared against itself* (glibc's tcache hands freed chunks back LIFO, so the
// second caller sees mem1/mem2 in the opposite address order).
//
// Forking twice from the identical parent state gives both libraries
// byte-identical heap layouts, which is the only way to compare this function
// meaningfully. Both children are forked from the same point, so neither side
// is privileged.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn close(fd: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, n: usize) -> isize;
    fn write(fd: i32, buf: *const u8, n: usize) -> isize;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// Outcome of running a closure in a forked child.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Outcome {
    /// Child exited normally and sent back an `i32`.
    Value(i32),
    /// Child died by signal (e.g. 11 = SIGSEGV) — the C's unchecked derefs.
    Signal(i32),
    /// Child exited without sending a value.
    ExitedSilently(i32),
}

/// Serializes all forking. A child inherits *every* open descriptor, so if two
/// test threads had result pipes open at once, each child would hold the other
/// thread's write end open and defeat its EOF detection. Holding this lock
/// guarantees at most one result pipe exists at any instant.
static FORK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` in a forked child; return what it produced.
///
/// The closure must only touch the loaded `.so` — it runs after `fork()` in a
/// process that shares the parent's address space image. glibc's `fork()` takes
/// the malloc locks around the clone, so allocating in the child is safe.
pub fn fork_call<F: FnOnce() -> i32>(f: F) -> Outcome {
    // Both `.so`s MUST be dlopen'd before the fork. A child that dlopen'd
    // lazily would grab the loader lock, which another test thread may hold —
    // an instant deadlock in the child.
    let _ = impls();
    let _guard = FORK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut fds = [0i32; 2];
    unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // Child. Must never return into the test harness, and must never
            // unwind past this point (a panicking child would otherwise start
            // running the remaining tests).
            close(fds[0]);
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            match r {
                Ok(v) => {
                    let buf = v.to_ne_bytes();
                    write(fds[1], buf.as_ptr(), 4);
                    close(fds[1]);
                    _exit(0);
                }
                Err(_) => {
                    close(fds[1]);
                    _exit(99);
                }
            }
        }
        // Parent.
        close(fds[1]);
        let mut buf = [0u8; 4];
        let mut got = 0usize;
        while got < 4 {
            let n = read(fds[0], buf.as_mut_ptr().add(got), 4 - got);
            if n <= 0 {
                break;
            }
            got += n as usize;
        }
        close(fds[0]);
        let mut status = 0i32;
        waitpid(pid, &mut status, 0);

        let signalled = (status & 0x7f) != 0 && (status & 0x7f) != 0x7f;
        if signalled {
            return Outcome::Signal(status & 0x7f);
        }
        let code = (status >> 8) & 0xff;
        if got == 4 {
            Outcome::Value(i32::from_ne_bytes(buf))
        } else {
            Outcome::ExitedSilently(code)
        }
    }
}

/// Run the same logical operation against both libraries under fork isolation
/// and return `(c_outcome, rust_outcome)`.
pub fn fork_both<F>(mut op: F) -> (Outcome, Outcome)
where
    F: FnMut(&'static Impl) -> i32,
{
    // Make sure both libraries are already dlopen'd (and thus have identical
    // relocation/heap footprints) *before* either fork.
    let _ = impls();
    let a = fork_call(|| op(c()));
    let b = fork_call(|| op(rs()));
    (a, b)
}

/// `strcpy`-able NUL-terminated buffer from a byte slice.
pub fn cstr(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}
