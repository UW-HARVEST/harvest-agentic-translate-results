// Shared harness for the C-vs-Rust differential tests.
//
// Both the C `.so` and the Rust `.so` are loaded through `libloading` and driven
// only through their exported C symbols -- the Rust functions are never called
// directly, so the `#[no_mangle]` / `extern "C"` wrappers are under test too.
#![allow(dead_code)]

use libloading::os::unix::Library as UnixLibrary;
use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

pub type FnPrint = unsafe extern "C" fn(*const c_int);
pub type FnVoid = unsafe extern "C" fn();
pub type FnDriver = unsafe extern "C" fn(c_int);
/// Deliberately-wrong signature used to smuggle 64-bit garbage into the high
/// half of `driver`'s 32-bit `int` parameter.
pub type FnDriver64 = unsafe extern "C" fn(i64);

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

pub const PROT_NONE: c_int = 0;
pub const PROT_READ: c_int = 1;
pub const PROT_WRITE: c_int = 2;
pub const MAP_PRIVATE: c_int = 0x02;
pub const MAP_ANONYMOUS: c_int = 0x20;
pub const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;
pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;

// Both libraries are opened with RTLD_NOW so that every relocation is resolved
// at load time. With libloading's default RTLD_LAZY, the first call to
// `bad@plt`/`good@plt` from inside `driver` runs the dynamic linker's lazy
// resolver, whose own stack usage overwrites the very slot that `bad()` reads
// back -- a loader artifact that perturbs the CWE-457 read and has nothing to do
// with either implementation. RTLD_NOW removes that variable from the comparison.
const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;

/// The four exported entry points of one implementation.
pub struct Api {
    pub name: &'static str,
    pub print_int_ptr_line: FnPrint,
    pub bad: FnVoid,
    pub good: FnVoid,
    pub driver: FnDriver,
    pub driver64: FnDriver64,
    _lib: Library,
}

impl Api {
    pub fn load(name: &'static str, path: &PathBuf) -> Api {
        unsafe {
            let lib: Library = UnixLibrary::open(Some(path), RTLD_NOW | RTLD_LOCAL)
                .unwrap_or_else(|e| panic!("cannot dlopen {} ({}): {e}", path.display(), name))
                .into();
            let print_int_ptr_line = {
                let s: Symbol<FnPrint> = lib
                    .get(b"printIntPtrLine\0")
                    .expect("missing symbol printIntPtrLine");
                *s
            };
            let bad = {
                let s: Symbol<FnVoid> = lib.get(b"bad\0").expect("missing symbol bad");
                *s
            };
            let good = {
                let s: Symbol<FnVoid> = lib.get(b"good\0").expect("missing symbol good");
                *s
            };
            let driver = {
                let s: Symbol<FnDriver> = lib.get(b"driver\0").expect("missing symbol driver");
                *s
            };
            let driver64 = {
                let s: Symbol<FnDriver64> = lib.get(b"driver\0").expect("missing symbol driver");
                *s
            };
            Api {
                name,
                print_int_ptr_line,
                bad,
                good,
                driver,
                driver64,
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built at {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // current_exe() is target/<profile>/deps/<testbin>; the cdylib is one level up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    for cand in [
        deps.parent().map(|d| d.join("libdriver.so")),
        Some(deps.join("libdriver.so")),
    ]
    .into_iter()
    .flatten()
    {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found near {}; run `cargo build` first",
        deps.display()
    );
}

/// Serializes every fd-1 redirection and every `fork()` in the suite.
static IO_LOCK: Mutex<()> = Mutex::new(());

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let uniq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from(dir).join(format!("diffcap-{tag}-{}-{uniq}.out", std::process::id()))
}

/// Runs `f` with `stdout` (fd 1) redirected to a temp file and returns the exact
/// bytes written. `fflush(NULL)` is used on both sides of the window so that the
/// C `printf` block buffer -- shared by both `.so`s, since there is one libc --
/// is fully drained into the capture.
pub fn capture<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _guard = IO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = temp_path(tag);
    let file = File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);
    let out = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    out
}

/// How a child process finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Exited { code: i32, out: Vec<u8> },
    Signaled { sig: i32, out: Vec<u8> },
}

impl Outcome {
    pub fn out(&self) -> &[u8] {
        match self {
            Outcome::Exited { out, .. } | Outcome::Signaled { sig: _, out } => out,
        }
    }
    pub fn signal(&self) -> Option<i32> {
        match self {
            Outcome::Signaled { sig, .. } => Some(*sig),
            _ => None,
        }
    }
    /// Outcome shape ignoring any captured bytes -- what the two implementations
    /// must agree on for a faulting input.
    pub fn kind(&self) -> String {
        match self {
            Outcome::Exited { code, .. } => format!("exited({code})"),
            Outcome::Signaled { sig, .. } => format!("signaled({sig})"),
        }
    }
}

/// Runs `f` in a forked child with fd 1 and fd 2 redirected to a temp file, so a
/// `SIGSEGV` (the C library's only way of rejecting bad input) is observable
/// instead of killing the test runner.
pub fn run_isolated<F: FnOnce()>(tag: &str, f: F) -> Outcome {
    let _guard = IO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = temp_path(tag);
    let file = File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();
    unsafe {
        // Drain the parent's buffers so they are not duplicated by the fork.
        fflush(std::ptr::null_mut());
    }
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            dup2(fd, 1);
            dup2(fd, 2);
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
    }
    let mut status: c_int = 0;
    unsafe { waitpid(pid, &mut status, 0) };
    drop(file);
    let out = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    let low = status & 0x7f;
    if low == 0 {
        Outcome::Exited {
            code: (status >> 8) & 0xff,
            out,
        }
    } else {
        Outcome::Signaled { sig: low, out }
    }
}

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform non-zero `i32` -- every value that makes `driver` take the
    /// `good()` arm.
    pub fn next_nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }
}

pub const SEED: u64 = 0x5EED_1234;

pub fn both() -> (Api, Api) {
    let c = Api::load("C", &c_so_path());
    let r = Api::load("Rust", &rust_so_path());
    (c, r)
}

/// Anonymous private mapping helper for the pointer-shape rows.
pub struct Mapping {
    pub ptr: *mut u8,
    pub len: usize,
}

impl Mapping {
    pub fn new(len: usize, prot: c_int) -> Mapping {
        let ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                prot,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(ptr != MAP_FAILED && !ptr.is_null(), "mmap failed");
        Mapping {
            ptr: ptr as *mut u8,
            len,
        }
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe {
            munmap(self.ptr as *mut c_void, self.len);
        }
    }
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace('\n', "\\n")
}
