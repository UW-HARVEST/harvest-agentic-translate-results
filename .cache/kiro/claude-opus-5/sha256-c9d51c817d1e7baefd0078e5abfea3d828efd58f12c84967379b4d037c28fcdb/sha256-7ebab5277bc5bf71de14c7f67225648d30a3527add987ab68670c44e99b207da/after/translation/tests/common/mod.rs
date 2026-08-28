//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes the exported C ABI symbols of each.
//!
//! Nothing in here calls a Rust function directly -- every call goes through
//! the dynamic-library boundary, exactly like an external C caller, so the
//! `#[no_mangle] extern "C"` wrappers are exercised too.

#![allow(dead_code)]

pub mod deflate;

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub unfilter: unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int,
    pub cp_inflate: unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int,
    pub cp_error_reason: *mut *const c_char,
    pub cp_fixed_table: *mut u8,
    pub cp_permutation_order: *mut u8,
    pub cp_len_extra_bits: *mut u8,
    pub cp_len_base: *mut u32,
    pub cp_dist_extra_bits: *mut u8,
    pub cp_dist_base: *mut u32,
}

unsafe fn data_sym(lib: &libloading::Library, name: &[u8]) -> *mut u8 {
    // `Symbol<T>` derefs by reinterpreting the stored symbol address as `T`,
    // so `T = *mut u8` yields the address of the data object itself.
    let sym: libloading::Symbol<*mut u8> = unsafe {
        lib.get(name)
            .unwrap_or_else(|e| panic!("missing data symbol {}: {e}", String::from_utf8_lossy(name)))
    };
    *sym
}

impl Lib {
    fn open(name: &'static str, path: &Path) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));

            let unfilter: libloading::Symbol<
                unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int,
            > = lib.get(b"unfilter").expect("missing `unfilter`");
            let unfilter = *unfilter;

            let cp_inflate: libloading::Symbol<
                unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int,
            > = lib.get(b"cp_inflate").expect("missing `cp_inflate`");
            let cp_inflate = *cp_inflate;

            let cp_error_reason = data_sym(&lib, b"cp_error_reason") as *mut *const c_char;
            let cp_fixed_table = data_sym(&lib, b"cp_fixed_table");
            let cp_permutation_order = data_sym(&lib, b"cp_permutation_order");
            let cp_len_extra_bits = data_sym(&lib, b"cp_len_extra_bits");
            let cp_len_base = data_sym(&lib, b"cp_len_base") as *mut u32;
            let cp_dist_extra_bits = data_sym(&lib, b"cp_dist_extra_bits");
            let cp_dist_base = data_sym(&lib, b"cp_dist_base") as *mut u32;

            Lib {
                name,
                _lib: lib,
                unfilter,
                cp_inflate,
                cp_error_reason,
                cp_fixed_table,
                cp_permutation_order,
                cp_len_extra_bits,
                cp_len_base,
                cp_dist_extra_bits,
                cp_dist_base,
            }
        }
    }

    /// Current value of `cp_error_reason` as bytes (`None` when NULL).
    pub fn error_reason(&self) -> Option<Vec<u8>> {
        unsafe {
            let p = *self.cp_error_reason;
            if p.is_null() {
                None
            } else {
                Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec())
            }
        }
    }

    pub fn clear_error_reason(&self) {
        unsafe { *self.cp_error_reason = std::ptr::null() }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

// The raw pointers are addresses of process-global data objects inside the two
// dlopen'd libraries; they stay valid for the lifetime of the process. The
// state they reference (`cp_error_reason`) is guarded by `ERR_LOCK`.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

/// `cp_error_reason` is a process-global; tests that inspect it must serialise.
pub static ERR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let build = repo_root().join("c_src/build");
    let mut found = None;
    for entry in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("c_src/build missing ({e}); build the C library first"))
    {
        let p = entry.unwrap().path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

/// Build the Rust `cdylib` into a dedicated target directory (so we never
/// contend on the target-dir lock held by the outer `cargo test`) and return
/// the resulting `.so`.
fn rust_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = manifest.join("target/harness-so");

    let features = std::env::var("HARNESS_FEATURES").unwrap_or_default();
    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.current_dir(&manifest)
        .env("CARGO_TARGET_DIR", &target)
        .env_remove("RUSTFLAGS")
        .args(["build", "--release", "--lib"]);
    if !features.is_empty() || std::env::var("HARNESS_NO_DEFAULT").is_ok() {
        cmd.arg("--no-default-features");
        if !features.is_empty() {
            cmd.args(["--features", &features]);
        }
    }
    let out = cmd.output().expect("failed to spawn cargo build for cdylib");
    assert!(
        out.status.success(),
        "cargo build --release --lib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let so = target.join("release/libunfilter_lib.so");
    assert!(so.exists(), "expected cdylib at {}", so.display());
    so
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Lib::open("c", &c_so()),
        rust: Lib::open("rust", &rust_so()),
    })
}

// ---------------------------------------------------------------------------
// Aligned scratch buffers
// ---------------------------------------------------------------------------

/// Heap buffer whose base address is 16-byte aligned, so `base + k` gives a
/// precisely controlled alignment (`k mod 4` drives `cp_inflate`'s
/// `first_bytes` computation).
pub struct Aligned {
    ptr: *mut u8,
    layout: std::alloc::Layout,
    len: usize,
}

impl Aligned {
    pub fn new(len: usize) -> Aligned {
        let len = len.max(1);
        let layout = std::alloc::Layout::from_size_align(len, 16).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null());
        Aligned { ptr, layout, len }
    }

    pub fn from_slice_at(data: &[u8], offset: usize) -> Aligned {
        let b = Aligned::new(offset + data.len() + 16);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), b.ptr.add(offset), data.len());
        }
        b
    }

    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn at(&self, offset: usize) -> *mut u8 {
        assert!(offset <= self.len);
        unsafe { self.ptr.add(offset) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn fill(&mut self, data: &[u8]) {
        assert!(data.len() <= self.len);
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr, data.len()) };
    }
}

impl Drop for Aligned {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) }
    }
}

/// Deterministic xorshift PRNG so failures are reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        (self.next_u64() >> 11) % n
    }
}

pub fn hexdiff(a: &[u8], b: &[u8]) -> String {
    let mut s = String::new();
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            s.push_str(&format!("  [{i}] c=0x{x:02x} rust=0x{y:02x}\n"));
            if s.len() > 2000 {
                s.push_str("  ...\n");
                break;
            }
        }
    }
    if a.len() != b.len() {
        s.push_str(&format!("  length {} vs {}\n", a.len(), b.len()));
    }
    s
}
