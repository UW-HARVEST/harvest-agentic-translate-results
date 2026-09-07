// Differential test harness: loads BOTH the C `.so` and the Rust `.so` via
// `libloading` and compares their behaviour through the FFI boundary.
//
// Neither library is ever called as a Rust function directly — every call goes
// through `Library::get` + the exported `helloworld` symbol, exactly as an
// external consumer would, so the `#[no_mangle]`/`extern "C"` export wrapper is
// under test too.
//
// Phase B rows live in the `configs` module (see CONFIGS.md).
// Phase C rows live in the `errors` module (see ERRORS.md).

use std::ffi::{c_char, c_int, c_void};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bindings used by the harness itself (fd plumbing + stdio control).
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn write(fd: c_int, buf: *const u8, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
    fn clearerr(stream: *mut c_void);
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    static stdout: *mut c_void;
}

const O_RDONLY: c_int = 0;
const O_WRONLY: c_int = 1;
const _IOFBF: c_int = 0;
const _IOLBF: c_int = 1;
const _IONBF: c_int = 2;

fn libc_stdout() -> *mut c_void {
    unsafe { stdout }
}

/// The single line `helloworld` is expected to emit, once.
const LINE: &[u8] = b"Hello World!\n";

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELLO_C_SO") {
        return PathBuf::from(p);
    }
    // c_src/ is a sibling of translation/
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .join("c_src/build/libhello.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELLO_RUST_SO") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libhello.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libhello.so")
}

fn assert_exists(p: &Path, what: &str) {
    assert!(
        p.exists(),
        "{what} shared object not found at {}. Build it first \
         (C: cmake --build c_src/build ; Rust: cargo build --release).",
        p.display()
    );
}

/// Freshly `dlopen` the C library.
fn open_c() -> Library {
    let p = c_so_path();
    assert_exists(&p, "C");
    unsafe { Library::new(&p) }.expect("dlopen C libhello.so")
}

/// Freshly `dlopen` the Rust library.
fn open_rust() -> Library {
    let p = rust_so_path();
    assert_exists(&p, "Rust");
    unsafe { Library::new(&p) }.expect("dlopen Rust libhello.so")
}

/// Both libraries, kept open for the whole process. `libloading` uses
/// `RTLD_LOCAL`, so the two identically-named `helloworld` symbols do not
/// interpose on each other.
struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| Libs {
        c: open_c(),
        rust: open_rust(),
    })
}

/// `helloworld` under its true signature, as declared in `c_src/include/hello.h`.
type HelloFn = unsafe extern "C" fn() -> c_int;

fn hello_sym(lib: &Library) -> Symbol<'_, HelloFn> {
    unsafe { lib.get(b"helloworld\0") }.expect("helloworld must be exported")
}

/// Raw fn pointer (Copy + Send) for use across threads.
fn hello_ptr(lib: &Library) -> HelloFn {
    *hello_sym(lib)
}

/// Which implementation a test step should call.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Impl {
    C,
    Rust,
}

impl Impl {
    fn lib(self) -> &'static Library {
        match self {
            Impl::C => &libs().c,
            Impl::Rust => &libs().rust,
        }
    }
    fn ptr(self) -> HelloFn {
        hello_ptr(self.lib())
    }
    fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

const BOTH: [Impl; 2] = [Impl::C, Impl::Rust];

// ---------------------------------------------------------------------------
// fd-level stdout capture
//
// Both libraries write into the *process's* libc `stdout` FILE (the C source
// compiles `printf("Hello World!\n")` down to `puts`, and the Rust translation
// deliberately calls libc `printf` for the same reason). So the only faithful
// way to observe them is to capture file descriptor 1 itself, not Rust's
// `io::stdout`.
//
// fd 1 is process-global, so captures must be serialized.
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: Mutex<()> = Mutex::new(());
    &L
}

fn temp_dir() -> PathBuf {
    // TMPDIR is set-but-empty in some environments, which makes
    // `std::env::temp_dir()` yield a relative path; fall back to /tmp.
    match std::env::var_os("TMPDIR") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => PathBuf::from("/tmp"),
    }
}

/// Tags carry `/` and `=` for readability; strip them so they can be used in a
/// filename component.
fn sanitize(tag: &str) -> String {
    tag.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn temp_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    temp_dir().join(format!(
        "hello_diff_{}_{}_{}.out",
        std::process::id(),
        sanitize(tag),
        n
    ))
}

