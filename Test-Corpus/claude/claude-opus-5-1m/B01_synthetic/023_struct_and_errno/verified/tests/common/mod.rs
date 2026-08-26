//! Shared helpers for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects and called through the
//! FFI boundary (`libloading`), and both executables are spawned as child
//! processes.  Nothing from the Rust crate is linked directly.

#![allow(dead_code)]

use std::ffi::CString;
use std::io::Write;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

// ---------------------------------------------------------------------------
// Artifact locations
// ---------------------------------------------------------------------------

/// `target/<profile>` for the profile the tests were built with.
pub fn profile_dir() -> PathBuf {
    // current_exe() == target/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

pub fn rust_so() -> PathBuf {
    let p = profile_dir().join("libdriver.so");
    assert!(
        p.exists(),
        "missing {}: run `cargo build` (same profile) first",
        p.display()
    );
    p
}

pub fn c_so() -> PathBuf {
    let p = PathBuf::from(MANIFEST).join("c_build").join("libcdriver.so");
    assert!(
        p.exists(),
        "missing {}: run ./build_c.sh first",
        p.display()
    );
    p
}

pub fn c_exe() -> PathBuf {
    let p = PathBuf::from(MANIFEST)
        .join("c_src")
        .join("build")
        .join("driver");
    assert!(p.exists(), "missing {}: run ./build_c.sh first", p.display());
    p
}

pub fn rust_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// The C ABI surface
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct { int floors; int bedrooms; double bathrooms; } house_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct House {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: f64,
}

impl House {
    pub fn new(floors: i32, bedrooms: i32, bathrooms: f64) -> Self {
        House {
            floors,
            bedrooms,
            bathrooms,
        }
    }
    /// Bit-exact comparison (NaN payload included).
    pub fn bit_eq(&self, other: &House) -> bool {
        self.floors == other.floors
            && self.bedrooms == other.bedrooms
            && self.bathrooms.to_bits() == other.bathrooms.to_bits()
    }
    pub fn show(&self) -> String {
        format!(
            "House {{ floors: {}, bedrooms: {}, bathrooms: {:e} (bits {:#018x}) }}",
            self.floors,
            self.bedrooms,
            self.bathrooms,
            self.bathrooms.to_bits()
        )
    }
}

pub type RunFn = unsafe extern "C" fn(*mut House, c_int);
pub type MainFn = unsafe extern "C" fn() -> c_int;

struct Libs {
    c: libloading::Library,
    rust: libloading::Library,
    c_run: RunFn,
    rust_run: RunFn,
    c_main: MainFn,
    rust_main: MainFn,
}

// The libraries stay loaded for the lifetime of the test process.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = libloading::Library::new(c_so()).expect("dlopen C .so");
        let rust = libloading::Library::new(rust_so()).expect("dlopen Rust .so");
        let c_run = *c.get::<RunFn>(b"run\0").expect("C run");
        let rust_run = *rust.get::<RunFn>(b"run\0").expect("Rust run");
        let c_main = *c.get::<MainFn>(b"main\0").expect("C main");
        let rust_main = *rust.get::<MainFn>(b"main\0").expect("Rust main");
        Libs {
            c,
            rust,
            c_run,
            rust_run,
            c_main,
            rust_main,
        }
    })
}

pub fn c_run() -> RunFn {
    libs().c_run
}
pub fn rust_run() -> RunFn {
    libs().rust_run
}
pub fn c_main() -> MainFn {
    libs().c_main
}
pub fn rust_main() -> MainFn {
    libs().rust_main
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 0 / fd 1 are process-wide: serialise everything that touches them.
pub fn io_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

fn tmp_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let n = N.fetch_add(1, Ordering::SeqCst);
    PathBuf::from(dir).join(format!(
        "ffidiff-{}-{}-{}.bin",
        std::process::id(),
        tag,
        n
    ))
}

unsafe fn open_rw(path: &Path) -> c_int {
    let c = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let fd = libc::open(
        c.as_ptr(),
        libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
        0o600 as libc::c_uint,
    );
    assert!(fd >= 0, "open {} failed", path.display());
    fd
}

