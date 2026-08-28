//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes matching symbol accessors, so every comparison
//! goes through the real dynamic-linker/FFI boundary on both sides.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_long, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type TimeT = i64;

/// `typedef struct { int value; time_t timestamp; StatusCode status; }`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: TimeT,
    pub status: c_int,
}

pub type MathOperationFn = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Function-pointer signatures for every exported symbol
// ---------------------------------------------------------------------------
pub type FnIsValidOperation = unsafe extern "C" fn(c_char) -> bool;
pub type FnGetOperationPriority = unsafe extern "C" fn(c_int) -> c_int;
pub type FnBinOp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnSelectOperation = unsafe extern "C" fn(c_int) -> MathOperationFn;
pub type FnGetComputationTimestamp = unsafe extern "C" fn() -> TimeT;
pub type FnAllocateResults = unsafe extern "C" fn(c_int) -> *mut ComputationResult;
pub type FnPerformComputation =
    unsafe extern "C" fn(c_int, c_int, c_int, *mut *mut ComputationResult, *mut c_int) -> c_int;
pub type FnMathop = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// The complete public API of one implementation.
pub struct Api {
    _lib: Library,
    pub is_valid_operation: FnIsValidOperation,
    pub get_operation_priority: FnGetOperationPriority,
    pub add_operation: FnBinOp,
    pub multiply_operation: FnBinOp,
    pub subtract_operation: FnBinOp,
    pub divide_operation: FnBinOp,
    pub modulo_operation: FnBinOp,
    pub select_operation: FnSelectOperation,
    pub get_computation_timestamp: FnGetComputationTimestamp,
    pub allocate_results: FnAllocateResults,
    pub perform_computation_with_history: FnPerformComputation,
    pub mathop: FnMathop,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Api {
    fn load(path: &PathBuf) -> Api {
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
            Api {
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
                _lib: lib,
            }
        }
    }

    /// Address of each named exported function, used to identify which
    /// function a returned `MathOperation` pointer actually refers to.
    pub fn op_table(&self) -> [(&'static str, usize); 5] {
        [
            ("add_operation", self.add_operation as usize),
            ("multiply_operation", self.multiply_operation as usize),
            ("subtract_operation", self.subtract_operation as usize),
            ("divide_operation", self.divide_operation as usize),
            ("modulo_operation", self.modulo_operation as usize),
        ]
    }

    pub fn name_of_op(&self, p: MathOperationFn) -> String {
        let addr = p as usize;
        for (n, a) in self.op_table() {
            if a == addr {
                return n.to_string();
            }
        }
        format!("<unknown fn @ {addr:#x}>")
    }
}

pub struct Both {
    pub c: Api,
    pub rust: Api,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

pub fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not found ({e}); build the C library first"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop().expect("no lib*.so in c_src/build")
}

/// `cargo test` builds only the test binaries, not the `cdylib` lib target, so
/// the harness builds it explicitly. Without this a stale `.so` left over in
/// `target/*/deps/` would silently be tested instead of the current sources.
fn build_rust_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    let profile_name = profile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("debug")
        .to_string();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build").arg("--lib");
    cmd.arg("--manifest-path").arg(&manifest);
    match profile_name.as_str() {
        "debug" => {}
        "release" => {
            cmd.arg("--release");
        }
        other => {
            cmd.arg("--profile").arg(other);
        }
    }
    // Keep artefacts in the very target dir this test binary came from.
    if let Some(target_root) = profile_dir.parent() {
        cmd.env("CARGO_TARGET_DIR", target_root);
    }
    // Inherit the feature selection the test run was configured with.
    if let Ok(feats) = std::env::var("MATHOP_TEST_FEATURES") {
        cmd.arg("--no-default-features");
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats);
        }
    }
    cmd.env_remove("RUSTFLAGS_OVERRIDE");

    match cmd.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => panic!(
            "failed to build the cdylib under test:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => panic!("could not run cargo to build the cdylib: {e}"),
    }

    let canonical = profile_dir.join("libmathop_lib.so");
    if canonical.exists() {
        return canonical;
    }
    let fallback = deps.join("libmathop_lib.so");
    if fallback.exists() {
        return fallback;
    }
    panic!("libmathop_lib.so not produced in {profile_dir:?}");
}

pub fn find_rust_so() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(build_rust_so).clone()
}

static BOTH: OnceLock<Both> = OnceLock::new();

/// Both implementations, loaded exactly once per test process.
pub fn both() -> &'static Both {
    BOTH.get_or_init(|| Both {
        c: Api::load(&find_c_so()),
        rust: Api::load(&find_rust_so()),
    })
}

/// Serialises tests that touch process-wide state (`mathop`'s statics, stdout).
pub fn global_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// stdout capture (both libraries print through the shared libc stdout)
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Runs `f` with libc/Rust stdout redirected into a temp file and returns the
/// raw bytes that were written.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::AsRawFd;

    let path = std::env::temp_dir().join(format!(
        "mathop_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");

    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let out = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);

        file.seek(SeekFrom::Start(0)).expect("seek");
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read capture");
        let _ = std::fs::remove_file(&path);
        (out, buf)
    }
}

/// Raw byte view of a `ComputationResult` array, for layout-exact comparison.
pub unsafe fn raw_bytes(p: *const ComputationResult, count: usize) -> Vec<u8> {
    std::slice::from_raw_parts(
        p as *const u8,
        count * std::mem::size_of::<ComputationResult>(),
    )
    .to_vec()
}

const _SIZE_CHECK: () = assert!(std::mem::size_of::<ComputationResult>() == 24);
const _ALIGN_CHECK: () = assert!(std::mem::align_of::<ComputationResult>() == 8);

/// Silence "unused" warnings for the c_long import used by signatures below.
pub type _CLong = c_long;
