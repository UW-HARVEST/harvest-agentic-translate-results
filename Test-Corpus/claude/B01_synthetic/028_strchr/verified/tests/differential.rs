//! Differential tests: the C shared object vs the Rust shared object.
//!
//! Every call goes through `libloading` + the exported C-ABI symbols of BOTH
//! objects (`foo`, `driver`, `main`) — the Rust implementation is never called
//! directly, so the `#[no_mangle]` wrappers are part of what is under test.
//!
//! Layout of this file:
//!   * infrastructure  — artifact discovery/building, stdout capture, child runner, PRNG
//!   * `b*` tests      — Phase B, valid-path rows of CONFIGS.md
//!   * `c*` tests      — Phase C, error/rejection rows of ERRORS.md
//!   * `d*` tests      — Phase D, symbol parity
//!   * `zz_child_*`    — helper "test" that acts as a fresh child process worker

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs;
use std::io::Write as _;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// artifact discovery / building
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Scratch directory for anything this test suite produces.
fn artifacts_dir() -> PathBuf {
    let d = manifest_dir().join("target").join("difftest");
    fs::create_dir_all(&d).expect("create target/difftest");
    d
}

/// `target/<profile>/` (current_exe is `target/<profile>/deps/<test>-<hash>`).
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn mtime(p: &Path) -> Option<SystemTime> {
    fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

/// Newest mtime over the Rust sources, used to detect a stale cdylib.
fn newest_rust_src_mtime() -> SystemTime {
    let mut newest = SystemTime::UNIX_EPOCH;
    for name in ["src/lib.rs", "src/core_impl.rs", "src/main.rs"] {
        if let Some(t) = mtime(&manifest_dir().join(name)) {
            if t > newest {
                newest = t;
            }
        }
    }
    newest
}

fn run(cmd: &mut Command) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn {cmd:?}: {e}"));
    assert!(
        out.status.success(),
        "command {cmd:?} failed: {}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The C shared library, compiled from the untouched `c_src/src/main.c`.
fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = manifest_dir().join("c_src/src/main.c");
        // Prefer the one built next to the cmake artifacts, if it is fresh.
        let cmake_so = manifest_dir().join("c_src/build/libcdriver.so");
        if let (Some(so_t), Some(src_t)) = (mtime(&cmake_so), mtime(&src)) {
            if so_t >= src_t {
                return cmake_so;
            }
        }
        let out = artifacts_dir().join("libcdriver.so");
        run(Command::new("gcc").args(["-shared", "-fPIC", "-o"]).arg(&out).arg(&src));
        out
    })
    .as_path()
}

/// The Rust cdylib. `cargo test` does not necessarily build the `cdylib`
/// target, so fall back to invoking rustc directly (the crate has no
/// dependencies, so this is exactly what cargo would do).
fn rust_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let cargo_so = profile_dir().join("libdriver.so");
        if let Some(t) = mtime(&cargo_so) {
            if t >= newest_rust_src_mtime() {
                return cargo_so;
            }
        }
        let out = artifacts_dir().join("libdriver.so");
        run(Command::new("rustc")
            .args(["--edition", "2021", "--crate-type", "cdylib", "--crate-name", "driver"])
            .arg(manifest_dir().join("src/lib.rs"))
            .arg("-o")
            .arg(&out));
        out
    })
    .as_path()
}

/// The C executable (cmake target `driver`).
fn c_exe_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = manifest_dir().join("c_src/src/main.c");
        let cmake_exe = manifest_dir().join("c_src/build/driver");
        if let (Some(t), Some(src_t)) = (mtime(&cmake_exe), mtime(&src)) {
            if t >= src_t {
                return cmake_exe;
            }
        }
        let out = artifacts_dir().join("c_driver");
        run(Command::new("gcc").arg("-o").arg(&out).arg(&src));
        out
    })
    .as_path()
}

/// The Rust executable (cargo bin target `driver`).
fn rust_exe_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let cargo_exe = profile_dir().join("driver");
        if let Some(t) = mtime(&cargo_exe) {
            if t >= newest_rust_src_mtime() {
                return cargo_exe;
            }
        }
        let out = artifacts_dir().join("rust_driver");
        run(Command::new("rustc")
            .args(["--edition", "2021"])
            .arg(manifest_dir().join("src/main.rs"))
            .arg("-o")
            .arg(&out));
        out
    })
    .as_path()
}

fn c_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| unsafe { Library::new(c_so_path()).expect("dlopen C .so") })
}

fn rust_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| unsafe { Library::new(rust_so_path()).expect("dlopen Rust .so") })
}

// Exported signatures.
type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
/// Same symbol, but with the `char` parameter declared as `int`: on the SysV
/// x86-64 ABI this is how a C caller that has not seen a prototype passes the
/// argument, and it lets the tests push out-of-`char`-range values across the
/// FFI boundary.
type FooIntFn = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
type DriverFn = unsafe extern "C" fn(*const c_char);

fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

fn c_foo() -> FooFn {
    sym(c_lib(), b"foo\0")
}
fn rust_foo() -> FooFn {
    sym(rust_lib(), b"foo\0")
}
fn c_foo_int() -> FooIntFn {
    sym(c_lib(), b"foo\0")
}
fn rust_foo_int() -> FooIntFn {
    sym(rust_lib(), b"foo\0")
}
fn c_driver() -> DriverFn {
    sym(c_lib(), b"driver\0")
}
fn rust_driver() -> DriverFn {
    sym(rust_lib(), b"driver\0")
}

// ---------------------------------------------------------------------------
// stdout capture (for the `driver` export, which printf()s)
// ---------------------------------------------------------------------------

static IO_LOCK: Mutex<()> = Mutex::new(());
static CAP_SEQ: AtomicU64 = AtomicU64::new(0);

fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = IO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // libtest prints its progress through `io::stdout()` from other test
    // threads; holding that lock keeps those writes out of our capture window.
    let _stdout_guard = std::io::stdout().lock();
    let path = artifacts_dir().join(format!("capture_{}.bin", CAP_SEQ.fetch_add(1, Ordering::SeqCst)));
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    let data;
    unsafe {
        // Nothing of ours may still sit in a buffer aimed at the real stdout.
        libc::fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();

        let fd = libc::open(cpath.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o644);
        assert!(fd >= 0, "open capture file");
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1)");
        assert!(libc::dup2(fd, 1) >= 0, "dup2 -> 1");

        f();

        // C stdio is fully buffered when stdout is a file; the Rust side
        // flushes inside `driver`, but flush again for good measure.
        libc::fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();

        assert!(libc::dup2(saved, 1) >= 0, "restore fd 1");
        libc::close(saved);
        libc::close(fd);
        data = fs::read(&path).expect("read capture file");
    }
    let _ = fs::remove_file(&path);
    data
}

fn c_driver_output(s: &[u8]) -> Vec<u8> {
    let cs = CString::new(s).expect("no interior NUL");
    let f = c_driver();
    capture_stdout(|| unsafe { f(cs.as_ptr()) })
}

fn rust_driver_output(s: &[u8]) -> Vec<u8> {
    let cs = CString::new(s).expect("no interior NUL");
    let f = rust_driver();
    capture_stdout(|| unsafe { f(cs.as_ptr()) })
}

// ---------------------------------------------------------------------------
// deterministic PRNG (fixed seeds -> reproducible runs)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    /// Any byte except NUL (a NUL would end the C string).
    fn nonzero_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
}

/// Random NUL-free string over `alphabet`, length in `0..=max_len`.
fn rand_string(rng: &mut Rng, alphabet: &[u8], max_len: usize) -> Vec<u8> {
    let len = rng.below(max_len + 1);
    (0..len).map(|_| rng.pick(alphabet)).collect()
}

// ---------------------------------------------------------------------------
// child-process runner: gives every `main`/crash test a *fresh* process, so
// C stdio state (EOF flags, buffers) and Rust's BufReader are pristine.
// ---------------------------------------------------------------------------

const ENV_MODE: &str = "DIFF_CHILD_MODE";
const ENV_SO: &str = "DIFF_CHILD_SO";
const ENV_OUT: &str = "DIFF_CHILD_OUT";

#[derive(Debug, PartialEq, Eq)]
struct ChildResult {
    stdout: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

/// Run `mode` against `so` in a fresh child process, feeding `stdin_bytes`
/// (when `Some`) on fd 0 and collecting whatever the callee writes to fd 1.
fn run_child(mode: &str, so: &Path, stdin_bytes: Option<&[u8]>) -> ChildResult {
    let seq = CAP_SEQ.fetch_add(1, Ordering::SeqCst);
    let out_path = artifacts_dir().join(format!("child_out_{seq}.bin"));
    let _ = fs::remove_file(&out_path);

    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "zz_child_worker", "--nocapture", "--test-threads=1"])
        .env(ENV_MODE, mode)
        .env(ENV_SO, so)
        .env(ENV_OUT, &out_path)
        // libtest's own chatter must not land in the captured output.
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    match stdin_bytes {
        Some(bytes) => {
            let in_path = artifacts_dir().join(format!("child_in_{seq}.bin"));
            fs::write(&in_path, bytes).expect("write child stdin file");
            cmd.stdin(Stdio::from(fs::File::open(&in_path).expect("open child stdin file")));
        }
        None => {
            cmd.stdin(Stdio::null());
        }
    }

    let mut child = cmd.spawn().expect("spawn child worker");
    let status = wait_with_timeout(&mut child, Duration::from_secs(60));
    let stdout = fs::read(&out_path).unwrap_or_default();
    let _ = fs::remove_file(&out_path);
    let _ = fs::remove_file(artifacts_dir().join(format!("child_in_{seq}.bin")));

    use std::os::unix::process::ExitStatusExt;
    ChildResult {
        stdout,
        code: status.code(),
        signal: status.signal(),
    }
}

/// Like `run_child`, but stdin is an already-open file/handle.
fn run_child_with_stdin(mode: &str, so: &Path, stdin: Stdio) -> ChildResult {
    let seq = CAP_SEQ.fetch_add(1, Ordering::SeqCst);
    let out_path = artifacts_dir().join(format!("child_out_{seq}.bin"));
    let _ = fs::remove_file(&out_path);

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "zz_child_worker", "--nocapture", "--test-threads=1"])
        .env(ENV_MODE, mode)
        .env(ENV_SO, so)
        .env(ENV_OUT, &out_path)
        .stdin(stdin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child worker");

    let status = wait_with_timeout(&mut child, Duration::from_secs(60));
    let stdout = fs::read(&out_path).unwrap_or_default();
    let _ = fs::remove_file(&out_path);

    use std::os::unix::process::ExitStatusExt;
    ChildResult {
        stdout,
        code: status.code(),
        signal: status.signal(),
    }
}

