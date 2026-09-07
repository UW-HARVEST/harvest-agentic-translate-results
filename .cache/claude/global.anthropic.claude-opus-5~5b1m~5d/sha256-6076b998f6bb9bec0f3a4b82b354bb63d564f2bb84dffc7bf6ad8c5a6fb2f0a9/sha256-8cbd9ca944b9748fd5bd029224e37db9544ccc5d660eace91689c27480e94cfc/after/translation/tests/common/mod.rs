//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes a symmetric wrapper over the 12 exported symbols.
//!
//! Rust functions are NEVER called directly — everything goes through
//! `dlsym` on `libmathop_lib.so`, exactly as an external C consumer would,
//! so the `#[no_mangle]` export wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_long};
use std::path::PathBuf;

pub type TimeT = c_long;

/// `typedef struct { int value; time_t timestamp; StatusCode status; }`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: TimeT,
    pub status: c_int,
}

pub type MathOperation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    // The test binary lives at target/<profile>/deps/<name>-<hash>, and the
    // cdylib it was built alongside is at target/<profile>/libmathop_lib.so.
    // Deriving the path this way guarantees we never load a STALE .so from a
    // different profile (e.g. testing debug against a release artifact).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            let p = profile_dir.join("libmathop_lib.so");
            if p.exists() {
                return p;
            }
        }
    }
    let root = workspace_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libmathop_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libmathop_lib.so not found under {}. Build it: cargo build --release",
        root.display()
    )
}

// ---------------------------------------------------------------------------
// One loaded library, with typed accessors for every exported symbol
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    lib: Library,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", path.display(), name));
        Lib { name, lib }
    }

    fn sym<T>(&self, s: &str) -> Symbol<'_, T> {
        let mut owned = s.as_bytes().to_vec();
        owned.push(0);
        unsafe { self.lib.get::<T>(&owned) }
            .unwrap_or_else(|e| panic!("dlsym `{s}` missing from the {} .so: {e}", self.name))
    }

    // -- the 12 exported symbols ------------------------------------------

    pub fn is_valid_operation(&self, op_char: c_char) -> bool {
        let f: Symbol<unsafe extern "C" fn(c_char) -> bool> = self.sym("is_valid_operation");
        unsafe { f(op_char) }
    }

    /// Same symbol, but read back as a raw `u8` so we can also compare the
    /// exact byte the ABI puts in `al` (C `bool` return is a single byte).
    pub fn is_valid_operation_raw(&self, op_char: c_char) -> u8 {
        let f: Symbol<unsafe extern "C" fn(c_char) -> u8> = self.sym("is_valid_operation");
        unsafe { f(op_char) }
    }

    pub fn get_operation_priority(&self, op: c_int) -> c_int {
        let f: Symbol<unsafe extern "C" fn(c_int) -> c_int> = self.sym("get_operation_priority");
        unsafe { f(op) }
    }

    pub fn add_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        let f: Symbol<MathOperation> = self.sym("add_operation");
        unsafe { f(a, b, u) }
    }

    pub fn multiply_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        let f: Symbol<MathOperation> = self.sym("multiply_operation");
        unsafe { f(a, b, u) }
    }

    pub fn subtract_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        let f: Symbol<MathOperation> = self.sym("subtract_operation");
        unsafe { f(a, b, u) }
    }

    pub fn divide_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        let f: Symbol<MathOperation> = self.sym("divide_operation");
        unsafe { f(a, b, u) }
    }

    pub fn modulo_operation(&self, a: c_int, b: c_int, u: c_int) -> c_int {
        let f: Symbol<MathOperation> = self.sym("modulo_operation");
        unsafe { f(a, b, u) }
    }

    /// Returns the raw pointer AND a callable, so tests can both compare
    /// behaviour and check pointer identity *within* one `.so`.
    pub fn select_operation(&self, op: c_int) -> MathOperation {
        let f: Symbol<unsafe extern "C" fn(c_int) -> MathOperation> = self.sym("select_operation");
        unsafe { f(op) }
    }

    /// Address of one of this `.so`'s own operation symbols, for identity
    /// comparison against `select_operation`'s return value.
    pub fn op_addr(&self, name: &str) -> usize {
        let f: Symbol<MathOperation> = self.sym(name);
        (unsafe { f.into_raw() }.into_raw()) as usize
    }

    pub fn get_computation_timestamp(&self) -> TimeT {
        let f: Symbol<unsafe extern "C" fn() -> TimeT> = self.sym("get_computation_timestamp");
        unsafe { f() }
    }

    pub fn allocate_results(&self, count: c_int) -> *mut ComputationResult {
        let f: Symbol<unsafe extern "C" fn(c_int) -> *mut ComputationResult> =
            self.sym("allocate_results");
        unsafe { f(count) }
    }

    pub unsafe fn perform_computation_with_history(
        &self,
        a: c_int,
        b: c_int,
        op: c_int,
        history: *mut *mut ComputationResult,
        history_count: *mut c_int,
    ) -> c_int {
        let f: Symbol<
            unsafe extern "C" fn(
                c_int,
                c_int,
                c_int,
                *mut *mut ComputationResult,
                *mut c_int,
            ) -> c_int,
        > = self.sym("perform_computation_with_history");
        f(a, b, op, history, history_count)
    }

    pub fn mathop(&self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            self.sym("mathop");
        unsafe { f(p1, p2, p3, p4) }
    }

    /// `mathop`, capturing everything it `printf`s to stdout.
    pub fn mathop_capture(&self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> (c_int, String) {
        let cap = StdoutCapture::begin();
        let r = self.mathop(p1, p2, p3, p4);
        let out = cap.finish();
        (r, out)
    }
}

