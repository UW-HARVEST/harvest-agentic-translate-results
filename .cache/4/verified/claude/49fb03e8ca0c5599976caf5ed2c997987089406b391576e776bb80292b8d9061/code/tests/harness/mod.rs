// Shared differential-test harness.
//
// Both implementations are loaded as shared objects with `libloading` and every
// call goes through `dlsym`, so the `#[no_mangle]` / `extern "C"` export
// wrappers of the Rust translation are exercised exactly like an external C
// consumer would exercise them.  Rust functions are never called directly.
#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// libc bits used by the harness
// ---------------------------------------------------------------------------

extern "C" {
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
}

const SEEK_CUR: c_int = 1;

/// Serialises everything: the harness mutates process-global state (environment
/// variables and file descriptors 1/2), which must never happen concurrently.
pub static GLOBAL: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Function signatures of the five exported symbols
// ---------------------------------------------------------------------------

pub type FnEnvy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnParseEnvNumeric = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
pub type FnInitConfig = unsafe extern "C" fn(*mut u32);
pub type FnPerformOperation = unsafe extern "C" fn(c_int, c_int, *mut u32) -> c_int;
pub type FnApplyBitOperations = unsafe extern "C" fn(c_int, *mut u32) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub envy: FnEnvy,
    pub parse_env_numeric: FnParseEnvNumeric,
    pub init_config_from_env: FnInitConfig,
    pub perform_operation: FnPerformOperation,
    pub apply_bit_operations: FnApplyBitOperations,
    // Keep the library alive for the whole process lifetime.
    _lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        unsafe {
            let envy = *lib
                .get::<FnEnvy>(b"envy\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(envy): {e}"));
            let parse_env_numeric = *lib
                .get::<FnParseEnvNumeric>(b"parse_env_numeric\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(parse_env_numeric): {e}"));
            let init_config_from_env = *lib
                .get::<FnInitConfig>(b"init_config_from_env\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(init_config_from_env): {e}"));
            let perform_operation = *lib
                .get::<FnPerformOperation>(b"perform_operation\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(perform_operation): {e}"));
            let apply_bit_operations = *lib
                .get::<FnApplyBitOperations>(b"apply_bit_operations\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(apply_bit_operations): {e}"));
            Impl {
                name,
                path: path.to_path_buf(),
                envy,
                parse_env_numeric,
                init_config_from_env,
                perform_operation,
                apply_bit_operations,
                _lib: lib,
            }
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libtranslated_rust.so`, built on demand with cmake.
/// `ENVY_C_SO` overrides it (used to cross-check other optimisation levels).
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("ENVY_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "ENVY_C_SO does not exist: {}", p.display());
        return p;
    }
    let build = manifest_dir().join("c_src/build");
    let so = build.join("libtranslated_rust.so");
    if !so.exists() {
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let st = std::process::Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .expect("run cmake");
        assert!(st.success(), "cmake configure failed");
        let st = std::process::Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .status()
            .expect("run cmake --build");
        assert!(st.success(), "cmake build failed");
    }
    assert!(so.exists(), "missing C shared library: {}", so.display());
    so
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

/// The Rust `cdylib`.
///
/// `cargo test` does **not** build a `crate-type = ["cdylib"]` target, so the
/// artifact produced by `cargo build [--release]` is used when it is present and
/// newer than `src/lib.rs`; otherwise the harness compiles the cdylib itself
/// with `rustc` (same source, same profile flags).  `ENVY_RUST_SO` overrides
/// everything, which is how the release-profile run is wired up.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("ENVY_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "ENVY_RUST_SO does not exist: {}", p.display());
        return p;
    }

    let src = manifest_dir().join("src/lib.rs");
    let src_mtime = mtime(&src);
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir").to_path_buf();
    let profile_dir = deps.parent().expect("profile dir").to_path_buf();

    for cand in [
        profile_dir.join("libenvy_lib.so"),
        deps.join("libenvy_lib.so"),
    ] {
        if let (Some(a), Some(b)) = (mtime(&cand), src_mtime) {
            if a >= b {
                return cand;
            }
        }
    }

    // Fall back to building the cdylib straight from the source.
    let out = profile_dir.join("libenvy_lib_harness.so");
    let release = profile_dir.file_name().and_then(|s| s.to_str()) == Some("release");
    let mut cmd = std::process::Command::new("rustc");
    cmd.arg("--crate-name")
        .arg("envy_lib")
        .arg("--crate-type")
        .arg("cdylib")
        .arg("--edition")
        .arg("2021");
    if release {
        cmd.args(["-O", "-C", "panic=abort"]);
    } else {
        cmd.args(["-C", "debug-assertions=on", "-C", "overflow-checks=on"]);
    }
    let st = cmd
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .status()
        .expect("run rustc to build the cdylib");
    assert!(st.success(), "rustc failed to build the cdylib");
    out
}

/// Loads the C and the Rust implementation (in that order).
pub fn load_impls() -> (Impl, Impl) {
    let c = Impl::load("C", &c_so_path());
    let r = Impl::load("RUST", &rust_so_path());
    (c, r)
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
//
// fd 1 and fd 2 are redirected into two temporary files for the whole lifetime
// of a `Capture`.  `take()` flushes every C stream and returns the bytes that
// were appended since the previous `take()`, which makes it cheap enough to
// bracket tens of thousands of individual calls.
// ---------------------------------------------------------------------------

/// The real stderr, saved while a `Capture` is active, so that panic messages
/// are not swallowed by the redirection.
static SAVED_ERR_FD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
static HOOK_ONCE: std::sync::Once = std::sync::Once::new();

fn install_panic_hook() {
    HOOK_ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let fd = SAVED_ERR_FD.load(std::sync::atomic::Ordering::SeqCst);
            if fd >= 0 {
                let msg = format!("\n[panic while fds 1/2 were redirected]\n{info}\n");
                unsafe {
                    write(fd, msg.as_ptr() as *const c_void, msg.len());
                }
            }
            default(info);
        }));
    });
}

pub struct Capture {
    out_w: File,
    err_w: File,
    out_r: File,
    err_r: File,
    out_path: PathBuf,
    err_path: PathBuf,
    saved_out: c_int,
    saved_err: c_int,
    out_pos: u64,
    err_pos: u64,
}

impl Capture {
    pub fn new(tag: &str) -> Capture {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let out_path = dir.join(format!("difftest-{tag}-{pid}.out"));
        let err_path = dir.join(format!("difftest-{tag}-{pid}.err"));
        let out_w = File::create(&out_path).expect("create stdout capture file");
        let err_w = File::create(&err_path).expect("create stderr capture file");
        let out_r = File::open(&out_path).expect("open stdout capture file");
        let err_r = File::open(&err_path).expect("open stderr capture file");
        install_panic_hook();
        unsafe {
            // Flush anything still sitting in libc's buffers before switching.
            fflush(std::ptr::null_mut());
            let saved_out = dup(1);
            let saved_err = dup(2);
            assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
            SAVED_ERR_FD.store(saved_err, std::sync::atomic::Ordering::SeqCst);
            assert!(dup2(out_w.as_raw_fd(), 1) >= 0, "dup2 stdout failed");
            assert!(dup2(err_w.as_raw_fd(), 2) >= 0, "dup2 stderr failed");
            Capture {
                out_w,
                err_w,
                out_r,
                err_r,
                out_path,
                err_path,
                saved_out,
                saved_err,
                out_pos: 0,
                err_pos: 0,
            }
        }
    }

    /// Bytes written to (stdout, stderr) since the previous `take()`.
    pub fn take(&mut self) -> (Vec<u8>, Vec<u8>) {
        unsafe {
            fflush(std::ptr::null_mut());
        }
        let out_end = unsafe { lseek(1, 0, SEEK_CUR) };
        let err_end = unsafe { lseek(2, 0, SEEK_CUR) };
        assert!(out_end >= 0 && err_end >= 0, "lseek failed");
        let out = read_range(&self.out_r, self.out_pos, out_end as u64);
        let err = read_range(&self.err_r, self.err_pos, err_end as u64);
        self.out_pos = out_end as u64;
        self.err_pos = err_end as u64;
        (out, err)
    }

    /// Drops anything buffered so far without comparing it.
    pub fn discard(&mut self) {
        let _ = self.take();
    }
}

fn read_range(f: &File, start: u64, end: u64) -> Vec<u8> {
    assert!(end >= start, "capture file shrank ({start} -> {end})");
    let mut buf = vec![0u8; (end - start) as usize];
    if !buf.is_empty() {
        f.read_exact_at(&mut buf, start).expect("read capture file");
    }
    buf
}

impl Drop for Capture {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved_out, 1);
            dup2(self.saved_err, 2);
            SAVED_ERR_FD.store(-1, std::sync::atomic::Ordering::SeqCst);
            close(self.saved_out);
            close(self.saved_err);
        }
        let _ = &self.out_w;
        let _ = &self.err_w;
        let _ = std::fs::remove_file(&self.out_path);
        let _ = std::fs::remove_file(&self.err_path);
    }
}

// ---------------------------------------------------------------------------
// Environment helpers
// ---------------------------------------------------------------------------

pub const ENV_VARS: [&str; 5] = [
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
];

/// Returns `true` when libc accepted the change.  `setenv` rejects names that
/// contain `'='` or are empty (EINVAL), which is itself part of the surface the
/// tests explore, so this must not panic.
pub fn put_env(name: &str, value: Option<&str>) -> bool {
    let n = CString::new(name).unwrap();
    unsafe {
        match value {
            Some(v) => {
                let v = CString::new(v).unwrap();
                setenv(n.as_ptr(), v.as_ptr(), 1) == 0
            }
            None => unsetenv(n.as_ptr()) == 0,
        }
    }
}

/// Like `put_env` but for values that are not valid UTF-8 (C strings are just
/// bytes, and the library only ever passes the pointer to libc).
pub fn put_env_bytes(name: &str, value: &[u8]) -> bool {
    let n = CString::new(name).unwrap();
    let v = CString::new(value).unwrap();
    unsafe { setenv(n.as_ptr(), v.as_ptr(), 1) == 0 }
}

pub fn clear_prog_env() {
    for v in ENV_VARS {
        put_env(v, None);
    }
}

/// Applies `(PROG_VERBOSE, PROG_DEBUG, PROG_OPTIMIZE, PROG_BASE_OFFSET, PROG_MULTIPLIER)`.
pub fn apply_env(env: &EnvCfg) {
    put_env("PROG_VERBOSE", env.verbose);
    put_env("PROG_DEBUG", env.debug);
    put_env("PROG_OPTIMIZE", env.optimize);
    put_env("PROG_BASE_OFFSET", env.base_offset);
    put_env("PROG_MULTIPLIER", env.multiplier);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EnvCfg {
    pub verbose: Option<&'static str>,
    pub debug: Option<&'static str>,
    pub optimize: Option<&'static str>,
    pub base_offset: Option<&'static str>,
    pub multiplier: Option<&'static str>,
}

/// The three states `init_config_from_env` distinguishes for VERBOSE / DEBUG
/// (absent, present without a '1', present with a '1') and the two it
/// distinguishes for OPTIMIZE.
pub const VERBOSE_STATES: [Option<&'static str>; 3] = [None, Some("0"), Some("1")];
pub const DEBUG_STATES: [Option<&'static str>; 3] = [None, Some("no"), Some("x1y")];
pub const OPTIMIZE_STATES: [Option<&'static str>; 2] = [None, Some("")];

/// Extra shapes used for the flag variables (still only three/two classes, but
/// with more exotic values).
pub const VERBOSE_STATES_WIDE: [Option<&'static str>; 6] = [
    None,
    Some(""),
    Some("0"),
    Some("true"),
    Some("1"),
    Some("a1b1"),
];
pub const OPTIMIZE_STATES_WIDE: [Option<&'static str>; 5] =
    [None, Some(""), Some("0"), Some("no"), Some("1")];

/// The seven value shapes `parse_env_numeric` distinguishes.
pub const NUMERIC_SHAPES: [Option<&'static str>; 16] = [
    None,                    // absent      -> default
    Some(""),                // empty       -> atoi("") == 0
    Some("0"),               // zero
    Some("7"),               // small
    Some("100"),             // decimal, not octal
    Some("0100"),            // leading zero, still decimal 100
    Some("-5"),              // negative
    Some("-100000"),         // large negative
    Some("2147483647"),      // INT_MAX
    Some("-2147483648"),     // INT_MIN
    Some("  42"),            // leading whitespace
    Some("+7"),              // explicit plus
    Some("abc"),             // garbage      -> 0
    Some("12abc"),           // prefix parse -> 12
    Some("1,2"),             // comma        -> warning + default
    Some("3;4"),             // semicolon    -> warning + default
];

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// A mixture of small, medium and full-range values, plus the boundaries.
    pub fn next_interesting_i32(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 8 {
            0 => 0,
            1 => (r >> 8) as i8 as c_int,               // -128..127
            2 => (r >> 8) as i16 as c_int,              // -32768..32767
            3 => ((r >> 8) as u32 & 0xFFFF) as c_int,   // 0..65535
            4 => i32::MAX,
            5 => i32::MIN,
            6 => -(((r >> 8) as u32 % 1000) as c_int),
            _ => self.next_i32(),
        }
    }
    pub fn choice<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

pub const SEED: u64 = 0x2026_0819_C0FF_EE01;

pub const BOUNDARY_I32: [c_int; 11] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    i32::MAX,
    i32::MIN,
    0x4000_0000,
    -0x4000_0000,
    0x7FFF_FFFE,
];

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Runs `f` against the C implementation and then against the Rust
/// implementation, capturing return value, stdout and stderr for both, and
/// records a detailed message in `fails` when anything differs.
///
/// `f` must be side-effect free apart from the library call itself (it sets up
/// its own scratch state on every invocation).
pub fn differential<F>(
    fails: &mut Vec<String>,
    cap: &mut Capture,
    c: &Impl,
    r: &Impl,
    label: &str,
    mut f: F,
) where
    F: FnMut(&Impl) -> i64,
{
    let c_ret = f(c);
    let (c_out, c_err) = cap.take();
    let r_ret = f(r);
    let (r_out, r_err) = cap.take();

    if c_ret != r_ret || c_out != r_out || c_err != r_err {
        let mut m = format!("MISMATCH [{label}]\n");
        if c_ret != r_ret {
            m += &format!(
                "  return : C = {c_ret} (0x{:016x})  RUST = {r_ret} (0x{:016x})\n",
                c_ret as u64, r_ret as u64
            );
        }
        if c_out != r_out {
            m += &format!(
                "  stdout : C = {:?}\n           R = {:?}\n",
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
        }
        if c_err != r_err {
            m += &format!(
                "  stderr : C = {:?}\n           R = {:?}\n",
                String::from_utf8_lossy(&c_err),
                String::from_utf8_lossy(&r_err)
            );
        }
        fails.push(m);
    }
}

fn contains(hay: &[u8], needle: &str) -> bool {
    let n = needle.as_bytes();
    hay.windows(n.len()).any(|w| w == n)
}

/// Proves that the capture machinery really observes what the shared objects
/// print (otherwise every stdout/stderr comparison would trivially "pass").
/// Must be called while `cap` is active; the caller reports the error after
/// dropping `cap`.
pub fn self_check_capture(cap: &mut Capture, imp: &Impl) -> Result<(), String> {
    clear_prog_env();
    put_env("PROG_VERBOSE", Some("1"));
    put_env("PROG_DEBUG", Some("1"));
    cap.discard();
    let _ = call_envy(imp, 1, 2, 3, 4);
    let (out, err) = cap.take();
    if !contains(&out, "Verbose mode enabled\n") {
        return Err(format!(
            "capture self-check failed: stdout of {} did not contain the verbose banner (got {:?})",
            imp.name,
            String::from_utf8_lossy(&out)
        ));
    }
    if !contains(&out, "Debug: Result string format validated\n") {
        return Err(format!(
            "capture self-check failed: stdout of {} lacked the debug line (got {:?})",
            imp.name,
            String::from_utf8_lossy(&out)
        ));
    }
    if !err.is_empty() {
        return Err(format!(
            "capture self-check failed: unexpected stderr {:?}",
            String::from_utf8_lossy(&err)
        ));
    }

    put_env("PROG_BASE_OFFSET", Some("1,2"));
    let _ = call_envy(imp, 1, 2, 3, 4);
    let (_out, err) = cap.take();
    if !contains(&err, "Warning: Invalid character in PROG_BASE_OFFSET\n") {
        return Err(format!(
            "capture self-check failed: stderr of {} did not contain the comma warning (got {:?})",
            imp.name,
            String::from_utf8_lossy(&err)
        ));
    }
    clear_prog_env();
    cap.discard();
    Ok(())
}

/// Per-row check-off bookkeeping for CONFIGS.md / ERRORS.md.
pub struct Rows {
    pub table: &'static str,
    pub entries: Vec<(u32, String, usize, usize)>,
}

impl Rows {
    pub fn new(table: &'static str) -> Rows {
        Rows {
            table,
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, row: u32, name: &str, checked: usize, failed: usize) {
        self.entries.push((row, name.to_string(), checked, failed));
    }

    pub fn print(&self) {
        println!("\n{} row check-off:", self.table);
        println!(
            "{:>4}  {:<52} {:>8} {:>8}",
            "row", "configuration / test", "checks", "failed"
        );
        for (row, name, checked, failed) in &self.entries {
            println!(
                "{:>4}  {:<52} {:>8} {:>8} {}",
                row,
                name,
                checked,
                failed,
                if *failed == 0 && *checked > 0 {
                    "[x]"
                } else {
                    "[FAIL]"
                }
            );
        }
    }

    /// Panics unless every expected row ran at least one comparison.
    pub fn assert_covers(&self, expected: &[u32]) {
        let missing: Vec<u32> = expected
            .iter()
            .copied()
            .filter(|row| {
                !self
                    .entries
                    .iter()
                    .any(|(r, _, checked, _)| r == row && *checked > 0)
            })
            .collect();
        assert!(
            missing.is_empty(),
            "{} rows without any check: {missing:?}",
            self.table
        );
    }
}

pub fn report(fails: Vec<String>, checked: usize, what: &str) {
    println!("{what}: {checked} differential comparisons performed");
    if !fails.is_empty() {
        let shown: Vec<String> = fails.iter().take(25).cloned().collect();
        panic!(
            "{}/{} comparisons diverged in {what}\n\nfirst {} failure(s):\n{}",
            fails.len(),
            checked,
            shown.len(),
            shown.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// Convenience wrappers used by the tests
// ---------------------------------------------------------------------------

pub fn call_parse(imp: &Impl, name: &CString, default_val: c_int) -> i64 {
    unsafe { (imp.parse_env_numeric)(name.as_ptr(), default_val) as i64 }
}

pub fn call_init(imp: &Impl, prefill: u32) -> i64 {
    let mut bits = prefill;
    unsafe { (imp.init_config_from_env)(&mut bits) };
    bits as i64
}

pub fn call_perform(imp: &Impl, v1: c_int, v2: c_int, bits: u32) -> i64 {
    let mut b = bits;
    let ret = unsafe { (imp.perform_operation)(v1, v2, &mut b) };
    // The C function must not modify the flags: fold the final bit pattern into
    // the compared value so an accidental write would be caught too.
    ((b as u64) << 32 | (ret as u32 as u64)) as i64
}

pub fn call_apply(imp: &Impl, value: c_int, bits: u32) -> i64 {
    let mut b = bits;
    let ret = unsafe { (imp.apply_bit_operations)(value, &mut b) };
    ((b as u64) << 32 | (ret as u32 as u64)) as i64
}

pub fn call_envy(imp: &Impl, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> i64 {
    unsafe { (imp.envy)(p1, p2, p3, p4) as i64 }
}

// --- misaligned `struct ConfigFlags*` (a caller casting a byte buffer) ------
//
// x86-64 allows unaligned 4-byte accesses and that is exactly what gcc emits
// for the bit-field read-modify-write, so the C library happily accepts such a
// pointer.  These helpers place the storage unit at `off` bytes into a scratch
// buffer and return both the call's result and the resulting buffer contents.

fn scratch_with(bits: u32, off: usize) -> [u8; 16] {
    let mut buf = [0xEEu8; 16];
    buf[off..off + 4].copy_from_slice(&bits.to_ne_bytes());
    buf
}

fn fold(buf: &[u8; 16], ret: c_int) -> i64 {
    // 8 bytes around the storage unit + the return value, so a stray write is
    // detected as well.
    let mut h: u64 = 0;
    for b in buf.iter() {
        h = h.wrapping_mul(31).wrapping_add(*b as u64);
    }
    ((h as u32 as u64) << 32 | (ret as u32 as u64)) as i64
}

pub fn call_perform_unaligned(imp: &Impl, v1: c_int, v2: c_int, bits: u32, off: usize) -> i64 {
    let mut buf = scratch_with(bits, off);
    let p = unsafe { buf.as_mut_ptr().add(off) } as *mut u32;
    let ret = unsafe { (imp.perform_operation)(v1, v2, p) };
    fold(&buf, ret)
}

pub fn call_apply_unaligned(imp: &Impl, value: c_int, bits: u32, off: usize) -> i64 {
    let mut buf = scratch_with(bits, off);
    let p = unsafe { buf.as_mut_ptr().add(off) } as *mut u32;
    let ret = unsafe { (imp.apply_bit_operations)(value, p) };
    fold(&buf, ret)
}

pub fn call_init_unaligned(imp: &Impl, bits: u32, off: usize) -> i64 {
    let mut buf = scratch_with(bits, off);
    let p = unsafe { buf.as_mut_ptr().add(off) } as *mut u32;
    unsafe { (imp.init_config_from_env)(p) };
    fold(&buf, 0)
}

/// Builds a `ConfigFlags` bit pattern from the individual fields.
pub fn flags(verbose: bool, debug: bool, optimize: bool, cache: bool, log_level: u32) -> u32 {
    (verbose as u32)
        | (debug as u32) << 1
        | (optimize as u32) << 2
        | (cache as u32) << 3
        | (log_level & 7) << 4
}
