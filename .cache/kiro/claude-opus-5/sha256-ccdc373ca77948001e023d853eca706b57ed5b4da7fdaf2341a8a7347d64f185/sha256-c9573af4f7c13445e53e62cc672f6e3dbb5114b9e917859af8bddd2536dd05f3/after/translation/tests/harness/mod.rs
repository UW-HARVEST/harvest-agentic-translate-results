//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` and driven **only** through
//! their exported symbols (`array`, `perform_expensive_operations`,
//! `long_exec`).  Nothing in this module calls a Rust function from the crate
//! directly, so the `#[no_mangle]` export wrappers and the exported `.bss`
//! object are part of what is under test.
//!
//! Discovery:
//!   * C  `.so`: `$LONG_C_SO`, else `<workspace>/c_src/build/liblong.so`
//!   * Rust `.so`: `$LONG_RUST_SO`, else `liblong.so` next to the test binary's
//!     target profile directory (so it matches the feature set `cargo test` was
//!     invoked with).
//!
//! Both libraries own a mutable 1 MiB global, so every access is serialised
//! through `LOCK`.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const ARRAY_SIZE: usize = 256 * 1024;

/// Serialises access to the two libraries' mutable globals.
static LOCK: Mutex<()> = Mutex::new(());

pub fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Directory containing this crate (`translation/`).
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory holding the pre-computed C reference outputs.
pub fn reference_dir() -> PathBuf {
    manifest_dir().join("tests/reference")
}

fn target_profile_dir() -> PathBuf {
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("LONG_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/liblong.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("LONG_RUST_SO") {
        return PathBuf::from(p);
    }
    target_profile_dir().join("liblong.so")
}

/// One loaded shared object plus resolved symbols.
pub struct Lib {
    _lib: Library,
    pub name: &'static str,
    array: *mut i32,
    pxo: unsafe extern "C" fn(),
    exec: unsafe extern "C" fn(u32),
}

// The pointers are into a dlopen'd image that stays mapped for the process
// lifetime; all use is serialised through `LOCK`.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

impl Lib {
    fn open(path: &PathBuf, name: &'static str) -> Lib {
        assert!(
            path.exists(),
            "{} shared object not found at {}\n\
             build it first:\n  \
             C:    cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
             Rust: cd translation && cargo build --release",
            name,
            path.display()
        );
        if name == "Rust" {
            assert_not_stale(path);
        }
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {e}", path));
            // For a data symbol we need the *address of the object* itself, not
            // the pointer-sized value stored there, so go through the raw
            // `dlsym` result rather than `Deref`.
            let arr: Symbol<*mut i32> = lib
                .get(b"array\0")
                .unwrap_or_else(|e| panic!("dlsym array in {:?}: {e}", path));
            let array = arr.into_raw().into_raw() as *mut i32;
            assert!(!array.is_null(), "dlsym array returned NULL in {:?}", path);
            let pxo: Symbol<unsafe extern "C" fn()> = lib
                .get(b"perform_expensive_operations\0")
                .unwrap_or_else(|e| panic!("dlsym perform_expensive_operations: {e}"));
            let pxo = *pxo;
            let exec: Symbol<unsafe extern "C" fn(u32)> = lib
                .get(b"long_exec\0")
                .unwrap_or_else(|e| panic!("dlsym long_exec: {e}"));
            let exec = *exec;
            Lib {
                _lib: lib,
                name,
                array,
                pxo,
                exec,
            }
        }
    }

    pub fn array(&self) -> &[i32] {
        unsafe { std::slice::from_raw_parts(self.array, ARRAY_SIZE) }
    }

    pub fn array_mut(&self) -> &mut [i32] {
        unsafe { std::slice::from_raw_parts_mut(self.array, ARRAY_SIZE) }
    }

    /// `perform_expensive_operations()` called `k` times in a row.
    pub fn pxo(&self, k: usize) {
        for _ in 0..k {
            unsafe { (self.pxo)() }
        }
    }

    /// `long_exec(seed)`; returns the exact bytes the library wrote to stdout.
    pub fn long_exec_capture(&self, seed: u32) -> Vec<u8> {
        capture_fd(1, || unsafe { (self.exec)(seed) })
    }

    /// `long_exec(seed)`; returns the exact bytes the library wrote to stderr.
    pub fn long_exec_capture_stderr(&self, seed: u32) -> Vec<u8> {
        capture_fd(2, || unsafe { (self.exec)(seed) })
    }

    pub fn long_exec(&self, seed: u32) {
        unsafe { (self.exec)(seed) }
    }
}

/// `cargo test` builds the *test* binaries but not necessarily the `cdylib`, so a
/// stale `liblong.so` would silently be tested instead of the current source.
/// Refuse to run in that case.
fn assert_not_stale(so: &PathBuf) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src = manifest_dir().join("src");
    let Ok(entries) = std::fs::read_dir(&src) else {
        return;
    };
    for e in entries.flatten() {
        if e.path().extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
            assert!(
                t <= so_mtime,
                "{} is older than {} -- run `cargo build --release` (with the same \
                 --features) before `cargo test`, or the tests would verify a stale library",
                so.display(),
                e.path().display()
            );
        }
    }
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c() -> &'static Lib {
    C_LIB.get_or_init(|| Lib::open(&c_so_path(), "C"))
}

