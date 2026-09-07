//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects with `libloading` and
//! invoked purely through their exported C symbols -- the Rust crate is never
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits used to capture what the libraries write to fd 1 via `printf`
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, including the
    /// `stdout` shared by both dlopen'd libraries.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Flush *everything* that could still be sitting in a userspace buffer aimed
/// at fd 1: libc's stdio streams (used by both `.so`s) **and** Rust's own
/// `std::io::Stdout` buffer (used by libtest for its progress output). Missing
/// the latter would let libtest's pending bytes land inside a capture window.
fn flush_all() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<repo>/c_src/build/libdriver.so`, built on demand with cmake.
fn c_so_path() -> PathBuf {
    let repo = manifest_dir().parent().unwrap().to_path_buf();
    let c_src = repo.join("c_src");
    let build = c_src.join("build");
    let so = build.join("libdriver.so");
    if !so.exists() {
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let ok = std::process::Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "cmake configure of c_src failed");
        let ok = std::process::Command::new("cmake")
            .arg("--build")
            .arg(".")
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "cmake build of c_src failed");
    }
    assert!(so.exists(), "missing C shared object: {}", so.display());
    so
}

/// `target/<profile>/` for the currently running test binary.
pub fn profile_dir() -> PathBuf {
    // current_exe() == target/<profile>/deps/<testbin>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn is_release() -> bool {
    profile_dir().file_name().map(|s| s == "release").unwrap_or(false)
}

/// The `cdylib` under test. `cargo test` does not itself emit the cdylib
/// artifact (the crate's only `crate-type` is `cdylib`, which the test harness
/// does not link), so build it on demand into a dedicated target directory.
/// The `.so` is then dlopen'd exactly as an external C consumer would.
///
/// The artifact is ALWAYS rebuilt here rather than reusing a possibly stale
/// `target/<profile>/libdriver.so` left behind by an earlier `cargo build`:
/// `cargo test` does not refresh that file, and testing a stale `.so` would
/// silently verify the wrong code.
pub fn rust_so_path() -> PathBuf {
    let target = manifest_dir().join("target/ffi-so");
    let profile = if is_release() { "release" } else { "debug" };
    let so = target.join(profile).join("libdriver.so");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--offline")
        .arg("--lib")
        .current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &target)
        // Do not inherit the outer `cargo test` invocation's state.
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_MAKEFLAGS");
    if is_release() {
        cmd.arg("--release");
    }
    let status = cmd.status().expect("spawn cargo build for the cdylib");
    assert!(status.success(), "`cargo build --lib` for the cdylib failed");
    assert!(
        so.exists(),
        "cargo build did not produce {}",
        so.display()
    );
    so
}

fn c_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| unsafe { Library::new(c_so_path()).expect("dlopen C libdriver.so") })
}

fn rust_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| unsafe { Library::new(rust_so_path()).expect("dlopen Rust libdriver.so") })
}

// ---------------------------------------------------------------------------
// The exported surface, as function pointers resolved out of each .so
// ---------------------------------------------------------------------------

/// `void f(char)` -- the declared signature.
pub type FnChar = unsafe extern "C" fn(c_char);
/// `void f(char)` viewed as `void f(int)`, to push over-wide values across the
/// ABI boundary (the analogue of an out-of-range enum value).
pub type FnInt = unsafe extern "C" fn(c_int);

/// Which of the two implementations to exercise.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
    fn lib(self) -> &'static Library {
        match self {
            Impl::C => c_lib(),
            Impl::Rust => rust_lib(),
        }
    }
}

/// Every symbol in `SYMBOLS.md`, in both ABI views.
pub const SYMBOLS: [&str; 2] = ["printHexCharLine", "driver"];

pub fn sym_char(imp: Impl, name: &str) -> FnChar {
    let lib = imp.lib();
    unsafe {
        let s: Symbol<FnChar> = lib
            .get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("{} .so is missing symbol `{name}`: {e}", imp.name()));
        *s
    }
}

pub fn sym_int(imp: Impl, name: &str) -> FnInt {
    let lib = imp.lib();
    unsafe {
        let s: Symbol<FnInt> = lib
            .get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("{} .so is missing symbol `{name}`: {e}", imp.name()));
        *s
    }
}

/// True iff the symbol is present in that `.so` at all.
pub fn has_symbol(imp: Impl, name: &str) -> bool {
    unsafe {
        imp.lib()
            .get::<FnChar>(format!("{name}\0").as_bytes())
            .is_ok()
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Redirect fd 1 into a temp file, run `f`, flush every stdio stream, restore
/// fd 1 and return the raw bytes that were written.
///
/// `stdout` is a regular file while `f` runs, so it is *fully buffered*: this
/// deliberately exercises configuration axis A8 (nothing is flushed until we
/// call `fflush(NULL)`), which is the shape in which a buffering difference
/// between the two libraries would show up.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::AsRawFd;

    // Serialise: fd 1 is process-global state.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver-diff-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture temp file");

    flush_all();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    flush_all();
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    if let Err(p) = res {
        let _ = std::fs::remove_file(&path);
        std::panic::resume_unwind(p);
    }

    let mut buf = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek");
    file.read_to_end(&mut buf).expect("read capture");
    let _ = std::fs::remove_file(&path);
    buf
}

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Run the same `char` argument through the C and the Rust export of `name`
/// and assert the captured stdout bytes are identical.
pub fn assert_same_char(name: &str, arg: c_char) {
    let c = sym_char(Impl::C, name);
    let r = sym_char(Impl::Rust, name);
    let out_c = capture(|| unsafe { c(arg) });
    let out_r = capture(|| unsafe { r(arg) });
    assert_eq!(
        out_c,
        out_r,
        "{name}({arg}i8 / 0x{:02x}): C printed \"{}\" but Rust printed \"{}\"",
        arg as u8,
        show(&out_c),
        show(&out_r)
    );
}

/// Same, but pushing a full-width `int` across the ABI boundary.
pub fn assert_same_int(name: &str, arg: c_int) {
    let c = sym_int(Impl::C, name);
    let r = sym_int(Impl::Rust, name);
    let out_c = capture(|| unsafe { c(arg) });
    let out_r = capture(|| unsafe { r(arg) });
    assert_eq!(
        out_c,
        out_r,
        "{name}({arg}i32 / 0x{arg:08x}): C printed \"{}\" but Rust printed \"{}\"",
        show(&out_c),
        show(&out_r)
    );
}

/// Assert both implementations produce identical output for a whole *sequence*
/// of calls captured as one byte stream (axis A7/A8).
pub fn assert_same_sequence(calls: &[(&str, c_char)]) {
    let run = |imp: Impl| -> Vec<u8> {
        let fns: Vec<(FnChar, c_char)> = calls
            .iter()
            .map(|&(n, a)| (sym_char(imp, n), a))
            .collect();
        capture(|| {
            for (f, a) in &fns {
                unsafe { f(*a) }
            }
        })
    };
    let out_c = run(Impl::C);
    let out_r = run(Impl::Rust);
    assert_eq!(
        out_c,
        out_r,
        "sequence of {} calls diverged:\n  C   = \"{}\"\n  Rust= \"{}\"",
        calls.len(),
        show(&out_c),
        show(&out_r)
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed -> reproducible property-style testing)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        // SplitMix64
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
    /// Uniform in `lo..=hi` (inclusive), for `u8` ranges.
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u8
    }
}

/// Number of randomized samples used per `CONFIGS.md` row.
pub const SAMPLES: usize = 64;
