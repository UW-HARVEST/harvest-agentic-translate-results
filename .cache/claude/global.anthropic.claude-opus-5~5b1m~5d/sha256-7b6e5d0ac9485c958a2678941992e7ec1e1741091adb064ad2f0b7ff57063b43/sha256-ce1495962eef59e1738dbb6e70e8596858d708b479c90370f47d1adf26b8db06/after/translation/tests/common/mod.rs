// Shared differential-testing harness.
//
// BOTH implementations are loaded as shared objects through `libloading` and
// invoked *only* through their exported C symbols — the Rust crate is never
// called directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers are part
// of what is under test.
#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// exported ABI of libdriver.so
// ---------------------------------------------------------------------------

pub type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
pub type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
pub type DriverFn = unsafe extern "C" fn(*const c_char);

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub fma_array: FmaArrayFn,
    pub call_fma: CallFmaFn,
    pub driver: DriverFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // The cdylib MUST come from the same profile as this test binary.
    //
    // Deliberately NO fallback to another profile: `cargo test` alone does not
    // build a `cdylib`-only lib target, so a fallback would silently test the
    // release `.so` while the test binary (and its `cfg!(debug_assertions)`)
    // says "dev".  The dev and release artifacts genuinely behave differently
    // for raw-pointer UB, so testing the wrong one would invalidate the result.
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test bin>  ->  .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|d| d.parent())
        .expect("unexpected test binary location");
    let so = profile_dir.join("libdriver.so");
    assert!(
        so.exists(),
        "Rust cdylib not found at {so:?}.\n\
         `cargo test` does not build a cdylib-only lib target, so build it \
         explicitly for THIS profile first:\n  \
         cd translation && cargo build{}    # then re-run cargo test\n\
         (or point DRIVER_RUST_SO at the .so you want tested)",
        if profile_dir.ends_with("release") {
            " --release"
        } else {
            ""
        }
    );
    so
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    // SAFETY: loading a plain C shared library with no initialisers of note.
    let lib = unsafe {
        libloading::Library::new(&path).unwrap_or_else(|e| panic!("dlopen({path:?}): {e}"))
    };
    let fma_array = unsafe {
        *lib.get::<FmaArrayFn>(b"fma_array\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol fma_array: {e}"))
    };
    let call_fma = unsafe {
        *lib.get::<CallFmaFn>(b"call_fma\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol call_fma: {e}"))
    };
    let driver = unsafe {
        *lib.get::<DriverFn>(b"driver\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol driver: {e}"))
    };
    // Keep the library mapped for the whole process lifetime so the raw fn
    // pointers stay valid.
    std::mem::forget(lib);
    Impl {
        name,
        path,
        fma_array,
        call_fma,
        driver,
    }
}

pub fn c_impl() -> &'static Impl {
    static C: OnceLock<Impl> = OnceLock::new();
    C.get_or_init(|| load("C", c_so_path()))
}

