//! Differential-test harness shared by the Phase B and Phase C test binaries.
//!
//! Rules of engagement:
//!
//! * The code under test is NEVER called directly from Rust. Both the C and the
//!   Rust implementation are `dlopen`ed (via `libloading`) and reached through
//!   `dlsym`, exactly like an external consumer, so the `#[no_mangle]` export
//!   wrappers are part of what is being verified.
//! * Every scenario runs in a `fork()`ed child, which gives each run a private
//!   copy of the file-descriptor table and of glibc's `FILE *stdout` state.
//!   That keeps the parent's fd 1 (used by libtest to print progress)
//!   untouched, makes buffering-mode selection deterministic, and prevents a
//!   scenario that leaves `stdout` in an error state from leaking into the
//!   next one.
//! * The libc functions declared below are harness plumbing (fd redirection,
//!   stream configuration, process control) — they are not the code under test.

#![allow(dead_code)]

use libloading::Library;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The exact bytes `printf("Hello World!\n")` must put on fd 1.
pub const HELLO: &[u8] = b"Hello World!\n";

/// Signature of both exported functions: `int f()`.
pub type CFn = unsafe extern "C" fn() -> c_int;

extern "C" {
    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    pub fn _exit(code: c_int) -> !;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, mode: c_uint) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    pub fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    pub fn fflush(stream: *mut c_void) -> c_int;
    pub fn clearerr(stream: *mut c_void);
    pub fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    pub fn printf(format: *const c_char, ...) -> c_int;
    pub fn signal(sig: c_int, handler: usize) -> usize;
    /// glibc's `FILE *stdout` — the very stream the C code prints to.
    pub static mut stdout: *mut c_void;
}

pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;
pub const O_CREAT: c_int = 0o100;
pub const O_TRUNC: c_int = 0o1000;
pub const IOFBF: c_int = 0;
pub const IOLBF: c_int = 1;
pub const IONBF: c_int = 2;
pub const SIGPIPE: c_int = 13;
pub const SIG_IGN: usize = 1;
pub const SIG_DFL: usize = 0;

pub fn wifexited(status: c_int) -> bool {
    (status & 0x7f) == 0
}
pub fn wexitstatus(status: c_int) -> c_int {
    (status >> 8) & 0xff
}
pub fn wtermsig(status: c_int) -> c_int {
    status & 0x7f
}

/// Deterministic PRNG (SplitMix64) so every randomized row is reproducible.
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
    /// Uniform-ish value in `lo..=hi`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next_u64() % (hi - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            // printable ASCII, so a divergence is easy to eyeball in a diff
            .map(|_| 0x21 + (self.next_u64() % 0x5e) as u8)
            .collect()
    }
    /// Random blob whose length is itself random in `lo..=hi`.
    pub fn blob(&mut self, lo: u64, hi: u64) -> Vec<u8> {
        let len = self.range(lo, hi) as usize;
        self.bytes(len)
    }
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of the C shared object, building it from `c_src/src/*.c` if needed.
///
/// `c_src/CMakeLists.txt` only declares an executable, so the shared object is
/// produced out-of-tree with the same two translation units and the same
/// (default, i.e. no `-O`) flags. Nothing in `c_src/` is written to.
/// `DRIVER_C_SO` overrides the choice (used to also test an optimized build).
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_C_SO={} is not a file", p.display());
        return p;
    }
    let dir = manifest_dir().join("target/c_build");
    let so = dir.join("libcdriver.so");
    let sources = [
        manifest_dir().join("c_src/src/sillymain.c"),
        manifest_dir().join("c_src/src/main.c"),
    ];
    let needs_build = match std::fs::metadata(&so).and_then(|m| m.modified()) {
        Ok(so_time) => sources.iter().any(|s| {
            std::fs::metadata(s)
                .and_then(|m| m.modified())
                .map(|t| t > so_time)
                .unwrap_or(true)
        }),
        Err(_) => true,
    };
    if needs_build {
        std::fs::create_dir_all(&dir).expect("create target/c_build");
        let out = std::process::Command::new("gcc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-o")
            .arg(&so)
            .args(&sources)
            .output()
            .expect("run gcc to build the C shared object");
        assert!(
            out.status.success(),
            "building the C shared object failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    so
}

/// Path of the Rust `cdylib`, searched relative to the test executable so that
/// it works for any profile and any `CARGO_TARGET_DIR`.
///
/// `cargo test` does not build `cdylib` artifacts, so if `cargo build` has not
/// run (or ran into a different target dir) the library is compiled here with
/// `rustc` straight from `src/lib.rs` — same sources, same edition, still a real
/// `.so` that has to be `dlopen`ed. `verify.sh` runs `cargo build` first, so the
/// cargo-produced artifact is what is normally used.
/// `DRIVER_RUST_SO` overrides the choice (used for negative controls).
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_SO={} is not a file", p.display());
        return p;
    }
    // Cargo leaves copies in both `target/<profile>/` and
    // `target/<profile>/deps/`; pick the most recently written one so a stale
    // artifact can never shadow a fresh build.
    let exe = std::env::current_exe().expect("current_exe");
    let mut candidates: Vec<PathBuf> =
        exe.ancestors().map(|d| d.join("libdriver.so")).collect();
    for profile in ["debug", "release"] {
        candidates.push(manifest_dir().join("target").join(profile).join("libdriver.so"));
    }
    let newest = candidates
        .into_iter()
        .filter(|p| p.is_file())
        .filter_map(|p| {
            let t = std::fs::metadata(&p).and_then(|m| m.modified()).ok()?;
            Some((t, p))
        })
        .max_by_key(|(t, _)| *t);
    match newest {
        Some((_, p)) => p,
        None => build_rust_lib_with_rustc(),
    }
}

