//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and exposes every exported symbol as a typed wrapper.
//!
//! Nothing here calls into the Rust crate directly — every call goes through
//! `dlsym` on the built `cdylib`, exactly as an external C consumer would, so
//! the `#[unsafe(no_mangle)] extern "C"` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

pub type TimeT = i64;

/// `typedef struct { int value; time_t timestamp; StatusCode status; }`
///
/// Probed on this target: size 24, align 8, offsets 0 / 8 / 16.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: TimeT,
    pub status: c_int,
}

pub const RESULT_SIZE: usize = 24;
pub const HISTORY_CAPACITY: usize = 10;

pub const OP_ADD: c_int = 1;
pub const OP_MULTIPLY: c_int = 2;
pub const OP_SUBTRACT: c_int = 3;
pub const OP_DIVIDE: c_int = 4;
pub const OP_MODULO: c_int = 5;

pub type MathOperationRaw = *const c_void;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// `translation/` — the crate root.
pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The working directory that holds both `c_src/` and `translation/`.
pub fn work_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

/// The C `.so`. `CMakeLists.txt` names the project after the *parent* directory
/// of `c_src`, so the file name is discovered rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    let build_dir = work_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C lib first.", build_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one C .so in {}, found {found:?}", build_dir.display());
    found.pop().unwrap()
}

/// The Rust `cdylib`, next to the test binary itself (`target/<profile>/deps/..`).
///
/// Because the lib is `crate-type = ["cdylib"]` and the integration tests reach
/// it through `dlopen` rather than by linking, `cargo test` does **not** rebuild
/// it. A stale `.so` would silently invalidate every result, so the file is
/// checked against the crate sources and the run fails loudly instead.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>  ->  .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary lives in target/<profile>/deps");
    let p = profile_dir.join("libmathop_lib.so");
    assert!(p.exists(), "Rust cdylib not found at {} — run `cargo build` first", p.display());
    assert_not_stale(&p);
    p
}

