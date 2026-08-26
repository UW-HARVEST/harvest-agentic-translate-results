// Shared differential-testing harness.
//
// Both implementations are always driven through their C ABI: the C code is
// compiled into `libcdriver.so` and the Rust code into `libdriver.so`, both are
// loaded with `libloading`, and only the exported `driver` / `main` symbols are
// called.  No Rust function of the crate under test is ever called directly, so
// the `#[no_mangle] extern "C"` wrappers are covered as well.
//
// In addition `cli.rs`-style tests spawn the two real executables
// (`c_src/build/driver` built by CMake vs. `target/<profile>/driver`) and compare
// stdout, stderr and the exit status.

#![allow(dead_code)]

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use libloading::Library;

// ---------------------------------------------------------------------------
// libc bits used to move fd 0 / fd 1 around
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
    fn fseek(stream: *mut c_void, offset: i64, whence: c_int) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    static stdin: *mut c_void;
}

/// Serialises everything that touches the process-wide file descriptors 0/1.
static IO_LOCK: Mutex<()> = Mutex::new(());
static BUILD_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Artefact locations
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/debug` or `target/release` (the test binary lives in `.../deps`).
pub fn target_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target dir")
        .to_path_buf()
}

pub fn tmp_dir() -> PathBuf {
    let d = target_dir().join("difftmp");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// The Rust shared object built from `src/lib.rs` (`crate-type = ["cdylib", …]`).
///
/// `cargo test` only builds the `rlib` flavour of the library target, so the
/// `cdylib` is built on demand here.  That keeps a plain `cargo test` working
/// without having to remember a preceding `cargo build`.
pub fn rust_so() -> PathBuf {
    let p = target_dir().join("libdriver.so");
    if !p.exists() {
        build_rust_cdylib();
    }
    assert!(
        p.exists(),
        "{} is missing and `cargo build --lib` did not produce it",
        p.display()
    );
    p
}

fn build_rust_cdylib() {
    let _g = BUILD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if target_dir().join("libdriver.so").exists() {
        return;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let release = target_dir().file_name().map(|s| s == "release").unwrap_or(false);
    let mut cmd = Command::new(&cargo);
    cmd.arg("build").arg("--lib");
    if release {
        cmd.arg("--release");
    }
    cmd.current_dir(manifest_dir());
    // Do not inherit the harness' own capture state; just let it print.
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run `{cargo} build --lib`: {e}"));
    assert!(status.success(), "`{cargo} build --lib` failed");
}

/// `c_src/src/main.c` compiled as a shared object.
pub fn c_so() -> PathBuf {
    let out = target_dir().join("libcdriver.so");
    build_once(&out, &["-shared", "-fPIC", "-O2"]);
    out
}

/// The C executable.  The CMake build (`c_src/build/driver`) is preferred so the
/// artefact described by `c_src/CMakeLists.txt` is the one under test.
pub fn c_exe() -> PathBuf {
    let cmake = manifest_dir().join("c_src/build/driver");
    if cmake.exists() {
        return cmake;
    }
    let out = target_dir().join("cdriver");
    build_once(&out, &["-O2"]);
    out
}

pub fn rust_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn build_once(out: &Path, flags: &[&str]) {
    let _g = BUILD_LOCK.lock().unwrap();
    if out.exists() {
        return;
    }
    let src = manifest_dir().join("c_src/src/main.c");
    let tmp = out.with_extension(format!("tmp{}", std::process::id()));
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = Command::new(&cc)
        .args(flags)
        .arg("-o")
        .arg(&tmp)
        .arg(&src)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));
    assert!(status.success(), "{cc} {flags:?} failed for {}", src.display());
    // Atomic publish so parallel test binaries cannot observe a partial file.
    std::fs::rename(&tmp, out).expect("rename compiled artefact");
}

// ---------------------------------------------------------------------------
// The two loaded shared objects
// ---------------------------------------------------------------------------

pub type DriverFn = unsafe extern "C" fn(f64);
pub type MainFn = unsafe extern "C" fn() -> c_int;

pub struct Libs {
    c: Library,
    rust: Library,
}

// `libloading::Library` is a plain handle; sharing it across threads is fine.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so();
        let r_path = rust_so();
        unsafe {
            Libs {
                c: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
                rust: Library::new(&r_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display())),
            }
        }
    })
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    C,
    Rust,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::C => "C",
            Side::Rust => "Rust",
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Side::C => "c",
            Side::Rust => "rust",
        }
    }
}

impl Libs {
    fn lib(&self, side: Side) -> &Library {
        match side {
            Side::C => &self.c,
            Side::Rust => &self.rust,
        }
    }

    pub fn driver(&'static self, side: Side) -> DriverFn {
        unsafe {
            *self
                .lib(side)
                .get::<DriverFn>(b"driver\0")
                .expect("dlsym driver")
        }
    }

    pub fn main(&'static self, side: Side) -> MainFn {
        unsafe { *self.lib(side).get::<MainFn>(b"main\0").expect("dlsym main") }
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Restores file descriptor 1 even if the body panics — otherwise a single
/// failing case would send every later case's output into a stale file.
struct Fd1Guard {
    saved: c_int,
}

impl Drop for Fd1Guard {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
    }
}

/// Runs `f` with file descriptor 1 pointing at a fresh file and returns
/// everything that was written to it.  Both the C stdio buffers and the Rust
/// `std::io::stdout` buffers of *this* process are flushed before and after, and
/// the shared objects flush their own buffers on every call.
///
/// The test binaries are built with `harness = false` (see `Cargo.toml`) so that
/// nothing else in the process can write to fd 1 while it is redirected — the
/// libtest harness prints its own progress lines from a different thread, which
/// would otherwise be captured as if the library had emitted them.
fn with_stdout_to_file<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _g = IO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = tmp_dir().join(format!("stdout-{tag}-{}.bin", std::process::id()));

    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let file = std::fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    let guard = Fd1Guard { saved };
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto 1 failed");

    f();

    let _ = std::io::Write::flush(&mut std::io::stdout());
    drop(guard);
    drop(file);

    std::fs::read(&path).expect("read capture file")
}

/// Splits captured output into one entry per emitted line (the C code always
/// terminates its single line with `\n`).
fn split_lines(raw: &[u8], expected: usize, side: Side) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::with_capacity(expected);
    let mut cur: Vec<u8> = Vec::new();
    for &b in raw {
        if b == b'\n' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(b);
        }
    }
    assert!(
        cur.is_empty(),
        "{} side produced unterminated output {:?}",
        side.name(),
        String::from_utf8_lossy(&cur)
    );
    assert_eq!(
        out.len(),
        expected,
        "{} side produced {} lines, expected {}",
        side.name(),
        out.len(),
        expected
    );
    out
}


/// Set `DIFF_COUNTS=1` to have every differential comparison report how many
/// inputs it actually compared (used to fill in the tables in CONFIGS.md).
fn report(what: &str, n: usize) {
    if std::env::var_os("DIFF_COUNTS").is_some() {
        println!("COUNT\t{what}\t{n}");
    }
}

// ---------------------------------------------------------------------------
// `driver` (the low-level entry point)
// ---------------------------------------------------------------------------

pub fn driver_lines(side: Side, values: &[u64]) -> Vec<Vec<u8>> {
    let f = libs().driver(side);
    let raw = with_stdout_to_file(side.tag(), || {
        for &bits in values {
            unsafe { f(f64::from_bits(bits)) };
        }
    });
    split_lines(&raw, values.len(), side)
}

/// Calls `driver` in both shared objects for every bit pattern and asserts the
/// output is byte-identical.
pub fn diff_driver_bits(what: &str, values: &[u64]) {
    if values.is_empty() {
        return;
    }
    let c = driver_lines(Side::C, values);
    let r = driver_lines(Side::Rust, values);
    let mut bad = 0usize;
    for (i, &bits) in values.iter().enumerate() {
        if c[i] != r[i] {
            if bad < 10 {
                eprintln!(
                    "MISMATCH [{what}] driver(bits={bits:#018x} = {:e})\n  C   : {:?}\n  Rust: {:?}",
                    f64::from_bits(bits),
                    String::from_utf8_lossy(&c[i]),
                    String::from_utf8_lossy(&r[i]),
                );
            }
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "[{what}] {bad}/{} driver outputs differ", values.len());
    report(what, values.len());
}

pub fn diff_driver(what: &str, values: &[f64]) {
    let bits: Vec<u64> = values.iter().map(|v| v.to_bits()).collect();
    diff_driver_bits(what, &bits);
}

// ---------------------------------------------------------------------------
// `main` (the composed pipeline: scanf -> driver)
// ---------------------------------------------------------------------------

/// Presents `path` to the implementation under test as the whole of standard
/// input, positioned at offset 0.
///
/// The C side has to go through `freopen` for *every* call: `fseek` is not enough
/// because glibc satisfies a seek that lands inside the current stdio read buffer
/// without re-reading the file, which would hand the previous test input to
/// `scanf`.  (That is a real trap - it showed up as a spurious mismatch while
/// building this harness.)  The Rust side reads fd 0 unbuffered, so rewinding the
/// descriptor is exact.
fn reset_stdin(side: Side, path: &Path) {
    match side {
        Side::C => {
            let p = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
            let mode = CString::new("r").unwrap();
            let r = unsafe { freopen(p.as_ptr(), mode.as_ptr(), stdin) };
            assert!(!r.is_null(), "freopen({}) failed", path.display());
        }
        Side::Rust => {
            let r = unsafe { lseek(0, 0, 0 /* SEEK_SET */) };
            assert_eq!(r, 0, "lseek(0, 0, SEEK_SET) failed");
        }
    }
}

pub fn main_lines(side: Side, inputs: &[Vec<u8>]) -> Vec<Vec<u8>> {
    use std::os::unix::fs::FileExt;

    let m = libs().main(side);
    let in_path = tmp_dir().join(format!("stdin-{}-{}.bin", side.tag(), std::process::id()));
    let mut rets: Vec<c_int> = Vec::with_capacity(inputs.len());
    let raw = with_stdout_to_file(side.tag(), || {
        let saved0 = unsafe { dup(0) };
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&in_path)
            .expect("create stdin file");
        if side == Side::Rust {
            assert!(
                unsafe { dup2(file.as_raw_fd(), 0) } >= 0,
                "dup2 onto 0 failed"
            );
        }
        for inp in inputs {
            file.set_len(0).expect("truncate stdin file");
            if !inp.is_empty() {
                file.write_all_at(inp, 0).expect("write stdin file");
            }
            reset_stdin(side, &in_path);
            rets.push(unsafe { m() });
        }
        if saved0 >= 0 {
            unsafe { dup2(saved0, 0) };
            unsafe { close(saved0) };
        }
    });
    for (i, r) in rets.iter().enumerate() {
        assert_eq!(
            *r,
            0,
            "{} main() returned {} for input {:?}",
            side.name(),
            r,
            String::from_utf8_lossy(&inputs[i])
        );
    }
    split_lines(&raw, inputs.len(), side)
}

/// Calls `main` in both shared objects with each input as the whole of standard
/// input and asserts the output is byte-identical.
pub fn diff_main(what: &str, inputs: &[Vec<u8>]) {
    if inputs.is_empty() {
        return;
    }
    let c = main_lines(Side::C, inputs);
    let r = main_lines(Side::Rust, inputs);
    let mut bad = 0usize;
    for (i, inp) in inputs.iter().enumerate() {
        if c[i] != r[i] {
            if bad < 10 {
                eprintln!(
                    "MISMATCH [{what}] stdin={:?} (len {}, bytes {:02x?})\n  C   : {:?}\n  Rust: {:?}",
                    String::from_utf8_lossy(inp),
                    inp.len(),
                    &inp[..inp.len().min(40)],
                    String::from_utf8_lossy(&c[i]),
                    String::from_utf8_lossy(&r[i]),
                );
            }
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "[{what}] {bad}/{} main outputs differ", inputs.len());
    report(what, inputs.len());
}

pub fn diff_main_strs(what: &str, inputs: &[&str]) {
    let v: Vec<Vec<u8>> = inputs.iter().map(|s| s.as_bytes().to_vec()).collect();
    diff_main(what, &v);
}

// ---------------------------------------------------------------------------
// Real processes
// ---------------------------------------------------------------------------

pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
}

pub fn run_exe(exe: &Path, input: &[u8]) -> Run {
    use std::io::Write;
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    {
        let mut si = child.stdin.take().unwrap();
        let _ = si.write_all(input);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Runs both executables on the same standard input and compares stdout, stderr
/// and the exit status.
pub fn diff_exe(what: &str, inputs: &[Vec<u8>]) {
    let c_path = c_exe();
    let r_path = rust_exe();
    let mut bad = 0usize;
    for inp in inputs {
        let c = run_exe(&c_path, inp);
        let r = run_exe(&r_path, inp);
        if c.stdout != r.stdout || c.stderr != r.stderr || c.code != r.code {
            if bad < 10 {
                eprintln!(
                    "MISMATCH [{what}] exe stdin={:?}\n  C   : code={:?} out={:?} err={:?}\n  Rust: code={:?} out={:?} err={:?}",
                    String::from_utf8_lossy(inp),
                    c.code,
                    String::from_utf8_lossy(&c.stdout),
                    String::from_utf8_lossy(&c.stderr),
                    r.code,
                    String::from_utf8_lossy(&r.stdout),
                    String::from_utf8_lossy(&r.stderr),
                );
            }
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "[{what}] {bad}/{} executable runs differ", inputs.len());
    report(what, inputs.len());
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) + input generators
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    pub fn chance(&mut self, one_in: u64) -> bool {
        self.below(one_in) == 0
    }
    pub fn flip(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, s: &'a [T]) -> &'a T {
        &s[self.below(s.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// Minimal sequential test runner (`harness = false`)
// ---------------------------------------------------------------------------

pub type Case = (&'static str, fn());

/// Runs every case in order in the calling thread, reporting like libtest and
/// exiting non-zero if any case failed.  Sequential execution is required: the
/// differential comparison temporarily redirects the process-wide fd 0/1.
pub fn run_suite(suite: &str, cases: &[Case]) {
    use std::io::Write;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list") {
        for (name, _) in cases {
            println!("{name}: test");
        }
        return;
    }
    let filters: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .collect();

    // Build/load the artefacts up front so that no build output can ever land in
    // a captured window later on.
    let _ = libs();
    let _ = c_exe();

    println!("\nrunning {} cases in `{suite}`", cases.len());
    let mut passed = 0usize;
    let mut filtered = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    for (name, body) in cases {
        if !filters.is_empty() && !filters.iter().any(|f| name.contains(f)) {
            filtered += 1;
            continue;
        }
        print!("test {suite}::{name} ... ");
        let _ = std::io::stdout().flush();
        let t0 = std::time::Instant::now();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
        let dt = t0.elapsed();
        match outcome {
            Ok(()) => {
                println!("ok ({:.2}s)", dt.as_secs_f64());
                passed += 1;
            }
            Err(_) => {
                println!("FAILED");
                failed.push(name);
            }
        }
        let _ = std::io::stdout().flush();
    }
    println!(
        "\nresult: {}. {} passed; {} failed; {} filtered out",
        if failed.is_empty() { "ok" } else { "FAILED" },
        passed,
        failed.len(),
        filtered
    );
    let _ = std::io::stdout().flush();
    if !failed.is_empty() {
        eprintln!("failures in `{suite}`:");
        for f in &failed {
            eprintln!("    {suite}::{f}");
        }
        std::process::exit(1);
    }
}

pub const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
pub const DIGITS: &[u8] = b"0123456789";
pub const HEXDIGITS: &[u8] = b"0123456789abcdefABCDEF";

/// Randomly flips the case of ASCII letters.
pub fn random_case(rng: &mut Rng, s: &[u8]) -> Vec<u8> {
    s.iter()
        .map(|&c| {
            if c.is_ascii_alphabetic() && rng.flip() {
                c ^ 0x20
            } else {
                c
            }
        })
        .collect()
}

pub fn push_ws(rng: &mut Rng, out: &mut Vec<u8>) {
    let n = rng.below(4);
    for _ in 0..n {
        out.push(*rng.pick(&WS));
    }
}

pub fn push_sign(rng: &mut Rng, out: &mut Vec<u8>) {
    match rng.below(4) {
        0 => out.push(b'-'),
        1 => out.push(b'+'),
        _ => {}
    }
}

pub fn push_digits(rng: &mut Rng, out: &mut Vec<u8>, n: usize, alphabet: &[u8]) {
    for _ in 0..n {
        out.push(*rng.pick(alphabet));
    }
}

/// Appends between `min` and `max` (inclusive) random characters of `alphabet`.
pub fn push_n_digits(
    rng: &mut Rng,
    out: &mut Vec<u8>,
    min: usize,
    max: usize,
    alphabet: &[u8],
) {
    let n = min + rng.below((max - min + 1) as u64) as usize;
    for _ in 0..n {
        out.push(*rng.pick(alphabet));
    }
}
