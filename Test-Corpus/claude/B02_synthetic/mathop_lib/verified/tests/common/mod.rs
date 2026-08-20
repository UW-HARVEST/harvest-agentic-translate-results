// Shared harness for the C-vs-Rust differential tests.
//
// Both shared objects are loaded with `libloading` and driven *only* through
// their exported symbols, so the `#[no_mangle] extern "C"` wrappers are part of
// what is under test. Nothing in the Rust crate is called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (the test binary links libc).
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(p: *mut c_void);
    fn time(t: *mut i64) -> i64;
}

/// `free()` a block that was handed out by either library's `allocate_results`.
pub unsafe fn libc_free<T>(p: *mut T) {
    if !p.is_null() {
        free(p as *mut c_void);
    }
}

/// Current `time()` reading, for cross-checking `get_computation_timestamp`.
pub fn now() -> i64 {
    let mut t: i64 = 0;
    unsafe { time(&mut t) }
}

// ---------------------------------------------------------------------------
// The C `ComputationResult`.
//
//   typedef struct { int value; time_t timestamp; StatusCode status; }
//
// On the reference ABI: size 24, align 8, offsets 0 / 8 / 16.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: i64,
    pub status: c_int,
}

pub const STATUS_SUCCESS: c_int = 0;
pub const STATUS_ERROR: c_int = -1;
pub const STATUS_WARNING: c_int = 1;

pub const OP_ADD: c_int = 1;
pub const OP_MULTIPLY: c_int = 2;
pub const OP_SUBTRACT: c_int = 3;
pub const OP_DIVIDE: c_int = 4;
pub const OP_MODULO: c_int = 5;

// ---------------------------------------------------------------------------
// Exported-symbol signatures.
//
// `bool` is deliberately loaded as `u8` so that a non-0/1 byte coming back from
// either library is *observed and compared* rather than being instant UB in the
// test harness.
// `select_operation`'s `MathOperation` return is loaded as `usize` so the
// returned code address can be compared against the library's own exported
// symbols (identity of the selected function, not just its numeric output).
// ---------------------------------------------------------------------------

pub type FnIsValid = unsafe extern "C" fn(c_char) -> u8;
pub type FnPriority = unsafe extern "C" fn(c_int) -> c_int;
pub type FnBinOp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnSelect = unsafe extern "C" fn(c_int) -> usize;
pub type FnTimestamp = unsafe extern "C" fn() -> i64;
pub type FnAllocate = unsafe extern "C" fn(c_int) -> *mut ComputationResult;
pub type FnPerform = unsafe extern "C" fn(
    c_int,
    c_int,
    c_int,
    *mut *mut ComputationResult,
    *mut c_int,
) -> c_int;
pub type FnMathop = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded library plus every symbol from `SYMBOLS.md`.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub is_valid_operation: FnIsValid,
    pub get_operation_priority: FnPriority,
    pub add_operation: FnBinOp,
    pub multiply_operation: FnBinOp,
    pub subtract_operation: FnBinOp,
    pub divide_operation: FnBinOp,
    pub modulo_operation: FnBinOp,
    pub select_operation: FnSelect,
    pub get_computation_timestamp: FnTimestamp,
    pub allocate_results: FnAllocate,
    pub perform_computation_with_history: FnPerform,
    pub mathop: FnMathop,
    /// Addresses of the five exported op symbols *in this library*, in
    /// `[add, multiply, subtract, divide, modulo]` order — used to identify
    /// what `select_operation` returned.
    pub op_addrs: [usize; 5],
    _lib: Library,
}

fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

fn addr_of(lib: &Library, name: &[u8]) -> usize {
    unsafe {
        let s: Symbol<*const c_void> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        // For a function symbol libloading hands back the code address itself.
        s.into_raw().into_raw() as usize
    }
}

