// Shared harness for the differential tests.
//
// Both implementations are loaded as shared objects through `libloading` and
// invoked only through their exported C symbols — the Rust functions are never
// called directly, so the `#[no_mangle] extern "C"` wrappers are under test too.
//
// The library's entire observable behaviour is bytes written to C `stdout`, so
// the harness redirects file descriptor 1 to a temporary file around each call
// and returns exactly what was produced.

#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// `c_src/build/libdriver.so`, built by cmake.
pub fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = manifest
        .parent()
        .expect("crate root has a parent")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {p:?}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// `target/<profile>/libdriver.so`, i.e. the cdylib produced by this crate.
///
/// Derived from the running test executable (`target/<profile>/deps/<test>`) so
/// that it is always the artifact from the same `cargo test` invocation and the
/// same feature selection.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<exe>")
        .to_path_buf();
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.is_file(),
        "Rust cdylib not found at {p:?} (test exe {exe:?})"
    );
    p
}

// ---------------------------------------------------------------------------
// The exported ABI, as an external caller sees it
// ---------------------------------------------------------------------------

pub type PrintLineFn = unsafe extern "C" fn(*const c_char);
pub type VoidFn = unsafe extern "C" fn();
pub type DriverFn = unsafe extern "C" fn(c_int);

/// Function pointers pulled out of one loaded `.so`.
#[derive(Clone, Copy)]
pub struct Api {
    pub print_line: PrintLineFn,
    pub bad: VoidFn,
    pub good: VoidFn,
    pub driver: DriverFn,
}

/// Resolve all four symbols, asserting each one is present.
///
/// The returned pointers alias `lib`; callers must keep it alive (the global
/// pair below leaks its libraries for exactly this reason).
pub fn resolve(lib: &Library, which: &str) -> Api {
    unsafe fn sym<T: Copy>(lib: &Library, name: &[u8], which: &str) -> T {
        let s: Symbol<T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
            panic!(
                "symbol `{}` missing from the {} shared object: {e}",
                String::from_utf8_lossy(name),
                which
            )
        });
        *s
    }
    unsafe {
        Api {
            print_line: sym(lib, b"printLine\0", which),
            bad: sym(lib, b"bad\0", which),
            good: sym(lib, b"good\0", which),
            driver: sym(lib, b"driver\0", which),
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        // Both are opened RTLD_LOCAL (libloading's default), so each object
        // resolves its own `printLine` — in the C build `bad`/`good` reach it
        // through the PLT, and we must not let the two objects interpose on one
        // another. `Box::leak` keeps them mapped for the whole process.
        let c = Box::leak(Box::new(
            unsafe { Library::new(c_so_path()) }.expect("dlopen C libdriver.so"),
        ));
        let rust = Box::leak(Box::new(
            unsafe { Library::new(rust_so_path()) }.expect("dlopen Rust libdriver.so"),
        ));
        Pair {
            c: resolve(c, "C"),
            rust: resolve(rust, "Rust"),
        }
    })
}

pub fn c_api() -> Api {
    pair().c
}

pub fn rust_api() -> Api {
    pair().rust
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

// fd 1 is process-global; captures must not overlap even though cargo runs
// tests on several threads.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Run `f` with C `stdout` redirected to a scratch file and return every byte
/// it wrote.
///
/// `f` is run more than once: libtest's own progress lines (`test foo ... ok`)
/// are written to fd 1 from the harness's main thread while a worker thread may
/// be holding the redirect, which would otherwise splice harness text into the
/// capture. Repeating until two consecutive captures agree filters that
/// non-deterministic contamination out while leaving any *deterministic*
/// divergence between the two libraries fully visible.
pub fn capture<F: FnMut()>(mut f: F) -> Vec<u8> {
    let mut prev = capture_once(&mut f);
    for _ in 0..8 {
        let next = capture_once(&mut f);
        if next == prev {
            return next;
        }
        prev = next;
    }
    prev
}

/// A single redirect-run-read cycle.
fn capture_once(f: &mut dyn FnMut()) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_diff_{}_{}.out", std::process::id(), n));
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).expect("temp path has no NUL");

    unsafe {
        // Push out anything libc has already buffered for the real stdout so it
        // cannot leak into the capture.
        libc::fflush(std::ptr::null_mut());

        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_int,
        );
        assert!(fd >= 0, "open({path:?}) failed");
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        f();

        // Flush while fd 1 still points at the scratch file.
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);

        assert!(libc::lseek(fd, 0, libc::SEEK_SET) == 0, "lseek failed");
        let mut out = Vec::new();
        let mut chunk = vec![0u8; 1 << 16];
        loop {
            let got = libc::read(fd, chunk.as_mut_ptr().cast(), chunk.len());
            assert!(got >= 0, "read failed");
            if got == 0 {
                break;
            }
            out.extend_from_slice(&chunk[..got as usize]);
        }
        libc::close(fd);
        libc::unlink(cpath.as_ptr());
        out
    }
}

/// Capture the same closure body against the C API and the Rust API and assert
/// the two byte streams are identical.
pub fn assert_same<F>(label: &str, mut body: F)
where
    F: FnMut(Api),
{
    let c = capture(|| body(c_api()));
    let r = capture(|| body(rust_api()));
    if c != r {
        panic!(
            "output divergence [{label}]\n  C    ({} bytes): {}\n  Rust ({} bytes): {}",
            c.len(),
            render(&c),
            r.len(),
            render(&r)
        );
    }
}

fn render(b: &[u8]) -> String {
    let head: Vec<u8> = b.iter().copied().take(256).collect();
    let s = String::from_utf8_lossy(&head).escape_debug().to_string();
    if b.len() > 256 {
        format!("\"{s}\"... (+{} bytes)", b.len() - 256)
    } else {
        format!("\"{s}\"")
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible runs
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

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
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform byte in `1..=255` (never a NUL, so it cannot terminate a string).
    pub fn non_nul_byte(&mut self) -> u8 {
        (self.next_u64() % 255) as u8 + 1
    }
    pub fn printable_byte(&mut self) -> u8 {
        b' ' + (self.next_u64() % 95) as u8
    }
}

/// A NUL-terminated buffer of `len` random non-NUL bytes.
pub fn random_cstr(rng: &mut Rng, len: usize, printable: bool) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        v.push(if printable {
            rng.printable_byte()
        } else {
            rng.non_nul_byte()
        });
    }
    v.push(0);
    v
}

/// A NUL-terminated buffer of `len` copies of `b`.
pub fn filled_cstr(len: usize, b: u8) -> Vec<u8> {
    let mut v = vec![b; len];
    v.push(0);
    v
}
