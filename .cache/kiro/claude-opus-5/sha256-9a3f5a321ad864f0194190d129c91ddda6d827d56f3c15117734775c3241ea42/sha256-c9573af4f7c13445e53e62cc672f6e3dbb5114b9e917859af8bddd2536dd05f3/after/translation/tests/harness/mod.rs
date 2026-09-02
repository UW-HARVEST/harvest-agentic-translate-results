//! Differential test harness: loads BOTH the C `libdriver.so` and the Rust
//! `libdriver.so` through `libloading` and compares their exported symbols'
//! behaviour byte-for-byte.
//!
//! Nothing here calls the Rust implementation directly — every Rust call goes
//! through `dlsym` on the built `cdylib`, so the `#[no_mangle]` / `extern "C"`
//! wrappers are under test too.

#![allow(dead_code)]

use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits used by the harness itself
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    /// glibc: usable bytes in an allocated block. A deterministic function of
    /// the size that was requested, so comparing it across the two
    /// implementations detects allocation-size divergence even when the buffer
    /// *contents* happen to agree.
    fn malloc_usable_size(p: *mut c_void) -> usize;
}

/// `strlen` re-exported for the test files.
pub unsafe fn c_strlen(s: *const c_char) -> usize {
    unsafe { strlen(s) }
}

// ---------------------------------------------------------------------------
// FFI signatures of the two exported symbols
// ---------------------------------------------------------------------------

pub type ExtractFilenameFn = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;
pub type FioCreateFn = unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;

/// One loaded implementation (either the C one or the Rust one).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub extract_filename: ExtractFilenameFn,
    pub fio_create: FioCreateFn,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {name} at {}: {e}", path.display()));
        let extract_filename: ExtractFilenameFn = unsafe {
            let s: Symbol<ExtractFilenameFn> = lib
                .get(b"extractFilename\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol extractFilename: {e}"));
            *s
        };
        let fio_create: FioCreateFn = unsafe {
            let s: Symbol<FioCreateFn> = lib
                .get(b"FIO_createFilename_fromOutDir\0")
                .unwrap_or_else(|e| {
                    panic!("{name}: missing symbol FIO_createFilename_fromOutDir: {e}")
                });
            *s
        };
        Impl {
            name,
            _lib: lib,
            extract_filename,
            fio_create,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared object not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    // Explicit override wins, so the runner script can point the suite at a
    // specific profile's cdylib deterministically.
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_SO={} is not a file", p.display());
        return p;
    }
    let base = manifest_dir().join("target");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["debug", "release"] {
        let cand = base.join(profile).join("libdriver.so");
        if let Ok(md) = std::fs::metadata(&cand) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, cand));
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found under {}. Build it with `cargo build` / `cargo build --release`.",
            base.display()
        )
    })
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &c_so_path()),
        rs: Impl::load("Rust", &rust_so_path()),
    })
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
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A random NUL-free byte from the full 0x01..=0xFF domain.
    pub fn nonzero_byte(&mut self) -> u8 {
        let b = self.byte();
        if b == 0 { 1 } else { b }
    }
    /// A random NUL-free byte that is also not the given byte.
    pub fn nonzero_byte_except(&mut self, ex: u8) -> u8 {
        for _ in 0..8 {
            let b = self.nonzero_byte();
            if b != ex {
                return b;
            }
        }
        if ex == 1 { 2 } else { 1 }
    }
}

// ---------------------------------------------------------------------------
// String building helpers. All produce NUL-terminated byte buffers.
// ---------------------------------------------------------------------------

pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0), "test input must not contain NUL");
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Random NUL-free bytes of `len`, never containing `forbid`.
pub fn rand_bytes_without(rng: &mut Rng, len: usize, forbid: u8) -> Vec<u8> {
    (0..len).map(|_| rng.nonzero_byte_except(forbid)).collect()
}

/// Random NUL-free bytes of `len`, `/` allowed to appear naturally.
pub fn rand_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.nonzero_byte()).collect()
}

// ---------------------------------------------------------------------------
// Differential comparators
// ---------------------------------------------------------------------------

