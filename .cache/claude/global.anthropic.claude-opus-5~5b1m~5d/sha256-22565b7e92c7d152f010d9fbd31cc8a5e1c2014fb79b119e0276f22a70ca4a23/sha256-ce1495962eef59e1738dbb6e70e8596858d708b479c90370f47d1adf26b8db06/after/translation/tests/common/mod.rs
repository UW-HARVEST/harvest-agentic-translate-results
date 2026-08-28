// Shared differential-test harness.
//
// Both the C shared object and the Rust shared object are loaded with
// `libloading` and every call goes through the loaded `.so`'s exported symbol.
// Nothing in this crate is ever called directly, so the `#[no_mangle]`
// `extern "C"` wrappers are part of what is under test.
//
// The library under test reads the *process environment* and writes to the
// *process stdout/stderr*, both of which are global. All observation therefore
// happens under a single global lock, and stdout/stderr are captured by
// temporarily `dup2`-ing them onto temporary files.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for env control, fd redirection and crash-path forking.
// ---------------------------------------------------------------------------

extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

// ---------------------------------------------------------------------------
// The five exported entry points, as raw `extern "C"` function pointers.
//
// `struct ConfigFlags*` is modelled as `*mut u8` so the tests can control and
// inspect all four raw bytes of the bit-field allocation unit, including the
// three bytes the C code never touches.
// ---------------------------------------------------------------------------

pub type FnParseEnvNumeric = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
pub type FnInitConfigFromEnv = unsafe extern "C" fn(*mut u8);
pub type FnPerformOperation = unsafe extern "C" fn(c_int, c_int, *mut u8) -> c_int;
pub type FnApplyBitOperations = unsafe extern "C" fn(c_int, *mut u8) -> c_int;
pub type FnEnvy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub parse_env_numeric: FnParseEnvNumeric,
    pub init_config_from_env: FnInitConfigFromEnv,
    pub perform_operation: FnPerformOperation,
    pub apply_bit_operations: FnApplyBitOperations,
    pub envy: FnEnvy,
}

/// The four bytes of a `struct ConfigFlags`, with the C alignment (4).
#[repr(C, align(4))]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Flags4(pub [u8; 4]);

impl Flags4 {
    pub fn new(bytes: [u8; 4]) -> Flags4 {
        Flags4(bytes)
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
    /// Build a byte-0 value from the individual bit-fields, using the x86-64
    /// SysV little-endian layout the C compiler picks:
    /// bit0 verbose, bit1 debug, bit2 optimize, bit3 cache_enabled,
    /// bits4..6 log_level, bit7 reserved.
    pub fn from_fields(
        verbose: u8,
        debug: u8,
        optimize: u8,
        cache_enabled: u8,
        log_level: u8,
        reserved: u8,
    ) -> Flags4 {
        let b = (verbose & 1)
            | ((debug & 1) << 1)
            | ((optimize & 1) << 2)
            | ((cache_enabled & 1) << 3)
            | ((log_level & 7) << 4)
            | ((reserved & 1) << 7);
        Flags4([b, 0, 0, 0])
    }
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — the name is derived from the parent
/// directory by `c_src/CMakeLists.txt`, so glob for it instead of hardcoding.
///
/// `DIFFTEST_C_SO` overrides the location, which is how the suite is re-run
/// against a C reference built at a different optimisation level (gcc is free
/// to codegen bit-field stores differently at `-O2` than at `-O0`).
pub fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DIFFTEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.exists(),
            "DIFFTEST_C_SO points at {} which does not exist",
            p.display()
        );
        return p;
    }
    let build = manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src")
        .join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
            {
                found.push(p);
            }
        }
    }
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {} (build the C library first: \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .), found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// `target/<profile>/libenvy_lib.so`. The test executable lives in
/// `target/<profile>/deps/`, so the cdylib is one directory up.
///
/// IMPORTANT: `cargo test` builds the crate as an rlib for the test harness but
/// does **not** rebuild the `cdylib`. Testing whatever `.so` happens to be lying
/// in `target/` would silently verify a stale artifact, so this function first
/// rebuilds the library and then refuses to hand back a `.so` that is older than
/// `src/lib.rs`.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let deps = exe.parent().expect("deps dir");
        let profile_dir = deps.parent().expect("profile dir");
        let release = profile_dir
            .file_name()
            .map(|n| n == "release")
            .unwrap_or(false);

        // Rebuild the cdylib for the profile the tests are running under.
        let mut cmd = std::process::Command::new(
            std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()),
        );
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--lib")
            .arg("--offline");
        if release {
            cmd.arg("--release");
        }
        // Never inherit the harness's captured stdio into a build.
        match cmd.output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => panic!(
                "`cargo build --lib` failed while refreshing the cdylib:\n{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            Err(e) => panic!("could not run `cargo build --lib` to refresh the cdylib: {e}"),
        }

        let candidates = [profile_dir.join("libenvy_lib.so"), deps.join("libenvy_lib.so")];
        let so = candidates
            .iter()
            .find(|c| c.exists())
            .unwrap_or_else(|| {
                panic!("libenvy_lib.so not found in {candidates:?} — run `cargo build` first")
            })
            .clone();

        // Freshness guard: a stale cdylib would make every differential test a lie.
        let src = manifest_dir().join("src").join("lib.rs");
        if let (Ok(sm), Ok(om)) = (
            std::fs::metadata(&src).and_then(|m| m.modified()),
            std::fs::metadata(&so).and_then(|m| m.modified()),
        ) {
            assert!(
                om >= sm,
                "STALE cdylib: {} is older than {}.\n\
                 `cargo test` does not rebuild a cdylib — run `cargo build` first.",
                so.display(),
                src.display()
            );
        }
        so
    })
    .clone()
}

