// Shared differential-testing harness.
//
// Loads BOTH shared objects through `libloading`:
//   * the C  `.so` built by c_src/CMakeLists.txt
//   * the Rust `.so` (cdylib) built from translation/src/lib.rs
//
// Every call in every test goes through these dynamically resolved symbols, so
// the `#[no_mangle]` / `extern "C"` export wrappers are exercised exactly the
// way an external consumer would exercise them. Rust functions are NEVER called
// directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Types mirroring the C ABI
// ---------------------------------------------------------------------------

/// `typedef int (*operation_func)(int, int);`
pub type OperationFunc = unsafe extern "C" fn(c_int, c_int) -> c_int;
/// Nullable `operation_func` (null-pointer-optimised => ABI compatible).
pub type OptOperationFunc = Option<OperationFunc>;

/// `typedef struct { int accumulator; int operation_count; unsigned int checksum; } ComputeState;`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

/// A `ComputeState` embedded in a larger buffer with a poison guard after it, so
/// that a translation writing too many bytes is detected.
pub const STATE_SIZE: usize = std::mem::size_of::<ComputeState>();
pub const GUARD_BUF: usize = 32;
pub const GUARD_BYTE: u8 = 0xA5;

/// Byte buffer big enough for a `ComputeState` plus a poison guard region.
#[repr(C, align(8))]
pub struct StateBuf(pub [u8; GUARD_BUF]);

impl StateBuf {
    pub fn new() -> Self {
        StateBuf([GUARD_BYTE; GUARD_BUF])
    }
    pub fn as_ptr(&mut self) -> *mut ComputeState {
        self.0.as_mut_ptr() as *mut ComputeState
    }
    pub fn state(&self) -> ComputeState {
        unsafe { *(self.0.as_ptr() as *const ComputeState) }
    }
    pub fn set_state(&mut self, s: ComputeState) {
        unsafe { *(self.0.as_mut_ptr() as *mut ComputeState) = s };
    }
    /// Full raw bytes, including the guard region.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn guard(&self) -> &[u8] {
        &self.0[STATE_SIZE..]
    }
    pub fn guard_intact(&self) -> bool {
        self.guard().iter().all(|&b| b == GUARD_BYTE)
    }
}

// ---------------------------------------------------------------------------
// Function-pointer table resolved out of one shared object
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub multiply_with_static: OperationFunc,
    pub add_with_static: OperationFunc,
    pub xor_operation: OperationFunc,
    pub shift_with_static: OperationFunc,
    pub get_operation: unsafe extern "C" fn(c_int) -> OptOperationFunc,
    pub execute_operation:
        unsafe extern "C" fn(OptOperationFunc, c_int, c_int, *const c_char) -> c_int,
    pub compute_checksum: unsafe extern "C" fn(*mut c_int, c_int) -> c_uint,
    pub init_state: unsafe extern "C" fn(*mut ComputeState, c_int),
    pub apply_operation: unsafe extern "C" fn(*mut ComputeState, c_int, OptOperationFunc),
    pub checkshift: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("symbol {:?} not found: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Lib {
    fn load(name: &'static str, path: PathBuf) -> Lib {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()))
        }));
        unsafe {
            Lib {
                name,
                path,
                multiply_with_static: sym(lib, b"multiply_with_static"),
                add_with_static: sym(lib, b"add_with_static"),
                xor_operation: sym(lib, b"xor_operation"),
                shift_with_static: sym(lib, b"shift_with_static"),
                get_operation: sym(lib, b"get_operation"),
                execute_operation: sym(lib, b"execute_operation"),
                compute_checksum: sym(lib, b"compute_checksum"),
                init_state: sym(lib, b"init_state"),
                apply_operation: sym(lib, b"apply_operation"),
                checkshift: sym(lib, b"checkshift"),
            }
        }
    }

    /// The four arithmetic kernels, indexed the way `get_operation` indexes them.
    pub fn kernel(&self, opcode: usize) -> OperationFunc {
        match opcode {
            0 => self.multiply_with_static,
            1 => self.add_with_static,
            2 => self.xor_operation,
            3 => self.shift_with_static,
            _ => panic!("no kernel {opcode}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workdir() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workdir().join("c_src/build");
    let mut hits: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                hits.push(p);
            }
        }
    }
    hits.sort();
    hits.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Load the cdylib built with the SAME profile as this test binary, so that a
    // `cargo test` run and a `cargo test --release` run each verify the artifact
    // they actually correspond to. Optimisation level changes code generation
    // (e.g. whether an observable libc call survives), so this matters.
    let base = manifest_dir().join("target");
    let (first, second) = if cfg!(debug_assertions) {
        ("debug", "release")
    } else {
        ("release", "debug")
    };
    let preferred = base.join(format!("{first}/libcheckshift_lib.so"));
    if preferred.exists() {
        return preferred;
    }
    let fallback = base.join(format!("{second}/libcheckshift_lib.so"));
    if fallback.exists() {
        eprintln!(
            "warning: {first} cdylib not found, falling back to {}",
            fallback.display()
        );
        return fallback;
    }
    panic!(
        "no Rust cdylib found under {}. Build it with:\n  cd translation && cargo build --release",
        base.display()
    )
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c_lib() -> &'static Lib {
    C_LIB.get_or_init(|| Lib::load("C", find_c_so()))
}