fn build_rust_lib_with_rustc() -> PathBuf {
    let dir = manifest_dir().join("target/c_build");
    std::fs::create_dir_all(&dir).expect("create target/c_build");
    let so = dir.join("libdriver.so");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let out = std::process::Command::new(rustc)
        .args(["--edition", "2021", "--crate-type", "cdylib", "--crate-name", "driver"])
        .arg(manifest_dir().join("src/lib.rs"))
        .arg("-o")
        .arg(&so)
        .output()
        .expect("run rustc to build the Rust cdylib");
    assert!(
        out.status.success(),
        "building the Rust cdylib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    so
}

/// The two exported entry points of one library, as raw C function pointers.
#[derive(Copy, Clone)]
pub struct Fns {
    pub hello: CFn,
    pub main_: CFn,
}

struct Loaded {
    _c: Library,
    _rust: Library,
    c: Fns,
    rust: Fns,
}

// SAFETY: `Loaded` only holds `libloading::Library` handles (Send + Sync) and
// plain function pointers into those libraries.
unsafe impl Send for Loaded {}
unsafe impl Sync for Loaded {}

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| unsafe {
        let c = Library::new(c_lib_path()).expect("dlopen C libcdriver.so");
        let rust = Library::new(rust_lib_path()).expect("dlopen Rust libdriver.so");
        let c_fns = Fns {
            hello: *c.get::<CFn>(b"helloworld\0").expect("dlsym C helloworld"),
            main_: *c.get::<CFn>(b"main\0").expect("dlsym C main"),
        };
        let rust_fns = Fns {
            hello: *rust.get::<CFn>(b"helloworld\0").expect("dlsym Rust helloworld"),
            main_: *rust.get::<CFn>(b"main\0").expect("dlsym Rust main"),
        };
        Loaded {
            _c: c,
            _rust: rust,
            c: c_fns,
            rust: rust_fns,
        }
    })
}

/// C implementation, reached only through `dlopen`/`dlsym`.
pub fn c_fns() -> Fns {
    loaded().c
}

/// Rust implementation, reached only through `dlopen`/`dlsym` — never called as
/// a Rust function.
pub fn rust_fns() -> Fns {
    loaded().rust
}

/// Serializes scenarios so parallel test threads cannot interleave forks,
/// temp-file names or `stdout` stream state.
pub fn harness_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn tmp_path(tag: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut dir = std::env::temp_dir();
    dir.push(format!("driver-diff-{}-{}-{}.out", std::process::id(), tag, n));
    dir
}