fn load(name: &'static str, path: PathBuf) -> Api {
    // Leak the Library so the symbols are valid for the whole process lifetime;
    // unloading it mid-test would invalidate the function pointers.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
    }));
    unsafe {
        let get = |sym: &[u8]| -> *const c_void {
            let s: libloading::Symbol<*const c_void> = lib.get(sym).unwrap_or_else(|e| {
                panic!(
                    "symbol {:?} missing from {}: {e}",
                    String::from_utf8_lossy(sym),
                    path.display()
                )
            });
            *s
        };
        Api {
            name,
            parse_env_numeric: std::mem::transmute::<*const c_void, FnParseEnvNumeric>(get(
                b"parse_env_numeric\0",
            )),
            init_config_from_env: std::mem::transmute::<*const c_void, FnInitConfigFromEnv>(get(
                b"init_config_from_env\0",
            )),
            perform_operation: std::mem::transmute::<*const c_void, FnPerformOperation>(get(
                b"perform_operation\0",
            )),
            apply_bit_operations: std::mem::transmute::<*const c_void, FnApplyBitOperations>(get(
                b"apply_bit_operations\0",
            )),
            envy: std::mem::transmute::<*const c_void, FnEnvy>(get(b"envy\0")),
            path,
        }
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| load("C", c_so_path()))
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| load("Rust", rust_so_path()))
}

/// Both implementations, C first. Also forces both libraries to be loaded.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Global serialisation: the environment and the stdio file descriptors are
// process-wide, so only one test may touch them at a time.
// ---------------------------------------------------------------------------

static GLOBAL: Mutex<()> = Mutex::new(());

/// Refuse to run with more than one test thread.
///
/// A mutex around each test body is NOT enough: libtest writes its own
/// `test foo ... ok` progress lines to the real fd 1 from the harness thread,
/// outside any test body, and those bytes would land inside whichever capture
/// file currently has fd 1 dup2'd onto it. `.cargo/config.toml` sets
/// `RUST_TEST_THREADS = "1"`, and this guard catches an explicit override.
fn assert_serial() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let mut threads: Option<String> = std::env::var("RUST_TEST_THREADS").ok();

        // An explicit `--test-threads N` / `--test-threads=N` on the command
        // line overrides the environment variable.
        let args: Vec<String> = std::env::args().collect();
        for (i, a) in args.iter().enumerate() {
            if let Some(v) = a.strip_prefix("--test-threads=") {
                threads = Some(v.to_string());
            } else if a == "--test-threads" {
                if let Some(v) = args.get(i + 1) {
                    threads = Some(v.clone());
                }
            }
        }

        if let Some(t) = threads {
            if t.trim() != "1" {
                panic!(
                    "these differential tests must run serially (got --test-threads={t}).\n\
                     The library under test writes to the process's stdout/stderr and reads \
                     the process environment, so the harness redirects fds 1 and 2 while it \
                     runs; concurrent libtest progress output would corrupt the captures.\n\
                     Run: cargo test -- --test-threads=1"
                );
            }
        }
    });
}