unsafe fn open_ro(path: &Path) -> c_int {
    let c = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let fd = libc::open(c.as_ptr(), libc::O_RDONLY);
    assert!(fd >= 0, "open {} failed", path.display());
    fd
}

/// Runs `f` with fd 1 redirected to a temporary file and returns everything
/// that was written (C `printf` buffers are flushed afterwards).
pub unsafe fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = io_lock().lock().unwrap();
    capture_stdout_locked(f)
}

pub unsafe fn capture_stdout_locked<F: FnOnce()>(f: F) -> Vec<u8> {
    libc::fflush(std::ptr::null_mut());
    let _ = std::io::stdout().flush();

    let path = tmp_path("out");
    let fd = open_rw(&path);
    let saved = libc::dup(1);
    assert!(saved >= 0);
    assert!(libc::dup2(fd, 1) >= 0);

    f();

    libc::fflush(std::ptr::null_mut());
    let _ = std::io::stdout().flush();
    assert!(libc::dup2(saved, 1) >= 0);
    libc::close(saved);
    libc::close(fd);

    let data = std::fs::read(&path).expect("read capture");
    let _ = std::fs::remove_file(&path);
    data
}

/// Result of a forked call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkOutcome {
    pub stdout: Vec<u8>,
    /// exit status, or None if killed by a signal
    pub exit: Option<i32>,
    /// terminating signal, if any
    pub signal: Option<i32>,
}

impl ForkOutcome {
    pub fn show(&self) -> String {
        format!(
            "exit={:?} signal={:?} stdout={:?}",
            self.exit,
            self.signal,
            String::from_utf8_lossy(&self.stdout)
        )
    }
}

