//! Shared differential-test harness.
//!
//! Both the C library (`c_src/build/libdriver.so`) and the Rust library
//! (`target/<profile>/libdriver.so`) are loaded with `libloading` and driven
//! exclusively through their exported C symbols — the Rust functions are never
//! called directly, so the `#[no_mangle]` / `extern "C"` wrappers are part of
//! what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (declared inline to avoid extra deps)
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn free(p: *mut c_void);
    fn strerror(errnum: c_int) -> *mut c_char;
}

pub const ENOMEM: c_int = 12;

/// `strerror(ENOMEM)` as bytes — the exact text both libraries print when
/// `calloc` fails.
pub fn strerror_bytes(errnum: c_int) -> Vec<u8> {
    unsafe {
        let p = strerror(errnum);
        let mut out = Vec::new();
        let mut i = 0isize;
        while *p.offset(i) != 0 {
            out.push(*p.offset(i) as u8);
            i += 1;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of the C shared library built from `c_src/`.
pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Path of the Rust `cdylib`, located next to the running test executable
/// (`target/<profile>/deps/<test>` ⇒ `target/<profile>/libdriver.so`).
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let candidates = [
        deps.join("libdriver.so"),
        deps.parent().map(|p| p.join("libdriver.so")).unwrap_or_default(),
    ];
    for c in &candidates {
        if c.exists() {
            assert_fresh(c);
            return c.clone();
        }
    }
    panic!("Rust cdylib libdriver.so not found near {deps:?}");
}

/// `cargo test` does **not** relink a `crate-type = ["cdylib"]` artifact, so a
/// stale `libdriver.so` would silently be tested. Refuse to run in that case.
fn assert_fresh(so: &Path) {
    let so_m = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat .so");
    let src = manifest_dir().join("src/lib.rs");
    let src_m = std::fs::metadata(&src)
        .and_then(|m| m.modified())
        .expect("stat src/lib.rs");
    assert!(
        so_m >= src_m,
        "STALE Rust cdylib: {so:?} is older than {src:?}.\n\
         `cargo test` does not relink cdylib targets — run `cargo build` first \
         (or use ./run_all.sh)."
    );
}

/// Locate the compiled `child` test binary (`harness = false`), used by the
/// error-path tests. Cargo names it `child-<hash>` inside the deps directory.
pub fn child_bin_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir").to_path_buf();
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&deps).expect("read deps dir").flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("child-") {
            continue;
        }
        if path.extension().is_some() {
            continue; // skip child-<hash>.d and friends
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
            best = Some((mtime, path));
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!("`child` test binary not found in {deps:?} — run `cargo test` (not a filtered single target)")
    })
}

// ---------------------------------------------------------------------------
// Typed views on the two libraries
// ---------------------------------------------------------------------------

pub type ExtractFilenameFn = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;
pub type CreateFilenameFn = unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub extract_filename: ExtractFilenameFn,
    pub create_filename: CreateFilenameFn,
}

impl Impl {
    pub fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
            let extract: Symbol<ExtractFilenameFn> = lib
                .get(b"extractFilename\0")
                .unwrap_or_else(|e| panic!("{path:?} missing extractFilename: {e}"));
            let create: Symbol<CreateFilenameFn> = lib
                .get(b"FIO_createFilename_fromOutDir\0")
                .unwrap_or_else(|e| panic!("{path:?} missing FIO_createFilename_fromOutDir: {e}"));
            let extract_filename = *extract;
            let create_filename = *create;
            Impl {
                name,
                _lib: lib,
                extract_filename,
                create_filename,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("C", &c_so_path()),
        rust: Impl::load("Rust", &rust_so_path()),
    }
}