pub fn rust_impl() -> &'static Impl {
    static R: OnceLock<Impl> = OnceLock::new();
    R.get_or_init(|| load("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn i32_full(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
    /// A value biased towards the interesting corners of `i32`.
    pub fn i32_corner(&mut self) -> i32 {
        const CORNERS: [i32; 12] = [
            i32::MIN,
            i32::MIN + 1,
            -2,
            -1,
            0,
            1,
            2,
            3,
            i32::MAX - 1,
            i32::MAX,
            0x4000_0000,
            -0x4000_0000,
        ];
        match self.below(4) {
            0 => self.i32_full(),
            1 => self.range_i32(-100, 100),
            _ => self.pick(&CORNERS),
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture (the C `driver()` writes with printf)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

/// `RLIMIT_CORE` on Linux.
const RLIMIT_CORE: c_int = 4;

/// Called in every forked child: the crash-path tests deliberately provoke
/// `SIGSEGV`, and writing a core dump for each one is both slow (tens of
/// seconds) and litters the working directory.
fn suppress_core_dumps() {
    let zero = RLimit { cur: 0, max: 0 };
    unsafe { setrlimit(RLIMIT_CORE, &zero) };
}

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

fn temp_path(tag: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("driver-{tag}-{}-{n}.out", std::process::id()))
}

/// Run `f` in a **forked child** whose file descriptor 1 is redirected to a
/// temporary file, and return every byte the child wrote there.
///
/// Forking (rather than redirecting fd 1 in-process) is essential: `libtest`
/// writes its own progress lines to fd 1 from the main thread, so an
/// in-process redirect would splice `"test foo ... ok"` into the captured
/// bytes and produce bogus divergences. The child has a private fd 1, so
/// nothing else in the process can contaminate the capture.
///
/// `fflush(NULL)` is issued in the child because the C code never flushes
/// stdout itself, and `_exit` deliberately does not flush.
pub fn capture_stdout_outcome<F: FnOnce()>(f: F) -> (Outcome, Vec<u8>) {
    use std::os::fd::AsRawFd;

    preload();
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = temp_path("capture");
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    let outcome = unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            suppress_core_dumps();
            if dup2(fd, 1) < 0 {
                _exit(102);
            }
            let code = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
                .map(|_| 0)
                .unwrap_or(101);
            fflush(std::ptr::null_mut());
            _exit(code);
        }
        wait_for(pid)
    };
    drop(file);
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (outcome, bytes)
}

/// Like [`capture_stdout_outcome`] but requires a clean exit.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let (outcome, bytes) = capture_stdout_outcome(f);
    assert_eq!(
        outcome,
        Outcome::Exited(0),
        "captured callee terminated abnormally (partial output: {:?})",
        String::from_utf8_lossy(&bytes)
    );
    bytes
}

// ---------------------------------------------------------------------------
// differential helpers
// ---------------------------------------------------------------------------

pub fn cstring(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("input must not contain interior NUL")
}

fn show(bytes: &[u8]) -> String {
    let s: String = bytes
        .iter()
        .map(|&b| match b {
            b'\n' => "\\n".to_string(),
            b'\t' => "\\t".to_string(),
            b'\r' => "\\r".to_string(),
            0x0b => "\\v".to_string(),
            0x0c => "\\f".to_string(),
            0x20..=0x7e => (b as char).to_string(),
            _ => format!("\\x{b:02x}"),
        })
        .collect();
    s
}

/// Run `driver()` once per input inside a SINGLE forked child, returning one
/// output line per input.
///
/// Batching keeps the cost at one `fork()` per (test, library) instead of one
/// per call, while still attributing every output line to its input: `driver`
/// always prints exactly one `"%d\n"` line per call.
pub fn driver_lines(imp: &Impl, inputs: &[CString]) -> Vec<Vec<u8>> {
    let ptrs: Vec<*const c_char> = inputs.iter().map(|c| c.as_ptr()).collect();
    let f = imp.driver;
    let bytes = capture_stdout(move || {
        for p in &ptrs {
            unsafe { f(*p) };
        }
    });
    let lines: Vec<Vec<u8>> = bytes
        .split_inclusive(|&b| b == b'\n')
        .map(|s| s.to_vec())
        .collect();
    assert_eq!(
        lines.len(),
        inputs.len(),
        "{}: driver() must print exactly one line per input (got {:?})",
        imp.name,
        String::from_utf8_lossy(&bytes)
    );
    lines
}

/// Differential `driver()` over a batch of inputs; panics on the first
/// divergence and returns the (identical) per-input outputs.
pub fn assert_driver_batch(inputs: &[CString]) -> Vec<Vec<u8>> {
    if inputs.is_empty() {
        return Vec::new();
    }
    let c = driver_lines(c_impl(), inputs);
    let r = driver_lines(rust_impl(), inputs);
    for (i, (cl, rl)) in c.iter().zip(r.iter()).enumerate() {
        if cl != rl {
            panic!(
                "driver() output diverged on input #{i}\n  input : \"{}\"\n  C     : \"{}\"\n  Rust  : \"{}\"",
                show(inputs[i].as_bytes()),
                show(cl),
                show(rl)
            );
        }
    }
    for (i, cl) in c.iter().enumerate() {
        assert!(
            cl.ends_with(b"\n"),
            "unexpected C output shape for input #{i} (\"{}\"): \"{}\"",
            show(inputs[i].as_bytes()),
            show(cl)
        );
    }
    c
}