pub fn lock() -> MutexGuard<'static, ()> {
    assert_serial();
    match GLOBAL.lock() {
        Ok(g) => g,
        // A previous test panicked while holding the lock; the environment is
        // reset at the start of every test anyway, so the data is not poisoned
        // in any meaningful sense.
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Environment control
// ---------------------------------------------------------------------------

/// Every variable the C library reads (`grep -o 'PROG_[A-Z_]*' c_src/src/lib.c`).
pub const PROG_ENV_VARS: [&str; 5] = [
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
];

/// `setenv` rejects an empty name and any name containing `=` with `EINVAL`.
/// Such a name can therefore never be present in the environment, which is
/// itself a valid input to `parse_env_numeric` (the "absent" case).
pub fn env_name_is_settable(name: &str) -> bool {
    !name.is_empty() && !name.contains('=')
}

pub fn env_set(name: &str, value: &str) {
    let n = CString::new(name).expect("env name has no NUL");
    let v = CString::new(value).expect("env value has no NUL");
    let rc = unsafe { setenv(n.as_ptr(), v.as_ptr(), 1) };
    if env_name_is_settable(name) {
        assert_eq!(rc, 0, "setenv({name:?}, {value:?}) failed");
    } else {
        assert_eq!(
            rc, -1,
            "setenv({name:?}, …) unexpectedly succeeded for an invalid name"
        );
    }
}

pub fn env_unset(name: &str) {
    let n = CString::new(name).expect("env name has no NUL");
    unsafe { unsetenv(n.as_ptr()) };
}

pub fn env_apply(name: &str, value: Option<&str>) {
    match value {
        Some(v) => env_set(name, v),
        None => env_unset(name),
    }
}

/// Remove every `PROG_*` variable so each configuration starts from a clean,
/// fully-determined environment.
pub fn env_clear_prog() {
    for v in PROG_ENV_VARS.iter() {
        env_unset(v);
    }
}

/// Apply a whole environment configuration: `(name, Some(value))` sets,
/// `(name, None)` unsets. All `PROG_*` variables are cleared first.
pub fn env_config(pairs: &[(&str, Option<&str>)]) {
    env_clear_prog();
    for (n, v) in pairs {
        env_apply(n, *v);
    }
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Observed {
    pub ret: c_int,
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

impl std::fmt::Debug for Observed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ret={} stdout={:?} stderr={:?}",
            self.ret,
            String::from_utf8_lossy(&self.out),
            String::from_utf8_lossy(&self.err)
        )
    }
}

fn tmp_dir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
}

static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Two scratch files, created once and reused (truncate + rewind) for every
/// capture. Tens of thousands of randomized iterations run through here, so
/// creating and unlinking a file pair per call would dominate the runtime.
struct Scratch {
    out: File,
    err: File,
}

fn scratch() -> &'static Mutex<Scratch> {
    static SCRATCH: OnceLock<Mutex<Scratch>> = OnceLock::new();
    SCRATCH.get_or_init(|| {
        let seq = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        let dir = tmp_dir();
        let mk = |ext: &str| {
            let p = dir.join(format!("difftest-{pid}-{seq}.{ext}"));
            let f = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&p)
                .unwrap_or_else(|e| panic!("create capture file {}: {e}", p.display()));
            // Unlink immediately: the open handle keeps it alive and nothing is
            // left behind if the test process dies.
            let _ = std::fs::remove_file(&p);
            f
        };
        Mutex::new(Scratch {
            out: mk("out"),
            err: mk("err"),
        })
    })
}

fn rewind_and_read(f: &mut File) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    f.seek(SeekFrom::Start(0)).expect("seek capture file");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read capture file");
    buf
}

fn reset(f: &mut File) {
    use std::io::{Seek, SeekFrom};
    f.set_len(0).expect("truncate capture file");
    f.seek(SeekFrom::Start(0)).expect("rewind capture file");
}

/// Run `f`, capturing its return value plus every byte it writes to the
/// process's `stdout` and `stderr` (the library writes through libc `printf` /
/// `fprintf(stderr, …)`, i.e. real file descriptors 1 and 2).
pub fn capture<F: FnOnce() -> c_int>(f: F) -> Observed {
    let mut sc = match scratch().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    unsafe {
        // Drain anything libc already had buffered for fds 1/2 before we steal
        // them, so no unrelated bytes end up in the capture.
        fflush(std::ptr::null_mut());

        reset(&mut sc.out);
        reset(&mut sc.err);

        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");

        assert!(dup2(sc.out.as_raw_fd(), 1) >= 0, "dup2 stdout");
        assert!(dup2(sc.err.as_raw_fd(), 2) >= 0, "dup2 stderr");

        let ret = f();

        // Push out everything the library buffered while fds 1/2 pointed at the
        // temporary files.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved_out, 1) >= 0, "restore stdout");
        assert!(dup2(saved_err, 2) >= 0, "restore stderr");
        close(saved_out);
        close(saved_err);

        let out = rewind_and_read(&mut sc.out);
        let err = rewind_and_read(&mut sc.err);

        Observed { ret, out, err }
    }
}

