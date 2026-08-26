//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls everything through
//! `dlsym`, so the Rust side is exercised exactly like an external C consumer
//! would exercise it (this also tests the `#[no_mangle]` export wrappers).
//!
//! Nothing from the `checkshift_lib` crate is ever called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// ABI mirror of the C types (must match `c_src/src/lib.c` exactly)
// ---------------------------------------------------------------------------

/// `typedef int (*operation_func)(int, int)`; `None` == C `NULL`.
pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

/// ```c
/// typedef struct { int accumulator; int operation_count; unsigned int checksum; } ComputeState;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

impl ComputeState {
    /// Raw 12-byte object representation, for byte-for-byte comparison.
    pub fn bytes(&self) -> [u8; std::mem::size_of::<ComputeState>()] {
        unsafe {
            std::ptr::read(self as *const ComputeState as *const [u8; std::mem::size_of::<ComputeState>()])
        }
    }
}

type FnIi = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnGetOperation = unsafe extern "C" fn(c_int) -> OperationFunc;
type FnExecuteOperation = unsafe extern "C" fn(OperationFunc, c_int, c_int, *const c_char) -> c_int;
type FnComputeChecksum = unsafe extern "C" fn(*mut c_int, c_int) -> c_uint;
type FnInitState = unsafe extern "C" fn(*mut ComputeState, c_int);
type FnApplyOperation = unsafe extern "C" fn(*mut ComputeState, c_int, OperationFunc);
type FnCheckshift = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture (declared directly; no `libc` crate)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// One loaded library, with every exported symbol resolved through dlsym
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    multiply_with_static: FnIi,
    add_with_static: FnIi,
    xor_operation: FnIi,
    shift_with_static: FnIi,
    get_operation: FnGetOperation,
    execute_operation: FnExecuteOperation,
    compute_checksum: FnComputeChecksum,
    init_state: FnInitState,
    apply_operation: FnApplyOperation,
    checkshift: FnCheckshift,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $name:literal) => {{
        let s: Symbol<$t> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("dlsym {} failed: {e}", $name));
        // Deref out of the `Symbol` guard: the `Library` is kept alive in the
        // same struct, so the raw pointer stays valid for the process lifetime.
        *s
    }};
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        let l = Lib {
            name,
            multiply_with_static: sym!(lib, FnIi, "multiply_with_static"),
            add_with_static: sym!(lib, FnIi, "add_with_static"),
            xor_operation: sym!(lib, FnIi, "xor_operation"),
            shift_with_static: sym!(lib, FnIi, "shift_with_static"),
            get_operation: sym!(lib, FnGetOperation, "get_operation"),
            execute_operation: sym!(lib, FnExecuteOperation, "execute_operation"),
            compute_checksum: sym!(lib, FnComputeChecksum, "compute_checksum"),
            init_state: sym!(lib, FnInitState, "init_state"),
            apply_operation: sym!(lib, FnApplyOperation, "apply_operation"),
            checkshift: sym!(lib, FnCheckshift, "checkshift"),
            path,
            _lib: lib,
        };
        l
    }

    // -- direct wrappers ----------------------------------------------------

    pub fn multiply_with_static(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.multiply_with_static)(a, b) }
    }
    pub fn add_with_static(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.add_with_static)(a, b) }
    }
    pub fn xor_operation(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.xor_operation)(a, b) }
    }
    pub fn shift_with_static(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.shift_with_static)(a, b) }
    }
    pub fn get_operation(&self, opcode: c_int) -> OperationFunc {
        unsafe { (self.get_operation)(opcode) }
    }
    /// Address of this `.so`'s own exported arithmetic symbol, for pointer
    /// identity checks against `get_operation`.
    pub fn op_symbol_addr(&self, opcode: c_int) -> usize {
        let f: FnIi = match opcode {
            0 => self.multiply_with_static,
            1 => self.add_with_static,
            2 => self.xor_operation,
            3 => self.shift_with_static,
            _ => panic!("bad opcode"),
        };
        f as usize
    }
    pub unsafe fn execute_operation(
        &self,
        func: OperationFunc,
        a: c_int,
        b: c_int,
        name: *const c_char,
    ) -> c_int {
        (self.execute_operation)(func, a, b, name)
    }
    pub unsafe fn compute_checksum(&self, values: *mut c_int, count: c_int) -> c_uint {
        (self.compute_checksum)(values, count)
    }
    pub unsafe fn init_state(&self, state: *mut ComputeState, initial: c_int) {
        (self.init_state)(state, initial)
    }
    pub unsafe fn apply_operation(&self, state: *mut ComputeState, value: c_int, f: OperationFunc) {
        (self.apply_operation)(state, value, f)
    }
    pub fn checkshift(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        unsafe { (self.checkshift)(a, b, c, d) }
    }
    /// Raw resolved addresses of all ten exported symbols, for harness
    /// self-checks (the two `.so`s must never resolve to the same code).
    pub fn all_symbol_addrs(&self) -> [usize; 10] {
        [
            self.multiply_with_static as usize,
            self.add_with_static as usize,
            self.xor_operation as usize,
            self.shift_with_static as usize,
            self.get_operation as usize,
            self.execute_operation as usize,
            self.compute_checksum as usize,
            self.init_state as usize,
            self.apply_operation as usize,
            self.checkshift as usize,
        ]
    }
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the test executable's own location
/// (`target/<profile>/deps/<test>-<hash>`), so it follows `--release` etc.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    target_profile_dir().join("libcheckshift_lib.so")
}