/// The two libraries under differential test.
pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn pair() -> Pair {
    let rs_path = rust_so_path();
    assert_not_stale(&rs_path);
    Pair {
        c: Lib::open("C", c_so_path()),
        rs: Lib::open("Rust", rs_path),
    }
}

/// `cargo test` builds the test harness but does NOT necessarily re-emit the
/// `cdylib`, so `target/<profile>/libmathop_lib.so` can silently lag behind
/// `src/lib.rs`. Testing a stale `.so` would give false confidence, so refuse
/// to run instead.
fn assert_not_stale(so: &PathBuf) {
    let src = workspace_root().join("translation").join("src").join("lib.rs");
    let (Ok(so_m), Ok(src_m)) = (
        std::fs::metadata(so).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) else {
        return;
    };
    assert!(
        so_m >= src_m,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not re-emit the cdylib. Run `cargo build` (same \
         profile) first, e.g.:\n  cargo build --release && cargo test --release",
        so.display(),
        src.display()
    );
}

// ---------------------------------------------------------------------------
// stdout capture (fd-level, because the output comes from C `printf`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// fd 1 is process-global, so only one capture may be active at a time even
/// when the test harness runs tests in parallel.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct StdoutCapture {
    saved: c_int,
    file: std::fs::File,
    path: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl StdoutCapture {
    pub fn begin() -> StdoutCapture {
        let _guard = CAPTURE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe { fflush(std::ptr::null_mut()) };
        let mut path = std::env::temp_dir();
        path.push(format!(
            "mathop_cap_{}_{:?}_{}.txt",
            std::process::id(),
            std::thread::current().id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let file = std::fs::File::create(&path).expect("create capture file");
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        use std::os::unix::io::AsRawFd;
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
        StdoutCapture { saved, file, path, _guard }
    }

    pub fn finish(self) -> String {
        unsafe { fflush(std::ptr::null_mut()) };
        unsafe {
            dup2(self.saved, 1);
            close(self.saved);
        }
        drop(self.file);
        let s = std::fs::read_to_string(&self.path).unwrap_or_default();
        let _ = std::fs::remove_file(&self.path);
        s
    }
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style testing)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Biased towards small magnitudes and boundary values, which is where the
    /// interesting branches live, while still covering the full range.
    pub fn i32_interesting(&mut self) -> i32 {
        let r = self.next_u64();
        match r % 8 {
            0 => (r >> 8) as i32 % 11 - 5,
            1 => (r >> 8) as i32 % 256 - 128,
            2 => i32::MAX,
            3 => i32::MIN,
            4 => -((r >> 8) as u32 as i32).wrapping_abs(),
            _ => r as u32 as i32,
        }
    }
    pub fn nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.i32();
            if v != 0 {
                return v;
            }
        }
    }
}

/// The `INT_MIN / -1` pair is UB in C and traps (`SIGFPE`) on x86-64;
/// see ERRORS.md row 20. Skip it in randomized divide/modulo sweeps.
pub fn is_div_trap(a: i32, b: i32) -> bool {
    b == 0 || (a == i32::MIN && b == -1)
}

// ---------------------------------------------------------------------------
// Independent reference model of the C, used only to predict which inputs make
// the C library die with SIGFPE so the in-process sweeps can route those to the
// fork-based trap test instead of killing the harness.
// ---------------------------------------------------------------------------

/// `select_operation(op)` applied to `(a, b)`. `None` means "x86 `idiv` traps".
pub fn c_apply(op: i32, a: i32, b: i32) -> Option<i32> {
    match op {
        2 => Some(a.wrapping_mul(b)),
        3 => Some(a.wrapping_sub(b)),
        4 => {
            if b == 0 {
                Some(0)
            } else if a == i32::MIN && b == -1 {
                None
            } else {
                Some(a.wrapping_div(b))
            }
        }
        5 => {
            if b == 0 {
                Some(0)
            } else if a == i32::MIN && b == -1 {
                None
            } else {
                Some(a.wrapping_rem(b))
            }
        }
        // case OP_ADD and the `default:` fall-through
        _ => Some(a.wrapping_add(b)),
    }
}

pub fn mathop_op1(p3: i32) -> i32 {
    p3.wrapping_rem(5).wrapping_add(1)
}

pub fn mathop_op2(p4: i32) -> i32 {
    p4.wrapping_add(1).wrapping_rem(5).wrapping_add(1)
}

/// True when `mathop(p1, p2, p3, p4)` reaches `INT_MIN / -1` and dies.
pub fn mathop_traps(p1: i32, p2: i32, p3: i32, p4: i32) -> bool {
    let mid = match c_apply(mathop_op1(p3), p1, p2) {
        Some(v) => v,
        None => return true,
    };
    c_apply(mathop_op2(p4), mid, p4).is_none()
}

/// Full reference model of `mathop`'s return value (excluding the timestamp
/// term, which the caller adds), used as a third opinion alongside C and Rust.
pub fn mathop_model(p1: i32, p2: i32, p3: i32, p4: i32, timestamp: TimeT) -> Option<i32> {
    let op1 = mathop_op1(p3);
    let mid = c_apply(op1, p1, p2)?;
    let fin = c_apply(mathop_op2(p4), mid, p4)?;
    Some(
        fin.wrapping_add(op1.wrapping_mul(10))
            .wrapping_add((timestamp % 100) as i32),
    )
}
