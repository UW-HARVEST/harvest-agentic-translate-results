//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls them only through
//! their exported C symbols:
//!
//!   * the C ground truth  : `../c_src/build/libdriver.so`
//!   * the Rust translation: `target/<profile>/libdriver.so`
//!
//! Rust functions are never called directly — always via the `.so` export, so
//! the `#[no_mangle] extern "C"` wrappers are under test too.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::os::unix::Library;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

// The result buffer of FIO_createFilename_fromOutDir() comes from libc calloc()
// in BOTH libraries, so the test frees it with the very same libc free().
unsafe extern "C" {
    fn free(p: *mut c_void);
    fn malloc(n: usize) -> *mut c_void;
    fn memset(p: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    /// glibc: the real usable size of an allocation. Used to detect a
    /// divergence in the *requested allocation size* — comparing only the bytes
    /// would silently pass if one side under-allocated and the bytes past the
    /// end happened to be zero.
    fn malloc_usable_size(p: *mut c_void) -> usize;
}

/// Poison `n` bytes of soon-to-be-reused heap with 0xAA, so that if a library
/// under-allocates, the bytes past its allocation read back as 0xAA instead of
/// an indistinguishable 0. Complements the `malloc_usable_size` assertion.
fn poison_heap(n: usize) {
    let n = n.max(1);
    // SAFETY: plain malloc/memset/free of a block we own exclusively.
    unsafe {
        let p = malloc(n);
        if !p.is_null() {
            memset(p, 0xAA, n);
            free(p);
        }
    }
}

/// `const char* extractFilename(const char* path, char separator)`
pub type ExtractFn = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;

/// The same symbol viewed with an `int` second parameter. A C caller passing an
/// `int` where the callee declares `char` is exactly how out-of-range narrow
/// values cross the FFI boundary, so this view is needed for Phase C.
pub type ExtractIntFn = unsafe extern "C" fn(*const c_char, c_int) -> *const c_char;

/// `char* FIO_createFilename_fromOutDir(const char*, const char*, size_t)`
pub type CreateFn = unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;

pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub extract: ExtractFn,
    pub extract_int: ExtractIntFn,
    pub create: CreateFn,
}