/// Drain every buffer that could hold bytes destined for fd 1, so a capture
/// only ever sees bytes produced inside the closure.
fn drain_all() {
    let _ = io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

/// Redirect fd 1 to a temp file, run `f`, restore fd 1, and return `f`'s value
/// together with the exact bytes that were written to fd 1.
fn capture<R, F: FnOnce() -> R>(tag: &str, f: F) -> (R, Vec<u8>) {
    let guard = capture_lock();
    let _g = guard.lock().unwrap_or_else(|e| e.into_inner());

    drain_all();

    let path = temp_path(tag);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture temp file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    let r = f();

    drain_all();
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore fd 1 failed");
    unsafe {
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).expect("read captured bytes");
    let _ = std::fs::remove_file(&path);
    (r, bytes)
}

/// Run the same closure once against C and once against Rust, capturing fd 1
/// for each, and assert the produced bytes and returned values are identical.
///
/// `body` receives the implementation to drive and returns whatever the test
/// wants compared (typically the vector of return codes).
fn assert_same<T, F>(tag: &str, body: F) -> Vec<u8>
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(Impl) -> T,
{
    let (rc_c, out_c) = capture(tag, || body(Impl::C));
    let (rc_r, out_r) = capture(tag, || body(Impl::Rust));

    assert_eq!(
        rc_c, rc_r,
        "[{tag}] return values diverge: C={rc_c:?} Rust={rc_r:?}"
    );
    assert_eq!(
        out_c,
        out_r,
        "[{tag}] stdout diverges:\n  C    ({} bytes) = {:?}\n  Rust ({} bytes) = {:?}",
        out_c.len(),
        String::from_utf8_lossy(&trunc(&out_c)),
        out_r.len(),
        String::from_utf8_lossy(&trunc(&out_r)),
    );
    out_c
}

fn trunc(b: &[u8]) -> Vec<u8> {
    b.iter().copied().take(160).collect()
}

/// Assert `bytes` is exactly `n` back-to-back copies of `LINE`.
fn assert_n_lines(tag: &str, bytes: &[u8], n: usize) {
    let expected: Vec<u8> = LINE.repeat(n);
    assert_eq!(
        bytes.len(),
        expected.len(),
        "[{tag}] expected {n} lines ({} bytes), got {} bytes: {:?}",
        expected.len(),
        bytes.len(),
        String::from_utf8_lossy(&trunc(bytes))
    );
    assert_eq!(
        bytes,
        expected.as_slice(),
        "[{tag}] byte content mismatch for {n} lines"
    );
}

// ---------------------------------------------------------------------------
// Fixed-seed PRNG (property-style inputs, reproducible)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    /// Seed noted in CONFIGS.md so runs are reproducible.
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish in `lo..=hi`.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(hi >= lo);
        lo + self.next_u64() % (hi - lo + 1)
    }
    fn flip(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

const SEED: u64 = 0x5DEE_CE66_D;

// ---------------------------------------------------------------------------
// Interleaving plans (used by rows C5, C6, C7, C9)
//
// A plan is replayed identically against C and against Rust. Because every
// step's bytes are known, the plan also yields the exact expected output, so
// these rows verify absolute stdio ordering, not merely "C == Rust".
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
enum Step {
    /// Call the library's exported `helloworld`.
    Hello,
    /// Harness-side libc `printf` — same FILE buffer the library writes into.
    LibcPrintf(u32),
    /// Harness-side raw `write(2)` — bypasses the FILE buffer entirely.
    RawWrite(u32),
    /// Harness-side `std::io::stdout()` — Rust's own buffer, not libc's.
    RustWrite(u32),
    /// Explicit `fflush(NULL)`.
    Flush,
}

fn marker(kind: &str, n: u32) -> Vec<u8> {
    format!("{kind}-{n}\n").into_bytes()
}

/// Bytes a step is expected to contribute, in program order.
fn step_bytes(s: Step) -> Vec<u8> {
    match s {
        Step::Hello => LINE.to_vec(),
        Step::LibcPrintf(n) => marker("MARK", n),
        Step::RawWrite(n) => marker("RAW", n),
        Step::RustWrite(n) => marker("RS", n),
        Step::Flush => Vec::new(),
    }
}

fn plan_expected(plan: &[Step]) -> Vec<u8> {
    let mut v = Vec::new();
    for &s in plan {
        v.extend_from_slice(&step_bytes(s));
    }
    v
}

/// Replay a plan against one implementation, returning every `helloworld`
/// return value observed.
///
/// Ordering discipline: steps that write through a *different* buffer than the
/// library's (`RawWrite`, `RustWrite`) are preceded by a flush, so the resulting
/// byte order is exactly program order for both implementations.
fn run_plan(imp: Impl, plan: &[Step]) -> Vec<c_int> {
    let f = imp.ptr();
    let mut rcs = Vec::new();
    for &s in plan {
        match s {
            Step::Hello => rcs.push(unsafe { f() }),
            Step::LibcPrintf(n) => {
                let mut z = marker("MARK", n);
                z.push(0);
                unsafe {
                    printf(b"%s\0".as_ptr() as *const c_char, z.as_ptr() as *const c_char);
                }
            }
            Step::RawWrite(n) => {
                // Flush libc's buffer first so the raw write lands in order.
                unsafe {
                    fflush(std::ptr::null_mut());
                }
                let msg = marker("RAW", n);
                let mut off = 0usize;
                while off < msg.len() {
                    let w = unsafe { write(1, msg[off..].as_ptr(), msg.len() - off) };
                    assert!(w > 0, "raw write to fd 1 failed");
                    off += w as usize;
                }
            }
            Step::RustWrite(n) => {
                unsafe {
                    fflush(std::ptr::null_mut());
                }
                let msg = marker("RS", n);
                let mut so = io::stdout();
                so.write_all(&msg).expect("rust stdout write");
                so.flush().expect("rust stdout flush");
            }
            Step::Flush => unsafe {
                fflush(std::ptr::null_mut());
            },
        }
    }
    rcs
}

/// Replay `plan` against both implementations and assert identical bytes,
/// identical return codes, and that the bytes equal the exact expected stream.
fn assert_plan(tag: &str, plan: &[Step]) {
    let out = assert_same(tag, |imp| run_plan(imp, plan));
    let expected = plan_expected(plan);
    assert_eq!(
        out,
        expected,
        "[{tag}] absolute stdout ordering wrong\n  got      = {:?}\n  expected = {:?}",
        String::from_utf8_lossy(&trunc(&out)),
        String::from_utf8_lossy(&trunc(&expected)),
    );
    let n_hello = plan.iter().filter(|s| matches!(s, Step::Hello)).count();
    for imp in BOTH {
        let rcs = capture(tag, || run_plan(imp, plan)).0;
        assert_eq!(rcs.len(), n_hello, "[{tag}/{}] call count", imp.name());
        assert!(
            rcs.iter().all(|&r| r == 0),
            "[{tag}/{}] non-zero return code: {rcs:?}",
            imp.name()
        );
    }
}

/// Call `helloworld` `n` times, returning every return value.
fn call_n(imp: Impl, n: usize) -> Vec<c_int> {
    let f = imp.ptr();
    (0..n).map(|_| unsafe { f() }).collect()
}

// ===========================================================================
// PHASE B — valid-path differential tests (one test per CONFIGS.md row)
// ===========================================================================

/// C1 — single call, stdout -> regular file. Exact baseline bytes.
#[test]
fn c1_single_call_baseline() {
    let out = assert_same("C1", |imp| call_n(imp, 1));
    assert_n_lines("C1", &out, 1);
    assert_eq!(out, LINE, "C1: expected exactly \"Hello World!\\n\"");

    // Also assert each side individually against the literal, so a *shared*
    // wrong answer cannot slip through the C-vs-Rust comparison.
    for imp in BOTH {
        let (rc, bytes) = capture("C1", || call_n(imp, 1));
        assert_eq!(rc, vec![0], "C1/{}: return value", imp.name());
        assert_eq!(bytes, LINE, "C1/{}: bytes", imp.name());
    }
}

/// C2 — N successive calls; N randomized over 0..=512 (empty / one / many).
#[test]
fn c2_repeated_calls_randomized() {
    let mut rng = Rng::new(SEED);
    let mut ns: Vec<usize> = vec![0, 1, 2, 3];
    for _ in 0..64 {
        ns.push(rng.range(0, 512) as usize);
    }
    for n in ns {
        let tag = format!("C2/n={n}");
        let out = assert_same(&tag, |imp| call_n(imp, n));
        assert_n_lines(&tag, &out, n);
    }
}

/// C3 — call counts straddling the stdio buffer flush boundaries.
#[test]
fn c3_buffer_boundary_counts() {
    let mut rng = Rng::new(SEED ^ 0x3);
    let line = LINE.len(); // 13
    let mut ns: Vec<usize> = Vec::new();
    for boundary in [512usize, 1024, 4096, 8192, 16384] {
        let at = boundary / line; // largest N with 13*N <= boundary
        for d in [-2i64, -1, 0, 1, 2] {
            let cand = at as i64 + d;
            if cand >= 0 {
                ns.push(cand as usize);
            }
        }
        for _ in 0..4 {
            let j = rng.range(0, 8) as i64 - 4;
            let cand = at as i64 + j;
            if cand >= 0 {
                ns.push(cand as usize);
            }
        }
    }
    ns.sort_unstable();
    ns.dedup();
    for n in ns {
        let tag = format!("C3/n={n}");
        let out = assert_same(&tag, |imp| call_n(imp, n));
        assert_n_lines(&tag, &out, n);
    }
}

// ---------------------------------------------------------------------------
// Alternative fd-1 destinations (row C4). glibc chooses a FILE's buffering
// mode from `fstat` on the descriptor, so a pipe, a regular file and a
// character device are three genuinely different stdio paths.
// ---------------------------------------------------------------------------

/// Redirect fd 1 to a pipe, run `f`, and return what the read end yields.
fn capture_pipe<R, F: FnOnce() -> R>(tag: &str, f: F) -> (R, Vec<u8>) {
    let guard = capture_lock();
    let _g = guard.lock().unwrap_or_else(|e| e.into_inner());

    drain_all();

    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "[{tag}] pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "[{tag}] dup(1) failed");
    assert!(unsafe { dup2(wr, 1) } >= 0, "[{tag}] dup2 onto fd 1 failed");

    let r = f();

    drain_all();
    assert!(unsafe { dup2(saved, 1) } >= 0, "[{tag}] restore fd 1 failed");
    unsafe {
        close(saved);
        // Close every writer so the read end sees EOF.
        close(wr);
    }

    let mut out = Vec::new();
    {
        use std::io::Read;
        use std::os::unix::io::FromRawFd;
        let mut file = unsafe { std::fs::File::from_raw_fd(rd) };
        file.read_to_end(&mut out).expect("read pipe");
        // `file` closes `rd` on drop.
    }
    (r, out)
}

/// Redirect fd 1 to `path` opened with `flags`, run `f`, restore. Bytes are not
/// recoverable (that is the point for `/dev/null`), so only `f`'s value is
/// returned.
fn with_fd1_to<R, F: FnOnce() -> R>(tag: &str, path: &str, flags: c_int, f: F) -> R {
    let guard = capture_lock();
    let _g = guard.lock().unwrap_or_else(|e| e.into_inner());

    drain_all();

    let mut z = path.as_bytes().to_vec();
    z.push(0);
    let fd = unsafe { open(z.as_ptr() as *const c_char, flags) };
    assert!(fd >= 0, "[{tag}] open({path}) failed");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "[{tag}] dup(1) failed");
    assert!(unsafe { dup2(fd, 1) } >= 0, "[{tag}] dup2 onto fd 1 failed");

    let r = f();

    unsafe {
        fflush(std::ptr::null_mut());
        // A write error on a bad fd sets the FILE's sticky error flag; clear it
        // so it cannot leak into later rows.
        clearerr(libc_stdout());
        dup2(saved, 1);
        close(saved);
        close(fd);
    }
    r
}

