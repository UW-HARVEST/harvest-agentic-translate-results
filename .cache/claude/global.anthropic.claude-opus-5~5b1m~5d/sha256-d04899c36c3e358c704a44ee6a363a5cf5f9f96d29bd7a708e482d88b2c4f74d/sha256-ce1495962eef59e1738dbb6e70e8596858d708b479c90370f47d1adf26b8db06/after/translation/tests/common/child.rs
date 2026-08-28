//! Child-process plumbing for the error-path tests.
//!
//! Two things cannot be tested in-process:
//!
//! * `assert(string != NULL)` — it calls `__assert_fail`, which aborts the
//!   whole process, so each side has to run in its own process and be compared
//!   on exit signal + stderr;
//! * the `malloc`/`realloc`/`strdup` failure branches — reaching them requires
//!   interposing the allocator, which requires `LD_PRELOAD`, which requires a
//!   fresh process.
//!
//! The child is the very same test binary, re-executed with
//! `--exact zz_child_entry` and a handful of `DIFF_CHILD_*` environment
//! variables. `zz_child_entry` is a no-op when those are absent.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

pub const ENV_MODE: &str = "DIFF_CHILD_MODE";
pub const ENV_IMPL: &str = "DIFF_CHILD_IMPL";
pub const ENV_ARG: &str = "DIFF_CHILD_ARG";
pub const ENV_REPL: &str = "DIFF_CHILD_REPL";
pub const ENV_IN: &str = "DIFF_CHILD_IN";
pub const ENV_OUT: &str = "DIFF_CHILD_OUT";
pub const ENV_PRELOAD: &str = "DIFF_CHILD_PRELOAD";
pub const CHILD_TEST: &str = "zz_child_entry";

// ---------------------------------------------------------------------------
// scratch directory + LD_PRELOAD fixture
// ---------------------------------------------------------------------------

/// `target/diff-scratch`, wiped once per test-process so that repeated runs do
/// not accumulate input/output files (each child writes two).
pub fn scratch_dir() -> PathBuf {
    static CELL: OnceLock<PathBuf> = OnceLock::new();
    CELL.get_or_init(|| {
        let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/diff-scratch");
        // Only the parent wipes it; children must not delete files in flight.
        if std::env::var(ENV_MODE).is_err() {
            let _ = std::fs::remove_dir_all(&d);
        }
        std::fs::create_dir_all(&d).expect("create scratch dir");
        d
    })
    .clone()
}

/// Compile `tests/fixtures/failalloc.c` into a shared object (once per
/// process) and return its path.
pub fn failalloc_so() -> PathBuf {
    static CELL: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    let cell = CELL.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock().unwrap();
    if let Some(p) = guard.as_ref() {
        return p.clone();
    }
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/failalloc.c");
    let out = scratch_dir().join("libfailalloc.so");
    let st = Command::new("gcc")
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&out)
        .arg(&src)
        .status()
        .expect("run gcc to build the LD_PRELOAD fixture");
    assert!(st.success(), "gcc failed to build {}", src.display());
    *guard = Some(out.clone());
    out
}

// ---------------------------------------------------------------------------
// parent side
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ChildResult {
    pub signal: Option<i32>,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// contents of the child's result file (`None` if it never wrote one)
    pub result: Option<String>,
}

#[derive(Clone, Default)]
pub struct ChildSpec<'a> {
    pub mode: &'a str,
    /// `"c"` or `"rust"`
    pub imp: &'a str,
    /// mode-specific numeric argument
    pub arg: u64,
    /// value of the `replacement` parameter
    pub repl: u8,
    /// raw input bytes (NUL terminator is added by the child)
    pub input: Option<&'a [u8]>,
    /// preload the allocator interposer
    pub preload: bool,
}

