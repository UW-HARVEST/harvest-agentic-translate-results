//! Shared harness for the C-vs-Rust differential tests.
//!
//! Provides
//!   * on-demand builds of the C artifacts (executable via CMake, shared
//!     object via `gcc -shared -fPIC`) — `c_src/` is never modified,
//!   * locators for the Rust artifacts produced by `cargo test`,
//!   * `run_exe` to drive a program end to end over stdin/stdout,
//!   * `capture_fd1` to capture whatever a called function writes to file
//!     descriptor 1, which is what makes FFI-level comparison of `driver`
//!     possible (the C side writes with `printf`, the Rust side with
//!     `io::stdout()`).

#![allow(dead_code)]

pub mod corpus;

use std::ffi::c_void;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(fd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut c_void) -> i32;
}

// ---------------------------------------------------------------------------
// paths
// ---------------------------------------------------------------------------

/// `target/<profile>/` — the directory holding the artifacts cargo just built.
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    exe.parent()
        .and_then(Path::parent)
        .expect("profile dir")
        .to_path_buf()
}

/// Crate root (the directory containing `Cargo.toml`).
pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust executable built from `src/main.rs`.
pub fn rust_exe() -> PathBuf {
    let p = profile_dir().join("driver");
    assert!(p.is_file(), "rust executable not found at {p:?}");
    p
}

/// The Rust `cdylib` built from `src/lib.rs`.
///
/// `cargo test` only needs the `rlib`, so the `cdylib` may be missing — or,
/// worse, left over from a run with a *different* feature set. Either way it
/// is (re)built here with the same profile and feature set as the running test
/// binary, so what we `dlopen` is exactly the library this configuration
/// produces. (Cargo has released its build lock by the time the test binaries
/// run, so the nested invocation is safe, and it is a no-op when up to date.)
pub fn rust_so() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let p = profile_dir().join("libdriver.so");
        let dir = profile_dir();
        let profile = dir
            .file_name()
            .and_then(|s| s.to_str())
            .expect("profile dir name");
        let target_dir = dir.parent().expect("target dir").to_path_buf();

        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.arg("build")
            .arg("--lib")
            .arg("--manifest-path")
            .arg(crate_root().join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target_dir)
            .arg("--no-default-features");
        if profile == "release" {
            cmd.arg("--release");
        }
        // Mirror the feature set this test binary was compiled with, so the
        // symbol surface of the loaded `.so` matches the assertions.
        if cfg!(feature = "c_main") {
            cmd.arg("--features").arg("c_main");
        }
        let out = cmd
            .stdout(Stdio::null())
            .output()
            .expect("run cargo build --lib");
        assert!(
            out.status.success(),
            "cargo build --lib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            p.is_file(),
            "rust cdylib still not found at {p:?} after `cargo build --lib`"
        );
        p
    })
    .clone()
}

fn scratch() -> PathBuf {
    let d = profile_dir().join("ctest");
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

// ---------------------------------------------------------------------------
// C artifact builds
// ---------------------------------------------------------------------------

fn build_once(final_name: &str, build: impl FnOnce(&Path)) -> PathBuf {
    let out = scratch().join(final_name);
    if out.is_file() {
        return out;
    }
    // Build to a process-unique temporary name, then rename atomically so
    // that concurrently running test binaries cannot observe a partial file.
    let tmp = scratch().join(format!("{final_name}.{}.tmp", std::process::id()));
    build(&tmp);
    assert!(tmp.is_file(), "C build produced nothing at {tmp:?}");
    // `rename` over an existing file is atomic; a lost race is harmless
    // because both racers produce identical bytes.
    let _ = std::fs::rename(&tmp, &out);
    assert!(out.is_file(), "C artifact missing at {out:?}");
    out
}

/// The C program, built exactly the way `c_src/CMakeLists.txt` specifies.
pub fn c_exe() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        build_once("c_driver", |tmp| {
            let build_dir = scratch().join(format!("cmake-{}", std::process::id()));
            std::fs::create_dir_all(&build_dir).unwrap();
            let st = Command::new("cmake")
                .arg("-S")
                .arg(crate_root().join("c_src"))
                .arg("-B")
                .arg(&build_dir)
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .stdout(Stdio::null())
                .status()
                .expect("run cmake configure");
            assert!(st.success(), "cmake configure failed");
            let st = Command::new("cmake")
                .arg("--build")
                .arg(&build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("run cmake build");
            assert!(st.success(), "cmake build failed");
            std::fs::copy(build_dir.join("driver"), tmp).expect("copy cmake output");
        })
    })
    .clone()
}

