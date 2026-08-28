//! Differential-test harness.
//!
//! Both implementations — the C `libhello.so` built by `c_src/CMakeLists.txt`
//! and the Rust `libhello.so` built from this crate — are loaded with
//! `libloading` and driven **only** through their exported `helloworld` symbol.
//! No Rust function is ever called directly, so the `#[unsafe(no_mangle)]`
//! export wrapper is under test too.
//!
//! `helloworld`'s entire observable behaviour is (a) its `int` return value and
//! (b) the bytes it pushes into the C `stdout` stream.  The harness therefore
//! provides fd-level capture of `stdout` plus control over the stream's
//! buffering mode and the kind of descriptor underneath it.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bindings (resolved from the test binary's own libc, which is the very
// same libc.so.6 both shared objects link against, hence the same `stdout`).
// ---------------------------------------------------------------------------

pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;
pub const O_CREAT: c_int = 0o100;
pub const O_TRUNC: c_int = 0o1000;
pub const O_APPEND: c_int = 0o2000;
pub const O_DIRECTORY: c_int = 0o200000;

pub const IOFBF: c_int = 0;
pub const IOLBF: c_int = 1;
pub const IONBF: c_int = 2;

pub const SEEK_SET: c_int = 0;
pub const SEEK_END: c_int = 2;

unsafe extern "C" {
    pub safe fn dup(oldfd: c_int) -> c_int;
    pub safe fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    pub safe fn close(fd: c_int) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    pub fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    pub safe fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    pub fn fflush(stream: *mut c_void) -> c_int;
    pub fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    pub fn clearerr(stream: *mut c_void);
    pub fn ferror(stream: *mut c_void) -> c_int;
    pub fn fputs(s: *const c_char, stream: *mut c_void) -> c_int;
    pub safe fn __errno_location() -> *mut c_int;
    pub fn signal(sig: c_int, handler: usize) -> usize;

    /// glibc's `FILE *stdout`.
    pub static mut stdout: *mut c_void;
}

pub const EBADF: c_int = 9;
pub const EISDIR: c_int = 21;
pub const EPIPE: c_int = 32;

pub const SIGPIPE: c_int = 13;
pub const SIG_IGN: usize = 1;

/// Make `write(2)` on a broken pipe report `EPIPE` instead of killing the
/// process.  Applied identically before the C and the Rust run, so the
/// comparison stays apples-to-apples.
pub fn ignore_sigpipe() {
    unsafe { signal(SIGPIPE, SIG_IGN) };
}

/// The `FILE*` that both `.so`s write through.
pub fn stdout_file() -> *mut c_void {
    unsafe { stdout }
}

pub fn errno() -> c_int {
    unsafe { *__errno_location() }
}

pub fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}

// ---------------------------------------------------------------------------
// The two implementations under test
// ---------------------------------------------------------------------------

/// `int helloworld(void)` — the one and only exported symbol.
pub type HelloFn = unsafe extern "C" fn() -> c_int;

/// The same symbol seen through C's unprototyped `int helloworld();`
/// declaration, which lets a caller pass any number of arguments.
pub type HelloFnExtraArgs = unsafe extern "C" fn(c_int, c_int, c_int, c_int, f64) -> c_int;

/// The same symbol called with a variadic signature.
pub type HelloFnVariadic = unsafe extern "C" fn(c_int, ...) -> c_int;

/// The same symbol called with more integer arguments than the SysV ABI has
/// registers for, so some of them are pushed on the stack.
pub type HelloFnManyArgs =
    unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

/// Which implementation a value came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Which {
    C,
    Rust,
}

impl Which {
    pub const BOTH: [Which; 2] = [Which::C, Which::Rust];