pub fn rust() -> &'static Lib {
    RUST_LIB.get_or_init(|| Lib::open(&rust_so_path(), "Rust"))
}

/// Redirect `fd` (1 = stdout, 2 = stderr) to a temporary file for the duration of
/// `f`, then return everything that was written.  `printf`/`fprintf` inside the
/// loaded library share the process' libc stdio, and Rust's `eprintln!` writes to
/// fd 2 directly, so this captures both exactly.
pub fn capture_fd<F: FnOnce()>(fd: i32, f: F) -> Vec<u8> {
    unsafe {
        let mut tmpl: Vec<u8> = b"/tmp/long_diff_capture_XXXXXX\0".to_vec();
        let tmp_fd = libc::mkstemp(tmpl.as_mut_ptr() as *mut libc::c_char);
        assert!(tmp_fd >= 0, "mkstemp failed");
        let path = std::ffi::CStr::from_ptr(tmpl.as_ptr() as *const libc::c_char)
            .to_str()
            .unwrap()
            .to_owned();

        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(fd);
        assert!(saved >= 0, "dup({fd}) failed");
        assert!(libc::dup2(tmp_fd, fd) >= 0, "dup2 failed");

        f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, fd) >= 0, "dup2 restore failed");
        libc::close(saved);
        libc::close(tmp_fd);

        let bytes = std::fs::read(&path).expect("read captured fd");
        let _ = std::fs::remove_file(&path);
        bytes
    }
}

pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    capture_fd(1, f)
}

pub fn capture_stderr<F: FnOnce()>(f: F) -> Vec<u8> {
    capture_fd(2, f)
}

/// splitmix32 — byte-identical to the stream in `tools/runner.c`, so
/// in-process and out-of-process runs use the same inputs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

pub fn rand_fill(seed: u64) -> Vec<i32> {
    let mut r = Rng::new(seed);
    (0..ARRAY_SIZE).map(|_| r.next_i32()).collect()
}

/// Non-negative shape, i.e. what `rand()` itself produces (`0 ..= RAND_MAX`).
pub fn rand_fill_nonneg(seed: u64) -> Vec<i32> {
    let mut r = Rng::new(seed);
    (0..ARRAY_SIZE)
        .map(|_| (r.next_u32() & 0x7fff_ffff) as i32)
        .collect()
}

/// Load `input` into both libraries, call `perform_expensive_operations` `k`
/// times in each, and assert the resulting 1 MiB arrays are byte-identical.
pub fn diff_pxo(row: &str, input: &[i32], k: usize) {
    assert_eq!(input.len(), ARRAY_SIZE);
    let _g = lock();
    let cl = c();
    let rl = rust();
    cl.array_mut().copy_from_slice(input);
    rl.array_mut().copy_from_slice(input);
    cl.pxo(k);
    rl.pxo(k);
    assert_arrays_eq(row, k, input, cl.array(), rl.array());
}

pub fn assert_arrays_eq(row: &str, k: usize, input: &[i32], ca: &[i32], ra: &[i32]) {
    if ca == ra {
        return;
    }
    let i = ca
        .iter()
        .zip(ra.iter())
        .position(|(a, b)| a != b)
        .expect("arrays differ but no differing index found");
    let ndiff = ca.iter().zip(ra.iter()).filter(|(a, b)| a != b).count();
    panic!(
        "[{row}] divergence after k={k} perform_expensive_operations call(s):\n  \
         {ndiff} of {ARRAY_SIZE} elements differ\n  \
         first at index {i}: input={} C={} Rust={}",
        input[i], ca[i], ra[i]
    );
}

/// Byte comparison against a cached C reference array dump.
pub fn assert_matches_reference_array(row: &str, file: &str, got: &[i32]) {
    let path = reference_dir().join(file);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("[{row}] missing C reference {}: {e}", path.display()));
    assert_eq!(
        bytes.len(),
        ARRAY_SIZE * 4,
        "[{row}] reference {} has wrong size",
        path.display()
    );
    let expect: Vec<i32> = bytes
        .chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    assert_arrays_eq(row, 2000, &expect, &expect, got);
}

pub fn read_reference_stdout(row: &str, file: &str) -> Vec<u8> {
    let path = reference_dir().join(file);
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("[{row}] missing C reference {}: {e}", path.display()))
}

/// FNV-1a over the array's little-endian bytes — byte-identical to the `hash` op
/// in `tools/runner.c`, so a stored hash is evidence about the exact bytes.
pub fn fnv1a(arr: &[i32]) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for &v in arr {
        for b in (v as u32).to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    h
}