/// The same C translation unit compiled as a shared object, so that its
/// `driver` becomes a real dynamic export we can `dlopen` and call.
pub fn c_so() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        build_once("libcdriver.so", |tmp| {
            let st = Command::new("gcc")
                .arg("-shared")
                .arg("-fPIC")
                .arg("-O2")
                .arg("-o")
                .arg(tmp)
                .arg(crate_root().join("c_src/src/main.c"))
                .status()
                .expect("run gcc -shared");
            assert!(st.success(), "gcc -shared failed");
        })
    })
    .clone()
}

// ---------------------------------------------------------------------------
// process-level driving
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
pub struct Outcome {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} stdout={:?}",
            self.code,
            String::from_utf8_lossy(&self.stdout)
        )
    }
}

/// Feed `input` to `exe` on stdin and collect its stdout and exit status.
pub fn run_exe(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {exe:?}: {e}"));
    let mut stdin = child.stdin.take().unwrap();
    let data = input.to_vec();
    // Write on a helper thread: a large input can fill the pipe buffer before
    // the child has read any of it.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        // dropping `stdin` closes the pipe, giving the child EOF
    });
    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();
    Outcome {
        code: out.status.code(),
        stdout: out.stdout,
    }
}

/// Run both programs on the same bytes and assert byte-identical results.
#[track_caller]
pub fn assert_same(input: &[u8], label: &str) {
    let c = run_exe(&c_exe(), input);
    let r = run_exe(&rust_exe(), input);
    assert!(
        c == r,
        "divergence for {label}\n  input: {input:?}\n  C:    {c:?}\n  RUST: {r:?}"
    );
}

/// `assert_same` for every element of an iterator of byte strings.
///
/// Each input costs two process spawns, so the work is spread over a small
/// pool of threads; that keeps corpora of tens of thousands of inputs inside
/// the per-test time budget.
#[track_caller]
pub fn assert_same_all<I, T>(inputs: I, group: &str)
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    let c = c_exe();
    let r = rust_exe();
    let all: Vec<Vec<u8>> = inputs
        .into_iter()
        .map(|t| t.as_ref().to_vec())
        .collect();
    assert!(!all.is_empty(), "group {group} produced no inputs");
    let n = all.len();

    let threads = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
        .clamp(1, 16);
    let chunk = n.div_ceil(threads);

    let failures: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = all
            .chunks(chunk)
            .map(|part| {
                let c = c.clone();
                let r = r.clone();
                s.spawn(move || {
                    let mut local = Vec::new();
                    for input in part {
                        let co = run_exe(&c, input);
                        let ro = run_exe(&r, input);
                        if co != ro {
                            local.push(format!(
                                "  input {:?} (bytes {:?})\n    C:    {co:?}\n    RUST: {ro:?}",
                                String::from_utf8_lossy(input),
                                &input[..input.len().min(64)]
                            ));
                            if local.len() >= 10 {
                                break;
                            }
                        }
                    }
                    local
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("worker thread panicked"))
            .collect()
    });

    assert!(
        failures.is_empty(),
        "{} divergence(s) in group `{group}` (of {n} inputs):\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ---------------------------------------------------------------------------
// fd-1 capture, for FFI-level comparison
// ---------------------------------------------------------------------------

static CAP_SEQ: AtomicUsize = AtomicUsize::new(0);

/// File descriptor 1 is process-wide, so only one capture may be installed at
/// a time; libtest runs `#[test]` functions on parallel threads by default.
static CAP_LOCK: Mutex<()> = Mutex::new(());

/// Capture everything written to file descriptor 1 while `f` runs.
///
/// Both the Rust `io::stdout()` buffer and every C `FILE*` buffer are flushed
/// before the redirect is installed and again before it is removed, so the
/// captured bytes are exactly what `f` emitted — this is what lets the
/// `printf`-based C `driver` and the `io::stdout()`-based Rust `driver` be
/// compared directly.
pub fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let seq = CAP_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = scratch().join(format!("cap-{}-{seq}.bin", std::process::id()));

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let file = std::fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let mut buf = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut buf)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);

    match result {
        Ok(()) => buf,
        Err(p) => std::panic::resume_unwind(p),
    }
}

// ---------------------------------------------------------------------------
// symbol listing
// ---------------------------------------------------------------------------

/// Names of the symbols `nm -D --defined-only` reports for `obj`, excluding
/// linker/toolchain boilerplate that is not part of any API contract.
pub fn exported_symbols(obj: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(obj)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {obj:?}");
    let boilerplate = [
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "_IO_stdin_used",
        "__TMC_END__",
        "__data_start",
        "__dso_handle",
        "_DYNAMIC",
        "_GLOBAL_OFFSET_TABLE_",
    ];
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // skip weak/unique-object toolchain hooks
            if kind == "w" || kind == "V" || kind == "u" {
                return None;
            }
            if boilerplate.contains(&name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    v.sort();
    v.dedup();
    v
}
