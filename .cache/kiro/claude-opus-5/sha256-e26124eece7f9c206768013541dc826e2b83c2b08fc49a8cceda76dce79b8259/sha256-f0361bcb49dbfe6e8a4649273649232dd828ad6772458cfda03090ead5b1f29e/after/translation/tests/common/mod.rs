//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded **through `libloading`** from their
//! respective shared objects; no Rust function is ever called directly, so the
//! `#[no_mangle]` / `extern "C"` export wrappers are part of what is tested.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::Library;

/// Function-pointer table for one implementation (C or Rust).
#[derive(Clone, Copy)]
pub struct Api {
    pub name: &'static str,
    pub convert_double_to_int: unsafe extern "C" fn(f64) -> c_int,
    pub find_value_in_buffer: unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int,
    pub process_negation: unsafe extern "C" fn(c_int) -> c_int,
    pub create_numeric_buffer: unsafe extern "C" fn(*mut c_char, c_int, c_int),
    pub calculate_with_doubles: unsafe extern "C" fn(c_int, c_int, c_int) -> f64,
    pub doubleneg: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

fn workspace_root() -> PathBuf {
    // .../<root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // `cargo test` does not build a cdylib, so the `.so` comes from a prior
    // `cargo build [--release]`. Prefer the one matching the running test
    // binary's profile, then fall back to either profile.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe
            .ancestors()
            .find(|p| p.file_name().map(|n| n == "deps").unwrap_or(false))
            .and_then(|deps| deps.parent())
        {
            candidates.push(profile_dir.join("libdoubleneg_lib.so"));
        }
    }
    let target = workspace_root().join("translation/target");
    candidates.push(target.join("release/libdoubleneg_lib.so"));
    candidates.push(target.join("debug/libdoubleneg_lib.so"));

    let so = candidates.into_iter().find(|p| p.is_file()).unwrap_or_else(|| {
        panic!(
            "libdoubleneg_lib.so not found under {}. `cargo test` does not build \
             a cdylib — run `cargo build --release` first.",
            target.display()
        )
    });

    // Guard against testing a stale artifact: the `.so` must be at least as new
    // as every file in `src/`.
    if let Ok(so_mtime) = std::fs::metadata(&so).and_then(|m| m.modified()) {
        let src = workspace_root().join("translation/src");
        if let Ok(entries) = std::fs::read_dir(&src) {
            for e in entries.filter_map(|e| e.ok()) {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    assert!(
                        m <= so_mtime,
                        "{} is NEWER than {} — the tests would verify a stale \
                         build. Run `cargo build --release` and retry.",
                        e.path().display(),
                        so.display()
                    );
                }
            }
        }
    }
    so
}

unsafe fn load(path: &Path, name: &'static str) -> Api {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    macro_rules! sym {
        ($t:ty, $s:literal) => {{
            let s = unsafe { lib.get::<$t>(concat!($s, "\0").as_bytes()) }.unwrap_or_else(|e| {
                panic!("missing symbol {} in {}: {e}", $s, path.display())
            });
            *s
        }};
    }
    let api = Api {
        name,
        convert_double_to_int: sym!(unsafe extern "C" fn(f64) -> c_int, "convert_double_to_int"),
        find_value_in_buffer: sym!(
            unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int,
            "find_value_in_buffer"
        ),
        process_negation: sym!(unsafe extern "C" fn(c_int) -> c_int, "process_negation"),
        create_numeric_buffer: sym!(
            unsafe extern "C" fn(*mut c_char, c_int, c_int),
            "create_numeric_buffer"
        ),
        calculate_with_doubles: sym!(
            unsafe extern "C" fn(c_int, c_int, c_int) -> f64,
            "calculate_with_doubles"
        ),
        doubleneg: sym!(
            unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
            "doubleneg"
        ),
    };
    // Keep the library mapped for the whole process lifetime so the function
    // pointers stay valid.
    std::mem::forget(lib);
    api
}

/// `(c_api, rust_api)` — both loaded via `libloading`.
pub fn apis() -> (Api, Api) {
    static APIS: OnceLock<(Api, Api)> = OnceLock::new();
    *APIS.get_or_init(|| unsafe {
        (load(&find_c_so(), "C"), load(&find_rust_so(), "Rust"))
    })
}

// ---------------------------------------------------------------------------
// stdout capture (fd-level, so it catches the libc `printf` inside both .so's)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// fd 1 is process-global, so captures must be serialised.
pub fn stdout_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f`, capturing everything written to file descriptor 1, and return
/// `(return value, captured bytes)`.
///
/// fd 1 is process-global, so this is serialised on [`stdout_lock`] and fd 1 is
/// restored even if `f` unwinds. Callers must additionally guarantee that no
/// *other* writer touches fd 1 concurrently — in particular the libtest
/// harness prints its own progress lines to fd 1, which is why every
/// stdout-comparing row lives in a `harness = false` test target that runs
/// sequentially.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let _guard = stdout_lock().lock().unwrap_or_else(|e| e.into_inner());

    struct Restore(c_int);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                fflush(std::ptr::null_mut());
                dup2(self.0, 1);
                close(self.0);
            }
        }
    }

    let mut tmp = tempfile();
    unsafe { fflush(std::ptr::null_mut()) };
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    let restore = Restore(saved);
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let ret = f();

    drop(restore); // flushes and restores fd 1

    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read captured stdout");
    (ret, out)
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "dnv-capture-{}-{}.tmp",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create temp capture file");
    let _ = std::fs::remove_file(&path); // unlink; fd keeps it alive
    f
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[-range, range]`.
    pub fn small_i32(&mut self, range: i32) -> i32 {
        let span = (range as i64) * 2 + 1;
        ((self.next_u64() % span as u64) as i64 - range as i64) as i32
    }
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn range_usize(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + (self.next_u64() as usize) % (hi_inclusive - lo + 1)
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Compare two `f64` by raw bits so NaN payloads and `-0.0` are distinguished.
#[track_caller]
pub fn assert_f64_bits_eq(c: f64, r: f64, ctx: &str) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "f64 mismatch [{ctx}]: C = {c:?} (bits {:#018x}) vs Rust = {r:?} (bits {:#018x})",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn assert_bytes_eq(c: &[u8], r: &[u8], ctx: &str) {
    if c == r {
        return;
    }
    let pos = c.iter().zip(r).position(|(a, b)| a != b);
    panic!(
        "stdout mismatch [{ctx}]: first differing byte at {pos:?}; \
         C len {} Rust len {}\n--- C ---\n{}\n--- Rust ---\n{}",
        c.len(),
        r.len(),
        String::from_utf8_lossy(c),
        String::from_utf8_lossy(r),
    );
}
