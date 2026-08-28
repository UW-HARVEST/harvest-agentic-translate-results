//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` and driven **only** through
//! their exported `extern "C"` symbols, exactly as an external consumer would.
//! No Rust function is ever called directly, so the `#[no_mangle]` wrappers are
//! part of what is under test.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// ABI types
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct { int** matrix; int width; int height; } matrix_t;
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MatrixT {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

pub type FnAllocateMatrix = unsafe extern "C" fn(c_int, c_int) -> *mut MatrixT;
pub type FnFreeMatrix = unsafe extern "C" fn(*mut MatrixT);
pub type FnInitFromString = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut MatrixT;
pub type FnMultiply = unsafe extern "C" fn(*mut MatrixT, *mut MatrixT) -> *mut MatrixT;
pub type FnToString = unsafe extern "C" fn(*mut MatrixT) -> *mut c_char;
pub type FnWriteToFile = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnDriver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

/// The complete exported surface of one `.so`.
pub struct Api {
    pub name: &'static str,
    pub allocate_matrix: FnAllocateMatrix,
    pub free_matrix: FnFreeMatrix,
    pub initialize_matrix_from_string: FnInitFromString,
    pub multiply_matrices: FnMultiply,
    pub matrix_to_string: FnToString,
    pub write_to_file: FnWriteToFile,
    pub driver: FnDriver,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

fn load(name: &'static str, path: &Path) -> Api {
    unsafe {
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
        ));
        macro_rules! sym {
            ($t:ty, $s:literal) => {
                *lib.get::<$t>(concat!($s, "\0").as_bytes())
                    .unwrap_or_else(|e| panic!("{} missing {} : {e}", path.display(), $s))
            };
        }
        Api {
            name,
            allocate_matrix: sym!(FnAllocateMatrix, "allocate_matrix"),
            free_matrix: sym!(FnFreeMatrix, "free_matrix"),
            initialize_matrix_from_string: sym!(FnInitFromString, "initialize_matrix_from_string"),
            multiply_matrices: sym!(FnMultiply, "multiply_matrices"),
            matrix_to_string: sym!(FnToString, "matrix_to_string"),
            write_to_file: sym!(FnWriteToFile, "write_to_file"),
            driver: sym!(FnDriver, "driver"),
        }
    }
}

pub fn c_so_path() -> PathBuf {
    std::env::var_os("DRIVER_C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../c_src/build/libdriver.so"))
}

/// The Rust artifact under test. Defaults to the **release** `cdylib`, because
/// that is the shipped artifact and the apples-to-apples counterpart of the C
/// `.so`: a debug build additionally carries rustc's `debug_assertions`
/// instrumentation (e.g. the "null pointer dereference occurred" panic), which
/// the C — where such a dereference is plain undefined behaviour — does not
/// have. Both profiles are exercised by `scripts/run_all.sh`.
pub fn rust_so_path() -> PathBuf {
    std::env::var_os("DRIVER_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/release/libdriver.so"))
}

/// True when the Rust `.so` under test is built without `debug_assertions`, in
/// which case undefined-behaviour paths (NULL dereference) must terminate the
/// process in *exactly* the same way as the C.
pub fn ub_strict() -> bool {
    if let Some(v) = std::env::var_os("DIFFTEST_UB_STRICT") {
        return v == "1";
    }
    !rust_so_path().to_string_lossy().contains("debug")
}

/// Guard against testing a stale `.so`: every `src/*.rs` must be older than the
/// shared object we are about to load.
fn assert_fresh(so: &Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(e) => panic!("cannot stat {}: {e}", so.display()),
    };
    if let Ok(rd) = std::fs::read_dir("src") {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "rs").unwrap_or(false) {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    assert!(
                        t <= so_mtime,
                        "STALE SHARED OBJECT: {} is newer than {}. \
                         Run `cargo build` (or scripts/run_all.sh) first.",
                        p.display(),
                        so.display()
                    );
                }
            }
        }
    }
}

pub fn c_api() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| load("C", &c_so_path()))
}

pub fn rust_api() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| {
        let p = rust_so_path();
        assert_fresh(&p);
        load("Rust", &p)
    })
}