/// C4 — stdout destination: pipe vs regular file vs /dev/null.
#[test]
fn c4_stdout_destination_kinds() {
    let mut rng = Rng::new(SEED ^ 0x4);
    for case in 0..24 {
        let n = rng.range(0, 200) as usize;

        // (a) pipe
        let tag = format!("C4a/pipe/n={n}");
        let (rc_c, out_c) = capture_pipe(&tag, || call_n(Impl::C, n));
        let (rc_r, out_r) = capture_pipe(&tag, || call_n(Impl::Rust, n));
        assert_eq!(rc_c, rc_r, "[{tag}] return values diverge");
        assert_eq!(out_c, out_r, "[{tag}] pipe stdout diverges");
        assert_n_lines(&tag, &out_c, n);

        // (b) regular file (same as the default capture, kept in this row so the
        //     three destinations are compared against each other)
        let tag = format!("C4b/file/n={n}");
        let out_file = assert_same(&tag, |imp| call_n(imp, n));
        assert_n_lines(&tag, &out_file, n);
        assert_eq!(
            out_file, out_c,
            "[{tag}] file and pipe destinations disagree"
        );

        // (c) /dev/null (character device). Bytes are discarded; the observable
        //     is the return codes and the absence of a crash.
        let tag = format!("C4c/devnull/n={n}/case={case}");
        let rc_c = with_fd1_to(&tag, "/dev/null", O_WRONLY, || call_n(Impl::C, n));
        let rc_r = with_fd1_to(&tag, "/dev/null", O_WRONLY, || call_n(Impl::Rust, n));
        assert_eq!(rc_c, rc_r, "[{tag}] return values diverge");
        assert!(rc_c.iter().all(|&r| r == 0), "[{tag}] non-zero return");
        assert_eq!(rc_c.len(), n, "[{tag}] wrong number of calls");
    }
}

