//! Shared plumbing for the differential C-vs-Rust tests.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and driven purely through their exported symbols, so the
//! `#[no_mangle]` wrappers are part of what is under test.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C structs (must match c_src/include/task_manager.h byte for byte)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskRaw {
    pub description: [u8; 256],
    pub priority: c_int,
}

#[repr(C)]
pub struct TaskManagerRaw {
    pub tasks: *mut TaskRaw,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    static stdout: *mut c_void;
    static stderr: *mut c_void;
}

pub fn env_set(name: &str, value: Option<&str>) {
    let n = CString::new(name).unwrap();
    unsafe {
        match value {
            Some(v) => {
                let v = CString::new(v).unwrap();
                setenv(n.as_ptr(), v.as_ptr(), 1);
            }
            None => {
                unsetenv(n.as_ptr());
            }
        }
    }
}

/// Same, but for values that are not valid UTF-8 / contain arbitrary bytes.
pub fn env_set_bytes(name: &str, value: &[u8]) {
    let n = CString::new(name).unwrap();
    let v = CString::new(value).unwrap();
    unsafe {
        setenv(n.as_ptr(), v.as_ptr(), 1);
    }
}

// ---------------------------------------------------------------------------
// The loaded API surface
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    _lib: libloading::Library,
    pub initialize_logger: unsafe extern "C" fn() -> c_int,
    pub log_info: unsafe extern "C" fn(*const c_char),
    pub log_warning: unsafe extern "C" fn(*const c_char),
    pub log_error: unsafe extern "C" fn(*const c_char),
    pub finalize_logger: unsafe extern "C" fn(),
    pub create_task_manager: unsafe extern "C" fn() -> *mut TaskManagerRaw,
    pub add_task: unsafe extern "C" fn(*mut TaskManagerRaw, *const c_char, c_int),
    pub print_tasks: unsafe extern "C" fn(*const TaskManagerRaw),
    pub destroy_task_manager: unsafe extern "C" fn(*mut TaskManagerRaw),
    pub driver: unsafe extern "C" fn(*const c_char) -> c_int,
}

impl Api {
    pub fn load(name: &'static str, path: &Path) -> Api {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            macro_rules! sym {
                ($t:ty, $n:literal) => {{
                    let s: libloading::Symbol<$t> = lib
                        .get($n)
                        .unwrap_or_else(|e| panic!("{} missing {}: {e}", path.display(), stringify!($n)));
                    *s.into_raw()
                }};
            }
            let initialize_logger = sym!(unsafe extern "C" fn() -> c_int, b"initialize_logger\0");
            let log_info = sym!(unsafe extern "C" fn(*const c_char), b"log_info\0");
            let log_warning = sym!(unsafe extern "C" fn(*const c_char), b"log_warning\0");
            let log_error = sym!(unsafe extern "C" fn(*const c_char), b"log_error\0");
            let finalize_logger = sym!(unsafe extern "C" fn(), b"finalize_logger\0");
            let create_task_manager =
                sym!(unsafe extern "C" fn() -> *mut TaskManagerRaw, b"create_task_manager\0");
            let add_task = sym!(
                unsafe extern "C" fn(*mut TaskManagerRaw, *const c_char, c_int),
                b"add_task\0"
            );
            let print_tasks = sym!(unsafe extern "C" fn(*const TaskManagerRaw), b"print_tasks\0");
            let destroy_task_manager =
                sym!(unsafe extern "C" fn(*mut TaskManagerRaw), b"destroy_task_manager\0");
            let driver = sym!(unsafe extern "C" fn(*const c_char) -> c_int, b"driver\0");
            Api {
                name,
                _lib: lib,
                initialize_logger,
                log_info,
                log_warning,
                log_error,
                finalize_logger,
                create_task_manager,
                add_task,
                print_tasks,
                destroy_task_manager,
                driver,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Build + load, once per test process
// ---------------------------------------------------------------------------

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn build_c() {
    if c_so_path().exists() {
        return;
    }
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).unwrap();
    let cfg = std::process::Command::new("cmake")
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .current_dir(&build)
        .output()
        .expect("run cmake");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let out = std::process::Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        out.status.success(),
        "cmake build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(c_so_path().exists(), "C .so not produced");
}

fn build_rust() {
    // Feature flags of the crate under test, forwarded from the runner script.
    let mut args: Vec<String> = vec!["build".into(), "--release".into(), "--lib".into()];
    if std::env::var_os("DIFFTEST_NO_DEFAULT_FEATURES").is_some() {
        args.push("--no-default-features".into());
    }
    if let Ok(f) = std::env::var("DIFFTEST_FEATURES") {
        if !f.is_empty() {
            args.push("--features".into());
            args.push(f);
        }
    }
    let out = std::process::Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("RUSTFLAGS")
        .output()
        .expect("run cargo build");
    assert!(
        out.status.success(),
        "cargo build --release failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(rust_so_path().exists(), "Rust .so not produced");
}

struct Both {
    c: Api,
    rust: Api,
}

static BOTH: OnceLock<Both> = OnceLock::new();
static SERIAL: Mutex<()> = Mutex::new(());

fn both() -> &'static Both {
    BOTH.get_or_init(|| {
        build_c();
        build_rust();
        // Every test writes its log/temp files into a scratch directory so the
        // `LOG_FILE`-unset case (which uses a relative "default.log") is safe.
        let scratch = scratch_dir();
        std::fs::create_dir_all(&scratch).unwrap();
        std::env::set_current_dir(&scratch).unwrap();
        Both {
            c: Api::load("C", &c_so_path()),
            rust: Api::load("Rust", &rust_so_path()),
        }
    })
}

pub fn scratch_dir() -> PathBuf {
    PathBuf::from(format!("/tmp/difftest-driver-{}", std::process::id()))
}

/// Makes sure both shared libraries exist on disk (used by tests that do not go
/// through [`compare`]).
pub fn ensure_built() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        build_c();
        build_rust();
    });
}