/// Convenience: batch of raw byte inputs.
pub fn assert_driver_batch_bytes<I, B>(inputs: I) -> Vec<Vec<u8>>
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    let cs: Vec<CString> = inputs.into_iter().map(|b| cstring(b.as_ref())).collect();
    assert_driver_batch(&cs)
}

/// Differential `driver()` on a single input.
pub fn assert_driver_eq(input: &[u8]) {
    assert_driver_batch(&[cstring(input)]);
}

/// Differential `driver()` plus an assertion on what the C library prints,
/// so the test pins down the *actual* C behaviour and not just "they agree".
pub fn assert_driver_cases(cases: &[(&str, &str)]) {
    let cs: Vec<CString> = cases.iter().map(|(i, _)| cstring(i.as_bytes())).collect();
    let out = assert_driver_batch(&cs);
    for (i, (input, expect)) in cases.iter().enumerate() {
        assert_eq!(
            String::from_utf8_lossy(&out[i]),
            *expect,
            "unexpected C behaviour for input {input:?}"
        );
    }
}

/// Call `call_fma()` in both libraries and require identical return values.
pub fn assert_call_fma_eq(data: &[i32], len: c_int) {
    let ptr = data.as_ptr();
    let c_ret = unsafe { (c_impl().call_fma)(ptr, len) };
    let r_ret = unsafe { (rust_impl().call_fma)(ptr, len) };
    assert_eq!(
        c_ret, r_ret,
        "call_fma(len={len}) diverged: C={c_ret} Rust={r_ret}, data(first 16)={:?}",
        &data[..data.len().min(16)]
    );
}

/// How the four `fma_array` pointers are wired onto backing storage.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Alias {
    /// four independent buffers
    Distinct,
    OutEqMul1,
    OutEqMul2,
    OutEqAdd,
    Mul1EqMul2,
    Mul1EqAdd,
    AllInputsSame,
    EverythingSame,
    /// `out = buf[0..]`, `mul1 = buf[1..]`
    PartialOverlapForward,
    /// `out = buf[1..]`, `mul1 = buf[0..]`
    PartialOverlapBackward,
}