    pub fn name(self) -> &'static str {
        match self {
            Which::C => "C",
            Which::Rust => "Rust",
        }
    }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libhello.so`, produced by the CMake build.
pub fn c_lib_path() -> PathBuf {
    let root = crate_dir()
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf();
    let p = root.join("c_src/build/libhello.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// This crate's `cdylib`, found next to the running test binary
/// (`<target>/<profile>/libhello.so`), so it works with any `CARGO_TARGET_DIR`
/// and any profile.
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELLO_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // <target>/<profile>/deps/<test-bin>  ->  <target>/<profile>
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("test binary lives in <target>/<profile>/deps");
    let p = profile_dir.join("libhello.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {} — run `cargo build` first",
        p.display()
    );
    assert_so_is_fresh(&p);
    p
}

/// Newest mtime across everything that feeds the `cdylib`.
fn newest_source_mtime() -> (std::time::SystemTime, PathBuf) {
    let mut newest = (std::time::SystemTime::UNIX_EPOCH, PathBuf::new());
    let mut consider = |p: PathBuf| {
        if let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if m > newest.0 {
                newest = (m, p);
            }
        }
    };
    consider(crate_dir().join("Cargo.toml"));

    let mut stack = vec![crate_dir().join("src")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                consider(p);
            }
        }
    }
    newest
}

/// **Guards against the most dangerous failure mode of this whole test suite.**
///
/// `cargo test` does *not* build a `cdylib`-only library target: only
/// `cargo build` produces `<target>/<profile>/libhello.so`.  So a bare
/// `cargo test` happily runs every differential test against whatever `.so` was
/// left over from an earlier build.  A stale artifact makes the entire suite
/// pass vacuously — source changes, including regressions, are simply not under
/// test.
///
/// Failing loudly here turns that silent false pass into an obvious error.
fn assert_so_is_fresh(so: &Path) {
    let so_m = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat the Rust cdylib");
    let (src_m, src) = newest_source_mtime();
    assert!(
        so_m >= src_m,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib-only library target, so the \
         differential tests would run against an out-of-date .so and pass \
         vacuously.\n\
         Rebuild first:  cargo build{}   (or just run ./verify.sh)",
        so.display(),
        src.display(),
        if so.to_string_lossy().contains("/release/") {
            " --release"
        } else {
            ""
        }
    );
}

/// Both libraries, dlopen'd once (RTLD_LOCAL, so the two identically named
/// `helloworld` symbols cannot shadow one another) and leaked for `'static`
/// access.
pub fn libs() -> [&'static Library; 2] {
    static LIBS: OnceLock<[&'static Library; 2]> = OnceLock::new();
    *LIBS.get_or_init(|| {
        let c = unsafe { Library::new(c_lib_path()) }.expect("dlopen C libhello.so");
        let r = unsafe { Library::new(rust_lib_path()) }.expect("dlopen Rust libhello.so");
        [
            &*Box::leak(Box::new(c)) as &'static Library,
            &*Box::leak(Box::new(r)) as &'static Library,
        ]
    })
}

pub fn lib(which: Which) -> &'static Library {
    match which {
        Which::C => libs()[0],
        Which::Rust => libs()[1],
    }
}

/// Resolve `helloworld` from the given `.so` — a fresh `dlsym` on every call.
pub fn resolve(which: Which) -> HelloFn {
    let l = lib(which);
    let sym: Symbol<HelloFn> = unsafe { l.get(b"helloworld\0") }
        .unwrap_or_else(|e| panic!("{} .so does not export `helloworld`: {e}", which.name()));
    *sym
}

/// Cached function pointers for the hot paths.
pub fn hello(which: Which) -> HelloFn {
    static FNS: OnceLock<[HelloFn; 2]> = OnceLock::new();
    let fns = FNS.get_or_init(|| [resolve(Which::C), resolve(Which::Rust)]);
    match which {
        Which::C => fns[0],
        Which::Rust => fns[1],
    }
}

/// `helloworld` re-typed as the unprototyped-with-arguments C call.
pub fn hello_extra_args(which: Which) -> HelloFnExtraArgs {
    unsafe { std::mem::transmute::<HelloFn, HelloFnExtraArgs>(hello(which)) }
}

/// `helloworld` re-typed as a variadic C call.
pub fn hello_variadic(which: Which) -> HelloFnVariadic {
    unsafe { std::mem::transmute::<HelloFn, HelloFnVariadic>(hello(which)) }
}

/// `helloworld` re-typed as an 8-integer-argument C call (stack arguments).
pub fn hello_many_args(which: Which) -> HelloFnManyArgs {
    unsafe { std::mem::transmute::<HelloFn, HelloFnManyArgs>(hello(which)) }
}