/// C5 — interleaved with caller-side libc `printf` (same FILE buffer).
/// This is the row that would catch a translation writing into a buffer other
/// than libc's: the two streams would interleave in the wrong order.
#[test]
fn c5_interleaved_with_libc_printf() {
    let mut rng = Rng::new(SEED ^ 0x5);
    for case in 0..32 {
        let len = rng.range(1, 40) as u32;
        let plan: Vec<Step> = (0..len)
            .map(|i| {
                if rng.flip() {
                    Step::Hello
                } else {
                    Step::LibcPrintf(i)
                }
            })
            .collect();
        assert_plan(&format!("C5/case={case}"), &plan);
    }
}

/// C6 — interleaved with caller-side raw `write(2)` plus explicit flushes.
#[test]
fn c6_interleaved_with_raw_write() {
    let mut rng = Rng::new(SEED ^ 0x6);
    for case in 0..32 {
        let len = rng.range(1, 40) as u32;
        let plan: Vec<Step> = (0..len)
            .map(|i| match rng.range(0, 2) {
                0 => Step::Hello,
                1 => Step::RawWrite(i),
                _ => Step::Flush,
            })
            .collect();
        assert_plan(&format!("C6/case={case}"), &plan);
    }
}

/// C7 — interleaved with Rust's own `std::io::stdout()` buffer.
#[test]
fn c7_interleaved_with_rust_stdout() {
    let mut rng = Rng::new(SEED ^ 0x7);
    for case in 0..32 {
        let len = rng.range(1, 30) as u32;
        let plan: Vec<Step> = (0..len)
            .map(|i| match rng.range(0, 3) {
                0 => Step::Hello,
                1 => Step::RustWrite(i),
                2 => Step::LibcPrintf(i),
                _ => Step::Flush,
            })
            .collect();
        assert_plan(&format!("C7/case={case}"), &plan);
    }
}