/// Compare `extractFilename` across both implementations. The result is a
/// pointer *into* `path`, so the strongest possible comparison is the returned
/// offset (this also stays valid for the `separator == '\0'` case, where the
/// result legitimately points one byte past the end of the buffer).
#[track_caller]
pub fn diff_extract(path: &[u8], separator: u8, ctx: &str) {
    let p = pair();
    let base = path.as_ptr() as *const c_char;
    let c_ret = unsafe { (p.c.extract_filename)(base, separator as c_char) };
    let r_ret = unsafe { (p.rs.extract_filename)(base, separator as c_char) };
    let c_off = (c_ret as isize) - (base as isize);
    let r_off = (r_ret as isize) - (base as isize);
    assert_eq!(
        c_off, r_off,
        "extractFilename offset mismatch [{ctx}]: sep={separator:#04x} \
         path={:?} C=+{c_off} Rust=+{r_off}",
        String::from_utf8_lossy(&path[..path.len().saturating_sub(1)])
    );
    assert_eq!(
        c_ret as usize, r_ret as usize,
        "extractFilename pointer mismatch [{ctx}]: sep={separator:#04x}"
    );
}

/// The exact number of bytes `calloc` was asked for by the C code, reproduced
/// here purely so the harness knows how much of the returned buffer to compare
/// (wrapping arithmetic included, exactly like C's `size_t` math).
fn alloc_size(path: &[u8], out_dir: &[u8], suffix_len: usize) -> usize {
    let p = pair();
    let fstart = unsafe { (p.c.extract_filename)(path.as_ptr() as *const c_char, b'/' as c_char) };
    let fname_len = unsafe { strlen(fstart) };
    let out_dir_len = unsafe { strlen(out_dir.as_ptr() as *const c_char) };
    out_dir_len
        .wrapping_add(1)
        .wrapping_add(fname_len)
        .wrapping_add(suffix_len)
        .wrapping_add(1)
}

/// Cap on how many bytes of the returned buffer we memcmp (keeps multi-MiB
/// `suffixLen` rows fast while still covering the whole real allocation).
const MAX_CMP: usize = 8 * 1024 * 1024;

/// Compare `FIO_createFilename_fromOutDir` across both implementations over the
/// **entire** allocated buffer: the composed path bytes *and* the zero tail
/// `calloc` promises.
#[track_caller]
pub fn diff_fio_ptr(path: *const c_char, out_dir: *const c_char, suffix_len: usize, cmp_len: usize, ctx: &str) {
    diff_fio_ptr_raw(path, out_dir, suffix_len, cmp_len, true, ctx)
}

