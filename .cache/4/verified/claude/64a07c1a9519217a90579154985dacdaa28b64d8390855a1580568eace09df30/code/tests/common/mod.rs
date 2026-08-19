// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
// call goes through the exported symbols, exactly as an external consumer would
// do it.  Rust functions are NEVER called directly, so the `#[no_mangle]`
// wrappers and the C ABI are part of what is under test.
//
// stdout capture strategy: the library under test writes to libc's `stdout`
// (fd 1), so capturing it means redirecting fd 1.  Doing that in-process would
// also swallow (and be corrupted by) libtest's own progress output from other
// threads, so instead every measurement happens in a `fork()`ed child which
// redirects *its own* fd 1 into a temp file.  The parent's fd 1 is never
// touched, which makes the capture immune to test-thread interleaving.
// glibc resets the stdio/malloc locks in the child across `fork()`, so calling
// `printf`/`malloc` there is safe.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits we need for capture / fork
// ---------------------------------------------------------------------------
extern "C" {
    fn fflush(stream: *mut u8) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn write(fd: c_int, buf: *const u8, count: usize) -> isize;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// ABI mirror of the (file-private but ABI-public) C `house_t`
//   typedef struct { int floors; int bedrooms; double bathrooms; } house_t;
// offsets 0 / 4 / 8, size 16, align 8
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HouseT {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: f64,
}

impl HouseT {
    pub fn new(floors: i32, bedrooms: i32, bathrooms: f64) -> Self {
        HouseT {
            floors,
            bedrooms,
            bathrooms,
        }
    }
    /// Canonical initializer used by `driver`:
    /// `{.floors = 2, .bedrooms = 5, .bathrooms = 2.5}`
    pub fn canonical() -> Self {
        HouseT::new(2, 5, 2.5)
    }
    /// Raw bytes, so struct mutation is compared bit-for-bit (incl. NaN payloads).
    pub fn raw(&self) -> [u8; 16] {
        unsafe { std::mem::transmute_copy(self) }
    }
}

pub type DriverFn = unsafe extern "C" fn(*const c_char);
pub type RunFn = unsafe extern "C" fn(*mut HouseT, c_int);

pub struct Impl {
    pub name: &'static str,
    pub driver: DriverFn,
    pub run: RunFn,
    _lib: libloading::Library,
}

pub struct Libs {
    pub c: Impl,
    pub rust: Impl,
}

unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn profile_dir() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("profile dir")
        .to_path_buf()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn rust_so_path() -> PathBuf {
    profile_dir().join("libdriver.so")
}

pub fn c_so_path() -> PathBuf {
    profile_dir()
        .parent()
        .expect("target dir")
        .join("c_build")
        .join("libdriver.so")
}

/// Build `c_src` as a shared library via its own CMakeLists (out of tree, so
/// nothing under `c_src/` is touched).  Race-safe across parallel test binaries.
fn ensure_c_so() {
    let so = c_so_path();
    if so.exists() {
        return;
    }
    let build_dir = so.parent().unwrap().to_path_buf();
    let lock = build_dir.with_extension("lock");
    std::fs::create_dir_all(build_dir.parent().unwrap()).ok();
    let got_lock = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .is_ok();
    if got_lock {
        let src = manifest_dir().join("c_src");
        let cfg = std::process::Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build_dir)
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .output()
            .expect("run cmake configure");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );
        let bld = std::process::Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("run cmake build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&bld.stderr)
        );
        std::fs::remove_file(&lock).ok();
    } else {
        for _ in 0..1200 {
            if so.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    assert!(so.exists(), "C shared library not built at {:?}", so);
}

unsafe fn load(name: &'static str, path: &Path) -> Impl {
    let lib =
        libloading::Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e));
    let driver: libloading::Symbol<DriverFn> = lib
        .get(b"driver\0")
        .unwrap_or_else(|e| panic!("dlsym driver in {:?}: {}", path, e));
    let run: libloading::Symbol<RunFn> = lib
        .get(b"run\0")
        .unwrap_or_else(|e| panic!("dlsym run in {:?}: {}", path, e));
    let d = *driver;
    let r = *run;
    Impl {
        name,
        driver: d,
        run: r,
        _lib: lib,
    }
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        ensure_c_so();
        let rust_so = rust_so_path();
        assert!(
            rust_so.exists(),
            "Rust cdylib not found at {:?} (run `cargo build`)",
            rust_so
        );
        unsafe {
            Libs {
                c: load("C", &c_so_path()),
                rust: load("Rust", &rust_so),
            }
        }
    })
}

