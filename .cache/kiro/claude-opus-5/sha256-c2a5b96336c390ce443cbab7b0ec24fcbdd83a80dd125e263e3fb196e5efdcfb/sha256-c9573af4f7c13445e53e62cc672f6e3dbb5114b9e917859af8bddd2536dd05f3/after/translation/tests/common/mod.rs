//! Shared differential-testing harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! only through their exported `bin2hex` symbol. The Rust implementation is
//! never called directly, so the `#[no_mangle] extern "C"` wrapper is part of
//! what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};

/// `char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len)`
pub type Bin2HexFn =
    unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;

/// Bytes of untouched padding placed before and after every buffer so that
/// out-of-bounds writes are caught by the byte-for-byte comparison.
pub const GUARD: usize = 32;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    // Field order matters: `bin2hex` must be dropped before `lib` is unloaded.
    pub bin2hex: Bin2HexFn,
    _lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let sym: Symbol<Bin2HexFn> = unsafe { lib.get(b"bin2hex\0") }.unwrap_or_else(|e| {
            panic!("symbol `bin2hex` not exported by {}: {e}", path.display())
        });
        let bin2hex = *sym;
        Impl {
            name,
            path: path.to_path_buf(),
            bin2hex,
            _lib: lib,
        }
    }
}

/// The directory holding both `c_src/` and `translation/`.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate root has a parent")
        .to_path_buf()
}

/// The C shared library produced by `c_src/CMakeLists.txt`. Its name is derived
/// from the working-directory name by CMake, so it is discovered by globbing.
pub fn c_so_path() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust `cdylib`. Prefers the release artifact (which carries
/// `panic = "abort"`, matching the C `abort()` semantics) and falls back to the
/// debug artifact.
pub fn rust_so_path() -> PathBuf {
    let base = workspace_root().join("translation").join("target");
    let candidates = [
        base.join("release").join("libbin2hex_lib.so"),
        base.join("debug").join("libbin2hex_lib.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found; tried {:?}. Run `cargo build --release` first.",
        candidates
    );
}

pub fn c_impl() -> Impl {
    Impl::open("C", &c_so_path())
}

pub fn rust_impl() -> Impl {
    Impl::open("Rust", &rust_so_path())
}

pub fn both() -> (Impl, Impl) {
    (c_impl(), rust_impl())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every row reproducible.
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }
    /// Uniform-ish value in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
    pub fn bytes_from(&mut self, n: usize, pool: &[u8]) -> Vec<u8> {
        (0..n)
            .map(|_| pool[(self.next_u64() % pool.len() as u64) as usize])
            .collect()
    }
}

// ---------------------------------------------------------------------------
// One differential call
// ---------------------------------------------------------------------------

/// A single invocation of `bin2hex`, described independently of the
/// implementation under test.
#[derive(Clone, Debug)]
pub struct Case<'a> {
    /// `None` passes a NULL `bin` pointer.
    pub bin: Option<&'a [u8]>,
    /// `bin_len` argument. Normally `bin.unwrap().len()`.
    pub bin_len: usize,
    /// `hex_maxlen` argument, exactly as the C function receives it. May be far
    /// larger than what is actually allocated: the C code never touches bytes
    /// past `bin_len * 2`.
    pub hex_maxlen: usize,
    /// Usable bytes actually allocated for the output (must be
    /// `>= bin_len * 2 + 1` for accepted calls).
    pub hex_alloc: usize,
    /// Byte offset of `hex` inside its allocation (exercises misalignment).
    pub hex_off: usize,
    /// Byte offset of `bin` inside its allocation.
    pub bin_off: usize,
    /// Pre-fill pattern for the output allocation, so untouched bytes are
    /// distinguishable from written ones.
    pub fill: u8,
}

