//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls their exported symbols
//! through the FFI boundary. The Rust implementation is NEVER called directly —
//! always through `dlopen`/`dlsym` on `libdriver.so`, exactly as an external
//! C consumer would, so the `#[no_mangle] extern "C"` wrappers are under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

unsafe extern "C" {
    fn free(p: *mut std::ffi::c_void);
}

type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
// `bool` in the C signature; taken as u8 so we can also feed non-canonical bytes.
type FilterFn = unsafe extern "C" fn(*const c_char, u8) -> *mut c_char;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        Impl { name, path, lib }
    }

    fn drop_fn(&self) -> Symbol<'_, DropFn> {
        unsafe { self.lib.get(b"w_utf8_drop\0") }
            .unwrap_or_else(|e| panic!("{}: dlsym(w_utf8_drop): {e}", self.name))
    }

    fn filter_fn(&self) -> Symbol<'_, FilterFn> {
        unsafe { self.lib.get(b"w_utf8_filter\0") }
            .unwrap_or_else(|e| panic!("{}: dlsym(w_utf8_filter): {e}", self.name))
    }

    /// Calls `w_utf8_drop` and returns the byte offset of the returned pointer.
    /// `input` must be NUL-terminated.
    pub fn drop_offset(&self, input: &[u8]) -> usize {
        assert_eq!(input.last(), Some(&0), "input must be NUL terminated");
        let f = self.drop_fn();
        unsafe {
            let base = input.as_ptr() as *const c_char;
            let r = f(base);
            assert!(!r.is_null(), "{}: w_utf8_drop returned NULL", self.name);
            let off = r.offset_from(base);
            assert!(
                off >= 0 && (off as usize) < input.len(),
                "{}: w_utf8_drop returned out-of-range offset {off} (len {})",
                self.name,
                input.len()
            );
            off as usize
        }
    }

    /// Calls `w_utf8_filter` and returns the NUL-terminated result as bytes
    /// (excluding the terminator), then `free()`s the C-allocated buffer.
    /// `None` means the function returned NULL.
    pub fn filter(&self, input: &[u8], replacement: u8) -> Option<Vec<u8>> {
        assert_eq!(input.last(), Some(&0), "input must be NUL terminated");
        let f = self.filter_fn();
        unsafe {
            let p = f(input.as_ptr() as *const c_char, replacement);
            if p.is_null() {
                return None;
            }
            let mut out = Vec::new();
            let mut q = p as *const u8;
            while *q != 0 {
                out.push(*q);
                q = q.add(1);
            }
            // The buffer comes from the same libc malloc/strdup in both
            // implementations, so the test process can free it. (CONFIGS C36)
            free(p.cast());
            Some(out)
        }
    }
}

fn target_dir() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target dir")
        .to_path_buf()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    // Prefer the cdylib built for the same profile as this test binary.
    let mut candidates = vec![target_dir().join("libdriver.so")];
    let root = manifest_dir().join("target");
    for prof in ["debug", "release"] {
        candidates.push(root.join(prof).join("libdriver.so"));
    }
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    panic!(
        "Rust cdylib not found (looked in {:?}). Build it first with `cargo build --offline`.",
        candidates
    );
}

struct Pair {
    c: Impl,
    rust: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", c_so_path()),
        rust: Impl::load("RUST", rust_so_path()),
    })
}

pub fn c() -> &'static Impl {
    &pair().c
}

pub fn rust() -> &'static Impl {
    &pair().rust
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X} ")).collect()
}

/// Differential check of `w_utf8_drop` for one input (NUL terminator appended).
#[track_caller]
pub fn check_drop(body: &[u8]) {
    let mut s = body.to_vec();
    s.push(0);
    let a = c().drop_offset(&s);
    let b = rust().drop_offset(&s);
    assert_eq!(
        a,
        b,
        "w_utf8_drop divergence: input=[{}] C offset={a} RUST offset={b}",
        hex(body)
    );
}

/// Differential check of `w_utf8_filter` for one input and one flag byte.
#[track_caller]
pub fn check_filter(body: &[u8], replacement: u8) {
    let mut s = body.to_vec();
    s.push(0);
    let a = c().filter(&s, replacement);
    let b = rust().filter(&s, replacement);
    match (&a, &b) {
        (Some(x), Some(y)) => assert_eq!(
            x,
            y,
            "w_utf8_filter divergence: input=[{}] replacement={replacement}\n  C   =[{}]\n  RUST=[{}]",
            hex(body),
            hex(x),
            hex(y)
        ),
        _ => assert!(
            a.is_none() == b.is_none(),
            "w_utf8_filter NULL-ness divergence: input=[{}] replacement={replacement} C_null={} RUST_null={}",
            hex(body),
            a.is_none(),
            b.is_none()
        ),
    }
}

/// Both entry points, both canonical flag values.
#[track_caller]
pub fn check_all(body: &[u8]) {
    check_drop(body);
    check_filter(body, 0);
    check_filter(body, 1);
}

/// Deterministic xorshift64* PRNG — fixed seed, so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.range(0, xs.len() - 1)]
    }
}

/// Push a random *valid* UTF-8 sequence of the requested byte width, using the
/// same acceptance rules as the C macros (so it lands inside `valid_N`).
pub fn push_valid(rng: &mut Rng, out: &mut Vec<u8>, width: u8) {
    match width {
        1 => out.push(rng.range(0x01, 0x7F) as u8), // never 0: that ends the string
        2 => {
            out.push(rng.range(0xC2, 0xDF) as u8);
            out.push(rng.range(0x80, 0xBF) as u8);
        }
        3 => {
            let lead = rng.pick(&[0xE0u8, 0xE1, 0xE7, 0xEC, 0xED, 0xEE, 0xEF]);
            let b1 = match lead {
                0xE0 => rng.range(0xA0, 0xBF),
                0xED => rng.range(0x80, 0x9F),
                _ => rng.range(0x80, 0xBF),
            } as u8;
            out.push(lead);
            out.push(b1);
            out.push(rng.range(0x80, 0xBF) as u8);
        }
        4 => {
            let lead = rng.pick(&[0xF0u8, 0xF1, 0xF3, 0xF4]);
            let b1 = match lead {
                0xF0 => rng.range(0x90, 0xBF),
                0xF4 => rng.range(0x80, 0x8F),
                _ => rng.range(0x80, 0xBF),
            } as u8;
            out.push(lead);
            out.push(b1);
            out.push(rng.range(0x80, 0xBF) as u8);
            out.push(rng.range(0x80, 0xBF) as u8);
        }
        _ => unreachable!(),
    }
}

/// A byte that starts no valid sequence at all.
pub fn invalid_lead(rng: &mut Rng) -> u8 {
    let choices: [u8; 5] = [0x80, 0xBF, 0xC0, 0xC1, 0xF5];
    let extra = rng.range(0, 2);
    match extra {
        0 => rng.pick(&choices),
        1 => rng.range(0x80, 0xBF) as u8,
        _ => rng.pick(&[0xF6u8, 0xF7, 0xF8, 0xFB, 0xFD, 0xFE, 0xFF]),
    }
}

/// `push_valid` with a randomly chosen width (avoids double-borrowing `rng`).
pub fn push_valid_rand(rng: &mut Rng, out: &mut Vec<u8>) {
    let w = rng.range(1, 4) as u8;
    push_valid(rng, out, w);
}