fn cpath(p: &Path) -> Vec<u8> {
    let mut v = p.as_os_str().as_encoded_bytes().to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Scenario description
// ---------------------------------------------------------------------------

/// Which exported entry point to call.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// `int helloworld()` — the lowest-level public function.
    Hello,
    /// `int main()` — the composed entry point (`return helloworld();`).
    Main,
}

/// Argument shapes an FFI caller can legally throw at the C functions.
///
/// `int helloworld();` and `int main()` are *unprototyped* (K&R) declarators, so
/// a C caller may pass any arguments and the callee must ignore them. Each
/// variant is called by transmuting the resolved symbol to the corresponding
/// `extern "C"` signature — the same machine-level call a C caller would make
/// on the SysV AMD64 ABI (integer registers, SSE registers and stack slots).
#[derive(Clone, Debug, PartialEq)]
pub enum ArgShape {
    /// `int f(void)` — the declared shape.
    None,
    /// `int f(int)`
    Int(c_int),
    /// `int f(int, int)`
    TwoInts(c_int, c_int),
    /// `int f(int, void *)` — the conventional `main(argc, argv)` shape.
    IntPtr(c_int, usize),
    /// `int f(void *)`
    Ptr(usize),
    /// `int f(size_t)`
    Size(usize),
    /// `int f(int, int, int, int, int, int)` — fills every integer register.
    SixInts([c_int; 6]),
    /// Mixed integer/floating point plus stack arguments.
    Mixed { i: c_int, p: usize, d: f64, u: u64, f: f32, extra: [i64; 4] },
    /// `long f(void)` — inspects the full 64-bit return register; only the low
    /// 32 bits (the C `int`) are reported, since the upper half is
    /// ABI-undefined for a function returning `int`.
    RetLong,
}

/// One action of a scenario. A scenario is driven identically against both
/// libraries; `lib` selects which of the two function tables a call uses (used
/// by the cross-library interleaving row).
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Call { entry: Entry, lib: u8 },
    /// Like `Call`, but through a different (legal for an unprototyped C
    /// declarator) FFI signature.
    CallWith { entry: Entry, lib: u8, args: ArgShape },
    /// Raw `write(2)` to fd 1: bypasses stdio, so it exposes *when* the
    /// library's own output reaches the fd.
    Marker(Vec<u8>),
    /// The caller prints through the same `FILE *stdout` the library uses.
    CallerPrint(Vec<u8>),
    /// `fflush(stdout)`
    Flush,
    /// `fflush(NULL)`
    FlushAll,
    /// `setvbuf(stdout, buf, mode, size)`; `size == 0` passes a NULL buffer.
    SetVbuf { mode: c_int, size: usize },
}

impl Step {
    pub fn hello() -> Step {
        Step::Call { entry: Entry::Hello, lib: 0 }
    }
    pub fn main_() -> Step {
        Step::Call { entry: Entry::Main, lib: 0 }
    }
    pub fn call(entry: Entry, lib: u8) -> Step {
        Step::Call { entry, lib }
    }
    pub fn call_with(entry: Entry, args: ArgShape) -> Step {
        Step::CallWith { entry, lib: 0, args }
    }
    pub fn caller_print(payload: &[u8]) -> Step {
        let mut v = payload.to_vec();
        v.push(0);
        Step::CallerPrint(v)
    }
}

/// Where fd 1 points during the scenario.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dest {
    /// Regular file — glibc picks full buffering; contents are captured.
    File,
    /// Pipe — glibc picks full buffering; contents are captured.
    Pipe,
    /// `/dev/null` — writes succeed, nothing observable.
    DevNull,
    /// `/dev/full` — every write fails with `ENOSPC`.
    DevFull,
    /// fd 1 is an `O_RDONLY` description — writes fail with `EBADF`.
    ReadOnly,
    /// fd 1 is closed — writes fail with `EBADF`.
    Closed,
    /// fd 1 is a pipe with no reader — writes fail with `EPIPE`.
    BrokenPipe,
}

