// Shared differential-test harness.
//
// Loads BOTH the C `.so` and the Rust `.so` via `libloading` and calls every
// function through its exported C symbol, so the `#[no_mangle]`/`extern "C"`
// wrappers are part of what is under test. Rust functions are never called
// directly.
//
// stdout / stderr are captured by redirecting fd 1 / fd 2 onto temp files
// around each call, so the `printf`/`fprintf` output of both libraries can be
// compared byte-for-byte. Both `.so`s share the process's single libc, hence
// the same `FILE* stdout` and the same buffering behaviour.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Outcome of running a call in a forked child: either it returned normally
/// with the given `int`, or it was killed by a signal.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Outcome {
    Returned(i32),
    Signalled(i32),
}

/// Run `f` in a forked child and report how the child terminated. Used for the
/// inputs that make the C library fault (null `struct` pointers): the two
/// libraries must fault the *same* way, and that cannot be observed in-process.
pub fn fork_outcome<F: FnOnce() -> i32>(f: F) -> Outcome {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let r = f();
            // Only the low 8 bits survive exit(); the callers below use small
            // sentinel values so this is lossless for them.
            _exit(r & 0x7F);
        }
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        // WIFSIGNALED / WTERMSIG / WEXITSTATUS, per glibc's <sys/wait.h>.
        let term_sig = status & 0x7F;
        if term_sig != 0 && term_sig != 0x7F {
            Outcome::Signalled(term_sig)
        } else {
            Outcome::Returned((status >> 8) & 0xFF)
        }
    }
}

// ---------------------------------------------------------------------------
// Global serialization: fd redirection and the environment are process-global.
// ---------------------------------------------------------------------------

fn big_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

pub fn lock() -> MutexGuard<'static, ()> {
    match big_lock().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// ConfigFlags mirror: 4-byte allocation unit, byte 0 holds all six bit-fields.
// ---------------------------------------------------------------------------

pub const B_VERBOSE: u8 = 1 << 0;
pub const B_DEBUG: u8 = 1 << 1;
pub const B_OPTIMIZE: u8 = 1 << 2;
pub const B_CACHE: u8 = 1 << 3;
pub const B_RESERVED: u8 = 1 << 7;

/// Build a `ConfigFlags` byte-0 value from individual fields.
pub fn flag_byte(verbose: bool, debug: bool, optimize: bool, cache: bool, log_level: u8) -> u8 {
    let mut b = 0u8;
    if verbose {
        b |= B_VERBOSE;
    }
    if debug {
        b |= B_DEBUG;
    }
    if optimize {
        b |= B_OPTIMIZE;
    }
    if cache {
        b |= B_CACHE;
    }
    b |= (log_level & 0x7) << 4;
    b
}

/// A 4-byte, 4-aligned `struct ConfigFlags` allocation the tests hand to the
/// libraries. `#[repr(C, align(4))]` guarantees the same alignment gcc assumes.
#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Flags(pub [u8; 4]);

