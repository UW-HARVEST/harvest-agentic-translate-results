// Shared differential-test harness.
//
// Loads BOTH shared objects with `libloading` and calls every entry point
// through its exported C symbol — the Rust functions are never called
// directly, so the `#[no_mangle]` / `extern "C"` wrappers are under test too.
//
// Both implementations log to the process-wide `stdout` (the C compiler lowers
// `printf("literal\n")` to `puts("literal")`, and so does LLVM for the Rust
// side), so the harness also taps file descriptor 1 around every call in order
// to compare the emitted log bytes, not just the return value.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// FFI signatures of the exported symbols
// ---------------------------------------------------------------------------

/// `int gotomach(int a, int b, int c, int d);`
pub type GotomachFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
/// `int (*operation_fn)(int value, int unused_param, void *unused_context);`
pub type OpFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

// ---------------------------------------------------------------------------
// libc bits used for the stdout tap
// ---------------------------------------------------------------------------

const SEEK_SET: c_int = 0;
const SEEK_CUR: c_int = 1;

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn pread(fd: c_int, buf: *mut c_void, count: usize, offset: i64) -> isize;
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The C `.so`. Its file name is derived by CMake from the *parent directory*
/// name (`cmake_path(GET parent FILENAME project_name)`), so it is discovered
/// by globbing rather than hard-coded. Override with `C_SO=/path/to/lib.so`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src").join("build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    cands.sort();
    cands
        .pop()
        .unwrap_or_else(|| panic!("no *.so found in {}", dir.display()))
}

/// The Rust `cdylib`. Prefers `target/release` (the shipped artifact) and falls
/// back to `target/debug`. Override with `RUST_SO=/path/to/lib.so`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libgotomach_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libgotomach_lib.so not found under {}; run `cargo build --release` first",
        base.display()
    );
}

// ---------------------------------------------------------------------------
// One loaded implementation
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub gotomach: GotomachFn,
    pub process_value: OpFn,
    pub double_value: OpFn,
    pub triple_value: OpFn,
    // Kept last so the library outlives the function pointers above.
    _lib: Library,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8], from: &PathBuf) -> T {
    let s: Symbol<T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol `{}` missing from {}: {e}",
            String::from_utf8_lossy(&name[..name.len() - 1]),
            from.display()
        )
    });
    *s
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        unsafe {
            Impl {
                name,
                gotomach: sym(&lib, b"gotomach\0", &path),
                process_value: sym(&lib, b"process_value\0", &path),
                double_value: sym(&lib, b"double_value\0", &path),
                triple_value: sym(&lib, b"triple_value\0", &path),
                path,
                _lib: lib,
            }
        }
    }

    pub fn op(&self, which: Op) -> OpFn {
        match which {
            Op::Process => self.process_value,
            Op::Double => self.double_value,
            Op::Triple => self.triple_value,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Process,
    Double,
    Triple,
}

impl Op {
    pub const ALL: [Op; 3] = [Op::Process, Op::Double, Op::Triple];
    pub fn sym_name(self) -> &'static str {
        match self {
            Op::Process => "process_value",
            Op::Double => "double_value",
            Op::Triple => "triple_value",
        }
    }
    /// The `mode` value that selects this operation in `gotomach`.
    pub fn mode(self) -> c_int {
        match self {
            Op::Process => 0,
            Op::Double => 1,
            Op::Triple => 2,
        }
    }
}

// ---------------------------------------------------------------------------
// stdout tap
// ---------------------------------------------------------------------------

/// Owns a scratch file that fd 1 is temporarily pointed at.
///
/// The redirection is installed *per call* and torn down immediately, so the
/// test harness's own output and any panic message stay visible.
struct Tap {
    write: File,
    read: File,
}

impl Tap {
    fn new() -> Tap {
        let path = std::env::temp_dir().join(format!(
            "c2rust_diff_stdout_{}_{:p}.log",
            std::process::id(),
            &0u8 as *const u8
        ));
        let write = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap_or_else(|e| panic!("cannot create tap file {}: {e}", path.display()));
        let read = File::open(&path)
            .unwrap_or_else(|e| panic!("cannot reopen tap file {}: {e}", path.display()));
        // Unlink now; the two descriptors keep the inode alive until drop.
        let _ = std::fs::remove_file(&path);
        Tap { write, read }
    }