/// Loads exactly one of the two libraries, leaking it so that any `atexit`
/// handler it registered still runs at process exit.  Used by the child
/// processes of the exit-time tests, which must not have the other library
/// loaded at the same time.
pub fn load_single(which: &str) -> &'static Api {
    ensure_built();
    let api = match which {
        "c" => Api::load("C", &c_so_path()),
        "rust" => Api::load("Rust", &rust_so_path()),
        other => panic!("unknown library selector {other:?}"),
    };
    Box::leak(Box::new(api))
}

/// Serialises tests: the environment, the process CWD and fds 1/2 are all
/// global state that the comparison manipulates.
pub fn lock() -> MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

pub struct Captured {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Captured) {
    let dir = scratch_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let out_path = dir.join("capture.out");
    let err_path = dir.join("capture.err");
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);

    let out_file = std::fs::File::create(&out_path).unwrap();
    let err_file = std::fs::File::create(&err_path).unwrap();
    let (out_fd, err_fd) = {
        use std::os::unix::io::AsRawFd;
        (out_file.as_raw_fd(), err_file.as_raw_fd())
    };

    let result;
    unsafe {
        fflush(stdout);
        fflush(stderr);
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0);
        dup2(out_fd, 1);
        dup2(err_fd, 2);

        result = f();

        fflush(stdout);
        fflush(stderr);
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
    }
    drop(out_file);
    drop(err_file);

    let so = std::fs::read(&out_path).unwrap_or_default();
    let se = std::fs::read(&err_path).unwrap_or_default();
    (
        result,
        Captured {
            stdout: strip_libtest_noise(&so),
            stderr: strip_libtest_noise(&se),
        },
    )
}

