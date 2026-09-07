//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading`:
//!   * the C one  : `c_src/build/libdriver.so`
//!   * the Rust one: `target/<profile>/libdriver.so`
//!
//! Rust functions are NEVER called directly — always through the `.so`'s
//! exported symbols, exactly like an external C consumer, so the
//! `#[no_mangle] extern "C"` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type ExtractFn = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;
pub type ExtractIntFn = unsafe extern "C" fn(*const c_char, std::ffi::c_int) -> *const c_char;
pub type CreateFn = unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;

unsafe extern "C" {
    pub fn free(p: *mut std::ffi::c_void);
    pub fn fork() -> i32;
    pub fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    pub fn _exit(code: i32) -> !;
}

/// One loaded implementation.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub extract: ExtractFn,
    pub extract_int: ExtractIntFn,
    pub create: CreateFn,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {} ({}): {e}", path.display(), name));
        let extract: ExtractFn = unsafe {
            let s: Symbol<ExtractFn> = lib
                .get(b"extractFilename\0")
                .expect("missing symbol extractFilename");
            *s
        };
        let extract_int: ExtractIntFn = unsafe {
            let s: Symbol<ExtractIntFn> = lib
                .get(b"extractFilename\0")
                .expect("missing symbol extractFilename");
            *s
        };
        let create: CreateFn = unsafe {
            let s: Symbol<CreateFn> = lib
                .get(b"FIO_createFilename_fromOutDir\0")
                .expect("missing symbol FIO_createFilename_fromOutDir");
            *s
        };
        Impl {
            name,
            _lib: lib,
            extract,
            extract_int,
            create,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe = target/<profile>/deps/<testbin>  ->  target/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let target_profile_dir = exe
        .parent()
        .and_then(|d| d.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let candidate = target_profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    // fall back to the release artifact
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c = c_so_path();
        let r = rust_so_path();
        assert!(
            c.exists(),
            "C shared object not built: {}\nrun: cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c.display()
        );
        assert!(r.exists(), "Rust shared object not built: {}", r.display());

        // `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact,
        // so without this guard the suite would silently test a stale .so and
        // report false PASSes. Refuse to run if the .so predates the sources.
        let so_time = std::fs::metadata(&r)
            .and_then(|m| m.modified())
            .expect("mtime of the Rust .so");
        for src in ["src/lib.rs", "Cargo.toml"] {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(src);
            if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
                assert!(
                    t <= so_time,
                    "STALE ARTIFACT: {} is newer than {}.\n\
                     `cargo test` does not rebuild a cdylib — run `cargo build [--release]` \
                     first (or use ./run_verification.sh).",
                    p.display(),
                    r.display()
                );
            }
        }
        // Same guard for the C side.
        let c_time = std::fs::metadata(&c)
            .and_then(|m| m.modified())
            .expect("mtime of the C .so");
        for src in ["c_src/src/lib.c", "c_src/include/lib.h"] {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(src);
            if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
                assert!(
                    t <= c_time,
                    "STALE ARTIFACT: {} is newer than {}; rebuild the C library.",
                    p.display(),
                    r.display()
                );
            }
        }

        Pair {
            c: Impl::load("C", &c),
            rs: Impl::load("Rust", &r),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_5678_9ABC;

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }
    pub fn seeded(s: u64) -> Rng {
        Rng(if s == 0 { 1 } else { s })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    /// A random byte that is never 0 (so it can live inside a C string).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
    /// A random printable ASCII byte, never 0 and never `excl`.
    pub fn ascii_except(&mut self, excl: u8) -> u8 {
        loop {
            let b = self.range(0x21, 0x7e) as u8;
            if b != excl {
                return b;
            }
        }
    }
    pub fn byte_except(&mut self, excl: u8) -> u8 {
        loop {
            let b = self.nonzero_byte();
            if b != excl {
                return b;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// NUL-terminated byte buffer helper (holds ownership of the bytes).
// ---------------------------------------------------------------------------
pub struct CBuf(Vec<u8>);

impl CBuf {
    /// `bytes` must not contain an interior NUL.
    pub fn new(bytes: &[u8]) -> CBuf {
        assert!(!bytes.contains(&0), "interior NUL in test input");
        let mut v = Vec::with_capacity(bytes.len() + 1);
        v.extend_from_slice(bytes);
        v.push(0);
        CBuf(v)
    }
    pub fn ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
    pub fn bytes(&self) -> &[u8] {
        &self.0[..self.0.len() - 1]
    }
    pub fn len(&self) -> usize {
        self.0.len() - 1
    }
}

// ---------------------------------------------------------------------------
// Independent reference computations (used only to size the comparison window,
// never to decide correctness of the values themselves).
// ---------------------------------------------------------------------------

/// Byte offset, relative to the start of `path`, of the filename part —
/// computed straight from the input bytes, independently of either `.so`.
/// Mirrors `strrchr` semantics including the `separator == 0` case where the
/// terminating NUL matches.
pub fn ref_filename_offset(path: &[u8], separator: u8) -> usize {
    if separator == 0 {
        // strrchr(path, 0) -> &path[len]; extractFilename returns that + 1
        return path.len() + 1;
    }
    match path.iter().rposition(|&b| b == separator) {
        Some(i) => i + 1,
        None => 0,
    }
}

/// `strlen(filenameStart)` for the POSIX separator.
pub fn ref_filename_len(path: &[u8], separator: u8) -> usize {
    let off = ref_filename_offset(path, separator);
    path.len() - off
}

/// `calloc` size used by `FIO_createFilename_fromOutDir`, with C `size_t`
/// wrap-around semantics.
pub fn ref_alloc_size(out_dir_len: usize, filename_len: usize, suffix_len: usize) -> usize {
    out_dir_len
        .wrapping_add(1)
        .wrapping_add(filename_len)
        .wrapping_add(suffix_len)
        .wrapping_add(1)
}

pub const SEP: u8 = b'/';

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Run `extractFilename` on both `.so`s and assert the returned pointer has
/// the identical offset into `path`.
pub fn diff_extract(path: &[u8], separator: u8, ctx: &str) {
    let p = pair();
    let buf_c = CBuf::new(path);
    let buf_r = CBuf::new(path);
    let sep = separator as i8 as c_char;
    let rc = unsafe { (p.c.extract)(buf_c.ptr(), sep) };
    let rr = unsafe { (p.rs.extract)(buf_r.ptr(), sep) };
    assert!(!rc.is_null(), "{ctx}: C returned NULL (impossible)");
    assert!(
        !rr.is_null(),
        "{ctx}: Rust returned NULL but C returned a pointer"
    );
    let off_c = rc as usize - buf_c.ptr() as usize;
    let off_r = rr as usize - buf_r.ptr() as usize;
    assert_eq!(
        off_c, off_r,
        "{ctx}: extractFilename offset mismatch (C={off_c}, Rust={off_r}) \
         path={:?} sep={:#04x}",
        String::from_utf8_lossy(path),
        separator
    );
    let expect = ref_filename_offset(path, separator);
    assert_eq!(
        off_c, expect,
        "{ctx}: C offset {off_c} disagrees with the independent reference {expect}"
    );
}

/// Same but passing the separator as a full `int` (ABI-level over-wide value).
pub fn diff_extract_int(path: &[u8], separator: std::ffi::c_int, ctx: &str) {
    let p = pair();
    let buf_c = CBuf::new(path);
    let buf_r = CBuf::new(path);
    let rc = unsafe { (p.c.extract_int)(buf_c.ptr(), separator) };
    let rr = unsafe { (p.rs.extract_int)(buf_r.ptr(), separator) };
    let off_c = rc as usize - buf_c.ptr() as usize;
    let off_r = rr as usize - buf_r.ptr() as usize;
    assert_eq!(
        off_c, off_r,
        "{ctx}: extractFilename(int) offset mismatch (C={off_c}, Rust={off_r}) sep={separator:#x}"
    );
}

/// Run `FIO_createFilename_fromOutDir` on both `.so`s and compare the FULL
/// `calloc`ed buffer byte-for-byte (including the zero padding that `calloc`
/// provides and the `suffixLen` reservation).
pub fn diff_create(path: &[u8], out_dir: &[u8], suffix_len: usize, ctx: &str) -> Vec<u8> {
    let p = pair();
    let bp = CBuf::new(path);
    let bd = CBuf::new(out_dir);
    let n = ref_alloc_size(out_dir.len(), ref_filename_len(path, SEP), suffix_len);
    let out_c = unsafe { (p.c.create)(bp.ptr(), bd.ptr(), suffix_len) };
    let out_r = unsafe { (p.rs.create)(bp.ptr(), bd.ptr(), suffix_len) };
    assert!(!out_c.is_null(), "{ctx}: C returned NULL");
    assert!(!out_r.is_null(), "{ctx}: Rust returned NULL, C did not");
    let sc = unsafe { std::slice::from_raw_parts(out_c as *const u8, n) }.to_vec();
    let sr = unsafe { std::slice::from_raw_parts(out_r as *const u8, n) }.to_vec();
    unsafe {
        free(out_c as *mut _);
        free(out_r as *mut _);
    }
    if sc != sr {
        let first = sc.iter().zip(sr.iter()).position(|(a, b)| a != b);
        panic!(
            "{ctx}: FIO_createFilename_fromOutDir buffer mismatch at byte {first:?}\n\
             path   = {:?}\n outDir = {:?}\n suffixLen = {suffix_len}, size = {n}\n\
             C    = {:?}\n Rust = {:?}",
            String::from_utf8_lossy(path),
            String::from_utf8_lossy(out_dir),
            &sc[..sc.len().min(96)],
            &sr[..sr.len().min(96)],
        );
    }
    sc
}

/// Variant of `diff_create` where `outDirName` points *into* a caller-supplied
/// buffer, so the out-of-bounds `outDirName[strlen(outDirName)-1]` read that
/// the C performs for an empty `outDirName` reads a byte we control.
///
/// `raw` is the whole buffer (must be NUL terminated at `raw[off..]`'s end);
/// `off` is the offset used as `outDirName`.
pub fn diff_create_raw_outdir(
    path: &[u8],
    raw: &[u8],
    off: usize,
    suffix_len: usize,
    ctx: &str,
) -> Vec<u8> {
    let p = pair();
    let bp = CBuf::new(path);
    let mut c_raw = raw.to_vec();
    let mut r_raw = raw.to_vec();
    let dir_c = unsafe { c_raw.as_mut_ptr().add(off) } as *const c_char;
    let dir_r = unsafe { r_raw.as_mut_ptr().add(off) } as *const c_char;
    let out_dir_len = raw[off..].iter().position(|&b| b == 0).expect("NUL");
    let n = ref_alloc_size(out_dir_len, ref_filename_len(path, SEP), suffix_len);
    let out_c = unsafe { (p.c.create)(bp.ptr(), dir_c, suffix_len) };
    let out_r = unsafe { (p.rs.create)(bp.ptr(), dir_r, suffix_len) };
    assert!(!out_c.is_null() && !out_r.is_null(), "{ctx}: NULL result");
    let sc = unsafe { std::slice::from_raw_parts(out_c as *const u8, n) }.to_vec();
    let sr = unsafe { std::slice::from_raw_parts(out_r as *const u8, n) }.to_vec();
    unsafe {
        free(out_c as *mut _);
        free(out_r as *mut _);
    }
    assert_eq!(
        sc,
        sr,
        "{ctx}: raw-outDir buffer mismatch\n raw={raw:?} off={off} path={:?}\n C={sc:?}\n R={sr:?}",
        String::from_utf8_lossy(path)
    );
    sc
}

// ---------------------------------------------------------------------------
// Fork helpers, for the paths that terminate or crash the process.
// ---------------------------------------------------------------------------

/// Raw `wait` status of a forked child running `f`. The child `_exit(0)`s if
/// `f` returns normally.
pub fn fork_status<F: FnOnce()>(f: F) -> i32 {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: i32 = -1;
    let r = unsafe { waitpid(pid, &mut status as *mut i32, 0) };
    assert_eq!(r, pid, "waitpid failed");
    status
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signalled(i32),
    Other(i32),
}

pub fn classify(status: i32) -> Outcome {
    // WIFEXITED / WEXITSTATUS / WIFSIGNALED / WTERMSIG for glibc
    if status & 0x7f == 0x7f {
        Outcome::Other(status)
    } else if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signalled(status & 0x7f)
    }
}

/// Run the same closure (parameterised by implementation) in two forked
/// children and assert both terminate identically.
pub fn diff_fork<F: Fn(&'static Impl)>(ctx: &str, f: F) -> Outcome {
    let p = pair();
    let c: &'static Impl = &p.c;
    let r: &'static Impl = &p.rs;
    let oc = classify(fork_status(|| f(c)));
    let or = classify(fork_status(|| f(r)));
    assert_eq!(oc, or, "{ctx}: termination mismatch C={oc:?} Rust={or:?}");
    oc
}