    /// Runs `f` with fd 1 redirected into the scratch file, returning `f`'s
    /// value plus the bytes it wrote (when `collect` is set).
    fn capture<R>(&mut self, collect: bool, f: impl FnOnce() -> R) -> (R, Vec<u8>) {
        // Push anything Rust has buffered (libtest's own "test foo ... "
        // progress text lives in a `LineWriter`) out to the *real* stdout
        // before we steal fd 1, otherwise it would land in the tap file.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        unsafe {
            // Same for anything buffered in C stdio.
            fflush(ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(self.write.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
            lseek(1, 0, SEEK_SET);

            let r = f();

            fflush(ptr::null_mut());
            let end = lseek(1, 0, SEEK_CUR);

            assert!(dup2(saved, 1) >= 0, "dup2 restore of fd 1 failed");
            close(saved);

            let mut buf = Vec::new();
            if collect && end > 0 {
                buf.resize(end as usize, 0u8);
                let n = pread(
                    self.read.as_raw_fd(),
                    buf.as_mut_ptr() as *mut c_void,
                    end as usize,
                    0,
                );
                assert_eq!(n, end as isize, "pread of tapped stdout came up short");
            }
            (r, buf)
        }
    }
}

// ---------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------

pub struct Harness {
    pub c: Impl,
    pub r: Impl,
    tap: Tap,
    /// `Some(saved_fd1)` while a bulk-sweep silence window is open.
    silenced: Option<c_int>,
}

static HARNESS: OnceLock<Mutex<Harness>> = OnceLock::new();

/// Grabs the process-wide harness. The lock serialises tests because the
/// stdout tap manipulates the process-wide fd 1.
pub fn harness() -> MutexGuard<'static, Harness> {
    let m = HARNESS.get_or_init(|| {
        Mutex::new(Harness {
            c: Impl::load("C", c_so_path()),
            r: Impl::load("Rust", rust_so_path()),
            tap: Tap::new(),
            silenced: None,
        })
    });
    // Recover rather than cascade-fail if an earlier test panicked.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Argument tuple for `gotomach`, kept around for error messages.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub iterations: c_int,
    pub seed: c_int,
    pub mode: c_int,
    pub threshold: c_int,
}

impl std::fmt::Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "gotomach(iterations={}, seed={}, mode={}, threshold={})",
            self.iterations, self.seed, self.mode, self.threshold
        )
    }
}

