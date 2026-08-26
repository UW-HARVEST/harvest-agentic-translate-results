// Shared differential-test harness.
//
// Both the C shared library (`c_src/build/libdriver.so`) and the Rust shared
// library (`target/{debug,release}/libdriver.so`) are loaded with `libloading`
// and driven exclusively through their exported C symbols, so the `#[no_mangle]`
// export wrappers are part of what is under test. No Rust function is ever
// called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

pub type FmaFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
pub type DriverFn = unsafe extern "C" fn(*const c_int, c_int);

pub const INT_MAX: c_int = c_int::MAX;
pub const INT_MIN: c_int = c_int::MIN;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: &'static Library,
    pub fma_array: FmaFn,
    pub driver: DriverFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load(name: &'static str, path: PathBuf) -> Lib {
    assert!(
        path.exists(),
        "shared library {} not found at {}\n\
         (build the C library with:\n\
         \x20 cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         and the Rust library with `cargo build`)",
        name,
        path.display()
    );
    // Leaked on purpose: the function pointers below must stay valid for the
    // whole process lifetime, and the library must never be unloaded.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));
    let fma: Symbol<FmaFn> = unsafe {
        lib.get(b"fma_array\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol `fma_array`: {e}"))
    };
    let drv: Symbol<DriverFn> = unsafe {
        lib.get(b"driver\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"))
    };
    Lib {
        name,
        path,
        fma_array: *fma,
        driver: *drv,
        _lib: lib,
    }
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // `cargo test` builds the cdylib next to the test binaries' parent dir.
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>/
    if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
        let cand = profile_dir.join("libdriver.so");
        if cand.exists() {
            return cand;
        }
    }
    for p in ["target/debug/libdriver.so", "target/release/libdriver.so"] {
        let cand = manifest_dir().join(p);
        if cand.exists() {
            return cand;
        }
    }
    manifest_dir().join("target/debug/libdriver.so")
}

pub fn c_lib() -> &'static Lib {
    static S: OnceLock<Lib> = OnceLock::new();
    S.get_or_init(|| load("C", c_so_path()))
}

pub fn rust_lib() -> &'static Lib {
    static S: OnceLock<Lib> = OnceLock::new();
    S.get_or_init(|| load("Rust", rust_so_path()))
}