/// As `diff_fio_ptr`, but `check_strlen` can be disabled for the pathological
/// wrap-to-zero allocation where the NUL terminator is never written and the
/// bytes past the written region are genuinely indeterminate.
#[track_caller]
pub fn diff_fio_ptr_raw(
    path: *const c_char,
    out_dir: *const c_char,
    suffix_len: usize,
    cmp_len: usize,
    check_strlen: bool,
    ctx: &str,
) {
    let p = pair();
    let c_ret = unsafe { (p.c.fio_create)(path, out_dir, suffix_len) };
    let r_ret = unsafe { (p.rs.fio_create)(path, out_dir, suffix_len) };
    assert_eq!(
        c_ret.is_null(),
        r_ret.is_null(),
        "FIO_createFilename_fromOutDir null-ness mismatch [{ctx}]"
    );
    if c_ret.is_null() {
        return;
    }
    let n = cmp_len.min(MAX_CMP);
    let c_bytes = unsafe { std::slice::from_raw_parts(c_ret as *const u8, n) };
    let r_bytes = unsafe { std::slice::from_raw_parts(r_ret as *const u8, n) };
    if c_bytes != r_bytes {
        let first = c_bytes
            .iter()
            .zip(r_bytes.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "FIO_createFilename_fromOutDir buffer mismatch [{ctx}] at byte {first}: \
             C={:#04x} Rust={:#04x}\n  C   = {:?}\n  Rust= {:?}",
            c_bytes[first],
            r_bytes[first],
            String::from_utf8_lossy(&c_bytes[..n.min(160)]),
            String::from_utf8_lossy(&r_bytes[..n.min(160)]),
        );
    }
    // Deterministic under-allocation check: glibc guarantees
    // `malloc_usable_size(p) >= requested`, so if either implementation asked
    // for fewer bytes than the C formula requires, this fires. (Equality of
    // usable sizes is NOT asserted: glibc reports the reused chunk's capacity,
    // which depends on heap history, so it is not a function of the request.
    // The exact request size is compared instead in `tests/phase_e_alloc.rs`,
    // which interposes `calloc`.)
    if cmp_len >= 1 {
        let c_usable = unsafe { malloc_usable_size(c_ret as *mut c_void) };
        let r_usable = unsafe { malloc_usable_size(r_ret as *mut c_void) };
        assert!(
            c_usable >= cmp_len,
            "C under-allocated [{ctx}]: usable {c_usable} < needed {cmp_len}"
        );
        assert!(
            r_usable >= cmp_len,
            "Rust under-allocated [{ctx}]: usable {r_usable} < needed {cmp_len} \
             (C usable {c_usable})"
        );
    }

    // Also assert the C-visible strings agree (redundant but pinpoints
    // truncation bugs in the failure message).
    if check_strlen {
        let c_str_len = unsafe { strlen(c_ret) };
        let r_str_len = unsafe { strlen(r_ret) };
        assert_eq!(
            c_str_len, r_str_len,
            "FIO_createFilename_fromOutDir strlen mismatch [{ctx}]"
        );
    }
    unsafe {
        free(c_ret as *mut c_void);
        free(r_ret as *mut c_void);
    }
}

/// Convenience wrapper for NUL-terminated `Vec<u8>` inputs.
#[track_caller]
pub fn diff_fio(path: &[u8], out_dir: &[u8], suffix_len: usize, ctx: &str) {
    let n = alloc_size(path, out_dir, suffix_len);
    diff_fio_ptr(
        path.as_ptr() as *const c_char,
        out_dir.as_ptr() as *const c_char,
        suffix_len,
        n,
        ctx,
    );
}

// ---------------------------------------------------------------------------
// Child-process plumbing for the fatal error paths (exit(30) / SIGSEGV).
// ---------------------------------------------------------------------------

pub const CHILD_ENV: &str = "DIFFTEST_CHILD_IMPL";

/// Which implementation a child process should exercise, or `None` when we are
/// the parent and the body must be skipped.
pub fn child_impl() -> Option<&'static Impl> {
    match std::env::var(CHILD_ENV).ok()?.as_str() {
        "c" => Some(&pair().c),
        "rust" => Some(&pair().rs),
        other => panic!("unexpected {CHILD_ENV}={other}"),
    }
}

/// Re-invoke this very test binary, running only `test_name`, with the child
/// marker set to `which`. Returns `(exit_code, signal)`.
pub fn run_child(test_name: &str, which: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child test process");
    (out.status.code(), out.status.signal())
}

/// Run the named child test under both implementations and assert the two
/// processes terminate identically (same exit code, same signal).
#[track_caller]
pub fn diff_fatal(test_name: &str, expect_code: Option<i32>, expect_signal: Option<i32>) {
    let c = run_child(test_name, "c");
    let r = run_child(test_name, "rust");
    assert_eq!(
        c, r,
        "fatal-path termination mismatch for {test_name}: C=(code {:?}, signal {:?}) \
         Rust=(code {:?}, signal {:?})",
        c.0, c.1, r.0, r.1
    );
    if let Some(code) = expect_code {
        assert_eq!(
            c.0,
            Some(code),
            "{test_name}: expected exit code {code}, got (code {:?}, signal {:?})",
            c.0,
            c.1
        );
    }
    if let Some(sig) = expect_signal {
        assert_eq!(
            c.1,
            Some(sig),
            "{test_name}: expected signal {sig}, got (code {:?}, signal {:?})",
            c.0,
            c.1
        );
    }
}
