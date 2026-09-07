// Shared differential-test harness.
//
// BOTH libraries are loaded as shared objects through `libloading` and every
// call goes through an exported `extern "C"` symbol — the Rust functions are
// never called directly, so the `#[no_mangle]` export wrappers are under test
// too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// ABI mirror of the C struct
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

// ---------------------------------------------------------------------------
// libc bits we need directly (no `libc` crate dependency)
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// Loaded library
// ---------------------------------------------------------------------------
pub type FnCreateBuffer = unsafe extern "C" fn(c_int) -> *mut StringBuffer;
pub type FnAppendToBuffer = unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int;
pub type FnDestroyBuffer = unsafe extern "C" fn(*mut StringBuffer);
pub type FnGetOperationName = unsafe extern "C" fn(c_int) -> *const c_char;
pub type FnPerformOperation = unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int;
pub type FnBuffapp = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Impl {
    pub name: &'static str,
    pub create_buffer: FnCreateBuffer,
    pub append_to_buffer: FnAppendToBuffer,
    pub destroy_buffer: FnDestroyBuffer,
    pub get_operation_name: FnGetOperationName,
    pub perform_operation: FnPerformOperation,
    pub buffapp: FnBuffapp,
    _lib: Library,
}

// The `Library` handle keeps the .so mapped for the whole process; the raw fn
// pointers are plain code addresses.
unsafe impl Send for Impl {}
unsafe impl Sync for Impl {}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let s: Symbol<$t> = lib.get($s.as_bytes()).unwrap_or_else(|e| {
                        panic!("{} .so is missing exported symbol {}: {e}", name, $s)
                    });
                    *s
                }};
            }
            let create_buffer = sym!(FnCreateBuffer, "create_buffer\0");
            let append_to_buffer = sym!(FnAppendToBuffer, "append_to_buffer\0");
            let destroy_buffer = sym!(FnDestroyBuffer, "destroy_buffer\0");
            let get_operation_name = sym!(FnGetOperationName, "get_operation_name\0");
            let perform_operation = sym!(FnPerformOperation, "perform_operation\0");
            let buffapp = sym!(FnBuffapp, "buffapp\0");
            Impl {
                name,
                create_buffer,
                append_to_buffer,
                destroy_buffer,
                get_operation_name,
                perform_operation,
                buffapp,
                _lib: lib,
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("manifest has a parent").to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("BUFFAPP_C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}; build the C lib first", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C .so in {}, found {candidates:?}",
        build.display()
    );
    candidates.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("BUFFAPP_RUST_SO") {
        return PathBuf::from(p);
    }
    let target = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libbuffapp_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libbuffapp_lib.so not found under {}; run `cargo build --release` first",
        target.display()
    );
}

static C_IMPL: OnceLock<Impl> = OnceLock::new();
static R_IMPL: OnceLock<Impl> = OnceLock::new();

pub fn c() -> &'static Impl {
    C_IMPL.get_or_init(|| Impl::load("C", &find_c_so()))
}

pub fn rust() -> &'static Impl {
    R_IMPL.get_or_init(|| Impl::load("Rust", &find_rust_so()))
}