static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn run_child(spec: &ChildSpec) -> ChildResult {
    use std::os::unix::process::ExitStatusExt;

    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let tag = format!("{}-{}-{}-{n}", spec.mode, spec.imp, std::process::id());
    let out_path = scratch_dir().join(format!("out-{tag}"));
    let _ = std::fs::remove_file(&out_path);

    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(exe);
    cmd.args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=1"])
        .env(ENV_MODE, spec.mode)
        .env(ENV_IMPL, spec.imp)
        .env(ENV_ARG, spec.arg.to_string())
        .env(ENV_REPL, spec.repl.to_string())
        .env(ENV_OUT, &out_path)
        .env_remove("RUST_BACKTRACE");

    if let Some(input) = spec.input {
        let in_path = scratch_dir().join(format!("in-{tag}"));
        std::fs::write(&in_path, input).expect("write child input");
        cmd.env(ENV_IN, &in_path);
    }
    if spec.preload {
        let fa = failalloc_so();
        cmd.env(ENV_PRELOAD, &fa);
        cmd.env("LD_PRELOAD", &fa);
    }

    let o = cmd.output().expect("spawn child test process");
    let res = ChildResult {
        signal: o.status.signal(),
        code: o.status.code(),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        result: std::fs::read_to_string(&out_path).ok(),
    };
    // Don't leak scratch files: a full suite run spawns several hundred
    // children, each with an input and an output file.
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(scratch_dir().join(format!("in-{tag}")));
    res
}

/// Run the same spec against both implementations.
pub fn run_both(spec: &ChildSpec) -> (ChildResult, ChildResult) {
    let mut c = spec.clone();
    c.imp = "c";
    let mut r = spec.clone();
    r.imp = "rust";
    (run_child(&c), run_child(&r))
}

// ---------------------------------------------------------------------------
// child side: handle to the LD_PRELOAD fixture
// ---------------------------------------------------------------------------

pub struct FailAlloc {
    pub arm: unsafe extern "C" fn(i32, i32, i32),
    pub set_min_size: unsafe extern "C" fn(usize),
    pub fired: unsafe extern "C" fn() -> i32,
    pub disarm: unsafe extern "C" fn(),
    pub trace_begin: unsafe extern "C" fn(),
    pub trace_end: unsafe extern "C" fn(),
    pub trace_count: unsafe extern "C" fn() -> usize,
    pub trace_overflow: unsafe extern "C" fn() -> usize,
    pub trace_kind: unsafe extern "C" fn(usize) -> i32,
    pub trace_arg: unsafe extern "C" fn(usize) -> usize,
    _lib: Library,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $name:literal) => {{
        let s: Symbol<$t> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("failalloc has no {}: {e}", $name));
        *s
    }};
}

pub fn load_failalloc(path: &Path) -> FailAlloc {
    unsafe {
        let lib = Library::new(path).expect("dlopen failalloc");
        FailAlloc {
            arm: sym!(lib, unsafe extern "C" fn(i32, i32, i32), "failalloc_arm"),
            set_min_size: sym!(lib, unsafe extern "C" fn(usize), "failalloc_set_min_size"),
            fired: sym!(lib, unsafe extern "C" fn() -> i32, "failalloc_fired"),
            disarm: sym!(lib, unsafe extern "C" fn(), "failalloc_disarm"),
            trace_begin: sym!(lib, unsafe extern "C" fn(), "failalloc_trace_begin"),
            trace_end: sym!(lib, unsafe extern "C" fn(), "failalloc_trace_end"),
            trace_count: sym!(lib, unsafe extern "C" fn() -> usize, "failalloc_trace_count"),
            trace_overflow: sym!(
                lib,
                unsafe extern "C" fn() -> usize,
                "failalloc_trace_overflow"
            ),
            trace_kind: sym!(lib, unsafe extern "C" fn(usize) -> i32, "failalloc_trace_kind"),
            trace_arg: sym!(lib, unsafe extern "C" fn(usize) -> usize, "failalloc_trace_arg"),
            _lib: lib,
        }
    }
}

/// FNV-1a — lets a child summarise a large output in its result file.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}