impl<'a> Case<'a> {
    /// Accepted call with the minimum legal `hex_maxlen` (`bin_len * 2 + 1`).
    pub fn exact(bin: &'a [u8]) -> Case<'a> {
        let need = bin.len() * 2 + 1;
        Case {
            bin: Some(bin),
            bin_len: bin.len(),
            hex_maxlen: need,
            hex_alloc: need,
            hex_off: 0,
            bin_off: 0,
            fill: 0xAA,
        }
    }
    /// Accepted call with slack: `hex_maxlen` larger than strictly required.
    pub fn slack(bin: &'a [u8], hex_maxlen: usize) -> Case<'a> {
        assert!(hex_maxlen > bin.len() * 2);
        Case {
            bin: Some(bin),
            bin_len: bin.len(),
            hex_maxlen,
            hex_alloc: hex_maxlen,
            hex_off: 0,
            bin_off: 0,
            fill: 0xAA,
        }
    }
}

struct Run {
    /// Whole allocation, including guard padding on both sides.
    hex_alloc_bytes: Vec<u8>,
    bin_alloc_bytes: Vec<u8>,
    /// `true` when the function returned exactly the `hex` pointer it was given.
    returned_hex_ptr: bool,
    ret_offset_from_hex: isize,
}

fn run_one(imp: &Impl, case: &Case<'_>) -> Run {
    let hex_total = case.hex_off + case.hex_alloc + GUARD;
    let mut hex_buf = vec![case.fill; hex_total];

    let (mut bin_buf, bin_ptr) = match case.bin {
        None => (Vec::new(), std::ptr::null::<u8>()),
        Some(b) => {
            let total = case.bin_off + b.len() + GUARD;
            let mut v = vec![0x5Au8; total];
            v[case.bin_off..case.bin_off + b.len()].copy_from_slice(b);
            let p = unsafe { v.as_ptr().add(case.bin_off) };
            (v, p)
        }
    };
    // Keep `bin_buf` alive for the duration of the call.
    let _ = &mut bin_buf;

    let hex_ptr = unsafe { hex_buf.as_mut_ptr().add(case.hex_off) };
    let ret = unsafe {
        (imp.bin2hex)(
            hex_ptr.cast::<c_char>(),
            case.hex_maxlen,
            bin_ptr,
            case.bin_len,
        )
    };
    let ret_u8 = ret.cast::<u8>();

    Run {
        hex_alloc_bytes: hex_buf,
        bin_alloc_bytes: bin_buf,
        returned_hex_ptr: ret_u8 == hex_ptr,
        ret_offset_from_hex: (ret_u8 as isize) - (hex_ptr as isize),
    }
}

/// Runs `case` against both implementations and asserts byte-for-byte identical
/// output buffers, identical (non-)mutation of the input buffer, and identical
/// return-pointer semantics.
#[track_caller]
pub fn assert_identical(c: &Impl, r: &Impl, case: &Case<'_>, label: &str) {
    if case.bin.is_some() || case.bin_len == 0 {
        // ok
    }
    assert!(
        case.hex_alloc >= case.bin_len * 2 + 1,
        "{label}: test bug — hex_alloc {} too small for bin_len {}",
        case.hex_alloc,
        case.bin_len
    );

    let rc = run_one(c, case);
    let rr = run_one(r, case);

    assert!(
        rc.returned_hex_ptr,
        "{label}: C did not return the hex pointer (offset {})",
        rc.ret_offset_from_hex
    );
    assert!(
        rr.returned_hex_ptr,
        "{label}: Rust did not return the hex pointer (offset {})",
        rr.ret_offset_from_hex
    );

    if rc.hex_alloc_bytes != rr.hex_alloc_bytes {
        let idx = rc
            .hex_alloc_bytes
            .iter()
            .zip(&rr.hex_alloc_bytes)
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "{label}: output differs at allocation byte {idx} (hex_off={}, bin_len={}, \
             hex_maxlen={})\n  C   : {:02x?}\n  Rust: {:02x?}",
            case.hex_off,
            case.bin_len,
            case.hex_maxlen,
            &rc.hex_alloc_bytes[idx.saturating_sub(8)..(idx + 8).min(rc.hex_alloc_bytes.len())],
            &rr.hex_alloc_bytes[idx.saturating_sub(8)..(idx + 8).min(rr.hex_alloc_bytes.len())],
        );
    }
    assert_eq!(
        rc.bin_alloc_bytes, rr.bin_alloc_bytes,
        "{label}: input buffer was mutated differently"
    );
    // Neither implementation may modify the input at all.
    if let Some(b) = case.bin {
        assert_eq!(
            &rc.bin_alloc_bytes[case.bin_off..case.bin_off + b.len()],
            b,
            "{label}: C mutated the input buffer"
        );
        assert_eq!(
            &rr.bin_alloc_bytes[case.bin_off..case.bin_off + b.len()],
            b,
            "{label}: Rust mutated the input buffer"
        );
    }
}

// ---------------------------------------------------------------------------
// Child-process execution, for the `abort()` paths
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Runs `f` in a forked child and reports how the child terminated. Needed
/// because the only failure mode of `bin2hex` is `abort()`, which kills the
/// process.
pub fn in_child<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // Suppress core dumps from the expected SIGABRTs.
            let rl = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            f();
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid() failed");
        if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else if libc::WIFEXITED(status) {
            Outcome::Exited(libc::WEXITSTATUS(status))
        } else {
            panic!("child neither exited nor signalled: raw status {status}");
        }
    }
}

/// Calls `bin2hex` with raw arguments inside a child process and returns the
/// child's termination outcome.
pub fn outcome_of(imp: &Impl, hex: *mut c_char, hex_maxlen: usize, bin: *const u8, bin_len: usize) -> Outcome {
    let hex = hex as usize;
    let bin = bin as usize;
    in_child(|| unsafe {
        let p = (imp.bin2hex)(hex as *mut c_char, hex_maxlen, bin as *const u8, bin_len);
        // Consume the result so the call cannot be optimised away.
        std::hint::black_box(p);
    })
}

