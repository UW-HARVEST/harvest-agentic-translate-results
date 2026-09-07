//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through the dynamic symbol table, so the `#[no_mangle]` export
//! wrappers are exercised exactly as an external C consumer would exercise
//! them. No function from this crate is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type CInt = std::os::raw::c_int;
pub type CChar = std::os::raw::c_char;

// ---------------------------------------------------------------------------
// libc bits used only by the harness (fd juggling + flushing the shared stdout)
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
    fn free(p: *mut std::ffi::c_void);
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
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

fn rust_so_path() -> PathBuf {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the profile the tests themselves were built with.
    let profiles: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for p in profiles {
        let cand = target.join(p).join("libcomplexmode_lib.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "libcomplexmode_lib.so not found under {}. Run `cargo build` (and/or \
         `cargo build --release`) before `cargo test`.",
        target.display()
    );
}

// ---------------------------------------------------------------------------
// The loaded pair
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();
static IO_LOCK: Mutex<()> = Mutex::new(());

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("failed to dlopen the C .so");
        let rs = Library::new(rust_so_path()).expect("failed to dlopen the Rust .so");
        Libs { c, rs }
    })
}

fn io_guard() -> MutexGuard<'static, ()> {
    match IO_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both `.so`s resolve `printf`/`puts` from the one glibc already linked into
// this process, so they share a single `stdout` FILE*. Redirecting fd 1 to a
// temporary file around each call therefore captures exactly the bytes the
// library under test emitted.
// ---------------------------------------------------------------------------

pub fn capture<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    let _g = io_guard();
    unsafe { fflush(std::ptr::null_mut()) };

    let path = std::env::temp_dir().join(format!(
        "ctorust-cap-{}-{:?}.bin",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = fs::File::create(&path).expect("cannot create capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let out = f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = fs::read(&path).expect("cannot read capture file");
    let _ = fs::remove_file(&path);
    (out, bytes)
}

/// `free` from the shared glibc, for buffers `malloc`ed by either library.
pub unsafe fn c_free(p: *mut CChar) {
    unsafe { free(p as *mut std::ffi::c_void) }
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// Typed symbol lookup — one accessor per exported C function.
// ---------------------------------------------------------------------------

type FnCheckPermissions = unsafe extern "C" fn(CInt, CInt) -> CInt;
type FnSafeAdd = unsafe extern "C" fn(CInt, CInt, CInt) -> CInt;
type FnCreateResultString = unsafe extern "C" fn(*const CChar, CInt) -> *mut CChar;
type FnMultiplyWithLog = unsafe extern "C" fn(CInt, CInt, *mut *mut CChar) -> CInt;
type FnCopyAndSum = unsafe extern "C" fn(*mut CInt, CInt) -> CInt;
type FnCompareOperations = unsafe extern "C" fn(*const CChar, *const CChar) -> CInt;
type FnComplexmode = unsafe extern "C" fn(CInt, CInt, CInt, CInt) -> CInt;

fn sym<'l, T>(lib: &'l Library, name: &[u8]) -> Symbol<'l, T> {
    unsafe { lib.get(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol {} missing from .so: {e}",
            String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
        )
    })
}

