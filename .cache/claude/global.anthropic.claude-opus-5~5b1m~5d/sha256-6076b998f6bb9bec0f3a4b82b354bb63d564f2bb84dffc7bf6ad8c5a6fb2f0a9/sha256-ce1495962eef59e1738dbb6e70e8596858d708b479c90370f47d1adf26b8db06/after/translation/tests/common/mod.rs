// Shared differential-test harness.
//
// BOTH implementations are loaded as shared objects through `libloading` and
// called only through their exported C symbols -- the Rust crate is never
// linked or called directly, so the `#[no_mangle] extern "C"` wrappers are
// themselves under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C types mirrored for the FFI boundary
// ---------------------------------------------------------------------------

/// `typedef struct { int value; time_t timestamp; StatusCode status; }`
/// Verified against the C ABI: size 24, align 8, offsets 0 / 8 / 16.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ComputationResult {
    pub value: i32,
    pub timestamp: i64,
    pub status: i32,
}

pub const OP_ADD: i32 = 1;
pub const OP_MULTIPLY: i32 = 2;
pub const OP_SUBTRACT: i32 = 3;
pub const OP_DIVIDE: i32 = 4;
pub const OP_MODULO: i32 = 5;

pub const HISTORY_CAPACITY: i32 = 10;

pub type FnIsValid = unsafe extern "C" fn(i8) -> u8;
pub type FnPriority = unsafe extern "C" fn(i32) -> i32;
pub type FnMath = unsafe extern "C" fn(i32, i32, i32) -> i32;
pub type FnSelect = unsafe extern "C" fn(i32) -> Option<FnMath>;
pub type FnTimestamp = unsafe extern "C" fn() -> i64;
pub type FnAlloc = unsafe extern "C" fn(i32) -> *mut ComputationResult;
pub type FnPcwh =
    unsafe extern "C" fn(i32, i32, i32, *mut *mut ComputationResult, *mut i32) -> i32;
pub type FnMathop = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

