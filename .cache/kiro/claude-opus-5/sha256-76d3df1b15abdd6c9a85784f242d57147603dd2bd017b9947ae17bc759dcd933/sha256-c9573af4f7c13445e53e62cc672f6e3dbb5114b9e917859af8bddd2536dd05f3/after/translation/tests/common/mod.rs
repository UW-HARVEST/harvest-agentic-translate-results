//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `extern "C"` symbols — never by linking
//! the Rust crate directly — so the `#[no_mangle]` wrappers are part of what is
//! under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::Library;

/// Fixed seed so every randomised row is reproducible.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// libc pieces the harness itself needs
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, which is what makes the
    /// C `.so`'s and the Rust `.so`'s `printf` output land in the capture file
    /// before it is read back.
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut core::ffi::c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const core::ffi::c_void, n: usize) -> isize;
    fn setrlimit(resource: c_int, rlim: *const Rlimit) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// `struct rlimit` — only used to switch core dumps off in forked children that
/// are expected to segfault, so probing the C code's UB paths does not litter
/// the disk with core files.
#[repr(C)]
struct Rlimit {
    cur: u64,
    max: u64,
}

/// `RLIMIT_CORE` on Linux.
const RLIMIT_CORE: c_int = 4;

// ---------------------------------------------------------------------------
// The exported ABI of both libraries
// ---------------------------------------------------------------------------

pub type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
pub type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
pub type DriverFn = unsafe extern "C" fn(*const c_char);

/// Raw function pointers pulled out of one `.so`.
#[derive(Clone, Copy)]
pub struct Api {
    pub fma_array: FmaArrayFn,
    pub call_fma: CallFmaFn,
    pub driver: DriverFn,
}

impl Api {
    fn load(path: &Path) -> Api {
        // Leaked on purpose: the function pointers must outlive the `Library`
        // borrow for the whole test run.
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
        }));
        unsafe {
            Api {
                fma_array: *lib
                    .get::<FmaArrayFn>(b"fma_array\0")
                    .unwrap_or_else(|e| panic!("dlsym fma_array in {}: {e}", path.display())),
                call_fma: *lib
                    .get::<CallFmaFn>(b"call_fma\0")
                    .unwrap_or_else(|e| panic!("dlsym call_fma in {}: {e}", path.display())),
                driver: *lib
                    .get::<DriverFn>(b"driver\0")
                    .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display())),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, produced by the CMake build.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libdriver.so")
}

/// The `cdylib` cargo just built. Derived from the test executable's own
/// location (`target/<profile>/deps/<test>`) so it follows `--release`,
/// `CARGO_TARGET_DIR`, and custom profiles automatically.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<test>");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["target/debug/libdriver.so", "target/release/libdriver.so"] {
        let c = manifest_dir().join(p);
        if c.exists() {
            return c;
        }
    }
    candidate
}

/// The two APIs, loaded once per test binary.
pub fn apis() -> &'static (Api, Api) {
    static APIS: OnceLock<(Api, Api)> = OnceLock::new();
    APIS.get_or_init(|| {
        let c = c_so_path();
        let r = rust_so_path();
        assert!(
            c.exists(),
            "C shared object not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c.display()
        );
        assert!(
            r.exists(),
            "Rust shared object not found at {}. Build it with: cargo build",
            r.display()
        );
        (Api::load(&c), Api::load(&r))
    })
}

pub fn c_api() -> Api {
    apis().0
}

pub fn rust_api() -> Api {
    apis().1
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — no external dev-dependency needed
// ---------------------------------------------------------------------------

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform over the whole `i32` range, so multiply/add overflow is hit often.
    pub fn i32_full(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Inclusive on both ends.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range(0, xs.len() - 1)]
    }
}

/// Values chosen to make `mul1*mul2 + add` overflow in as many ways as possible.
pub const EXTREMES: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    i32::MAX / 2,
    i32::MIN / 2,
    65536,
    -65536,
    46341, // just above sqrt(INT_MAX): 46341^2 overflows
    -46341,
];

pub fn random_vec(rng: &mut Rng, len: usize) -> Vec<i32> {
    (0..len).map(|_| rng.i32_full()).collect()
}

pub fn extreme_vec(rng: &mut Rng, len: usize) -> Vec<i32> {
    (0..len).map(|_| *rng.pick(EXTREMES)).collect()
}

// ---------------------------------------------------------------------------
// stdout capture (for `driver`, which prints instead of returning)
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so captures must not overlap. `cargo test` runs test
/// functions on multiple threads by default, hence the lock.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn scratch_file() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open scratch file");
    // Unlink immediately; the open handle keeps it alive and nothing is left
    // behind even if the test panics.
    let _ = std::fs::remove_file(&path);
    f
}

/// Runs `f` with fd 1 redirected to a scratch file and returns the exact bytes
/// it wrote. `fflush(NULL)` before and after means partially buffered output
/// from earlier work never bleeds into the capture and nothing stays buffered
/// when the capture is read.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::os::fd::AsRawFd;

    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut scratch = scratch_file();

    unsafe { fflush(std::ptr::null_mut()) };
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(scratch.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(guard);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }

    scratch.seek(SeekFrom::Start(0)).expect("seek scratch");
    let mut buf = Vec::new();
    scratch.read_to_end(&mut buf).expect("read scratch");
    buf
}

