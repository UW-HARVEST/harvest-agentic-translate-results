// Shared harness: loads BOTH the C .so and the Rust .so via libloading and
// exposes symbol-for-symbol wrappers so every call in every test crosses the
// real FFI boundary (never a direct Rust call).
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

pub type FnCreateBuffer = unsafe extern "C" fn(c_int) -> *mut StringBuffer;
pub type FnAppendToBuffer = unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int;
pub type FnDestroyBuffer = unsafe extern "C" fn(*mut StringBuffer);
pub type FnGetOperationName = unsafe extern "C" fn(c_int) -> *const c_char;
pub type FnPerformOperation = unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int;
pub type FnBuffapp = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C .so or the Rust .so).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub create_buffer: FnCreateBuffer,
    pub append_to_buffer: FnAppendToBuffer,
    pub destroy_buffer: FnDestroyBuffer,
    pub get_operation_name: FnGetOperationName,
    pub perform_operation: FnPerformOperation,
    pub buffapp: FnBuffapp,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("load {}: {e}", path.display()));
        macro_rules! sym {
            ($t:ty, $s:literal) => {{
                let s: Symbol<$t> = lib
                    .get($s)
                    .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name, stringify!($s)));
                *s
            }};
        }
        let create_buffer = sym!(FnCreateBuffer, b"create_buffer\0");
        let append_to_buffer = sym!(FnAppendToBuffer, b"append_to_buffer\0");
        let destroy_buffer = sym!(FnDestroyBuffer, b"destroy_buffer\0");
        let get_operation_name = sym!(FnGetOperationName, b"get_operation_name\0");
        let perform_operation = sym!(FnPerformOperation, b"perform_operation\0");
        let buffapp = sym!(FnBuffapp, b"buffapp\0");
        Impl {
            name,
            _lib: lib,
            create_buffer,
            append_to_buffer,
            destroy_buffer,
            get_operation_name,
            perform_operation,
            buffapp,
        }
    }

    /// Read back a buffer's fields plus its NUL-terminated payload.
    pub unsafe fn snapshot(&self, b: *mut StringBuffer) -> Snapshot {
        if b.is_null() {
            return Snapshot { null: true, capacity: 0, length: 0, bytes: Vec::new() };
        }
        let cap = (*b).capacity;
        let len = (*b).length;
        let data = (*b).data;
        let bytes = if data.is_null() {
            Vec::new()
        } else {
            CStr::from_ptr(data).to_bytes().to_vec()
        };
        Snapshot { null: false, capacity: cap, length: len, bytes }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snapshot {
    pub null: bool,
    pub capacity: c_int,
    pub length: c_int,
    pub bytes: Vec<u8>,
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found in {} - build the C library first", dir.display()))
}

fn find_rust_so() -> PathBuf {
    // Allows verifying a specific profile's cdylib (e.g. the dev profile, which
    // enables overflow-checks) without changing the tests.
    if let Ok(p) = std::env::var("BUFFAPP_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "BUFFAPP_RUST_SO={} does not exist", p.display());
        return p;
    }
    let base = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libbuffapp_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libbuffapp_lib.so not found under {} - run `cargo build --release`", base.display());
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Impl::load("C", &find_c_so()),
            rs: Impl::load("RUST", &find_rust_so()),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed per test for reproducibility.
// ---------------------------------------------------------------------------
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Small-ish int, biased toward interesting values.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 14] = [
            0, 1, -1, 2, -2, 3, -3, 4, -4, 7, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
        ];
        match self.next_u32() % 3 {
            0 => SPECIAL[(self.next_u32() as usize) % SPECIAL.len()],
            1 => (self.next_u32() % 41) as i32 - 20,
            _ => self.next_i32(),
        }
    }
    pub fn range(&mut self, lo: u32, hi_inclusive: u32) -> u32 {
        lo + self.next_u32() % (hi_inclusive - lo + 1)
    }
}

