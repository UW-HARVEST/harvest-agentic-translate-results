// Shared differential-test harness.
//
// Both the C library and the Rust library are loaded as shared objects with
// `libloading` and driven exclusively through their exported symbols, so the
// `#[no_mangle]` / `extern "C"` wrappers are part of what is under test. No Rust
// function is ever called directly.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Exported signatures, as an external C consumer sees them.
//
// `operation_func` values are carried as raw `*const c_void` so that a pointer
// produced by one library can be handed to the other library, and so that a
// NULL return can be observed without constructing an invalid `fn` value.
// ---------------------------------------------------------------------------
pub type BinOp = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type GetOperation = unsafe extern "C" fn(c_int) -> *const c_void;
pub type ExecuteOperation =
    unsafe extern "C" fn(*const c_void, c_int, c_int, *const c_char) -> c_int;
pub type ComputeChecksum = unsafe extern "C" fn(*mut c_int, c_int) -> c_uint;
pub type InitState = unsafe extern "C" fn(*mut u8, c_int);
pub type ApplyOperation = unsafe extern "C" fn(*mut u8, c_int, *const c_void);
pub type Checkshift = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// sizeof(ComputeState) == 3 * sizeof(int); no padding (4+4+4, align 4).
pub const STATE_SIZE: usize = 12;

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub multiply_with_static: BinOp,
    pub add_with_static: BinOp,
    pub xor_operation: BinOp,
    pub shift_with_static: BinOp,
    pub get_operation: GetOperation,
    pub execute_operation: ExecuteOperation,
    pub compute_checksum: ComputeChecksum,
    pub init_state: InitState,
    pub apply_operation: ApplyOperation,
    pub checkshift: Checkshift,
}

/// Every symbol the C `.so` exports; the Rust `.so` must export all of them.
pub const EXPECTED_SYMBOLS: [&str; 10] = [
    "add_with_static",
    "apply_operation",
    "checkshift",
    "compute_checksum",
    "execute_operation",
    "get_operation",
    "init_state",
    "multiply_with_static",
    "shift_with_static",
    "xor_operation",
];

unsafe fn sym<T: Copy>(lib: &Library, name: &str, which: &str) -> T {
    let full = format!("{name}\0");
    let s: Symbol<T> = unsafe {
        lib.get(full.as_bytes())
            .unwrap_or_else(|e| panic!("{which} .so is missing exported symbol `{name}`: {e}"))
    };
    *s
}

impl Api {
    fn open(name: &'static str, path: PathBuf) -> Api {
        let lib = unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("failed to dlopen {path:?}: {e}"))
        };
        unsafe {
            Api {
                name,
                multiply_with_static: sym(&lib, "multiply_with_static", name),
                add_with_static: sym(&lib, "add_with_static", name),
                xor_operation: sym(&lib, "xor_operation", name),
                shift_with_static: sym(&lib, "shift_with_static", name),
                get_operation: sym(&lib, "get_operation", name),
                execute_operation: sym(&lib, "execute_operation", name),
                compute_checksum: sym(&lib, "compute_checksum", name),
                init_state: sym(&lib, "init_state", name),
                apply_operation: sym(&lib, "apply_operation", name),
                checkshift: sym(&lib, "checkshift", name),
                path,
                _lib: lib,
            }
        }
    }

    /// The four leaf operations, in `get_operation` opcode order.
    pub fn leaf(&self, opcode: c_int) -> BinOp {
        match opcode {
            0 => self.multiply_with_static,
            1 => self.add_with_static,
            2 => self.xor_operation,
            3 => self.shift_with_static,
            _ => panic!("no leaf op for opcode {opcode}"),
        }
    }

    pub fn leaf_name(opcode: c_int) -> &'static str {
        ["multiply_with_static", "add_with_static", "xor_operation", "shift_with_static"]
            [opcode as usize]
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects.
// ---------------------------------------------------------------------------
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {dir:?} ({e}); build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {dir:?}, found {found:?}");
    found.pop().unwrap()
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libcheckshift_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no libcheckshift_lib.so under {base:?}; run `cargo build --release` first");
}

static C_API: OnceLock<Api> = OnceLock::new();
static R_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| Api::open("C", c_so_path()))
}

pub fn rust_api() -> &'static Api {
    R_API.get_or_init(|| Api::open("Rust", rust_so_path()))
}

/// `(c, rust)` — the two implementations under differential test.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed per test for reproducibility.
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
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Mix of full-range values and "interesting" small/extreme values, so that
    /// randomized rows also hit the value classes the C code branches on.
    pub fn next_i32_biased(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 8 {
            0 => 0,
            1 => (r >> 3) as i8 as c_int,               // tiny, both signs
            2 => (r >> 3) as i16 as c_int,              // 16-bit
            3 => *pick(&INTERESTING, (r >> 3) as usize),
            _ => self.next_i32(),                        // full range
        }
    }
}

fn pick<T>(xs: &[T], i: usize) -> &T {
    &xs[i % xs.len()]
}

/// Boundary values derived from the C source: the `static` tuning constants
/// (3, 100, 2), the literals (`0xABCD`, `MAGIC_NUMBER`, `MASK_LOWER`) and the
/// `int` extremes / shift-relevant high-bit patterns.
pub const INTERESTING: [c_int; 26] = [
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
    !0xABCD,
    0xFFFF,
    0x1_0000,
    -0x1_0000,
    0x2000_0000,
    0x4000_0000,
    0x7FFF_FFFF,
    -0x8000_0000,
    -0x7FFF_FFFF,
    0x7FFF_FFFF / 3,
    -0x8000_0000i32 / 3,
    0x7FFF_FFFF - 100,
    -0x8000_0000i32 + 100,
    0xDEAD_BEEFu32 as c_int,
];

