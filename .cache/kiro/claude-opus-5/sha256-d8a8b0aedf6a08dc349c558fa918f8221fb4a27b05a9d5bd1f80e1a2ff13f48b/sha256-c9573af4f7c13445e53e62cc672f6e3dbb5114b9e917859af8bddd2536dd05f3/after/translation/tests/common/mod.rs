//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function
//! across the FFI boundary. The Rust implementation is NEVER called directly —
//! only via the symbols its `cdylib` exports, so the `#[no_mangle]` wrappers
//! are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::ffi::c_int;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// FFI type mirrors (must match c_src/src/lib.c byte-for-byte)
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct { int id; char name[32]; uint8_t flags; } DataBlock;
/// ```
/// 4 + 32 + 1 = 37 bytes of payload, size 40, align 4 => bytes 37..40 are
/// PADDING and are uninitialized in the C implementation (`DataBlock block;`).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: u8,
}

/// ```c
/// typedef struct { int *data; size_t size; } MemoryBlock;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

pub type CreateBlockFn = unsafe extern "C" fn(c_int, *const c_char, u8) -> DataBlock;
pub type AllocateBlockFn = unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock;
pub type FreeBlockFn = unsafe extern "C" fn(*mut MemoryBlock);
pub type ComputeHashFn = unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int;
pub type BetagammaFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Library location
// ---------------------------------------------------------------------------

/// Workspace root = the directory that contains both `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// `target/<profile>/` for the profile the test binary itself was built with,
/// so `cargo test` and `cargo test --release` both find the matching cdylib.
fn rust_target_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    exe.parent()
        .and_then(Path::parent)
        .expect("test binary should live in target/<profile>/deps")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    let dir = rust_target_dir();
    let p = dir.join("libbetagamma_lib.so");
    let so = if p.exists() {
        p
    } else {
        // Fall back to the release build if the tests were run in a different profile.
        let alt = workspace_root()
            .join("translation")
            .join("target")
            .join("release")
            .join("libbetagamma_lib.so");
        if !alt.exists() {
            panic!(
                "Rust cdylib not found at {} or {}",
                p.display(),
                alt.display()
            );
        }
        alt
    };
    assert_fresh(&so);
    so
}