/// Global exclusion guard: an in-process mutex *plus* an `flock` on a lock file
/// so that separate test binaries cannot interleave either.
pub struct Guard {
    _m: MutexGuard<'static, ()>,
    fd: i32,
}

impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.fd, libc::LOCK_UN);
            libc::close(self.fd);
        }
    }
}

/// Needed because the tests temporarily redirect fd 2 and because `driver`
/// unconditionally writes `./matrix.txt` in the current directory.
pub fn lock() -> Guard {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    let m = match M.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let path = cstr(scratch("global.lock").to_str().unwrap());
    unsafe {
        let fd = libc::open(path.as_ptr(), libc::O_RDWR | libc::O_CREAT, 0o600);
        assert!(fd >= 0, "cannot open lock file");
        assert_eq!(libc::flock(fd, libc::LOCK_EX), 0, "flock failed");
        Guard { _m: m, fd }
    }
}

// ---------------------------------------------------------------------------
// Scratch files
// ---------------------------------------------------------------------------

pub fn scratch_dir() -> PathBuf {
    let p = PathBuf::from("target/difftest");
    std::fs::create_dir_all(&p).expect("create scratch dir");
    p
}

pub fn scratch(name: &str) -> PathBuf {
    scratch_dir().join(name)
}

pub fn cstr(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

// ---------------------------------------------------------------------------
// stderr capture
// ---------------------------------------------------------------------------

/// Run `f` with fd 2 redirected to a fresh file and return `(result, stderr)`.
///
/// The test binary, the C `.so` and the Rust `.so` all share a single
/// `libc.so.6`, hence a single `stderr` `FILE*`; redirecting the descriptor
/// therefore captures diagnostics from either library.
pub fn with_captured_stderr<R>(tag: &str, f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    /// Restores fd 2 even if `f` unwinds, so panic messages are not swallowed.
    struct Restore {
        saved: i32,
        fd: i32,
    }
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                libc::fflush(std::ptr::null_mut());
                libc::dup2(self.saved, 2);
                libc::close(self.saved);
                libc::close(self.fd);
            }
        }
    }

    let path = scratch(&format!("stderr-{tag}-{}.txt", std::process::id()));
    let cpath = cstr(path.to_str().unwrap());
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(2);
        assert!(saved >= 0, "dup(2) failed");
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open {} failed", path.display());
        assert!(libc::dup2(fd, 2) >= 0, "dup2 failed");
        let r = {
            let _restore = Restore { saved, fd };
            f()
        };
        let bytes = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        (r, bytes)
    }
}

// ---------------------------------------------------------------------------
// Observation helpers: turn opaque C results into comparable values
// ---------------------------------------------------------------------------

/// A comparable snapshot of a `matrix_t*` returned across the FFI boundary.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MatObs {
    Null,
    /// `width`, `height`, whether the row array pointer is non-null, and every
    /// cell (only read when the dimensions make that defined).
    Some {
        width: c_int,
        height: c_int,
        rows_non_null: bool,
        cells: Option<Vec<c_int>>,
    },
}

/// Read a `matrix_t*` into a comparable snapshot. Cells are only read when
/// `0 <= width` and `0 <= height` (otherwise the C never wrote them either).
pub unsafe fn observe(mat: *mut MatrixT) -> MatObs {
    if mat.is_null() {
        return MatObs::Null;
    }
    let w = (*mat).width;
    let h = (*mat).height;
    let rows = (*mat).matrix;
    let cells = if w > 0 && h > 0 && !rows.is_null() {
        let mut v = Vec::with_capacity((w as usize) * (h as usize));
        for i in 0..h as isize {
            let row = *rows.offset(i);
            if row.is_null() {
                return MatObs::Some {
                    width: w,
                    height: h,
                    rows_non_null: true,
                    cells: None,
                };
            }
            for j in 0..w as isize {
                v.push(*row.offset(j));
            }
        }
        Some(v)
    } else {
        Some(Vec::new())
    };
    MatObs::Some {
        width: w,
        height: h,
        rows_non_null: !rows.is_null(),
        cells,
    }
}

