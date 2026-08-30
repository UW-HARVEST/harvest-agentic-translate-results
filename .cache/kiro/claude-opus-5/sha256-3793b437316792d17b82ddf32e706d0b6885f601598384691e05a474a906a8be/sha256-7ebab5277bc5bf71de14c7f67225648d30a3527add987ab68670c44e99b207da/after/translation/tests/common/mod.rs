//! Shared harness for the differential tests.
//!
//! Both the C `libSieve.so` and the Rust `libSieve.so` are loaded with
//! `libloading` and exercised strictly through their exported `sieve` symbol,
//! so the `#[no_mangle]` wrapper is part of what is under test.
//!
//! `sieve` communicates only via `printf`, so the harness captures file
//! descriptor 1 around each call and compares the resulting bytes.

use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open stdio stream in the process, which is
    /// what both libraries' `printf` calls write into.
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type SieveFn = unsafe extern "C" fn(c_int);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library produced by `c_src/CMakeLists.txt`, building it
/// on demand so `cargo test` works from a clean checkout.
pub fn c_lib_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = manifest_dir().join("../c_src");
        let build = c_src.join("build");
        let lib = build.join("libSieve.so");
        if !lib.exists() {
            std::fs::create_dir_all(&build).expect("failed to create c_src/build");
            run(
                Command::new("cmake")
                    .current_dir(&build)
                    .arg("..")
                    .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON"),
                "cmake configure",
            );
            run(
                Command::new("cmake").current_dir(&build).args(["--build", "."]),
                "cmake build",
            );
        }
        assert!(
            lib.exists(),
            "C shared library still missing at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            lib.display()
        );
        lib
    })
    .clone()
}

/// Path to the Rust `cdylib` under test.
///
/// `cargo test` does **not** produce `target/<profile>/libSieve.so` on its own:
/// an integration test does not link the `cdylib`, so no such artifact is built
/// and a stale one from an earlier `cargo build` would be loaded instead. The
/// harness therefore builds the library itself, into a dedicated target
/// directory so it cannot collide with the parent cargo invocation's lock.
///
/// Feature flags are forwarded through `SIEVE_TEST_CARGO_ARGS` (space
/// separated), letting the same tests run against any feature combination:
///
/// ```text
/// SIEVE_TEST_CARGO_ARGS="--no-default-features --features foo" \
///     cargo test --no-default-features --features foo
/// ```
pub fn rust_lib_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let target_dir = manifest_dir().join("target/under-test");
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };

        let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&target_dir);
        if profile == "release" {
            cmd.arg("--release");
        }
        if let Ok(extra) = std::env::var("SIEVE_TEST_CARGO_ARGS") {
            cmd.args(extra.split_whitespace());
        }
        cmd.env_remove("CARGO_BUILD_TARGET_DIR");
        run(&mut cmd, "cargo build --lib (library under test)");

        let lib = target_dir.join(profile).join("libSieve.so");
        assert!(
            lib.exists(),
            "expected the Rust cdylib at {} after building it",
            lib.display()
        );
        lib
    })
    .clone()
}

fn run(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    assert!(
        out.status.success(),
        "{what} failed ({}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn load() -> Self {
        unsafe {
            Libs {
                c: Library::new(c_lib_path()).expect("failed to load C libSieve.so"),
                rust: Library::new(rust_lib_path()).expect("failed to load Rust libSieve.so"),
            }
        }
    }

    pub fn c_sieve(&self) -> Symbol<'_, SieveFn> {
        unsafe { self.c.get(b"sieve\0").expect("C .so does not export `sieve`") }
    }

    pub fn rust_sieve(&self) -> Symbol<'_, SieveFn> {
        unsafe {
            self.rust
                .get(b"sieve\0")
                .expect("Rust .so does not export `sieve`")
        }
    }
}

/// Redirect fd 1 to a temporary file for the duration of `f` and return the
/// bytes written to it.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    // fd 1 is process-global, so only one capture may be active at a time even
    // though the test harness runs tests on multiple threads.
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "sieve-capture-{}-{}.out",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let mut tmp = File::options()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("failed to create capture file");

    let bytes = unsafe {
        // Flush anything already buffered so it is not misattributed: Rust's
        // own `stdout` buffer first, then every libc stdio stream.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        f();

        // The redirected fd is a regular file, so stdio is fully buffered:
        // flush before restoring or output would be lost/reordered.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "failed to restore stdout");
        close(saved);

        let mut buf = Vec::new();
        tmp.seek(SeekFrom::Start(0)).expect("seek failed");
        tmp.read_to_end(&mut buf).expect("read failed");
        buf
    };

    drop(tmp);
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Run `sieve(val)` in both libraries and assert the emitted bytes are equal.
pub fn assert_same(libs: &Libs, val: i32) {
    let c = libs.c_sieve();
    let rust = libs.rust_sieve();

    let c_out = capture_stdout(|| unsafe { c(val) });
    let rust_out = capture_stdout(|| unsafe { rust(val) });

    if c_out != rust_out {
        panic!(
            "sieve({val}) mismatch\n  C   ({} bytes): {:?}\n  Rust({} bytes): {:?}",
            c_out.len(),
            String::from_utf8_lossy(&c_out).chars().take(400).collect::<String>(),
            rust_out.len(),
            String::from_utf8_lossy(&rust_out).chars().take(400).collect::<String>(),
        );
    }
}
