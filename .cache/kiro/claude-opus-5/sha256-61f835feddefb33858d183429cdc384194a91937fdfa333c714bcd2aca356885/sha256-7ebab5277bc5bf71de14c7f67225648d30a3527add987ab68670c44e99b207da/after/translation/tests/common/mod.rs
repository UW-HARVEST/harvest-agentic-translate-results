#![allow(dead_code)]

//! Shared harness: loads the C reference `.so` and the Rust `.so` and provides
//! helpers for calling their exported symbols and capturing their stdout.

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type HashBytesFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type SipHashFn = unsafe extern "C" fn(c_int) -> ();

fn workspace_root() -> PathBuf {
    // translation/ -> parent is the working directory holding c_src/ and translation/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// Locate the C shared library produced by the CMake build.
/// `HARVEST_C_SO` overrides the path, which lets the same suite be run against the
/// reference compiled with different compilers/optimisation levels.
fn c_library_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locate the Rust cdylib. The test binary lives in `target/<profile>/deps/`, so the
/// sibling `target/<profile>/libsiphash_lib.so` is the artifact for the same profile
/// and feature set that is currently being tested.
fn rust_library_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let direct = dir.join("libsiphash_lib.so");
    if direct.exists() {
        return direct;
    }
    // Fall back to scanning the usual profile directories.
    for profile in ["debug", "release"] {
        let p = workspace_root()
            .join("translation")
            .join("target")
            .join(profile)
            .join("libsiphash_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libsiphash_lib.so not found next to the test binary ({}); run `cargo build` first",
        direct.display()
    );
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    c_path: PathBuf,
    rust_path: PathBuf,
}

impl Libs {
    pub fn c_path(&self) -> PathBuf {
        self.c_path.clone()
    }

    pub fn rust_path(&self) -> PathBuf {
        self.rust_path.clone()
    }

    pub fn hash_bytes(&self) -> (Symbol<'_, HashBytesFn>, Symbol<'_, HashBytesFn>) {
        unsafe {
            (
                self.c.get(b"stbds_hash_bytes\0").expect("C stbds_hash_bytes"),
                self.rust.get(b"stbds_hash_bytes\0").expect("Rust stbds_hash_bytes"),
            )
        }
    }

    pub fn siphash(&self) -> (Symbol<'_, SipHashFn>, Symbol<'_, SipHashFn>) {
        unsafe {
            (
                self.c.get(b"siphash\0").expect("C siphash"),
                self.rust.get(b"siphash\0").expect("Rust siphash"),
            )
        }
    }
}

/// Both libraries are dlopen'ed once and kept alive for the whole test binary.
/// `libloading` uses RTLD_LOCAL, so the two identically-named symbol sets do not
/// collide: each `get` resolves within its own handle.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        unsafe {
            Libs {
                c: Library::new(&c_path).unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
                rust: Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display())),
                c_path,
                rust_path,
            }
        }
    })
}

/// Redirect fd 1 to a temporary file, run `f`, then restore. Captures output written
/// by C `printf` from either library (they share the process's libc stdio).
///
/// Serialised on a process-global lock: fd 1 is process-wide state and cargo runs
/// tests on multiple threads.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "siphash-capture-{}-{}-{:?}.txt",
        std::process::id(),
        tag,
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = {
        use std::os::fd::AsRawFd;
        file.as_raw_fd()
    };

    let out = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        std::fs::read(&path).expect("read capture file")
    };
    drop(file);
    let _ = std::fs::remove_file(&path);
    out
}

/// Deterministic PRNG (splitmix64) so failures are reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            let n = chunk.len();
            chunk.copy_from_slice(&v[..n]);
        }
    }
}

/// Format a byte slice compactly for assertion messages.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