/// Refuse to run against a stale `.so`.
///
/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact, because
/// integration tests never link against it — they `dlopen` it. So without this
/// guard the suite silently tests whatever `.so` happens to be sitting in
/// `target/`, and every change to `src/lib.rs` (including a real regression)
/// appears to pass. Build with `cargo build --release` first; the
/// `scripts/verify_all.sh` driver does this.
fn assert_fresh(so: &Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));

    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src_dir];
    while let Some(d) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&d) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                        if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                            newest = Some((p, t));
                        }
                    }
                }
            }
        }
    }

    if let Some((path, t)) = newest {
        assert!(
            so_mtime >= t,
            "STALE cdylib: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib — run `cargo build --release` \
             (or scripts/verify_all.sh) first, otherwise this suite would silently \
             verify an out-of-date library.",
            so.display(),
            path.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Loaded pair
// ---------------------------------------------------------------------------

pub struct Impl {
    pub tag: &'static str,
    _lib: Library,
    pub create_block: CreateBlockFn,
    pub allocate_block: AllocateBlockFn,
    pub free_block: FreeBlockFn,
    pub compute_hash: ComputeHashFn,
    pub betagamma: BetagammaFn,
}

impl Impl {
    unsafe fn load(tag: &'static str, path: &Path) -> Impl {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));

        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                let s: Symbol<$ty> = lib.get(concat!($name, "\0").as_bytes()).unwrap_or_else(|e| {
                    panic!("{} missing symbol `{}`: {e}", path.display(), $name)
                });
                *s
            }};
        }

        let create_block = sym!("create_block", CreateBlockFn);
        let allocate_block = sym!("allocate_block", AllocateBlockFn);
        let free_block = sym!("free_block", FreeBlockFn);
        let compute_hash = sym!("compute_hash", ComputeHashFn);
        let betagamma = sym!("betagamma", BetagammaFn);

        Impl {
            tag,
            _lib: lib,
            create_block,
            allocate_block,
            free_block,
            compute_hash,
            betagamma,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

/// Load both shared objects. Each test target gets its own `Pair`.
pub fn load_pair() -> Pair {
    unsafe {
        let c = Impl::load("C", &find_c_so());
        let rs = Impl::load("Rust", &find_rust_so());
        Pair { c, rs }
    }
}

// ---------------------------------------------------------------------------
// DataBlock comparison that respects C's uninitialized bytes
// ---------------------------------------------------------------------------

/// The C `create_block` leaves (a) the 3 trailing padding bytes and (b) every
/// `name[]` byte past the copied NUL terminator uninitialized. Only the bytes
/// the C code actually writes are defined, so those are the bytes we compare.
#[derive(Debug, PartialEq, Eq)]
pub struct DefinedBlock {
    pub id: c_int,
    /// name bytes up to and including the first NUL
    pub name: Vec<u8>,
    pub flags: u8,
}

pub fn defined(b: &DataBlock) -> DefinedBlock {
    let bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(b.name.as_ptr() as *const u8, b.name.len()) };
    let nul = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
    DefinedBlock {
        id: b.id,
        name: bytes[..=nul.min(bytes.len() - 1)].to_vec(),
        flags: b.flags,
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed, no external crates)
// ---------------------------------------------------------------------------

/// SplitMix64 — fixed seed, fully reproducible across runs and machines.
pub struct Rng(pub u64);

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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// An `i32` biased toward interesting values (extremes, small magnitudes,
    /// and multiples/near-multiples of 10 which drive `param1 % 10`).
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(8) {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => 0,
            3 => (self.below(41) as i64 - 20) as i32,
            4 => (self.below(2001) as i64 - 1000) as i32,
            5 => i32::MIN + (self.below(20) as i32),
            6 => i32::MAX - (self.below(20) as i32),
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Heap-state-equalised differential execution
// ---------------------------------------------------------------------------
//
// `compute_hash` (and therefore `betagamma`) branches on RAW HEAP ADDRESSES:
//
//     if (mb1->data < mb2->data) hash += 100; else if (>) hash += 200;
//     if (mb1        < mb2)      hash +=  10; else if (>) hash +=  20;
//
// So its return value is a function of (inputs, global heap state). Calling C
// and then Rust in one process compares them under two DIFFERENT heap states,
// which is not a meaningful differential test -- the first call perturbs the
// allocator for the second.
//
// `fork()` solves this exactly: two children forked from the same instruction
// inherit byte-identical heaps (same arena bases, same free lists, same top
// chunk). Child 1 runs the whole C batch, child 2 the whole Rust batch. Because
// the two implementations issue the SAME malloc/calloc/free sequence with the
// SAME sizes, their heaps evolve in lockstep for the entire batch -- so any
// divergence is a genuine translation defect, and "the Rust performs an
// identical allocation sequence" becomes part of what is verified.

extern "C" {
    fn fork() -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn read(fd: i32, buf: *mut u8, n: usize) -> isize;
    fn write(fd: i32, buf: *const u8, n: usize) -> isize;
    fn close(fd: i32) -> i32;
    fn _exit(code: i32) -> !;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
}

fn read_exact_fd(fd: i32, buf: &mut [u8]) -> bool {
    let mut off = 0usize;
    while off < buf.len() {
        let n = unsafe { read(fd, buf.as_mut_ptr().add(off), buf.len() - off) };
        if n <= 0 {
            return false;
        }
        off += n as usize;
    }
    true
}

/// Run `run` against the C impl and the Rust impl under *identical* heap state,
/// returning `(c_bytes, rust_bytes)` of length `out_len` each.
///
/// `run` MUST write exactly `out_len` bytes into the slice it is given and MUST
/// NOT allocate (the buffer is allocated in the parent before forking), so the
/// only heap traffic in the children is the library's own.
pub fn dual_batch<F>(pair: &Pair, out_len: usize, run: F) -> (Vec<u8>, Vec<u8>)
where
    F: Fn(&Impl, &mut [u8]),
{
    // Everything that allocates happens BEFORE the forks.
    let mut scratch = vec![0u8; out_len];
    let mut c_out = vec![0u8; out_len];
    let mut rs_out = vec![0u8; out_len];

    let mut p1 = [0i32; 2];
    let mut p2 = [0i32; 2];
    unsafe {
        assert_eq!(pipe(p1.as_mut_ptr()), 0, "pipe() failed");
        assert_eq!(pipe(p2.as_mut_ptr()), 0, "pipe() failed");
    }

    // --- fork both children back to back, allocating nothing in between, so
    // --- they observe the exact same heap image.
    let pid_c = unsafe { fork() };
    if pid_c == 0 {
        unsafe {
            close(p1[0]);
            close(p2[0]);
            close(p2[1]);
            run(&pair.c, &mut scratch);
            let mut off = 0usize;
            while off < out_len {
                let n = write(p1[1], scratch.as_ptr().add(off), out_len - off);
                if n <= 0 {
                    _exit(3);
                }
                off += n as usize;
            }
            close(p1[1]);
            _exit(0);
        }
    }
    let pid_rs = unsafe { fork() };
    if pid_rs == 0 {
        unsafe {
            close(p1[0]);
            close(p1[1]);
            close(p2[0]);
            run(&pair.rs, &mut scratch);
            let mut off = 0usize;
            while off < out_len {
                let n = write(p2[1], scratch.as_ptr().add(off), out_len - off);
                if n <= 0 {
                    _exit(3);
                }
                off += n as usize;
            }
            close(p2[1]);
            _exit(0);
        }
    }

    assert!(pid_c > 0 && pid_rs > 0, "fork() failed");

    unsafe {
        close(p1[1]);
        close(p2[1]);
    }

    let ok_c = read_exact_fd(p1[0], &mut c_out);
    let ok_rs = read_exact_fd(p2[0], &mut rs_out);

    let mut st_c = 0i32;
    let mut st_rs = 0i32;
    unsafe {
        close(p1[0]);
        close(p2[0]);
        waitpid(pid_c, &mut st_c, 0);
        waitpid(pid_rs, &mut st_rs, 0);
    }

    assert!(
        ok_c,
        "C child produced short output (exited abnormally, raw status {st_c:#x})"
    );
    assert!(
        ok_rs,
        "Rust child produced short output (exited abnormally, raw status {st_rs:#x})"
    );
    assert_eq!(st_c, 0, "C child exited with raw status {st_c:#x}");
    assert_eq!(st_rs, 0, "Rust child exited with raw status {st_rs:#x}");

    (c_out, rs_out)
}

/// `dual_batch` specialised to a batch of `i32` results -- the shape every
/// address-sensitive entry point (`betagamma`, `compute_hash`) returns.
pub fn dual_i32_batch<F>(pair: &Pair, n: usize, run: F) -> (Vec<i32>, Vec<i32>)
where
    F: Fn(&Impl, &mut [i32]),
{
    let (a, b) = dual_batch(pair, n * 4, |imp, raw| {
        // Reinterpret the pre-allocated byte buffer as i32s; no allocation.
        let s = unsafe { std::slice::from_raw_parts_mut(raw.as_mut_ptr() as *mut i32, n) };
        run(imp, s);
    });
    let to_vec = |v: Vec<u8>| -> Vec<i32> {
        v.chunks_exact(4)
            .map(|c| i32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    };
    (to_vec(a), to_vec(b))
}

/// Assert two i32 batches match, reporting the first few divergences with the
/// case label produced by `label`.
pub fn assert_i32_batches_eq<L: Fn(usize) -> String>(
    what: &str,
    c: &[i32],
    rs: &[i32],
    label: L,
) {
    assert_eq!(c.len(), rs.len(), "{what}: batch length mismatch");
    let mut bad = Vec::new();
    for i in 0..c.len() {
        if c[i] != rs[i] && bad.len() < 12 {
            bad.push(format!("  [{i}] {} -> C={} Rust={}", label(i), c[i], rs[i]));
        }
    }
    if !bad.is_empty() {
        let total = (0..c.len()).filter(|&i| c[i] != rs[i]).count();
        panic!(
            "{what}: {total}/{} cases diverged (heap state was equalised via fork):\n{}",
            c.len(),
            bad.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// Crash-equivalence helper (for the C's unguarded dereferences)
// ---------------------------------------------------------------------------

/// Raw `waitpid` status of a child that just runs `f`.
///
/// Some C entry points dereference their arguments with NO null check, so
/// passing `NULL` is a hard fault rather than a rejection. The Rust must fault
/// the SAME way instead of "improving" the C by adding a check, and the only
/// way to observe that differentially is to run each in its own process and
/// compare how it died.
pub fn child_status<F: FnOnce()>(f: F) -> i32 {
    let pid = unsafe { fork() };
    if pid == 0 {
        f();
        unsafe { _exit(0) }
    }
    assert!(pid > 0, "fork() failed");
    let mut st = 0i32;
    unsafe { waitpid(pid, &mut st, 0) };
    st
}

/// Decode a raw wait status into a comparable, printable outcome.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32),
    Signalled(i32),
}

pub fn outcome(raw: i32) -> Outcome {
    if raw & 0x7f == 0x7f {
        Outcome::Exited((raw >> 8) & 0xff) // stopped; treat as exit
    } else if raw & 0x7f == 0 {
        Outcome::Exited((raw >> 8) & 0xff)
    } else {
        Outcome::Signalled(raw & 0x7f)
    }
}