/// C8 — stdout buffering mode: unbuffered / line-buffered / fully buffered.
/// `_IONBF` makes every `puts` its own `write(2)`; `_IOLBF` flushes per line;
/// `_IOFBF` batches. Three different stdio write paths.
#[test]
fn c8_buffering_modes() {
    let mut rng = Rng::new(SEED ^ 0x8);
    for (mode, mode_name) in [
        (_IONBF, "IONBF"),
        (_IOLBF, "IOLBF"),
        (_IOFBF, "IOFBF"),
    ] {
        for _ in 0..8 {
            let n = rng.range(0, 120) as usize;
            let tag = format!("C8/{mode_name}/n={n}");
            let run = |imp: Impl| {
                unsafe {
                    fflush(libc_stdout());
                    setvbuf(libc_stdout(), std::ptr::null_mut(), mode, 0);
                }
                let rcs = call_n(imp, n);
                unsafe {
                    fflush(libc_stdout());
                    // Restore the default so the mode cannot leak into other rows.
                    setvbuf(libc_stdout(), std::ptr::null_mut(), _IOFBF, 0);
                }
                rcs
            };
            let out = assert_same(&tag, run);
            assert_n_lines(&tag, &out, n);
        }
    }
}

/// C9 — bytes left sitting in the buffer with no explicit flush inside the
/// operation, versus flushed part-way through. Both must yield the same stream.
#[test]
fn c9_unflushed_vs_flushed() {
    let mut rng = Rng::new(SEED ^ 0x9);
    for case in 0..24 {
        let n = rng.range(1, 300) as usize;

        // (a) no flush anywhere inside; only the capture's final drain flushes.
        let tag = format!("C9a/nofl/n={n}");
        let out_nofl = assert_same(&tag, |imp| call_n(imp, n));
        assert_n_lines(&tag, &out_nofl, n);

        // (b) flush at a randomized point part-way through.
        let cut = rng.range(0, n as u64) as usize;
        let tag = format!("C9b/flush@{cut}/n={n}/case={case}");
        let out_fl = assert_same(&tag, |imp| {
            let mut rcs = call_n(imp, cut);
            unsafe {
                fflush(std::ptr::null_mut());
            }
            rcs.extend(call_n(imp, n - cut));
            rcs
        });
        assert_n_lines(&tag, &out_fl, n);
        assert_eq!(
            out_nofl, out_fl,
            "[{tag}] flushing part-way changed the byte stream"
        );
    }
}

/// C10 — concurrent calls from several threads. glibc locks the FILE, so each
/// line is written atomically; since every line is identical the resulting byte
/// stream is deterministic and can be compared exactly.
#[test]
fn c10_concurrent_threads() {
    let mut rng = Rng::new(SEED ^ 0xA);
    for case in 0..12 {
        let threads = rng.range(2, 8) as usize;
        let per = rng.range(1, 64) as usize;
        let tag = format!("C10/case={case}/t={threads}/k={per}");

        let out = assert_same(&tag, |imp| {
            // A raw fn pointer is Copy + Send, so it can cross into threads
            // while the `Library` stays owned by the main thread.
            let f = imp.ptr();
            let handles: Vec<_> = (0..threads)
                .map(|_| {
                    std::thread::spawn(move || {
                        let mut rcs = Vec::with_capacity(per);
                        for _ in 0..per {
                            rcs.push(unsafe { f() });
                        }
                        rcs
                    })
                })
                .collect();
            let mut all: Vec<c_int> = Vec::new();
            for h in handles {
                all.extend(h.join().expect("worker thread panicked"));
            }
            all.sort_unstable();
            all
        });
        assert_n_lines(&tag, &out, threads * per);
    }
}

/// C11 — both `.so`s loaded at once, calls alternating between them mid-stream.
/// Verifies `RTLD_LOCAL` isolation (no symbol interposition between the two
/// identically-named `helloworld` exports) and that the two are byte-for-byte
/// interchangeable within a single output stream.
#[test]
fn c11_both_libraries_interleaved() {
    let mut rng = Rng::new(SEED ^ 0xB);

    // Prove the two symbols really are distinct code, not one interposing on
    // the other — otherwise this row would pass vacuously.
    let pc = unsafe { libs().c.get::<HelloFn>(b"helloworld\0") }.map(|s| *s as usize);
    let pr = unsafe { libs().rust.get::<HelloFn>(b"helloworld\0") }.map(|s| *s as usize);
    let (pc, pr) = (pc.expect("C symbol"), pr.expect("Rust symbol"));
    assert_ne!(
        pc, pr,
        "C11: both libraries resolved to the SAME address — the two .so files \
         are interposing, so no differential comparison is actually happening"
    );

    for case in 0..24 {
        let len = rng.range(1, 64) as usize;
        let pattern: Vec<Impl> = (0..len)
            .map(|_| if rng.flip() { Impl::C } else { Impl::Rust })
            .collect();
        let tag = format!("C11/case={case}/len={len}");

        let mixed = capture(&tag, || {
            let mut rcs = Vec::new();
            for &imp in &pattern {
                let f = imp.ptr();
                rcs.push(unsafe { f() });
            }
            rcs
        });
        let all_c = capture(&tag, || call_n(Impl::C, len));
        let all_r = capture(&tag, || call_n(Impl::Rust, len));

        assert_eq!(mixed.0, all_c.0, "[{tag}] mixed vs all-C return codes");
        assert_eq!(mixed.0, all_r.0, "[{tag}] mixed vs all-Rust return codes");
        assert_eq!(
            mixed.1, all_c.1,
            "[{tag}] interleaving C and Rust changed the byte stream vs all-C"
        );
        assert_eq!(
            mixed.1, all_r.1,
            "[{tag}] interleaving C and Rust changed the byte stream vs all-Rust"
        );
        assert_n_lines(&tag, &mixed.1, len);
    }
}

