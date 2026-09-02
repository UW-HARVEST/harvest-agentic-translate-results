// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
// only through their exported `driver` symbol, exactly as an external consumer
// would. The Rust implementation is never called directly, so the
// `#[no_mangle] extern "C"` wrapper is under test too.
//
// `driver` reports its result on `stdout`, so the harness redirects file
// descriptor 1 to a temporary file around each batch of calls and reads the
// bytes back. Both libraries share the process's single glibc `stdout`, so
// `fflush(NULL)` is enough to make the capture deterministic.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// fd 1 is process-global, so only one capture may be in flight at a time.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn first_existing(candidates: &[PathBuf], what: &str) -> PathBuf {
    for c in candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not locate the {what} shared object; looked in {:?}. \
         Build the C library with \
         `cd c_src && mkdir -p build && cd build && \
          cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .` \
         and the Rust library with `cd translation && cargo build --release`.",
        candidates
    );
}

/// Path to the C shared object, overridable with `C_DRIVER_SO`.
pub fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    let ws = root.parent().unwrap_or(Path::new("..")).to_path_buf();
    first_existing(
        &[
            ws.join("c_src/build/libdriver.so"),
            ws.join("c_src/build/lib/libdriver.so"),
        ],
        "C",
    )
}

/// Path to the Rust cdylib, overridable with `RUST_DRIVER_SO`.
pub fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    first_existing(
        &[
            root.join("target/release/libdriver.so"),
            root.join("target/debug/libdriver.so"),
        ],
        "Rust",
    )
}

type DriverFn = unsafe extern "C" fn(c_int);

struct Libs {
    c_lib: Library,
    rust_lib: Library,
}

// SAFETY: the loaded libraries hold no thread-affine state; `driver` only
// touches an automatic struct and the shared `stdout`, which the harness
// serialises with `CAPTURE_LOCK`.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        // SAFETY: both paths point at this project's own shared objects.
        unsafe {
            Libs {
                c_lib: Library::new(c_so_path())
                    .unwrap_or_else(|e| panic!("failed to dlopen the C .so: {e}")),
                rust_lib: Library::new(rust_so_path())
                    .unwrap_or_else(|e| panic!("failed to dlopen the Rust .so: {e}")),
            }
        }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    // SAFETY: the symbol's type matches `void driver(int)` from driver.h.
    unsafe { libs().c_lib.get(b"driver\0").expect("C .so exports `driver`") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    // SAFETY: same ABI as the C symbol, by construction of the translation.
    unsafe {
        libs()
            .rust_lib
            .get(b"driver\0")
            .expect("Rust .so exports `driver`")
    }
}

/// True when a symbol is exported by the Rust `.so`.
pub fn rust_exports(name: &str) -> bool {
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    // SAFETY: only queried, never called.
    unsafe { libs().rust_lib.get::<*const c_void>(&bytes).is_ok() }
}

/// True when a symbol is exported by the C `.so`.
pub fn c_exports(name: &str) -> bool {
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    // SAFETY: only queried, never called.
    unsafe { libs().c_lib.get::<*const c_void>(&bytes).is_ok() }
}