/// The exact byte sequence one call must emit.
pub const LINE: &[u8] = b"Hello World!\n";

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — no external crate, fully reproducible.
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

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(lo <= hi);
        lo + self.next_u64() % (hi - lo + 1)
    }

    pub fn usize_range(&mut self, lo: usize, hi: usize) -> usize {
        self.range(lo as u64, hi as u64) as usize
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.usize_range(0, xs.len() - 1)]
    }

    /// A short random ASCII token, used as a foreign payload interleaved with
    /// the library's own output.
    pub fn token(&mut self) -> String {
        let n = self.usize_range(1, 8);
        (0..n)
            .map(|_| (b'a' + (self.next_u64() % 26) as u8) as char)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Serialises every test that touches process-wide fd 1 / the `stdout` stream.
pub fn stdout_lock() -> &'static Mutex<()> {
    assert_serial_execution();
    static L: Mutex<()> = Mutex::new(());
    &L
}

/// Capturing `helloworld`'s output means redirecting the *process-wide* fd 1.
/// libtest writes its own progress lines to fd 1 as well, so if another test is
/// running concurrently its `test ... ok` output lands inside our capture file
/// and the byte comparison fails for a reason that has nothing to do with the
/// translation.  The internal `stdout_lock()` cannot prevent that — libtest's
/// writes do not take it.
///
/// So the whole suite must run serially.  `translation/.cargo/config.toml` sets
/// `RUST_TEST_THREADS=1` to arrange that automatically; this check makes the
/// requirement explicit instead of letting it degrade into flaky failures.
fn assert_serial_execution() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        if std::env::var("RUST_TEST_THREADS").as_deref() == Ok("1") {
            return;
        }
        // Also accept `cargo test -- --test-threads=1` / `--test-threads 1`.
        let args: Vec<String> = std::env::args().collect();
        let via_cli = args.windows(2).any(|w| w[0] == "--test-threads" && w[1] == "1")
            || args.iter().any(|a| a == "--test-threads=1");
        assert!(
            via_cli,
            "\nThese differential tests redirect the process-wide stdout \
             descriptor, so they must run one at a time — otherwise libtest's own \
             progress output is captured alongside `helloworld`'s and the byte \
             comparison fails spuriously.\n\
             Run them serially:\n  \
               cargo test -- --test-threads=1\n  \
             or with RUST_TEST_THREADS=1 (translation/.cargo/config.toml sets \
             this for you), or just run ./verify.sh\n"
        );
    });
}

fn scratch_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let d = exe
        .parent()
        .and_then(Path::parent)
        .expect("<target>/<profile>")
        .join("difftmp");
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

fn unique_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    scratch_dir().join(format!("{tag}-{}-{n}.bin", std::process::id()))
}

/// A fresh, empty scratch file, for tests that need to drive `stdout` by hand.
pub fn scratch_file(tag: &str) -> PathBuf {
    let p = unique_path(tag);
    std::fs::write(&p, b"").expect("create scratch file");
    p
}

/// `open(path, O_RDWR)`, panicking on failure.
pub fn open_rw(path: &Path) -> c_int {
    let cp = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let fd = unsafe { open(cp.as_ptr(), O_RDWR, 0o600 as c_int) };
    assert!(fd >= 0, "open({}) failed, errno {}", path.display(), errno());
    fd
}

/// Where the redirected `stdout` should point.
#[derive(Clone, Copy, Debug)]
pub enum Sink {
    /// A fresh, empty regular file (bytes are recoverable).
    File,
    /// A fresh regular file pre-seeded with `n` filler bytes; the descriptor is
    /// left positioned at the end, so output must land at a non-zero offset.
    FileWithPrefix(usize),
    /// A fresh regular file opened `O_APPEND`.
    FileAppend,
    /// The write end of a pipe (bytes are recoverable by draining it).
    Pipe,
    /// `/dev/null` — writes succeed, bytes are unobservable.
    DevNull,
}