impl Flags {
    pub fn from_byte0(b: u8) -> Flags {
        Flags([b, 0, 0, 0])
    }
    pub fn dirty(b: u8) -> Flags {
        Flags([b, 0xFF, 0xFF, 0xFF])
    }
    pub fn as_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible.
// ---------------------------------------------------------------------------

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Biased toward "interesting" magnitudes: small values, powers of two and
    /// full-range values all appear, so both the overflow and the tiny-value
    /// code paths get hit.
    pub fn interesting_i32(&mut self) -> i32 {
        let r = self.next_u64();
        match r % 8 {
            0 => (r >> 3) as i32 % 4,
            1 => -((r >> 3) as i32 % 4),
            2 => (r >> 3) as i32 % 1000,
            3 => -((r >> 3) as i32 % 1000),
            4 => 1i32.wrapping_shl((r >> 3) as u32 % 32),
            5 => (1i32.wrapping_shl((r >> 3) as u32 % 32)).wrapping_neg(),
            _ => self.next_i32(),
        }
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

pub const BOUNDARY_I32: [i32; 13] = [
    i32::MIN,
    i32::MIN + 1,
    i32::MIN + 2,
    -1073741824,
    -3,
    -2,
    -1,
    0,
    1,
    2,
    3,
    i32::MAX - 1,
    i32::MAX,
];

// ---------------------------------------------------------------------------
// Environment control (via libc setenv/unsetenv so getenv in both .so's sees it)
// ---------------------------------------------------------------------------

pub const ENV_NAMES: [&str; 5] = [
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
];

pub fn env_set(name: &str, value: &str) {
    let n = CString::new(name).unwrap();
    let v = CString::new(value).unwrap();
    unsafe {
        setenv(n.as_ptr(), v.as_ptr(), 1);
    }
}

pub fn env_unset(name: &str) {
    let n = CString::new(name).unwrap();
    unsafe {
        unsetenv(n.as_ptr());
    }
}

/// Apply a whole environment description: `None` means unset.
pub fn env_apply(vars: &[(&str, Option<&str>)]) {
    for name in ENV_NAMES.iter() {
        env_unset(name);
    }
    for (n, v) in vars {
        match v {
            Some(s) => env_set(n, s),
            None => env_unset(n),
        }
    }
}

pub fn env_clear_all() {
    for name in ENV_NAMES.iter() {
        env_unset(name);
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    // `ENVY_C_SO` lets the suite be pointed at an alternative build of the same
    // C source (e.g. a -O2 build) to confirm the Rust matches gcc's behaviour
    // under optimisation too.
    if let Some(p) = std::env::var_os("ENVY_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "ENVY_C_SO={} does not exist", p.display());
        return p;
    }
    let build = repo_root().join("c_src").join("build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                found = Some(p);
                break;
            }
        }
    }
    let so = found.unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}. Build it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    });
    // The C source is the ground truth and is never modified, but guard against
    // silently comparing against a stale build of it anyway.
    let c_src = repo_root().join("c_src").join("src").join("lib.c");
    assert!(
        mtime(&c_src) <= mtime(&so),
        "STALE ARTIFACT: {} is newer than {}. Rebuild the C library.",
        c_src.display(),
        so.display()
    );
    so
}

fn mtime(p: &std::path::Path) -> std::time::SystemTime {
    std::fs::metadata(p)
        .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
        .modified()
        .unwrap()
}

/// `cargo test` builds the integration-test binaries but does NOT rebuild the
/// `cdylib` artifact (nothing links against it), so without this the tests
/// would happily keep exercising a stale `libenvy_lib.so` and report success
/// for a translation that no longer matches the source. Rebuild it explicitly,
/// then assert it really is newer than every Rust source file.
fn ensure_rust_so_fresh(so: &std::path::Path) {
    let manifest = repo_root().join("translation").join("Cargo.toml");
    let status = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()))
        .arg("build")
        .arg("--release")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(&manifest)
        // Do not inherit the parent `cargo test` invocation's bookkeeping vars.
        .env_remove("CARGO_MAKEFLAGS")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .status()
        .expect("failed to spawn `cargo build --release` for the cdylib");
    assert!(status.success(), "`cargo build --release` for the cdylib failed");

    assert!(
        so.exists(),
        "libenvy_lib.so still missing after `cargo build --release`: {}",
        so.display()
    );
    let so_t = mtime(so);
    let src_dir = repo_root().join("translation").join("src");
    for e in std::fs::read_dir(&src_dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            assert!(
                mtime(&p) <= so_t,
                "STALE ARTIFACT: {} is newer than {}. The differential tests would \
                 have compared the C library against an out-of-date Rust .so.",
                p.display(),
                so.display()
            );
        }
    }
}