/// Write `cells` (row-major) into an already allocated matrix.
pub unsafe fn fill(mat: *mut MatrixT, cells: &[c_int]) {
    assert!(!mat.is_null());
    let w = (*mat).width as isize;
    let h = (*mat).height as isize;
    assert_eq!(cells.len() as isize, w * h);
    let rows = (*mat).matrix;
    for i in 0..h {
        let row = *rows.offset(i);
        for j in 0..w {
            *row.offset(j) = cells[(i * w + j) as usize];
        }
    }
}

/// Snapshot of a `char*` returned across the FFI boundary, then `free`d.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum StrObs {
    Null,
    Bytes(Vec<u8>),
}

pub unsafe fn observe_and_free_cstring(p: *mut c_char) -> StrObs {
    if p.is_null() {
        return StrObs::Null;
    }
    let bytes = CStr::from_ptr(p).to_bytes().to_vec();
    libc::free(p as *mut libc::c_void);
    StrObs::Bytes(bytes)
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(salt: u64) -> Self {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }
    pub fn i32_full(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Value whose decimal form is at most 10 characters, so that the C
    /// `matrix_to_string` buffer arithmetic stays inside its allocation.
    pub fn safe_cell(&mut self) -> i32 {
        self.range(-999_999_999, 999_999_999) as i32
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// Forked-child execution (for OOM injection and for faulting inputs)
// ---------------------------------------------------------------------------

/// How a forked child finished.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Exit {
    Code(i32),
    Signal(i32),
    Timeout,
}

/// Fork, optionally cap the child's address space, redirect the child's stderr
/// into `stderr_path`, run `body`, and report how it finished.
///
/// `body` receives a raw fd it may use to report a payload; the parent returns
/// whatever bytes were written to `payload_path`.
///
/// glibc's `fork()` takes the malloc and stdio locks around the fork and
/// re-initialises them in the child, so allocation in the child is safe here.
pub fn run_in_child(
    tag: &str,
    extra_address_space: Option<u64>,
    body: impl FnOnce(i32),
) -> (Exit, Vec<u8>, Vec<u8>) {
    let stderr_path = scratch(&format!("child-{tag}-stderr.txt"));
    let payload_path = scratch(&format!("child-{tag}-payload.bin"));
    let _ = std::fs::remove_file(&stderr_path);
    let _ = std::fs::write(&payload_path, b"");

    // Everything that needs to allocate happens before the fork.
    let c_err = cstr(stderr_path.to_str().unwrap());
    let c_pay = cstr(payload_path.to_str().unwrap());
    let vm_size = current_vm_size_bytes();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            let efd = libc::open(
                c_err.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                0o600,
            );
            if efd >= 0 {
                libc::dup2(efd, 2);
                libc::close(efd);
            }
            let pfd = libc::open(
                c_pay.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                0o600,
            );
            if let Some(extra) = extra_address_space {
                let lim = libc::rlimit {
                    rlim_cur: vm_size.saturating_add(extra),
                    rlim_max: vm_size.saturating_add(extra),
                };
                libc::setrlimit(libc::RLIMIT_AS, &lim);
            }
            body(pfd);
            libc::fflush(std::ptr::null_mut());
            libc::fsync(pfd);
            libc::_exit(0);
        }

        // ---- parent ----
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let mut status: c_int = 0;
        let exit = loop {
            let r = libc::waitpid(pid, &mut status, libc::WNOHANG);
            if r == pid {
                break if libc::WIFEXITED(status) {
                    Exit::Code(libc::WEXITSTATUS(status))
                } else if libc::WIFSIGNALED(status) {
                    Exit::Signal(libc::WTERMSIG(status))
                } else {
                    Exit::Code(-1)
                };
            }
            if std::time::Instant::now() > deadline {
                libc::kill(pid, libc::SIGKILL);
                libc::waitpid(pid, &mut status, 0);
                break Exit::Timeout;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        };

        let err = std::fs::read(&stderr_path).unwrap_or_default();
        let pay = std::fs::read(&payload_path).unwrap_or_default();
        let _ = std::fs::remove_file(&stderr_path);
        let _ = std::fs::remove_file(&payload_path);
        (exit, err, pay)
    }
}

pub fn current_vm_size_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = s
        .split_whitespace()
        .next()
        .and_then(|x| x.parse().ok())
        .unwrap_or(0);
    pages * 4096
}

