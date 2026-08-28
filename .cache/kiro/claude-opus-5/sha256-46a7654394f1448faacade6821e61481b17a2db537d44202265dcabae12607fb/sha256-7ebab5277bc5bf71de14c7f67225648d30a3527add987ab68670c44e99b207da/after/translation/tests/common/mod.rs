//! Shared harness: locates and dlopen()s both the C reference `.so` and the
//! Rust `.so`, and captures stdout written by either of them.

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

// glibc exposes `stdout` as a data symbol.
extern "C" {
    static mut stdout: *mut c_void;
}

fn flush_stdout() {
    unsafe {
        fflush(stdout);
    }
}

/// Repository root (parent of the `translation/` crate directory).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn find_so(dir: &Path, hint: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<PathBuf> = None;
    for e in entries.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name()?.to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") && name.contains(hint) {
            best = Some(p);
        }
    }
    best
}

/// Path to the C reference shared library, building it on demand.
fn c_library_path() -> PathBuf {
    let root = repo_root();
    let build_dir = root.join("c_src").join("build");

    if let Some(p) = find_so(&build_dir, "") {
        return p;
    }

    // Not built yet: run cmake exactly as documented.
    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let ok = std::process::Command::new("cmake")
        .current_dir(&build_dir)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "cmake configure failed");
    let ok = std::process::Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", "."])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "cmake build failed");

    find_so(&build_dir, "").expect("C shared library not found in c_src/build")
}

/// Path to the Rust `cdylib`.
///
/// `cargo test` does not emit the `cdylib` artifact (the crate only declares
/// `crate-type = ["cdylib"]`, which integration tests cannot link against), so
/// the library is built on demand into a dedicated target directory. Features
/// can be forwarded through `RUST_SO_FEATURES` / `RUST_SO_NO_DEFAULT_FEATURES`
/// so the same harness covers every feature combination.
fn rust_library_path() -> PathBuf {
    static ONCE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ONCE.get_or_init(build_rust_library).clone()
}

fn build_rust_library() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target").join("so-build");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(&manifest)
        .arg("build")
        .arg("--lib")
        .arg("--release")
        .arg("--target-dir")
        .arg(&target_dir)
        // Avoid inheriting the outer test invocation's cargo state.
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTC_WRAPPER");

    if std::env::var("RUST_SO_NO_DEFAULT_FEATURES").is_ok() {
        cmd.arg("--no-default-features");
    }
    if let Ok(feats) = std::env::var("RUST_SO_FEATURES") {
        if !feats.trim().is_empty() {
            cmd.arg("--features").arg(feats);
        }
    }

    let out = cmd.output().expect("failed to spawn cargo build for cdylib");
    assert!(
        out.status.success(),
        "cargo build --lib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let release_dir = target_dir.join("release");
    find_so(&release_dir, "complexmode_lib").unwrap_or_else(|| {
        panic!(
            "libcomplexmode_lib.so not found under {}",
            release_dir.display()
        )
    })
}

/// The two implementations under comparison.
pub struct Pair {
    pub c: Library,
    pub rs: Library,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = c_library_path();
        let rs_path = rust_library_path();
        unsafe {
            Pair {
                c: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
                rs: Library::new(&rs_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display())),
            }
        }
    }

    pub fn sym<T>(&self, which: Side, name: &str) -> Symbol<'_, T> {
        let lib = match which {
            Side::C => &self.c,
            Side::Rust => &self.rs,
        };
        unsafe {
            lib.get(format!("{name}\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol `{name}` in {which:?} library: {e}"))
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Side {
    C,
    Rust,
}

/// Serializes fd-1 redirection: `cargo test` runs test functions on multiple
/// threads, but stdout is process-global.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Runs `f` with fd 1 redirected into a temporary file and returns everything
/// the callee wrote to stdout.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let seq = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!("cmode-capture-{}-{seq}.txt", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let file = std::fs::File::create(&path).expect("create capture file");
    let tmp_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    flush_stdout();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp_fd, 1) } >= 0, "dup2 failed");

    let out = f();

    flush_stdout();
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe {
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (out, bytes)
}

/// Reads a NUL-terminated C string into owned bytes (excluding the NUL).
pub unsafe fn c_str_bytes(p: *const c_char) -> Vec<u8> {
    let n = strlen(p);
    std::slice::from_raw_parts(p as *const u8, n).to_vec()
}

/// `free()` from the same libc both libraries allocate from.
pub unsafe fn c_free(p: *mut c_void) {
    free(p);
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}