/// How the `stdout` stream should be buffered during the call.
#[derive(Clone, Copy, Debug)]
pub enum Buffering {
    /// Leave the stream's buffering as-is.
    Default,
    /// `setvbuf(stdout, NULL, _IOFBF, 0)` — fully buffered, glibc's own buffer.
    Full,
    /// `setvbuf(stdout, buf, _IOFBF, n)` — fully buffered with a tiny
    /// caller-owned buffer, which forces writes to be split mid-line.
    FullTiny(usize),
    /// `setvbuf(stdout, NULL, _IOLBF, 0)` — line buffered.
    Line,
    /// `setvbuf(stdout, NULL, _IONBF, 0)` — unbuffered; every `puts` becomes an
    /// immediate `write(2)`.
    NoneBuf,
}

/// Handed to the body of a captured run so it can also write to the stream
/// itself and peek at what has reached the file so far.
pub struct Ctx {
    /// The descriptor that fd 1 currently aliases.
    pub fd: c_int,
    path: Option<PathBuf>,
}

impl Ctx {
    /// Write `s` through the *same* `FILE*` the library uses.
    pub fn stdio_write(&self, s: &str) {
        let cs = CString::new(s).unwrap();
        let r = unsafe { fputs(cs.as_ptr(), stdout_file()) };
        assert!(r >= 0, "fputs failed");
    }

    /// Write `s` straight to fd 1, bypassing the `FILE*` buffer.
    pub fn raw_write(&self, s: &str) {
        let b = s.as_bytes();
        let n = unsafe { write(1, b.as_ptr() as *const c_void, b.len()) };
        assert_eq!(n, b.len() as isize, "raw write to fd 1 failed");
    }

    /// Bytes that have physically reached the backing file so far (regular-file
    /// sinks only).  Used to prove that e.g. unbuffered mode really is in
    /// effect before any `fflush`.
    pub fn snapshot(&self) -> Vec<u8> {
        match &self.path {
            Some(p) => std::fs::read(p).unwrap_or_default(),
            None => Vec::new(),
        }
    }
}

/// Result of one captured run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run<R> {
    /// Whatever the body returned (typically the collected `helloworld` return
    /// values).
    pub value: R,
    /// Every byte that reached the sink.
    pub bytes: Vec<u8>,
    /// `ferror(stdout)` observed *before* the stream was cleaned up.
    pub ferror: c_int,
}

/// Puts `stdout` back the way it was — on the normal path *and* while
/// unwinding from a failed assertion inside a capture.
struct RestoreStdout {
    saved: c_int,
    /// A caller-supplied stdio buffer, if one was installed.  Owned here so it
    /// outlives the stream's reference to it.
    tiny: Vec<c_char>,
}

impl Drop for RestoreStdout {
    fn drop(&mut self) {
        // Return the stream to a glibc-owned buffer *before* `tiny` dies.
        //
        // `setvbuf(fp, NULL, _IOFBF, 0)` is NOT enough: glibc's `_IO_setvbuf`
        // returns early for `_IOFBF` with a NULL buffer whenever the stream
        // already has one, so it would keep pointing at `tiny` and every later
        // write would scribble over freed memory.  `_IONBF` goes through
        // `_IO_SETBUF(fp, NULL, 0)`, which really does drop the stream's
        // reference; only then can the following `_IOFBF` hand it a fresh
        // internal buffer.
        unsafe {
            setvbuf(stdout_file(), std::ptr::null_mut(), IONBF, 0);
            setvbuf(stdout_file(), std::ptr::null_mut(), IOFBF, 0);
            clearerr(stdout_file());
        }
        self.tiny = Vec::new();

        if dup2(self.saved, 1) < 0 && !std::thread::panicking() {
            panic!("restoring fd 1 failed");
        }
        close(self.saved);
    }
}

