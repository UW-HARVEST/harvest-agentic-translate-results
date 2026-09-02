//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded with `libloading` (`dlopen`) and every call goes
//! through `dlsym`-resolved function pointers, so the Rust `#[no_mangle]`
//! export wrappers are exercised exactly as an external C consumer would
//! exercise them. No Rust function is ever called directly.
//!
//! The library under test communicates only through `stdout` (every function
//! returns `void`), so the observable behaviour is the exact byte stream each
//! call writes. `capture` redirects fd 1 to a temporary file, runs the call,
//! flushes the C streams, and returns the bytes.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C output stream in the process. Both
    /// `.so`s write through the process's single libc `stdout`, so this is what
    /// makes the redirected bytes visible in the temp file.
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type FnVoid = unsafe extern "C" fn();
pub type FnStr = unsafe extern "C" fn(*const c_char);
pub type FnInt = unsafe extern "C" fn(c_int);

/// The five exported entry points of one library.
pub struct Api {
    pub name: &'static str,
    pub print_line: FnStr,
    pub print_int_line: FnInt,
    pub bad: FnVoid,
    pub good: FnVoid,
    pub driver: FnVoid,
}

impl Api {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
        // Leaked on purpose: the resolved function pointers must stay valid for
        // the whole test-binary lifetime.
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
        }));
        unsafe fn sym<T: Copy>(lib: &'static Library, n: &[u8]) -> T {
            let s: Symbol<T> = unsafe { lib.get(n) }
                .unwrap_or_else(|e| panic!("dlsym({}) failed: {e}", String::from_utf8_lossy(n)));
            *s
        }
        unsafe {
            Api {
                name,
                print_line: sym(lib, b"printLine\0"),
                print_int_line: sym(lib, b"printIntLine\0"),
                bad: sym(lib, b"bad\0"),
                good: sym(lib, b"good\0"),
                driver: sym(lib, b"driver\0"),
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // <root>/translation/  ->  <root>
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // current_exe is target/<profile>/deps/<testbin>; walk up looking for the
    // cdylib so the same test works under debug and release.
    let exe = std::env::current_exe().expect("current_exe");
    for dir in exe.ancestors().skip(1) {
        let cand = dir.join("libdriver.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found near {}. Build it with `cargo build` \
         (or `cargo build --release`) before running the tests.",
        exe.display()
    );
}

/// Both libraries, loaded once per test process.
pub struct Libs {
    pub c: Api,
    pub rust: Api,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        Libs {
            c: Api::load("C", &c_so_path()),
            rust: Api::load("Rust", &rust_so_path()),
        }
    })
}

/// fd 1 is process-global, so captures must not overlap.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f` with fd 1 redirected to a temp file and return everything written.
///
/// The backing file lives under Cargo's target tmpdir and is unlinked as soon as
/// it is opened, so nothing is left behind even if `f` or a later assertion
/// panics; the bytes are read back through the still-open descriptor.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let mut file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");
    // Unlink now; the descriptor keeps the inode alive until `file` is dropped.
    std::fs::remove_file(&path).ok();

    unsafe {
        // Don't let our own buffered output land in the capture file.
        std::io::stdout().flush().ok();
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }

    let mut bytes = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("rewind capture file");
    file.read_to_end(&mut bytes).expect("read capture file");
    drop(guard);
    bytes
}

fn render(b: &[u8]) -> String {
    let shown: Vec<u8> = b.iter().copied().take(400).collect();
    format!(
        "{} bytes: {:?}{}",
        b.len(),
        String::from_utf8_lossy(&shown),
        if b.len() > 400 { " …(truncated)" } else { "" }
    )
}

/// Call the same closure against both libraries and assert byte equality.
pub fn assert_same<F>(case: &str, mut call: F)
where
    F: FnMut(&Api),
{
    let l = libs();
    let c_out = capture(|| call(&l.c));
    let r_out = capture(|| call(&l.rust));
    if c_out != r_out {
        let at = c_out
            .iter()
            .zip(r_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c_out.len().min(r_out.len()));
        panic!(
            "DIVERGENCE [{case}] at byte {at}\n  C   : {}\n  Rust: {}",
            render(&c_out),
            render(&r_out)
        );
    }
}

/// Convenience: `printLine` with an explicit byte string (NUL appended here).
pub fn assert_same_print_line(case: &str, content: &[u8]) {
    assert!(
        !content.contains(&0),
        "a C string cannot contain an interior NUL"
    );
    let mut buf = content.to_vec();
    buf.push(0);
    assert_same(case, |api| unsafe {
        (api.print_line)(buf.as_ptr() as *const c_char)
    });
}

pub fn assert_same_print_int_line(case: &str, v: i32) {
    assert_same(case, |api| unsafe { (api.print_int_line)(v as c_int) });
}

/// SplitMix64 — deterministic, fixed-seed PRNG for the property-style rows.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }
    pub fn seeded(s: u64) -> Rng {
        Rng(s)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Uniform inclusive range.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}
