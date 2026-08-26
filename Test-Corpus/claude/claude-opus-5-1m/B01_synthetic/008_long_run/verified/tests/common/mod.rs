//! Shared plumbing for the differential test-suite.
//!
//! Every test loads BOTH shared objects with `libloading` and calls them only
//! through their exported C symbols:
//!
//! * `libc_driver_O0.so` / `libc_driver_O2.so` — the ground truth, built from
//!   `c_src/src/main.c` by `build.rs` at two optimisation levels.
//! * `libdriver.so` — the Rust translation's cdylib.
//!
//! Nothing here calls a Rust function of the crate directly, so the
//! `#[no_mangle]` export wrappers are part of what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_uint};
use std::path::PathBuf;

pub const ARRAY_SIZE: usize = 256 * 1024;

pub type MainFn = unsafe extern "C" fn(c_int, *const *const c_char) -> c_int;
pub type PerformFn = unsafe extern "C" fn();
pub type SrandFn = unsafe extern "C" fn(c_uint);
pub type RandFn = unsafe extern "C" fn() -> c_int;
pub type ParseSeedFn = unsafe extern "C" fn(*const c_char, *mut c_uint) -> c_int;
pub type SizeFn = unsafe extern "C" fn() -> usize;

/// One loaded implementation, addressed purely through `dlsym`.
pub struct Impl {
    pub name: String,
    pub path: PathBuf,
    lib: Library,
}

impl Impl {
    pub fn load(name: &str, path: PathBuf) -> Impl {
        assert!(
            path.exists(),
            "shared object {} is missing (run `cargo build` first)",
            path.display()
        );
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        Impl {
            name: name.to_string(),
            path,
            lib,
        }
    }

    fn sym<T>(&self, name: &[u8]) -> Symbol<'_, T> {
        unsafe { self.lib.get::<T>(name) }.unwrap_or_else(|e| {
            panic!(
                "{}: dlsym({}) failed: {e}",
                self.name,
                String::from_utf8_lossy(name)
            )
        })
    }

    pub fn has_symbol(&self, name: &[u8]) -> bool {
        unsafe { self.lib.get::<*mut ()>(name) }.is_ok()
    }

    /// Address of the exported `int array[ARRAY_SIZE]` object.
    pub fn array_ptr(&self) -> *mut i32 {
        let sym: Symbol<'_, *mut i32> = self.sym(b"array\0");
        *sym
    }

    pub fn set_array(&self, data: &[i32]) {
        assert_eq!(data.len(), ARRAY_SIZE);
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.array_ptr(), ARRAY_SIZE) };
    }

    pub fn get_array(&self) -> Vec<i32> {
        let mut out = vec![0i32; ARRAY_SIZE];
        unsafe { std::ptr::copy_nonoverlapping(self.array_ptr(), out.as_mut_ptr(), ARRAY_SIZE) };
        out
    }

    /// `void perform_expensive_operations(void)`
    pub fn perform(&self) {
        let f: Symbol<'_, PerformFn> = self.sym(b"perform_expensive_operations\0");
        unsafe { f() };
    }

    /// `int main(int argc, char *argv[])`
    pub fn call_main(&self, argc: c_int, argv: *const *const c_char) -> c_int {
        let f: Symbol<'_, MainFn> = self.sym(b"main\0");
        unsafe { f(argc, argv) }
    }

    // --- harness hooks (Rust side only; the C side uses real glibc) ---

    pub fn harness_srand(&self, seed: u32) {
        let f: Symbol<'_, SrandFn> = self.sym(b"harness_srand\0");
        unsafe { f(seed) };
    }

    pub fn harness_rand(&self) -> i32 {
        let f: Symbol<'_, RandFn> = self.sym(b"harness_rand\0");
        unsafe { f() }
    }

    /// Returns `Ok(seed)` when the program would accept `arg`, `Err(())` when it
    /// would print `Invalid seed: '...'`.
    pub fn harness_parse_seed(&self, arg: &[u8]) -> Result<u32, ()> {
        let f: Symbol<'_, ParseSeedFn> = self.sym(b"harness_parse_seed\0");
        let c = std::ffi::CString::new(arg).expect("argument must not contain NUL");
        let mut out: c_uint = 0xDEAD_BEEF;
        let rc = unsafe { f(c.as_ptr(), &mut out) };
        if rc == 0 {
            Ok(out)
        } else {
            Err(())
        }
    }

    pub fn harness_array_size(&self) -> usize {
        let f: Symbol<'_, SizeFn> = self.sym(b"harness_array_size\0");
        unsafe { f() }
    }

    pub fn harness_iterations(&self) -> usize {
        let f: Symbol<'_, SizeFn> = self.sym(b"harness_iterations\0");
        unsafe { f() }
    }
}

