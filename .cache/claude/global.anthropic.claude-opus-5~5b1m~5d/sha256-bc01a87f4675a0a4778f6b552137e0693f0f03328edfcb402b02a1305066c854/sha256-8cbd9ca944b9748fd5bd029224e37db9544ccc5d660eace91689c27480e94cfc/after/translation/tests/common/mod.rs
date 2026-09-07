//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C one built by CMake and the Rust cdylib)
//! with `libloading` and calls every entry point through `dlsym`, so the
//! `#[no_mangle] extern "C"` export wrappers are exercised exactly as an
//! external consumer would exercise them.  No Rust function is ever called
//! directly.
#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C ABI types mirrored from c_src/src/lib.c
// ---------------------------------------------------------------------------

/// `typedef int (*operation_func)(int, int);`
pub type OpFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type OpFnOpt = Option<OpFn>;

/// `typedef struct { int accumulator; int operation_count; unsigned int checksum; } ComputeState;`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct LibHandle {
    pub lib: libloading::Library,
    pub which: &'static str,
}
unsafe impl Send for LibHandle {}
unsafe impl Sync for LibHandle {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so` name is derived from the *parent* directory name by
/// `CMakeLists.txt`, so glob for it instead of hard-coding.
fn find_c_so() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C lib first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    cands.sort();
    assert!(!cands.is_empty(), "no lib*.so found in {}", build.display());
    cands.remove(0)
}

fn find_rust_so() -> PathBuf {
    let base = manifest_dir().join("target");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libcheckshift_lib.so");
        if let Ok(md) = std::fs::metadata(&p) {
            let t = md.modified().unwrap();
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, p));
            }
        }
    }
    best.map(|(_, p)| p).expect(
        "libcheckshift_lib.so not found under target/{release,debug}; run `cargo build --release`",
    )
}

pub fn c_so_path() -> PathBuf {
    find_c_so()
}

pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

static C_LIB: OnceLock<LibHandle> = OnceLock::new();
static R_LIB: OnceLock<LibHandle> = OnceLock::new();

pub fn clib() -> &'static LibHandle {
    C_LIB.get_or_init(|| {
        let p = find_c_so();
        LibHandle {
            lib: unsafe { libloading::Library::new(&p) }
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", p.display())),
            which: "C",
        }
    })
}

pub fn rlib() -> &'static LibHandle {
    R_LIB.get_or_init(|| {
        let p = find_rust_so();
        LibHandle {
            lib: unsafe { libloading::Library::new(&p) }
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", p.display())),
            which: "RUST",
        }
    })
}

/// Both libraries, in (C, Rust) order.
pub fn both() -> (&'static LibHandle, &'static LibHandle) {
    (clib(), rlib())
}

impl LibHandle {
    fn sym<T>(&self, name: &[u8]) -> libloading::Symbol<'_, T> {
        unsafe { self.lib.get(name) }.unwrap_or_else(|e| {
            panic!(
                "symbol {:?} missing from {} lib: {e}",
                String::from_utf8_lossy(name),
                self.which
            )
        })
    }

    // --- the four low-level operation functions -----------------------------
    pub fn multiply_with_static(&self, a: c_int, b: c_int) -> c_int {
        let f: libloading::Symbol<OpFn> = self.sym(b"multiply_with_static\0");
        unsafe { f(a, b) }
    }
    pub fn add_with_static(&self, a: c_int, b: c_int) -> c_int {
        let f: libloading::Symbol<OpFn> = self.sym(b"add_with_static\0");
        unsafe { f(a, b) }
    }
    pub fn xor_operation(&self, a: c_int, b: c_int) -> c_int {
        let f: libloading::Symbol<OpFn> = self.sym(b"xor_operation\0");
        unsafe { f(a, b) }
    }
    pub fn shift_with_static(&self, a: c_int, b: c_int) -> c_int {
        let f: libloading::Symbol<OpFn> = self.sym(b"shift_with_static\0");
        unsafe { f(a, b) }
    }

    /// Address of one of the exported op functions (for pointer-identity checks).
    pub fn op_addr(&self, name: &[u8]) -> *mut c_void {
        let f: libloading::Symbol<OpFn> = self.sym(name);
        unsafe { *(&*f as *const OpFn as *const *mut c_void) }
    }

    /// `operation_func get_operation(int opcode)` returned as a raw pointer so
    /// that NULL-ness and identity can be inspected.
    pub fn get_operation_raw(&self, opcode: c_int) -> *mut c_void {
        let f: libloading::Symbol<unsafe extern "C" fn(c_int) -> *mut c_void> =
            self.sym(b"get_operation\0");
        unsafe { f(opcode) }
    }

    pub fn get_operation(&self, opcode: c_int) -> OpFnOpt {
        let p = self.get_operation_raw(opcode);
        if p.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute::<*mut c_void, OpFn>(p) })
        }
    }

    /// `int execute_operation(operation_func, int, int, const char*)`
    pub fn execute_operation(
        &self,
        func: OpFnOpt,
        a: c_int,
        b: c_int,
        name: *const c_char,
    ) -> c_int {
        let f: libloading::Symbol<
            unsafe extern "C" fn(OpFnOpt, c_int, c_int, *const c_char) -> c_int,
        > = self.sym(b"execute_operation\0");
        unsafe { f(func, a, b, name) }
    }

    /// `unsigned int compute_checksum(int* values, int count)`
    pub fn compute_checksum(&self, values: *mut c_int, count: c_int) -> c_uint {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut c_int, c_int) -> c_uint> =
            self.sym(b"compute_checksum\0");
        unsafe { f(values, count) }
    }

    /// `void init_state(ComputeState*, int)`
    pub fn init_state(&self, state: *mut ComputeState, initial: c_int) {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut ComputeState, c_int)> =
            self.sym(b"init_state\0");
        unsafe { f(state, initial) }
    }

    /// `void apply_operation(ComputeState*, int, operation_func)`
    pub fn apply_operation(&self, state: *mut ComputeState, value: c_int, func: OpFnOpt) {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut ComputeState, c_int, OpFnOpt)> =
            self.sym(b"apply_operation\0");
        unsafe { f(state, value, func) }
    }

    /// `int checkshift(int, int, int, int)`
    pub fn checkshift(&self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
        let f: libloading::Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            self.sym(b"checkshift\0");
        unsafe { f(p1, p2, p3, p4) }
    }
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both libraries print through the *same* libc `stdout` of the test process, so
// fd 1 is temporarily redirected to a scratch file around each call and all
// streams are flushed with `fflush(NULL)` (glibc: flush every open stream).
// A process-wide mutex serialises this; nesting is not allowed.
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

pub fn capture_lock() -> MutexGuard<'static, ()> {
    require_single_threaded();
    CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// fd 1 is a *process-wide* resource, so the test binary must not run tests
/// concurrently: another thread's libtest progress output would land inside a
/// capture.  Enforce it instead of silently producing bogus diffs.
fn require_single_threaded() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let ok = std::env::var("RUST_TEST_THREADS").map(|v| v == "1").unwrap_or(false);
        assert!(
            ok,
            "these differential tests redirect fd 1 and must run serially; \
             set RUST_TEST_THREADS=1 (see ./run_tests.sh)"
        );
    });
}

/// Run `f` with fd 1 redirected; return `(result, captured_bytes)`.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _g = capture_lock();
    capture_locked(f)
}

/// Same as [`capture`] but assumes the caller already holds the capture lock.
pub fn capture_locked<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom, Write};

    let mut path = std::env::temp_dir();
    path.push(format!(
        "ctors_capture_{}_{:p}.txt",
        std::process::id(),
        &path as *const _
    ));

    // Drain *both* buffering layers before stealing fd 1, otherwise libtest's
    // own buffered progress output ("test foo ... ok") would be flushed into
    // the capture file and be mistaken for library output.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("cannot create capture file");

    let fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { libc::dup2(fd, 1) } >= 0, "dup2 failed");

    let result = f();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
    }

    let mut file = file;
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    drop(file);
    let _ = std::fs::remove_file(&path);

    (result, buf)
}

/// Run the same closure against the C lib and the Rust lib, capturing stdout
/// for each, and assert that both the returned value and the byte stream match.
pub fn diff<T: PartialEq + std::fmt::Debug>(
    ctx: &str,
    mut f: impl FnMut(&'static LibHandle) -> T,
) -> T {
    let _g = capture_lock();
    let (cv, cout) = capture_locked(|| f(clib()));
    let (rv, rout) = capture_locked(|| f(rlib()));
    assert_eq!(
        cv, rv,
        "{ctx}: return value mismatch\n  C   = {cv:?}\n  RUST= {rv:?}"
    );
    if cout != rout {
        panic!(
            "{ctx}: stdout mismatch\n--- C ({} bytes) ---\n{}\n--- RUST ({} bytes) ---\n{}\n--- C hex ---\n{:02x?}\n--- RUST hex ---\n{:02x?}",
            cout.len(),
            String::from_utf8_lossy(&cout),
            rout.len(),
            String::from_utf8_lossy(&rout),
            cout,
            rout
        );
    }
    cv
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds for reproducibility
// ---------------------------------------------------------------------------

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
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn i32(&mut self) -> c_int {
        self.u32() as c_int
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.u32() % n
    }
    /// Full-range value, biased so that ~1/3 of draws are interesting edges.
    pub fn i32_biased(&mut self) -> c_int {
        const EDGES: [c_int; 14] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            -3,
            4,
            -4,
            i32::MAX,
            i32::MIN,
            0x4000_0000u32 as i32,
            0x2000_0000,
            0x0000_ABCD,
        ];
        match self.below(3) {
            0 => EDGES[self.below(EDGES.len() as u32) as usize],
            1 => {
                // small magnitudes, both signs
                let v = (self.below(2001) as i32) - 1000;
                v
            }
            _ => self.i32(),
        }
    }
}

pub const BOUNDARY: [c_int; 12] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    -4,
    i32::MAX,
    i32::MIN,
    0x4000_0000u32 as i32,
    0x2000_0000,
];
