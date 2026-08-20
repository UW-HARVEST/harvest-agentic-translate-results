//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and calls `UTIL_createLinePointers` through the dynamic-symbol boundary in
//! both, exactly as an external C consumer would.
//!
//! Nothing in this crate is called directly — the Rust implementation is only
//! ever reached through `dlsym("UTIL_createLinePointers")` on the cdylib, so the
//! `#[no_mangle] extern "C"` export wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

/// `const char** UTIL_createLinePointers(char*, size_t, size_t)`
pub type CreateLinePointersFn =
    unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char;

extern "C" {
    /// The same libc `free` both libraries allocate from.
    fn free(ptr: *mut c_void);
    /// glibc extension: usable bytes of a `malloc`ed block. Used to check that
    /// both implementations asked `malloc` for the *same* number of bytes
    /// (i.e. that `sizeof(const char**)` was translated as 8, and that the
    /// `numLines * sizeof(...)` arithmetic matches).
    fn malloc_usable_size(ptr: *mut c_void) -> usize;
}

/// Allocation-size parity: call each implementation, record the usable size of
/// the block it returned, and free it before the next call so glibc hands out
/// the same chunk (making the measurement deterministic).
///
/// Returns `Some((c_size, rust_size))` when both succeeded.
pub fn diff_alloc_size(
    buf: &mut [u8],
    num_lines: usize,
    buffer_size: usize,
    label: &str,
) -> Option<(usize, usize)> {
    assert!(buffer_size <= buf.len());
    let p = buf.as_mut_ptr() as *mut c_char;
    unsafe {
        let pc = c_create()(p, num_lines, buffer_size);
        let uc = if pc.is_null() {
            None
        } else {
            let u = malloc_usable_size(pc as *mut c_void);
            free(pc as *mut c_void);
            Some(u)
        };
        let pr = rust_create()(p, num_lines, buffer_size);
        let ur = if pr.is_null() {
            None
        } else {
            let u = malloc_usable_size(pr as *mut c_void);
            free(pr as *mut c_void);
            Some(u)
        };
        assert_eq!(
            uc.is_some(),
            ur.is_some(),
            "{label}: NULL-ness diverged (numLines={num_lines}, bufferSize={buffer_size})"
        );
        match (uc, ur) {
            (Some(a), Some(b)) => {
                assert_eq!(
                    a, b,
                    "{label}: malloc request size diverged for numLines={num_lines} \
                     (C usable={a}, Rust usable={b}) — the \
                     `numLines * sizeof(const char**)` arithmetic does not match"
                );
                Some((a, b))
            }
            _ => None,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libdriver.so")
}

/// Locate the Rust cdylib. Defaults to the directory that holds the current test
/// binary's parent (`target/<profile>/`), so it follows `--release` automatically.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test-bin>  ->  target/<profile>/libdriver.so
    let candidate = exe
        .parent()
        .and_then(|deps| deps.parent())
        .map(|prof| prof.join("libdriver.so"));
    match candidate {
        Some(p) if p.exists() => p,
        _ => manifest_dir().join("target/debug/libdriver.so"),
    }
}

/// GUARD AGAINST VACUOUS TESTS.
///
/// `cargo test` does **not** rebuild the cdylib, because `crate-type =
/// ["cdylib"]` means no integration test ever links the lib target. Without this
/// check the whole suite happily loads a stale `libdriver.so` and passes no
/// matter what `src/lib.rs` says. Refuse to run if the `.so` predates any Rust
/// source file.
fn assert_so_fresh(so: &PathBuf) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("cannot stat {so:?}: {e}"));

    let src = manifest_dir().join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(t) = entry.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                        newest = Some((p, t));
                    }
                }
            }
        }
    }

    if let Some((path, t)) = newest {
        assert!(
            so_mtime >= t,
            "STALE RUST .so — refusing to run vacuous tests.\n  \
             {so:?}\n  is OLDER than {path:?}\n\n\
             `cargo test` does not build a cdylib-only lib target. Run\n    \
             cargo build --no-default-features\n\
             (or use ./run_tests.sh) before `cargo test`."
        );
    }
}

struct Libs {
    _c: Library,
    _r: Library,
    c_fn: CreateLinePointersFn,
    r_fn: CreateLinePointersFn,
}

// Safety: raw `extern "C"` fn pointers into libraries kept alive for the whole
// process by this `static`.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "C shared library not found at {cp:?}; build it with cmake first"
        );
        assert!(
            rp.exists(),
            "Rust shared library not found at {rp:?}; run `cargo build` first"
        );
        assert_so_fresh(&rp);
        let c = Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}"));
        let r = Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}"));
        let c_sym: Symbol<CreateLinePointersFn> = c
            .get(b"UTIL_createLinePointers\0")
            .expect("C .so is missing UTIL_createLinePointers");
        let r_sym: Symbol<CreateLinePointersFn> = r
            .get(b"UTIL_createLinePointers\0")
            .expect("Rust .so is missing UTIL_createLinePointers");
        let c_fn = *c_sym;
        let r_fn = *r_sym;
        Libs {
            _c: c,
            _r: r,
            c_fn,
            r_fn,
        }
    })
}