/// Run `body` with fd 1 redirected to `sink` and `stdout` buffered as
/// requested, then restore everything and hand back the emitted bytes.
///
/// The redirect is done at the descriptor level, so it captures output no
/// matter which `.so` produced it, and the surrounding `fflush`es make the
/// captured bytes independent of when glibc happens to drain its buffer.
pub fn run_captured<R>(sink: Sink, buffering: Buffering, body: impl FnOnce(&mut Ctx) -> R) -> Run<R> {
    // dlopen both libraries and run the staleness check BEFORE fd 1 is
    // redirected, so a load failure reports itself on the real stderr/stdout
    // instead of disappearing into the capture file.
    let _ = libs();

    let _guard = stdout_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Don't let the harness's own pending output land in the capture.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    unsafe { fflush(stdout_file()) };

    let mut path: Option<PathBuf> = None;
    let mut pipe_read: Option<c_int> = None;
    let target: c_int;

    match sink {
        Sink::File | Sink::FileWithPrefix(_) | Sink::FileAppend => {
            let p = unique_path("cap");
            if let Sink::FileWithPrefix(n) = sink {
                std::fs::write(&p, vec![b'.'; n]).expect("seed capture file");
            } else {
                std::fs::write(&p, b"").expect("create capture file");
            }
            let cp = CString::new(p.as_os_str().as_encoded_bytes()).unwrap();
            let flags = match sink {
                Sink::FileAppend => O_RDWR | O_APPEND,
                _ => O_RDWR,
            };
            let fd = unsafe { open(cp.as_ptr(), flags, 0o600 as c_int) };
            assert!(fd >= 0, "open({}) failed, errno {}", p.display(), errno());
            if matches!(sink, Sink::FileWithPrefix(_)) {
                assert!(lseek(fd, 0, SEEK_END) >= 0, "lseek to end failed");
            }
            path = Some(p);
            target = fd;
        }
        Sink::Pipe => {
            let mut fds = [0 as c_int; 2];
            assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
            pipe_read = Some(fds[0]);
            target = fds[1];
        }
        Sink::DevNull => {
            let cp = CString::new("/dev/null").unwrap();
            let fd = unsafe { open(cp.as_ptr(), O_WRONLY, 0 as c_int) };
            assert!(fd >= 0, "open(/dev/null) failed");
            target = fd;
        }
    }

    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    assert!(dup2(target, 1) >= 0, "dup2(target, 1) failed");

    // From here on the restoration MUST happen even if `body` panics: if fd 1
    // were left pointing at the capture file, libtest's failure report would be
    // written into that file and the real divergence message would vanish.
    let mut restore = RestoreStdout {
        saved,
        tiny: Vec::new(),
    };

    // Apply the requested buffering.  A caller-owned buffer must outlive every
    // use of the stream, so it is owned by the restoration guard and dropped
    // only after the stream has been handed back a glibc-owned buffer.
    let tiny = &mut restore.tiny;
    match buffering {
        Buffering::Default => {}
        Buffering::Full => assert_eq!(
            unsafe { setvbuf(stdout_file(), std::ptr::null_mut(), IOFBF, 0) },
            0,
            "setvbuf(_IOFBF) failed"
        ),
        Buffering::FullTiny(n) => {
            *tiny = vec![0 as c_char; n.max(1)];
            assert_eq!(
                unsafe { setvbuf(stdout_file(), tiny.as_mut_ptr(), IOFBF, n.max(1)) },
                0,
                "setvbuf(_IOFBF, tiny) failed"
            );
        }
        Buffering::Line => assert_eq!(
            unsafe { setvbuf(stdout_file(), std::ptr::null_mut(), IOLBF, 0) },
            0,
            "setvbuf(_IOLBF) failed"
        ),
        Buffering::NoneBuf => assert_eq!(
            unsafe { setvbuf(stdout_file(), std::ptr::null_mut(), IONBF, 0) },
            0,
            "setvbuf(_IONBF) failed"
        ),
    }

    let mut ctx = Ctx {
        fd: target,
        path: path.clone(),
    };
    let value = body(&mut ctx);

    unsafe { fflush(stdout_file()) };
    let ferr = unsafe { ferror(stdout_file()) };

    // Undo the buffering change and put fd 1 back (also done by `Drop` if the
    // body panicked instead of returning).
    drop(restore);

    let bytes = match sink {
        Sink::Pipe => {
            close(target); // EOF for the reader
            let rfd = pipe_read.unwrap();
            let mut out = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = unsafe { read(rfd, buf.as_mut_ptr() as *mut c_void, buf.len()) };
                if n <= 0 {
                    break;
                }
                out.extend_from_slice(&buf[..n as usize]);
            }
            close(rfd);
            out
        }
        Sink::DevNull => {
            close(target);
            Vec::new()
        }
        _ => {
            close(target);
            let p = path.as_ref().unwrap();
            let b = std::fs::read(p).unwrap_or_default();
            let _ = std::fs::remove_file(p);
            b
        }
    };

    Run {
        value,
        bytes,
        ferror: ferr,
    }
}