/// Force both libraries to be `dlopen`ed before any `fork()` happens.
pub fn preload() {
    let _ = c_lib();
    let _ = rust_lib();
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x5EED_1234_ABCD_0001)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Full-range `int` (overflow-heavy).
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// `|v| <= 46340`, so `v*v` never overflows an `int`.
    pub fn small_i32(&mut self) -> c_int {
        (self.next_u64() % 92_681) as i64 as c_int - 46_340
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn fill_full(&mut self, v: &mut [c_int]) {
        for x in v.iter_mut() {
            *x = self.next_i32();
        }
    }
    pub fn fill_small(&mut self, v: &mut [c_int]) {
        for x in v.iter_mut() {
            *x = self.small_i32();
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with fd 1 redirected to a temporary file and returns everything
/// that was written to it. Both libraries share the process' libc `stdout`
/// stream, so `fflush(NULL)` before and after makes the comparison exact.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let out = unsafe { capture_stdout_inner(f) };
    drop(guard);
    out
}

/// Restores fd 1 even if the closure panics, so a failing assertion cannot
/// swallow the whole test binary's output.
struct FdRestore {
    saved: c_int,
    done: bool,
}

impl FdRestore {
    fn restore(&mut self) {
        if !self.done {
            self.done = true;
            unsafe {
                libc::fflush(std::ptr::null_mut());
                libc::dup2(self.saved, 1);
                libc::close(self.saved);
            }
        }
    }
}

impl Drop for FdRestore {
    fn drop(&mut self) {
        self.restore();
    }
}

unsafe fn capture_stdout_inner<F: FnOnce()>(f: F) -> Vec<u8> {
    libc::fflush(std::ptr::null_mut());

    let saved = libc::dup(1);
    assert!(saved >= 0, "dup(1) failed");
    let mut restore = FdRestore { saved, done: false };

    let mut template: Vec<u8> = std::env::temp_dir()
        .join("driver-cap-XXXXXX")
        .into_os_string()
        .into_string()
        .expect("temp dir is not utf-8")
        .into_bytes();
    template.push(0);
    let fd = libc::mkstemp(template.as_mut_ptr() as *mut c_char);
    assert!(fd >= 0, "mkstemp failed");
    libc::unlink(template.as_ptr() as *const c_char);

    assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

    f();

    restore.restore();

    libc::lseek(fd, 0, libc::SEEK_SET);
    let mut out = Vec::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = libc::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    libc::close(fd);
    out
}

// ---------------------------------------------------------------------------
// Crash-disposition comparison (for the undefined-behaviour / fault rows)
// ---------------------------------------------------------------------------

/// Wall-clock budget for a forked child. A child that exceeds it is killed by
/// `SIGALRM`, which both libraries are subject to equally.
pub const CHILD_TIMEOUT_SECS: u32 = 2;

/// Cap on how much of a child's stdout is retained (a runaway print loop can
/// emit unboundedly much).
pub const CHILD_OUTPUT_CAP: usize = 4 << 20;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Disposition {
    Exited(i32),
    Signaled(i32),
    Other(i32),
}

impl std::fmt::Display for Disposition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Disposition::Exited(c) => write!(f, "exited({c})"),
            Disposition::Signaled(s) => write!(f, "killed by signal {s}"),
            Disposition::Other(s) => write!(f, "raw status {s:#x}"),
        }
    }
}

/// Removes the *test harness's* own signal interference from a child before it
/// calls into either library.
///
/// Rust's runtime installs a `SIGSEGV`/`SIGBUS` handler that reclassifies a fault
/// landing in the thread's guard page as "stack overflow" and turns it into a
/// non-unwinding abort (`SIGABRT`). That handler belongs to the test binary, not
/// to either library under test, and it can classify the C's and the Rust's
/// faults *differently* merely because their stack frames differ by a few bytes.
/// Restoring the default dispositions makes a fault a fault for both, which is
/// also what either library would see when loaded by a plain C program.
unsafe fn reset_fault_signals() {
    libc::signal(libc::SIGSEGV, libc::SIG_DFL);
    libc::signal(libc::SIGBUS, libc::SIG_DFL);
    libc::signal(libc::SIGILL, libc::SIG_DFL);
    libc::signal(libc::SIGFPE, libc::SIG_DFL);
    libc::signal(libc::SIGABRT, libc::SIG_DFL);
}

/// Runs `f` in a forked child (stdout sent to `/dev/null`) and reports how the
/// child terminated. Used for the rows where the C library faults: the two
/// implementations must fault the *same* way (same signal number), not merely
/// "both fail somehow".
pub fn run_in_child<F: FnOnce()>(f: F) -> Disposition {
    preload();
    // Serialised against `capture_stdout` so that no other thread is holding a
    // redirected fd 1 / a dirty libc stdout buffer while we fork.
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 1);
                libc::dup2(devnull, 2);
            }
            // The C library's undefined-behaviour paths can smash their own stack
            // frame badly enough to spin forever (e.g. `driver` with certain
            // negative lengths re-enters its print loop with a clobbered `len`).
            // Turn that into a deterministic, comparable disposition instead of a
            // hung test run.
            libc::alarm(CHILD_TIMEOUT_SECS);
            reset_fault_signals();
            f();
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
        let mut status: c_int = 0;
        loop {
            let r = libc::waitpid(pid, &mut status, 0);
            if r == pid {
                break;
            }
            if r < 0 && *libc::__errno_location() != libc::EINTR {
                panic!("waitpid failed");
            }
        }
        if libc::WIFEXITED(status) {
            Disposition::Exited(libc::WEXITSTATUS(status))
        } else if libc::WIFSIGNALED(status) {
            Disposition::Signaled(libc::WTERMSIG(status))
        } else {
            Disposition::Other(status)
        }
    }
}