impl Api {
    fn open(name: &'static str, path: &PathBuf) -> Api {
        assert!(
            path.exists(),
            "shared object for `{name}` not found at {}.\n\
             Build both libraries first (see translation/run_tests.sh):\n  \
             cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
             cd translation && cargo build --offline",
            path.display()
        );
        // SAFETY: loading a plain C shared library with no init side effects.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        // SAFETY: the signatures below match the C declarations exactly.
        unsafe {
            let extract = *lib
                .get::<ExtractFn>(b"extractFilename\0")
                .unwrap_or_else(|e| panic!("{name}: extractFilename missing: {e}"));
            let extract_int = *lib
                .get::<ExtractIntFn>(b"extractFilename\0")
                .unwrap_or_else(|e| panic!("{name}: extractFilename missing: {e}"));
            let create = *lib
                .get::<CreateFn>(b"FIO_createFilename_fromOutDir\0")
                .unwrap_or_else(|e| {
                    panic!("{name}: FIO_createFilename_fromOutDir missing: {e}")
                });
            Api { name, _lib: lib, extract, extract_int, create }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/libdriver.so`, derived from the running test binary so the
/// profile always matches the one under test (never a stale artifact).
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test binary layout")
        .to_path_buf();
    profile_dir.join("libdriver.so")
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

/// The C ground truth.
pub fn c_api() -> Api {
    Api::open("C", &c_so_path())
}

/// The Rust translation, loaded as an external consumer would.
pub fn rust_api() -> Api {
    Api::open("Rust", &rust_so_path())
}

pub fn both() -> (Api, Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible
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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte_from(&mut self, alphabet: &[u8]) -> u8 {
        alphabet[self.below(alphabet.len())]
    }
    /// A random NUL-free byte in `0x01..=0xFF` (the full range a C string can hold).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.range(1, 255)) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Build a NUL-terminated byte vector of `len` bytes drawn from `alphabet`.
pub fn rand_cstring(rng: &mut Rng, len: usize, alphabet: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        v.push(rng.byte_from(alphabet));
    }
    v.push(0);
    v
}

/// `rand_cstring` with the length drawn from `lo..=hi` by the same `rng`
/// (avoids a double mutable borrow at the call site).
pub fn rand_cstr(rng: &mut Rng, lo: usize, hi: usize, alphabet: &[u8]) -> Vec<u8> {
    let len = rng.range(lo, hi);
    rand_cstring(rng, len, alphabet)
}

/// `rand_cstring_full` with the length drawn from `lo..=hi` by the same `rng`.
pub fn rand_cstr_full(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let len = rng.range(lo, hi);
    rand_cstring_full(rng, len)
}

/// Build a NUL-terminated byte vector of `len` bytes over the full `0x01..=0xFF`
/// range (exercises high / sign-extended bytes).
pub fn rand_cstring_full(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        v.push(rng.nonzero_byte());
    }
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Differential comparison helpers
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    let body: String = b
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| {
            if (0x20..0x7f).contains(&c) {
                (c as char).to_string()
            } else {
                format!("\\x{c:02x}")
            }
        })
        .collect();
    format!("\"{body}\"")
}

/// Call `extractFilename` in both libraries on the SAME buffer and assert the
/// returned pointers land at the same offset.
///
/// Comparing offsets (not raw addresses) is the only meaningful comparison: the
/// C function returns an interior pointer into the caller's buffer.
#[track_caller]
pub fn diff_extract(c: &Api, r: &Api, path: &[u8], sep: u8) -> isize {
    assert_eq!(path.last(), Some(&0), "test bug: path must be NUL terminated");
    let base = path.as_ptr() as *const c_char;
    // SAFETY: `base` is a NUL-terminated C string living in `path`.
    let (co, ro) = unsafe {
        let cr = (c.extract)(base, sep as i8 as c_char);
        let rr = (r.extract)(base, sep as i8 as c_char);
        (
            cr as isize - base as isize,
            rr as isize - base as isize,
        )
    };
    assert_eq!(
        co, ro,
        "extractFilename divergence: path={} sep=0x{sep:02x} -> C offset {co}, Rust offset {ro}",
        show(path)
    );
    co
}

/// Same, but passing the separator as a full `int` (out-of-range narrow value).
#[track_caller]
pub fn diff_extract_int(c: &Api, r: &Api, path: &[u8], sep: c_int) -> isize {
    assert_eq!(path.last(), Some(&0), "test bug: path must be NUL terminated");
    let base = path.as_ptr() as *const c_char;
    // SAFETY: `base` is a NUL-terminated C string living in `path`.
    let (co, ro) = unsafe {
        let cr = (c.extract_int)(base, sep);
        let rr = (r.extract_int)(base, sep);
        (
            cr as isize - base as isize,
            rr as isize - base as isize,
        )
    };
    assert_eq!(
        co, ro,
        "extractFilename(int) divergence: path={} sep={sep} -> C offset {co}, Rust offset {ro}",
        show(path)
    );
    co
}

/// The exact number of bytes `calloc` is asked for by the C source:
/// `strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1`
/// (wrapping, like `size_t`).
pub fn alloc_size(path: &[u8], out_dir: &[u8], suffix_len: usize) -> usize {
    let dir_len = cstr_len(out_dir);
    let name_len = filename_tail_len(path);
    dir_len
        .wrapping_add(1)
        .wrapping_add(name_len)
        .wrapping_add(suffix_len)
        .wrapping_add(1)
}

pub fn cstr_len(s: &[u8]) -> usize {
    s.iter().position(|&c| c == 0).unwrap_or(s.len())
}

/// Length of `extractFilename(path, '/')`, computed independently of both libraries.
pub fn filename_tail_len(path: &[u8]) -> usize {
    let body = &path[..cstr_len(path)];
    match body.iter().rposition(|&c| c == b'/') {
        Some(i) => body.len() - i - 1,
        None => body.len(),
    }
}

/// Call `FIO_createFilename_fromOutDir` in both libraries and compare the
/// **whole allocation** byte-for-byte (content *and* the `calloc` zero-fill),
/// then `free()` both buffers.
#[track_caller]
pub fn diff_create(c: &Api, r: &Api, path: &[u8], out_dir: &[u8], suffix_len: usize) {
    let n = alloc_size(path, out_dir, suffix_len);
    diff_create_n(c, r, path, out_dir, suffix_len, n);
}

/// As `diff_create`, but compares exactly `read_len` bytes. Used where the
/// `size_t` arithmetic wraps and the full allocation is not the natural size.
#[track_caller]
pub fn diff_create_n(
    c: &Api,
    r: &Api,
    path: &[u8],
    out_dir: &[u8],
    suffix_len: usize,
    read_len: usize,
) {
    assert_eq!(path.last(), Some(&0), "test bug: path must be NUL terminated");
    assert_eq!(out_dir.last(), Some(&0), "test bug: outDir must be NUL terminated");
    diff_create_raw(
        c,
        r,
        path.as_ptr() as *const c_char,
        out_dir.as_ptr() as *const c_char,
        suffix_len,
        read_len,
        &format!("path={} outDir={} suffixLen={suffix_len}", show(path), show(out_dir)),
    );
}

/// Call one library's `FIO_createFilename_fromOutDir`, copy out `read_len`
/// bytes, `free()` the buffer and return the copy.
///
/// # Safety
/// `path`/`out_dir` must be valid NUL-terminated C strings and the library must
/// legitimately produce at least `read_len` bytes.
pub unsafe fn create_bytes(
    api: &Api,
    path: *const c_char,
    out_dir: *const c_char,
    suffix_len: usize,
    read_len: usize,
) -> Vec<u8> {
    // SAFETY: guaranteed by the caller.
    unsafe {
        let p = (api.create)(path, out_dir, suffix_len);
        assert!(!p.is_null(), "{}: returned NULL unexpectedly", api.name);
        let v = std::slice::from_raw_parts(p as *const u8, read_len).to_vec();
        free(p as *mut c_void);
        v
    }
}

/// Lowest-level differential call: raw pointers, explicit compare length.
///
/// # Safety
/// `path` and `out_dir` must be valid NUL-terminated C strings, and both
/// libraries must legitimately produce at least `read_len` bytes.
#[track_caller]
pub fn diff_create_raw(
    c: &Api,
    r: &Api,
    path: *const c_char,
    out_dir: *const c_char,
    suffix_len: usize,
    read_len: usize,
    label: &str,
) {
    // SAFETY: caller guarantees the C-string preconditions; the returned
    // buffers come from libc calloc() in both libraries and are freed below.
    unsafe {
        // Each library is invoked under the SAME heap conditions: poison, call,
        // snapshot, free. Doing them one at a time (rather than both, then both
        // frees) keeps the allocator state comparable.
        poison_heap(read_len);
        let cp = (c.create)(path, out_dir, suffix_len);
        assert!(!cp.is_null(), "C returned NULL, which it never should ({label})");
        let c_usable = malloc_usable_size(cp as *mut c_void);
        let cb = std::slice::from_raw_parts(cp as *const u8, read_len).to_vec();
        free(cp as *mut c_void);

        poison_heap(read_len);
        let rp = (r.create)(path, out_dir, suffix_len);
        assert!(!rp.is_null(), "Rust returned NULL but C did not ({label})");
        let r_usable = malloc_usable_size(rp as *mut c_void);
        let rb = std::slice::from_raw_parts(rp as *const u8, read_len).to_vec();
        free(rp as *mut c_void);

        // The requested allocation size is part of the contract: the caller is
        // promised room for outDir + separator + filename + suffixLen + NUL.
        //
        // NOTE: `malloc_usable_size` is only a LOWER bound on the request —
        // glibc may serve a request from a larger bin, so the two libraries can
        // legitimately report different usable sizes for the same request
        // (observed: request 55 -> 56 for one call, 72 for another). Asserting
        // equality is therefore wrong; asserting "no under-allocation" is both
        // deterministic and exactly the property that matters.
        assert!(
            c_usable >= read_len,
            "C under-allocated ({label}): usable={c_usable} < needed={read_len}"
        );
        assert!(
            r_usable >= read_len,
            "Rust UNDER-ALLOCATED ({label}): usable={r_usable} < needed={read_len} \
             (C gave {c_usable}) — the caller is promised room for \
             outDir + separator + filename + suffixLen + NUL"
        );

        if cb != rb {
            let at = cb.iter().zip(rb.iter()).position(|(a, b)| a != b);
            panic!(
                "FIO_createFilename_fromOutDir divergence ({label})\n  \
                 first differing byte index: {at:?}\n  \
                 C   ({} bytes): {:?}\n  Rust({} bytes): {:?}",
                cb.len(),
                show(&cb),
                rb.len(),
                show(&rb)
            );
        }
    }
}