// ---------------------------------------------------------------------------
// Isolated execution: every call whose observable effect includes stdout (or a
// fatal signal) is run in a forked child whose fd 1 points at a private temp
// file. That makes the captured bytes immune to the test harness's own output
// and lets us compare fatal signals (SIGSEGV / SIGFPE) as first-class results.
//
// NOTE: run the suite with `--test-threads=1` (see `run_tests.sh`); forking a
// multi-threaded process and then calling malloc/printf in the child is not
// safe in general.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Outcome {
    /// `Some(code)` if the child exited normally.
    pub exit_code: Option<c_int>,
    /// `Some(signo)` if the child died from a signal.
    pub signal: Option<c_int>,
    /// The `c_int` the called function returned (absent if the child died).
    pub ret: Option<c_int>,
    /// Everything the child wrote to fd 1.
    pub stdout: Vec<u8>,
}

fn tmp_path(tag: &str) -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("buffapp_{}_{}_{}.bin", tag, std::process::id(), n))
}

/// Run `f` in a forked child; capture its stdout, its returned `c_int`, and how
/// it terminated. `f` returning `None` means "no int result to record".
pub fn run_isolated(f: impl FnOnce() -> Option<c_int>) -> Outcome {
    let out_path = tmp_path("out");
    let ret_path = tmp_path("ret");
    unsafe {
        let out_file = std::fs::File::options()
            .create(true).read(true).write(true).truncate(true)
            .open(&out_path).expect("open stdout temp file");
        let ret_file = std::fs::File::options()
            .create(true).read(true).write(true).truncate(true)
            .open(&ret_path).expect("open retval temp file");
        use std::os::unix::io::AsRawFd;
        let out_fd = out_file.as_raw_fd();
        let ret_fd = ret_file.as_raw_fd();

        // Drain both buffering layers so nothing pending is duplicated into the child.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        fflush(std::ptr::null_mut());

        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // Child.
            dup2(out_fd, 1);
            let r = f();
            fflush(std::ptr::null_mut());
            if let Some(v) = r {
                let bytes = v.to_ne_bytes();
                write(ret_fd, bytes.as_ptr() as *const c_void, 4);
            }
            close(out_fd);
            close(ret_fd);
            _exit(0);
        }

        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");

        let (exit_code, signal) = if status & 0x7f == 0 {
            (Some((status >> 8) & 0xff), None)
        } else {
            (None, Some(status & 0x7f))
        };

        drop(out_file);
        drop(ret_file);
        let stdout = std::fs::read(&out_path).unwrap_or_default();
        let ret_bytes = std::fs::read(&ret_path).unwrap_or_default();
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&ret_path);
        let ret = if ret_bytes.len() == 4 {
            Some(c_int::from_ne_bytes([ret_bytes[0], ret_bytes[1], ret_bytes[2], ret_bytes[3]]))
        } else {
            None
        };
        Outcome { exit_code, signal, ret, stdout }
    }
}

/// Convenience: isolated run recording a `c_int` result.
pub fn isolated_int(f: impl FnOnce() -> c_int) -> Outcome {
    run_isolated(|| Some(f()))
}

/// Convenience: isolated run of something with no int result.
pub fn isolated_void(f: impl FnOnce()) -> Outcome {
    run_isolated(|| {
        f();
        None
    })
}

/// Call both `buffapp`s on the same params, comparing return value, stdout
/// bytes, and termination status.
pub fn diff_buffapp(p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let p = pair();
    let oc = isolated_int(|| unsafe { (p.c.buffapp)(p1, p2, p3, p4) });
    let or = isolated_int(|| unsafe { (p.rs.buffapp)(p1, p2, p3, p4) });
    assert_eq!(
        oc.ret, or.ret,
        "buffapp({p1}, {p2}, {p3}, {p4}) return value: C={:?} RUST={:?}",
        oc.ret, or.ret
    );
    assert_eq!(
        (oc.exit_code, oc.signal),
        (or.exit_code, or.signal),
        "buffapp({p1}, {p2}, {p3}, {p4}) termination: C={:?} RUST={:?}",
        (oc.exit_code, oc.signal),
        (or.exit_code, or.signal)
    );
    assert_eq!(
        oc.stdout,
        or.stdout,
        "buffapp({p1}, {p2}, {p3}, {p4}) stdout:\nC   = {:?}\nRUST= {:?}",
        String::from_utf8_lossy(&oc.stdout),
        String::from_utf8_lossy(&or.stdout)
    );
}
