//! Differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and calls
//! every function across the FFI boundary, exactly as an external consumer
//! would. The Rust implementation is NEVER called directly, so the
//! `#[no_mangle] extern "C"` export wrappers are under test too.
//!
//! ## Why every instance is loaded from a private copy of the `.so`
//!
//! `lib.c` keeps three mutable `static`s (`accumulator`, `multiplier`,
//! `operation_count`). `dlopen` de-duplicates by resolved path, so re-opening
//! the same file hands back the *same* already-initialised state. To obtain a
//! genuinely FRESH library (statics back at `0 / 1 / 0`) the harness copies
//! each `.so` to a unique temporary path and opens that copy.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C shared library produced by `c_src/CMakeLists.txt`. The CMake project
/// name is derived from the parent directory name, so the file name is not
/// fixed; glob the build directory instead of hard-coding it.
fn c_so_path() -> PathBuf {
    let build_dir = crate_root().join("../c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\n\
                 Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("lib"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build_dir.display(),
        candidates
    );
    candidates.pop().unwrap()
}

/// The Rust `cdylib`. Prefer the release artifact (that is the one with
/// `panic = "abort"`, i.e. what actually ships); fall back to debug.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libfindrep_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libfindrep_lib.so not found; run `cargo build --release` first");
}

// ---------------------------------------------------------------------------
// Unique private copies
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch_dir() -> PathBuf {
    let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let d = PathBuf::from(base).join("findrep-difftest");
    std::fs::create_dir_all(&d).expect("cannot create scratch dir");
    d
}

fn unique_copy(src: &Path, tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let dst = scratch_dir().join(format!("{tag}-{pid}-{n}.so"));
    std::fs::copy(src, &dst).unwrap_or_else(|e| {
        panic!(
            "copy {} -> {}: {e}\n\
             (if this is ENOSPC, the scratch dir failed to self-clean)",
            src.display(),
            dst.display()
        )
    });
    dst
}

/// The `.so` copies exist only so that `dlopen` treats each one as a distinct
/// object (fresh `static`s). Once `dlopen` has mapped the file, the directory
/// entry is no longer needed: on Linux the mapping keeps the inode alive.
/// Unlinking immediately means the scratch dir never grows, even if a test
/// panics -- tests create tens of thousands of these, so leaking them fills
/// the filesystem and makes later configurations fail spuriously.
fn unlink_now(p: &Path) {
    let _ = std::fs::remove_file(p);
}

// ---------------------------------------------------------------------------
// One loaded implementation
// ---------------------------------------------------------------------------

type Fn2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Fn1 = unsafe extern "C" fn(c_int) -> c_int;
type FnStr = unsafe extern "C" fn(*mut c_char, c_int);
type Fn4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Impl {
    name: &'static str,
    // `_lib` must outlive the extracted function pointers; it is dropped last.
    add_to_accumulator: Fn2,
    multiply_with_multiplier: Fn2,
    subtract_from_accumulator: Fn2,
    divide_multiplier: Fn2,
    process_octal_string: FnStr,
    find_and_replace_char: FnStr,
    validate_and_normalize: Fn1,
    findrep: Fn4,
    _lib: libloading::Library,
    _path: PathBuf,
}