/// C12 — every return value, on every call, under a large randomized battery.
/// A return value that depended on call count or on hidden state would show up
/// here rather than only on the first call.
#[test]
fn c12_return_value_under_load() {
    let mut rng = Rng::new(SEED ^ 0xC);
    let mut total = 0usize;
    for case in 0..40 {
        let n = rng.range(1, 400) as usize;
        let tag = format!("C12/case={case}/n={n}");
        let out = assert_same(&tag, |imp| call_n(imp, n));
        assert_n_lines(&tag, &out, n);
        for imp in BOTH {
            let (rcs, _) = capture(&tag, || call_n(imp, n));
            assert_eq!(rcs.len(), n);
            for (i, &rc) in rcs.iter().enumerate() {
                assert_eq!(
                    rc,
                    0,
                    "[{tag}/{}] call #{i} returned {rc}, expected 0",
                    imp.name()
                );
            }
        }
        total += n;
    }
    assert!(total > 1000, "C12 should exercise a meaningful call volume");
}

/// C13 — symbol re-resolved with a fresh `dlsym` before every call, versus
/// resolved once and reused. Probes lazy-PLT-binding differences between the
/// two objects.
#[test]
fn c13_fresh_dlsym_per_call() {
    let mut rng = Rng::new(SEED ^ 0xD);
    for case in 0..16 {
        let n = rng.range(1, 64) as usize;
        let tag = format!("C13/case={case}/n={n}");

        let fresh = assert_same(&tag, |imp| {
            let lib = imp.lib();
            (0..n)
                .map(|_| {
                    let s: Symbol<HelloFn> =
                        unsafe { lib.get(b"helloworld\0") }.expect("re-dlsym helloworld");
                    unsafe { s() }
                })
                .collect::<Vec<_>>()
        });
        assert_n_lines(&tag, &fresh, n);

        let cached = assert_same(&tag, |imp| call_n(imp, n));
        assert_eq!(
            fresh, cached,
            "[{tag}] re-resolving the symbol per call changed the output"
        );
    }
}

// ===========================================================================
// PHASE C — error-path differential tests (one test per ERRORS.md row)
//
// `helloworld` takes no parameters and has exactly one exit path (the
// unconditional `return 0`), so there are no `RETURN_ERROR`-style rows to
// derive. What follows is the generic FFI-boundary surface that applies to a
// zero-argument function: wrong arity, null/out-of-range values passed in the
// argument registers, mis-declared return width, oversized repetition, and a
// broken stdout. Each asserts C and Rust agree on the *same* result, not merely
// that both "did not crash".
//
// `libloading::Library::get` performs no type checking — it hands back the
// symbol's address cast to whatever fn type is requested — so these rows call
// the real exported symbol through deliberately mismatched signatures, which is
// exactly what a C caller using the non-prototype declaration
// `int helloworld();` is permitted to do.
// ===========================================================================

/// E1 — the nominal exit path, asserted exactly: the sole `return 0`.
#[test]
fn e1_return_code_is_zero() {
    for imp in BOTH {
        let (rc, _bytes) = capture("E1", || {
            let f = imp.ptr();
            unsafe { f() }
        });
        assert_eq!(rc, 0, "E1/{}: helloworld must return 0", imp.name());
    }
    // And differentially, so a shared non-zero value could not pass.
    assert_same("E1", |imp| {
        let f = imp.ptr();
        unsafe { f() }
    });
}

/// E2 — called through a wrong-arity pointer with extra integer arguments.
/// `hello.h:27` declares `int helloworld();` — an empty, non-prototype (K&R)
/// parameter list — so C places no arity requirement on callers and extra
/// arguments simply occupy unused SysV argument registers.
#[test]
fn e2_extra_args_non_prototype() {
    type Hello6 = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;
    let mut rng = Rng::new(SEED ^ 0xE2);

    for case in 0..24 {
        let a = rng.next_u64() as u32 as c_int;
        let b = rng.next_u64() as u32 as c_int;
        let c = rng.next_u64() as u32 as c_int;
        let d = rng.next_u64() as u32 as c_int;
        let e = rng.next_u64() as u32 as c_int;
        let g = rng.next_u64() as u32 as c_int;
        let tag = format!("E2/case={case}");

        let out = assert_same(&tag, |imp| {
            let f: Symbol<Hello6> =
                unsafe { imp.lib().get(b"helloworld\0") }.expect("helloworld");
            unsafe { f(a, b, c, d, e, g) }
        });
        // Extra arguments must be ignored: still exactly one line.
        assert_n_lines(&tag, &out, 1);
    }
}