/// Which implementation a closure is being run against.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn lib(self) -> &'static Library {
        match self {
            Impl::C => &libs().c,
            Impl::Rust => &libs().rs,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

pub const BOTH: [Impl; 2] = [Impl::C, Impl::Rust];

/// Raw, pre-resolved `extern "C"` function pointers for one implementation.
///
/// Needed by the heap-exhaustion tests: `dlsym` through `libloading` allocates
/// (it builds a `CString` for the symbol name), which is impossible inside a
/// child whose allocator has been drained. Resolving everything up-front and
/// caching it in a `OnceLock` makes the child allocation-free.
#[derive(Copy, Clone)]
pub struct Raw {
    pub check_permissions: FnCheckPermissions,
    pub safe_add: FnSafeAdd,
    pub create_result_string: FnCreateResultString,
    pub multiply_with_log: FnMultiplyWithLog,
    pub copy_and_sum: FnCopyAndSum,
    pub compare_operations: FnCompareOperations,
    pub complexmode: FnComplexmode,
}

static RAW_C: OnceLock<Raw> = OnceLock::new();
static RAW_RS: OnceLock<Raw> = OnceLock::new();

pub fn raw(i: Impl) -> Raw {
    let cell = match i {
        Impl::C => &RAW_C,
        Impl::Rust => &RAW_RS,
    };
    *cell.get_or_init(|| {
        let l = i.lib();
        Raw {
            check_permissions: *sym::<FnCheckPermissions>(l, b"check_permissions\0"),
            safe_add: *sym::<FnSafeAdd>(l, b"safe_add\0"),
            create_result_string: *sym::<FnCreateResultString>(l, b"create_result_string\0"),
            multiply_with_log: *sym::<FnMultiplyWithLog>(l, b"multiply_with_log\0"),
            copy_and_sum: *sym::<FnCopyAndSum>(l, b"copy_and_sum\0"),
            compare_operations: *sym::<FnCompareOperations>(l, b"compare_operations\0"),
            complexmode: *sym::<FnComplexmode>(l, b"complexmode\0"),
        }
    })
}

pub fn check_permissions(i: Impl, perms: CInt, required: CInt) -> CInt {
    let f: Symbol<FnCheckPermissions> = sym(i.lib(), b"check_permissions\0");
    unsafe { f(perms, required) }
}

pub fn safe_add(i: Impl, a: CInt, b: CInt, perms: CInt) -> CInt {
    let f: Symbol<FnSafeAdd> = sym(i.lib(), b"safe_add\0");
    unsafe { f(a, b, perms) }
}

/// Calls `create_result_string` and returns the resulting C string's bytes
/// (up to but excluding the NUL), or `None` when the callee returned NULL.
///
/// Only bytes up to the terminating NUL are compared: the tail of the 64-byte
/// `malloc` buffer is never written by `snprintf` and is therefore
/// indeterminate in the C as well.
pub fn create_result_string(i: Impl, op: &CStr, val: CInt) -> Option<Vec<u8>> {
    let f: Symbol<FnCreateResultString> = sym(i.lib(), b"create_result_string\0");
    let p = unsafe { f(op.as_ptr(), val) };
    if p.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(p) }.to_bytes().to_vec();
    // The buffer came from the shared glibc `malloc`, so the harness may free it.
    unsafe { free(p as *mut std::ffi::c_void) };
    Some(bytes)
}

/// Calls `multiply_with_log`, returning `(retval, Option<log bytes>)`.
pub fn multiply_with_log(i: Impl, a: CInt, b: CInt) -> (CInt, Option<Vec<u8>>) {
    let f: Symbol<FnMultiplyWithLog> = sym(i.lib(), b"multiply_with_log\0");
    // Poison the out-parameter so we can tell "left untouched" from "set to NULL".
    let mut out: *mut CChar = 0x1 as *mut CChar;
    let r = unsafe { f(a, b, &mut out) };
    let log = if out.is_null() || out as usize == 0x1 {
        None
    } else {
        let v = unsafe { CStr::from_ptr(out) }.to_bytes().to_vec();
        unsafe { free(out as *mut std::ffi::c_void) };
        Some(v)
    };
    (r, log)
}

pub fn copy_and_sum(i: Impl, src: *mut CInt, count: CInt) -> CInt {
    let f: Symbol<FnCopyAndSum> = sym(i.lib(), b"copy_and_sum\0");
    unsafe { f(src, count) }
}

pub fn compare_operations(i: Impl, op1: *const CChar, op2: *const CChar) -> CInt {
    let f: Symbol<FnCompareOperations> = sym(i.lib(), b"compare_operations\0");
    unsafe { f(op1, op2) }
}

pub fn complexmode(i: Impl, mode: CInt, v1: CInt, v2: CInt, v3: CInt) -> CInt {
    let f: Symbol<FnComplexmode> = sym(i.lib(), b"complexmode\0");
    unsafe { f(mode, v1, v2, v3) }
}

// ---------------------------------------------------------------------------
// Differential assertion helper: run the same closure against C then Rust,
// capturing stdout for each, and require identical (value, stdout).
// ---------------------------------------------------------------------------