/// An anonymous mapping of `usable` writable bytes followed immediately by a
/// `PROT_NONE` guard page, so that running off the end of the region raises
/// `SIGSEGV` **deterministically** at a known offset.
///
/// This is how the "guard accepted an astronomically large `bin_len`" cases are
/// observed: the pointers handed to `bin2hex` are genuine, non-null and aligned,
/// so both implementations perform the same sequence of reads/writes and die the
/// same way when they reach the guard page. Using NULL pointers instead would
/// compare two *undefined* behaviours (a C null dereference against Rust's
/// debug-build null-pointer UB check), which is not a meaningful comparison.
pub struct GuardedRegion {
    base: *mut u8,
    total: usize,
    pub usable: usize,
}

impl GuardedRegion {
    pub fn new(usable_pages: usize) -> GuardedRegion {
        assert!(usable_pages >= 1);
        let ps = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let total = (usable_pages + 1) * ps;
        let p = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                total,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(p != libc::MAP_FAILED, "mmap failed");
        let base = p.cast::<u8>();
        let usable = usable_pages * ps;
        let rc = unsafe { libc::mprotect(base.add(usable).cast(), ps, libc::PROT_NONE) };
        assert_eq!(rc, 0, "mprotect(PROT_NONE) failed");
        GuardedRegion {
            base,
            total,
            usable,
        }
    }
    pub fn ptr(&self) -> *mut u8 {
        self.base
    }
    pub fn fill(&self, byte: u8) {
        unsafe { std::ptr::write_bytes(self.base, byte, self.usable) };
    }
}

impl Drop for GuardedRegion {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base.cast(), self.total);
        }
    }
}

/// Asserts that both implementations terminate the same way for the same
/// (invalid) arguments, and that the way is exactly `SIGABRT`.
#[track_caller]
pub fn assert_both_abort(
    c: &Impl,
    r: &Impl,
    hex_maxlen: usize,
    bin_len: usize,
    with_real_buffers: bool,
    label: &str,
) {
    // When the guard is expected to fire, the pointers are never dereferenced;
    // still offer the option of passing genuine allocations.
    let mut hex_store: Vec<u8>;
    let bin_store: Vec<u8>;
    let (hexp, binp) = if with_real_buffers {
        hex_store = vec![0xAAu8; hex_maxlen.min(1 << 16) + 64];
        bin_store = vec![0x5Au8; bin_len.min(1 << 16) + 64];
        (
            hex_store.as_mut_ptr().cast::<c_char>(),
            bin_store.as_ptr() as *const u8,
        )
    } else {
        (std::ptr::null_mut::<c_char>(), std::ptr::null::<u8>())
    };

    let oc = outcome_of(c, hexp, hex_maxlen, binp, bin_len);
    let or = outcome_of(r, hexp, hex_maxlen, binp, bin_len);

    assert_eq!(
        oc,
        Outcome::Signaled(libc::SIGABRT),
        "{label}: C did not abort (hex_maxlen={hex_maxlen}, bin_len={bin_len})"
    );
    assert_eq!(
        or,
        Outcome::Signaled(libc::SIGABRT),
        "{label}: Rust did not abort identically (hex_maxlen={hex_maxlen}, bin_len={bin_len})"
    );
    assert_eq!(oc, or, "{label}: termination outcomes differ");
}

/// Asserts that both implementations *accept* the given arguments (child exits
/// 0), i.e. the guard does not fire.
#[track_caller]
pub fn assert_both_accept(c: &Impl, r: &Impl, hex_maxlen: usize, bin_len: usize, label: &str) {
    let alloc = bin_len * 2 + 1 + 64;
    let mut hex_c = vec![0xAAu8; alloc];
    let mut hex_r = vec![0xAAu8; alloc];
    let bin = vec![0x5Au8; bin_len + 1];
    let oc = outcome_of(
        c,
        hex_c.as_mut_ptr().cast::<c_char>(),
        hex_maxlen,
        bin.as_ptr(),
        bin_len,
    );
    let or = outcome_of(
        r,
        hex_r.as_mut_ptr().cast::<c_char>(),
        hex_maxlen,
        bin.as_ptr(),
        bin_len,
    );
    assert_eq!(
        oc,
        Outcome::Exited(0),
        "{label}: C rejected an input it should accept (hex_maxlen={hex_maxlen}, bin_len={bin_len})"
    );
    assert_eq!(
        or,
        Outcome::Exited(0),
        "{label}: Rust rejected an input it should accept (hex_maxlen={hex_maxlen}, bin_len={bin_len})"
    );
}