fn find_rust_so() -> PathBuf {
    // `ENVY_RUST_SO` lets the suite be pointed at another build of the crate
    // (e.g. the unoptimised `debug` cdylib, which is compiled with
    // panic=unwind instead of panic=abort).
    if let Some(p) = std::env::var_os("ENVY_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "ENVY_RUST_SO={} does not exist", p.display());
        return p;
    }
    let p = repo_root()
        .join("translation")
        .join("target")
        .join("release")
        .join("libenvy_lib.so");
    ensure_rust_so_fresh(&p);
    p
}

pub type FnEnvy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnParseEnv = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
pub type FnInitCfg = unsafe extern "C" fn(*mut u8);
pub type FnPerformOp = unsafe extern "C" fn(c_int, c_int, *mut u8) -> c_int;
pub type FnApplyBits = unsafe extern "C" fn(c_int, *mut u8) -> c_int;

pub struct Lib {
    pub lib: Library,
    pub tag: &'static str,
}

impl Lib {
    pub fn envy(&self) -> Symbol<'_, FnEnvy> {
        unsafe { self.lib.get(b"envy\0").expect("envy") }
    }
    pub fn parse_env_numeric(&self) -> Symbol<'_, FnParseEnv> {
        unsafe { self.lib.get(b"parse_env_numeric\0").expect("parse_env_numeric") }
    }
    pub fn init_config_from_env(&self) -> Symbol<'_, FnInitCfg> {
        unsafe {
            self.lib
                .get(b"init_config_from_env\0")
                .expect("init_config_from_env")
        }
    }
    pub fn perform_operation(&self) -> Symbol<'_, FnPerformOp> {
        unsafe { self.lib.get(b"perform_operation\0").expect("perform_operation") }
    }
    pub fn apply_bit_operations(&self) -> Symbol<'_, FnApplyBits> {
        unsafe {
            self.lib
                .get(b"apply_bit_operations\0")
                .expect("apply_bit_operations")
        }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

/// Both libraries are loaded exactly once for the whole test binary and shared
/// (they are stateless apart from the process environment).
pub fn libs() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }.expect("dlopen C .so");
        let rs = unsafe { Library::new(&rs_path) }.expect("dlopen Rust .so");
        Pair {
            c: Lib { lib: c, tag: "C" },
            rs: Lib { lib: rs, tag: "Rust" },
            c_path,
            rs_path,
        }
    })
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

fn tmp_dir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn read_and_truncate(f: &mut std::fs::File) -> Vec<u8> {
    use std::io::{Seek, SeekFrom};
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    f.set_len(0).unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    buf
}

pub struct Captured<T> {
    pub ret: T,
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

/// The two scratch files fd 1 / fd 2 are redirected onto. Opened once per test
/// process and reused, so a sweep of thousands of differential calls does not
/// pay for a file create/unlink each time.
fn scratch() -> &'static Mutex<(std::fs::File, std::fs::File)> {
    static S: OnceLock<Mutex<(std::fs::File, std::fs::File)>> = OnceLock::new();
    S.get_or_init(|| {
        let dir = tmp_dir();
        let pid = std::process::id();
        let mk = |which: &str| {
            let p = dir.join(format!("envy_diff_{which}_{pid}.tmp"));
            let f = std::fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(&p)
                .unwrap_or_else(|e| panic!("open scratch {}: {e}", p.display()));
            // Unlink immediately: the open fd keeps it alive, nothing is left
            // behind on disk when the process exits.
            let _ = std::fs::remove_file(&p);
            f
        };
        Mutex::new((mk("out"), mk("err")))
    })
}