/// Lay out `len` elements according to `alias`, run `fma_array` on both
/// libraries, and require every byte of every buffer to match afterwards.
///
/// `seed_*` supply the initial contents of the logical arrays.
pub fn assert_fma_array_eq(
    alias: Alias,
    len: c_int,
    seed_out: &[i32],
    seed_mul1: &[i32],
    seed_mul2: &[i32],
    seed_add: &[i32],
) {
    let n = seed_out.len();
    assert!(seed_mul1.len() == n && seed_mul2.len() == n && seed_add.len() == n);

    // For each implementation build a fresh set of buffers, wire up the four
    // pointers per the aliasing mode, call, and snapshot all storage.
    let run = |imp: &Impl| -> Vec<Vec<i32>> {
        // `store` holds the independent backing buffers; the closure below maps
        // the aliasing mode onto offsets within them.
        let mut store: Vec<Vec<i32>> = match alias {
            Alias::Distinct => vec![
                seed_out.to_vec(),
                seed_mul1.to_vec(),
                seed_mul2.to_vec(),
                seed_add.to_vec(),
            ],
            Alias::OutEqMul1 => vec![seed_mul1.to_vec(), seed_mul2.to_vec(), seed_add.to_vec()],
            Alias::OutEqMul2 => vec![seed_mul2.to_vec(), seed_mul1.to_vec(), seed_add.to_vec()],
            Alias::OutEqAdd => vec![seed_add.to_vec(), seed_mul1.to_vec(), seed_mul2.to_vec()],
            Alias::Mul1EqMul2 => vec![seed_out.to_vec(), seed_mul1.to_vec(), seed_add.to_vec()],
            Alias::Mul1EqAdd => vec![seed_out.to_vec(), seed_mul1.to_vec(), seed_mul2.to_vec()],
            Alias::AllInputsSame => vec![seed_out.to_vec(), seed_mul1.to_vec()],
            Alias::EverythingSame => vec![seed_mul1.to_vec()],
            Alias::PartialOverlapForward | Alias::PartialOverlapBackward => {
                // one buffer of n+1 elements shared by out and mul1
                let mut shared = Vec::with_capacity(n + 1);
                shared.extend_from_slice(seed_mul1);
                shared.push(seed_out.first().copied().unwrap_or(0));
                vec![shared, seed_mul2.to_vec(), seed_add.to_vec()]
            }
        };

        let base: Vec<*mut c_int> = store.iter_mut().map(|v| v.as_mut_ptr()).collect();
        let (out, mul1, mul2, add): (*mut c_int, *const c_int, *const c_int, *const c_int) =
            match alias {
                Alias::Distinct => (base[0], base[1], base[2], base[3]),
                Alias::OutEqMul1 => (base[0], base[0], base[1], base[2]),
                Alias::OutEqMul2 => (base[0], base[1], base[0], base[2]),
                Alias::OutEqAdd => (base[0], base[1], base[2], base[0]),
                Alias::Mul1EqMul2 => (base[0], base[1], base[1], base[2]),
                Alias::Mul1EqAdd => (base[0], base[1], base[2], base[1]),
                Alias::AllInputsSame => (base[0], base[1], base[1], base[1]),
                Alias::EverythingSame => (base[0], base[0], base[0], base[0]),
                Alias::PartialOverlapForward => (base[0], unsafe { base[0].add(1) }, base[1], base[2]),
                Alias::PartialOverlapBackward => {
                    (unsafe { base[0].add(1) }, base[0], base[1], base[2])
                }
            };

        unsafe { (imp.fma_array)(out, mul1, mul2, add, len) };
        store
    };

    let c_store = run(c_impl());
    let r_store = run(rust_impl());
    if c_store != r_store {
        panic!(
            "fma_array(alias={alias:?}, len={len}) diverged\n  C   : {c_store:?}\n  Rust: {r_store:?}\n  \
             seeds: out={seed_out:?} mul1={seed_mul1:?} mul2={seed_mul2:?} add={seed_add:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// stack budget
// ---------------------------------------------------------------------------

/// The C `call_fma` puts THREE `int[len]` VLAs on the caller's stack, i.e. it
/// needs roughly `12 * len` bytes of stack. `libtest` runs each test on a
/// thread with the default (2 MiB) stack, which would cap `len` at ~170 000, so
/// large-`len` rows are run on a thread with an explicitly large stack instead.
pub const BIG_STACK: usize = 256 * 1024 * 1024;

/// Run `f` on a freshly spawned thread with a `BIG_STACK`-sized stack.
pub fn on_big_stack<T, F>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(BIG_STACK)
        .spawn(f)
        .expect("spawn big-stack thread")
        .join()
        .expect("big-stack thread panicked")
}

/// Run `f` on a thread whose stack is exactly `bytes` large.
pub fn on_stack_of<T, F>(bytes: usize, f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(bytes)
        .spawn(f)
        .expect("spawn sized-stack thread")
        .join()
        .expect("sized-stack thread panicked")
}

// ---------------------------------------------------------------------------
// fork-isolated execution, for the C-UB (crashing) rows of ERRORS.md
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(c_int),
    Signaled(c_int),
}

/// Make sure both libraries are dlopen'ed and their `OnceLock`s initialised
/// *before* any `fork()`, so the child never has to allocate/dlopen.
pub fn preload() {
    let _ = c_impl();
    let _ = rust_impl();
}

/// Run `f` in a forked child and report how the child terminated.
///
/// The child performs the FFI call, then `_exit`s with the code `f` returned
/// (or `101` if `f` panicked), so forking from a (possibly multi-threaded) test
/// harness is safe here and a panic can never leak into the child's copy of the
/// test harness.
pub fn run_isolated<F: FnOnce()>(f: F) -> Outcome {
    run_isolated_code(|| {
        f();
        0
    })
}

pub fn run_isolated_code<F: FnOnce() -> c_int>(f: F) -> Outcome {
    preload();
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            suppress_core_dumps();
            let code = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or(101);
            fflush(std::ptr::null_mut());
            _exit(code);
        }
        wait_for(pid)
    }
}

/// `waitpid` + decode, per `<bits/waitstatus.h>`.
unsafe fn wait_for(pid: c_int) -> Outcome {
    let mut status: c_int = 0;
    let w = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    if status & 0x7f == 0x7f {
        Outcome::Signaled((status >> 8) & 0xff) // stopped -- not expected
    } else if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signaled(status & 0x7f)
    }
}