fn assert_not_stale(so: &std::path::Path) {
    let so_mtime = std::fs::metadata(so).and_then(|m| m.modified());
    let src_mtime = std::fs::metadata(crate_root().join("src/lib.rs")).and_then(|m| m.modified());
    if let (Ok(so_t), Ok(src_t)) = (so_mtime, src_mtime) {
        assert!(
            so_t >= src_t,
            "{} is OLDER than src/lib.rs — `cargo test` does not rebuild a cdylib that the \
             tests only dlopen. Run `cargo build` (or `cargo build --release`) first.",
            so.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Typed view over one library
// ---------------------------------------------------------------------------

type FnIsValid = unsafe extern "C" fn(c_char) -> bool;
type FnPriority = unsafe extern "C" fn(c_int) -> c_int;
type FnBinop = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type FnSelect = unsafe extern "C" fn(c_int) -> MathOperationRaw;
type FnTimestamp = unsafe extern "C" fn() -> TimeT;
type FnAllocate = unsafe extern "C" fn(c_int) -> *mut ComputationResult;
type FnPcwh =
    unsafe extern "C" fn(c_int, c_int, c_int, *mut *mut ComputationResult, *mut c_int) -> c_int;
type FnMathop = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    lib: Library,
}

/// Resolves an exported symbol. Must be invoked inside an `unsafe` block:
/// `Library::get` and calling the resulting pointer are both unsafe.
macro_rules! sym {
    ($self:expr, $ty:ty, $name:literal) => {{
        let s: Symbol<$ty> = $self
            .lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("{}: missing symbol {}: {e}", $self.name, $name));
        *s
    }};
}

impl Lib {
    pub fn open(name: &'static str, path: PathBuf) -> Self {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Lib { name, path, lib }
    }

    pub fn c() -> Self {
        Lib::open("C", c_so_path())
    }
    pub fn rust() -> Self {
        Lib::open("Rust", rust_so_path())
    }

    /// Raw address of an exported symbol — used to *identify* the function
    /// pointer returned by `select_operation` within its own library.
    pub fn addr(&self, name: &str) -> *const c_void {
        let mut owned = name.as_bytes().to_vec();
        owned.push(0);
        let s: Symbol<*const c_void> = unsafe { self.lib.get(&owned) }
            .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.name));
        unsafe { s.into_raw().into_raw() as *const c_void }
    }

    pub fn is_valid_operation(&self, op_char: c_char) -> bool {
        unsafe { sym!(self, FnIsValid, "is_valid_operation")(op_char) }
    }
    pub fn get_operation_priority(&self, op: c_int) -> c_int {
        unsafe { sym!(self, FnPriority, "get_operation_priority")(op) }
    }
    pub fn add_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        unsafe { sym!(self, FnBinop, "add_operation")(a, b, u) }
    }
    pub fn multiply_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        unsafe { sym!(self, FnBinop, "multiply_operation")(a, b, u) }
    }
    pub fn subtract_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        unsafe { sym!(self, FnBinop, "subtract_operation")(a, b, u) }
    }
    pub fn divide_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        unsafe { sym!(self, FnBinop, "divide_operation")(a, b, u) }
    }
    pub fn modulo_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        unsafe { sym!(self, FnBinop, "modulo_operation")(a, b, u) }
    }
    pub fn select_operation(&self, op: c_int) -> MathOperationRaw {
        unsafe { sym!(self, FnSelect, "select_operation")(op) }
    }
    pub fn get_computation_timestamp(&self) -> TimeT {
        unsafe { sym!(self, FnTimestamp, "get_computation_timestamp")() }
    }
    pub fn allocate_results(&self, count: c_int) -> *mut ComputationResult {
        unsafe { sym!(self, FnAllocate, "allocate_results")(count) }
    }
    /// # Safety
    /// `history` / `history_count` must be valid, or must be deliberately
    /// invalid in a test that expects the same fault from both libraries.
    pub unsafe fn perform_computation_with_history(
        &self,
        a: c_int,
        b: c_int,
        op: c_int,
        history: *mut *mut ComputationResult,
        history_count: *mut c_int,
    ) -> c_int {
        unsafe { sym!(self, FnPcwh, "perform_computation_with_history")(a, b, op, history, history_count) }
    }
    pub fn mathop(&self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
        unsafe { sym!(self, FnMathop, "mathop")(p1, p2, p3, p4) }
    }

    /// Call a `MathOperation` pointer obtained from *this* library.
    ///
    /// # Safety
    /// `f` must be a non-null pointer returned by this library's
    /// `select_operation`.
    pub unsafe fn call_mathfn(&self, f: MathOperationRaw, a: c_int, b: c_int, u: c_int) -> c_int {
        assert!(!f.is_null(), "{}: select_operation returned NULL", self.name);
        let f: FnBinop = unsafe { std::mem::transmute(f) };
        unsafe { f(a, b, u) }
    }

    /// Which of the five leaf operations is `f`? Compared against *this*
    /// library's own exports, so it is meaningful across the two `.so`s.
    pub fn identify_mathfn(&self, f: MathOperationRaw) -> &'static str {
        const NAMES: [&str; 5] = [
            "add_operation",
            "multiply_operation",
            "subtract_operation",
            "divide_operation",
            "modulo_operation",
        ];
        for n in NAMES {
            if self.addr(n) == f {
                return n;
            }
        }
        panic!("{}: select_operation returned an unrecognised pointer {f:?}", self.name)
    }
}

/// Both libraries, opened once per test.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