/// Run `f`, capturing everything the libraries write to fd 1 and fd 2.
pub fn capture<T, F: FnOnce() -> T>(f: F) -> Captured<T> {
    let mut guard = match scratch().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let (out_file, err_file) = &mut *guard;

    use std::os::unix::io::AsRawFd;
    let (ret, out, err) = unsafe {
        // Flush anything libc/Rust already buffered so it is not attributed to
        // the call under test.
        fflush(std::ptr::null_mut());
        let saved_out = dup(1);
        let saved_err = dup(2);
        dup2(out_file.as_raw_fd(), 1);
        dup2(err_file.as_raw_fd(), 2);

        let ret = f();

        // Force the libraries' stdio buffers out before we look at the files.
        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);

        let out = read_and_truncate(out_file);
        let err = read_and_truncate(err_file);
        (ret, out, err)
    };

    Captured { ret, out, err }
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

pub fn assert_same<T: PartialEq + std::fmt::Debug>(
    ctx: &str,
    c: &Captured<T>,
    rs: &Captured<T>,
) {
    assert_eq!(
        c.ret, rs.ret,
        "RETURN VALUE mismatch [{ctx}]\n  C   = {:?}\n  Rust= {:?}",
        c.ret, rs.ret
    );
    assert_eq!(
        c.out,
        rs.out,
        "STDOUT mismatch [{ctx}]\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&c.out),
        show(&rs.out)
    );
    assert_eq!(
        c.err,
        rs.err,
        "STDERR mismatch [{ctx}]\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&c.err),
        show(&rs.err)
    );
}

// ---------------------------------------------------------------------------
// Differential drivers — one per exported symbol. Each calls the C export then
// the Rust export under identical conditions and compares return + output.
// ---------------------------------------------------------------------------

/// Differential `envy(param1, param2, param3, param4)`.
pub fn diff_envy(ctx: &str, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let l = libs();
    let cf = l.c.envy();
    let rf = l.rs.envy();
    let c = capture(|| unsafe { cf(p1, p2, p3, p4) });
    let r = capture(|| unsafe { rf(p1, p2, p3, p4) });
    assert_same(
        &format!("{ctx} envy({p1},{p2},{p3},{p4})"),
        &c,
        &r,
    );
    c.ret
}

/// Differential `parse_env_numeric(name, default_val)`.
pub fn diff_parse_env(ctx: &str, name: &str, default_val: i32) -> i32 {
    let l = libs();
    let cf = l.c.parse_env_numeric();
    let rf = l.rs.parse_env_numeric();
    let cname = CString::new(name).unwrap();
    let c = capture(|| unsafe { cf(cname.as_ptr(), default_val) });
    let r = capture(|| unsafe { rf(cname.as_ptr(), default_val) });
    assert_same(
        &format!("{ctx} parse_env_numeric({name:?},{default_val})"),
        &c,
        &r,
    );
    c.ret
}

/// Differential `init_config_from_env(&flags)`. Compares all four bytes of the
/// `ConfigFlags` allocation unit, including the never-named padding bytes.
pub fn diff_init_config(ctx: &str, initial: Flags) -> Flags {
    let l = libs();
    let cf = l.c.init_config_from_env();
    let rf = l.rs.init_config_from_env();

    let mut cflags = initial;
    let mut rflags = initial;
    let c = capture(|| unsafe {
        cf(cflags.as_ptr());
    });
    let r = capture(|| unsafe {
        rf(rflags.as_ptr());
    });
    assert_same(&format!("{ctx} init_config_from_env({initial:?})"), &c, &r);
    assert_eq!(
        cflags, rflags,
        "ConfigFlags BYTES mismatch [{ctx} init_config_from_env(initial={initial:?})]\n  \
         C   = {cflags:?}\n  Rust= {rflags:?}"
    );
    cflags
}