pub fn diff<T, F>(ctx: &str, mut run: F)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(Impl) -> T,
{
    let (cv, cout) = capture(|| run(Impl::C));
    let (rv, rout) = capture(|| run(Impl::Rust));
    assert_eq!(cv, rv, "return value mismatch for {ctx}");
    assert_eq!(
        cout,
        rout,
        "stdout mismatch for {ctx}\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&cout),
        show(&rout)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x2024_0C0D_E5EE_D001;

impl Rng {
    pub fn new() -> Self {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(if s == 0 { SEED } else { s })
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
    /// Small-magnitude value, biased towards interesting small ints.
    pub fn small(&mut self) -> i32 {
        (self.next_u64() % 2001) as i32 - 1000
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    /// A mix of small values, extremes and full-range values.
    pub fn any_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => 0,
            1 => i32::MAX,
            2 => i32::MIN,
            3 => -1,
            4 | 5 => self.small(),
            _ => self.next_i32(),
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 13) as u8
    }
}

/// Random NUL-free byte string of length `len`.
pub fn rand_cstring(rng: &mut Rng, len: usize) -> CString {
    let mut v = Vec::with_capacity(len);
    for _ in 0..len {
        let mut b = rng.byte();
        if b == 0 {
            b = 1;
        }
        v.push(b);
    }
    CString::new(v).unwrap()
}

pub const INTERESTING_INTS: [i32; 13] = [
    0,
    1,
    -1,
    2,
    -2,
    7,
    -7,
    255,
    65535,
    65536,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
];

// ---------------------------------------------------------------------------
// Heap-exhaustion harness (ERRORS.md rows 1, 3, 14).
//
// `malloc` cannot be made to fail in-process without wrecking the test
// harness, so each case runs in a `fork()`ed child which:
//   1. redirects fd 1 to a private file,
//   2. lowers RLIMIT_AS to just above the current address-space size,
//   3. drains every remaining free block out of the allocator,
//   4. calls the library function under test,
//   5. writes the return value to a pre-opened result fd and `_exit`s.
//
// The C child and the Rust child follow the identical script, so a difference
// in the captured (return value, stdout) is a real behavioural difference.
// ---------------------------------------------------------------------------

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

const RLIMIT_AS: i32 = 9; // Linux

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
    fn setrlimit(resource: i32, rlim: *const RLimit) -> i32;
    fn getrlimit(resource: i32, rlim: *mut RLimit) -> i32;
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn write(fd: i32, buf: *const std::ffi::c_void, n: usize) -> isize;
    fn ftruncate(fd: i32, len: i64) -> i32;
    fn lseek(fd: i32, off: i64, whence: i32) -> i64;
    fn printf(fmt: *const CChar, ...) -> i32;
}

/// Address space currently mapped, in bytes (field 1 of /proc/self/statm, in pages).
fn vm_size_bytes() -> u64 {
    let s = fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = s
        .split_whitespace()
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);
    pages * 4096
}

/// Drain the allocator: chain every obtainable block through its own first word
/// so no auxiliary container (which would itself allocate) is needed.
unsafe fn drain_heap() {
    let mut head: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut sz: usize = 4 * 1024 * 1024;
    while sz >= 16 {
        loop {
            let p = unsafe { malloc(sz) };
            if p.is_null() {
                break;
            }
            unsafe { *(p as *mut usize) = head as usize };
            head = p;
        }
        sz /= 2;
    }
    std::hint::black_box(head);
}

/// Result of one heap-exhausted child run.
#[derive(Debug, PartialEq, Eq)]
pub struct OomRun {
    pub ret: i64,
    pub stdout: Vec<u8>,
}