/// Run the *same* scripted body against both `.so`s and require that the return
/// values, the emitted bytes and the resulting stream error state all match.
pub fn diff<R: PartialEq + std::fmt::Debug>(
    label: &str,
    sink: Sink,
    buffering: Buffering,
    mut body: impl FnMut(Which, &mut Ctx) -> R,
) -> Run<R> {
    let c = run_captured(sink, buffering, |ctx| body(Which::C, ctx));
    let r = run_captured(sink, buffering, |ctx| body(Which::Rust, ctx));

    assert_eq!(
        c.value, r.value,
        "[{label}] return values differ (sink={sink:?}, buffering={buffering:?})"
    );
    assert_eq!(
        show(&c.bytes),
        show(&r.bytes),
        "[{label}] emitted bytes differ (sink={sink:?}, buffering={buffering:?})"
    );
    assert_eq!(
        c.bytes, r.bytes,
        "[{label}] emitted bytes differ (sink={sink:?}, buffering={buffering:?})"
    );
    assert_eq!(
        c.ferror != 0,
        r.ferror != 0,
        "[{label}] ferror(stdout) differs (sink={sink:?}, buffering={buffering:?})"
    );
    c
}

/// Readable rendering of a captured stream for assertion messages.
pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace('\n', "\\n")
}

/// `LINE` repeated `n` times.
pub fn lines(n: usize) -> Vec<u8> {
    LINE.repeat(n)
}

// ---------------------------------------------------------------------------
// Low-level stdout surgery, for the Phase C error paths
// ---------------------------------------------------------------------------

/// An RAII scope that owns the process's `stdout`.
///
/// It takes the global stdout lock, flushes both the C stream and Rust's
/// `io::Stdout`, and remembers fd 1 so it can be put back.  Inside the scope a
/// test is free to point fd 1 at a hostile descriptor, or close it outright, to
/// make `puts` fail.  On drop the stream's buffering, error flag and descriptor
/// are all restored.
pub struct StdoutSwap {
    saved: c_int,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl StdoutSwap {
    pub fn new() -> Self {
        let guard = stdout_lock().lock().unwrap_or_else(|e| e.into_inner());
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        unsafe { fflush(stdout_file()) };
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        StdoutSwap {
            saved,
            _guard: guard,
        }
    }

    /// Alias fd 1 onto `fd`.
    pub fn point_at(&self, fd: c_int) {
        assert!(dup2(fd, 1) >= 0, "dup2({fd}, 1) failed");
    }

    /// Close fd 1, so every `write(1, ...)` fails with `EBADF`.
    pub fn close_fd1(&self) {
        assert_eq!(close(1), 0, "close(1) failed");
    }

    /// Turn off buffering so a failing `puts` fails *during* the call rather
    /// than at some later flush.
    pub fn unbuffered(&self) {
        assert_eq!(
            unsafe { setvbuf(stdout_file(), std::ptr::null_mut(), IONBF, 0) },
            0,
            "setvbuf(_IONBF) failed"
        );
    }

    pub fn ferror(&self) -> c_int {
        unsafe { ferror(stdout_file()) }
    }

    /// Deliberately put the stream into its error state by attempting a write
    /// that cannot succeed.  Leaves the error indicator set.
    pub fn poison(&self) {
        let cs = CString::new("poison").unwrap();
        unsafe { fputs(cs.as_ptr(), stdout_file()) };
        unsafe { fflush(stdout_file()) };
        assert_ne!(self.ferror(), 0, "failed to set the stdout error flag");
    }
}

impl Drop for StdoutSwap {
    fn drop(&mut self) {
        unsafe {
            // Discard anything the hostile descriptor could not take.
            setvbuf(stdout_file(), std::ptr::null_mut(), IONBF, 0);
            clearerr(stdout_file());
        }
        assert!(dup2(self.saved, 1) >= 0, "restoring fd 1 failed");
        close(self.saved);
        unsafe {
            setvbuf(stdout_file(), std::ptr::null_mut(), IOFBF, 0);
            clearerr(stdout_file());
        }
        set_errno(0);
    }
}

/// A file descriptor that no `write` can ever succeed on.
#[derive(Clone, Copy, Debug)]
pub enum HostileFd {
    /// fd 1 is closed outright.
    Closed,
    /// fd 1 is a regular file opened `O_RDONLY`.
    ReadOnly,
    /// fd 1 is the write end of a pipe whose read end is gone.
    BrokenPipe,
    /// fd 1 is a directory.
    Directory,
}

/// Everything observable about one call made against a hostile `stdout`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrRun {
    /// Return value(s) of `helloworld`.
    pub ret: Vec<c_int>,
    /// Whether `ferror(stdout)` was set afterwards.
    pub failed: bool,
    /// `errno` after the call.
    pub errno: c_int,
}

