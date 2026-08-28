//! Shared harness: loads the C reference `.so` and the Rust `.so` through
//! `libloading` and captures the stdout each of them produces so the two can be
//! compared byte-for-byte.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// C ABI declarations mirroring c_src/src/lib.c
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

pub type FnBinop = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnGetOperation = unsafe extern "C" fn(c_int) -> OperationFunc;
pub type FnExecuteOperation =
    unsafe extern "C" fn(OperationFunc, c_int, c_int, *const c_char) -> c_int;
pub type FnComputeChecksum = unsafe extern "C" fn(*mut c_int, c_int) -> c_uint;
pub type FnInitState = unsafe extern "C" fn(*mut ComputeState, c_int);
pub type FnApplyOperation = unsafe extern "C" fn(*mut ComputeState, c_int, OperationFunc);
pub type FnCheckshift = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with file descriptor 1 redirected to a temporary file, returning
/// `f`'s value together with everything written to stdout (by Rust *or* by the
/// C `printf` inside either shared library).
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let _guard = capture_lock();

    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("xlat_cap_{}_{}.out", std::process::id(), n));

    let file = File::create(&path).expect("create capture file");
    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    unsafe {
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
    }

    let result = f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);

    let data = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (result, data)
}

// ---------------------------------------------------------------------------
// library discovery / loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workdir() -> PathBuf {
    manifest_dir().parent().expect("workspace root").to_path_buf()
}

fn newest_so(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        if name.starts_with(prefix) && name.ends_with(".so") {
            let t = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// The C reference shared library. Its file name follows the enclosing
/// directory name (see `c_src/CMakeLists.txt`), so it is discovered by glob.
pub fn c_so_path() -> PathBuf {
    let build = workdir().join("c_src").join("build");
    newest_so(&build, "lib").unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Builds the `cdylib` into a dedicated target directory. `cargo test` only
/// emits the cdylib for `--release`, and nesting a build into the *same* target
/// directory would deadlock on cargo's build lock, hence the separate one.
fn build_cdylib(release: bool) -> Option<PathBuf> {
    let target = manifest_dir().join("target").join("cdylib-under-test");
    let cargo = option_env!("CARGO").unwrap_or("cargo");
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build").arg("--lib");
    if release {
        cmd.arg("--release");
    }
    cmd.current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &target)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit());
    let status = cmd.status().ok()?;
    if !status.success() {
        return None;
    }
    let dir = target.join(if release { "release" } else { "debug" });
    newest_so(&dir, "libcheckshift_lib")
}

/// The Rust shared library actually under test. Always loaded through
/// `libloading`, never linked, so the `#[no_mangle]` wrappers are exercised
/// exactly as an external C caller would exercise them.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        // `cargo test` places the test executable in target/<profile>/deps/;
        // the cdylib, when present, sits in target/<profile>/.
        let exe = std::env::current_exe().expect("current_exe");
        let release = exe.components().any(|c| c.as_os_str() == "release");

        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            if d.file_name().map(|n| n == "target").unwrap_or(false) {
                break;
            }
            if let Some(p) = newest_so(&d, "libcheckshift_lib") {
                return p;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }

        build_cdylib(release).unwrap_or_else(|| {
            panic!("libcheckshift_lib.so not found and `cargo build --lib` did not produce it")
        })
    })
    .clone()
}


pub struct Impls {
    pub c: Library,
    pub rs: Library,
}

impl Impls {
    pub fn sym<T>(&self, which: Which, name: &str) -> Symbol<'_, T> {
        let lib = match which {
            Which::C => &self.c,
            Which::Rust => &self.rs,
        };
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        unsafe { lib.get::<T>(&bytes) }
            .unwrap_or_else(|e| panic!("symbol `{}` missing from {:?} lib: {}", name, which, e))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Which {
    C,
    Rust,
}

pub const BOTH: [Which; 2] = [Which::C, Which::Rust];

/// Both libraries, loaded once per test process.
pub fn impls() -> &'static Impls {
    static LIBS: OnceLock<Impls> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rs_path = rust_so_path();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("load {}: {}", c_path.display(), e));
        let rs = unsafe { Library::new(&rs_path) }
            .unwrap_or_else(|e| panic!("load {}: {}", rs_path.display(), e));
        Impls { c, rs }
    })
}

// ---------------------------------------------------------------------------
// interesting integer inputs
// ---------------------------------------------------------------------------

/// A spread of values covering zero, small magnitudes, sign changes, powers of
/// two, shift boundaries and the extremes of `int`.
pub fn sample_ints() -> Vec<c_int> {
    vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        4,
        -4,
        7,
        -7,
        8,
        16,
        -16,
        31,
        32,
        -32,
        63,
        64,
        100,
        -100,
        255,
        256,
        -256,
        1000,
        -1000,
        0xABCD,
        0xBEEF,
        0xFFFF,
        -0xFFFF,
        0x1_0000,
        0x7FFF,
        -0x8000,
        12345,
        -12345,
        65536,
        1 << 20,
        -(1 << 20),
        1 << 29,
        1 << 30,
        -(1 << 30),
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        -0x4000_0000,
        0x5555_5555,
        -0x5555_5555,
        0x3333_3333,
        0x0F0F_0F0F,
        -0x0F0F_0F0F,
        0x1234_5678,
        -0x1234_5678,
    ]
}

/// Deterministic xorshift PRNG so both sides see the exact same inputs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
}

/// Renders captured stdout for assertion messages.
pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}

// ---------------------------------------------------------------------------
// minimal sequential test runner
// ---------------------------------------------------------------------------
//
// These targets use `harness = false`. The reason is that comparing C `printf`
// output requires redirecting file descriptor 1, which is process-global: the
// default libtest harness writes its own progress lines to stdout from other
// threads and those bytes would land in the capture file. Running the cases
// sequentially in a binary that reports to stderr removes that interference.

pub struct Runner {
    passed: usize,
    failed: Vec<String>,
    filter: Option<String>,
    skipped: usize,
}

impl Runner {
    pub fn new() -> Self {
        // Accept the usual `cargo test <target> -- <substring>` filter.
        let filter = std::env::args()
            .skip(1)
            .find(|a| !a.starts_with('-'))
            .filter(|a| !a.is_empty());
        Runner {
            passed: 0,
            failed: Vec::new(),
            filter,
            skipped: 0,
        }
    }

    pub fn case<F: FnOnce() + std::panic::UnwindSafe>(&mut self, name: &str, f: F) {
        if let Some(filter) = &self.filter {
            if !name.contains(filter.as_str()) {
                self.skipped += 1;
                return;
            }
        }
        eprint!("test {name} ... ");
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(f);
        std::panic::set_hook(prev);
        match result {
            Ok(()) => {
                eprintln!("ok");
                self.passed += 1;
            }
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    (*s).to_string()
                } else {
                    "<non-string panic>".to_string()
                };
                eprintln!("FAILED");
                eprintln!("---- {name} ----\n{msg}\n");
                self.failed.push(name.to_string());
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
