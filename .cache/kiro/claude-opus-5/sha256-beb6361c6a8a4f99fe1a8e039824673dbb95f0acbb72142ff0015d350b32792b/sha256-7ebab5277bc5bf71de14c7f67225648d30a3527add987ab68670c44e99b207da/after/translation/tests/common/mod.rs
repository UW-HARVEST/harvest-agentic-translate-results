//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! invoked only through their exported C symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test. No function of the crate is ever
//! called directly.
//!
//! Every call sequence runs in its own freshly `exec`ed child process. That
//! buys three things:
//!
//! * The output is compared as the raw bytes that reach file descriptor 1,
//!   including whatever is sitting in the C `stdio` buffer. Redirecting fd 1
//!   inside the test process would race against libtest's own writes from
//!   other threads.
//! * `bad()` is undefined behaviour in the original C and can fault; a child
//!   dying takes nothing else with it.
//! * Each sequence starts from a pristine `stdio` state and a pristine stack.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Which shared object the child should load.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn path(self) -> PathBuf {
        match self {
            Impl::C => c_lib_path(),
            Impl::Rust => rust_lib_path(),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

pub const BOTH: [Impl; 2] = [Impl::C, Impl::Rust];

/// One call into the library under test.
#[derive(Clone, Debug)]
pub enum Op {
    /// `printIntPtrLine(&v)` where `v` is a fresh local.
    Print(i32),
    /// `printIntPtrLine(&arr[i])` - exercises a pointer into the middle of an
    /// object rather than to a standalone local.
    PrintArrayElem(Vec<i32>, usize),
    /// `good()`
    Good,
    /// `bad()` - undefined behaviour, may fault.
    Bad,
    /// `driver(n)`
    Driver(i32),
    /// Not a library call: the *caller* writes this text with its own libc
    /// `printf`. Interleaving it with library calls checks that the library
    /// shares the caller's `stdio` buffer, so output ordering matches. Must
    /// contain no `:` or `;`.
    HostPrint(String),
    /// `printIntPtrLine(NULL)`. Undefined in C, and in practice a fault; a
    /// translation that added a null check would diverge here.
    PrintNull,
    /// `printIntPtrLine` on a deliberately misaligned pointer holding `v`.
    PrintUnaligned(i32),
}

impl Op {
    fn encode(&self) -> String {
        match self {
            Op::Print(v) => format!("p:{v}"),
            Op::PrintArrayElem(vals, i) => {
                let csv: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                format!("a:{}:{}", csv.join(","), i)
            }
            Op::Good => "g".to_string(),
            Op::Bad => "b".to_string(),
            Op::Driver(v) => format!("d:{v}"),
            Op::HostPrint(s) => {
                assert!(
                    !s.contains(':') && !s.contains(';'),
                    "HostPrint text may not contain the op separators"
                );
                format!("h:{s}")
            }
            Op::PrintNull => "n".to_string(),
            Op::PrintUnaligned(v) => format!("u:{v}"),
        }
    }

    fn decode(s: &str) -> Op {
        let bits: Vec<&str> = s.split(':').collect();
        match bits[0] {
            "p" => Op::Print(bits[1].parse().expect("bad int in op")),
            "a" => {
                let vals = bits[1]
                    .split(',')
                    .map(|v| v.parse().expect("bad int in array op"))
                    .collect();
                Op::PrintArrayElem(vals, bits[2].parse().expect("bad index"))
            }
            "g" => Op::Good,
            "b" => Op::Bad,
            "d" => Op::Driver(bits[1].parse().expect("bad int in op")),
            "h" => Op::HostPrint(bits[1].to_string()),
            "n" => Op::PrintNull,
            "u" => Op::PrintUnaligned(bits[1].parse().expect("bad int in op")),
            other => panic!("unknown op {other:?}"),
        }
    }
}

/// How a child process finished.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Ran the whole sequence and returned from every call.
    Completed,
    /// Killed by a signal, e.g. SIGSEGV from the bad dereference.
    Signalled(i32),
    /// Anything else: a panic, an abort, a failure to load or resolve.
    Failed(String),
}

pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;

/// Result of one child run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub status: Status,
    /// Exact bytes that reached fd 1.
    pub bytes: Vec<u8>,
}

impl Outcome {
    pub fn completed(&self) -> bool {
        self.status == Status::Completed
    }

    /// True if the run died on a memory-access fault.
    pub fn faulted(&self) -> bool {
        matches!(self.status, Status::Signalled(s) if s == SIGSEGV || s == SIGBUS)
    }
}

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

/// Repository root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C shared library built from `c_src/`.
pub fn c_lib_path() -> PathBuf {
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && \
         cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib`, preferring the profile these tests were built
/// with so `cargo test` and `cargo test --release` each check their own build.
///
/// The crate is `crate-type = ["cdylib"]`, so `cargo test` does not necessarily
/// refresh the shared object; the staleness check keeps a run from silently
/// validating an old build.
pub fn rust_lib_path() -> PathBuf {
    let target = repo_root().join("translation/target");
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in preferred {
        let p = target.join(profile).join("libdriver.so");
        if p.is_file() {
            assert_not_stale(&p);
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {} - run `cargo build` and `cargo build --release` first",
        target.display()
    );
}

/// Fails loudly if the shared object predates the source it is built from.
fn assert_not_stale(lib: &Path) {
    let src = repo_root().join("translation/src/lib.rs");
    let modified = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    assert!(
        modified(lib) >= modified(&src),
        "{} is older than src/lib.rs - rebuild before running the differential tests",
        lib.display()
    );
}

// ---------------------------------------------------------------------------
// Parent side: launching children
// ---------------------------------------------------------------------------

const ENV_LIB: &str = "DRIVER_DIFF_LIB";
const ENV_OPS: &str = "DRIVER_DIFF_OPS";
const ENV_OUT: &str = "DRIVER_DIFF_OUT";

/// Exit code the worker uses on success. Deliberately not 0, so a child that
/// never reached the worker body (a bad filter, a libtest change) is reported
/// as a failure instead of silently comparing two empty outputs.
const WORKER_OK: i32 = 42;

/// Name of the `#[test]` function that acts as the child-side worker. Each test
/// binary must define one with this name that calls [`run_worker_if_child`].
const WORKER_TEST: &str = "difftest_worker";

/// Runs `ops` against one implementation in a fresh process.
pub fn run(which: Impl, ops: &[Op]) -> Outcome {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let out_path = std::env::temp_dir().join(format!(
        "driver-difftest-{}-{}-{}",
        std::process::id(),
        which.name(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&out_path);
    std::fs::write(&out_path, b"").expect("could not create capture file");

    let encoded: Vec<String> = ops.iter().map(Op::encode).collect();
    let lib = which.path();

    let status = Command::new(std::env::current_exe().expect("current_exe"))
        .args(["--exact", WORKER_TEST, "--test-threads", "1"])
        .env(ENV_LIB, &lib)
        .env(ENV_OPS, encoded.join(";"))
        .env(ENV_OUT, &out_path)
        .stdin(Stdio::null())
        // libtest's own chatter goes nowhere; the library's output goes to the
        // capture file, which the worker installs as fd 1 itself.
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn worker child");

    let bytes = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);

    use std::os::unix::process::ExitStatusExt;
    let st = if let Some(sig) = status.status.signal() {
        Status::Signalled(sig)
    } else if status.status.code() == Some(WORKER_OK) {
        Status::Completed
    } else {
        Status::Failed(format!(
            "exit code {:?}; stderr: {}",
            status.status.code(),
            String::from_utf8_lossy(&status.stderr).trim()
        ))
    };

    Outcome { status: st, bytes }
}

/// Runs `ops` against both implementations and asserts the observable results
/// are identical: same completion status, same bytes on fd 1. Returns the
/// (agreed) bytes.
///
/// Only use this for sequences that are fully defined in the C; the `bad()`
/// path is not.
pub fn assert_identical(ops: &[Op], label: &str) -> Vec<u8> {
    let c = run(Impl::C, ops);
    let rust = run(Impl::Rust, ops);

    assert!(
        c.completed(),
        "{label}: the C implementation did not complete: {:?}",
        c.status
    );
    assert!(
        rust.completed(),
        "{label}: the Rust implementation did not complete: {:?} (C: {:?}, C output {})",
        rust.status,
        c.status,
        show(&c.bytes)
    );
    assert_eq!(
        c.bytes,
        rust.bytes,
        "{label}: output differs\n  C    wrote {}\n  Rust wrote {}",
        show(&c.bytes),
        show(&rust.bytes)
    );
    c.bytes
}

/// Renders bytes for assertion messages, keeping non-UTF-8 output readable.
pub fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

// ---------------------------------------------------------------------------
// Child side: the worker
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
    fn _exit(code: i32) -> !;
    fn open(path: *const std::ffi::c_char, flags: i32, ...) -> i32;
    fn printf(fmt: *const std::ffi::c_char, ...) -> i32;
}

type VoidFn = unsafe extern "C" fn();
type IntFn = unsafe extern "C" fn(std::ffi::c_int);
type IntPtrFn = unsafe extern "C" fn(*const std::ffi::c_int);

/// If this process was launched by [`run`], carry out the requested calls and
/// exit without returning. Otherwise do nothing, so the parent's own
/// invocation of the worker test is a no-op.
pub fn run_worker_if_child() {
    let Ok(lib) = std::env::var(ENV_LIB) else {
        return;
    };
    let ops: Vec<Op> = std::env::var(ENV_OPS)
        .expect("worker needs an op list")
        .split(';')
        .filter(|s| !s.is_empty())
        .map(Op::decode)
        .collect();
    let out = std::env::var(ENV_OUT).expect("worker needs an output path");

    unsafe {
        let library = libloading::Library::new(&lib)
            .unwrap_or_else(|e| panic!("worker could not dlopen {lib}: {e}"));

        let print: libloading::Symbol<IntPtrFn> =
            library.get(b"printIntPtrLine\0").expect("dlsym printIntPtrLine");
        let good: libloading::Symbol<VoidFn> = library.get(b"good\0").expect("dlsym good");
        let bad: libloading::Symbol<VoidFn> = library.get(b"bad\0").expect("dlsym bad");
        let driver: libloading::Symbol<IntFn> = library.get(b"driver\0").expect("dlsym driver");

        // Install the capture file as fd 1 only now, so nothing written during
        // start-up can pollute it.
        let path = std::ffi::CString::new(out).unwrap();
        const O_WRONLY: i32 = 0o1;
        const O_TRUNC: i32 = 0o1000;
        let fd = open(path.as_ptr(), O_WRONLY | O_TRUNC);
        assert!(fd >= 0, "worker could not open its capture file");
        assert!(dup2(fd, 1) >= 0, "worker could not redirect fd 1");

        for op in &ops {
            match op {
                Op::Print(v) => {
                    let local: std::ffi::c_int = *v;
                    print(&raw const local);
                }
                Op::PrintArrayElem(vals, i) => {
                    let arr: Vec<std::ffi::c_int> = vals.clone();
                    print(arr.as_ptr().add(*i));
                }
                Op::Good => good(),
                Op::Bad => bad(),
                Op::Driver(v) => driver(*v),
                Op::HostPrint(s) => {
                    // The caller's own printf, deliberately not the library's.
                    let text = std::ffi::CString::new(s.as_str()).unwrap();
                    printf(c"%s".as_ptr(), text.as_ptr());
                }
                Op::PrintNull => print(std::ptr::null()),
                Op::PrintUnaligned(v) => {
                    // Place the int at an odd offset so the pointer handed to
                    // the library is misaligned, as the C would permit.
                    let mut buf = [0u8; 16];
                    buf[1..5].copy_from_slice(&v.to_ne_bytes());
                    print(buf.as_ptr().add(1).cast::<std::ffi::c_int>())
                }
            }
        }

        // Flush the C buffer while fd 1 is still the capture file, then leave
        // immediately: no libtest summary, and a distinctive exit code.
        fflush(std::ptr::null_mut());
        _exit(WORKER_OK);
    }
}