/// Runs `f` in a forked child, capturing the child's stdout through a pipe, and
/// returns both the termination disposition and everything the child printed.
pub fn run_in_child_capturing<F: FnOnce()>(f: F) -> (Disposition, Vec<u8>) {
    preload();
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let mut fds = [0 as c_int; 2];
        assert!(libc::pipe(fds.as_mut_ptr()) == 0, "pipe failed");
        let (rd, wr) = (fds[0], fds[1]);

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            libc::close(rd);
            libc::dup2(wr, 1);
            libc::close(wr);
            libc::alarm(CHILD_TIMEOUT_SECS);
            reset_fault_signals();
            f();
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
        libc::close(wr);

        let mut out = Vec::new();
        let mut buf = [0u8; 65536];
        let mut total = 0usize;
        loop {
            let n = libc::read(rd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            total += n as usize;
            if out.len() < CHILD_OUTPUT_CAP {
                out.extend_from_slice(&buf[..n as usize]);
            }
        }
        let _ = total;
        libc::close(rd);

        let mut status: c_int = 0;
        loop {
            let r = libc::waitpid(pid, &mut status, 0);
            if r == pid {
                break;
            }
            if r < 0 && *libc::__errno_location() != libc::EINTR {
                panic!("waitpid failed");
            }
        }
        let disp = if libc::WIFEXITED(status) {
            Disposition::Exited(libc::WEXITSTATUS(status))
        } else if libc::WIFSIGNALED(status) {
            Disposition::Signaled(libc::WTERMSIG(status))
        } else {
            Disposition::Other(status)
        };
        (disp, out)
    }
}

/// Assert that the C and the Rust library terminate identically for a faulting
/// input.
pub fn assert_same_fault(what: &str, c_call: impl FnOnce(), rust_call: impl FnOnce()) {
    let c = run_in_child(c_call);
    let r = run_in_child(rust_call);
    assert_eq!(
        c, r,
        "[{what}] disposition mismatch: C {c}, Rust {r}\n\
         (both implementations must fail the same way)"
    );
}

// ---------------------------------------------------------------------------
// Guard-page-protected buffers (for the "one past the end" rows)
// ---------------------------------------------------------------------------

/// A buffer of `n` `int`s whose last element ends exactly at a `PROT_NONE`
/// guard page, so that reading or writing element `n` faults deterministically.
pub struct GuardedInts {
    map: *mut u8,
    map_len: usize,
    data: *mut c_int,
    n: usize,
}

impl GuardedInts {
    pub fn new(n: usize) -> Self {
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let bytes = n * std::mem::size_of::<c_int>();
        let data_pages = bytes.div_ceil(page).max(1);
        let map_len = (data_pages + 1) * page;
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(map != libc::MAP_FAILED, "mmap failed");
        let map = map as *mut u8;
        // Everything but the final guard page becomes readable/writable.
        assert!(
            unsafe { libc::mprotect(map as *mut c_void, data_pages * page, libc::PROT_READ | libc::PROT_WRITE) } == 0,
            "mprotect failed"
        );
        // Place the array so that its end touches the guard page.
        let data = unsafe { map.add(data_pages * page - bytes) } as *mut c_int;
        Self { map, map_len, data, n }
    }

    pub fn ptr(&self) -> *mut c_int {
        self.data
    }
    pub fn len(&self) -> usize {
        self.n
    }
    pub fn as_slice(&self) -> &[c_int] {
        unsafe { std::slice::from_raw_parts(self.data, self.n) }
    }
    pub fn as_mut_slice(&mut self) -> &mut [c_int] {
        unsafe { std::slice::from_raw_parts_mut(self.data, self.n) }
    }
}