/// Runs `body` in a forked child whose allocator has been drained.
/// `body` must not allocate other than through the library under test.
pub fn run_oom<F>(tag: &str, imp: Impl, body: F) -> OomRun
where
    F: FnOnce(Impl) -> i64,
{
    let _g = io_guard();
    // Resolve the symbols and touch the libraries *before* forking so that no
    // lazy dlopen/dlsym allocation happens inside the drained child.
    let _ = libs();
    let _ = raw(imp);
    let _ = check_permissions(imp, 0, 0);

    let dir = std::env::temp_dir();
    let out_path = dir.join(format!("ctorust-oom-{}-{}-{}.out", std::process::id(), tag, imp.name()));
    let ret_path = dir.join(format!("ctorust-oom-{}-{}-{}.ret", std::process::id(), tag, imp.name()));
    let out_file = fs::File::create(&out_path).expect("oom out file");
    let ret_file = fs::File::create(&ret_path).expect("oom ret file");
    let out_fd = out_file.as_raw_fd();
    let ret_fd = ret_file.as_raw_fd();

    let mut lim = RLimit { rlim_cur: 0, rlim_max: 0 };
    unsafe { getrlimit(RLIMIT_AS, &mut lim) };
    let budget = vm_size_bytes() + 8 * 1024 * 1024;

    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---- child ----
        unsafe {
            dup2(out_fd, 1);
            // Force glibc to allocate stdout's buffer now, then rewind the file
            // so the pre-test newline is not part of the captured bytes.
            printf(c"\n".as_ptr());
            fflush(std::ptr::null_mut());
            lseek(1, 0, 0);
            ftruncate(1, 0);

            let newlim = RLimit {
                rlim_cur: budget.min(lim.rlim_max),
                rlim_max: lim.rlim_max,
            };
            setrlimit(RLIMIT_AS, &newlim);
            drain_heap();

            let r = body(imp);

            fflush(std::ptr::null_mut());
            write(ret_fd, &r as *const i64 as *const std::ffi::c_void, 8);
            _exit(0);
        }
    }

    let mut status: i32 = 0;
    let got = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(got, pid, "waitpid failed");

    drop(out_file);
    drop(ret_file);
    let stdout = fs::read(&out_path).unwrap_or_default();
    let raw = fs::read(&ret_path).unwrap_or_default();
    assert_eq!(
        raw.len(),
        8,
        "heap-exhausted {} child for {tag} did not report a return value (exit status {status:#x})",
        imp.name()
    );
    let ret = i64::from_ne_bytes(raw.try_into().unwrap());
    let _ = fs::remove_file(&out_path);
    let _ = fs::remove_file(&ret_path);
    OomRun { ret, stdout }
}

/// Differential version of [`run_oom`].
pub fn diff_oom<F>(tag: &str, body: F)
where
    F: Fn(Impl) -> i64,
{
    let c = run_oom(tag, Impl::C, &body);
    let r = run_oom(tag, Impl::Rust, &body);
    assert_eq!(
        c.ret, r.ret,
        "heap-exhausted return value mismatch for {tag}: C={} Rust={}",
        c.ret, r.ret
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "heap-exhausted stdout mismatch for {tag}\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
}

/// Like [`multiply_with_log`] but reports whether the out-parameter was left
/// NULL, encoded so it can travel through the `i64` channel of [`run_oom`].
/// Returns `ret * 4 + (log_was_null as i64) * 2 + (log_untouched as i64)`.
pub fn multiply_with_log_encoded(i: Impl, a: CInt, b: CInt) -> i64 {
    let f = raw(i).multiply_with_log;
    let mut out: *mut CChar = 0x1 as *mut CChar;
    let r = unsafe { f(a, b, &mut out) };
    let untouched = out as usize == 0x1;
    let is_null = out.is_null();
    (r as i64) * 4 + (is_null as i64) * 2 + (untouched as i64)
}

// ---------------------------------------------------------------------------
// Batched differential comparison.
//
// `diff` redirects fd 1 for every single call, which limits how many random
// inputs are practical. `diff_batch` instead runs a whole batch of calls under
// one redirection, so millions of inputs can be compared: the accumulated
// return values AND the concatenated stdout of the entire batch must match.
// ---------------------------------------------------------------------------

pub fn diff_batch<T, F>(ctx: &str, mut run: F)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(Impl) -> Vec<T>,
{
    let (cv, cout) = capture(|| run(Impl::C));
    let (rv, rout) = capture(|| run(Impl::Rust));
    assert_eq!(cv.len(), rv.len(), "batch length mismatch for {ctx}");
    for (k, (a, b)) in cv.iter().zip(rv.iter()).enumerate() {
        assert_eq!(a, b, "batch {ctx}: return value mismatch at index {k}");
    }
    if cout != rout {
        // Report the first differing byte and its neighbourhood.
        let at = cout
            .iter()
            .zip(rout.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(cout.len().min(rout.len()));
        let lo = at.saturating_sub(120);
        panic!(
            "batch {ctx}: stdout diverges at byte {at} (C {} bytes, Rust {} bytes)\n\
             C   ...\"{}\"\n  Rust...\"{}\"",
            cout.len(),
            rout.len(),
            show(&cout[lo..(at + 120).min(cout.len())]),
            show(&rout[lo..(at + 120).min(rout.len())]),
        );
    }
}