/// Runs `f` in a forked child with fd 1 redirected to a temp file, and returns
/// everything the child wrote.
///
/// The fork is what makes this reliable: the test harness itself writes progress
/// lines to the real fd 1 from other threads, so redirecting fd 1 *in-process*
/// would splice libtest's output into the captured stream. In the child, fd 1
/// belongs to us alone. The child leaves via `_exit`, which skips Rust's
/// `stdout` flush, so no inherited buffer contents land in the capture either.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "driver_diff_capture_{}_{}.bin",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let c_path = std::ffi::CString::new(tmp.as_os_str().as_encoded_bytes()).unwrap();

    // Pre-touch the C `stdout` buffer in the parent so the child's `printf`
    // calls do not need to allocate (keeps the post-fork path allocation-free).
    // SAFETY: writing zero bytes to the C `stdout` is a no-op flush.
    unsafe {
        fflush(std::ptr::null_mut());
    }

    // SAFETY: `fork` here is safe because every capture is serialised by
    // `CAPTURE_LOCK`, the child performs only `open`/`dup2`/`printf`/`_exit`,
    // and it never returns to the Rust test harness.
    let status = unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            let fd = open(c_path.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
            if fd < 0 {
                _exit(101);
            }
            if dup2(fd, 1) < 0 {
                _exit(102);
            }
            close(fd);

            f();

            if fflush(std::ptr::null_mut()) != 0 {
                _exit(103);
            }
            _exit(0);
        }

        let mut status: c_int = 0;
        let waited = waitpid(pid, &mut status, 0);
        assert_eq!(waited, pid, "waitpid() failed");
        status
    };

    // WIFEXITED / WEXITSTATUS
    let exited_normally = (status & 0x7f) == 0;
    let code = (status >> 8) & 0xff;
    assert!(
        exited_normally && code == 0,
        "capture child terminated abnormally (raw wait status {status:#x}); \
         a panic or crash inside the loaded library is the likely cause"
    );

    let bytes = std::fs::read(&tmp).expect("reading captured stdout");
    let _ = std::fs::remove_file(&tmp);
    drop(guard);
    bytes
}

/// Captured `stdout` of the C `driver` for each input, in order.
pub fn c_outputs(inputs: &[i32]) -> Vec<Vec<u8>> {
    let f = c_driver();
    // SAFETY: `driver` takes any `int`; there is no invalid value.
    let raw = capture(|| {
        for &x in inputs {
            unsafe { f(x) }
        }
    });
    split_lines(&raw, inputs.len(), "C")
}

/// Captured `stdout` of the Rust `driver` for each input, in order.
pub fn rust_outputs(inputs: &[i32]) -> Vec<Vec<u8>> {
    let f = rust_driver();
    // SAFETY: same contract as the C symbol.
    let raw = capture(|| {
        for &x in inputs {
            unsafe { f(x) }
        }
    });
    split_lines(&raw, inputs.len(), "Rust")
}

fn split_lines(raw: &[u8], expected: usize, which: &str) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::with_capacity(expected);
    let mut cur: Vec<u8> = Vec::new();
    for &b in raw {
        cur.push(b);
        if b == b'\n' {
            out.push(std::mem::take(&mut cur));
        }
    }
    assert!(
        cur.is_empty(),
        "{which} .so produced trailing bytes with no terminating newline: {:?}",
        String::from_utf8_lossy(&cur)
    );
    assert_eq!(
        out.len(),
        expected,
        "{which} .so produced {} newline-terminated records for {expected} calls",
        out.len()
    );
    out
}

/// Core differential assertion: same inputs, byte-identical outputs.
pub fn assert_same(label: &str, inputs: &[i32]) {
    assert!(!inputs.is_empty(), "{label}: empty input set");
    let c = c_outputs(inputs);
    let r = rust_outputs(inputs);
    for (i, x) in inputs.iter().enumerate() {
        assert_eq!(
            c[i],
            r[i],
            "{label}: divergence for driver({x}) (input index {i})\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c[i]),
            String::from_utf8_lossy(&r[i]),
        );
    }
}

/// Differential assertion where the two libraries are called alternately in one
/// captured stream, so any shared-`stdout` or residual-state divergence shows up.
pub fn assert_same_interleaved(label: &str, inputs: &[i32]) {
    let cf = c_driver();
    let rf = rust_driver();
    // SAFETY: both symbols are `void driver(int)`.
    let raw = capture(|| {
        for &x in inputs {
            unsafe {
                cf(x);
                rf(x);
            }
        }
    });
    let recs = split_lines(&raw, inputs.len() * 2, "interleaved");
    for (i, x) in inputs.iter().enumerate() {
        assert_eq!(
            recs[2 * i],
            recs[2 * i + 1],
            "{label}: interleaved divergence for driver({x})\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&recs[2 * i]),
            String::from_utf8_lossy(&recs[2 * i + 1]),
        );
    }
}

/// Deterministic PRNG (SplitMix64) so every randomized row is reproducible.
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