impl Drop for GuardedInts {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.map as *mut c_void, self.map_len);
        }
    }
}

// ---------------------------------------------------------------------------
// Reference model of the C semantics (used only as an extra cross-check)
// ---------------------------------------------------------------------------

/// `out[i] = mul1[i] * mul2[i] + add[i]`, two's-complement wrapping, ascending.
pub fn model_fma(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

/// What `driver(data, len)` must print: `x*x + x` per element, `%d\n`.
pub fn model_driver_stdout(data: &[c_int]) -> Vec<u8> {
    let mut s = String::new();
    for &x in data {
        let v = x.wrapping_mul(x).wrapping_add(x);
        s.push_str(&v.to_string());
        s.push('\n');
    }
    s.into_bytes()
}

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

pub fn hex_head(v: &[u8], n: usize) -> String {
    String::from_utf8_lossy(&v[..v.len().min(n)]).to_string()
}

/// Diff two byte streams and describe the first difference.
pub fn describe_diff(a: &[u8], b: &[u8]) -> String {
    if a == b {
        return "identical".into();
    }
    let mut i = 0;
    while i < a.len() && i < b.len() && a[i] == b[i] {
        i += 1;
    }
    let lo = i.saturating_sub(40);
    format!(
        "first difference at byte {i} (len C={} Rust={})\n  C   ...{:?}\n  Rust...{:?}",
        a.len(),
        b.len(),
        String::from_utf8_lossy(&a[lo..a.len().min(i + 40)]),
        String::from_utf8_lossy(&b[lo..b.len().min(i + 40)]),
    )
}

/// Run `driver` on both libraries with the same input and compare stdout.
pub fn diff_driver(label: &str, data: &[c_int], len: c_int) {
    let c = capture_stdout(|| unsafe { (c_lib().driver)(data.as_ptr(), len) });
    let r = capture_stdout(|| unsafe { (rust_lib().driver)(data.as_ptr(), len) });
    assert!(
        c == r,
        "[{label}] driver(len={len}) stdout mismatch: {}",
        describe_diff(&c, &r)
    );
    // Cross-check against the independent model, so a shared bug cannot hide.
    if len >= 0 {
        let expect = model_driver_stdout(&data[..len as usize]);
        assert!(
            c == expect,
            "[{label}] driver(len={len}) disagrees with the reference model: {}",
            describe_diff(&c, &expect)
        );
    }
}

/// Run `fma_array` on both libraries with the same (independently allocated)
/// buffers and compare the full destination buffer plus stdout.
///
/// `layout` receives a mutable scratch buffer and returns
/// `(out_offset, mul1_offset, mul2_offset, add_offset)` element offsets into it,
/// which is how every aliasing/overlap configuration is expressed.
pub fn diff_fma_layout(
    label: &str,
    scratch: &[c_int],
    offs: (usize, usize, usize, usize),
    len: c_int,
) {
    let run = |lib: &Lib| -> (Vec<c_int>, Vec<u8>) {
        let mut buf = scratch.to_vec();
        let base = buf.as_mut_ptr();
        let out = capture_stdout(|| unsafe {
            (lib.fma_array)(
                base.add(offs.0),
                base.add(offs.1),
                base.add(offs.2),
                base.add(offs.3),
                len,
            )
        });
        (buf, out)
    };
    let (cb, co) = run(c_lib());
    let (rb, ro) = run(rust_lib());
    assert!(
        cb == rb,
        "[{label}] fma_array(len={len}, offs={offs:?}) buffer mismatch\n C   ={:?}\n Rust={:?}",
        &cb[..cb.len().min(64)],
        &rb[..rb.len().min(64)]
    );
    assert!(
        co == ro,
        "[{label}] fma_array unexpectedly produced differing stdout: {}",
        describe_diff(&co, &ro)
    );
    assert!(
        co.is_empty(),
        "[{label}] fma_array must not print anything, got {:?}",
        hex_head(&co, 64)
    );
}