// ---------------------------------------------------------------------------
// fork-isolated capture
// ---------------------------------------------------------------------------
static SEQ: AtomicU64 = AtomicU64::new(0);

fn tmp_file(tag: &str) -> (std::fs::File, PathBuf) {
    let p = std::env::temp_dir().join(format!(
        "difftest_{}_{}_{}.bin",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst),
        tag
    ));
    let f = std::fs::File::create(&p).expect("create temp file");
    (f, p)
}

pub struct DiffOut {
    pub out_c: Vec<u8>,
    pub out_r: Vec<u8>,
    pub state_c: [u8; 16],
    pub state_r: [u8; 16],
    pub status: c_int,
}

/// Run the C side then the Rust side inside ONE forked child, each with its own
/// redirected stdout, and bring back both byte streams plus 16 bytes of
/// caller-chosen state per side.
pub fn child_diff<F, G>(fc: F, fr: G) -> DiffOut
where
    F: FnOnce() -> [u8; 16],
    G: FnOnce() -> [u8; 16],
{
    let (f_oc, p_oc) = tmp_file("oc");
    let (f_or, p_or) = tmp_file("or");
    let (f_st, p_st) = tmp_file("st");
    let status = unsafe {
        use std::os::unix::io::AsRawFd;
        let (d_oc, d_or, d_st) = (f_oc.as_raw_fd(), f_or.as_raw_fd(), f_st.as_raw_fd());
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child: only this process' fd 1 is redirected ----
            dup2(d_oc, 1);
            let a = fc();
            fflush(std::ptr::null_mut());
            dup2(d_or, 1);
            let b = fr();
            fflush(std::ptr::null_mut());
            write(d_st, a.as_ptr(), 16);
            write(d_st, b.as_ptr(), 16);
            _exit(0);
        }
        let mut st: c_int = -1;
        assert!(waitpid(pid, &mut st, 0) == pid, "waitpid failed");
        st
    };
    drop(f_oc);
    drop(f_or);
    drop(f_st);
    let out_c = std::fs::read(&p_oc).unwrap_or_default();
    let out_r = std::fs::read(&p_or).unwrap_or_default();
    let st_bytes = std::fs::read(&p_st).unwrap_or_default();
    std::fs::remove_file(&p_oc).ok();
    std::fs::remove_file(&p_or).ok();
    std::fs::remove_file(&p_st).ok();
    let mut state_c = [0u8; 16];
    let mut state_r = [0u8; 16];
    if st_bytes.len() == 32 {
        state_c.copy_from_slice(&st_bytes[0..16]);
        state_r.copy_from_slice(&st_bytes[16..32]);
    }
    DiffOut {
        out_c,
        out_r,
        state_c,
        state_r,
        status,
    }
}

/// Capture the stdout bytes produced by `f` (single side).
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let d = child_diff(
        || {
            f();
            [0u8; 16]
        },
        || [0u8; 16],
    );
    assert_eq!(
        exit_code(d.status),
        Some(0),
        "capture child died: {}",
        describe_status(d.status)
    );
    d.out_c
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
}

fn assert_clean(d: &DiffOut, ctx: &str) {
    assert_eq!(
        exit_code(d.status),
        Some(0),
        "differential child died [{ctx}]: {}",
        describe_status(d.status)
    );
}

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

/// Call `run` `n` times on a fresh copy of `init` in both libraries; assert the
/// printed bytes AND the final struct bytes are identical.
pub fn diff_run(init: HouseT, extra: i32, n: usize, ctx: &str) {
    let l = libs();
    let d = child_diff(
        || {
            let mut h = init;
            for _ in 0..n {
                unsafe { (l.c.run)(&mut h, extra) };
            }
            h.raw()
        },
        || {
            let mut h = init;
            for _ in 0..n {
                unsafe { (l.rust.run)(&mut h, extra) };
            }
            h.raw()
        },
    );
    assert_clean(&d, ctx);
    assert_eq!(
        d.out_c,
        d.out_r,
        "run() stdout mismatch [{ctx}]\n  init={init:?} extra={extra} n={n}\n  C   : {}\n  Rust: {}",
        show(&d.out_c),
        show(&d.out_r)
    );
    assert_eq!(
        d.state_c, d.state_r,
        "run() struct mutation mismatch [{ctx}] init={init:?} extra={extra} n={n}"
    );
}

