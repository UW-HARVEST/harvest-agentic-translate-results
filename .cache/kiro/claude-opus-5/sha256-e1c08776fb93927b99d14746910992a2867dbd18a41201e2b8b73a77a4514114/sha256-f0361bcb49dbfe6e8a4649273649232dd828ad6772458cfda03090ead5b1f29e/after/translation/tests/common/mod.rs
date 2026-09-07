//! Shared plumbing for the differential tests: locate and `dlopen` both the C
//! and the Rust shared object and call `searchAndReplace` through the FFI
//! boundary in each of them.
//!
//! Nothing in here calls the Rust implementation directly — every invocation
//! goes through `libloading` and the exported `#[no_mangle]` symbol, exactly
//! as an external consumer would.

#![allow(dead_code)]

use std::ffi::{c_char, c_void};
use std::path::PathBuf;

pub type SearchAndReplaceFn =
    unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

extern "C" {
    fn free(p: *mut c_void);
}

/// Directory holding the crate (`translation/`).
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of the C shared library built from `c_src/`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir()
        .parent()
        .expect("crate dir has a parent")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Path of the Rust `cdylib` under test.
///
/// The test binary lives in `target/<profile>/deps/`, so the sibling `.so`
/// produced by the same `cargo test` invocation is two levels up. That keeps
/// the lookup correct for any profile and any `--features` combination.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {p:?}. `cargo test` does not build the \
         cdylib artifact (integration tests do not link it), so run\n  \
         cargo build\nfirst, or use tests/run_all_features.sh which does \
         both. Alternatively set DRIVER_RUST_SO."
    );
    p
}

/// A loaded implementation: the library must outlive the function pointer, so
/// both are kept together.
pub struct Impl {
    _lib: libloading::Library,
    pub f: SearchAndReplaceFn,
    pub name: &'static str,
}

impl Impl {
    fn load(path: PathBuf, name: &'static str) -> Impl {
        unsafe {
            let lib = libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {path:?} failed: {e}"));
            let sym: libloading::Symbol<SearchAndReplaceFn> = lib
                .get(b"searchAndReplace\0")
                .unwrap_or_else(|e| panic!("dlsym searchAndReplace in {path:?} failed: {e}"));
            let f = *sym;
            Impl {
                _lib: lib,
                f,
                name,
            }
        }
    }

    pub fn c() -> Impl {
        Impl::load(c_so_path(), "C")
    }

    pub fn rust() -> Impl {
        Impl::load(rust_so_path(), "Rust")
    }

    /// Call the implementation with three NUL-terminated byte strings and
    /// return the result as an owned `Option<Vec<u8>>` (`None` == `NULL`
    /// return), freeing the returned buffer with `free()` exactly as the C
    /// contract requires.
    pub fn call(&self, orig: &[u8], search: &[u8], value: &[u8]) -> Option<Vec<u8>> {
        let o = cstr(orig);
        let s = cstr(search);
        let v = cstr(value);
        unsafe {
            let p = (self.f)(o.as_ptr() as *const c_char, s.as_ptr() as *const c_char,
                             v.as_ptr() as *const c_char);
            if p.is_null() {
                return None;
            }
            let out = read_cstr(p);
            free(p as *mut c_void);
            Some(out)
        }
    }
}

/// NUL-terminated copy of `b`. Panics if `b` already contains a NUL, since
/// such a value cannot be passed to a `const char *` API.
pub fn cstr(b: &[u8]) -> Vec<u8> {
    assert!(!b.contains(&0), "interior NUL in test input");
    let mut v = Vec::with_capacity(b.len() + 1);
    v.extend_from_slice(b);
    v.push(0);
    v
}

unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut len = 0usize;
    while *p.add(len) != 0 {
        len += 1;
    }
    std::slice::from_raw_parts(p as *const u8, len).to_vec()
}

/// Both implementations, loaded once.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

impl Pair {
    pub fn new() -> Pair {
        Pair {
            c: Impl::c(),
            rust: Impl::rust(),
        }
    }

    /// Run one input through both libraries and assert byte-identical results.
    #[track_caller]
    pub fn assert_same(&self, orig: &[u8], search: &[u8], value: &[u8]) -> Option<Vec<u8>> {
        let got_c = self.c.call(orig, search, value);
        let got_r = self.rust.call(orig, search, value);
        if got_c != got_r {
            panic!(
                "divergence\n  orig   = {}\n  search = {}\n  value  = {}\n  C    -> {}\n  Rust -> {}",
                show(orig),
                show(search),
                show(value),
                show_opt(&got_c),
                show_opt(&got_r),
            );
        }
        got_c
    }
}

pub fn show(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &c in b {
        if c.is_ascii_graphic() || c == b' ' {
            s.push(c as char);
        } else {
            s.push_str(&format!("\\x{c:02x}"));
        }
    }
    s.push('"');
    format!("{s} (len {})", b.len())
}

pub fn show_opt(o: &Option<Vec<u8>>) -> String {
    match o {
        None => "NULL".to_string(),
        Some(v) => show(v),
    }
}

/// Deterministic xorshift64* PRNG — fixed seed keeps every randomized row
/// reproducible. Uses interior mutability so that nested calls such as
/// `r.word(r.range(1, 4), A2)` are ergonomic.
pub struct Rng(std::cell::Cell<u64>);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(std::cell::Cell::new(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        }))
    }
    pub fn next_u64(&self) -> u64 {
        let mut x = self.0.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0.set(x);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }
    /// Random string of `len` bytes drawn from `alphabet`.
    pub fn word(&self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len).map(|_| alphabet[self.below(alphabet.len())]).collect()
    }
    /// Random non-NUL byte string (every value in `0x01..=0xFF`).
    pub fn bytes(&self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.below(255) + 1) as u8).collect()
    }
    /// Random string of `len` bytes drawn from the high half of the byte range.
    pub fn high_bytes(&self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (0x80 + self.below(0x80)) as u8).collect()
    }
}

pub const SEED: u64 = 0x5EED_1234;

/// The reference semantics, written independently of both implementations
/// straight from `c_src/src/lib.c`, used as a third opinion on a subset of
/// rows so that a shared misunderstanding cannot hide.
pub fn model(orig: &[u8], search: &[u8], value: &[u8]) -> Vec<u8> {
    assert!(!search.is_empty(), "model is undefined for an empty needle");
    let find = |from: usize| -> Option<usize> {
        if search.len() > orig.len() {
            return None;
        }
        (from..=orig.len() - search.len()).find(|&i| &orig[i..i + search.len()] == search)
    };
    let first = match find(0) {
        None => return orig.to_vec(),
        Some(i) => i,
    };
    let mut out: Vec<u8> = orig[..first].to_vec();
    let mut inx_start = first;
    let mut from = first + search.len();
    let mut p = Some(first);
    while p.is_some() {
        out.extend_from_slice(value);
        p = find(inx_start + search.len());
        if let Some(i2) = p {
            if i2 > from {
                out.extend_from_slice(&orig[from..i2]);
            }
            inx_start = i2;
        }
        from = inx_start + search.len();
    }
    if from < orig.len() && from > 0 {
        out.extend_from_slice(&orig[from..]);
    }
    out
}