/// Allocation-free report of a small payload from inside a child.
pub unsafe fn report(fd: i32, bytes: &[u8]) {
    if fd >= 0 {
        libc::write(fd, bytes.as_ptr() as *const libc::c_void, bytes.len());
    }
}

// glibc `mallopt` parameters (`<malloc.h>`).
unsafe extern "C" {
    fn mallopt(param: c_int, value: c_int) -> c_int;
}
const M_TRIM_THRESHOLD: c_int = -1;
const M_TOP_PAD: c_int = -2;
const M_MMAP_THRESHOLD: c_int = -3;

/// glibc refuses `M_MMAP_THRESHOLD` above `DEFAULT_MMAP_THRESHOLD_MAX`, which is
/// `4 * 1024 * 1024 * sizeof(long)` = 32 MiB on LP64. Passing anything larger
/// makes `mallopt` return 0 and leave the threshold at its default 128 KiB.
const MMAP_THRESHOLD_MAX: c_int = 32 * 1024 * 1024;

/// Force every allocation through the main heap (`brk`) and stop `free` from
/// handing pages back, so that a heap budget established by
/// [`constrain_heap_to`] cannot be sidestepped by `mmap`/`munmap`.
///
/// Returns `true` only if glibc accepted every setting.
pub unsafe fn pin_allocations_to_heap() -> bool {
    let a = mallopt(M_MMAP_THRESHOLD, MMAP_THRESHOLD_MAX);
    let b = mallopt(M_TRIM_THRESHOLD, i32::MAX);
    let c = mallopt(M_TOP_PAD, 0);
    a == 1 && b == 1 && c == 1
}

/// Fault in a generous slice of stack so that later calls never need the kernel
/// to extend the stack VMA (which would fail once the address space is capped).
#[inline(never)]
pub unsafe fn pretouch_stack() {
    const N: usize = 1 << 20;
    let mut buf = std::mem::MaybeUninit::<[u8; N]>::uninit();
    let p = buf.as_mut_ptr() as *mut u8;
    let mut i = 0usize;
    while i < N {
        std::ptr::write_volatile(p.add(i), 0u8);
        i += 4096;
    }
    std::ptr::read_volatile(p);
}

/// Leave *exactly* `reserve` bytes of allocatable heap behind.
///
/// An `RLIMIT_AS` cap on its own is not enough to make a specific `malloc`
/// fail: a process always carries a pool of already-mapped but free heap that
/// requests can be carved out of without growing the address space. So instead
/// of guessing that pool's size, reserve a block of the wanted budget, consume
/// absolutely everything else, and only then release the reservation.
///
/// Returns `false` if the reservation or the exhaustion did not take, which the
/// caller should surface rather than silently mis-testing.
pub unsafe fn constrain_heap_to(reserve: usize) -> bool {
    let pinned = pin_allocations_to_heap();
    pretouch_stack();
    let r = libc::malloc(reserve);
    if r.is_null() {
        return false;
    }
    std::ptr::write_bytes(r as *mut u8, 0, 1);
    let exhausted = exhaust_heap();
    libc::free(r);
    pinned && exhausted
}