impl Dest {
    fn captures(self) -> bool {
        matches!(self, Dest::File | Dest::Pipe)
    }
}

#[derive(Clone, Debug)]
pub struct RunOpts {
    pub dest: Dest,
    /// Whether the child flushes stdio before `_exit` (a normal `exit()` would;
    /// `false` models being killed before the buffers are drained).
    pub final_flush: bool,
    /// Disposition installed for `SIGPIPE` in the child before any call:
    /// `None` inherits (a Rust test binary starts with `SIG_IGN`), while
    /// `Some(SIG_DFL)` reproduces what a C program starts with.
    pub sigpipe: Option<usize>,
    /// `setvbuf` applied before any I/O happens on the stream.
    pub setvbuf_first: Option<(c_int, usize)>,
}

impl Default for RunOpts {
    fn default() -> Self {
        RunOpts { dest: Dest::File, final_flush: true, sigpipe: None, setvbuf_first: None }
    }
}

impl RunOpts {
    pub fn dest(dest: Dest) -> Self {
        RunOpts { dest, ..Default::default() }
    }
    pub fn vbuf(mode: c_int, size: usize) -> Self {
        RunOpts { setvbuf_first: Some((mode, size)), ..Default::default() }
    }
    pub fn no_final_flush(mut self) -> Self {
        self.final_flush = false;
        self
    }
    /// `SIGPIPE` ignored, so a broken pipe surfaces as an `EPIPE` error.
    pub fn ignoring_sigpipe(mut self) -> Self {
        self.sigpipe = Some(SIG_IGN);
        self
    }
    /// `SIGPIPE` at its default disposition, exactly like a C program: a broken
    /// pipe then kills the process.
    pub fn default_sigpipe(mut self) -> Self {
        self.sigpipe = Some(SIG_DFL);
        self
    }
}

/// Everything an external observer can see from one scenario run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunResult {
    /// Bytes that reached fd 1 (empty for destinations that discard or reject).
    pub bytes: Vec<u8>,
    /// Value returned by every `Call` step, in order.
    pub rets: Vec<c_int>,
    /// Exit status of the scenario process: `WEXITSTATUS`, or `-signal` when
    /// the process was terminated by a signal.
    pub exit: i32,
}

const WNOHANG: c_int = 1;
const SIGKILL: c_int = 9;

extern "C" {
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn unlink(path: *const c_char) -> c_int;
}

// ---------------------------------------------------------------------------
// Scenario runner
// ---------------------------------------------------------------------------

