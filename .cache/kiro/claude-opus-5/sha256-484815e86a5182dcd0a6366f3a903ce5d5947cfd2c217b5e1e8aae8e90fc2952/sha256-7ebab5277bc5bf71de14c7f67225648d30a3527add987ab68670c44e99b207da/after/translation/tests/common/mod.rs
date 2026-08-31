//! Shared harness: builds the C reference shared library and the Rust
//! `cdylib`, loads both through `libloading`, and captures everything each
//! library writes to `stdout` so the two can be compared byte-for-byte.
//!
//! Nothing in here calls a Rust function directly: `driver` is always reached
//! through the `#[no_mangle]` export of the built `.so`, exactly as an external
//! C caller would reach it.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Guards every process-global side effect in this harness: redirecting fd 1
/// and spawning child processes. Both must be serialised, because a child
/// spawned while fd 1 is redirected would inherit the capture file.
///
/// Poisoning is deliberately ignored: a failed assertion in one test must not
/// turn every other test into an unrelated `PoisonError`.
fn serial_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_serial() -> std::sync::MutexGuard<'static, ()> {
    match serial_lock().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Workspace root (the directory holding `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Spawn a child process under the global serial lock.
///
/// Callers must not already hold the lock; the harness only reaches `run` from
/// one-time initialisation, never from inside a capture.
fn run(cmd: &mut Command) -> String {
    let _guard = lock_serial();
    let rendered = format!("{cmd:?}");
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {rendered}: {e}"));
    if !out.status.success() {
        panic!(
            "command failed: {rendered}\nstatus: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Build `c_src` as a shared library and return the path to `libdriver.so`.
///
/// `c_src/` itself is never modified: the CMake binary directory lives under
/// `translation/target/`.
pub fn c_library() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let root = repo_root();
        let c_src = root.join("c_src");
        let build_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/c_reference");
        std::fs::create_dir_all(&build_dir).expect("create C build dir");

        let cmake_ok = {
            let _guard = lock_serial();
            Command::new("cmake")
                .arg("-S")
                .arg(&c_src)
                .arg("-B")
                .arg(&build_dir)
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };

        if cmake_ok {
            run(Command::new("cmake").arg("--build").arg(&build_dir));
            let so = build_dir.join("libdriver.so");
            if so.is_file() {
                return so;
            }
        }

        // Fallback: compile directly, matching CMake's include paths.
        let so = build_dir.join("libdriver.so");
        run(Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-I")
            .arg(c_src.join("include"))
            .arg("-I")
            .arg(c_src.join("src"))
            .arg(c_src.join("src/driver.c"))
            .arg("-o")
            .arg(&so));
        so
    })
    .as_path()
}

/// Build the crate's own `cdylib` and return the path to it.
///
/// A dedicated `CARGO_TARGET_DIR` is used so this nested build never contends
/// with the `cargo test` invocation that spawned it.
///
/// `DRIVER_TEST_FEATURES` selects a feature combination (with
/// `--no-default-features`) and `DRIVER_TEST_PROFILE=release` selects the
/// release profile, so the same tests can be pointed at any build
/// configuration.
pub fn rust_library() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let profile = std::env::var("DRIVER_TEST_PROFILE").unwrap_or_else(|_| "dev".to_string());
        let target_dir = manifest_dir.join(format!("target/rust_cdylib_{profile}"));
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let mut cmd = Command::new(cargo);
        cmd.arg("build")
            .arg("--lib")
            .arg("--profile")
            .arg(&profile)
            .current_dir(&manifest_dir)
            .env("CARGO_TARGET_DIR", &target_dir)
            .env_remove("RUSTFLAGS");

        // Reproduce whatever feature selection the outer test run used.
        if let Ok(features) = std::env::var("DRIVER_TEST_FEATURES") {
            cmd.arg("--no-default-features");
            if !features.is_empty() {
                cmd.arg("--features").arg(features);
            }
        }
        run(&mut cmd);

        // `--profile dev` writes to `debug/`; every other profile uses its name.
        let subdir = if profile == "dev" { "debug" } else { &profile };
        let so = target_dir.join(subdir).join("libdriver.so");
        assert!(so.is_file(), "expected Rust cdylib at {}", so.display());
        so
    })
    .as_path()
}

/// Run `f` in a forked child whose fd 1 points at a fresh temporary file, and
/// return the bytes the child wrote.
///
/// Forking is what makes this reliable: fd 1 is process-global, so redirecting
/// it in-process races with libtest's own progress output ("test foo ... ok"),
/// which is written from a different thread and would be captured as if the
/// library had printed it. The parent's fd 1 is never touched here.
///
/// glibc's `fork` takes the stdio and malloc locks around the fork, so the
/// child may safely call `printf`. The child does inherit the parent's
/// *unflushed* `stdout` buffer, so it flushes that into `/dev/null` before
/// pointing fd 1 at the capture file.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let tmp = std::env::temp_dir().join(format!(
        "driver-capture-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));

    // Open both files in the parent so the child performs no allocation.
    let out_file = std::fs::File::create(&tmp).expect("create capture file");
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("open /dev/null");

    let (out_fd, null_fd) = {
        use std::os::fd::AsRawFd;
        (out_file.as_raw_fd(), devnull.as_raw_fd())
    };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        // Child: discard anything inherited in stdout's buffer, then capture.
        unsafe {
            dup2(null_fd, 1);
            fflush(std::ptr::null_mut());
            dup2(out_fd, 1);
        }
        f();
        unsafe {
            fflush(std::ptr::null_mut());
            _exit(0);
        }
    }

    let mut status: c_int = 0;
    let waited = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(waited, pid, "waitpid failed");
    assert!(
        // WIFEXITED(status) && WEXITSTATUS(status) == 0
        status & 0x7f == 0 && (status >> 8) & 0xff == 0,
        "child terminated abnormally (raw status {status:#x})"
    );

    drop(out_file);
    drop(devnull);

    let bytes = std::fs::read(&tmp).expect("read capture file");
    let _ = std::fs::remove_file(&tmp);
    bytes
}

/// A loaded library plus the resolved `driver` symbol.
pub struct Driver {
    _lib: libloading::Library,
    driver: unsafe extern "C" fn(c_char),
}

impl Driver {
    pub fn load(path: &Path) -> Self {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            let sym: libloading::Symbol<unsafe extern "C" fn(c_char)> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display()));
            let driver = *sym;
            Driver { _lib: lib, driver }
        }
    }

    /// Call the exported `driver` and return exactly what it printed.
    pub fn call(&self, c: c_char) -> Vec<u8> {
        capture_stdout(|| unsafe { (self.driver)(c) })
    }
}

/// Both implementations, loaded once.
pub struct Pair {
    pub c: Driver,
    pub rust: Driver,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Driver::load(c_library()),
        rust: Driver::load(rust_library()),
    })
}

/// Render bytes for assertion messages, escaping anything non-printable.
pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// `nm -D --defined-only` symbol names for a library.
pub fn exported_symbols(path: &Path) -> Vec<String> {
    let out = run(Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path));
    let mut names: Vec<String> = out
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2).map(str::to_string))
        .collect();
    names.sort();
    names.dedup();
    names
}