/// E3 — extra *pointer* arguments, including `NULL`. This is the "null pointer"
/// generic boundary for an API with no pointer parameter to nullify: the null
/// must be ignored, never dereferenced.
#[test]
fn e3_extra_null_pointer_args() {
    type HelloPtrs =
        unsafe extern "C" fn(*const c_void, *const c_void, *const c_void, *const c_void) -> c_int;

    let dangling = 1usize as *const c_void; // deliberately unmapped, must be untouched
    let cases: [[*const c_void; 4]; 5] = [
        [std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null()],
        [dangling, std::ptr::null(), dangling, std::ptr::null()],
        [usize::MAX as *const c_void, std::ptr::null(), std::ptr::null(), dangling],
        [std::ptr::null(), usize::MAX as *const c_void, dangling, dangling],
        [dangling, dangling, dangling, dangling],
    ];

    for (i, args) in cases.iter().enumerate() {
        let tag = format!("E3/case={i}");
        let out = assert_same(&tag, |imp| {
            let f: Symbol<HelloPtrs> =
                unsafe { imp.lib().get(b"helloworld\0") }.expect("helloworld");
            unsafe { f(args[0], args[1], args[2], args[3]) }
        });
        assert_n_lines(&tag, &out, 1);
    }
}

/// E4 — out-of-range enum values passed across the FFI boundary. A C `enum`
/// accepts any `int`, so ints with no valid variant are real inputs. This API
/// declares no enum, so the identical correct behaviour is to ignore them.
#[test]
fn e4_out_of_range_enum_values() {
    type Hello3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

    let sentinels: [c_int; 8] = [
        -1,
        0,
        1,
        4242,
        c_int::MIN,
        c_int::MAX,
        0x7FFF_FFFF,
        -0x8000_0000,
    ];

    for (i, &v) in sentinels.iter().enumerate() {
        let tag = format!("E4/value={v}/case={i}");
        let out = assert_same(&tag, |imp| {
            let f: Symbol<Hello3> =
                unsafe { imp.lib().get(b"helloworld\0") }.expect("helloworld");
            unsafe { f(v, v.wrapping_neg(), v ^ 0x5555_5555) }
        });
        assert_n_lines(&tag, &out, 1);
    }
}

/// E5 — return value read through a mis-declared *wider* return type. The C
/// leaves only `eax` defined, so only the low 32 bits are meaningful; both
/// implementations must agree there.
#[test]
fn e5_return_width_low32() {
    type HelloWide = unsafe extern "C" fn() -> i64;

    let mut lows = Vec::new();
    for imp in BOTH {
        let (v, _) = capture("E5", || {
            let f: Symbol<HelloWide> =
                unsafe { imp.lib().get(b"helloworld\0") }.expect("helloworld");
            unsafe { f() }
        });
        lows.push(v as u32);
    }
    assert_eq!(
        lows[0], lows[1],
        "E5: low 32 bits of the return value diverge: C={:#x} Rust={:#x}",
        lows[0], lows[1]
    );
    assert_eq!(lows[0], 0, "E5: low 32 bits must be 0");
}

/// E6 — oversized/repeated invocation: 100 000 successive calls, the
/// "oversized length" analogue for a function with no length parameter. Probes
/// hidden state, counter overflow, or one-shot initialisation.
#[test]
fn e6_oversized_repeated_invocation() {
    const N: usize = 100_000;
    let tag = "E6";

    // Bytes are compared by digest rather than held in memory twice.
    let mut summaries = Vec::new();
    for imp in BOTH {
        let (rcs, bytes) = capture(tag, || call_n(imp, N));
        assert_eq!(rcs.len(), N, "[{tag}/{}] call count", imp.name());
        let bad = rcs.iter().position(|&r| r != 0);
        assert!(
            bad.is_none(),
            "[{tag}/{}] call #{} returned non-zero",
            imp.name(),
            bad.unwrap()
        );
        assert_eq!(
            bytes.len(),
            LINE.len() * N,
            "[{tag}/{}] wrong total byte count",
            imp.name()
        );
        // Confirm the whole stream really is N copies of the line.
        assert!(
            bytes.chunks(LINE.len()).all(|c| c == LINE),
            "[{tag}/{}] stream is not N copies of the line",
            imp.name()
        );
        summaries.push(bytes);
    }
    assert_eq!(
        summaries[0], summaries[1],
        "[{tag}] 100k-call output diverges between C and Rust"
    );
}