fn wait_with_timeout(child: &mut std::process::Child, limit: Duration) -> std::process::ExitStatus {
    let start = std::time::Instant::now();
    loop {
        if let Some(st) = child.try_wait().expect("try_wait") {
            return st;
        }
        if start.elapsed() > limit {
            let _ = child.kill();
            return child.wait().expect("wait after kill");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The child worker. Acts as a plain no-op test unless the env is set, in which
/// case it dlopens one shared object, performs one operation, and exits.
#[test]
fn zz_child_worker() {
    let mode = match std::env::var(ENV_MODE) {
        Ok(m) => m,
        Err(_) => return, // normal test run: nothing to do
    };
    let so = std::env::var(ENV_SO).expect("DIFF_CHILD_SO");
    let out = std::env::var(ENV_OUT).expect("DIFF_CHILD_OUT");

    unsafe {
        let cout = CString::new(out).unwrap();
        let fd = libc::open(cout.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o644);
        assert!(fd >= 0, "child: open out file");
        assert!(libc::dup2(fd, 1) >= 0, "child: dup2 -> 1");
    }

    let lib = unsafe { Library::new(&so).expect("child: dlopen") };
    unsafe {
        match mode.as_str() {
            // int main(void)
            "main" => {
                let f: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"main\0").unwrap();
                let rc = f();
                libc::fflush(std::ptr::null_mut());
                let _ = std::io::stdout().flush();
                std::process::exit(rc);
            }
            // void driver(const char*) for every NUL-terminated record on stdin
            "driver_batch" => {
                let f: Symbol<DriverFn> = lib.get(b"driver\0").unwrap();
                let mut raw = Vec::new();
                std::io::Read::read_to_end(&mut std::io::stdin(), &mut raw).unwrap();
                // records are NUL-terminated and themselves NUL-free
                let mut start = 0usize;
                for i in 0..raw.len() {
                    if raw[i] == 0 {
                        let rec = CString::new(&raw[start..i]).unwrap();
                        f(rec.as_ptr());
                        start = i + 1;
                    }
                }
                libc::fflush(std::ptr::null_mut());
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            // foo(NULL, 'A')
            "foo_null" => {
                let f: Symbol<FooFn> = lib.get(b"foo\0").unwrap();
                let r = f(std::ptr::null(), b'A' as c_char);
                println!("unexpected return {r}");
            }
            // driver(NULL)
            "driver_null" => {
                let f: Symbol<DriverFn> = lib.get(b"driver\0").unwrap();
                f(std::ptr::null());
            }
            // foo(s, '\0'): strchr matches the terminating NUL for ever
            "foo_nul_needle" => {
                let f: Symbol<FooFn> = lib.get(b"foo\0").unwrap();
                let s = CString::new("AAxxAA").unwrap();
                let r = f(s.as_ptr(), 0);
                println!("unexpected return {r}");
            }
            other => panic!("unknown child mode {other}"),
        }
    }
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// executable-level helpers (end-to-end `main`, same code path as the .so)
// ---------------------------------------------------------------------------

fn run_exe(exe: &Path, stdin_bytes: &[u8]) -> ChildResult {
    let seq = CAP_SEQ.fetch_add(1, Ordering::SeqCst);
    let in_path = artifacts_dir().join(format!("exe_in_{seq}.bin"));
    fs::write(&in_path, stdin_bytes).expect("write exe stdin file");
    let out = Command::new(exe)
        .stdin(Stdio::from(fs::File::open(&in_path).unwrap()))
        .output()
        .expect("run exe");
    let _ = fs::remove_file(&in_path);
    use std::os::unix::process::ExitStatusExt;
    ChildResult {
        stdout: out.stdout,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Run `driver` once per input inside one fresh child process per library and
/// return everything it wrote to stdout. Robust against libtest's own output
/// (the child's fd 1 is a private file), and cheap: two processes per batch.
fn driver_batch(so: &Path, inputs: &[Vec<u8>]) -> ChildResult {
    let mut payload = Vec::new();
    for i in inputs {
        assert!(!i.contains(&0), "batch records must be NUL-free");
        payload.extend_from_slice(i);
        payload.push(0);
    }
    run_child("driver_batch", so, Some(&payload))
}

/// Concatenated reference output for a batch.
fn expected_batch(inputs: &[Vec<u8>]) -> Vec<u8> {
    let mut v = Vec::new();
    for i in inputs {
        v.extend_from_slice(&expected(count(i, b'A'), count(i, b'x')));
    }
    v
}

fn expected(a: usize, x: usize) -> Vec<u8> {
    format!("A: {a}\nx: {x}\n").into_bytes()
}

/// Reference count: occurrences of `c` in the NUL-terminated prefix.
fn count(bytes: &[u8], c: u8) -> usize {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    bytes[..end].iter().filter(|&&b| b == c).count()
}

/// `main` is only well defined when `in[1000]` ends up NUL-terminated, i.e. the
/// input is shorter than the buffer or contains a NUL inside the first 1000
/// bytes. Otherwise the C `strchr` walk leaves the array (ERRORS.md row 10).
fn main_input_is_well_defined(input: &[u8]) -> bool {
    input.len() < 1000 || input[..1000].contains(&0)
}

fn parse_counts(out: &[u8]) -> Option<(usize, usize)> {
    let s = std::str::from_utf8(out).ok()?;
    let mut it = s.lines();
    let a = it.next()?.strip_prefix("A: ")?.parse().ok()?;
    let x = it.next()?.strip_prefix("x: ")?.parse().ok()?;
    Some((a, x))
}

/// Assertion used for inputs that leave `in` unterminated. The C behavior is
/// undefined there: it keeps scanning whatever the process' stack holds behind
/// the 1000-byte array, which was measured to depend on the environment the
/// process was started in (identical input, same binary: `A: 134` from a shell,
/// `A: 135` when spawned from the test harness). What is still guaranteed:
///   * the Rust translation stops at the end of the array, deterministically,
///     which is what the compiled C program does in a normal environment;
///   * the C can therefore only ever see the same or MORE occurrences.
fn assert_unterminated_buffer_consistent(label: &str, c: &ChildResult, r: &ChildResult, model: (usize, usize)) {
    assert_eq!(
        r.stdout,
        expected(model.0, model.1),
        "{label}: Rust must report exactly the first 1000 bytes"
    );
    assert_eq!(r.code, Some(0), "{label}: Rust must exit 0");
    if c.code != Some(0) {
        eprintln!("note: {label}: C died in undefined behavior ({c:?}) — nothing to compare");
        return;
    }
    let (ca, cx) = parse_counts(&c.stdout).unwrap_or_else(|| panic!("{label}: unparsable C output {c:?}"));
    assert!(
        ca >= model.0 && cx >= model.1,
        "{label}: C reported fewer hits ({ca},{cx}) than the buffer holds {model:?}"
    );
    if (ca, cx) != model {
        eprintln!(
            "note: {label}: C scanned {} extra 'A' / {} extra 'x' past the end of in[1000] \
             (documented UB, ERRORS.md row 10)",
            ca - model.0,
            cx - model.1
        );
    } else {
        assert_eq!(c.stdout, r.stdout, "{label}: outputs must match when C stays in bounds");
    }
}

// ===========================================================================
// Phase B — valid-path differential tests (rows of CONFIGS.md)
// ===========================================================================

/// CONFIGS rows 1-3: `foo` with the two needles the library itself uses plus
/// arbitrary ASCII needles, over randomized strings from a tiny alphabet (so
/// hits are dense) and from a wide alphabet (so hits are sparse).
#[test]
fn b01_foo_ascii_needles_random() {
    let cf = c_foo();
    let rf = rust_foo();
    let mut rng = Rng::new(0x1234_5678_9abc_def0);
    let alphabets: [&[u8]; 4] = [b"Ax", b"AaxX", b"AxBy \n\t", b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 "];
    for iter in 0..4000 {
        let alpha = alphabets[iter % alphabets.len()];
        let s = rand_string(&mut rng, alpha, 200);
        let needle = if iter % 3 == 0 {
            b'A'
        } else if iter % 3 == 1 {
            b'x'
        } else {
            rng.nonzero_byte() & 0x7f | 1
        };
        let cs = CString::new(s.clone()).unwrap();
        let (c, r) = unsafe { (cf(cs.as_ptr(), needle as c_char), rf(cs.as_ptr(), needle as c_char)) };
        assert_eq!(c, r, "foo mismatch: needle={needle:?} s={:?}", String::from_utf8_lossy(&s));
        assert_eq!(c as usize, count(&s, needle), "reference count mismatch (C is truth: {c})");
    }
}

/// CONFIGS row 4: full byte alphabet 1..=255, needles 1..=255 (including
/// needles with the high bit set, i.e. negative `char` values).
#[test]
fn b02_foo_full_byte_alphabet_random() {
    let cf = c_foo();
    let rf = rust_foo();
    let mut rng = Rng::new(0x0bad_c0de_dead_beef);
    for _ in 0..4000 {
        let len = rng.below(300);
        let s: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        // half the time pick a needle that is guaranteed to be present
        let needle = if !s.is_empty() && rng.below(2) == 0 {
            s[rng.below(s.len())]
        } else {
            rng.nonzero_byte()
        };
        let cs = CString::new(s.clone()).unwrap();
        let (c, r) = unsafe { (cf(cs.as_ptr(), needle as c_char), rf(cs.as_ptr(), needle as c_char)) };
        assert_eq!(c, r, "foo mismatch: needle={needle} len={len}");
        assert_eq!(c as usize, count(&s, needle));
    }
}

/// CONFIGS rows 4-6, exhaustive over the needle axis: every legal needle value
/// (1..=255; 0 is ERRORS.md row 3) against several fixed and random strings,
/// plus a 100 kB string so the counting loop runs far past any 8/16/32-bit
/// boundary.
#[test]
fn b02b_foo_all_needles_exhaustive() {
    let cf = c_foo();
    let rf = rust_foo();
    let mut rng = Rng::new(0x1357_9bdf_0246_8ace);
    let mut strings: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"Ax".to_vec(),
        b"The quick brown fox jumps over the lazy dog".to_vec(),
        (1u8..=255).collect(),
        (1u8..=255).chain(1u8..=255).collect(),
    ];
    for len in [1usize, 7, 64, 999, 1000] {
        strings.push((0..len).map(|_| rng.nonzero_byte()).collect());
    }
    // large input: 100_000 bytes, ~50_000 hits for 'A'
    strings.push((0..100_000).map(|i| if i % 2 == 0 { b'A' } else { b'x' }).collect());
    for s in &strings {
        let cs = CString::new(s.clone()).unwrap();
        for needle in 1u8..=255 {
            let (a, b) = unsafe { (cf(cs.as_ptr(), needle as c_char), rf(cs.as_ptr(), needle as c_char)) };
            assert_eq!(a, b, "foo mismatch: needle={needle} len={}", s.len());
            assert_eq!(a as usize, count(s, needle), "reference mismatch: needle={needle}");
        }
    }
}

/// CONFIGS rows 5-9: hand-picked boundary shapes — empty, single byte,
/// hit at the very first / very last position, all-hits, long runs, and the
/// 999 / 1000 / 1001 / 4096 length boundaries of `main`'s buffer.
#[test]
fn b03_foo_boundary_shapes() {
    let cf = c_foo();
    let rf = rust_foo();
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"A".to_vec(),
        b"x".to_vec(),
        b"B".to_vec(),
        b"AA".to_vec(),
        b"Ax".to_vec(),
        b"xA".to_vec(),
        b"ABA".to_vec(),
        b"BBBA".to_vec(),
        b"ABBB".to_vec(),
        b"AAAAAAAAAA".to_vec(),
        b"xxxxxxxxxx".to_vec(),
        b"aXaXaX".to_vec(),
        b"\x7f\x80\xff\x01".to_vec(),
        b"\xff\xff\xff".to_vec(),
    ];
    for len in [1usize, 2, 3, 255, 256, 511, 512, 998, 999, 1000, 1001, 4095, 4096] {
        cases.push(vec![b'A'; len]);
        cases.push(vec![b'x'; len]);
        let mut mixed = vec![b'B'; len];
        mixed[0] = b'A';
        mixed[len - 1] = b'x';
        cases.push(mixed);
    }
    let needles: [u8; 8] = [b'A', b'x', b'a', b'X', b'B', 0x01, 0x7f, 0xff];
    for s in &cases {
        let cs = CString::new(s.clone()).unwrap();
        for &n in &needles {
            let (c, r) = unsafe { (cf(cs.as_ptr(), n as c_char), rf(cs.as_ptr(), n as c_char)) };
            assert_eq!(c, r, "foo mismatch: needle={n} len={}", s.len());
            assert_eq!(c as usize, count(s, n));
        }
    }
}

/// CONFIGS rows 10-11: the `driver` export over randomized inputs — byte-exact
/// comparison of everything it writes to stdout.
#[test]
fn b04_driver_random() {
    let mut rng = Rng::new(0xfeed_face_cafe_babe);
    let alphabets: [&[u8]; 5] = [b"Ax", b"AaxX", b"AxB", b"BCD", b"Ax \n"];
    let inputs: Vec<Vec<u8>> = (0..2000)
        .map(|iter| rand_string(&mut rng, alphabets[iter % alphabets.len()], 300))
        .collect();
    let c = driver_batch(c_so_path(), &inputs);
    let r = driver_batch(rust_so_path(), &inputs);
    assert_eq!(c.code, Some(0));
    assert_eq!(c.stdout, r.stdout, "driver batch mismatch");
    assert_eq!(c, r, "driver batch status mismatch");
    assert_eq!(c.stdout, expected_batch(&inputs), "C output disagrees with reference model");
}

/// Same rows as `b04`, but calling `driver` in-process (through `dlsym`) with
/// stdout temporarily redirected — exercises the export from a live process
/// instead of a one-shot child.
#[test]
fn b04b_driver_random_in_process() {
    let mut rng = Rng::new(0x0102_0304_0506_0708);
    for iter in 0..40 {
        let alphabets: [&[u8]; 3] = [b"Ax", b"AaxX", b"AxB"];
        let s = rand_string(&mut rng, alphabets[iter % 3], 120);
        let c = c_driver_output(&s);
        let r = rust_driver_output(&s);
        assert_eq!(
            c,
            r,
            "driver mismatch for {:?}: C={:?} Rust={:?}",
            String::from_utf8_lossy(&s),
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
        assert_eq!(c, expected(count(&s, b'A'), count(&s, b'x')));
    }
}

/// CONFIGS rows 12-14: `driver` over the distinct shapes it distinguishes —
/// only 'A', only 'x', both, neither, wrong case, non-ASCII, and long inputs.
#[test]
fn b05_driver_shapes() {
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"A".to_vec(),
        b"x".to_vec(),
        b"Ax".to_vec(),
        b"xA".to_vec(),
        b"aX".to_vec(),
        b"hello world".to_vec(),
        b"AAAAxxxx".to_vec(),
        b"AxAxAxAxAx".to_vec(),
        b"\xc3\xa4\xc3\xb6A\xffx".to_vec(),
        b"no matches here!".to_vec(),
        b"\n\t\r A x".to_vec(),
    ];
    for len in [1usize, 999, 1000, 1001, 2000] {
        cases.push(vec![b'A'; len]);
        cases.push(vec![b'x'; len]);
        let mut alt = vec![b'A'; len];
        for (i, b) in alt.iter_mut().enumerate() {
            if i % 2 == 1 {
                *b = b'x';
            }
        }
        cases.push(alt);
    }
    let c = driver_batch(c_so_path(), &cases);
    let r = driver_batch(rust_so_path(), &cases);
    assert_eq!(c, r, "driver shape mismatch");
    assert_eq!(c.stdout, expected_batch(&cases), "C output disagrees with reference model");
}

/// CONFIGS rows 15-17: the `main` export through the .so, over the input-length
/// boundaries of the 1000-byte buffer, run in a fresh process each time.
#[test]
fn b06_main_via_so_lengths() {
    let mut rng = Rng::new(0x5eed_1234_5678_90ab);
    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"A".to_vec(),
        b"x".to_vec(),
        b"Ax\n".to_vec(),
        b"hello\nworld\n".to_vec(),
    ];
    // Lengths that keep `in` NUL-terminated, i.e. the well-defined domain.
    for len in [1usize, 2, 3, 17, 500, 997, 998, 999] {
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(rng.pick(b"AaxXB \n"));
        }
        inputs.push(v);
    }
    // Longer than the buffer, but with a NUL inside the first 1000 bytes, so
    // the C string is still terminated: also well defined.
    for len in [1000usize, 1001, 2000, 5000] {
        let mut v: Vec<u8> = (0..len).map(|_| rng.pick(b"AaxXB \n")).collect();
        v[999 - (len % 7)] = 0;
        inputs.push(v);
    }
    for input in &inputs {
        assert!(main_input_is_well_defined(input));
        let c = run_child("main", c_so_path(), Some(input));
        let r = run_child("main", rust_so_path(), Some(input));
        assert_eq!(c, r, "main(.so) mismatch for input len {}", input.len());
        let truncated = &input[..input.len().min(1000)];
        assert_eq!(
            c.stdout,
            expected(count(truncated, b'A'), count(truncated, b'x')),
            "unexpected C output"
        );
        assert_eq!(c.code, Some(0));
    }
}

/// CONFIGS row 18: embedded NUL bytes — the C string ends at the first NUL, so
/// everything behind it is invisible even though `fread` read it.
#[test]
fn b07_main_via_so_embedded_nul() {
    let inputs: Vec<Vec<u8>> = vec![
        b"\x00".to_vec(),
        b"\x00AAAxxx".to_vec(),
        b"AAx\x00AAAA".to_vec(),
        b"AAx\x00xxxx\x00AA".to_vec(),
        {
            let mut v = vec![b'A'; 1000];
            v[500] = 0;
            v
        },
        {
            let mut v = vec![b'x'; 1200];
            v[999] = 0;
            v
        },
    ];
    for input in &inputs {
        let c = run_child("main", c_so_path(), Some(input));
        let r = run_child("main", rust_so_path(), Some(input));
        assert_eq!(c, r, "main(.so) mismatch for embedded-NUL input");
        let truncated = &input[..input.len().min(1000)];
        assert_eq!(c.stdout, expected(count(truncated, b'A'), count(truncated, b'x')));
    }
}

/// CONFIGS rows 19-20: the two executables end to end, randomized inputs
/// (binary-safe, including NULs and bytes >= 0x80).
#[test]
fn b08_executables_random() {
    let mut rng = Rng::new(0xabcd_ef01_2345_6789);
    for iter in 0..150 {
        let len = rng.below(1500);
        let input: Vec<u8> = (0..len)
            .map(|_| {
                if iter % 4 == 0 {
                    // binary, may contain NUL
                    (rng.next_u64() & 0xff) as u8
                } else {
                    rng.pick(b"AaxX \n\t\xff")
                }
            })
            .collect();
        let c = run_exe(c_exe_path(), &input);
        let r = run_exe(rust_exe_path(), &input);
        let truncated = &input[..input.len().min(1000)];
        let model = (count(truncated, b'A'), count(truncated, b'x'));
        if !main_input_is_well_defined(&input) {
            // `in` is left unterminated -> C reads past the array (ERRORS.md row 10)
            assert_unterminated_buffer_consistent(&format!("exe iter {iter} len {len}"), &c, &r, model);
            continue;
        }
        if c != r {
            let p = artifacts_dir().join("b08_fail.bin");
            fs::write(&p, &input).unwrap();
            panic!("exe mismatch for len {len} (iter {iter}), input dumped to {p:?}\nC={c:?}\nR={r:?}");
        }
        assert_eq!(c.stdout, expected(model.0, model.1));
    }
}

/// CONFIGS row 20, wider shape sweep: six different input-shape families
/// (tiny, buffer-boundary, uniform-random binary, boundary lengths, mixed with
/// NULs, oversized) against both executables.
#[test]
fn b08b_executables_shape_fuzz() {
    let mut rng = Rng::new(0x2468_ace0_1357_9bdf);
    let mut compared = 0usize;
    let mut ub = 0usize;
    for t in 0..240 {
        let (len, data): (usize, Vec<u8>) = match t % 6 {
            0 => {
                let l = rng.below(21);
                (l, (0..l).map(|_| rng.pick(b"Ax")).collect())
            }
            1 => {
                let l = 990 + rng.below(21);
                (l, (0..l).map(|_| rng.pick(b"Ax")).collect())
            }
            2 => {
                let l = rng.below(1500);
                (l, (0..l).map(|_| (rng.next_u64() & 0xff) as u8).collect())
            }
            3 => {
                let l = *[0usize, 1, 999, 1000, 1001, 4096].get(rng.below(6)).unwrap();
                (l, (0..l).map(|_| rng.pick(b"AxB")).collect())
            }
            4 => {
                let l = rng.below(51);
                (l, (0..l).map(|_| rng.pick(b"AaxX \n\t\x00\xff")).collect())
            }
            _ => {
                let l = 1400 + rng.below(1600);
                (l, (0..l).map(|_| rng.pick(b"AxB\x00")).collect())
            }
        };
        let c = run_exe(c_exe_path(), &data);
        let r = run_exe(rust_exe_path(), &data);
        let head = &data[..data.len().min(1000)];
        let model = (count(head, b'A'), count(head, b'x'));
        if main_input_is_well_defined(&data) {
            compared += 1;
            if c != r {
                let p = artifacts_dir().join("b08b_fail.bin");
                fs::write(&p, &data).unwrap();
                panic!("exe mismatch len={len} (t={t}), dumped to {p:?}\nC={c:?}\nR={r:?}");
            }
            assert_eq!(c.stdout, expected(model.0, model.1));
        } else {
            ub += 1;
            assert_unterminated_buffer_consistent(&format!("exe shape t={t} len={len}"), &c, &r, model);
        }
    }
    assert!(compared > 150, "expected most shapes to be well defined, got {compared} (+{ub} UB)");
}

/// CONFIGS row 21: stdin delivered in several small chunks with pauses, so
/// `fread`'s internal retry loop (and the Rust equivalent) is exercised.
#[test]
fn b09_executables_chunked_pipe_stdin() {
    for exe_pair in [(c_exe_path(), rust_exe_path())] {
        let mut outs = Vec::new();
        for exe in [exe_pair.0, exe_pair.1] {
            let mut child = Command::new(exe)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("spawn");
            {
                let mut si = child.stdin.take().unwrap();
                for chunk in [&b"AAxx"[..], b"AA", b"xxxxA", b"BBBB", b"A"] {
                    si.write_all(chunk).unwrap();
                    si.flush().unwrap();
                    std::thread::sleep(Duration::from_millis(15));
                }
                // dropping `si` closes the pipe -> EOF
            }
            let out = child.wait_with_output().unwrap();
            outs.push(out.stdout);
        }
        assert_eq!(outs[0], outs[1], "chunked-stdin mismatch");
        assert_eq!(outs[0], expected(count(b"AAxxAAxxxxABBBBA", b'A'), count(b"AAxxAAxxxxABBBBA", b'x')));
    }
}

/// CONFIGS row 22: stdin from /dev/null and from a regular file.
#[test]
fn b10_main_stdin_sources() {
    // /dev/null
    let c = run_child_with_stdin("main", c_so_path(), Stdio::null());
    let r = run_child_with_stdin("main", rust_so_path(), Stdio::null());
    assert_eq!(c, r, "main(.so) mismatch for /dev/null stdin");
    assert_eq!(c.stdout, expected(0, 0));

    // regular file
    let f = artifacts_dir().join("b10_input.bin");
    fs::write(&f, b"AAAxxAAxB").unwrap();
    let c = run_child_with_stdin("main", c_so_path(), Stdio::from(fs::File::open(&f).unwrap()));
    let r = run_child_with_stdin("main", rust_so_path(), Stdio::from(fs::File::open(&f).unwrap()));
    assert_eq!(c, r, "main(.so) mismatch for regular-file stdin");
    assert_eq!(c.stdout, expected(count(b"AAAxxAAxB", b'A'), count(b"AAAxxAAxB", b'x')));

    // the executables, same two sources
    let ce = Command::new(c_exe_path()).stdin(Stdio::null()).output().unwrap();
    let re = Command::new(rust_exe_path()).stdin(Stdio::null()).output().unwrap();
    assert_eq!(ce.stdout, re.stdout);
    assert_eq!(ce.stdout, expected(0, 0));
    let _ = fs::remove_file(&f);
}

// ===========================================================================
// Phase C — error-path differential tests (rows of ERRORS.md)
// ===========================================================================

/// ERRORS row 1: `foo(NULL, 'A')` — `strchr(NULL, …)` dereferences a null
/// pointer. Both objects must fail the same way (same fatal signal).
#[test]
fn c01_foo_null_pointer() {
    let c = run_child("foo_null", c_so_path(), None);
    let r = run_child("foo_null", rust_so_path(), None);
    assert_eq!(c.signal, Some(libc::SIGSEGV), "C did not SIGSEGV: {c:?}");
    assert_eq!(r.signal, c.signal, "signal mismatch: C={c:?} Rust={r:?}");
    assert_eq!(r.code, c.code, "exit-code mismatch: C={c:?} Rust={r:?}");
    assert_eq!(r.stdout, c.stdout, "stdout mismatch: C={c:?} Rust={r:?}");
}

/// ERRORS row 2: `driver(NULL)` — propagates row 1 through `driver`; note that
/// the buffered "A: …" line is never flushed on either side.
#[test]
fn c02_driver_null_pointer() {
    let c = run_child("driver_null", c_so_path(), None);
    let r = run_child("driver_null", rust_so_path(), None);
    assert_eq!(c.signal, Some(libc::SIGSEGV), "C did not SIGSEGV: {c:?}");
    assert_eq!(r.signal, c.signal, "signal mismatch: C={c:?} Rust={r:?}");
    assert_eq!(r.code, c.code, "exit-code mismatch: C={c:?} Rust={r:?}");
    assert_eq!(r.stdout, c.stdout, "stdout mismatch: C={c:?} Rust={r:?}");
}

/// ERRORS row 3: `foo(s, '\0')` — `strchr` matches the terminating NUL, so the
/// loop never sees NULL and walks off the end of the object until it faults.
#[test]
fn c03_foo_nul_needle_runs_off_the_end() {
    let c = run_child("foo_nul_needle", c_so_path(), None);
    let r = run_child("foo_nul_needle", rust_so_path(), None);
    assert_eq!(c.signal, Some(libc::SIGSEGV), "C did not SIGSEGV: {c:?}");
    assert_eq!(r.signal, c.signal, "signal mismatch: C={c:?} Rust={r:?}");
    assert_eq!(r.code, c.code, "exit-code mismatch: C={c:?} Rust={r:?}");
}

/// ERRORS rows 4-5: values outside `char` range pushed across the FFI boundary
/// as `int` (C truncates silently; there is no validation). Multiples of 256
/// are excluded because they truncate to '\0' (row 3 territory).
#[test]
fn c04_foo_needle_out_of_char_range() {
    let cf = c_foo_int();
    let rf = rust_foo_int();
    let strings: [&[u8]; 6] = [
        b"AAxxA",
        b"",
        b"\xff\xfe\x80\x7f",
        b"BBBB",
        b"AxAxAx\xff",
        b"\x01\x02\x03",
    ];
    let mut needles: Vec<c_int> = vec![
        -1, -2, -128, -129, -191, -255, -256 + 65, 255, 256 + 65, 256 + 120, 511, 512 + 66, 65601,
        i32::MAX, i32::MIN + 1, i32::MIN, 0x141, 0x1_0041,
    ];
    needles.retain(|n| (n & 0xff) != 0); // '\0' after truncation == ERRORS row 3
    for s in strings {
        let cs = CString::new(s.to_vec()).unwrap();
        for &n in &needles {
            let (a, b) = unsafe { (cf(cs.as_ptr(), n), rf(cs.as_ptr(), n)) };
            assert_eq!(a, b, "out-of-range needle {n} on {s:?}: C={a} Rust={b}");
            // and the value really is used truncated to 8 bits
            assert_eq!(a as usize, count(s, (n & 0xff) as u8), "C truncation model wrong for {n}");
        }
    }
}

/// ERRORS row 6: no occurrence at all — `strchr` returns NULL on the very
/// first iteration, which is the C function's only "rejection" path.
#[test]
fn c05_foo_no_occurrence_returns_zero() {
    let cf = c_foo();
    let rf = rust_foo();
    for s in [&b""[..], b"BBBB", b"\xff\xff", b"hello"] {
        let cs = CString::new(s.to_vec()).unwrap();
        for n in [b'A', b'x', 0x7fu8] {
            let (a, b) = unsafe { (cf(cs.as_ptr(), n as c_char), rf(cs.as_ptr(), n as c_char)) };
            assert_eq!(a, b);
            assert_eq!(a, 0, "expected zero hits for {s:?}/{n}");
        }
    }
}

/// ERRORS rows 7-8: `fread` failure / EOF is never checked. Empty stdin, and
/// an stdin that cannot be read at all (a directory fd -> EISDIR), must both
/// yield the all-zero buffer and exit status 0 on both sides.
#[test]
fn c06_main_unreadable_or_empty_stdin() {
    // empty file
    let empty = artifacts_dir().join("c06_empty.bin");
    fs::write(&empty, b"").unwrap();
    let c = run_child_with_stdin("main", c_so_path(), Stdio::from(fs::File::open(&empty).unwrap()));
    let r = run_child_with_stdin("main", rust_so_path(), Stdio::from(fs::File::open(&empty).unwrap()));
    assert_eq!(c, r, "empty-stdin mismatch");
    assert_eq!(c.stdout, expected(0, 0));
    assert_eq!(c.code, Some(0));

    // a directory: read(2) fails with EISDIR
    if let (Ok(d1), Ok(d2)) = (fs::File::open("/"), fs::File::open("/")) {
        let c = run_child_with_stdin("main", c_so_path(), Stdio::from(d1));
        let r = run_child_with_stdin("main", rust_so_path(), Stdio::from(d2));
        assert_eq!(c, r, "unreadable-stdin mismatch");
        assert_eq!(c.stdout, expected(0, 0));
        assert_eq!(c.code, Some(0));
    }
    let _ = fs::remove_file(&empty);
}

/// ERRORS row 9: more than 1000 bytes on stdin are silently dropped (`fread`'s
/// short/complete count is ignored).
#[test]
fn c07_main_input_longer_than_buffer() {
    // (a) well-defined variant: a NUL inside the first 1000 bytes keeps `in`
    //     terminated, so the dropped tail must be provably invisible.
    for len in [1001usize, 1002, 1500, 4096, 20000] {
        let mut input: Vec<u8> = vec![b'A'; len];
        input[900] = 0; // string ends here
        for b in input[901..].iter_mut() {
            *b = b'x'; // everything behind must be ignored
        }
        let c = run_child("main", c_so_path(), Some(&input));
        let r = run_child("main", rust_so_path(), Some(&input));
        assert_eq!(c, r, "truncation mismatch at len {len}");
        assert_eq!(c.stdout, expected(900, 0), "C must only see the first 900 bytes");

        let ce = run_exe(c_exe_path(), &input);
        let re = run_exe(rust_exe_path(), &input);
        assert_eq!(ce, re, "exe truncation mismatch at len {len}");
        assert_eq!(ce.stdout, expected(900, 0));
    }

    // (b) the genuinely truncating variant: 1000 'A' followed by 'x' — the
    //     tail past the buffer is dropped by `fread`, but `in` is unterminated,
    //     so the C keeps scanning past the array (ERRORS.md row 10).
    for len in [1001usize, 1002, 1500, 4096, 20000] {
        let input: Vec<u8> = (0..len).map(|i| if i < 1000 { b'A' } else { b'x' }).collect();
        let c = run_child("main", c_so_path(), Some(&input));
        let r = run_child("main", rust_so_path(), Some(&input));
        assert_unterminated_buffer_consistent(&format!("so truncation len {len}"), &c, &r, (1000, 0));

        let ce = run_exe(c_exe_path(), &input);
        let re = run_exe(rust_exe_path(), &input);
        assert_unterminated_buffer_consistent(&format!("exe truncation len {len}"), &ce, &re, (1000, 0));
    }
}

/// ERRORS row 10: exactly 1000 non-NUL bytes leave `in` without a terminator
/// (undefined behavior in C). Assert the Rust matches whatever the C actually
/// does, both through the .so and through the executables.
#[test]
fn c08_main_exactly_1000_non_nul_bytes() {
    for fill in [b'A', b'x', b'B', 0xffu8] {
        let input = vec![fill; 1000];
        let model = (count(&input, b'A'), count(&input, b'x'));
        let c = run_child("main", c_so_path(), Some(&input));
        let r = run_child("main", rust_so_path(), Some(&input));
        assert_unterminated_buffer_consistent(&format!(".so fill {fill:#x}"), &c, &r, model);

        let ce = run_exe(c_exe_path(), &input);
        let re = run_exe(rust_exe_path(), &input);
        assert_unterminated_buffer_consistent(&format!("exe fill {fill:#x}"), &ce, &re, model);
    }
}

/// ERRORS row 11: `printf`'s return value is ignored, so a failing write is
/// silent. /dev/full makes every write fail with ENOSPC; both must still exit 0.
#[test]
fn c09_output_write_error_is_ignored() {
    let full = match fs::OpenOptions::new().write(true).open("/dev/full") {
        Ok(f) => f,
        Err(_) => return, // /dev/full unavailable in this environment
    };
    drop(full);
    let mut outs = Vec::new();
    for exe in [c_exe_path(), rust_exe_path()] {
        let out = fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
        let st = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::null())
            .status()
            .unwrap();
        use std::os::unix::process::ExitStatusExt;
        outs.push((st.code(), st.signal()));
    }
    assert_eq!(outs[0], outs[1], "write-error handling mismatch: {outs:?}");
    assert_eq!(outs[0], (Some(0), None), "C should ignore the write error");
}

/// ERRORS row 12: a C program has SIGPIPE at SIG_DFL, so writing to a stdout
/// whose reader is gone kills it with signal 13. The Rust translation must not
/// silently succeed there.
#[test]
fn c10_sigpipe_on_closed_stdout() {
    use std::os::fd::FromRawFd;
    let mut results = Vec::new();
    for exe in [c_exe_path(), rust_exe_path()] {
        // Build the pipe ourselves and close the read end BEFORE the child is
        // started, so the very first write is guaranteed to hit EPIPE (no race
        // with process start-up).
        let mut fds = [0 as c_int; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe()");
        let write_end = unsafe { fs::File::from_raw_fd(fds[1]) };
        unsafe { libc::close(fds[0]) };
        let mut child = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(write_end))
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let st = wait_with_timeout(&mut child, Duration::from_secs(30));
        use std::os::unix::process::ExitStatusExt;
        results.push((st.code(), st.signal()));
    }
    assert_eq!(
        results[0],
        (None, Some(libc::SIGPIPE)),
        "C is expected to die from SIGPIPE: {results:?}"
    );
    assert_eq!(results[1], results[0], "SIGPIPE behavior mismatch: {results:?}");
}

/// ERRORS row 13: `main` always reports success, whatever happened.
#[test]
fn c11_main_always_returns_zero() {
    for input in [&b""[..], b"AAAA", &[0u8; 1][..]] {
        let c = run_child("main", c_so_path(), Some(input));
        let r = run_child("main", rust_so_path(), Some(input));
        assert_eq!(c.code, Some(0));
        assert_eq!(r.code, Some(0));
        assert_eq!(c, r);
    }
}

/// ERRORS row 14: interior NUL right at offset 0 (shortest possible string) and
/// a lone NUL as the whole input — the degenerate accepted input.
#[test]
fn c12_empty_string_inputs() {
    let cf = c_foo();
    let rf = rust_foo();
    let empty = CString::new("").unwrap();
    for n in [b'A', b'x', b'B', 0xffu8] {
        let (a, b) = unsafe { (cf(empty.as_ptr(), n as c_char), rf(empty.as_ptr(), n as c_char)) };
        assert_eq!(a, b);
        assert_eq!(a, 0);
    }
    let c = c_driver_output(b"");
    let r = rust_driver_output(b"");
    assert_eq!(c, r);
    assert_eq!(c, expected(0, 0));
}

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

#[test]
fn d01_symbol_parity() {
    fn defined_symbols(so: &Path) -> Vec<String> {
        let out = run(Command::new("nm").args(["-D", "--defined-only"]).arg(so));
        let mut v: Vec<String> = out
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }
    let c_syms = defined_symbols(c_so_path());
    let r_syms = defined_symbols(rust_so_path());
    assert!(
        c_syms.iter().any(|s| s == "foo") && c_syms.iter().any(|s| s == "driver") && c_syms.iter().any(|s| s == "main"),
        "unexpected C symbol set: {c_syms:?}"
    );
    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(missing.is_empty(), "symbols exported by C but missing from Rust: {missing:?}");
}

/// Every exported symbol must be reachable through `dlsym`, not merely present.
#[test]
fn d02_all_symbols_callable_via_dlsym() {
    for lib in [c_lib(), rust_lib()] {
        let _: FooFn = sym(lib, b"foo\0");
        let _: DriverFn = sym(lib, b"driver\0");
        let _: unsafe extern "C" fn() -> c_int = sym(lib, b"main\0");
    }
}