/// One shared `Pair` per test process (dlopen is cheap but this keeps output tidy).
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(load_pair)
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Call `extractFilename` in both libraries with the *same* input buffer and
/// compare the returned pointers as offsets from the start of that buffer
/// (pointer identity is what the C contract is about: it returns either `path`
/// itself or an interior pointer).
///
/// `path_bytes` must include its own terminating NUL.
pub fn diff_extract_filename(path_bytes: &[u8], separator: u8, ctx: &str) -> isize {
    assert!(
        path_bytes.last() == Some(&0),
        "path buffer must be NUL terminated"
    );
    let p = path_bytes.as_ptr() as *const c_char;
    let (c_ret, r_ret) = unsafe {
        let f = pair();
        (
            (f.c.extract_filename)(p, separator as c_char),
            (f.rust.extract_filename)(p, separator as c_char),
        )
    };
    let c_off = (c_ret as isize) - (p as isize);
    let r_off = (r_ret as isize) - (p as isize);
    assert_eq!(
        c_off, r_off,
        "extractFilename offset mismatch [{ctx}]: sep=0x{separator:02x} path={:?}\n  C   -> +{c_off}\n  Rust-> +{r_off}",
        Escaped(path_bytes)
    );
    c_off
}

/// Like [`diff_extract_filename`] but the caller supplies the exact pointer to
/// pass (used for interior / one-past-the-end pointers).
pub fn diff_extract_filename_at(
    base: *const c_char,
    arg: *const c_char,
    separator: u8,
    ctx: &str,
) -> isize {
    let (c_ret, r_ret) = unsafe {
        let f = pair();
        (
            (f.c.extract_filename)(arg, separator as c_char),
            (f.rust.extract_filename)(arg, separator as c_char),
        )
    };
    let c_off = (c_ret as isize) - (base as isize);
    let r_off = (r_ret as isize) - (base as isize);
    assert_eq!(
        c_off, r_off,
        "extractFilename offset mismatch [{ctx}]: sep=0x{separator:02x}"
    );
    c_off
}

/// The allocation size the C code computes, reproduced with the same wrapping
/// `size_t` arithmetic. Used to decide how many bytes of the result are
/// defined and must therefore be compared.
pub fn expected_alloc_size(out_dir_len: usize, filename_len: usize, suffix_len: usize) -> usize {
    out_dir_len
        .wrapping_add(1)
        .wrapping_add(filename_len)
        .wrapping_add(suffix_len)
        .wrapping_add(1)
}

/// Reference computation of `filenameStart`'s length for the non-Windows build
/// (`separator == '/'`). Only valid for paths without interior NUL bytes.
pub fn filename_component_len(path: &[u8]) -> usize {
    match path.iter().rposition(|&b| b == b'/') {
        Some(i) => path.len() - i - 1,
        None => path.len(),
    }
}

/// Call `FIO_createFilename_fromOutDir` in both libraries and compare the full
/// defined contents of the two heap buffers byte-for-byte.
///
/// `path` / `out_dir` are given **without** the terminating NUL, which this
/// helper appends. `compare_len` is the number of bytes of the result that are
/// defined (normally [`expected_alloc_size`]).
pub fn diff_create_filename_raw(
    path: &[u8],
    out_dir: &[u8],
    suffix_len: usize,
    compare_len: usize,
    ctx: &str,
) -> Vec<u8> {
    let mut path_c = path.to_vec();
    path_c.push(0);
    let mut dir_c = out_dir.to_vec();
    dir_c.push(0);
    diff_create_filename_ptrs(
        path_c.as_ptr() as *const c_char,
        dir_c.as_ptr() as *const c_char,
        suffix_len,
        compare_len,
        ctx,
    )
}

/// Same as [`diff_create_filename_raw`] but with caller-supplied pointers, so
/// tests can control the byte *preceding* `outDirName` (the out-of-bounds read
/// the C performs when `outDirName` is empty).
pub fn diff_create_filename_ptrs(
    path: *const c_char,
    out_dir: *const c_char,
    suffix_len: usize,
    compare_len: usize,
    ctx: &str,
) -> Vec<u8> {
    unsafe {
        let f = pair();
        let c_ret = (f.c.create_filename)(path, out_dir, suffix_len);
        let r_ret = (f.rust.create_filename)(path, out_dir, suffix_len);
        assert!(!c_ret.is_null(), "C returned NULL [{ctx}]");
        assert!(!r_ret.is_null(), "Rust returned NULL [{ctx}]");
        let c_bytes = std::slice::from_raw_parts(c_ret as *const u8, compare_len).to_vec();
        let r_bytes = std::slice::from_raw_parts(r_ret as *const u8, compare_len).to_vec();
        free(c_ret as *mut c_void);
        free(r_ret as *mut c_void);
        assert_eq!(
            c_bytes,
            r_bytes,
            "FIO_createFilename_fromOutDir mismatch [{ctx}] suffixLen={suffix_len}\n  C   = {:?}\n  Rust= {:?}",
            Escaped(&c_bytes),
            Escaped(&r_bytes)
        );
        c_bytes
    }
}

