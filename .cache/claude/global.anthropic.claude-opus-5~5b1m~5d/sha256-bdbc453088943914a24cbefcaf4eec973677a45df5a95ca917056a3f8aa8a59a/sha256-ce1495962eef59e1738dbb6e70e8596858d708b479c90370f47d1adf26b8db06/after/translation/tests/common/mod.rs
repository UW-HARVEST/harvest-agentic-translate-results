//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked only through their exported C symbols — the Rust functions are never
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture (declared directly so no extra dep).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library built from `c_src/`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib`.
///
/// Defaults to the debug artifact, which `cargo test` always rebuilds, so the
/// tests can never silently validate a stale `.so`. Override with
/// `DRIVER_RUST_SO` to point at `target/release/libdriver.so`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("target/debug/libdriver.so");
    assert!(
        p.exists(),
        "Rust shared library not found at {} — run `cargo build`",
        p.display()
    );
    p
}

// ---------------------------------------------------------------------------
// The loaded API surface
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(*const c_int, c_int);
type FmaFn = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);

pub struct Impl {
    lib: Library,
    pub name: &'static str,
}

impl Impl {
    fn open(path: PathBuf, name: &'static str) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        Impl { lib, name }
    }

    pub fn driver_sym(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{}: missing symbol `driver`: {e}", self.name))
    }

    pub fn fma_sym(&self) -> Symbol<'_, FmaFn> {
        unsafe { self.lib.get(b"fma_array\0") }
            .unwrap_or_else(|e| panic!("{}: missing symbol `fma_array`: {e}", self.name))
    }

    /// `inner` is `static` in the C — neither `.so` may export it.
    pub fn has_inner(&self) -> bool {
        unsafe { self.lib.get::<*const ()>(b"inner\0") }.is_ok()
    }
}

/// The two implementations under comparison, opened once per test binary.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Impl::open(c_so_path(), "C"),
        rust: Impl::open(rust_so_path(), "Rust"),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f`, capturing everything written to file descriptor 1 (which is where
/// the libc `printf` inside both `.so`s ends up).
///
/// fd redirection is process-global, so this serializes across test threads.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::fd::AsRawFd;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Hold the Rust stdout lock for the whole redirect so that neither the
    // libtest progress printer nor another test thread can write into our
    // capture file, and flush whatever it has buffered out to the real fd 1
    // before we swap it.
    let mut rust_stdout = std::io::stdout().lock();
    rust_stdout.flush().ok();
    unsafe { fflush(std::ptr::null_mut()) };

    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}.txt",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = catch_unwind(AssertUnwindSafe(f));

    unsafe { fflush(std::ptr::null_mut()) };
    unsafe {
        dup2(saved, 1);
        close(saved);
    }

    let mut buf = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek capture file");
    file.read_to_end(&mut buf).expect("read capture file");
    drop(file);
    std::fs::remove_file(&path).ok();
    drop(rust_stdout);

    match result {
        Ok(()) => buf,
        Err(p) => std::panic::resume_unwind(p),
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform over the entire `i32` range, including `INT_MIN` / `INT_MAX`.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

/// Value sets the C's arithmetic distinguishes (see CONFIGS.md).
pub const SMALL_VALUES: [i32; 5] = [-2, -1, 0, 1, 2];
pub const BOUNDARY_VALUES: [i32; 7] = [
    i32::MIN,
    i32::MIN + 1,
    -1,
    0,
    1,
    i32::MAX - 1,
    i32::MAX,
];
/// Lengths that sweep typical unroll / vector widths and their off-by-ones.
pub const WIDTH_SWEEP: [i32; 18] = [
    0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65,
];

pub enum Values {
    Full,
    Small,
    Boundary,
    Zeros,
    Ones,
}

impl Values {
    pub fn sample(&self, rng: &mut Rng) -> i32 {
        match self {
            Values::Full => rng.next_i32(),
            Values::Small => rng.pick(&SMALL_VALUES),
            Values::Boundary => rng.pick(&BOUNDARY_VALUES),
            Values::Zeros => 0,
            Values::Ones => 1,
        }
    }
    pub fn fill(&self, rng: &mut Rng, n: usize) -> Vec<i32> {
        (0..n).map(|_| self.sample(rng)).collect()
    }
}

// ---------------------------------------------------------------------------
// fma_array differential driver, with explicit aliasing control
// ---------------------------------------------------------------------------

/// An aliasing pattern: byte offsets (in elements) of `out`, `mul1`, `mul2`,
/// `add` inside one shared arena, plus the arena size in elements.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub arena: usize,
    pub out: usize,
    pub mul1: usize,
    pub mul2: usize,
    pub add: usize,
}

