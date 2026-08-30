//! Shared harness: loads the C and Rust shared objects via `libloading` and
//! captures everything each one writes to file descriptor 1 so the two byte
//! streams can be compared exactly.

use std::ffi::{CString, c_char, c_int};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::Library;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library missing at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Locate the Rust cdylib produced by cargo for the profile under test,
/// falling back to any other profile directory that has one.
fn rust_so_path() -> PathBuf {
    let target = workspace_root().join("translation/target");
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Prefer the profile directory holding the current test executable.
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test>
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            candidates.push(profile_dir.join("libdriver.so"));
        }
    }
    for profile in ["debug", "release"] {
        candidates.push(target.join(profile).join("libdriver.so"));
    }

    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found. Tried: {:?}. Build it with `cargo build` / `cargo \
         build --release`.",
        candidates
    );
}

/// Which implementation a symbol should be resolved from.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

fn library(which: Impl) -> &'static Library {
    static C_LIB: OnceLock<Library> = OnceLock::new();
    static RUST_LIB: OnceLock<Library> = OnceLock::new();
    match which {
        Impl::C => C_LIB.get_or_init(|| unsafe {
            Library::new(c_so_path()).expect("failed to dlopen the C libdriver.so")
        }),
        Impl::Rust => RUST_LIB.get_or_init(|| unsafe {
            Library::new(rust_so_path()).expect("failed to dlopen the Rust libdriver.so")
        }),
    }
}

/// Resolve an exported symbol from one of the two shared objects. Panics with a
/// clear message when the symbol is absent, which is itself a test failure: the
/// Rust `.so` must export every symbol the C `.so` does.
pub fn sym<T>(which: Impl, name: &str) -> libloading::Symbol<'static, T> {
    let lib = library(which);
    let mut owned = name.as_bytes().to_vec();
    owned.push(0);
    unsafe { lib.get::<T>(&owned) }.unwrap_or_else(|e| {
        panic!("{:?} libdriver.so does not export `{}`: {}", which, name, e)
    })
}

/// Assert both shared objects export `name`, so symbol-level parity is covered
/// even for functions whose behaviour is exercised only indirectly.
pub fn assert_both_export(name: &str) {
    let _c = sym::<unsafe extern "C" fn()>(Impl::C, name);
    let _r = sym::<unsafe extern "C" fn()>(Impl::Rust, name);
}

/// Run `f` with file descriptor 1 redirected into a temporary file and return
/// the raw bytes written. C `stdio` buffers are flushed before and after the
/// swap so nothing leaks between captures; both libraries share the process's
/// single `stdout` FILE, so one `fflush(NULL)` covers them.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // fd 1 is process-global, so only one capture may be in flight at a time.
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    /// Restores fd 1 even if `f` panics, so a single failure cannot swallow all
    /// subsequent output.
    struct Restore(c_int);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                libc::fflush(std::ptr::null_mut());
                libc::dup2(self.0, 1);
                libc::close(self.0);
            }
        }
    }

    unsafe {
        libc::fflush(std::ptr::null_mut());

        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "driver-capture-{}-{:?}.out",
            std::process::id(),
            std::thread::current().id()
        ));
        let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

        let saved: c_int = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let tmp_fd: c_int = libc::open(
            cpath.as_ptr() as *const c_char,
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_int,
        );
        assert!(tmp_fd >= 0, "open() of capture file failed");

        assert!(libc::dup2(tmp_fd, 1) >= 0, "dup2 onto fd 1 failed");
        let restore = Restore(saved);

        f();

        drop(restore);

        let mut file = <std::fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(tmp_fd);
        file.seek(SeekFrom::Start(0)).expect("seek capture file");
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read capture file");
        drop(file);
        let _ = std::fs::remove_file(&path);
        buf
    }
}

/// Capture the output of `name(args...)` from both implementations and assert
/// the byte streams are identical.
pub fn assert_same_bytes(label: &str, c_run: impl FnOnce(), rust_run: impl FnOnce()) {
    let c_out = capture_stdout(c_run);
    let rust_out = capture_stdout(rust_run);
    assert_eq!(
        c_out,
        rust_out,
        "{label}: stdout mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
}