impl Api {
    fn load(name: &'static str, path: PathBuf) -> Api {
        let lib = unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()))
        };
        let api = Api {
            name,
            is_valid_operation: sym(&lib, b"is_valid_operation\0"),
            get_operation_priority: sym(&lib, b"get_operation_priority\0"),
            add_operation: sym(&lib, b"add_operation\0"),
            multiply_operation: sym(&lib, b"multiply_operation\0"),
            subtract_operation: sym(&lib, b"subtract_operation\0"),
            divide_operation: sym(&lib, b"divide_operation\0"),
            modulo_operation: sym(&lib, b"modulo_operation\0"),
            select_operation: sym(&lib, b"select_operation\0"),
            get_computation_timestamp: sym(&lib, b"get_computation_timestamp\0"),
            allocate_results: sym(&lib, b"allocate_results\0"),
            perform_computation_with_history: sym(&lib, b"perform_computation_with_history\0"),
            mathop: sym(&lib, b"mathop\0"),
            op_addrs: [
                addr_of(&lib, b"add_operation\0"),
                addr_of(&lib, b"multiply_operation\0"),
                addr_of(&lib, b"subtract_operation\0"),
                addr_of(&lib, b"divide_operation\0"),
                addr_of(&lib, b"modulo_operation\0"),
            ],
            path,
            _lib: lib,
        };
        api
    }

    /// Map a code address returned by `select_operation` onto an index into
    /// `[add, multiply, subtract, divide, modulo]`.
    pub fn identify_op(&self, addr: usize) -> Option<usize> {
        self.op_addrs.iter().position(|&a| a == addr)
    }

    pub fn op_by_index(&self, idx: usize) -> FnBinOp {
        match idx {
            0 => self.add_operation,
            1 => self.multiply_operation,
            2 => self.subtract_operation,
            3 => self.divide_operation,
            4 => self.modulo_operation,
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

pub fn c_so_path() -> PathBuf {
    // `C_SO_PATH` lets the same suite be pointed at an alternative build of the
    // C reference (e.g. an -O2 one) without touching c_src/.
    if let Some(p) = std::env::var_os("C_SO_PATH") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    // The test executable lives in <target>/<profile>/deps/, next to the cdylib.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let candidates = [
        deps.join("libmathop_lib.so"),
        deps.parent().unwrap_or(deps).join("libmathop_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("libmathop_lib.so not found near {}", deps.display());
}

static C_API: OnceLock<Api> = OnceLock::new();
static R_API: OnceLock<Api> = OnceLock::new();

pub fn c() -> &'static Api {
    C_API.get_or_init(|| {
        let p = c_so_path();
        assert!(
            p.exists(),
            "C shared library missing at {} — build it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            p.display()
        );
        Api::load("C", p)
    })
}

pub fn r() -> &'static Api {
    R_API.get_or_init(|| {
        let p = rust_so_path();
        assert_stale_free(&p);
        Api::load("Rust", p)
    })
}

/// `cargo test` does NOT rebuild a `cdylib` that the test crates do not link
/// against, so a stale `libmathop_lib.so` would silently make every differential
/// test compare against old code. Refuse to run in that case.
fn assert_stale_free(so: &std::path::Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("lib.rs");
    let m = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or_else(|e| panic!("cannot stat {}: {e}", p.display()))
    };
    let so_t = m(so);
    let src_t = m(&src);
    assert!(
        so_t >= src_t,
        "{} is OLDER than {} — run `cargo build` (not just `cargo test`) so the \
         cdylib is refreshed before the differential tests run",
        so.display(),
        src.display()
    );
}

/// Both APIs at once.
pub fn both() -> (&'static Api, &'static Api) {
    (c(), r())
}

// ---------------------------------------------------------------------------
// Serialization
//
// `mathop` mutates process-global state in each library and stdout capture
// swaps fd 1 process-wide, so those tests must not run concurrently.
// ---------------------------------------------------------------------------

static SERIAL: Mutex<()> = Mutex::new(());

pub fn serial() -> MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// stdout capture (libc `printf` writes straight to fd 1, so the Rust test
// harness's own capture does not see it — redirect the descriptor instead).
// ---------------------------------------------------------------------------

pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let mut tmp = tempfile();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read");
    out
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "mathop_diff_{}_{}.txt",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp file");
    let _ = std::fs::remove_file(&path); // unlink; fd keeps it alive
    f
}

// ---------------------------------------------------------------------------
// Running a call in a forked child, so that faulting inputs (null derefs,
// signed-division traps) can be compared between the two libraries without
// taking the test process down with them.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(c_int),
    Signaled(c_int),
}

pub const SIGSEGV: c_int = 11;
pub const SIGBUS: c_int = 7;
pub const SIGFPE: c_int = 8;
pub const SIGABRT: c_int = 6;
pub const SIGILL: c_int = 4;
pub const SIGTRAP: c_int = 5;

/// Run `f` in a forked child; report how the child terminated.
/// `f` should call `finish(code)` to exit with a specific code.
pub fn run_in_child<F: FnOnce() -> c_int>(f: F) -> Outcome {
    unsafe {
        fflush(std::ptr::null_mut());
    }
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: no panicking, no locks — just the call, then _exit.
        let code = f();
        unsafe {
            fflush(std::ptr::null_mut());
            _exit(code);
        }
    }
    let mut status: c_int = 0;
    let got = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(got, pid, "waitpid failed");
    if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signaled(status & 0x7f)
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seeds, reproducible runs.
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
    /// Uniform over the whole `i32` range.
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    /// A "spicy" i32: biased toward boundaries and small magnitudes, which is
    /// where the value-dependent behaviour lives, but still covers full range.
    pub fn spicy_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => *self.pick(&BOUNDARY),
            1 => self.range(-16, 16),
            2 => self.range(-300, 300),
            3 => self.range(i32::MIN, i32::MIN + 64),
            4 => self.range(i32::MAX - 64, i32::MAX),
            _ => self.i32(),
        }
    }
}

/// Interesting `int` values: signed boundaries, small magnitudes, and the
/// `char`/`% 128` and `% 5` inflection points the C code cares about.
pub const BOUNDARY: [i32; 26] = [
    i32::MIN,
    i32::MIN + 1,
    i32::MIN + 2,
    -2147483647,
    -1000000,
    -129,
    -128,
    -127,
    -6,
    -5,
    -2,
    -1,
    0,
    1,
    2,
    4,
    5,
    6,
    48,
    49,
    53,
    54,
    127,
    128,
    i32::MAX - 1,
    i32::MAX,
];

/// `divide_operation` / `modulo_operation` on `(INT_MIN, -1)` is signed-overflow
/// UB in C: the reference build executes `idiv` and dies from SIGFPE, so there
/// is no defined C result to be identical to (ERRORS.md E21). Randomized
/// generators route around it.
pub fn is_idiv_ub(a: c_int, b: c_int) -> bool {
    a == i32::MIN && b == -1
}

/// Does `mathop(p1, p2, p3, p4)` reach an `idiv` UB case? Mirrors `lib.c`
/// exactly so the filter is provably the same condition.
pub fn mathop_is_ub(p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> bool {
    let op1 = p3.wrapping_rem(5).wrapping_add(1);
    if (op1 == OP_DIVIDE || op1 == OP_MODULO) && is_idiv_ub(p1, p2) {
        return true;
    }
    // Reproduce the first computation to test the second call's operands.
    let intermediate = match op1 {
        OP_MULTIPLY => p1.wrapping_mul(p2),
        OP_SUBTRACT => p1.wrapping_sub(p2),
        OP_DIVIDE => {
            if p2 == 0 {
                0
            } else {
                p1.wrapping_div(p2)
            }
        }
        OP_MODULO => {
            if p2 == 0 {
                0
            } else {
                p1.wrapping_rem(p2)
            }
        }
        _ => p1.wrapping_add(p2), // OP_ADD and every out-of-range value
    };
    let op2 = p4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);
    (op2 == OP_DIVIDE || op2 == OP_MODULO) && is_idiv_ub(intermediate, p4)
}