/// `target/<profile>/libdriver.so`, derived from the test executable's location.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe layout");
    profile_dir.join("libdriver.so")
}

pub fn rust_impl() -> Impl {
    Impl::load("rust", rust_so_path())
}

/// The C ground truth, built by `build.rs` at `-O0` (what CMake's default
/// configuration produces) and at `-O2`.
pub fn c_impls() -> Vec<Impl> {
    let mut v = Vec::new();
    for (name, path) in [
        ("c-O0", env!("C_DRIVER_SO_O0")),
        ("c-O2", env!("C_DRIVER_SO_O2")),
    ] {
        v.push(Impl::load(name, PathBuf::from(path)));
    }
    v
}

/// Every implementation pair to compare: (c, rust) for each C optimisation level.
pub struct Pairs {
    pub c: Vec<Impl>,
    pub rust: Impl,
}

pub fn pairs() -> Pairs {
    Pairs {
        c: c_impls(),
        rust: rust_impl(),
    }
}

// ---------------------------------------------------------------------------
// Deterministic input generation (SplitMix64, fixed seeds per test).
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers.
// ---------------------------------------------------------------------------

pub fn xor_reduce(a: &[i32]) -> i32 {
    a.iter().fold(0i32, |acc, &v| acc ^ v)
}

/// Compares two full `array` snapshots and panics with the first difference.
pub fn assert_arrays_eq(context: &str, c_name: &str, c: &[i32], rust: &[i32], input: &[i32]) {
    if c == rust {
        return;
    }
    let mut diffs = 0usize;
    let mut first = None;
    for i in 0..c.len() {
        if c[i] != rust[i] {
            diffs += 1;
            if first.is_none() {
                first = Some(i);
            }
        }
    }
    let i = first.unwrap();
    panic!(
        "{context}: {c_name} vs rust differ in {diffs}/{} elements.\n\
         first difference at index {i}: input={} (0x{:08x}) {c_name}={} rust={}",
        c.len(),
        input[i],
        input[i] as u32,
        c[i],
        rust[i],
    );
}