/// `[c(), rust()]` — iterate to run the same script against both.
pub fn both() -> [&'static Impl; 2] {
    [c(), rust()]
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub const DEFAULT_SEED: u64 = 0x5EED_1234_ABCD_0001;

    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn default_seeded() -> Rng {
        Rng(Self::DEFAULT_SEED)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Small-magnitude value, biased toward interesting arithmetic.
    pub fn next_small_i32(&mut self) -> i32 {
        let v = self.next_u64();
        let magnitude = match v % 5 {
            0 => 4u32,
            1 => 16,
            2 => 256,
            3 => 65_536,
            _ => u32::MAX,
        };
        let m = (v >> 8) as u32 % magnitude;
        if (v >> 40) & 1 == 1 {
            -(m as i64) as i32
        } else {
            m as i32
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + (self.next_u64() as usize) % (hi_inclusive - lo + 1)
    }
    /// Random printable, NUL-free byte string of the requested length.
    pub fn ascii(&mut self, len: usize) -> CString {
        let bytes: Vec<u8> = (0..len)
            .map(|_| b'!' + (self.next_u64() % (b'~' - b'!' + 1) as u64) as u8)
            .collect();
        CString::new(bytes).unwrap()
    }
    /// Random printable string whose length is drawn from `lo..=hi`.
    pub fn ascii_range(&mut self, lo: usize, hi_inclusive: usize) -> CString {
        let n = self.range(lo, hi_inclusive);
        self.ascii(n)
    }
}

// ---------------------------------------------------------------------------
// fd-1 capture (byte-for-byte stdout comparison)
// ---------------------------------------------------------------------------
static FD1_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with fd 1 redirected to a temp file and returns the exact bytes
/// written to fd 1. Both `.so`s share this process's glibc `stdout`, so a
/// single `fflush(NULL)` flushes whichever of them wrote.
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    let _guard = FD1_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "buffapp_cap_{}_{:p}.bin",
        std::process::id(),
        &_guard as *const _
    ));
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    let out;
    let ret;
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        ret = f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        close(fd);
        out = std::fs::read(&path).expect("read capture file");
        let _ = std::fs::remove_file(&path);
    }
    (ret, out)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a NUL-terminated C string as raw bytes (no UTF-8 assumption).
pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    assert!(!p.is_null(), "unexpected NULL string");
    let n = strlen(p);
    std::slice::from_raw_parts(p as *const u8, n).to_vec()
}

/// Snapshot of the observable state of a `StringBuffer`.
#[derive(Debug, PartialEq, Eq)]
pub struct BufSnapshot {
    pub is_null: bool,
    pub data_is_null: bool,
    pub capacity: c_int,
    pub length: c_int,
    /// `data[0..length]` — the payload, independent of the NUL terminator.
    pub payload: Vec<u8>,
    /// The byte at `data[length]` (the terminator the C code maintains).
    pub terminator: Option<u8>,
}

pub unsafe fn snapshot(b: *mut StringBuffer) -> BufSnapshot {
    if b.is_null() {
        return BufSnapshot {
            is_null: true,
            data_is_null: true,
            capacity: 0,
            length: 0,
            payload: Vec::new(),
            terminator: None,
        };
    }
    let sb = &*b;
    let (payload, terminator) = if sb.data.is_null() {
        (Vec::new(), None)
    } else {
        let n = sb.length.max(0) as usize;
        (
            std::slice::from_raw_parts(sb.data as *const u8, n).to_vec(),
            Some(*(sb.data as *const u8).add(n)),
        )
    };
    BufSnapshot {
        is_null: false,
        data_is_null: sb.data.is_null(),
        capacity: sb.capacity,
        length: sb.length,
        payload,
        terminator,
    }
}

/// Allocate a `StringBuffer` with `malloc` so that either library's
/// `destroy_buffer` can free it, with a caller-chosen `data` pointer.
pub unsafe fn manual_buffer(data: *mut c_char, capacity: c_int, length: c_int) -> *mut StringBuffer {
    let b = malloc(std::mem::size_of::<StringBuffer>()) as *mut StringBuffer;
    assert!(!b.is_null());
    (*b).data = data;
    (*b).capacity = capacity;
    (*b).length = length;
    b
}

pub unsafe fn free_raw(p: *mut c_void) {
    free(p)
}

/// Run `f` in a forked child; return `Ok(exit_code)` or `Err(signal)`.
pub fn run_in_child<F: FnOnce()>(f: F) -> Result<i32, i32> {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        // WIFSIGNALED / WTERMSIG / WEXITSTATUS
        let termsig = status & 0x7f;
        if termsig != 0 && termsig != 0x7f {
            Err(termsig)
        } else {
            Ok((status >> 8) & 0xff)
        }
    }
}
