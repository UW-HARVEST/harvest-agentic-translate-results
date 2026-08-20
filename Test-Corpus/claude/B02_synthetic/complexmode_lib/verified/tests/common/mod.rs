// Shared harness for the C-vs-Rust differential tests.
//
// Both implementations are loaded as *shared objects* through `libloading` and
// every call goes through `dlsym`-resolved `extern "C"` function pointers, so
// the Rust `#[no_mangle]` export wrappers are exercised exactly the way an
// external C consumer would exercise them.  No Rust function of the crate is
// ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits used by the harness itself (never the library under test)
// ---------------------------------------------------------------------------

extern "C" {
    fn free(p: *mut c_void);
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

/// `free()` from the process C runtime — the very allocator both `.so`s
/// allocate from, so blocks handed back by either library can be released.
pub unsafe fn libc_free(p: *mut c_char) {
    free(p as *mut c_void)
}

/// Copy a NUL-terminated C string out of a raw pointer (bytes, no UTF-8
/// assumption).
pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let n = strlen(p);
    std::slice::from_raw_parts(p as *const u8, n).to_vec()
}

// ---------------------------------------------------------------------------
// FFI signatures of the 7 exported symbols
// ---------------------------------------------------------------------------

pub type FnCreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
pub type FnCheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnSafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnMultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
pub type FnCopyAndSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnCompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnComplexMode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// All exported entry points of one shared object.
pub struct Api {
    pub name: &'static str,
    pub create_result_string: FnCreateResultString,
    pub check_permissions: FnCheckPermissions,
    pub safe_add: FnSafeAdd,
    pub multiply_with_log: FnMultiplyWithLog,
    pub copy_and_sum: FnCopyAndSum,
    pub compare_operations: FnCompareOperations,
    pub complexmode: FnComplexMode,
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test binary
/// (`target/<profile>/deps/<name>-<hash>`), so it follows debug/release.
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent() // deps/
        .and_then(Path::parent) // <profile>/
        .expect("profile dir")
        .to_path_buf()
}

/// The C reference object.  `CDIFF_C_SO` overrides the default location so the
/// same suite can be re-run against a C library built with different compiler
/// settings (see `check_all_features.sh`, which also builds it at `-O2`/`-O3`).
pub fn c_so_path() -> PathBuf {
    match std::env::var_os("CDIFF_C_SO") {
        Some(p) => PathBuf::from(p),
        None => manifest_dir().join("c_src/build/libtranslated_rust.so"),
    }
}

/// The Rust object, from the profile the test binary itself was built in.
/// `CDIFF_RUST_SO` overrides it.
pub fn rust_so_path() -> PathBuf {
    match std::env::var_os("CDIFF_RUST_SO") {
        Some(p) => PathBuf::from(p),
        None => target_profile_dir().join("libcomplexmode_lib.so"),
    }
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("symbol {:?} missing: {e}", String::from_utf8_lossy(name)));
    *s
}

/// `cargo test` does not rebuild a `crate-type = ["cdylib"]` library (nothing
/// links it), and the C reference object is built by CMake, so make sure both
/// shared objects exist before the first `dlopen`.
pub fn ensure_built(path: &Path) {
    if path.exists() {
        return;
    }
    for var in ["CDIFF_C_SO", "CDIFF_RUST_SO"] {
        if let Some(p) = std::env::var_os(var) {
            assert!(
                Path::new(&p) != path,
                "{var} points at {} which does not exist",
                path.display()
            );
        }
    }
    if path == c_so_path() {
        let build = manifest_dir().join("c_src/build");
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let ok = std::process::Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            && std::process::Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        assert!(
            ok && path.exists(),
            "could not build the C reference library.  Run:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
    } else {
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.arg("build").current_dir(manifest_dir());
        if target_profile_dir().file_name().and_then(|s| s.to_str()) == Some("release") {
            cmd.arg("--release");
        }
        let ok = cmd.status().map(|s| s.success()).unwrap_or(false);
        assert!(
            ok && path.exists(),
            "could not build the Rust cdylib {} — run `cargo build` first",
            path.display()
        );
    }
}

fn load(path: &Path, name: &'static str) -> Api {
    ensure_built(path);
    assert!(
        path.exists(),
        "shared object {} not found.\n\
         Build the C side with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Build the Rust side with: cargo build",
        path.display()
    );
    unsafe {
        // Leaked on purpose: the resolved function pointers must stay valid for
        // the whole process lifetime.
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
        ));
        Api {
            name,
            create_result_string: sym(lib, b"create_result_string\0"),
            check_permissions: sym(lib, b"check_permissions\0"),
            safe_add: sym(lib, b"safe_add\0"),
            multiply_with_log: sym(lib, b"multiply_with_log\0"),
            copy_and_sum: sym(lib, b"copy_and_sum\0"),
            compare_operations: sym(lib, b"compare_operations\0"),
            complexmode: sym(lib, b"complexmode\0"),
        }
    }
}