// ---------------------------------------------------------------------------
// stdout/stderr capture for `main` calls (process-global: serialised).
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static CAPTURE_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub struct Captured {
    pub status: c_int,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Captured {
    pub fn describe(&self) -> String {
        format!(
            "status={} stdout={:?} stderr={:?}",
            self.status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Runs `f` with fds 1 and 2 redirected to temp files and returns what it wrote
/// plus its return value. C `FILE*` buffers are flushed before restoring.
pub fn capture<F: FnOnce() -> c_int>(f: F) -> Captured {
    use std::io::Read;
    use std::os::unix::io::AsRawFd;

    let _guard = CAPTURE_LOCK.lock().unwrap();
    let seq = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir();
    let out_path = dir.join(format!("driver_diff_{}_{}.out", std::process::id(), seq));
    let err_path = dir.join(format!("driver_diff_{}_{}.err", std::process::id(), seq));

    let status;
    unsafe {
        let saved_out = libc::dup(1);
        let saved_err = libc::dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");

        {
            let fout = std::fs::File::create(&out_path).expect("temp stdout");
            let ferr = std::fs::File::create(&err_path).expect("temp stderr");
            assert!(libc::dup2(fout.as_raw_fd(), 1) >= 0, "dup2(stdout)");
            assert!(libc::dup2(ferr.as_raw_fd(), 2) >= 0, "dup2(stderr)");
        }

        status = f();

        // C's printf() to a redirected stdout is block-buffered.
        libc::fflush(std::ptr::null_mut());

        assert!(libc::dup2(saved_out, 1) >= 0, "restore stdout");
        assert!(libc::dup2(saved_err, 2) >= 0, "restore stderr");
        libc::close(saved_out);
        libc::close(saved_err);
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    std::fs::File::open(&out_path)
        .expect("reopen stdout capture")
        .read_to_end(&mut stdout)
        .expect("read stdout capture");
    std::fs::File::open(&err_path)
        .expect("reopen stderr capture")
        .read_to_end(&mut stderr)
        .expect("read stderr capture");
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);

    Captured {
        status,
        stdout,
        stderr,
    }
}

/// A NULL-terminated `argv` built from raw byte strings (`None` = NULL pointer).
pub struct Argv {
    _storage: Vec<Option<std::ffi::CString>>,
    ptrs: Vec<*const c_char>,
}

impl Argv {
    pub fn new(args: &[Option<&[u8]>]) -> Argv {
        let storage: Vec<Option<std::ffi::CString>> = args
            .iter()
            .map(|a| a.map(|b| std::ffi::CString::new(b).expect("no interior NUL")))
            .collect();
        let mut ptrs: Vec<*const c_char> = storage
            .iter()
            .map(|s| match s {
                Some(c) => c.as_ptr(),
                None => std::ptr::null(),
            })
            .collect();
        ptrs.push(std::ptr::null()); // argv[argc] == NULL
        Argv {
            _storage: storage,
            ptrs,
        }
    }

    pub fn from_strs(args: &[&[u8]]) -> Argv {
        let v: Vec<Option<&[u8]>> = args.iter().map(|a| Some(*a)).collect();
        Argv::new(&v)
    }

    pub fn ptr(&self) -> *const *const c_char {
        self.ptrs.as_ptr()
    }
}

/// Calls `main(argc, argv)` on one implementation with output captured.
pub fn run_main(imp: &Impl, argc: c_int, argv: &Argv) -> Captured {
    capture(|| imp.call_main(argc, argv.ptr()))
}

/// Differential `main` call: asserts C and Rust agree on status, stdout, stderr.
pub fn assert_main_matches(context: &str, pairs: &Pairs, argc: c_int, args: &[Option<&[u8]>]) {
    let argv = Argv::new(args);
    let rust = run_main(&pairs.rust, argc, &argv);
    for c in &pairs.c {
        let cout = run_main(c, argc, &argv);
        assert_eq!(
            (cout.status, &cout.stdout, &cout.stderr),
            (rust.status, &rust.stdout, &rust.stderr),
            "{context}: {} gave [{}] but rust gave [{}] (argc={argc}, argv={:?})",
            c.name,
            cout.describe(),
            rust.describe(),
            args.iter()
                .map(|a| a.map(String::from_utf8_lossy))
                .collect::<Vec<_>>()
        );
    }
}

// ---------------------------------------------------------------------------
// The exported `array` object is process-global per `.so`, so every test that
// touches it must be serialised (cargo runs a file's tests on many threads).
// ---------------------------------------------------------------------------

static ARRAY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn array_guard() -> std::sync::MutexGuard<'static, ()> {
    ARRAY_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Core Phase-B primitive: load `input` into both `array` objects, call
/// `perform_expensive_operations()` `calls` times on each, and require the full
/// 1 MB result (and its XOR reduction) to match after **every** call.
pub fn diff_perform(context: &str, pairs: &Pairs, input: &[i32], calls: usize) {
    assert_eq!(input.len(), ARRAY_SIZE);
    assert!(calls >= 1);
    let _g = array_guard();

    pairs.rust.set_array(input);
    let mut rust_snaps: Vec<Vec<i32>> = Vec::with_capacity(calls);
    for _ in 0..calls {
        pairs.rust.perform();
        rust_snaps.push(pairs.rust.get_array());
    }

    for c in &pairs.c {
        c.set_array(input);
        for k in 0..calls {
            c.perform();
            let got = c.get_array();
            let prev: &[i32] = if k == 0 { input } else { &rust_snaps[k - 1] };
            assert_arrays_eq(
                &format!("{context} [after {} call(s)]", k + 1),
                &c.name,
                &got,
                &rust_snaps[k],
                prev,
            );
            assert_eq!(
                xor_reduce(&got),
                xor_reduce(&rust_snaps[k]),
                "{context} [after {} call(s)]: XOR reduction differs ({} vs rust)",
                k + 1,
                c.name
            );
        }
    }
}

/// glibc's `srand`/`rand` state is process-global, so tests that use the real
/// libc RNG as ground truth must not run concurrently.
static GLIBC_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn glibc_guard() -> std::sync::MutexGuard<'static, ()> {
    GLIBC_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