/// Calls `f()` in a forked child with fd 0 fed from `stdin_data` (when given)
/// and fd 1 captured.  Forking gives each call pristine stdio buffer state, so
/// repeated calls to `main()` cannot influence one another, and lets us observe
/// fatal signals.
pub unsafe fn call_forked<F: FnOnce() -> c_int>(stdin_data: Option<&[u8]>, f: F) -> ForkOutcome {
    let _guard = io_lock().lock().unwrap();

    libc::fflush(std::ptr::null_mut());
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let out_path = tmp_path("fout");
    let outfd = open_rw(&out_path);

    let in_path = tmp_path("fin");
    let infd = if let Some(data) = stdin_data {
        std::fs::write(&in_path, data).expect("write stdin file");
        open_ro(&in_path)
    } else {
        -1
    };

    let pid = libc::fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // child
        if infd >= 0 {
            libc::dup2(infd, 0);
        }
        libc::dup2(outfd, 1);
        let r = f();
        libc::fflush(std::ptr::null_mut());
        libc::_exit(r);
    }

    let mut status: c_int = 0;
    let w = libc::waitpid(pid, &mut status, 0);
    assert_eq!(w, pid, "waitpid");
    libc::close(outfd);
    if infd >= 0 {
        libc::close(infd);
    }

    let stdout = std::fs::read(&out_path).expect("read capture");
    let _ = std::fs::remove_file(&out_path);
    if infd >= 0 {
        let _ = std::fs::remove_file(&in_path);
    }

    let exited = libc::WIFEXITED(status);
    ForkOutcome {
        stdout,
        exit: if exited {
            Some(libc::WEXITSTATUS(status))
        } else {
            None
        },
        signal: if libc::WIFSIGNALED(status) {
            Some(libc::WTERMSIG(status))
        } else {
            None
        },
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Calls `run()` in both shared objects with the same starting state and
/// returns (stdout, resulting struct) for each.
pub fn diff_run(house: House, extra: i32) -> ((Vec<u8>, House), (Vec<u8>, House)) {
    unsafe {
        let mut hc = house;
        let cf = c_run();
        let c_out = capture_stdout(|| cf(&mut hc as *mut House, extra));

        let mut hr = house;
        let rf = rust_run();
        let r_out = capture_stdout(|| rf(&mut hr as *mut House, extra));

        ((c_out, hc), (r_out, hr))
    }
}

/// Asserts that both `run()` implementations agree, byte for byte.
pub fn assert_run_matches(house: House, extra: i32, ctx: &str) {
    let ((c_out, hc), (r_out, hr)) = diff_run(house, extra);
    if c_out != r_out {
        panic!(
            "run() stdout mismatch [{}]\n  start: {}\n  extra: {}\n  C   : {:?}\n  Rust: {:?}",
            ctx,
            house.show(),
            extra,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
    if !hc.bit_eq(&hr) {
        panic!(
            "run() struct mismatch [{}]\n  start: {}\n  extra: {}\n  C   : {}\n  Rust: {}",
            ctx,
            house.show(),
            extra,
            hc.show(),
            hr.show()
        );
    }
}

/// Batched version of [`assert_run_matches`]: every case is executed through
/// the C `.so` under one stdout capture and through the Rust `.so` under
/// another, then the concatenated output (4 lines per case) and the resulting
/// structs are compared. Mismatches are localised back to the offending case.
pub fn assert_run_batch(cases: &[(House, i32)], ctx: &str) {
    if cases.is_empty() {
        return;
    }
    let (c_out, c_states) = unsafe {
        let f = c_run();
        let mut states: Vec<House> = Vec::with_capacity(cases.len());
        let out = capture_stdout(|| {
            for (h, e) in cases {
                let mut hh = *h;
                f(&mut hh as *mut House, *e);
                states.push(hh);
            }
        });
        (out, states)
    };
    let (r_out, r_states) = unsafe {
        let f = rust_run();
        let mut states: Vec<House> = Vec::with_capacity(cases.len());
        let out = capture_stdout(|| {
            for (h, e) in cases {
                let mut hh = *h;
                f(&mut hh as *mut House, *e);
                states.push(hh);
            }
        });
        (out, states)
    };

    // Sanity: the capture must actually have seen the 4 lines per call that
    // `print_house` emits, otherwise the comparison would be vacuous.
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        4 * cases.len(),
        "[{}] captured {} bytes from the C .so, expected 4 lines per case",
        ctx,
        c_out.len()
    );

    if c_out != r_out {
        // Locate the first differing line; `run` prints exactly 4 lines.
        let cl: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();
        let mut i = 0;
        while i < cl.len() && i < rl.len() && cl[i] == rl[i] {
            i += 1;
        }
        let case = (i / 4).min(cases.len() - 1);
        panic!(
            "run() stdout mismatch [{}] at output line {} (case #{})\n  start: {}\n  extra: {}\n  C   : {:?}\n  Rust: {:?}",
            ctx,
            i,
            case,
            cases[case].0.show(),
            cases[case].1,
            String::from_utf8_lossy(cl.get(i).copied().unwrap_or(b"<eof>")),
            String::from_utf8_lossy(rl.get(i).copied().unwrap_or(b"<eof>")),
        );
    }
    for (i, (c, r)) in c_states.iter().zip(r_states.iter()).enumerate() {
        if !c.bit_eq(r) {
            panic!(
                "run() struct mismatch [{}] case #{}\n  start: {}\n  extra: {}\n  C   : {}\n  Rust: {}",
                ctx,
                i,
                cases[i].0.show(),
                cases[i].1,
                c.show(),
                r.show()
            );
        }
    }
}

/// Like [`assert_run_batch`] but calls `run` twice per case on the carried-over
/// struct — exactly what the C `main` does.
pub fn assert_run_twice_batch(cases: &[(House, i32)], ctx: &str) {
    let call_all = |f: RunFn| -> (Vec<u8>, Vec<House>) {
        unsafe {
            let mut states: Vec<House> = Vec::with_capacity(cases.len());
            let out = capture_stdout(|| {
                for (h, e) in cases {
                    let mut hh = *h;
                    f(&mut hh as *mut House, *e);
                    f(&mut hh as *mut House, *e);
                    states.push(hh);
                }
            });
            (out, states)
        }
    };
    let (c_out, c_states) = call_all(c_run());
    let (r_out, r_states) = call_all(rust_run());
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        8 * cases.len(),
        "[{}] captured {} bytes from the C .so, expected 8 lines per case",
        ctx,
        c_out.len()
    );
    if c_out != r_out {
        let cl: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();
        let mut i = 0;
        while i < cl.len() && i < rl.len() && cl[i] == rl[i] {
            i += 1;
        }
        let case = (i / 8).min(cases.len().saturating_sub(1));
        panic!(
            "run()x2 stdout mismatch [{}] at line {} (case #{})\n  start: {}\n  extra: {}\n  C   : {:?}\n  Rust: {:?}",
            ctx,
            i,
            case,
            cases[case].0.show(),
            cases[case].1,
            String::from_utf8_lossy(cl.get(i).copied().unwrap_or(b"<eof>")),
            String::from_utf8_lossy(rl.get(i).copied().unwrap_or(b"<eof>")),
        );
    }
    for (i, (c, r)) in c_states.iter().zip(r_states.iter()).enumerate() {
        assert!(
            c.bit_eq(r),
            "run()x2 struct mismatch [{}] case #{}: C {} vs Rust {}",
            ctx,
            i,
            c.show(),
            r.show()
        );
    }
}

/// Calls the exported `main()` of both shared objects with `input` on stdin.
pub fn diff_ffi_main(input: &[u8]) -> (ForkOutcome, ForkOutcome) {
    unsafe {
        let cf = c_main();
        let c = call_forked(Some(input), || cf());
        let rf = rust_main();
        let r = call_forked(Some(input), || rf());
        (c, r)
    }
}

pub fn assert_ffi_main_matches(input: &[u8], ctx: &str) {
    let (c, r) = diff_ffi_main(input);
    // Sanity: the C program always prints either 8 house lines or the error
    // line, so an empty capture would mean the comparison was vacuous.
    assert!(
        c.stdout == b"An error occurred\n"
            || c.stdout.iter().filter(|&&b| b == b'\n').count() == 8,
        "[{}] unexpected C main() output {:?}",
        ctx,
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(
        c, r,
        "exported main() mismatch [{}] for input {:?}\n  C   : {}\n  Rust: {}",
        ctx,
        String::from_utf8_lossy(input),
        c.show(),
        r.show()
    );
}

/// Spawns an executable, feeds `input` on stdin, and collects everything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit: Option<i32>,
    pub signal: Option<i32>,
}

impl ProcOutcome {
    pub fn show(&self) -> String {
        format!(
            "exit={:?} signal={:?}\n   stdout={:?}\n   stderr={:?}",
            self.exit,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

pub fn run_exe(path: &Path, input: &[u8]) -> ProcOutcome {
    use std::os::unix::process::ExitStatusExt;
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {}", path.display(), e));
    {
        let mut si = child.stdin.take().unwrap();
        // The child may exit without reading everything; a broken pipe is fine.
        let _ = si.write_all(input);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait_with_output");
    ProcOutcome {
        stdout: out.stdout,
        stderr: out.stderr,
        exit: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Spawns an executable with stdout connected to a pipe that has **no reader**
/// (the read end is closed before the child starts), so the first write fails.
/// `ignore_sigpipe` selects the disposition the child inherits.
/// Only the termination status is observable.
pub fn run_exe_without_stdout_reader(
    path: &Path,
    input: &[u8],
    ignore_sigpipe: bool,
) -> (Option<i32>, Option<i32>) {
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::{CommandExt, ExitStatusExt};

    let in_path = {
        let p = tmp_path("stdin");
        std::fs::write(&p, input).expect("write stdin file");
        p
    };
    let stdin_file = std::fs::File::open(&in_path).expect("open stdin file");

    let status = unsafe {
        let mut fds = [0 as c_int; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe");
        libc::close(fds[0]); // reader gone before the child even starts
        let child_stdout = Stdio::from_raw_fd(fds[1]);

        let mut cmd = Command::new(path);
        cmd.stdin(Stdio::from(stdin_file))
            .stdout(child_stdout)
            .stderr(Stdio::null());
        cmd.pre_exec(move || {
            let h = if ignore_sigpipe {
                libc::SIG_IGN
            } else {
                libc::SIG_DFL
            };
            libc::signal(libc::SIGPIPE, h);
            Ok(())
        });
        let mut child = cmd.spawn().expect("spawn");
        child.wait().expect("wait")
    };
    let _ = std::fs::remove_file(&in_path);
    (status.code(), status.signal())
}

/// Spawns an executable with file descriptor 0 **closed**, so `fgets` fails
/// with EBADF.
pub fn run_exe_with_closed_stdin(path: &Path) -> ProcOutcome {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let mut cmd = Command::new(path);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(|| {
            libc::close(0);
            Ok(())
        });
    }
    let out = cmd.spawn().expect("spawn").wait_with_output().expect("wait");
    ProcOutcome {
        stdout: out.stdout,
        stderr: out.stderr,
        exit: out.status.code(),
        signal: out.status.signal(),
    }
}

pub fn assert_exe_matches(input: &[u8], ctx: &str) {
    let c = run_exe(&c_exe(), input);
    let r = run_exe(&rust_exe(), input);
    assert!(
        c.stdout == b"An error occurred\n"
            || c.stdout.iter().filter(|&&b| b == b'\n').count() == 8,
        "[{}] unexpected C driver output {:?}",
        ctx,
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(
        c,
        r,
        "executable mismatch [{}] for input {:?}\n  C   : {}\n  Rust: {}",
        ctx,
        String::from_utf8_lossy(input),
        c.show(),
        r.show()
    );
}

/// Full stdin-driven comparison: the exported `main` of both `.so`s **and**
/// both executables.
pub fn assert_input_matches(input: &[u8], ctx: &str) {
    assert_ffi_main_matches(input, ctx);
    assert_exe_matches(input, ctx);
}

pub fn assert_inputs_match<'a, I: IntoIterator<Item = &'a [u8]>>(inputs: I, ctx: &str) {
    for (i, inp) in inputs.into_iter().enumerate() {
        assert_input_matches(inp, &format!("{}#{}", ctx, i));
    }
}

/// Calls `run` with a NULL `house_t*` in a forked child in both `.so`s and
/// compares how the process died.
pub fn assert_run_null_matches() {
    let c = unsafe {
        let f = c_run();
        call_forked(None, || {
            f(std::ptr::null_mut(), 1);
            0
        })
    };
    let r = unsafe {
        let f = rust_run();
        call_forked(None, || {
            f(std::ptr::null_mut(), 1);
            0
        })
    };
    assert_eq!(
        c.signal, r.signal,
        "run(NULL) terminating signal differs: C {} vs Rust {}",
        c.show(),
        r.show()
    );
    assert_eq!(
        c.exit, r.exit,
        "run(NULL) exit status differs: C {} vs Rust {}",
        c.show(),
        r.show()
    );
    assert_eq!(
        c.stdout, r.stdout,
        "run(NULL) stdout differs: C {:?} vs Rust {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — reproducible property-style testing
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
    /// Uniform in `[0, n)`
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Any bit pattern, i.e. finite values, subnormals, infinities and NaNs.
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A decimal-looking double: `m / 10^d` for random `m`, `d`. This is the
    /// generator that produces the most `x.x5`-style rounding decisions, i.e.
    /// the values where `%.1f` is hardest to reproduce.
    pub fn next_f64_decimalish(&mut self) -> f64 {
        let digits = 1 + self.below(18) as u32;
        let m = (self.next_u64() % 10u64.pow(digits)) as f64;
        let d = self.below(19) as i32;
        let v = m / 10f64.powi(d);
        if self.next_u64() & 1 == 0 {
            v
        } else {
            -v
        }
    }

    /// A "reasonable" double: mixes small magnitudes, tie-prone values and
    /// large magnitudes.
    pub fn next_f64_mixed(&mut self) -> f64 {
        match self.below(6) {
            0 => self.next_f64_bits(),
            1 => (self.next_i32() as f64) / 4.0,
            2 => (self.next_i32() as f64) / 10.0,
            3 => (self.next_i32() as f64) * 1e-3,
            4 => (self.next_u64() as f64) / 8.0,
            _ => {
                let e = self.below(40) as i32 - 20;
                (self.next_i32() as f64) * 10f64.powi(e)
            }
        }
    }
}