/// Runs `steps` in a forked child with fd 1 pointing at `opts.dest`, and
/// reports everything an outside observer can see.
///
/// `table[step.lib]` selects the library a `Call` step goes to, so the very same
/// scenario can be replayed against the C `.so`, against the Rust `.so`, or
/// against a mix of both.
pub fn run(steps: &[Step], table: [Fns; 2], opts: &RunOpts) -> RunResult {
    // Keep the volumes small enough that the pipes can never fill up (64 KiB
    // capacity) before the parent drains them after `waitpid`.
    let calls = steps
        .iter()
        .filter(|s| matches!(s, Step::Call { .. } | Step::CallWith { .. }))
        .count();
    assert!(calls <= 512, "scenario has too many calls for the return pipe");

    let _guard = harness_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Make sure symbols are resolved and libraries relocated *before* fork.
    let _ = table[0].hello;
    let _ = table[1].hello;

    // Buffer handed to setvbuf, allocated before the fork.
    let mut vbuf: Vec<u8> = match opts.setvbuf_first {
        Some((_, size)) if size > 0 => vec![0u8; size],
        _ => Vec::new(),
    };
    let vbuf_ptr = if vbuf.is_empty() { std::ptr::null_mut() } else { vbuf.as_mut_ptr() };

    // Also pre-allocate for the SetVbuf steps.
    let mut step_bufs: Vec<Vec<u8>> = steps
        .iter()
        .map(|s| match s {
            Step::SetVbuf { size, .. } if *size > 0 => vec![0u8; *size],
            _ => Vec::new(),
        })
        .collect();
    let step_buf_ptrs: Vec<*mut u8> = step_bufs
        .iter_mut()
        .map(|b| if b.is_empty() { std::ptr::null_mut() } else { b.as_mut_ptr() })
        .collect();

    let file_path = if opts.dest == Dest::File { Some(tmp_path("cap")) } else { None };

    unsafe {
        // Return-value channel.
        let mut retfds = [-1 as c_int; 2];
        assert_eq!(pipe(retfds.as_mut_ptr()), 0, "pipe() for return values failed");

        // Destination for fd 1.
        let (out_fd, cap_read) = setup_dest(opts.dest, file_path.as_ref());

        let pid = fork();
        assert!(pid >= 0, "fork() failed");

        if pid == 0 {
            // ---------------- child ----------------
            // Only libc calls and pre-forked memory from here on.
            close(retfds[0]);
            if cap_read >= 0 {
                close(cap_read);
            }
            if out_fd >= 0 {
                dup2(out_fd, 1);
                if out_fd != 1 {
                    close(out_fd);
                }
            } else {
                close(1);
            }
            if let Some(h) = opts.sigpipe {
                signal(SIGPIPE, h);
            }
            if let Some((mode, size)) = opts.setvbuf_first {
                setvbuf(stdout, vbuf_ptr as *mut c_char, mode, size);
            }

            let mut last: c_int = 0;
            for (i, step) in steps.iter().enumerate() {
                match step {
                    Step::Call { entry, lib } => {
                        let f = table[*lib as usize];
                        let r = match entry {
                            Entry::Hello => (f.hello)(),
                            Entry::Main => (f.main_)(),
                        };
                        last = r;
                        let raw = r.to_le_bytes();
                        write(retfds[1], raw.as_ptr() as *const c_void, 4);
                    }
                    Step::CallWith { entry, lib, args } => {
                        let f = table[*lib as usize];
                        let target = match entry {
                            Entry::Hello => f.hello,
                            Entry::Main => f.main_,
                        };
                        let r = call_with(target, args);
                        last = r;
                        let raw = r.to_le_bytes();
                        write(retfds[1], raw.as_ptr() as *const c_void, 4);
                    }
                    Step::Marker(bytes) => {
                        write(1, bytes.as_ptr() as *const c_void, bytes.len());
                    }
                    Step::CallerPrint(nul_terminated) => {
                        printf(b"%s\0".as_ptr() as *const c_char, nul_terminated.as_ptr());
                    }
                    Step::Flush => {
                        fflush(stdout);
                    }
                    Step::FlushAll => {
                        fflush(std::ptr::null_mut());
                    }
                    Step::SetVbuf { mode, size } => {
                        setvbuf(stdout, step_buf_ptrs[i] as *mut c_char, *mode, *size);
                    }
                }
            }
            if opts.final_flush {
                fflush(std::ptr::null_mut());
            }
            _exit(last & 0xff);
        }

        // ---------------- parent ----------------
        close(retfds[1]);
        if out_fd >= 0 {
            close(out_fd);
        }

        let exit = wait_child(pid, "scenario");

        let rets_raw = read_all(retfds[0]);
        close(retfds[0]);
        assert_eq!(rets_raw.len() % 4, 0, "truncated return-value stream");
        let rets: Vec<c_int> = rets_raw
            .chunks(4)
            .map(|c| c_int::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let bytes = collect_bytes(opts.dest, file_path.as_ref(), cap_read);

        drop(vbuf);
        drop(step_bufs);
        RunResult { bytes, rets, exit }
    }
}

/// Calls `f` through the signature `args` describes.
///
/// Transmuting a function pointer to another signature is exactly what a C
/// caller of an unprototyped declarator does; on the SysV AMD64 ABI the callee
/// simply never reads the extra registers/stack slots.
unsafe fn call_with(f: CFn, args: &ArgShape) -> c_int {
    use std::mem::transmute as t;
    match args {
        ArgShape::None => f(),
        ArgShape::Int(x) => t::<CFn, unsafe extern "C" fn(c_int) -> c_int>(f)(*x),
        ArgShape::TwoInts(a, b) => {
            t::<CFn, unsafe extern "C" fn(c_int, c_int) -> c_int>(f)(*a, *b)
        }
        ArgShape::IntPtr(a, p) => {
            t::<CFn, unsafe extern "C" fn(c_int, *const c_void) -> c_int>(f)(*a, *p as *const c_void)
        }
        ArgShape::Ptr(p) => {
            t::<CFn, unsafe extern "C" fn(*const c_void) -> c_int>(f)(*p as *const c_void)
        }
        ArgShape::Size(n) => t::<CFn, unsafe extern "C" fn(usize) -> c_int>(f)(*n),
        ArgShape::SixInts(v) => t::<
            CFn,
            unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int,
        >(f)(v[0], v[1], v[2], v[3], v[4], v[5]),
        ArgShape::Mixed { i, p, d, u, f: fl, extra } => t::<
            CFn,
            unsafe extern "C" fn(
                c_int,
                *const c_void,
                f64,
                u64,
                f32,
                i64,
                i64,
                i64,
                i64,
            ) -> c_int,
        >(f)(
            *i, *p as *const c_void, *d, *u, *fl, extra[0], extra[1], extra[2], extra[3]
        ),
        ArgShape::RetLong => {
            let g = t::<CFn, unsafe extern "C" fn() -> i64>(f);
            // Keep only the C `int` half of the return register.
            g() as u32 as c_int
        }
    }
}

/// Sets up the fd that will become the child's fd 1.
///
/// Returns `(out_fd, cap_read)`; `out_fd == -1` means "fd 1 must be closed",
/// `cap_read == -1` means "nothing to drain".
unsafe fn setup_dest(dest: Dest, file_path: Option<&PathBuf>) -> (c_int, c_int) {
    match dest {
        Dest::File => {
            let p = cpath(file_path.expect("Dest::File needs a path"));
            let fd = open(p.as_ptr() as *const c_char, O_WRONLY | O_CREAT | O_TRUNC, 0o600);
            assert!(fd >= 0, "open temp capture file failed");
            (fd, -1)
        }
        Dest::Pipe => {
            let mut fds = [-1 as c_int; 2];
            assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() for capture failed");
            (fds[1], fds[0])
        }
        Dest::BrokenPipe => {
            let mut fds = [-1 as c_int; 2];
            assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() for broken pipe failed");
            // Drop the only reader *before* forking, so the child's very first
            // write is guaranteed to hit EPIPE (no race).
            close(fds[0]);
            (fds[1], -1)
        }
        Dest::DevNull => {
            let fd = open(b"/dev/null\0".as_ptr() as *const c_char, O_WRONLY, 0);
            assert!(fd >= 0, "open /dev/null failed");
            (fd, -1)
        }
        Dest::DevFull => {
            let fd = open(b"/dev/full\0".as_ptr() as *const c_char, O_WRONLY, 0);
            assert!(fd >= 0, "open /dev/full failed");
            (fd, -1)
        }
        Dest::ReadOnly => {
            // A perfectly valid fd whose open file description is not writable:
            // write(2) fails with EBADF.
            let p = cpath(&manifest_dir().join("c_src/src/sillymain.c"));
            let fd = open(p.as_ptr() as *const c_char, O_RDONLY, 0);
            assert!(fd >= 0, "open sillymain.c read-only failed");
            (fd, -1)
        }
        Dest::Closed => (-1, -1),
    }
}

/// Waits for the scenario child, with a hard timeout so a wedged child can never
/// hang the suite. Returns `WEXITSTATUS`, or `-signal`.
unsafe fn wait_child(pid: c_int, what: &str) -> i32 {
    let mut status: c_int = 0;
    let mut waited = 0u32;
    loop {
        let r = waitpid(pid, &mut status, WNOHANG);
        if r == pid {
            return if wifexited(status) { wexitstatus(status) } else { -wtermsig(status) };
        }
        assert!(r == 0, "waitpid failed");
        if waited > 20_000 {
            kill(pid, SIGKILL);
            waitpid(pid, &mut status, 0);
            panic!("{what} child did not finish within 10s");
        }
        waited += 1;
        std::thread::sleep(std::time::Duration::from_micros(500));
    }
}

/// Collects whatever reached fd 1 (empty for destinations that discard/reject).
unsafe fn collect_bytes(dest: Dest, file_path: Option<&PathBuf>, cap_read: c_int) -> Vec<u8> {
    match dest {
        Dest::File => {
            let p = file_path.expect("Dest::File needs a path");
            let b = std::fs::read(p).expect("read capture file");
            let cp = cpath(p);
            unlink(cp.as_ptr() as *const c_char);
            b
        }
        Dest::Pipe => {
            let b = read_all(cap_read);
            close(cap_read);
            b
        }
        _ => Vec::new(),
    }
}

unsafe fn read_all(fd: c_int) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    out
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn hexdump(b: &[u8]) -> String {
    let shown: Vec<String> = b.iter().take(160).map(|x| format!("{:02x}", x)).collect();
    format!(
        "{} bytes: {}{}\n  as text: {:?}",
        b.len(),
        shown.join(" "),
        if b.len() > 160 { " ..." } else { "" },
        String::from_utf8_lossy(&b[..b.len().min(160)])
    )
}

/// Runs one scenario against the C `.so` and against the Rust `.so` and asserts
/// the two are indistinguishable: same bytes on fd 1, same return value from
/// every call, same process exit status.
pub fn assert_same(row: &str, steps: &[Step], opts: &RunOpts) -> RunResult {
    let c = run(steps, [c_fns(), c_fns()], opts);
    let r = run(steps, [rust_fns(), rust_fns()], opts);
    assert_eq!(
        c.bytes,
        r.bytes,
        "[{row}] bytes written to fd 1 differ\n  C   -> {}\n  Rust-> {}\n  opts: {:?}",
        hexdump(&c.bytes),
        hexdump(&r.bytes),
        opts
    );
    assert_eq!(c.rets, r.rets, "[{row}] return values differ (opts: {:?})", opts);
    assert_eq!(c.exit, r.exit, "[{row}] exit status differs (opts: {:?})", opts);
    c
}

/// `assert_same` plus an explicit expectation for the produced bytes and for
/// every return value, so a test cannot pass because *both* sides are broken.
pub fn assert_same_and_expect(
    row: &str,
    steps: &[Step],
    opts: &RunOpts,
    expected_bytes: &[u8],
    expected_rets: usize,
) {
    let c = assert_same(row, steps, opts);
    assert_eq!(
        c.bytes,
        expected_bytes,
        "[{row}] both libraries produced unexpected bytes\n  got     -> {}\n  expected-> {}",
        hexdump(&c.bytes),
        hexdump(expected_bytes)
    );
    assert_eq!(c.rets.len(), expected_rets, "[{row}] unexpected number of calls recorded");
    assert!(c.rets.iter().all(|&r| r == 0), "[{row}] a call returned non-zero: {:?}", c.rets);
    assert_eq!(c.exit, 0, "[{row}] unexpected exit status");
}

/// `HELLO` repeated `n` times — the expected fd-1 content of `n` successful calls.
pub fn hello_repeated(n: usize) -> Vec<u8> {
    HELLO.repeat(n)
}

// ---------------------------------------------------------------------------
// Fresh library handles (for the dlclose/dlopen row)
// ---------------------------------------------------------------------------

/// Opens a shared object and resolves both entry points, returning a handle the
/// caller can drop to `dlclose` the library again.
pub fn open_lib(path: &Path) -> (Library, Fns) {
    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let fns = Fns {
            hello: *lib.get::<CFn>(b"helloworld\0").expect("dlsym helloworld"),
            main_: *lib.get::<CFn>(b"main\0").expect("dlsym main"),
        };
        (lib, fns)
    }
}

