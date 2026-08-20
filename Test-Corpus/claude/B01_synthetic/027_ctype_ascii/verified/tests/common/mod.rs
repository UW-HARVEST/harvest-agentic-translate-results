//! Shared differential-testing harness.
//!
//! Both implementations are exercised *only* through their shared objects,
//! loaded with `libloading` and called through their exported C symbols
//! (`driver`, `main`) -- never by calling Rust functions directly.
//!
//! Each measurement runs in a freshly `fork()`ed child so that the stdio state
//! (buffered stdin, buffered stdout, locale) of one call cannot leak into the
//! next, exactly as it cannot leak between two runs of the real program.

#![allow(dead_code)]

use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), so every "randomised" run is reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform-ish value in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 40) as u8
    }

    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte()).collect()
    }

    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

// ---------------------------------------------------------------------------
// Paths / build-on-demand
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<target>/<profile>` (the directory that holds the built `.so` and binary).
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // <target>/<profile>/deps/<test-binary>
    exe.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("profile dir")
}

pub fn target_dir() -> PathBuf {
    profile_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir().join("target"))
}

fn tmp_dir() -> PathBuf {
    let d = target_dir().join("ittmp");
    fs::create_dir_all(&d).expect("create tmp dir");
    d
}

fn newer(src: &Path, dst: &Path) -> bool {
    match (fs::metadata(src), fs::metadata(dst)) {
        (Ok(s), Ok(d)) => match (s.modified(), d.modified()) {
            (Ok(sm), Ok(dm)) => sm > dm,
            _ => true,
        },
        (Ok(_), Err(_)) => true,
        _ => false,
    }
}

/// The C shared object. `c_src/CMakeLists.txt` only declares an executable, so
/// the very same translation unit is additionally compiled as a shared object
/// here (nothing inside `c_src/` is modified or written to).
pub fn c_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let src = manifest_dir().join("c_src/src/main.c");
        let dir = target_dir().join("c");
        fs::create_dir_all(&dir).expect("create target/c");
        let out = dir.join("libdriver_c.so");
        if newer(&src, &out) {
            let tmp = dir.join(format!("libdriver_c.{}.so", std::process::id()));
            let st = Command::new("gcc")
                .args(["-shared", "-fPIC", "-O2", "-o"])
                .arg(&tmp)
                .arg(&src)
                .status()
                .expect("run gcc");
            assert!(st.success(), "compiling the C shared object failed");
            fs::rename(&tmp, &out).expect("install C .so");
        }
        out
    })
    .clone()
}

/// The C executable, built exactly as `c_src/CMakeLists.txt` declares it.
pub fn c_exe_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = manifest_dir().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if newer(&c_src.join("src/main.c"), &exe) {
            fs::create_dir_all(&build).expect("create c_src/build");
            let st = Command::new("cmake")
                .current_dir(&build)
                .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                .status()
                .expect("run cmake");
            assert!(st.success(), "cmake configure failed");
            let st = Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .status()
                .expect("run cmake --build");
            assert!(st.success(), "cmake build failed");
        }
        assert!(exe.is_file(), "missing C executable {}", exe.display());
        exe
    })
    .clone()
}

/// `cargo test` builds the integration tests and the `rlib`, but not the
/// `cdylib`/binary the differential tests need, so build them on demand (a
/// nested `cargo build` is safe: cargo releases the build lock before running
/// test binaries, and `cargo build` does not build test targets).
fn ensure_rust_artifacts() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let so = profile_dir().join("libctype_driver.so");
        let exe = profile_dir().join("driver");
        if so.is_file() && exe.is_file() {
            return;
        }
        let release = profile_dir()
            .file_name()
            .map(|n| n == "release")
            .unwrap_or(false);
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        for extra in [vec!["--offline"], vec![]] {
            let mut cmd = Command::new(&cargo);
            cmd.arg("build")
                .current_dir(manifest_dir())
                .env_remove("RUSTFLAGS");
            if release {
                cmd.arg("--release");
            }
            cmd.args(&extra);
            if let Ok(st) = cmd.status() {
                if st.success() {
                    break;
                }
            }
        }
        assert!(
            so.is_file() && exe.is_file(),
            "could not build {} / {} -- run `cargo build` first",
            so.display(),
            exe.display()
        );
    });
}

pub fn rust_so_path() -> PathBuf {
    ensure_rust_artifacts();
    let p = profile_dir().join("libctype_driver.so");
    assert!(
        p.is_file(),
        "missing Rust shared object {} -- run `cargo build` first",
        p.display()
    );
    p
}

pub fn rust_exe_path() -> PathBuf {
    ensure_rust_artifacts();
    let p = profile_dir().join("driver");
    assert!(
        p.is_file(),
        "missing Rust executable {} -- run `cargo build` first",
        p.display()
    );
    p
}

/// Locales installed on this machine (`locale -a`), used to skip rows whose
/// locale is unavailable.
pub fn locale_available(name: &str) -> bool {
    static ALL: OnceLock<Vec<String>> = OnceLock::new();
    let all = ALL.get_or_init(|| match Command::new("locale").arg("-a").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .collect(),
        Err(_) => Vec::new(),
    });
    all.iter().any(|l| l == name)
}

// ---------------------------------------------------------------------------
// The two shared objects and their exported symbols
// ---------------------------------------------------------------------------

pub type DriverFn = unsafe extern "C" fn(c_char);
pub type DriverIntFn = unsafe extern "C" fn(c_int);
/// `driver` called through a prototype with extra (ignored) arguments.
pub type DriverExtraFn = unsafe extern "C" fn(c_char, c_int, usize) -> c_int;
pub type MainFn = unsafe extern "C" fn() -> c_int;
/// `int main()` in C is unprototyped, so a caller may pass `argc`/`argv`.
pub type MainArgsFn = unsafe extern "C" fn(c_int, usize, usize) -> c_int;

pub struct Impl {
    pub name: &'static str,
    pub driver: DriverFn,
    pub driver_int: DriverIntFn,
    pub driver_extra: DriverExtraFn,
    pub main: MainFn,
    pub main_args: MainArgsFn,
}

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c: Impl,
    pub rust: Impl,
}

unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn load(name: &'static str, path: &Path) -> (Library, Impl) {
    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let driver: Symbol<DriverFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display()));
        let driver_int: Symbol<DriverIntFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display()));
        let driver_extra: Symbol<DriverExtraFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display()));
        let main: Symbol<MainFn> = lib
            .get(b"main\0")
            .unwrap_or_else(|e| panic!("dlsym main in {}: {e}", path.display()));
        let main_args: Symbol<MainArgsFn> = lib
            .get(b"main\0")
            .unwrap_or_else(|e| panic!("dlsym main in {}: {e}", path.display()));
        let imp = Impl {
            name,
            driver: *driver,
            driver_int: *driver_int,
            driver_extra: *driver_extra,
            main: *main,
            main_args: *main_args,
        };
        (lib, imp)
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let (lc, c) = load("C", &c_so_path());
        let (lr, rust) = load("Rust", &rust_so_path());
        Libs {
            _c: lc,
            _rust: lr,
            c,
            rust,
        }
    })
}

// ---------------------------------------------------------------------------
// Child-process configuration
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum StdinSpec {
    /// Seekable regular file with the given bytes (empty vec == immediate EOF).
    File(Vec<u8>),
    /// Non-seekable pipe carrying the given bytes (must stay < 64 KiB).
    Pipe(Vec<u8>),
    /// `/dev/null`
    DevNull,
    /// File descriptor 0 closed.
    Closed,
    /// A write-only file as fd 0 (`read()` fails with `EBADF`).
    WriteOnly,
    /// A directory as fd 0 (`read()` fails with `EISDIR`).
    Directory,
}

#[derive(Clone, Copy, Debug)]
pub enum StdoutSpec {
    /// Seekable regular file (fully buffered stdio).
    File,
    /// Pipe, drained by the parent.
    Pipe,
    /// File descriptor 1 closed (`write()` fails with `EBADF`).
    Closed,
    /// Pipe whose read end is already closed (`write()` fails with `EPIPE`).
    BrokenPipe,
}

#[derive(Clone, Debug)]
pub struct Cfg {
    pub stdin: StdinSpec,
    pub stdout: StdoutSpec,
    /// `setlocale(LC_ALL, ...)` performed by the *caller* before the call.
    pub locale: Option<String>,
    /// Install `SIG_IGN` for `SIGPIPE` in the child before the call.
    pub ignore_sigpipe: bool,
    /// Restore `SIG_DFL` for `SIGPIPE` in the child (the disposition a C
    /// program starts with; the Rust test runtime sets `SIG_IGN`).
    pub default_sigpipe: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            stdin: StdinSpec::File(Vec::new()),
            stdout: StdoutSpec::File,
            locale: None,
            ignore_sigpipe: false,
            default_sigpipe: false,
        }
    }
}

impl Cfg {
    pub fn stdin_file(bytes: &[u8]) -> Self {
        Cfg {
            stdin: StdinSpec::File(bytes.to_vec()),
            ..Cfg::default()
        }
    }

    pub fn stdin_pipe(bytes: &[u8]) -> Self {
        Cfg {
            stdin: StdinSpec::Pipe(bytes.to_vec()),
            ..Cfg::default()
        }
    }

    pub fn with_stdout(mut self, s: StdoutSpec) -> Self {
        self.stdout = s;
        self
    }

    pub fn with_locale(mut self, l: &str) -> Self {
        self.locale = Some(l.to_string());
        self
    }

    pub fn ignoring_sigpipe(mut self) -> Self {
        self.ignore_sigpipe = true;
        self
    }

    pub fn with_default_sigpipe(mut self) -> Self {
        self.default_sigpipe = true;
        self
    }
}

/// One call to be performed inside the child, in order.
#[derive(Clone, Copy, Debug)]
pub enum Op {
    /// `driver(c)` through a `void driver(char)` prototype.
    Driver(i8),
    /// `driver(v)` through a `void driver(int)` prototype (FFI garbage test).
    DriverInt(i32),
    /// `driver(c, junk, junk)` through a prototype with extra arguments.
    DriverExtra(i8, i32, usize),
    /// `main()`
    Main,
    /// `main(argc, argv, envp)` -- `int main()` is unprototyped in C.
    MainArgs(i32, usize, usize),
}

/// What the child produced.
#[derive(Clone, PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    /// Return value of every `Op` (`driver` is `void`, recorded as 0).
    pub rets: Vec<i32>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Run {{ exit_code: {:?}, signal: {:?}, rets: {:?}, stdout ({} bytes): {} }}",
            self.exit_code,
            self.signal,
            self.rets,
            self.stdout.len(),
            escape(&self.stdout)
        )
    }
}

pub fn escape(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        match c {
            b'\n' => s.push_str("\\n"),
            0x20..=0x7E => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s
}

// ---------------------------------------------------------------------------
// The forking runner
// ---------------------------------------------------------------------------

fn unique(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}.{}.{n}", std::process::id()))
}

fn temp_rw(tag: &str) -> File {
    let p = unique(tag);
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&p)
        .expect("open temp file");
    let _ = fs::remove_file(&p); // unlinked but kept open
    f
}

fn temp_with(tag: &str, bytes: &[u8]) -> File {
    let mut f = temp_rw(tag);
    f.write_all(bytes).expect("write temp file");
    f.flush().expect("flush temp file");
    f.seek(SeekFrom::Start(0)).expect("rewind temp file");
    f
}

fn read_all(f: &mut File) -> Vec<u8> {
    let mut v = Vec::new();
    f.seek(SeekFrom::Start(0)).expect("rewind");
    f.read_to_end(&mut v).expect("read temp file");
    v
}

const SIGPIPE: c_int = 13;
const SIG_IGN: usize = 1;
const SIG_DFL: usize = 0;

/// Runs `ops` against `imp` in a fresh child process configured by `cfg`.
pub fn run_ops(imp: &Impl, ops: &[Op], cfg: &Cfg) -> Run {
    // stdin
    let mut in_pipe: Option<(c_int, c_int)> = None;
    let mut in_file: Option<File> = None;
    let mut in_dir_fd: Option<c_int> = None;
    let in_fd: Option<c_int> = match &cfg.stdin {
        StdinSpec::File(bytes) => {
            let f = temp_with("stdin", bytes);
            let fd = f.as_raw_fd();
            in_file = Some(f);
            Some(fd)
        }
        StdinSpec::Pipe(bytes) => {
            assert!(bytes.len() < 60_000, "pipe payload must fit the pipe buffer");
            let mut fds = [0 as c_int; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe()");
            if !bytes.is_empty() {
                let n = unsafe {
                    libc::write(fds[1], bytes.as_ptr() as *const libc::c_void, bytes.len())
                };
                assert_eq!(n, bytes.len() as isize, "priming stdin pipe");
            }
            in_pipe = Some((fds[0], fds[1]));
            Some(fds[0])
        }
        StdinSpec::DevNull => {
            let f = File::open("/dev/null").expect("open /dev/null");
            let fd = f.as_raw_fd();
            in_file = Some(f);
            Some(fd)
        }
        StdinSpec::Closed => None,
        StdinSpec::WriteOnly => {
            let p = unique("wronly");
            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&p)
                .expect("open write-only");
            let _ = fs::remove_file(&p);
            let fd = f.as_raw_fd();
            in_file = Some(f);
            Some(fd)
        }
        StdinSpec::Directory => {
            let c = CString::new(manifest_dir().to_str().unwrap()).unwrap();
            let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY) };
            assert!(fd >= 0, "open directory");
            in_dir_fd = Some(fd);
            Some(fd)
        }
    };

    // stdout
    let mut out_file: Option<File> = None;
    let mut out_pipe: Option<(c_int, c_int)> = None;
    let out_fd: Option<c_int> = match cfg.stdout {
        StdoutSpec::File => {
            let f = temp_rw("stdout");
            let fd = f.as_raw_fd();
            out_file = Some(f);
            Some(fd)
        }
        StdoutSpec::Pipe | StdoutSpec::BrokenPipe => {
            let mut fds = [0 as c_int; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe()");
            if matches!(cfg.stdout, StdoutSpec::BrokenPipe) {
                // Close the read end now: every write must fail with EPIPE.
                unsafe { libc::close(fds[0]) };
                out_pipe = Some((-1, fds[1]));
            } else {
                out_pipe = Some((fds[0], fds[1]));
            }
            Some(fds[1])
        }
        StdoutSpec::Closed => None,
    };

    let mut ret_file = temp_rw("rets");
    let ret_fd = ret_file.as_raw_fd();

    let locale = cfg
        .locale
        .as_ref()
        .map(|l| CString::new(l.as_str()).unwrap());

    // Nothing of ours may sit in a buffer that the child would duplicate.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork()");

    if pid == 0 {
        // ---- child ----
        unsafe {
            match out_fd {
                Some(fd) => {
                    libc::dup2(fd, 1);
                }
                None => {
                    libc::close(1);
                }
            }
            match in_fd {
                Some(fd) => {
                    libc::dup2(fd, 0);
                }
                None => {
                    libc::close(0);
                }
            }
            if let Some((rd, _)) = out_pipe {
                if rd >= 0 {
                    libc::close(rd);
                }
            }
            if let Some((_, wr)) = in_pipe {
                libc::close(wr);
            }
            if cfg.ignore_sigpipe {
                libc::signal(SIGPIPE as c_int, SIG_IGN);
            }
            if cfg.default_sigpipe {
                libc::signal(SIGPIPE as c_int, SIG_DFL);
            }
            if let Some(l) = &locale {
                libc::setlocale(libc::LC_ALL, l.as_ptr());
            }

            let mut rets: Vec<i32> = Vec::with_capacity(ops.len());
            for op in ops {
                match *op {
                    Op::Driver(c) => {
                        (imp.driver)(c as c_char);
                        rets.push(0);
                    }
                    Op::DriverInt(v) => {
                        (imp.driver_int)(v as c_int);
                        rets.push(0);
                    }
                    Op::DriverExtra(c, junk1, junk2) => {
                        // `driver` returns void: the value left in the return
                        // register is meaningless, so it is not compared.
                        let _ = (imp.driver_extra)(c as c_char, junk1 as c_int, junk2);
                        rets.push(0);
                    }
                    Op::Main => {
                        rets.push((imp.main)() as i32);
                    }
                    Op::MainArgs(argc, argv, envp) => {
                        rets.push((imp.main_args)(argc as c_int, argv, envp) as i32);
                    }
                }
            }

            // Flush the C implementation's stdio buffers (the Rust `.so`
            // flushes its own stdout inside the exported functions).
            libc::fflush(std::ptr::null_mut());

            let bytes: Vec<u8> = rets.iter().flat_map(|r| r.to_le_bytes()).collect();
            if !bytes.is_empty() {
                libc::write(ret_fd, bytes.as_ptr() as *const libc::c_void, bytes.len());
            }
            libc::_exit(0);
        }
    }

    // ---- parent ----
    if let Some((_, wr)) = in_pipe {
        unsafe { libc::close(wr) };
    }
    let mut stdout = Vec::new();
    if let Some((rd, wr)) = out_pipe {
        unsafe { libc::close(wr) };
        if rd >= 0 {
            let mut buf = [0u8; 4096];
            loop {
                let n = unsafe { libc::read(rd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
                if n <= 0 {
                    break;
                }
                stdout.extend_from_slice(&buf[..n as usize]);
            }
            unsafe { libc::close(rd) };
        }
    }

    let mut status: c_int = 0;
    let w = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid()");

    if let Some(mut f) = out_file {
        stdout = read_all(&mut f);
    }
    let ret_bytes = read_all(&mut ret_file);
    let rets: Vec<i32> = ret_bytes
        .chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    drop(in_file);
    if let Some(fd) = in_dir_fd {
        unsafe { libc::close(fd) };
    }
    if let Some((rd, _)) = in_pipe {
        unsafe { libc::close(rd) };
    }

    let exit_code = if libc::WIFEXITED(status) {
        Some(libc::WEXITSTATUS(status))
    } else {
        None
    };
    let signal = if libc::WIFSIGNALED(status) {
        Some(libc::WTERMSIG(status))
    } else {
        None
    };

    Run {
        stdout,
        rets,
        exit_code,
        signal,
    }
}

/// Runs `ops` in the same configuration against both shared objects and asserts
/// that everything an external caller can observe is identical.
pub fn assert_same(label: &str, ops: &[Op], cfg: &Cfg) -> Run {
    let l = libs();
    let c = run_ops(&l.c, ops, cfg);
    let r = run_ops(&l.rust, ops, cfg);
    if c != r {
        panic!(
            "divergence in {label}\n  ops:  {ops:?}\n  cfg:  {cfg:?}\n  C:    {c:?}\n  Rust: {r:?}"
        );
    }
    // Sanity: the child itself must not have failed for harness reasons.
    assert_eq!(c.exit_code, Some(0), "child exited abnormally in {label}");
    assert_eq!(c.signal, None, "child was signalled in {label}");
    c
}

/// Like `assert_same`, but for configurations in which the child is expected to
/// die (e.g. `SIGPIPE`); only the observable results are compared.
pub fn assert_same_allow_abnormal(label: &str, ops: &[Op], cfg: &Cfg) -> (Run, Run) {
    let l = libs();
    let c = run_ops(&l.c, ops, cfg);
    let r = run_ops(&l.rust, ops, cfg);
    if c != r {
        panic!(
            "divergence in {label}\n  ops:  {ops:?}\n  cfg:  {cfg:?}\n  C:    {c:?}\n  Rust: {r:?}"
        );
    }
    (c, r)
}

/// Convenience: `driver(c)` with the default configuration.
pub fn assert_driver_same(label: &str, c: i8) -> Run {
    assert_same(label, &[Op::Driver(c)], &Cfg::default())
}