/// Convenience wrapper: computes the compare length from the C code's own
/// formula and checks the whole (zero-padded) buffer.
pub fn diff_create_filename(path: &[u8], out_dir: &[u8], suffix_len: usize, ctx: &str) -> Vec<u8> {
    let flen = filename_component_len(path);
    let size = expected_alloc_size(out_dir.len(), flen, suffix_len);
    diff_create_filename_raw(path, out_dir, suffix_len, size, ctx)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — no external crates, fixed seed.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A byte that is never NUL and never `'/'` — safe as "ordinary" path content.
    pub fn plain_byte(&mut self) -> u8 {
        loop {
            let b = self.byte();
            if b != 0 && b != b'/' {
                return b;
            }
        }
    }
    /// Printable ASCII, excluding `'/'`.
    pub fn ascii_byte(&mut self) -> u8 {
        loop {
            let b = 0x21 + (self.below(0x5e) as u8);
            if b != b'/' {
                return b;
            }
        }
    }
    /// [`Rng::path_like`] with a random length in `lo..=hi`.
    /// (Exists so call sites don't need two simultaneous `&mut` borrows.)
    pub fn path_r(&mut self, lo: usize, hi: usize, sep_density: usize, high_bit: bool) -> Vec<u8> {
        let len = self.range(lo, hi);
        self.path_like(len, sep_density, high_bit)
    }
    /// [`Rng::path_like`] with a random length in `0..n`.
    pub fn path_b(&mut self, n: usize, sep_density: usize, high_bit: bool) -> Vec<u8> {
        let len = self.below(n);
        self.path_like(len, sep_density, high_bit)
    }
    /// [`Rng::path_like`] with a random length, random separator density and a
    /// random high-bit choice.
    pub fn path_rand(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let len = self.range(lo, hi);
        let density = self.below(30);
        let high = self.below(4) == 0;
        self.path_like(len, density, high)
    }
    /// [`Rng::path_like`] with a fixed length and a random separator density.
    pub fn path_d(&mut self, len: usize, max_density: usize, high_bit: bool) -> Vec<u8> {
        let density = self.below(max_density);
        self.path_like(len, density, high_bit)
    }
    /// [`Rng::path_like`] with a fixed length, random density, random high bit.
    pub fn path_dh(&mut self, len: usize, max_density: usize) -> Vec<u8> {
        let density = self.below(max_density);
        let high = self.below(3) == 0;
        self.path_like(len, density, high)
    }
    /// Random NUL-free string of `len` bytes; `sep_density` in percent controls
    /// how often a `'/'` is emitted.
    pub fn path_like(&mut self, len: usize, sep_density: usize, high_bit: bool) -> Vec<u8> {
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            if self.below(100) < sep_density {
                v.push(b'/');
            } else if high_bit {
                let b = self.plain_byte();
                v.push(b | 0x80);
            } else {
                v.push(self.ascii_byte());
            }
        }
        v
    }
}

// ---------------------------------------------------------------------------
// Pretty printing for assertion messages
// ---------------------------------------------------------------------------

pub struct Escaped<'a>(pub &'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for &b in self.0 {
            match b {
                0x20..=0x7e if b != b'"' && b != b'\\' => write!(f, "{}", b as char)?,
                b'"' => write!(f, "\\\"")?,
                b'\\' => write!(f, "\\\\")?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        write!(f, "\"")
    }
}