// ---------------------------------------------------------------------------
// Whole-program comparison (the `driver` executable)
// ---------------------------------------------------------------------------

/// The C `driver` program. `c_src/CMakeLists.txt` builds it from the same two
/// translation units; it is rebuilt here out-of-tree with the same default
/// flags so nothing in `c_src/` is touched.
/// `DRIVER_C_EXE` overrides the choice.
pub fn c_exe_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_EXE") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_C_EXE={} is not a file", p.display());
        return p;
    }
    let dir = manifest_dir().join("target/c_build");
    let exe = dir.join("driver_c");
    let sources = [
        manifest_dir().join("c_src/src/sillymain.c"),
        manifest_dir().join("c_src/src/main.c"),
    ];
    if !exe.is_file() {
        std::fs::create_dir_all(&dir).expect("create target/c_build");
        let out = std::process::Command::new("gcc")
            .arg("-o")
            .arg(&exe)
            .args(&sources)
            .output()
            .expect("run gcc to build the C driver executable");
        assert!(
            out.status.success(),
            "building the C driver executable failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    exe
}

/// The Rust `driver` binary (`src/main.rs`), which `cargo test` does build.
///
/// `DRIVER_RUST_EXE` overrides the choice.
pub fn rust_exe_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_EXE") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_EXE={} is not a file", p.display());
        return p;
    }
    let exe = std::env::current_exe().expect("current_exe");
    for dir in exe.ancestors().take(4) {
        let cand = dir.join("driver");
        if cand.is_file() {
            return cand;
        }
    }
    for profile in ["debug", "release"] {
        let cand = manifest_dir().join("target").join(profile).join("driver");
        if cand.is_file() {
            return cand;
        }
    }
    // Same fallback rationale as `build_rust_lib_with_rustc`.
    let dir = manifest_dir().join("target/c_build");
    std::fs::create_dir_all(&dir).expect("create target/c_build");
    let out_exe = dir.join("driver_rs");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let out = std::process::Command::new(rustc)
        .args(["--edition", "2021", "--crate-type", "bin", "--crate-name", "driver"])
        .arg(manifest_dir().join("src/main.rs"))
        .arg("-o")
        .arg(&out_exe)
        .output()
        .expect("run rustc to build the Rust driver binary");
    assert!(
        out.status.success(),
        "building the Rust driver binary failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out_exe
}