// ---------------------------------------------------------------------------
// A 4-byte-aligned ComputeState byte buffer, so the raw 12 bytes can be
// compared between the two libraries.
// ---------------------------------------------------------------------------
#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StateBuf(pub [u8; STATE_SIZE]);

impl StateBuf {
    pub fn poisoned() -> StateBuf {
        StateBuf([0xAA; STATE_SIZE])
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
    fn field(&self, i: usize) -> i32 {
        i32::from_ne_bytes(self.0[i * 4..i * 4 + 4].try_into().unwrap())
    }
    pub fn accumulator(&self) -> i32 {
        self.field(0)
    }
    pub fn operation_count(&self) -> i32 {
        self.field(1)
    }
    pub fn checksum(&self) -> u32 {
        self.field(2) as u32
    }
}

impl std::fmt::Debug for StateBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ComputeState {{ accumulator: {}, operation_count: {}, checksum: 0x{:08X} }} raw={:02X?}",
            self.accumulator(),
            self.operation_count(),
            self.checksum(),
            self.0
        )
    }
}

// ---------------------------------------------------------------------------
// NUL-terminated C string helper.
// ---------------------------------------------------------------------------
pub fn cstring(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// stdout capture at the file-descriptor level.
//
// Both libraries write through the process-wide libc `stdout`, so the only way
// to compare their emitted bytes is to redirect fd 1. `fflush(NULL)` flushes
// every stream (the C `.so` reaches `puts`, the Rust `.so` reaches `printf`;
// both share the same FILE*).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const SEEK_SET: c_int = 0;

/// Serializes fd-1 hijacking; `cargo test` runs tests in threads.
pub static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` while holding the stdout lock. Every test that calls a library
/// function which `printf`s must use this, whether or not it captures, so that
/// stray output cannot leak into another test's captured buffer.
pub fn serial<R>(f: impl FnOnce() -> R) -> R {
    let _g = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

// ---------------------------------------------------------------------------
// Minimal sequential test runner for the `harness = false` targets.
//
// The default libtest harness writes its own progress text to fd 1 from worker
// threads, which lands inside `capture_stdout`'s buffer and corrupts byte-exact
// stdout comparisons. These targets therefore run their own runner, which
// reports to *stderr* only, leaving fd 1 untouched except by the two libraries.
// ---------------------------------------------------------------------------
pub struct Runner {
    filter: Option<String>,
    passed: usize,
    skipped: usize,
    failed: Vec<String>,
}

impl Runner {
    pub fn new() -> Runner {
        let filter = std::env::args().skip(1).find(|a| !a.starts_with('-'));
        Runner { filter, passed: 0, skipped: 0, failed: Vec::new() }
    }

    pub fn case(&mut self, name: &str, f: impl FnOnce() + std::panic::UnwindSafe) {
        if let Some(f) = &self.filter {
            if !name.contains(f.as_str()) {
                self.skipped += 1;
                return;
            }
        }
        eprint!("test {name} ... ");
        match std::panic::catch_unwind(f) {
            Ok(()) => {
                self.passed += 1;
                eprintln!("ok");
            }
            Err(_) => {
                self.failed.push(name.to_string());
                eprintln!("FAILED");
            }
        }
    }

    pub fn finish(self) {
        eprintln!(
            "\nresult: {}. {} passed; {} failed; {} filtered out",
            if self.failed.is_empty() { "ok" } else { "FAILED" },
            self.passed,
            self.failed.len(),
            self.skipped
        );
        if !self.failed.is_empty() {
            eprintln!("failures:");
            for f in &self.failed {
                eprintln!("    {f}");
            }
            std::process::exit(1);
        }
    }
}

/// Run `f` with fd 1 redirected to a temp file and return everything written.
///
/// Panic-safe: fd 1 is restored before any unwind propagates, so a failing
/// assertion inside `f` cannot leave the process with a hijacked stdout.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let path = cstring(&format!(
        "/tmp/.checkshift-capture-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    unsafe {
        // Flush BOTH buffering layers: libc's `stdout` FILE (used by the two
        // libraries) and Rust's own `std::io::stdout` BufWriter.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let tmp = open(path.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(tmp >= 0, "open(tmpfile) failed");
        assert!(dup2(tmp, 1) >= 0, "dup2 failed");

        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        lseek(tmp, 0, SEEK_SET);
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(tmp, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(tmp);
        unlink(path.as_ptr());
        match r {
            Ok(v) => (v, out),
            Err(p) => std::panic::resume_unwind(p),
        }
    }
}

/// Byte-exact stdout comparison with a readable diff on failure.
pub fn assert_same_output(c_out: &[u8], r_out: &[u8], ctx: &str) {
    if c_out == r_out {
        return;
    }
    let at = c_out.iter().zip(r_out.iter()).position(|(x, y)| x != y).unwrap_or(
        c_out.len().min(r_out.len()),
    );
    let lo = at.saturating_sub(160);
    panic!(
        "stdout diverges [{ctx}] at byte {at} (C len {}, Rust len {})\n  C: {}\n  R: {}",
        c_out.len(),
        r_out.len(),
        show(&c_out[lo..(at + 160).min(c_out.len())]),
        show(&r_out[lo..(at + 160).min(r_out.len())])
    );
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}
