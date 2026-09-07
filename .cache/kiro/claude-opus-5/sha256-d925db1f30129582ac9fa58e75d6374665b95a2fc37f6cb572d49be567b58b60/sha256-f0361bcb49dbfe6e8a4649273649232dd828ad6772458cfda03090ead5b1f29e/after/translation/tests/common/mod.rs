//! Shared differential-testing harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and calls
//! every function purely through its exported C symbol — the Rust side is
//! never called directly, so the `#[no_mangle] extern "C"` wrappers are part
//! of what is under test.
//!
//! Both libraries keep mutable file-scope state (`accumulator`, `multiplier`,
//! `operation_count`), so each test needs a *pristine* pair of instances.
//! `dlopen` refcounts by path, which would silently share state between
//! tests; we therefore copy each `.so` to a unique temporary path before
//! loading it. A distinct path yields a distinct mapping with fresh statics.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

type FnIntInt = unsafe extern "C" fn(i32, i32) -> i32;
type FnInt = unsafe extern "C" fn(i32) -> i32;
type FnPtrInt = unsafe extern "C" fn(*mut std::ffi::c_char, i32);
type FnFindrep = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop()
        .unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so the cdylib is one
    // directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    for name in ["libfindrep_lib.so", "libharvest_work_iihEGb.so"] {
        let p = profile_dir.join(name);
        if p.exists() {
            return p;
        }
    }
    // Fall back to whatever cdylib is present.
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&profile_dir)
        .expect("read profile dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    hits.sort();
    hits.pop().unwrap_or_else(|| {
        panic!(
            "no Rust cdylib in {} — run `cargo build` first",
            profile_dir.display()
        )
    })
}

/// One freshly-loaded pair of implementations with independent static state.
pub struct Pair {
    pub c: Library,
    pub r: Library,
    tmp: PathBuf,
}

impl Drop for Pair {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.tmp);
    }
}

impl Pair {
    /// Loads a brand-new instance of each library (pristine statics).
    pub fn fresh() -> Pair {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!(
            "findrep_diff_{}_{}_{}",
            std::process::id(),
            n,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).expect("create temp dir");

        let c_dst = tmp.join(format!("libc_impl_{n}.so"));
        let r_dst = tmp.join(format!("librs_impl_{n}.so"));
        std::fs::copy(find_c_so(), &c_dst).expect("copy C .so");
        std::fs::copy(find_rust_so(), &r_dst).expect("copy Rust .so");

        let c = unsafe { Library::new(&c_dst) }.expect("dlopen C .so");
        let r = unsafe { Library::new(&r_dst) }.expect("dlopen Rust .so");
        Pair { c, r, tmp }
    }

    fn sym2(&self, lib: &Library, name: &str) -> FnIntInt {
        unsafe {
            let s: Symbol<FnIntInt> = lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {name}: {e}"));
            *s
        }
    }

    // --- one wrapper per exported symbol, per side ---------------------------

    pub fn c_add(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.sym2(&self.c, "add_to_accumulator"))(a, b) }
    }
    pub fn r_add(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.sym2(&self.r, "add_to_accumulator"))(a, b) }
    }

    pub fn c_mul(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.sym2(&self.c, "multiply_with_multiplier"))(a, b) }
    }
    pub fn r_mul(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.sym2(&self.r, "multiply_with_multiplier"))(a, b) }
    }

    pub fn c_sub(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.sym2(&self.c, "subtract_from_accumulator"))(a, b) }
    }
    pub fn r_sub(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.sym2(&self.r, "subtract_from_accumulator"))(a, b) }
    }

    pub fn c_div(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.sym2(&self.c, "divide_multiplier"))(a, b) }
    }
    pub fn r_div(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.sym2(&self.r, "divide_multiplier"))(a, b) }
    }

    pub fn c_validate(&self, v: i32) -> i32 {
        unsafe {
            let s: Symbol<FnInt> = self.c.get(b"validate_and_normalize").unwrap();
            (*s)(v)
        }
    }
    pub fn r_validate(&self, v: i32) -> i32 {
        unsafe {
            let s: Symbol<FnInt> = self.r.get(b"validate_and_normalize").unwrap();
            (*s)(v)
        }
    }

    pub fn c_findrep(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe {
            let s: Symbol<FnFindrep> = self.c.get(b"findrep").unwrap();
            (*s)(a, b, c, d)
        }
    }
    pub fn r_findrep(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe {
            let s: Symbol<FnFindrep> = self.r.get(b"findrep").unwrap();
            (*s)(a, b, c, d)
        }
    }

    /// Runs `process_octal_string` into a poisoned 128-byte buffer and returns
    /// the whole buffer, so trailing bytes / missing NUL are caught too.
    pub fn c_octal(&self, v: i32) -> Vec<u8> {
        octal_into(&self.c, v)
    }
    pub fn r_octal(&self, v: i32) -> Vec<u8> {
        octal_into(&self.r, v)
    }

    /// Runs `find_and_replace_char` over `s` (NUL-terminated inside a poisoned
    /// buffer) and returns the whole buffer.
    pub fn c_find_replace(&self, s: &[u8], needle: i32) -> Vec<u8> {
        find_replace_into(&self.c, s, needle)
    }
    pub fn r_find_replace(&self, s: &[u8], needle: i32) -> Vec<u8> {
        find_replace_into(&self.r, s, needle)
    }
}

const BUF: usize = 512;
const POISON: u8 = 0xAB;

fn octal_into(lib: &Library, v: i32) -> Vec<u8> {
    let mut buf = vec![POISON; BUF];
    unsafe {
        let s: Symbol<FnPtrInt> = lib.get(b"process_octal_string").unwrap();
        (*s)(buf.as_mut_ptr() as *mut std::ffi::c_char, v);
    }
    buf
}

fn find_replace_into(lib: &Library, src: &[u8], needle: i32) -> Vec<u8> {
    assert!(src.len() < BUF, "test string too long");
    let mut buf = vec![POISON; BUF];
    buf[..src.len()].copy_from_slice(src);
    buf[src.len()] = 0;
    unsafe {
        let s: Symbol<FnPtrInt> = lib.get(b"find_and_replace_char").unwrap();
        (*s)(buf.as_mut_ptr() as *mut std::ffi::c_char, needle);
    }
    buf
}

/// Deterministic xorshift64* PRNG — fixed seed keeps every run reproducible.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.below(span) as i64)
    }
    /// A value biased toward the interesting classes the C code branches on.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIALS: [i32; 20] = [
            i32::MIN,
            i32::MIN + 1,
            -1000,
            -512,
            -511,
            -64,
            -1,
            0,
            1,
            2,
            63,
            64,
            65,
            83,
            104,
            510,
            511,
            512,
            1000,
            i32::MAX,
        ];
        match self.below(3) {
            0 => SPECIALS[self.below(SPECIALS.len() as u64) as usize],
            1 => self.range(-1024, 1024) as i32,
            _ => self.next_i32(),
        }
    }
}

/// Random printable-ASCII string of length `len` (no NUL bytes).
pub fn rand_ascii(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| (rng.range(0x20, 0x7E)) as u8)
        .collect()
}