// ---------------------------------------------------------------------------
// Concurrency scenario
// ---------------------------------------------------------------------------

/// Calls `fns.hello` from `threads` threads, `per_thread` times each, inside a
/// forked child configured by `opts`.
pub fn run_threaded(fns: Fns, threads: usize, per_thread: usize, opts: &RunOpts) -> RunResult {
    let _guard = harness_lock().lock().unwrap_or_else(|e| e.into_inner());
    let file_path = if opts.dest == Dest::File { Some(tmp_path("thr")) } else { None };
    unsafe {
        let (out_fd, cap_read) = setup_dest(opts.dest, file_path.as_ref());
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            if cap_read >= 0 {
                close(cap_read);
            }
            if out_fd >= 0 {
                dup2(out_fd, 1);
                if out_fd != 1 {
                    close(out_fd);
                }
            } else {
                close(1);
            }
            if let Some(h) = opts.sigpipe {
                signal(SIGPIPE, h);
            }
            if let Some((mode, size)) = opts.setvbuf_first {
                setvbuf(stdout, std::ptr::null_mut(), mode, size);
            }
            let hello = fns.hello;
            let mut handles = Vec::with_capacity(threads);
            for _ in 0..threads {
                handles.push(std::thread::spawn(move || {
                    let mut nonzero = 0;
                    for _ in 0..per_thread {
                        if hello() != 0 {
                            nonzero += 1;
                        }
                    }
                    nonzero
                }));
            }
            // Exit code reports how many calls returned something other than 0,
            // so a divergence in the return value is visible even here.
            let mut bad = 0i32;
            for h in handles {
                match h.join() {
                    Ok(n) => bad += n,
                    Err(_) => bad += 1000,
                }
            }
            if opts.final_flush {
                fflush(std::ptr::null_mut());
            }
            _exit(bad & 0xff);
        }
        if out_fd >= 0 {
            close(out_fd);
        }
        let exit = wait_child(pid, "threaded scenario");
        let bytes = collect_bytes(opts.dest, file_path.as_ref(), cap_read);
        RunResult { bytes, rets: Vec::new(), exit }
    }
}