pub fn args(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> Args {
    Args {
        iterations,
        seed,
        mode,
        threshold,
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}

impl Harness {
    // -- gotomach ---------------------------------------------------------

    /// Calls `gotomach` in both `.so`s and asserts the **return value** and the
    /// **stdout log bytes** are identical. Returns the (shared) result.
    pub fn assert_gotomach(&mut self, a: Args) -> c_int {
        let cf = self.c.gotomach;
        let rf = self.r.gotomach;
        let t = &mut self.tap;
        let (rc, oc) = t.capture(true, || unsafe {
            cf(a.iterations, a.seed, a.mode, a.threshold)
        });
        let (rr, or) = t.capture(true, || unsafe {
            rf(a.iterations, a.seed, a.mode, a.threshold)
        });
        assert_eq!(
            rc, rr,
            "return value diverged for {a}\n  C    -> {rc}\n  Rust -> {rr}"
        );
        assert_eq!(
            oc,
            or,
            "stdout diverged for {a}\n  C    -> \"{}\"\n  Rust -> \"{}\"",
            show(&oc),
            show(&or)
        );
        rc
    }

    /// Cheaper variant: compares only the return value, but still swallows the
    /// libraries' log output so bulk sweeps do not flood the terminal.
    pub fn assert_gotomach_ret(&mut self, a: Args) -> c_int {
        let cf = self.c.gotomach;
        let rf = self.r.gotomach;
        let t = &mut self.tap;
        let (rc, _) = t.capture(false, || unsafe {
            cf(a.iterations, a.seed, a.mode, a.threshold)
        });
        let (rr, _) = t.capture(false, || unsafe {
            rf(a.iterations, a.seed, a.mode, a.threshold)
        });
        assert_eq!(
            rc, rr,
            "return value diverged for {a}\n  C    -> {rc}\n  Rust -> {rr}"
        );
        rc
    }

    /// Like `assert_gotomach` but also hands back the captured C log so a test
    /// can additionally assert *which* branch was taken.
    pub fn assert_gotomach_logged(&mut self, a: Args) -> (c_int, String) {
        let cf = self.c.gotomach;
        let rf = self.r.gotomach;
        let t = &mut self.tap;
        let (rc, oc) = t.capture(true, || unsafe {
            cf(a.iterations, a.seed, a.mode, a.threshold)
        });
        let (rr, or) = t.capture(true, || unsafe {
            rf(a.iterations, a.seed, a.mode, a.threshold)
        });
        assert_eq!(
            rc, rr,
            "return value diverged for {a}\n  C    -> {rc}\n  Rust -> {rr}"
        );
        assert_eq!(
            oc,
            or,
            "stdout diverged for {a}\n  C    -> \"{}\"\n  Rust -> \"{}\"",
            show(&oc),
            show(&or)
        );
        (rc, String::from_utf8_lossy(&oc).into_owned())
    }

    // -- bulk sweep mode --------------------------------------------------
    //
    // `assert_gotomach*` installs and tears down the fd-1 redirection around
    // every single call, which costs ~8 syscalls per call. For exhaustive
    // sweeps of millions of inputs that dominates the runtime, so
    // `sweep()` installs the redirection once for the whole loop and
    // `assert_gotomach_sweep()` only rewinds the scratch file (1 syscall).
    // Log bytes are not compared in this mode — the dedicated rows in
    // `CONFIGS.md` do that.

    fn silence_begin(&mut self) {
        assert!(self.silenced.is_none(), "silence window already open");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        unsafe {
            fflush(ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(self.tap.write.as_raw_fd(), 1) >= 0, "dup2 failed");
            lseek(1, 0, SEEK_SET);
            self.silenced = Some(saved);
        }
    }

    fn silence_end(&mut self) {
        if let Some(saved) = self.silenced.take() {
            unsafe {
                fflush(ptr::null_mut());
                assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
                close(saved);
            }
        }
    }

    /// Compares only the return value; requires an open `sweep()` window.
    pub fn assert_gotomach_sweep(&mut self, a: Args) -> c_int {
        debug_assert!(self.silenced.is_some(), "call inside `sweep()`");
        // Rewind the scratch file so the discarded log output cannot grow it.
        unsafe {
            lseek(1, 0, SEEK_SET);
        }
        let rc = unsafe { (self.c.gotomach)(a.iterations, a.seed, a.mode, a.threshold) };
        let rr = unsafe { (self.r.gotomach)(a.iterations, a.seed, a.mode, a.threshold) };
        if rc != rr {
            // Restore fd 1 before panicking so the message is readable.
            self.silence_end();
            panic!("return value diverged for {a}\n  C    -> {rc}\n  Rust -> {rr}");
        }
        rc
    }

    // -- the three operation_fn exports -----------------------------------

    /// Calls one of `process_value` / `double_value` / `triple_value` in both
    /// `.so`s and asserts the results match. These never log, so no tap.
    pub fn assert_op(&mut self, which: Op, value: c_int, unused: c_int, ctx: *mut c_void) -> c_int {
        let cf = self.c.op(which);
        let rf = self.r.op(which);
        let rc = unsafe { cf(value, unused, ctx) };
        let rr = unsafe { rf(value, unused, ctx) };
        assert_eq!(
            rc,
            rr,
            "{}({value}, {unused}, {ctx:p}) diverged\n  C    -> {rc}\n  Rust -> {rr}",
            which.sym_name()
        );
        rc
    }
}

/// Runs `f` with fd 1 redirected into the scratch file for its whole duration,
/// enabling `Harness::assert_gotomach_sweep`. Restores fd 1 even if `f` panics.
pub fn sweep<R>(h: &mut Harness, f: impl FnOnce(&mut Harness) -> R) -> R {
    h.silence_begin();
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(h)));
    h.silence_end();
    match out {
        Ok(v) => v,
        Err(e) => std::panic::resume_unwind(e),
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed everywhere for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1EAF_C0FF_EE01;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform over the whole `i32` range, including both extremes.
    pub fn i32_any(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Uniform inclusive range.
    pub fn range(&mut self, lo: c_int, hi: c_int) -> c_int {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as c_int
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A `mode` value that is guaranteed **not** to be 0, 1 or 2, i.e. one that
    /// lands in the `switch`'s `default:` arm.
    pub fn bad_mode(&mut self) -> c_int {
        loop {
            let m = self.i32_any();
            if m != 0 && m != 1 && m != 2 {
                return m;
            }
        }
    }
    /// Mixes uniformly-random `i32`s with boundary-ish values, which is what
    /// actually shakes out comparison and overflow bugs.
    pub fn i32_interesting(&mut self) -> c_int {
        const EDGES: [c_int; 24] = [
            c_int::MIN,
            c_int::MIN + 1,
            -2_000_000_000,
            -65_537,
            -65_536,
            -65_535,
            -1000,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            999,
            1000,
            1005,
            1998,
            2997,
            65_535,
            65_536,
            196_605,
            c_int::MAX - 1,
            c_int::MAX,
        ];
        if self.next_u64() % 3 == 0 {
            self.pick(&EDGES)
        } else {
            self.i32_any()
        }
    }
}

// ---------------------------------------------------------------------------
// Independent oracle: the C algorithm, re-implemented from `lib.c` and driven
// through the *C* `.so`'s operation functions.
// ---------------------------------------------------------------------------

pub const UINT16_MAX: c_int = 65535;

pub fn oracle(a: Args, op_for_mode: impl Fn(c_int) -> OpFn) -> c_int {
    // lib.c:114
    if a.iterations < 0 || a.iterations > UINT16_MAX {
        return -1;
    }
    // lib.c:120
    if a.seed < 0 || a.seed > UINT16_MAX {
        return -2;
    }
    let op = op_for_mode(a.mode); // lib.c:126-140
    let capacity = a.iterations as usize; // lib.c:142
    let mut results: Vec<c_int> = Vec::with_capacity(capacity);

    let mut current_value = a.seed; // lib.c:162
    let mut i: c_int = 0;
    while i < a.iterations {
        // is_valid_state: status(=1) != 0 && count < capacity   (lib.c:48-53)
        if !(results.len() < capacity) {
            return -6;
        }
        let v = unsafe { op(current_value, 0, ptr::null_mut()) };
        if v < a.threshold {
            results.push(v);
        }
        current_value = v % 1000;
        if results.len() >= UINT16_MAX as usize {
            break;
        }
        i += 1;
    }
    let mut result: c_int = 0;
    for v in &results {
        result = result.wrapping_add(*v);
    }
    result
}