pub fn c_api() -> &'static Api {
    static C: OnceLock<Api> = OnceLock::new();
    C.get_or_init(|| load(&c_so_path(), "C"))
}

pub fn rust_api() -> &'static Api {
    static R: OnceLock<Api> = OnceLock::new();
    R.get_or_init(|| load(&rust_so_path(), "Rust"))
}

/// `(c, rust)` pair, both dlopen'ed with RTLD_LOCAL so the identical symbol
/// names in the two objects cannot shadow each other.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// stdout capture (fd level, so it catches printf/puts inside the .so)
// ---------------------------------------------------------------------------

fn stdout_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// fd 1 is a process-wide resource, so concurrent tests would scribble into
/// each other's capture windows.  `.cargo/config.toml` sets
/// `RUST_TEST_THREADS=1`; bail out loudly if that got lost.
fn require_single_threaded_harness() {
    static OK: OnceLock<bool> = OnceLock::new();
    let ok = *OK.get_or_init(|| {
        std::env::var("RUST_TEST_THREADS").as_deref() == Ok("1")
            || std::env::args().any(|a| a == "--test-threads=1" || a == "-j1")
    });
    assert!(
        ok,
        "these differential tests capture fd 1 and must run single-threaded; \
         run `cargo test -- --test-threads=1` or keep RUST_TEST_THREADS=1 \
         (see .cargo/config.toml)"
    );
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// `(result, bytes_written_to_stdout)`.
///
/// `fflush(NULL)` is issued on both sides of the redirection so the C runtime's
/// `stdout` buffer is empty before and fully drained after the call.  Do not
/// use Rust's `print!` inside `f` — that buffer is separate.
pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::unix::io::AsRawFd;

    let _guard = stdout_lock();
    require_single_threaded_harness();
    // Drain libtest's own progress text ("test foo ... ") out of Rust's stdout
    // buffer, so it cannot be flushed into our redirected fd 1 later on.
    let _ = std::io::stdout().flush();

    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "cdiff-{}-{}.out",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));

    let mut file = std::fs::File::options()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp capture file");

    let result;
    let mut buf = Vec::new();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        result = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    file.seek(SeekFrom::Start(0)).expect("seek");
    file.read_to_end(&mut buf).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&path);
    (result, buf)
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Runs the same closure against the C api and the Rust api and asserts that
/// the returned value *and* the stdout bytes are identical.
pub fn diff<R: PartialEq + std::fmt::Debug, F: Fn(&'static Api) -> R>(ctx: &str, f: F) -> R {
    let (c, r) = both();
    let (cv, cout) = capture(|| f(c));
    let (rv, rout) = capture(|| f(r));
    assert_eq!(cv, rv, "return value mismatch [{ctx}]");
    assert_eq!(
        cout,
        rout,
        "stdout mismatch [{ctx}]\n  C   : {}\n  Rust: {}",
        show(&cout),
        show(&rout)
    );
    cv
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep failures reproducible
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
    /// Full-range `i32`, so overflow-prone operands appear naturally.
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Small signed value in `-bound..=bound`.
    pub fn small(&mut self, bound: i32) -> i32 {
        let span = (bound as i64) * 2 + 1;
        ((self.next_u64() % span as u64) as i64 - bound as i64) as i32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// Random NUL-free byte string of the given length.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len + 1);
        for _ in 0..len {
            let mut b = self.byte();
            if b == 0 {
                b = 1;
            }
            v.push(b);
        }
        v.push(0);
        v
    }
}

/// Interesting `i32` boundary values used across the tables.
pub const EDGE_I32: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    7,
    -7,
    255,
    256,
    -256,
    65535,
    65536,
    -65536,
    46341,
    -46341,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    1 << 30,
    -(1 << 30),
];

/// Permission words the C code cares about (`READ_PERM`, `WRITE_PERM`,
/// `EXEC_PERM`, the hard-coded `0644`, …) plus sign-bit boundaries.
pub const EDGE_PERMS: &[c_int] = &[
    0, 0o100, 0o200, 0o300, 0o400, 0o500, 0o600, 0o644, 0o700, 0o777, 0o77, 0o1000, -1, i32::MIN,
    i32::MAX,
];