/// Consume every byte of remaining heap so that subsequent `malloc`s fail.
/// Must be called after `RLIMIT_AS` has been tightened.
///
/// Bounded on purpose: if the cap is reached before `malloc` starts failing the
/// injection did not take, and the caller's assertion on the reported payload
/// will say so rather than the test hanging.
pub unsafe fn exhaust_heap() -> bool {
    const MAX_ALLOCS: u64 = 8_000_000;
    // Once the address space is full, *any* further growth fails — including an
    // on-demand expansion of the main thread's stack, which would kill the
    // process with SIGSEGV before the code under test ever runs. Fault the
    // stack pages in first so no growth is needed afterwards.
    pretouch_stack();
    let mut n: u64 = 0;
    for sz in [1usize << 20, 1 << 16, 1 << 12, 256, 64, 16] {
        loop {
            let p = libc::malloc(sz);
            if p.is_null() {
                break;
            }
            // Touch it so the mapping is real; never freed.
            std::ptr::write_bytes(p as *mut u8, 0, 1);
            n += 1;
            if n > MAX_ALLOCS {
                return false;
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Re-exec'd child ("child mode") for precise allocation-failure injection
// ---------------------------------------------------------------------------
//
// A forked child inherits the parent's heap, including everything earlier tests
// allocated and freed. That free pool lets `malloc` satisfy sizeable requests
// without growing the address space, which defeats an `RLIMIT_AS` cap that is
// only a few hundred kilobytes wide. Re-executing the test binary gives a
// pristine heap, so the cap lands exactly where it is meant to.

pub struct ChildCfg {
    /// `"c"` or `"rust"` — which shared object the child should drive.
    pub lib: String,
    /// Extra address space, in bytes, to allow on top of the child's own
    /// `VmSize` measured just before the payload runs.
    pub slack: Option<u64>,
    /// Where the child writes its result payload.
    pub out: PathBuf,
}

/// Returns `Some(cfg)` when this process was launched as a re-exec'd child.
pub fn child_cfg() -> Option<ChildCfg> {
    let lib = std::env::var("DIFFTEST_CHILD_LIB").ok()?;
    let out = PathBuf::from(std::env::var("DIFFTEST_CHILD_OUT").ok()?);
    let slack = std::env::var("DIFFTEST_CHILD_SLACK")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|v| *v > 0);
    Some(ChildCfg { lib, slack, out })
}

impl ChildCfg {
    pub fn api(&self) -> &'static Api {
        if self.lib == "c" {
            c_api()
        } else {
            rust_api()
        }
    }

    /// Cap this process's address space at `VmSize + slack`, measured now.
    /// Call immediately before the payload, after all setup allocations.
    pub fn apply_limit(&self) {
        if let Some(extra) = self.slack {
            let vm = current_vm_size_bytes();
            unsafe {
                let lim = libc::rlimit {
                    rlim_cur: vm.saturating_add(extra),
                    rlim_max: vm.saturating_add(extra),
                };
                libc::setrlimit(libc::RLIMIT_AS, &lim);
            }
        }
    }

    /// Open the result file and return a raw fd. Call this **before**
    /// `apply_limit()` / `exhaust_heap()`, because writing the payload later
    /// must not need to allocate.
    pub fn open_out(&self) -> i32 {
        let p = cstr(self.out.to_str().unwrap());
        unsafe {
            libc::open(
                p.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                0o600,
            )
        }
    }

    /// Terminate immediately without running any destructors (never returns).
    pub fn exit_now(&self, fd: i32) -> ! {
        unsafe {
            libc::fsync(fd);
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0)
        }
    }
}

/// Re-exec this test binary so that `test_name` runs in child mode against
/// `lib`, with `slack` bytes of address space head-room.
pub fn spawn_child(
    test_name: &str,
    lib: &str,
    slack: Option<u64>,
) -> (Exit, Vec<u8>, Vec<u8>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = scratch(&format!("selfchild-{test_name}-{lib}.out"));
    let errp = scratch(&format!("selfchild-{test_name}-{lib}.err"));
    let _ = std::fs::remove_file(&out);
    let errf = std::fs::File::create(&errp).expect("create child stderr file");

    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("DIFFTEST_CHILD_LIB", lib)
        .env("DIFFTEST_CHILD_OUT", &out)
        .env("DIFFTEST_CHILD_SLACK", slack.unwrap_or(0).to_string())
        .env("DRIVER_C_SO", c_so_path())
        .env("DRIVER_RUST_SO", rust_so_path())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(errf);

    let mut child = cmd.spawn().expect("spawn child");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (
                        Exit::Timeout,
                        std::fs::read(&errp).unwrap_or_default(),
                        std::fs::read(&out).unwrap_or_default(),
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    };
    let exit = if let Some(code) = status.code() {
        Exit::Code(code)
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            Exit::Signal(status.signal().unwrap_or(-1))
        }
        #[cfg(not(unix))]
        Exit::Code(-1)
    };
    let payload = std::fs::read(&out).unwrap_or_default();
    let stderr = std::fs::read(&errp).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&errp);
    (exit, stderr, payload)
}