impl Layout {
    /// Four fully distinct, non-overlapping buffers.
    pub fn distinct(len: usize) -> Layout {
        Layout { arena: 4 * len + 4, out: 0, mul1: len, mul2: 2 * len, add: 3 * len }
    }
    pub fn out_eq_mul1(len: usize) -> Layout {
        Layout { arena: 3 * len + 4, out: 0, mul1: 0, mul2: len, add: 2 * len }
    }
    pub fn out_eq_mul2(len: usize) -> Layout {
        Layout { arena: 3 * len + 4, out: 0, mul1: len, mul2: 0, add: 2 * len }
    }
    pub fn out_eq_add(len: usize) -> Layout {
        Layout { arena: 3 * len + 4, out: 0, mul1: len, mul2: 2 * len, add: 0 }
    }
    pub fn mul1_eq_mul2(len: usize) -> Layout {
        Layout { arena: 3 * len + 4, out: 0, mul1: len, mul2: len, add: 2 * len }
    }
    /// Exactly what the C's `inner` does: `fma_array(out, out, out, out, len)`.
    pub fn all_same(len: usize) -> Layout {
        Layout { arena: len + 4, out: 0, mul1: 0, mul2: 0, add: 0 }
    }
    /// `mul1 = out + 1`: forward loop reads an element it has not written yet.
    pub fn mul1_shifted(len: usize) -> Layout {
        Layout { arena: 3 * len + 8, out: 0, mul1: 1, mul2: len + 2, add: 2 * len + 4 }
    }
    /// `out = buf + 1`, `mul1 = buf`: forward loop reads an element it just wrote.
    pub fn out_shifted(len: usize) -> Layout {
        Layout { arena: 3 * len + 8, out: 1, mul1: 0, mul2: len + 2, add: 2 * len + 4 }
    }
}

/// Call `fma_array` in both `.so`s over identical copies of `init` with the
/// given layout, then compare the **entire arena** byte-for-byte (so writes
/// outside `out[0..len]` are caught too).
pub fn diff_fma(label: &str, init: &[i32], lay: Layout, len: c_int) {
    let p = pair();

    let run = |imp: &Impl| -> Vec<i32> {
        let mut buf = init.to_vec();
        let base = buf.as_mut_ptr();
        let f = imp.fma_sym();
        unsafe {
            f(
                base.add(lay.out),
                base.add(lay.mul1) as *const c_int,
                base.add(lay.mul2) as *const c_int,
                base.add(lay.add) as *const c_int,
                len,
            );
        }
        buf
    };

    let got_c = run(&p.c);
    let got_rust = run(&p.rust);

    assert_eq!(
        as_bytes(&got_c),
        as_bytes(&got_rust),
        "fma_array mismatch [{label}]\n  layout = {lay:?}\n  len    = {len}\n  input  = {:?}\n  C    = {:?}\n  Rust = {:?}",
        Trunc(init),
        Trunc(&got_c),
        Trunc(&got_rust),
    );
}

/// Call `driver` in both `.so`s and compare captured stdout byte-for-byte.
/// Also asserts neither implementation modified the caller's input buffer.
pub fn diff_driver(label: &str, data: &[i32], len: c_int) -> Vec<u8> {
    diff_driver_offset(label, data, 0, len)
}

/// As `diff_driver`, but hands the library a pointer `offset` elements into
/// `data` (CONFIGS row C30).
pub fn diff_driver_offset(label: &str, data: &[i32], offset: usize, len: c_int) -> Vec<u8> {
    let p = pair();

    let run = |imp: &Impl| -> (Vec<u8>, Vec<i32>) {
        let buf = data.to_vec();
        let ptr = unsafe { buf.as_ptr().add(offset) };
        let f = imp.driver_sym();
        let out = capture_stdout(|| unsafe { f(ptr, len) });
        (out, buf)
    };

    let (out_c, buf_c) = run(&p.c);
    let (out_rust, buf_rust) = run(&p.rust);

    assert_eq!(
        as_bytes(&buf_c),
        as_bytes(data),
        "driver [{label}]: C modified the caller's input buffer"
    );
    assert_eq!(
        as_bytes(&buf_rust),
        as_bytes(data),
        "driver [{label}]: Rust modified the caller's input buffer"
    );
    assert_eq!(
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_rust),
        "driver stdout mismatch [{label}]\n  len = {len}, offset = {offset}\n  input = {:?}",
        Trunc(data),
    );
    assert_eq!(out_c, out_rust, "driver stdout byte mismatch [{label}]");
    out_c
}

pub fn as_bytes(v: &[i32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

/// Debug helper that keeps failure messages readable for large inputs.
pub struct Trunc<'a>(pub &'a [i32]);

impl std::fmt::Debug for Trunc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() <= 24 {
            write!(f, "{:?}", self.0)
        } else {
            write!(f, "{:?}.. ({} elems)", &self.0[..24], self.0.len())
        }
    }
}