// `printf` from the process's libc, for the allocator-fault child (its output
// must interleave with the library's own `printf` exactly).
extern "C" {
    pub fn printf(fmt: *const c_char, ...) -> c_int;
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

/// The reference implementation (ground truth).
pub fn c() -> &'static Lib {
    C_LIB.get_or_init(|| {
        let p = c_so_path();
        assert!(
            p.exists(),
            "C shared object not found at {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            p.display()
        );
        Lib::open("C", p)
    })
}

/// The Rust translation, loaded as a plain `.so` through `dlopen`/`dlsym`.
pub fn r() -> &'static Lib {
    RUST_LIB.get_or_init(|| {
        let p = rust_so_path();
        assert!(
            p.exists(),
            "Rust shared object not found at {} (run `cargo build` first)",
            p.display()
        );
        Lib::open("Rust", p)
    })
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both `.so`s and this test binary share one glibc, hence one `stdout` FILE.
// We `fflush(NULL)` (flush *all* streams), swap fd 1 for a temp file, run the
// closure, flush again and restore. fd 1 is process-global so a single global
// lock serialises every capture; `cargo test` is also run with
// `--test-threads=1` by the driver scripts.
// ---------------------------------------------------------------------------

fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Run `f` with fd 1 redirected to a temp file; return `(f's value, stdout bytes)`.
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = capture_lock();

    let n = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "chkshift-cap-{}-{}.out",
        std::process::id(),
        n
    ));

    // Flush everything the process has buffered so far so it lands on the real
    // stdout, not into our capture file.
    unsafe { fflush(std::ptr::null_mut()) };

    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 onto fd 1 failed");

    let out = f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore fd 1 failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    (out, bytes)
}

/// Pretty-print captured bytes for assertion messages.
pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Run the same closure against the C lib and the Rust lib and assert that both
/// the returned value and the captured stdout match byte for byte.
pub fn diff<T: PartialEq + std::fmt::Debug>(ctx: &str, f: impl Fn(&'static Lib) -> T) {
    let (cv, cout) = capture(|| f(c()));
    let (rv, rout) = capture(|| f(r()));
    assert_eq!(cv, rv, "return value mismatch [{ctx}]");
    assert_eq!(
        cout,
        rout,
        "stdout mismatch [{ctx}]\n  C   : {}\n  Rust: {}",
        show(&cout),
        show(&rout)
    );
}

/// Like [`diff`] but the closure also reports an out-of-band byte buffer
/// (e.g. the raw `ComputeState` after the call) that must match too.
pub fn diff_bytes<T: PartialEq + std::fmt::Debug>(
    ctx: &str,
    f: impl Fn(&'static Lib) -> (T, Vec<u8>),
) {
    let ((cv, cbuf), cout) = capture(|| f(c()));
    let ((rv, rbuf), rout) = capture(|| f(r()));
    assert_eq!(cv, rv, "return value mismatch [{ctx}]");
    assert_eq!(cbuf, rbuf, "out-param bytes mismatch [{ctx}]");
    assert_eq!(
        cout,
        rout,
        "stdout mismatch [{ctx}]\n  C   : {}\n  Rust: {}",
        show(&cout),
        show(&rout)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + shared input corpora
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// A value biased towards "interesting" magnitudes: small, medium, huge.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(6) {
            0 => (self.next_u32() % 8) as i32,          // tiny
            1 => -((self.next_u32() % 8) as i32),       // tiny negative
            2 => (self.next_u32() % 0x1_0000) as i32,   // 16-bit
            3 => -((self.next_u32() % 0x1_0000) as i32),
            4 => EDGES[self.below(EDGES.len() as u32) as usize],
            _ => self.next_i32(),                       // full range
        }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }
}

/// Boundary values for every `int` parameter.
pub const EDGES: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    4,
    7,
    -7,
    0x7FFF,
    -0x8000,
    0xFFFF,
    0x1_0000,
    0x5555_5555u32 as i32,
    -0x5555_5556i64 as i32,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
];

/// Iterations for the property-style rows. Override with `DIFF_ITERS`.
pub fn iters(default: usize) -> usize {
    std::env::var("DIFF_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