/// Call `driver` with the given NUL-terminated bytes in both libraries and
/// assert identical output.  `bytes` must contain the trailing NUL.
pub fn diff_driver_raw(bytes: &[u8], ctx: &str) -> Vec<u8> {
    assert_eq!(bytes.last(), Some(&0), "input must be NUL terminated");
    let l = libs();
    let p = bytes.as_ptr() as *const c_char;
    let d = child_diff(
        || {
            unsafe { (l.c.driver)(p) };
            [0u8; 16]
        },
        || {
            unsafe { (l.rust.driver)(p) };
            [0u8; 16]
        },
    );
    assert_clean(&d, ctx);
    assert_eq!(
        d.out_c,
        d.out_r,
        "driver() stdout mismatch [{ctx}] input={:?}\n  C   : {}\n  Rust: {}",
        String::from_utf8_lossy(&bytes[..bytes.len() - 1]),
        show(&d.out_c),
        show(&d.out_r)
    );
    d.out_c
}

pub fn diff_driver(s: &str, ctx: &str) -> Vec<u8> {
    let c = CString::new(s).expect("no interior NUL; use diff_driver_raw");
    let b = c.into_bytes_with_nul();
    diff_driver_raw(&b, ctx)
}

pub const REJECT: &[u8] = b"An error occurred\n";

/// Assert that both sides rejected the input, i.e. printed exactly the C error
/// sentinel and nothing else (so "both diverged into the happy path" can't pass).
pub fn assert_rejected(s: &[u8], ctx: &str) {
    let out = diff_driver_raw(s, ctx);
    assert_eq!(
        out,
        REJECT,
        "expected the C reject sentinel [{ctx}], got {}",
        show(&out)
    );
}

/// Assert that both sides accepted the input (8 `print_house` lines, no error line).
pub fn assert_accepted(s: &[u8], ctx: &str) -> Vec<u8> {
    let out = diff_driver_raw(s, ctx);
    assert_eq!(
        out.iter().filter(|&&b| b == b'\n').count(),
        8,
        "expected 8 lines [{ctx}], got {}",
        show(&out)
    );
    assert!(
        !out.starts_with(b"An error"),
        "expected accept [{ctx}], got {}",
        show(&out)
    );
    out
}

// ---------------------------------------------------------------------------
// fatal-signal comparison (for the unchecked-pointer rows)
// ---------------------------------------------------------------------------

/// Run `f` in a forked child with stdout/stderr silenced; return the raw
/// `waitpid` status so C and Rust crash behaviour can be compared exactly.
pub fn child_status<F: FnOnce()>(f: F) -> c_int {
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("open /dev/null");
    unsafe {
        use std::os::unix::io::AsRawFd;
        let nullfd = devnull.as_raw_fd();
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            dup2(nullfd, 1);
            dup2(nullfd, 2);
            f();
            _exit(0);
        }
        let mut status: c_int = -1;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        status
    }
}

pub fn term_signal(status: c_int) -> Option<c_int> {
    let sig = status & 0x7f;
    if sig != 0 && sig != 0x7f {
        Some(sig)
    } else {
        None
    }
}

pub fn exit_code(status: c_int) -> Option<c_int> {
    if status & 0x7f == 0 {
        Some((status >> 8) & 0xff)
    } else {
        None
    }
}

pub fn describe_status(status: c_int) -> String {
    match (term_signal(status), exit_code(status)) {
        (Some(s), _) => format!("killed by signal {s}"),
        (_, Some(c)) => format!("exited with {c}"),
        _ => format!("raw status {status}"),
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*) — fixed seed, reproducible
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x2545F4914F6CDD1D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { SEED } else { seed })
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
    /// uniform in `[lo, hi]`
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// random finite f64 (rejection sampled)
    pub fn next_finite_f64(&mut self) -> f64 {
        loop {
            let v = self.next_f64_bits();
            if v.is_finite() {
                return v;
            }
        }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}