/// E7 — stdout closed before the call. `puts` fails and sets the FILE error
/// flag, but the C discards `printf`'s return value and still returns 0. Rust
/// must do the same, and must not panic (`panic = "abort"` in the release
/// profile would turn a panic into a process abort).
#[test]
fn e7_stdout_closed() {
    let mut rcs = Vec::new();
    for imp in BOTH {
        let guard = capture_lock();
        let _g = guard.lock().unwrap_or_else(|e| e.into_inner());

        drain_all();
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "E7: dup(1) failed");
        unsafe {
            // Make stdout unbuffered FIRST, so `puts` issues its `write(2)`
            // immediately — while fd 1 is actually closed. Left block-buffered,
            // the bytes would merely sit in the FILE buffer and get flushed
            // after fd 1 was restored, and the row would test nothing.
            setvbuf(libc_stdout(), std::ptr::null_mut(), _IONBF, 0);
            close(1);
        }

        let f = imp.ptr();
        let rc = unsafe { f() };

        unsafe {
            // Restore before doing anything that might want to write.
            dup2(saved, 1);
            close(saved);
            clearerr(libc_stdout());
            setvbuf(libc_stdout(), std::ptr::null_mut(), _IOFBF, 0);
            clearerr(libc_stdout());
        }
        rcs.push(rc);
    }
    assert_eq!(
        rcs[0], rcs[1],
        "E7: return values diverge with stdout closed: C={} Rust={}",
        rcs[0], rcs[1]
    );
    assert_eq!(rcs[0], 0, "E7: must still return 0 when stdout is closed");
}

/// E8 — stdout redirected to an unwritable sink (`/dev/null` opened read-only,
/// so writes fail with `EBADF`). Same contract as E7: the write error is
/// discarded and 0 is returned.
#[test]
fn e8_stdout_unwritable() {
    let mut rcs = Vec::new();
    for imp in BOTH {
        let rc = with_fd1_to("E8", "/dev/null", O_RDONLY, || {
            let f = imp.ptr();
            unsafe { f() }
        });
        rcs.push(rc);
    }
    assert_eq!(
        rcs[0], rcs[1],
        "E8: return values diverge with an unwritable stdout: C={} Rust={}",
        rcs[0], rcs[1]
    );
    assert_eq!(rcs[0], 0, "E8: must still return 0 when stdout is unwritable");

    // The library must remain usable afterwards, identically on both sides.
    let out = assert_same("E8/after", |imp| call_n(imp, 3));
    assert_n_lines("E8/after", &out, 3);
}

/// E9 — `dlclose` then `dlopen` again and call. Probes destructor/atexit or TLS
/// state that would make a second load misbehave.
#[test]
fn e9_reload_after_dlclose() {
    let mut results = Vec::new();
    for which in BOTH {
        let mut per_impl = Vec::new();
        for round in 0..3 {
            let lib = match which {
                Impl::C => open_c(),
                Impl::Rust => open_rust(),
            };
            let tag = format!("E9/{}/round={round}", which.name());
            let (rc, bytes) = capture(&tag, || {
                let s: Symbol<HelloFn> =
                    unsafe { lib.get(b"helloworld\0") }.expect("helloworld after reload");
                unsafe { s() }
            });
            assert_eq!(rc, 0, "[{tag}] return value after reload");
            assert_eq!(bytes, LINE, "[{tag}] bytes after reload");
            per_impl.push((rc, bytes));
            drop(lib); // dlclose
        }
        results.push(per_impl);
    }
    assert_eq!(
        results[0], results[1],
        "E9: behaviour across dlclose/dlopen rounds diverges"
    );
}

// ===========================================================================
// PHASE D — symbol parity, asserted from inside the test suite as well as from
// the shell, so `cargo test` alone is enough to catch a regression.
// ===========================================================================

/// Every dynamic symbol the C `.so` defines must also be defined by the Rust
/// `.so`, under the exact same name.
#[test]
fn d1_symbol_parity() {
    fn defined_symbols(p: &Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(p)
            .output()
            .expect("run nm -D");
        assert!(
            out.status.success(),
            "nm -D failed on {}: {}",
            p.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = c_so_path();
    let r = rust_so_path();
    assert_exists(&c, "C");
    assert_exists(&r, "Rust");

    let cs = defined_symbols(&c);
    let rs = defined_symbols(&r);

    assert!(
        cs.contains(&"helloworld".to_string()),
        "sanity: the C .so must export helloworld, got {cs:?}"
    );

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   exports: {cs:?}\n\
         Rust exports: {rs:?}",
        missing.len()
    );
}

/// The two `.so` files must be genuinely separate objects — if the harness were
/// accidentally comparing one library against itself, every other test would
/// pass vacuously.
#[test]
fn d2_libraries_are_distinct_objects() {
    let c = c_so_path();
    let r = rust_so_path();
    assert_ne!(
        std::fs::canonicalize(&c).unwrap(),
        std::fs::canonicalize(&r).unwrap(),
        "the C and Rust .so paths resolve to the same file"
    );
    let sc = std::fs::read(&c).unwrap();
    let sr = std::fs::read(&r).unwrap();
    assert_ne!(sc, sr, "the C and Rust .so files have identical contents");
}