/// Differential `perform_operation(val1, val2, &flags)`. Also verifies the
/// callee leaves the flags allocation unit byte-identical.
pub fn diff_perform_op(ctx: &str, val1: i32, val2: i32, flags: Flags) -> i32 {
    let l = libs();
    let cf = l.c.perform_operation();
    let rf = l.rs.perform_operation();

    let mut cflags = flags;
    let mut rflags = flags;
    let c = capture(|| unsafe { cf(val1, val2, cflags.as_ptr()) });
    let r = capture(|| unsafe { rf(val1, val2, rflags.as_ptr()) });
    assert_same(
        &format!("{ctx} perform_operation({val1},{val2},{flags:?})"),
        &c,
        &r,
    );
    assert_eq!(
        cflags, rflags,
        "ConfigFlags mutated differently [{ctx} perform_operation({val1},{val2},{flags:?})]"
    );
    c.ret
}

/// Differential `apply_bit_operations(value, &flags)`.
pub fn diff_apply_bits(ctx: &str, value: i32, flags: Flags) -> i32 {
    let l = libs();
    let cf = l.c.apply_bit_operations();
    let rf = l.rs.apply_bit_operations();

    let mut cflags = flags;
    let mut rflags = flags;
    let c = capture(|| unsafe { cf(value, cflags.as_ptr()) });
    let r = capture(|| unsafe { rf(value, rflags.as_ptr()) });
    assert_same(
        &format!("{ctx} apply_bit_operations({value},{flags:?})"),
        &c,
        &r,
    );
    assert_eq!(
        cflags, rflags,
        "ConfigFlags mutated differently [{ctx} apply_bit_operations({value},{flags:?})]"
    );
    c.ret
}

/// Differential run of the low-level pipeline in the same order `envy` uses it,
/// but composed by the caller out of the individual exports.
pub fn diff_pipeline(ctx: &str, p1: i32, p2: i32, p3: i32, p4: i32, initial: Flags) -> i32 {
    let l = libs();

    fn run(lib: &Lib, p1: i32, p2: i32, p3: i32, p4: i32, initial: Flags) -> (i32, Flags, i32, i32) {
        let init = lib.init_config_from_env();
        let op = lib.perform_operation();
        let bits = lib.apply_bit_operations();
        let pen = lib.parse_env_numeric();
        let base_name = CString::new("PROG_BASE_OFFSET").unwrap();
        let mult_name = CString::new("PROG_MULTIPLIER").unwrap();

        let mut flags = initial;
        unsafe {
            init(flags.as_ptr());
            let base_offset = pen(base_name.as_ptr(), 0o100);
            let multiplier = pen(mult_name.as_ptr(), 0o12);

            let mut result = op(p1, p2, flags.as_ptr());
            if p3 != 0 {
                result = result.wrapping_add(p3.wrapping_mul(multiplier));
            }
            if p4 != 0 {
                result = result.wrapping_add(p4 >> 2);
            }
            result = bits(result, flags.as_ptr());
            result = result.wrapping_add(base_offset);
            (result, flags, base_offset, multiplier)
        }
    }

    let c = capture(|| run(&l.c, p1, p2, p3, p4, initial));
    let r = capture(|| run(&l.rs, p1, p2, p3, p4, initial));
    let ctxs = format!("{ctx} pipeline({p1},{p2},{p3},{p4},{initial:?})");
    assert_eq!(
        c.ret.0, r.ret.0,
        "pipeline RESULT mismatch [{ctxs}]: C={} Rust={}",
        c.ret.0, r.ret.0
    );
    assert_eq!(
        c.ret.1, r.ret.1,
        "pipeline FLAGS mismatch [{ctxs}]: C={:?} Rust={:?}",
        c.ret.1, r.ret.1
    );
    assert_eq!(
        (c.ret.2, c.ret.3),
        (r.ret.2, r.ret.3),
        "pipeline base_offset/multiplier mismatch [{ctxs}]"
    );
    assert_eq!(
        c.out,
        r.out,
        "pipeline STDOUT mismatch [{ctxs}]\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&c.out),
        show(&r.out)
    );
    assert_eq!(
        c.err,
        r.err,
        "pipeline STDERR mismatch [{ctxs}]\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&c.err),
        show(&r.err)
    );
    c.ret.0
}