pub fn c_create() -> CreateLinePointersFn {
    libs().c_fn
}

pub fn rust_create() -> CreateLinePointersFn {
    libs().r_fn
}

/// Outcome of one call, normalised so C and Rust results are comparable.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Outcome {
    /// Rejected: returned `NULL`.
    Null,
    /// Accepted: returned a non-NULL array; the recorded values are the
    /// `read_n` raw pointers it contains, expressed as byte offsets from
    /// `buffer` (offsets, not absolute addresses, so they are stable).
    Ok(Vec<isize>),
}

/// Call one implementation and normalise its result.
///
/// `read_n` slots are read back from the returned array (must be `<=` the number
/// of slots the implementation was asked to fill). The returned block is
/// `free`d with libc `free`, mirroring the documented ownership of the C API.
unsafe fn call_one(
    f: CreateLinePointersFn,
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
    read_n: usize,
) -> Outcome {
    let ret = f(buffer, num_lines, buffer_size);
    if ret.is_null() {
        return Outcome::Null;
    }
    let mut offsets = Vec::with_capacity(read_n);
    for i in 0..read_n {
        let p = *ret.add(i);
        offsets.push(p as isize - buffer as isize);
    }
    free(ret as *mut c_void);
    Outcome::Ok(offsets)
}

/// Core differential driver.
///
/// The *same* buffer pointer is handed to both implementations (the C function
/// only reads `buffer`), so the returned pointers are directly comparable.
///
/// `read_n` controls how many slots of the result are inspected; pass
/// `num_lines` for ordinary calls, and `0` when `num_lines` is an absurd value
/// used only to probe the allocation-size arithmetic.
pub unsafe fn diff_raw(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
    read_n: usize,
    label: &str,
) -> Outcome {
    let c = call_one(c_create(), buffer, num_lines, buffer_size, read_n);
    let r = call_one(rust_create(), buffer, num_lines, buffer_size, read_n);

    match (&c, &r) {
        (Outcome::Null, Outcome::Null) => {}
        (Outcome::Ok(_), Outcome::Ok(_)) => {}
        _ => panic!(
            "{label}: NULL-ness diverged (numLines={num_lines}, bufferSize={buffer_size}): \
             C={c:?} Rust={r:?}"
        ),
    }
    assert_eq!(
        c, r,
        "{label}: line-pointer arrays diverged (numLines={num_lines}, bufferSize={buffer_size})"
    );
    c
}

/// Convenience wrapper for a real `&mut [u8]` buffer.
pub fn diff(buf: &mut [u8], num_lines: usize, buffer_size: usize, label: &str) -> Outcome {
    assert!(
        buffer_size <= buf.len(),
        "{label}: test bug — bufferSize {buffer_size} exceeds backing buffer {}",
        buf.len()
    );
    let p = buf.as_mut_ptr() as *mut c_char;
    unsafe { diff_raw(p, num_lines, buffer_size, num_lines, label) }
}

/// Same, but only read back `read_n` slots.
pub fn diff_read(
    buf: &mut [u8],
    num_lines: usize,
    buffer_size: usize,
    read_n: usize,
    label: &str,
) -> Outcome {
    assert!(buffer_size <= buf.len());
    let p = buf.as_mut_ptr() as *mut c_char;
    unsafe { diff_raw(p, num_lines, buffer_size, read_n, label) }
}

/// Independently compute what the C algorithm must produce, straight from
/// `c_src/src/lib.c`. Used to prove the differential tests actually reach the
/// success path (i.e. that they are not trivially "both returned NULL").
pub fn model(buf: &[u8], num_lines: usize, buffer_size: usize) -> Outcome {
    let mut line_index = 0usize;
    let mut pos = 0usize;
    let mut out = Vec::new();
    while line_index < num_lines && pos < buffer_size {
        let mut len = 0usize;
        out.push(pos as isize);
        line_index += 1;
        while pos + len < buffer_size && buf[pos + len] != 0 {
            len += 1;
        }
        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }
    if line_index != num_lines {
        return Outcome::Null;
    }
    Outcome::Ok(out)
}

/// How many lines the C scan loop would yield from `buf[..buffer_size]` if
/// `numLines` were unbounded. Lets a test pick a `numLines` that is guaranteed
/// satisfiable (valid path) or guaranteed too large (error path).
pub fn count_lines(buf: &[u8], buffer_size: usize) -> usize {
    let mut pos = 0usize;
    let mut n = 0usize;
    while pos < buffer_size {
        let mut len = 0usize;
        n += 1;
        while pos + len < buffer_size && buf[pos + len] != 0 {
            len += 1;
        }
        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }
    n
}

/// Deterministic xorshift64* PRNG — fixed seed per test for reproducibility.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
}