// ---------------------------------------------------------------------------
// Differential assertion
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Run the same closure against the C and the Rust `Api` (re-applying `env`
/// before each so both observe an identical environment) and require the return
/// value, stdout bytes and stderr bytes to be identical.
pub fn diff_with_env<F>(context: &str, env: &[(&str, Option<&str>)], mut f: F)
where
    F: FnMut(&Api) -> c_int,
{
    let (c, r) = both();
    env_config(env);
    let got_c = capture(|| f(c));
    env_config(env);
    let got_r = capture(|| f(r));

    if got_c.ret != got_r.ret {
        panic!(
            "[{context}] return value diverged: C={} Rust={}\n  env={:?}\n  C stdout={:?}\n  R stdout={:?}",
            got_c.ret,
            got_r.ret,
            env,
            show(&got_c.out),
            show(&got_r.out)
        );
    }
    if got_c.out != got_r.out {
        panic!(
            "[{context}] stdout diverged (ret={})\n  env={:?}\n  C   = \"{}\"\n  Rust= \"{}\"",
            got_c.ret,
            env,
            show(&got_c.out),
            show(&got_r.out)
        );
    }
    if got_c.err != got_r.err {
        panic!(
            "[{context}] stderr diverged (ret={})\n  env={:?}\n  C   = \"{}\"\n  Rust= \"{}\"",
            got_c.ret,
            env,
            show(&got_c.err),
            show(&got_r.err)
        );
    }
}

/// `diff_with_env` with no environment changes beyond clearing `PROG_*`.
pub fn diff<F>(context: &str, f: F)
where
    F: FnMut(&Api) -> c_int,
{
    diff_with_env(context, &[], f)
}

/// Differential check for a call that also mutates a `struct ConfigFlags`
/// buffer: compares the return value, the output streams *and* the resulting
/// four struct bytes.
pub fn diff_flags_with_env<F>(
    context: &str,
    env: &[(&str, Option<&str>)],
    initial: Flags4,
    mut f: F,
) where
    F: FnMut(&Api, *mut u8) -> c_int,
{
    let (c, r) = both();

    let mut fc = initial;
    env_config(env);
    let got_c = capture(|| f(c, fc.as_mut_ptr()));

    let mut fr = initial;
    env_config(env);
    let got_r = capture(|| f(r, fr.as_mut_ptr()));

    assert_eq!(
        got_c.ret, got_r.ret,
        "[{context}] return value diverged (env={env:?}, initial flags={:02x?})",
        initial.0
    );
    assert_eq!(
        show(&got_c.out),
        show(&got_r.out),
        "[{context}] stdout diverged (env={env:?}, initial flags={:02x?})",
        initial.0
    );
    assert_eq!(
        show(&got_c.err),
        show(&got_r.err),
        "[{context}] stderr diverged (env={env:?}, initial flags={:02x?})",
        initial.0
    );
    assert_eq!(
        fc, fr,
        "[{context}] struct ConfigFlags bytes diverged (env={env:?}, initial={:02x?}): C={:02x?} Rust={:02x?}",
        initial.0, fc.0, fr.0
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1E55_C0FF_EE01;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// An `int` biased towards the interesting magnitudes: extremes, small
    /// values, and powers of two near the overflow boundaries, plus uniform
    /// 32-bit noise.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 24] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            -3,
            4,
            -4,
            7,
            -7,
            8,
            15,
            16,
            -16,
            255,
            256,
            -256,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            0x4000_0000,
            -0x4000_0000,
        ];
        match self.below(4) {
            0 => SPECIAL[self.below(SPECIAL.len() as u64) as usize],
            1 => (self.next_u32() % 2001) as i32 - 1000,
            2 => (self.next_u32() % 200_001) as i32 - 100_000,
            _ => self.next_u32() as i32,
        }
    }
    /// A random `struct ConfigFlags` allocation unit: byte 0 fully random
    /// (covers all 256 flag bit patterns incl. `log_level` 4..7 and
    /// `reserved = 1`) and bytes 1..3 random garbage that the C never touches.
    pub fn flags4(&mut self) -> Flags4 {
        let v = self.next_u64();
        Flags4([v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8])
    }
    /// Random garbage for bytes 1..3, byte 0 given.
    pub fn flags4_with_byte0(&mut self, b0: u8) -> Flags4 {
        let v = self.next_u64();
        Flags4([b0, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8])
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// Random environment-value generation
// ---------------------------------------------------------------------------

/// Alphabet deliberately rich in the characters the C code inspects:
/// `,` (comma rejection), `;` (semicolon rejection), `1` (verbose/debug probe),
/// digits and signs (`atoi`), whitespace (`atoi` skips it) and plain letters.
const VALUE_ALPHABET: &[u8] = b"0123456789,;+- \tabxyZ1111";

pub fn random_value(rng: &mut Rng, max_len: usize) -> String {
    let len = rng.below(max_len as u64 + 1) as usize;
    let mut s = String::with_capacity(len);
    for _ in 0..len {
        s.push(*rng.pick(VALUE_ALPHABET) as char);
    }
    s
}

/// A decimal integer string, sometimes signed, sometimes with leading
/// whitespace or trailing garbage, sometimes far outside `int` range.
pub fn random_numeric_value(rng: &mut Rng) -> String {
    let digits = 1 + rng.below(22) as usize;
    let mut s = String::new();
    match rng.below(6) {
        0 => s.push('-'),
        1 => s.push('+'),
        2 => s.push_str("   "),
        3 => s.push_str(" -"),
        _ => {}
    }
    for i in 0..digits {
        let d = rng.below(10) as u8;
        // avoid an all-zero prefix dominating
        s.push((b'0' + if i == 0 && d == 0 { 1 } else { d }) as char);
    }
    match rng.below(8) {
        0 => s.push_str("abc"),
        1 => s.push(' '),
        2 => s.push_str(".5"),
        3 => s.push_str("x10"),
        _ => {}
    }
    s
}

/// A value guaranteed to contain `,` (and maybe `;` too).
pub fn random_comma_value(rng: &mut Rng) -> String {
    let mut s = random_value(rng, 12).replace(',', "x");
    let pos = rng.below(s.len() as u64 + 1) as usize;
    // insert at a char boundary (alphabet is ASCII, so any index works)
    s.insert(pos, ',');
    s
}

/// A value guaranteed to contain `;` and guaranteed **not** to contain `,`.
pub fn random_semicolon_value(rng: &mut Rng) -> String {
    let mut s = random_value(rng, 12).replace(',', "x").replace(';', "y");
    let pos = rng.below(s.len() as u64 + 1) as usize;
    s.insert(pos, ';');
    s
}

/// A value guaranteed to contain neither `,` nor `;`.
pub fn random_clean_value(rng: &mut Rng, max_len: usize) -> String {
    random_value(rng, max_len).replace(',', "q").replace(';', "w")
}

// ---------------------------------------------------------------------------
// Crash-path testing: run a closure in a forked child and report how it died.
// This is how the "no null check anywhere" rows of ERRORS.md are compared.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

pub fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: silence the streams so a crashing child cannot pollute the
        // test log, run the victim call, then leave without any unwinding or
        // atexit handlers.
        unsafe {
            let devnull = std::fs::File::create("/dev/null").ok();
            if let Some(dn) = devnull.as_ref() {
                dup2(dn.as_raw_fd(), 1);
                dup2(dn.as_raw_fd(), 2);
            }
        }
        f();
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    let rc = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(rc, pid, "waitpid failed");
    let sig = status & 0x7f;
    if sig == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else if sig == 0x7f {
        Outcome::Exited(-1) // stopped; should not happen here
    } else {
        Outcome::Signaled(sig)
    }
}

/// Assert the C and Rust libraries die (or survive) in exactly the same way.
pub fn diff_crash<F>(context: &str, mut f: F)
where
    F: FnMut(&Api),
{
    let (c, r) = both();
    let oc = run_in_child(|| f(c));
    let or = run_in_child(|| f(r));
    assert_eq!(
        oc, or,
        "[{context}] crash behaviour diverged: C={oc:?} Rust={or:?}"
    );
}

pub fn cstring(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

/// Helper: call `parse_env_numeric` on an `Api` with a Rust `&str` name.
pub fn call_parse(api: &Api, name: &str, default_val: c_int) -> c_int {
    let n = cstring(name);
    unsafe { (api.parse_env_numeric)(n.as_ptr(), default_val) }
}

pub fn path_of(api: &Api) -> &Path {
    &api.path
}
