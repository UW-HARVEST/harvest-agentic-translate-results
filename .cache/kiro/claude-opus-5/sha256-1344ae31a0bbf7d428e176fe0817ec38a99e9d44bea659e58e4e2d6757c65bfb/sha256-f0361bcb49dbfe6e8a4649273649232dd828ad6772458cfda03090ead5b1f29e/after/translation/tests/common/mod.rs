//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and calls every
//! function through its exported symbol, so the `#[no_mangle]` wrappers and the
//! C ABI are part of what is under test. No Rust function is ever called
//! directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Mirror of the C `ProcessState` so tests can read state back out.
//
//   typedef struct { PackedFlags flags; TypeConfusion data;
//                    char* buffer; int capacity; } ProcessState;
//
// `PackedFlags` is a 32-bit bit-field storage unit and `TypeConfusion` is a
// 4-byte union, so both are modelled as `u32`. `create_state` assigns every
// bit-field, so all 32 bits of `flags` are defined and safe to compare.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ProcessState {
    pub flags: u32,
    pub data: u32,
    pub buffer: *mut c_char,
    pub capacity: c_int,
}

/// Decoded bit-fields (System V x86-64: allocated from the least significant
/// bit of one 4-byte unit).
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Flags {
    pub flag1: u32,
    pub flag2: u32,
    pub flag3: u32,
    pub counter: u32,
    pub mode: u32,
    pub status: u32,
    pub reserved: u32,
}

pub fn decode_flags(raw: u32) -> Flags {
    Flags {
        flag1: raw & 0x1,
        flag2: (raw >> 1) & 0x1,
        flag3: (raw >> 2) & 0x1,
        counter: (raw >> 3) & 0x1F,
        mode: (raw >> 8) & 0x7,
        status: (raw >> 11) & 0x1F,
        reserved: (raw >> 16) & 0xFFFF,
    }
}

// ---------------------------------------------------------------------------
// Function pointer types matching the C signatures exactly.
// ---------------------------------------------------------------------------
pub type FnCreateState = unsafe extern "C" fn(c_int, c_int) -> *mut ProcessState;
pub type FnDestroyState = unsafe extern "C" fn(*mut ProcessState);
pub type FnProcessBuffer = unsafe extern "C" fn(*mut ProcessState, c_char) -> c_int;
pub type FnUpdateFlags = unsafe extern "C" fn(*mut ProcessState, c_int);
pub type FnConfuseTypes = unsafe extern "C" fn(*mut ProcessState, c_int) -> c_int;
pub type FnConfusion = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub create_state: FnCreateState,
    pub destroy_state: FnDestroyState,
    pub process_buffer: FnProcessBuffer,
    pub update_flags: FnUpdateFlags,
    pub confuse_types: FnConfuseTypes,
    pub confusion: FnConfusion,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to load {} ({}): {e}", name, path.display()));
            macro_rules! sym {
                ($n:literal, $t:ty) => {{
                    let s: Symbol<$t> = lib
                        .get(concat!($n, "\0").as_bytes())
                        .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name, $n));
                    *s
                }};
            }
            let create_state = sym!("create_state", FnCreateState);
            let destroy_state = sym!("destroy_state", FnDestroyState);
            let process_buffer = sym!("process_buffer", FnProcessBuffer);
            let update_flags = sym!("update_flags", FnUpdateFlags);
            let confuse_types = sym!("confuse_types", FnConfuseTypes);
            let confusion = sym!("confusion", FnConfusion);
            Impl {
                name,
                _lib: lib,
                create_state,
                destroy_state,
                process_buffer,
                update_flags,
                confuse_types,
                confusion,
            }
        }
    }

    /// Read the NUL-terminated buffer contents (bounded by `capacity`).
    pub unsafe fn buffer_bytes(&self, state: *mut ProcessState) -> Option<Vec<u8>> {
        if state.is_null() {
            return None;
        }
        let s = unsafe { *state };
        if s.buffer.is_null() || s.capacity <= 0 {
            return None;
        }
        Some(unsafe { CStr::from_ptr(s.buffer) }.to_bytes().to_vec())
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    // The CMake project name is derived from the parent directory name, so the
    // library file name is not fixed — glob the build directory instead.
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} — build the C library first", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {}, got {:?}", build.display(), found);
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    for profile in ["release", "debug"] {
        let p = manifest_dir()
            .join("target")
            .join(profile)
            .join("libconfusion_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libconfusion_lib.so not found — run `cargo build --release` first");
}

/// Both implementations, loaded once per test binary.
pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Impl::load("C", &find_c_so()),
        r: Impl::load("Rust", &find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture.
//
// Both `.so`s `printf` to the process's fd 1 through the same glibc, so the
// only reliable way to compare their output is to redirect fd 1 into a file
// around each call and flush libc's stream buffers.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

use std::sync::Mutex;
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` with fd 1 redirected to a temporary file and return `f`'s value
/// together with the exact bytes written to stdout.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::AsRawFd;

    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "cdiff-{}-{:?}-{}.out",
        std::process::id(),
        std::thread::current().id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp file");

    let out = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let r = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        r
    };

    file.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read");
    drop(file);
    let _ = std::fs::remove_file(&path);

    (out, buf)
}

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Render captured stdout for assertion messages.
pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace('\n', "\\n")
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — no external dependency, fixed seed.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