impl Pair {
    pub fn new() -> Self {
        Pair { c: Lib::c(), r: Lib::rust() }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

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
    /// Uniform over the whole `i32` range.
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as i32
    }
    /// Biased toward small / boundary values as well as the full range, so
    /// randomized rows also hit the interesting neighbourhoods.
    pub fn next_i32_mixed(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 8 {
            0 => (r >> 3) as i32 % 8,
            1 => -((r >> 3) as i32 % 8),
            2 => i32::MAX - ((r >> 3) as u32 % 4) as i32,
            3 => i32::MIN.wrapping_add(((r >> 3) as u32 % 4) as i32),
            4 => (r >> 3) as i32 % 128,
            5 => (r >> 3) as i32 % 1000 - 500,
            _ => self.next_i32(),
        }
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

// ---------------------------------------------------------------------------
// Byte-level buffer helpers for `ComputationResult` arrays
// ---------------------------------------------------------------------------

/// A zeroed, 8-byte-aligned array of `ComputationResult` that can be handed to
/// either library as a caller-provided history.
///
/// Backed by `Vec<u64>` so that the storage is genuinely 8-byte aligned and
/// **every** byte — including the two 4-byte padding holes in each 24-byte
/// element — starts out zero. That makes the byte-for-byte comparisons well
/// defined instead of reading uninitialised padding.
pub struct HistoryBuf {
    words: Vec<u64>,
    len: usize,
}

impl HistoryBuf {
    pub fn zeroed(len: usize) -> Self {
        assert_eq!(RESULT_SIZE % 8, 0);
        HistoryBuf { words: vec![0u64; len * (RESULT_SIZE / 8)], len }
    }
    pub fn as_mut_ptr(&mut self) -> *mut ComputationResult {
        self.words.as_mut_ptr() as *mut ComputationResult
    }
    pub fn len(&self) -> usize {
        self.len
    }
    /// Raw bytes, including the padding holes, so comparisons are genuinely
    /// byte-for-byte.
    pub fn bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.words.as_ptr() as *const u8, self.len * RESULT_SIZE)
        }
    }
    /// Decoded view of one slot.
    pub fn slot(&self, i: usize) -> ComputationResult {
        assert!(i < self.len);
        unsafe { *(self.words.as_ptr() as *const ComputationResult).add(i) }
    }
}

/// Read `count` results out of a library-allocated (`calloc`ed) history as raw
/// bytes.
///
/// # Safety
/// `p` must point to at least `count` valid `ComputationResult`s.
pub unsafe fn read_history_bytes(p: *const ComputationResult, count: usize) -> Vec<u8> {
    assert!(!p.is_null(), "history pointer is NULL");
    unsafe { std::slice::from_raw_parts(p as *const u8, count * RESULT_SIZE).to_vec() }
}

/// Pretty byte-for-byte comparison with a useful failure message.
pub fn assert_bytes_eq(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let first = c.iter().zip(r).position(|(a, b)| a != b);
    panic!(
        "{ctx}: history bytes differ (len C={} R={}), first difference at byte {:?}\n  C: {:02x?}\n  R: {:02x?}",
        c.len(),
        r.len(),
        first,
        c,
        r
    );
}

// ---------------------------------------------------------------------------
// Crashing-path helper: run one scenario in a child process and compare how it
// died. `INT_MIN / -1` raises SIGFPE and a NULL out-param raises SIGSEGV; both
// are real, observable C behaviour that the Rust must reproduce, and neither
// can be observed in-process.
// ---------------------------------------------------------------------------

pub const CRASH_ENV: &str = "HARVEST_CRASH_SCENARIO";

/// Outcome of a child run: either it exited, or a signal killed it.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32),
    Signal(i32),
}

/// Re-executes this very test binary with `CRASH_ENV=<scenario>:<lib>` and
/// reports how the child terminated.
pub fn run_crash_child(scenario: &str, which: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let status = std::process::Command::new(exe)
        .arg("--ignored")
        .arg("--exact")
        .arg("crash_child_entry")
        .arg("--test-threads=1")
        .env(CRASH_ENV, format!("{scenario}:{which}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn crash child");
    match status.signal() {
        Some(s) => Outcome::Signal(s),
        None => Outcome::Exited(status.code().unwrap_or(-1)),
    }
}

// ---------------------------------------------------------------------------
// stdout silencing
//
// `mathop` calls `printf` four times per invocation in *both* libraries, and
// those writes go to fd 1 directly (they are not captured by the Rust test
// harness). Tests that call `mathop` thousands of times therefore point fd 1
// at /dev/null for the duration. The two libraries share this process's glibc
// `stdout`, so the buffer is flushed before the fd is swapped back.
//
// A global mutex serialises the fd juggling against other threads.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct SilentStdout {
    saved: c_int,
    devnull: c_int,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl SilentStdout {
    pub fn new() -> Self {
        let guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            fflush(std::ptr::null_mut()); // flush *all* streams
            let saved = dup(1);
            let devnull = open(c"/dev/null".as_ptr(), 1 /* O_WRONLY */);
            assert!(saved >= 0 && devnull >= 0, "could not redirect stdout");
            dup2(devnull, 1);
            SilentStdout { saved, devnull, _guard: guard }
        }
    }
}

impl Drop for SilentStdout {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
            close(self.devnull);
        }
    }
}