pub fn rust_lib() -> &'static Lib {
    RUST_LIB.get_or_init(|| Lib::load("Rust", find_rust_so()))
}

/// Both libraries, as a convenience pair.
pub fn libs() -> (&'static Lib, &'static Lib) {
    (c_lib(), rust_lib())
}

// ---------------------------------------------------------------------------
// stdout capture
//
// The library under test writes with libc `printf`, so we capture at the file
// descriptor level: flush libc's stdout, dup2 a temp file over fd 1, run the
// closure, flush again, then restore fd 1.
//
// `cargo test` runs test functions on parallel threads and fd 1 is
// process-global, so all captures are serialised through CAPTURE_LOCK.
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` with fd 1 redirected to a temp file; return `f`'s value and the exact
/// bytes it wrote to stdout.
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Push out anything already sitting in libc's / Rust's stdout buffers so it
    // cannot leak into our capture file.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let mut tmp = tempfile();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let out = f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    tmp.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).unwrap();
    check_uncontaminated(&buf);
    (out, buf)
}

/// fd 1 is process-global. If libtest is running tests on more than one thread,
/// its progress output can land inside a capture window and masquerade as a
/// divergence. `.cargo/config.toml` sets `RUST_TEST_THREADS=1` to prevent this;
/// this check turns any remaining leak into an unmistakable message instead of a
/// confusing false failure.
fn check_uncontaminated(buf: &[u8]) {
    let s = String::from_utf8_lossy(buf);
    for marker in ["\ntest ", "running ", " ... ok", "test result:"] {
        if s.contains(marker) {
            panic!(
                "captured stdout contains libtest output ({marker:?}) -- the test harness is \
                 running multi-threaded and is polluting the capture.\nRe-run with \
                 `cargo test -- --test-threads=1` (or ensure RUST_TEST_THREADS=1).\nCaptured: {:?}",
                s
            );
        }
    }
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let n = N.fetch_add(1, Ordering::Relaxed);
    let path = PathBuf::from(dir).join(format!(
        "difftest-{}-{}-{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", path.display()));
    // Unlink immediately; the fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

/// Pretty-print captured bytes for assertion messages.
pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert two captured stdout streams are byte-identical.
#[track_caller]
pub fn assert_stdout_eq(ctx: &str, c_out: &[u8], rust_out: &[u8]) {
    if c_out != rust_out {
        panic!(
            "stdout divergence [{ctx}]\n  C    ({} bytes): \"{}\"\n  Rust ({} bytes): \"{}\"",
            c_out.len(),
            show(c_out),
            rust_out.len(),
            show(rust_out)
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
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
    /// Full-range i32.
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// A value biased toward "interesting" shapes: boundaries, small magnitudes,
    /// powers of two, and full-range noise.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(8) {
            0 => *pick(INTERESTING, self.next_u32() as usize),
            1 => (self.next_u32() % 16) as i32 - 8,
            2 => 1i32 << (self.next_u32() % 32),
            3 => (1i32 << (self.next_u32() % 32)).wrapping_sub(1),
            4 => -((1i32 << (self.next_u32() % 31)) as i32),
            _ => self.i32(),
        }
    }
}

fn pick<T>(s: &[T], i: usize) -> &T {
    &s[i % s.len()]
}

/// Boundary values that the arithmetic kernels treat distinctly.
pub const INTERESTING: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    4,
    -4,
    100,
    -100,
    0xABCD,
    -0xABCD,
    0xFFFF,
    -0xFFFF,
    0x1_0000,
    0x7FFF_FFFF,       // INT_MAX
    -0x8000_0000i64 as i32, // INT_MIN
    0x7FFF_FFFE,       // INT_MAX - 1
    -0x7FFF_FFFF,      // INT_MIN + 1
    0x4000_0000,
    0x2000_0000,
    0x6000_0000,
    0xC000_0000u32 as i32,
    0x8000_0001u32 as i32,
    0xAAAA_AAAAu32 as i32,
    0x5555_5555,
    0x0102_0304,
    0xFF00_0000u32 as i32,
    0x0000_00FFu32 as i32,
    0xDEAD_BEEFu32 as i32,
];