/// Install `kind` as `stdout`, run `body`, then restore.
pub fn run_with_hostile_stdout(kind: HostileFd, body: impl FnOnce() -> Vec<c_int>) -> ErrRun {
    ignore_sigpipe();
    let swap = StdoutSwap::new();
    // Unbuffer *before* breaking fd 1, so the setvbuf itself cannot fail.
    swap.unbuffered();

    // Keep the auxiliary descriptors alive for the duration of the call.
    let mut aux: Vec<c_int> = Vec::new();
    let mut path: Option<PathBuf> = None;

    match kind {
        HostileFd::Closed => swap.close_fd1(),
        HostileFd::ReadOnly => {
            let p = unique_path("ro");
            std::fs::write(&p, b"unwritable").unwrap();
            let cp = CString::new(p.as_os_str().as_encoded_bytes()).unwrap();
            let fd = unsafe { open(cp.as_ptr(), O_RDONLY, 0 as c_int) };
            assert!(fd >= 0, "open O_RDONLY failed");
            swap.point_at(fd);
            aux.push(fd);
            path = Some(p);
        }
        HostileFd::BrokenPipe => {
            let mut fds = [0 as c_int; 2];
            assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
            close(fds[0]); // reader gone
            swap.point_at(fds[1]);
            aux.push(fds[1]);
        }
        HostileFd::Directory => {
            let d = scratch_dir();
            let cp = CString::new(d.as_os_str().as_encoded_bytes()).unwrap();
            let fd = unsafe { open(cp.as_ptr(), O_RDONLY | O_DIRECTORY, 0 as c_int) };
            assert!(fd >= 0, "open(dir) failed");
            swap.point_at(fd);
            aux.push(fd);
        }
    }

    set_errno(0);
    let ret = body();
    let failed = swap.ferror() != 0;
    let err = errno();

    drop(swap);
    for fd in aux {
        close(fd);
    }
    if let Some(p) = path {
        let _ = std::fs::remove_file(p);
    }

    ErrRun {
        ret,
        failed,
        errno: err,
    }
}

/// Run the same hostile-stdout scenario against both `.so`s and require the
/// same return values, the same failure signal and the same `errno`.
pub fn diff_error(label: &str, kind: HostileFd, mut body: impl FnMut(Which) -> Vec<c_int>) -> ErrRun {
    let c = run_with_hostile_stdout(kind, || body(Which::C));
    let r = run_with_hostile_stdout(kind, || body(Which::Rust));

    assert_eq!(
        c.ret, r.ret,
        "[{label}] return values differ under {kind:?} (C={:?} Rust={:?})",
        c.ret, r.ret
    );
    assert_eq!(
        c.failed, r.failed,
        "[{label}] ferror(stdout) differs under {kind:?}"
    );
    assert_eq!(
        c.errno, r.errno,
        "[{label}] errno differs under {kind:?} (C={} Rust={})",
        c.errno, r.errno
    );
    c
}