impl Impl {
    fn open(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            // The mapping is established; drop the directory entry right away.
            unlink_now(&path);

            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let f: libloading::Symbol<$t> = lib
                        .get($s)
                        .unwrap_or_else(|e| panic!("{} missing {:?}: {e}", name, $s));
                    *f
                }};
            }

            let add_to_accumulator = sym!(Fn2, b"add_to_accumulator\0");
            let multiply_with_multiplier = sym!(Fn2, b"multiply_with_multiplier\0");
            let subtract_from_accumulator = sym!(Fn2, b"subtract_from_accumulator\0");
            let divide_multiplier = sym!(Fn2, b"divide_multiplier\0");
            let process_octal_string = sym!(FnStr, b"process_octal_string\0");
            let find_and_replace_char = sym!(FnStr, b"find_and_replace_char\0");
            let validate_and_normalize = sym!(Fn1, b"validate_and_normalize\0");
            let findrep = sym!(Fn4, b"findrep\0");

            Impl {
                name,
                add_to_accumulator,
                multiply_with_multiplier,
                subtract_from_accumulator,
                divide_multiplier,
                process_octal_string,
                find_and_replace_char,
                validate_and_normalize,
                findrep,
                _lib: lib,
                _path: path,
            }
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn add(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.add_to_accumulator)(a, b) }
    }
    pub fn mul(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.multiply_with_multiplier)(a, b) }
    }
    pub fn sub(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.subtract_from_accumulator)(a, b) }
    }
    pub fn div(&self, a: i32, b: i32) -> i32 {
        unsafe { (self.divide_multiplier)(a, b) }
    }
    pub fn normalize(&self, v: i32) -> i32 {
        unsafe { (self.validate_and_normalize)(v) }
    }
    pub fn findrep(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe { (self.findrep)(a, b, c, d) }
    }

    /// `process_octal_string` into a fixed 64-byte zeroed buffer; the WHOLE
    /// buffer is returned so trailing bytes are compared too.
    pub fn octal(&self, v: i32) -> Vec<u8> {
        let mut buf = [0u8; 64];
        unsafe { (self.process_octal_string)(buf.as_mut_ptr() as *mut c_char, v) };
        buf.to_vec()
    }

    /// `find_and_replace_char` on a copy of `bytes` (which must contain a NUL).
    /// The whole buffer is returned, including anything past the terminator.
    pub fn replace(&self, bytes: &[u8], ch: i32) -> Vec<u8> {
        assert!(
            bytes.contains(&0),
            "harness misuse: buffer must be NUL-terminated"
        );
        let mut buf = bytes.to_vec();
        unsafe { (self.find_and_replace_char)(buf.as_mut_ptr() as *mut c_char, ch) };
        buf
    }
}

// ---------------------------------------------------------------------------
// A matched C + Rust pair
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

impl Pair {
    /// A pair of libraries with FRESH static state
    /// (`accumulator=0, multiplier=1, operation_count=0`).
    pub fn fresh() -> Pair {
        let c = Impl::open("C", unique_copy(&c_so_path(), "c"));
        let rust = Impl::open("Rust", unique_copy(&rust_so_path(), "rust"));
        Pair { c, rust }
    }
}

/// Assert the two implementations agree, with a rich failure message.
#[track_caller]
pub fn same<T: PartialEq + std::fmt::Debug>(what: &str, c: T, rust: T) {
    assert!(
        c == rust,
        "DIVERGENCE in {what}\n  C    = {c:?}\n  Rust = {rust:?}"
    );
}

#[track_caller]
pub fn same_bytes(what: &str, c: &[u8], rust: &[u8]) {
    if c != rust {
        panic!(
            "DIVERGENCE in {what}\n  C    = {:?} ({:?})\n  Rust = {:?} ({:?})",
            String::from_utf8_lossy(c),
            c,
            String::from_utf8_lossy(rust),
            rust
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style tests)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Avoid the zero state of xorshift64*.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 40) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Pick a value from `validate_and_normalize` bucket `b`:
    /// 0=Z 1=N 2=L(1..63) 3=P(64..511) 4=U(>=512)
    pub fn bucket(&mut self, b: u32) -> i32 {
        match b {
            0 => 0,
            1 => {
                if self.bool() {
                    self.i32_in(i32::MIN, -1)
                } else {
                    self.i32_in(-1000, -1)
                }
            }
            2 => self.i32_in(1, 63),
            3 => self.i32_in(64, 511),
            _ => {
                if self.bool() {
                    self.i32_in(512, i32::MAX)
                } else {
                    self.i32_in(512, 5000)
                }
            }
        }
    }
}

/// Interesting integer values every numeric test should include.
pub const EDGE_I32: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    -100000,
    -512,
    -511,
    -64,
    -63,
    -2,
    -1,
    0,
    1,
    2,
    7,
    8,
    63,
    64,
    65,
    83, // 0o123, the literal findrep passes to process_octal_string
    104, // 0o150, the accumulator guard
    510,
    511,
    512,
    513,
    100000,
    i32::MAX - 1,
    i32::MAX,
];

/// Build a NUL-terminated buffer from `s`, padded to `total` bytes with
/// non-zero filler so that writes past the terminator are detectable.
pub fn buf_from(s: &[u8], total: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(total);
    v.extend_from_slice(s);
    v.push(0);
    while v.len() < total {
        v.push(0xAB);
    }
    v
}