/// Calls `driver(input)` in both libraries and asserts the printed bytes match.
/// Returns the (shared) output.
pub fn assert_driver_matches(input: &str, label: &str) -> Vec<u8> {
    assert_driver_matches_bytes(input.as_bytes(), label)
}

/// Same, for inputs that are not valid UTF-8.
pub fn assert_driver_matches_bytes(input: &[u8], label: &str) -> Vec<u8> {
    let mut cstr = input.to_vec();
    assert!(
        !cstr.contains(&0),
        "{label}: input must not contain an interior NUL"
    );
    cstr.push(0);

    let (c, r) = *apis();
    let ptr = cstr.as_ptr() as *const c_char;

    let c_out = capture_stdout(|| unsafe { (c.driver)(ptr) });
    let r_out = capture_stdout(|| unsafe { (r.driver)(ptr) });

    assert_eq!(
        c_out,
        r_out,
        "{label}: driver stdout differs\n  input : {:?}\n  C     : {:?}\n  Rust  : {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
    c_out
}

/// Calls `call_fma` in both libraries and asserts the return values match.
pub fn assert_call_fma_matches(data: &[i32], len: c_int, label: &str) -> c_int {
    let (c, r) = *apis();
    let cv = unsafe { (c.call_fma)(data.as_ptr(), len) };
    let rv = unsafe { (r.call_fma)(data.as_ptr(), len) };
    assert_eq!(
        cv, rv,
        "{label}: call_fma differs (len={len}) C={cv} Rust={rv}\n  data head: {:?}",
        &data[..data.len().min(16)]
    );
    cv
}

/// Calls `fma_array` in both libraries into two independent, identically
/// pre-filled output buffers and asserts the buffers are byte-identical
/// afterwards — including any tail past `len`, which proves the C's exact write
/// extent is reproduced.
pub fn assert_fma_array_matches(
    out_prefill: &[i32],
    mul1: &[i32],
    mul2: &[i32],
    add: &[i32],
    len: c_int,
    label: &str,
) -> Vec<i32> {
    let (c, r) = *apis();
    let mut c_out = out_prefill.to_vec();
    let mut r_out = out_prefill.to_vec();

    unsafe {
        (c.fma_array)(
            c_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len,
        );
        (r.fma_array)(
            r_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len,
        );
    }

    if c_out != r_out {
        let i = c_out
            .iter()
            .zip(&r_out)
            .position(|(a, b)| a != b)
            .expect("differ somewhere");
        panic!(
            "{label}: fma_array differs at index {i} (len={len})\n  \
             mul1[{i}]={} mul2[{i}]={} add[{i}]={}\n  C={} Rust={}",
            mul1.get(i).copied().unwrap_or_default(),
            mul2.get(i).copied().unwrap_or_default(),
            add.get(i).copied().unwrap_or_default(),
            c_out[i],
            r_out[i],
        );
    }
    c_out
}

// ---------------------------------------------------------------------------
// Crash-isolated calls, for probing the C code's undefined-behaviour corners
// without taking the test process down with it
// ---------------------------------------------------------------------------

/// Outcome of running a snippet in a forked child.
#[derive(Debug, PartialEq, Eq)]
pub enum Isolated {
    /// The child returned this `i32`.
    Value(i32),
    /// The child died on this signal (e.g. 11 = SIGSEGV).
    Signal(i32),
    /// The child exited non-zero without producing a value.
    Failed(i32),
}

/// Runs `f` in a forked child and brings its `i32` result back over a pipe.
/// A segfault in the child is reported as `Isolated::Signal` instead of
/// aborting the test run — which is what makes it safe to poke at the C code's
/// UB paths at all.
pub fn isolated<F: FnOnce() -> i32>(f: F) -> Isolated {
    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe failed");
    let (rd, wr) = (fds[0], fds[1]);

    // Flush before forking so buffered parent output is not duplicated.
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        unsafe {
            close(rd);
            // These children are deliberately steered into the C code's
            // undefined-behaviour paths, so some of them will die on SIGSEGV.
            // Disable core dumps rather than leaving one behind per probe.
            let no_core = Rlimit { cur: 0, max: 0 };
            setrlimit(RLIMIT_CORE, &no_core);
        }
        let v = f();
        let bytes = v.to_ne_bytes();
        unsafe {
            write(wr, bytes.as_ptr() as *const core::ffi::c_void, 4);
            close(wr);
            _exit(0)
        }
    }

    unsafe { close(wr) };
    let mut buf = [0u8; 4];
    let n = unsafe { read(rd, buf.as_mut_ptr() as *mut core::ffi::c_void, 4) };
    unsafe { close(rd) };

    let mut status: c_int = 0;
    unsafe { waitpid(pid, &mut status, 0) };

    let sig = status & 0x7f;
    if sig != 0 && sig != 0x7f {
        return Isolated::Signal(sig);
    }
    let code = (status >> 8) & 0xff;
    if n == 4 {
        Isolated::Value(i32::from_ne_bytes(buf))
    } else {
        Isolated::Failed(code)
    }
}
