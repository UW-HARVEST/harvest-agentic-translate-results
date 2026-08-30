//! Differential tests: load the C shared library and the Rust `cdylib` through
//! `libloading` and compare their observable behaviour byte-for-byte.
//!
//! Nothing here calls the Rust crate directly - every call goes through the
//! `#[no_mangle]` exports of `libconvert_pix_lib.so`, exactly like an external
//! C caller would.

// Each integration-test binary compiles this module separately and uses a
// different subset of it.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

type ConvertPixFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8, *mut CpPixel);
type CpInflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub convert_pix: ConvertPixFn,
    pub cp_inflate: CpInflateFn,
    pub cp_error_reason: *mut *const c_char,
    pub cp_fixed_table: *mut u8,
    pub cp_permutation_order: *mut u8,
    pub cp_len_extra_bits: *mut u8,
    pub cp_len_base: *mut u32,
    pub cp_dist_extra_bits: *mut u8,
    pub cp_dist_base: *mut u32,
    _lib: libloading::Library,
}

impl Lib {
    fn open(name: &'static str, path: &Path) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
            macro_rules! sym {
                ($t:ty, $s:expr) => {{
                    let s: libloading::Symbol<$t> = lib
                        .get($s)
                        .unwrap_or_else(|e| panic!("{}: missing symbol {:?}: {e}", name, $s));
                    *s
                }};
            }
            let convert_pix = sym!(ConvertPixFn, b"convert_pix\0");
            let cp_inflate = sym!(CpInflateFn, b"cp_inflate\0");
            let cp_error_reason = sym!(*mut *const c_char, b"cp_error_reason\0");
            let cp_fixed_table = sym!(*mut u8, b"cp_fixed_table\0");
            let cp_permutation_order = sym!(*mut u8, b"cp_permutation_order\0");
            let cp_len_extra_bits = sym!(*mut u8, b"cp_len_extra_bits\0");
            let cp_len_base = sym!(*mut u32, b"cp_len_base\0");
            let cp_dist_extra_bits = sym!(*mut u8, b"cp_dist_extra_bits\0");
            let cp_dist_base = sym!(*mut u32, b"cp_dist_base\0");
            Lib {
                name,
                convert_pix,
                cp_inflate,
                cp_error_reason,
                cp_fixed_table,
                cp_permutation_order,
                cp_len_extra_bits,
                cp_len_base,
                cp_dist_extra_bits,
                cp_dist_base,
                _lib: lib,
            }
        }
    }

    /// `cp_error_reason` as an owned byte string (`None` when NULL).
    pub fn error_reason(&self) -> Option<Vec<u8>> {
        unsafe {
            let p = *self.cp_error_reason;
            if p.is_null() {
                None
            } else {
                Some(CStr::from_ptr(p).to_bytes().to_vec())
            }
        }
    }

    pub fn clear_error(&self) {
        unsafe { *self.cp_error_reason = std::ptr::null() }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

// The libraries are only ever touched while holding `libs()`'s mutex.
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut found = None;
    for e in std::fs::read_dir(dir).ok()? {
        let p = e.ok()?.path();
        if p.extension().map(|e| e == "so").unwrap_or(false) {
            found = Some(p);
        }
    }
    found
}

fn c_so_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    find_so(&build).unwrap_or_else(|| {
        panic!(
            "no .so in {} - build the C library first (cmake .. && cmake --build .)",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    // Explicit override (used by verify_all.sh when sweeping configurations).
    if let Ok(p) = std::env::var("CONVERT_PIX_SO") {
        return PathBuf::from(p);
    }
    // target/<profile>/deps/<test-binary> -> target/<profile>/libconvert_pix_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();
    let direct = profile_dir.join("libconvert_pix_lib.so");
    if direct.exists() {
        return direct;
    }
    // `cargo test` only builds the lib target as a *test* binary; the cdylib
    // artifact itself is produced by `cargo build`.  Do that now so a bare
    // `cargo test` works from a clean target directory.  Cargo serialises
    // concurrent invocations through its own build lock.
    let profile = profile_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let target_dir = profile_dir.parent().expect("target dir");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest_dir().join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target_dir);
    if profile == "release" {
        cmd.arg("--release");
    }
    if let Ok(feats) = std::env::var("CONVERT_PIX_FEATURES") {
        cmd.arg("--no-default-features");
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats);
        }
    }
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn cargo build --lib: {e}"));
    assert!(status.success(), "cargo build --lib failed");
    assert!(
        direct.exists(),
        "{} still missing after `cargo build --lib`",
        direct.display()
    );
    direct
}

static LIBS: OnceLock<Mutex<Pair>> = OnceLock::new();

/// Serialises access: both libraries expose mutable globals (`cp_error_reason`).
pub fn libs() -> MutexGuard<'static, Pair> {
    LIBS.get_or_init(|| {
        Mutex::new(Pair {
            c: Lib::open("C", &c_so_path()),
            rs: Lib::open("Rust", &rust_so_path()),
        })
    })
    .lock()
    .unwrap()
}

// ---------------------------------------------------------------------------
// deterministic pseudo random data (no external crates)
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u32() >> 7) as u8
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

// ---------------------------------------------------------------------------
// vector manifest
// ---------------------------------------------------------------------------
pub struct Vector {
    pub name: String,
    pub data: Vec<u8>,
    pub raw_len: usize,
}

pub fn vectors() -> Vec<Vector> {
    let dir = manifest_dir().join("tests/data");
    let manifest = dir.join("manifest.txt");
    let text = std::fs::read_to_string(&manifest).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}) - run `python3 tests/gen_vectors.py`",
            manifest.display()
        )
    });
    let mut out = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let name = match it.next() {
            Some(n) => n.to_string(),
            None => continue,
        };
        let _dlen: usize = it.next().unwrap().parse().unwrap();
        let raw_len: usize = it.next().unwrap().parse().unwrap();
        let data = std::fs::read(dir.join(format!("{name}.deflate"))).unwrap();
        out.push(Vector {
            name,
            data,
            raw_len,
        });
    }
    assert!(!out.is_empty(), "no test vectors");
    out
}

// ---------------------------------------------------------------------------
// aligned input buffer helper
// ---------------------------------------------------------------------------

/// Holds `data` at a chosen `addr % 4` so both libraries see the *same*
/// pointer (and therefore the same `first_bytes` split inside `cp_inflate`).
pub struct InBuf {
    storage: Vec<u8>,
    offset: usize,
    len: usize,
}

impl InBuf {
    pub fn new(data: &[u8], want_align: usize) -> InBuf {
        // 8 bytes of slack in front (for alignment) and 8 behind so that the
        // `first_bytes` prologue read in cp_inflate never leaves the block.
        let mut storage = vec![0u8; data.len() + 32];
        let base = storage.as_ptr() as usize;
        let mut offset = 8;
        while (base + offset) % 4 != want_align % 4 {
            offset += 1;
        }
        storage[offset..offset + data.len()].copy_from_slice(data);
        InBuf {
            storage,
            offset,
            len: data.len(),
        }
    }

    pub fn ptr(&mut self) -> *mut c_void {
        unsafe { self.storage.as_mut_ptr().add(self.offset) as *mut c_void }
    }

    pub fn len(&self) -> c_int {
        self.len as c_int
    }
}