/// Belt and braces: libtest writes its own progress lines to fd 1.  Tests are
/// forced single-threaded (see .cargo/config.toml) so this should never fire,
/// but if it does the lines are dropped rather than mistaken for library
/// output.  None of these patterns can be produced by the library under test:
/// its stdout is only ever `Tasks:` and `  [n] ... (Priority: n)` lines.
fn strip_libtest_noise(bytes: &[u8]) -> Vec<u8> {
    let is_noise = |line: &[u8]| {
        let s = String::from_utf8_lossy(line);
        s.starts_with("running ") && s.ends_with(" tests")
            || s.starts_with("running 1 test")
            || s.starts_with("test result:")
            || (s.starts_with("test ")
                && (s.ends_with(" ... ok")
                    || s.ends_with(" ... FAILED")
                    || s.ends_with(" ... ignored")))
    };
    if !bytes.split(|&b| b == b'\n').any(is_noise) {
        return bytes.to_vec();
    }
    let mut out = Vec::with_capacity(bytes.len());
    let trailing_newline = bytes.last() == Some(&b'\n');
    let lines: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
    for (i, line) in lines.iter().enumerate() {
        let last = i + 1 == lines.len();
        if is_noise(line) {
            continue;
        }
        if last && !trailing_newline {
            out.extend_from_slice(line);
        } else if !last {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The differential driver
// ---------------------------------------------------------------------------

/// Everything observable about one run of a scenario.
pub struct Outcome {
    pub transcript: Vec<String>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub log: Vec<u8>,
}

/// Default log path used when a scenario does not pick its own.
pub fn default_log_path() -> PathBuf {
    scratch_dir().join("test.log")
}

pub fn bytes_to_path(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

/// Runs `scenario` against the C library and then against the Rust library
/// under identical conditions, and asserts that every observable matches.
///
/// * `env` is applied verbatim before each run (`None` == unset).
/// * The `LOG_FILE` target is wiped before each run so the two runs cannot see
///   each other's output.
pub fn compare<F>(case: &str, env: &[(&str, Option<&str>)], scenario: F)
where
    F: Fn(&Api, &mut Vec<String>),
{
    let env: Vec<(&str, Option<Vec<u8>>)> = env
        .iter()
        .map(|(k, v)| (*k, v.map(|s| s.as_bytes().to_vec())))
        .collect();
    compare_raw(case, &env, scenario)
}

/// Like [`compare`], but the environment values are raw bytes, which is how
/// `getenv` actually hands them to the library.
pub fn compare_raw<F>(case: &str, env: &[(&str, Option<Vec<u8>>)], scenario: F)
where
    F: Fn(&Api, &mut Vec<String>),
{
    let _guard = lock();
    let libs = both();

    // Which file will the logger open?  Mirrors logger.c's own decision.
    let log_target: PathBuf = match env.iter().rev().find(|(k, _)| *k == "LOG_FILE") {
        Some((_, Some(v))) => bytes_to_path(v),
        Some((_, None)) => PathBuf::from("default.log"),
        None => default_log_path(),
    };

    let mut outcomes: Vec<Outcome> = Vec::new();
    for api in [&libs.c, &libs.rust] {
        // Baseline environment, then the case's overrides.
        env_set("LOG_FILE", Some(default_log_path().to_str().unwrap()));
        env_set("MAX_TASKS", None);
        for (k, v) in env {
            match v {
                Some(bytes) => env_set_bytes(k, bytes),
                None => env_set(k, None),
            }
        }
        let _ = std::fs::remove_file(&log_target);

        let mut transcript = Vec::new();
        let ((), cap) = capture(|| scenario(api, &mut transcript));
        let log = std::fs::read(&log_target).unwrap_or_default();
        outcomes.push(Outcome {
            transcript,
            stdout: cap.stdout,
            stderr: cap.stderr,
            log,
        });
    }

    let (c, r) = (&outcomes[0], &outcomes[1]);
    let mut failures: Vec<String> = Vec::new();
    if c.transcript != r.transcript {
        failures.push(format!(
            "transcript mismatch:\n  C   : {:#?}\n  Rust: {:#?}",
            c.transcript, r.transcript
        ));
    }
    if c.stdout != r.stdout {
        failures.push(format!(
            "stdout mismatch:\n  C   : {:?}\n  Rust: {:?}",
            show(&c.stdout),
            show(&r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        failures.push(format!(
            "stderr mismatch:\n  C   : {:?}\n  Rust: {:?}",
            show(&c.stderr),
            show(&r.stderr)
        ));
    }
    if c.log != r.log {
        failures.push(format!(
            "log-file mismatch:\n  C   : {:?}\n  Rust: {:?}",
            show(&c.log),
            show(&r.log)
        ));
    }
    assert!(
        failures.is_empty(),
        "case `{case}` diverged:\n{}",
        failures.join("\n")
    );
}

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Convenience recorders
// ---------------------------------------------------------------------------

/// Appends a rendering of `*manager` (and of the initialised tasks) to `t`.
pub unsafe fn record_manager(t: &mut Vec<String>, manager: *const TaskManagerRaw) {
    if manager.is_null() {
        t.push("manager = NULL".into());
        return;
    }
    let m = &*manager;
    t.push(format!(
        "manager: max_tasks={} task_count={} tasks_null={}",
        m.max_tasks,
        m.task_count,
        m.tasks.is_null()
    ));
    if m.tasks.is_null() {
        return;
    }
    // Only the first `task_count` entries have been written; the rest is
    // uninitialised malloc memory in both implementations.
    for i in 0..m.task_count.max(0) {
        let task = &*m.tasks.add(i as usize);
        t.push(format!(
            "task[{i}] priority={} description={:?}",
            task.priority,
            show(&task.description)
        ));
    }
}

pub fn cstr(s: &[u8]) -> CString {
    CString::new(s).unwrap()
}
