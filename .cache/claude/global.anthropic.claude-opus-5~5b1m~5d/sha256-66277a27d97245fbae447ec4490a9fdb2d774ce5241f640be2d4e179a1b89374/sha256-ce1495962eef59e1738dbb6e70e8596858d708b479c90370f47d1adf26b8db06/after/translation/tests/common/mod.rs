// Shared differential-test harness.
//
// Both implementations are loaded as SHARED OBJECTS through `libloading` and
// called only through their exported `driver` symbol -- the Rust code is never
// called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.
//
// `driver` has no return value; its entire observable behaviour is the bytes it
// writes to stdout via libc. So the harness redirects fd 1 to a temp file around
// each call and compares the captured bytes.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn _exit(code: c_int) -> !;
    fn poll(fds: *mut PollFd, nfds: u64, timeout: c_int) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/libdriver.so`
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// `<crate>/target/{debug,release}/libdriver.so` (the cdylib cargo just built).
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("target"));
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in preferred {
        let p = target.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib libdriver.so not found under {target:?}; run `cargo build`");
}

struct Libs {
    c: Library,
    rust: Library,
}

// Both libraries stay loaded for the whole process.
fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        Libs {
            c: Library::new(c_so_path()).expect("dlopen C .so"),
            rust: Library::new(rust_so_path()).expect("dlopen Rust .so"),
        }
    })
}

pub fn c_driver() -> DriverFn {
    unsafe {
        let s: Symbol<DriverFn> = libs().c.get(b"driver\0").expect("C symbol `driver`");
        *s
    }
}

pub fn rust_driver() -> DriverFn {
    unsafe {
        let s: Symbol<DriverFn> = libs().rust.get(b"driver\0").expect("Rust symbol `driver`");
        *s
    }
}

fn cap_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

static SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with fd 1 redirected to a temp file and returns everything written.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    // fd 1 is process-wide: if the libtest harness (or another test thread) can
    // write to stdout concurrently, its bytes land in our capture file. Refuse to
    // run unless the harness is single-threaded.
    assert_eq!(
        std::env::var("RUST_TEST_THREADS").as_deref(),
        Ok("1"),
        "stdout capture requires a single-threaded harness; run:\n  \
         RUST_TEST_THREADS=1 cargo test --release -- --test-threads=1\n  \
         (or use ./run_all_combos.sh)"
    );
    let _guard = cap_lock();
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("driver_cap_{}_{}.bin", std::process::id(), n));

    let bytes = unsafe {
        fflush(std::ptr::null_mut()); // flush all streams before swapping fd 1
        let file = fs::File::create(&path).expect("create capture file");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        drop(file);

        let mut buf = Vec::new();
        fs::File::open(&path)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        buf
    };
    let _ = fs::remove_file(&path);
    bytes
}

pub fn run_c(x: c_int, y: c_int) -> Vec<u8> {
    let f = c_driver();
    capture(|| unsafe { f(x, y) })
}

pub fn run_rust(x: c_int, y: c_int) -> Vec<u8> {
    let f = rust_driver();
    capture(|| unsafe { f(x, y) })
}

fn show(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    if s.len() <= 400 {
        format!("{:?} ({} bytes)", s, b.len())
    } else {
        format!(
            "{:?} ... {:?} ({} bytes)",
            &s[..200],
            &s[s.len() - 200..],
            b.len()
        )
    }
}

/// Core differential assertion: C and Rust must emit byte-identical stdout.
#[track_caller]
pub fn assert_same(x: c_int, y: c_int) -> Vec<u8> {
    let c = run_c(x, y);
    let r = run_rust(x, y);
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.len().min(r.len()));
        panic!(
            "divergence for driver({x}, {y}) at byte {first}\n  C   : {}\n  Rust: {}",
            show(&c),
            show(&r)
        );
    }
    c
}

/// Same, but also pins the exact expected bytes (regression-proofs the C model).
#[track_caller]
pub fn assert_same_and_eq(x: c_int, y: c_int, expected: &str) {
    let got = assert_same(x, y);
    assert_eq!(
        String::from_utf8_lossy(&got),
        expected,
        "unexpected (but matching) output for driver({x}, {y})"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

// ---------------------------------------------------------------------------
// Unbounded-path support: run one implementation in a forked child whose stdout
// is a pipe, read a prefix, then kill it. Used for inputs where the C code
// loops ~2^31 times (x > 0 with y < 0) and therefore cannot be run to
// completion.
// ---------------------------------------------------------------------------

pub fn prefix_via_fork(f: DriverFn, x: c_int, y: c_int, want: usize) -> Vec<u8> {
    const SIGKILL: c_int = 9;

    // Pre-warm libc's stdout machinery in the parent so the child does no
    // first-time allocation while holding a possibly-forked malloc lock.
    unsafe {
        printf(b"\0".as_ptr() as *const c_char);
        fflush(std::ptr::null_mut());
    }

    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe failed");
    let (rd, wr) = (fds[0], fds[1]);

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // child
        unsafe {
            close(rd);
            dup2(wr, 1);
            close(wr);
            f(x, y);
            fflush(std::ptr::null_mut());
            _exit(0);
        }
    }
    unsafe { close(wr) };

    let mut out = Vec::with_capacity(want);
    let mut chunk = vec![0u8; 8192];
    while out.len() < want {
        // Never block forever: 10 s without data means give up on this prefix.
        let mut pfd = PollFd {
            fd: rd,
            events: 0x1, /* POLLIN */
            revents: 0,
        };
        let pr = unsafe { poll(&mut pfd, 1, 10_000) };
        if pr <= 0 {
            break;
        }
        let n = unsafe { read(rd, chunk.as_mut_ptr() as *mut c_void, chunk.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&chunk[..n as usize]);
    }
    unsafe {
        kill(pid, SIGKILL);
        close(rd);
        let mut st: c_int = 0;
        waitpid(pid, &mut st, 0);
    }
    out.truncate(want.min(out.len()));
    out
}