// ---------------------------------------------------------------------------
// helper-process plumbing
//
// Some rows can only be reached by changing the *process* environment
// (LD_PRELOAD malloc fault injection for the `malloc() == NULL` branches,
// MALLOC_PERTURB_ for a non-zero heap), which is impossible from inside a
// running test.  Those rows are driven through the `fault_child` example, which
// loads both `.so`s with libloading exactly like the in-process tests do.
// ---------------------------------------------------------------------------

/// Builds (once) the LD_PRELOAD malloc interposer from `tests/fixtures/`.
pub fn preload_so() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = manifest_dir().join("tests/fixtures/fail_malloc.c");
        assert!(src.exists(), "missing {}", src.display());
        let out = target_profile_dir().join("fail_malloc_preload.so");
        let st = std::process::Command::new("gcc")
            .args(["-O2", "-shared", "-fPIC", "-o"])
            .arg(&out)
            .arg(&src)
            .status()
            .expect("gcc is required to build the malloc fault injector");
        assert!(st.success(), "gcc failed building {}", src.display());
        out
    })
}

pub fn fault_child_bin() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let p = target_profile_dir().join("examples/fault_child");
        // `cargo test` does not build (or refresh) examples, so always ask cargo
        // to bring it up to date — a stale helper would silently test the wrong
        // code.  This is a no-op once it is current.
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args(["build", "--example", "fault_child"])
            .current_dir(manifest_dir());
        if target_profile_dir().file_name().and_then(|s| s.to_str()) == Some("release") {
            cmd.arg("--release");
        }
        let st = cmd.status().expect("cargo build --example fault_child");
        assert!(st.success(), "could not build the fault_child example");
        assert!(p.exists(), "missing helper binary {}", p.display());
        p
    })
}

/// Runs the helper and returns its stdout.  `envs` adds environment variables
/// (e.g. `MALLOC_PERTURB_`); `preload` decides whether the malloc interposer is
/// injected.
pub fn run_child(scenario: &str, fail_size: u64, envs: &[(&str, &str)], preload: bool) -> String {
    ensure_built(&c_so_path());
    ensure_built(&rust_so_path());
    let mut cmd = std::process::Command::new(fault_child_bin());
    cmd.arg(c_so_path())
        .arg(rust_so_path())
        .arg(scenario)
        .arg(fail_size.to_string());
    if preload {
        cmd.env("LD_PRELOAD", preload_so());
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("spawn fault_child");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "fault_child {scenario}/{fail_size} {envs:?} failed: {:?}\nstdout:\n{stdout}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("====END===="),
        "truncated report for {scenario}/{fail_size}:\n{stdout}"
    );
    stdout
}

/// Malloc-fault-injection run (ERRORS.md rows 1, 2, 5, 9, 11).
pub fn child_report(scenario: &str, fail_size: u64) -> String {
    run_child(scenario, fail_size, &[], true)
}

pub fn split_report(report: &str) -> (&str, &str) {
    let (_, rest) = report
        .split_once("====C====\n")
        .unwrap_or_else(|| panic!("no C section:\n{report}"));
    let (c, rest) = rest
        .split_once("====RUST====\n")
        .unwrap_or_else(|| panic!("no RUST section:\n{report}"));
    let (r, _) = rest
        .split_once("====END====\n")
        .unwrap_or_else(|| panic!("no END marker:\n{report}"));
    (c, r)
}

pub fn c_section(report: &str) -> &str {
    split_report(report).0
}

pub fn assert_sections_match(report: &str) {
    let (c, r) = split_report(report);
    if c != r {
        // Point at the first differing line to keep the failure readable.
        let first = c
            .lines()
            .zip(r.lines())
            .find(|(a, b)| a != b)
            .map(|(a, b)| format!("\n  first differing line:\n    C   : {a}\n    Rust: {b}"))
            .unwrap_or_default();
        panic!(
            "C and Rust sections differ ({} vs {} bytes){first}",
            c.len(),
            r.len()
        );
    }
}

pub fn assert_c_section_contains(report: &str, needles: &[&str]) {
    let c = c_section(report);
    for n in needles {
        assert!(
            c.contains(n),
            "expected {n:?} in the C section (fault injection did not fire?):\n{report}"
        );
    }
}