extern "C" {
    pub fn free(ptr: *mut c_void);
    fn fflush(stream: *mut c_void) -> i32;
    fn dup(fd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

// ---------------------------------------------------------------------------
// One loaded implementation
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub is_valid_operation: FnIsValid,
    pub get_operation_priority: FnPriority,
    pub add_operation: FnMath,
    pub multiply_operation: FnMath,
    pub subtract_operation: FnMath,
    pub divide_operation: FnMath,
    pub modulo_operation: FnMath,
    pub select_operation: FnSelect,
    pub get_computation_timestamp: FnTimestamp,
    pub allocate_results: FnAlloc,
    pub perform_computation_with_history: FnPcwh,
    pub mathop: FnMathop,
    // Kept last so the code above stays valid for the library's whole lifetime.
    _lib: Library,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib.get(name).unwrap_or_else(|e| {
        panic!(
            "symbol `{}` missing from shared object: {e}",
            String::from_utf8_lossy(name)
        )
    });
    *s
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("cannot load {} .so at {}: {e}", name, path.display()));
        unsafe {
            Impl {
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
                path,
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
            .collect(),
        Err(_) => Vec::new(),
    };
    found.sort();
    found
}

/// `c_src/build/lib<parent-dir-name>.so` -- the name is derived from the
/// enclosing directory by `CMakeLists.txt`, so glob for it instead of hardcoding.
///
/// If the C library has not been built yet, build it with the documented cmake
/// invocation so that a bare `cargo test` works from a clean checkout.
fn c_so_path() -> PathBuf {
    let c_src = manifest_dir().parent().unwrap().join("c_src");
    let build = c_src.join("build");

    if find_so(&build).is_empty() {
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let cfg = std::process::Command::new("cmake")
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .current_dir(&build)
            .status();
        let bld = std::process::Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .status();
        assert!(
            matches!(cfg, Ok(s) if s.success()) && matches!(bld, Ok(s) if s.success()),
            "could not build the C library; do it manually:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
    }

    let mut found = find_so(&build);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust cdylib, next to this test binary
/// (`target/<profile>/deps/<test>` -> `target/<profile>/libmathop_lib.so`).
///
/// `cargo test` alone does NOT emit the cdylib (integration tests do not depend
/// on it), so if it is absent we build it here. The bootstrap build uses its own
/// `CARGO_TARGET_DIR` so it can never contend for the lock held by the outer
/// `cargo test` invocation.
fn rust_so_path() -> PathBuf {
    const SO: &str = "libmathop_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let p = profile_dir.join(SO);
    if p.exists() {
        return p;
    }

    let release = profile_dir.file_name().map(|n| n == "release").unwrap_or(false);
    let boot = manifest_dir().join("target").join("so-bootstrap");
    let out = boot
        .join(if release { "release" } else { "debug" })
        .join(SO);

    let cargo = option_env!("CARGO").unwrap_or("cargo");
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build").arg("--offline").arg("--lib");
    if release {
        cmd.arg("--release");
    }
    let status = cmd
        .current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &boot)
        .env_remove("RUSTFLAGS")
        .status();

    assert!(
        matches!(status, Ok(s) if s.success()) && out.exists(),
        "could not build the Rust cdylib.\n\
         `cargo test` does not emit it on its own -- run `cargo build` first, \
         or use ci/verify_all.sh.\n\
         expected: {} or {}",
        p.display(),
        out.display()
    );
    out
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The two implementations, loaded once per test binary.
pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", c_so_path()),
        rust: Impl::load("Rust", rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Serialisation
//
// `mathop` keeps `static` state inside each .so, and stdout capture rewires
// fd 1 process-wide. Both must therefore be done under one global lock so the
// two implementations always observe the exact same call sequence.
// ---------------------------------------------------------------------------

static GLOBAL: Mutex<()> = Mutex::new(());

pub fn global_lock() -> MutexGuard<'static, ()> {
    GLOBAL.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*), fixed seed => reproducible failures
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new() -> Rng {
        Rng(Rng::SEED)
    }

    pub fn with_seed(seed: u64) -> Rng {
        Rng(if seed == 0 { Rng::SEED } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// A value biased towards the interesting corners of `i32`.
    pub fn interesting_i32(&mut self) -> i32 {
        const CORNERS: [i32; 14] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            -3,
            5,
            -5,
            10,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
        ];
        match self.below(4) {
            0 => CORNERS[self.below(CORNERS.len() as u64) as usize],
            1 => (self.next_u64() % 21) as i32 - 10, // tiny
            2 => (self.next_u64() % 2001) as i32 - 1000, // small
            _ => self.next_i32(),                    // full range
        }
    }

    pub fn interesting_op(&mut self) -> i32 {
        const OPS: [i32; 10] = [1, 2, 3, 4, 5, 0, 6, -1, i32::MIN, i32::MAX];
        OPS[self.below(OPS.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// C undefined behaviour that would kill the process
// ---------------------------------------------------------------------------

/// `INT_MIN / -1` and `INT_MIN % -1` are UB in C; on x86-64 the `idiv`
/// instruction traps and the process dies with SIGFPE, so the C function has no
/// return value to compare against. See ERRORS.md row 25.
pub fn is_c_div_trap(a: i32, b: i32) -> bool {
    a == i32::MIN && b == -1
}

// ---------------------------------------------------------------------------
// An independent model of the C, transcribed straight from c_src/src/lib.c.
// Used to check BOTH implementations against a third derivation, not just
// against each other.
// ---------------------------------------------------------------------------

/// The dispatch table of `select_operation` + the individual operations.
/// Anything outside 1..5 falls through to ADD, as the `default:` arm does.
pub fn apply_op(op: i32, a: i32, b: i32) -> i32 {
    match op {
        OP_MULTIPLY => a.wrapping_mul(b),
        OP_SUBTRACT => a.wrapping_sub(b),
        OP_DIVIDE => {
            if b == 0 {
                0
            } else {
                a.wrapping_div(b)
            }
        }
        OP_MODULO => {
            if b == 0 {
                0
            } else {
                a.wrapping_rem(b)
            }
        }
        _ => a.wrapping_add(b),
    }
}

/// `mathop`'s two operation selectors:
///   op1 = (param3 % 5) + 1
///   op2 = ((param4 + 1) % 5) + 1
pub fn mathop_ops(p3: i32, p4: i32) -> (i32, i32) {
    (
        p3.wrapping_rem(5).wrapping_add(1),
        p4.wrapping_add(1).wrapping_rem(5).wrapping_add(1),
    )
}

/// `mathop`'s return value, given the timestamp it will observe.
pub fn mathop_expected(p1: i32, p2: i32, p3: i32, p4: i32, timestamp: i64) -> i32 {
    let (op1, op2) = mathop_ops(p3, p4);
    let intermediate = apply_op(op1, p1, p2);
    apply_op(op2, intermediate, p4)
        .wrapping_add(op1.wrapping_mul(10)) // get_operation_priority(op1)
        .wrapping_add((timestamp % 100) as i32) // time_modifier
}

/// Does `mathop(p1,p2,p3,p4)` reach a trapping `idiv` inside the C code?
/// DIVIDE is 4 and MODULO is 5; both compile to `idiv`.
pub fn mathop_would_trap(p1: i32, p2: i32, p3: i32, p4: i32) -> bool {
    let (op1, op2) = mathop_ops(p3, p4);
    let uses_idiv = |op: i32| op == OP_DIVIDE || op == OP_MODULO;

    if uses_idiv(op1) && is_c_div_trap(p1, p2) {
        return true;
    }
    let intermediate = apply_op(op1, p1, p2);
    uses_idiv(op2) && is_c_div_trap(intermediate, p4)
}

// ---------------------------------------------------------------------------
// stdout capture (mathop printf output)
// ---------------------------------------------------------------------------

/// Run `f` with fd 1 redirected into a temp file and return everything the
/// C library printed. Must be called with `global_lock()` held.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    unsafe {
        fflush(std::ptr::null_mut()); // flush all streams before rewiring fd 1
    }
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "mathop_stdout_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 failed");

    let out = f();

    unsafe {
        fflush(std::ptr::null_mut()); // force the libc buffer out to the file
        dup2(saved, 1);
        close(saved);
    }
    drop(file);
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (out, bytes)
}

// ---------------------------------------------------------------------------
// fork isolation for inputs that legitimately kill the process
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signalled(i32),
}

/// Run `f` in a forked child and report how the child terminated. The child
/// performs the call and nothing else (no allocation, no unwinding) before
/// `_exit`, which keeps it safe in a multi-threaded parent.
pub fn run_isolated(f: impl FnOnce()) -> Outcome {
    unsafe {
        fflush(std::ptr::null_mut());
    }
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: i32 = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    let termsig = status & 0x7f;
    if termsig != 0 {
        Outcome::Signalled(termsig)
    } else {
        Outcome::Exited((status >> 8) & 0xff)
    }
}

// ---------------------------------------------------------------------------
// Small assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_same<T: PartialEq + std::fmt::Debug>(c: T, rust: T, ctx: &str) {
    assert_eq!(c, rust, "C/Rust divergence for {ctx}");
}

/// Read `count` records out of a raw buffer.
pub unsafe fn slots(p: *const ComputationResult, count: usize) -> Vec<ComputationResult> {
    (0..count).map(|i| *p.add(i)).collect()
}

/// Raw bytes of a record buffer, for byte-for-byte comparison including padding
/// holes (which `calloc` must have zeroed).
pub unsafe fn raw_bytes(p: *const ComputationResult, count: usize) -> Vec<u8> {
    std::slice::from_raw_parts(p as *const u8, count * std::mem::size_of::<ComputationResult>())
        .to_vec()
}

/// Compare two history buffers: first that they are both allocated or both not,
/// then their raw bytes. Never dereferences a NULL pointer.
#[track_caller]
pub unsafe fn assert_buffers_match(
    c: *const ComputationResult,
    rust: *const ComputationResult,
    count: usize,
    ctx: &str,
) {
    assert_eq!(
        c.is_null(),
        rust.is_null(),
        "{ctx}: allocation state differs (C {c:?} / Rust {rust:?})"
    );
    if !c.is_null() {
        assert_eq!(
            raw_bytes(c, count),
            raw_bytes(rust, count),
            "{ctx}: history buffer bytes differ"
        );
    }
}
