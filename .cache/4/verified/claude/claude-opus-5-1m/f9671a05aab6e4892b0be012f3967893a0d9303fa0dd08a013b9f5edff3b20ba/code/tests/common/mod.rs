//! Shared differential-test harness.
//!
//! Both implementations are loaded as SHARED OBJECTS through `libloading` and
//! called only through their exported `extern "C"` symbols — the Rust crate is
//! never linked directly, so the `#[unsafe(no_mangle)]` export wrappers are
//! part of what is under test.

#![allow(dead_code)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// ABI of the library under test (see c_src/src/driver.c)
// ---------------------------------------------------------------------------

pub type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
pub type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
pub type DriverFn = unsafe extern "C" fn(*const c_char);

/// One loaded implementation (C or Rust), reached only via `dlsym`.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub fma_array: FmaArrayFn,
    pub call_fma: CallFmaFn,
    pub driver: DriverFn,
}

struct Both {
    c: Impl,
    rust: Impl,
}

static BOTH: OnceLock<Both> = OnceLock::new();

/// `(c_impl, rust_impl)` — both loaded from their `.so` files.
pub fn both() -> (&'static Impl, &'static Impl) {
    let b = BOTH.get_or_init(|| Both {
        c: load(c_so_path(), "C"),
        rust: load(rust_so_path(), "RUST"),
    });
    (&b.c, &b.rust)
}

fn load(path: PathBuf, name: &'static str) -> Impl {
    assert!(
        path.is_file(),
        "{} shared object not found at {}",
        name,
        path.display()
    );
    // Leaked on purpose: the resolved function pointers must stay valid for the
    // whole process lifetime.
    let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
        libloading::Library::new(&path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));

    unsafe {
        let fma_array = *lib
            .get::<FmaArrayFn>(b"fma_array\0")
            .expect("missing export: fma_array");
        let call_fma = *lib
            .get::<CallFmaFn>(b"call_fma\0")
            .expect("missing export: call_fma");
        let driver = *lib
            .get::<DriverFn>(b"driver\0")
            .expect("missing export: driver");
        Impl {
            name,
            path,
            fma_array,
            call_fma,
            driver,
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of the C shared object, built on demand with cmake if absent.
pub fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().join("c_src").join("build");
    let so = build_dir.join("libdriver.so");
    if !so.is_file() {
        build_c_library(&build_dir);
    }
    so
}

fn build_c_library(build_dir: &Path) {
    use std::process::Command;
    std::fs::create_dir_all(build_dir).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(build_dir)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(build_dir)
        .output()
        .expect("run cmake --build");
    assert!(
        bld.status.success(),
        "cmake --build failed:\n{}",
        String::from_utf8_lossy(&bld.stderr)
    );
}

/// Path of the Rust cdylib under test, (re)built from `src/lib.rs` on demand.
///
/// `cargo test` does NOT produce the `cdylib` artifact for this crate — it only
/// builds the library as an rlib to link the test binaries against. Pointing the
/// harness at `target/<profile>/libdriver.so` therefore silently picks up a
/// stale `cargo build` output, so the harness compiles the cdylib itself with
/// `rustc`. The crate's `lib` target has no dependencies, so a bare `rustc`
/// invocation reproduces exactly what cargo would emit.
///
/// Overridable with `DRIVER_RUST_SO` (path) and `DRIVER_RUST_OPT` (opt-level,
/// default `2`) so the suite can be replayed against other builds.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(build_rust_cdylib).clone()
}

/// Whether the cdylib under test was compiled with `cfg(debug_assertions)`.
///
/// This matters for exactly one observable behaviour: with debug assertions on,
/// rustc's MIR null/alignment checks turn a raw-pointer NULL dereference into a
/// controlled panic (which becomes `SIGABRT` across the `extern "C"` boundary)
/// instead of letting the hardware raise `SIGSEGV` the way the C build does.
/// That is Rust's UB-detection instrumentation, not a translation difference,
/// and it cannot be suppressed from source — so the fault rows assert the exact
/// C signal in the shipping configuration (debug assertions off, as produced by
/// `cargo build --release`) and assert "dies abnormally via the Rust check" when
/// they are on.
///
/// Default: OFF, i.e. the shipping configuration. Set
/// `DRIVER_RUST_DEBUG_ASSERTIONS=on` to verify the dev-profile behaviour.
pub fn rust_has_debug_assertions() -> bool {
    matches!(
        std::env::var("DRIVER_RUST_DEBUG_ASSERTIONS")
            .unwrap_or_default()
            .as_str(),
        "1" | "on" | "yes" | "true"
    )
}

/// The termination a NULL dereference inside the Rust `.so` is expected to
/// produce, given the build configuration.
pub fn expected_rust_null_deref_term(c_term: Term) -> Term {
    if rust_has_debug_assertions() {
        // rustc's null-check panics; `extern "C"` turns the unwind into abort.
        Term::Signaled(libc::SIGABRT)
    } else {
        c_term
    }
}

fn build_rust_cdylib() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_SO={} does not exist", p.display());
        return p;
    }
    let opt = std::env::var("DRIVER_RUST_OPT").unwrap_or_else(|_| "2".to_string());
    let da = if rust_has_debug_assertions() { "on" } else { "off" };

    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let out_dir = profile_dir.join(format!("harness-so-opt{opt}-da{da}"));
    std::fs::create_dir_all(&out_dir).expect("create harness-so dir");
    let out = out_dir.join("libdriver.so");

    let src = manifest_dir().join("src").join("lib.rs");
    let newer = |a: &Path, b: &Path| -> bool {
        match (std::fs::metadata(a), std::fs::metadata(b)) {
            (Ok(ma), Ok(mb)) => ma.modified().ok() >= mb.modified().ok(),
            _ => false,
        }
    };
    if out.is_file() && newer(&out, &src) {
        return out;
    }

    let status = std::process::Command::new(
        std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string()),
    )
    .arg("--crate-type=cdylib")
    .arg("--crate-name=driver")
    .arg("--edition=2021")
    .arg(format!("-Copt-level={opt}"))
    // Pinned explicitly: rustc's default for this flag is "on when opt-level is
    // 0, off otherwise", which would silently change the fault behaviour of a
    // NULL dereference between opt levels. See `rust_has_debug_assertions`.
    .arg(format!("-Cdebug-assertions={da}"))
    .arg("-o")
    .arg(&out)
    .arg(&src)
    .output()
    .expect("run rustc");
    assert!(
        status.status.success(),
        "rustc failed to build the cdylib under test:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(out.is_file(), "rustc produced no {}", out.display());
    out
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — property-style testing with a fixed seed
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    /// A per-test stream derived from the global seed, so tests stay
    /// independent yet fully reproducible.
    pub fn for_test(tag: &str) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in tag.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        Rng(SEED ^ h)
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
    /// Uniform over the whole `i32` domain (includes `INT_MIN` / `INT_MAX`).
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Small magnitude value that cannot overflow `a*b + c`.
    pub fn small_i32(&mut self) -> i32 {
        (self.next_u32() % 2001) as i32 - 1000
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range(0, xs.len() - 1)]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Interesting `int` boundary values the arithmetic special-cases.
pub const INT_BOUNDARY: [i32; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    i32::MAX - 1,
    i32::MAX,
];

// ---------------------------------------------------------------------------
// stdout capture (for `driver`, whose only observable output is printf on fd 1)
// ---------------------------------------------------------------------------

static FORK_LOCK: Mutex<()> = Mutex::new(());
static CAP_SEQ: AtomicU64 = AtomicU64::new(0);

extern "C" {
    static stdout: *mut libc::FILE;
}

/// Runs `f` in a forked child whose fd 1 is a pipe, and returns
/// `(termination, bytes written to fd 1)`.
///
/// A child process is used instead of an in-process `dup2` because cargo's test
/// harness writes its own progress text to the real fd 1 from another thread;
/// an in-process redirection would capture that text too and produce bogus
/// mismatches. In a child, fd 1 belongs exclusively to the code under test.
pub fn fork_capture<F: FnOnce()>(f: F) -> (Term, Vec<u8>) {
    fork_capture_opts(f, false)
}

/// Like [`fork_capture`], but first restores the DEFAULT disposition of the
/// fatal memory-fault signals in the child.
///
/// Rust's std installs a `SIGSEGV`/`SIGBUS` handler (on an alternate signal
/// stack) that recognises guard-page hits and rewrites them into
/// `abort()` + "has overflowed its stack". That handler belongs to the *test
/// binary*, not to either `.so` — a plain C consumer of the Rust cdylib has no
/// such handler. Comparing fault behaviour with the handler installed therefore
/// measures the host runtime rather than the libraries: it reports `SIGABRT` for
/// a fault that lands exactly on the guard page and `SIGSEGV` for one that lands
/// further away. Resetting to `SIG_DFL` makes the child behave like a plain C
/// host, so the comparison sees the signal the kernel actually raised.
pub fn fork_capture_raw_signals<F: FnOnce()>(f: F) -> (Term, Vec<u8>) {
    fork_capture_opts(f, true)
}

fn fork_capture_opts<F: FnOnce()>(f: F, raw_signals: bool) -> (Term, Vec<u8>) {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    let _guard = FORK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _seq = CAP_SEQ.fetch_add(1, Ordering::SeqCst);

    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        let (rd, wr) = (fds[0], fds[1]);

        // Make sure nothing is pending in the C stdio buffer that the child
        // would inherit and re-emit.
        libc::fflush(stdout);

        let pid = libc::fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // ---- child ----
            libc::close(rd);
            libc::dup2(wr, 1);
            libc::close(wr);
            if raw_signals {
                libc::signal(libc::SIGSEGV, libc::SIG_DFL);
                libc::signal(libc::SIGBUS, libc::SIG_DFL);
                let mut ss: libc::stack_t = std::mem::zeroed();
                ss.ss_flags = libc::SS_DISABLE;
                libc::sigaltstack(&ss, std::ptr::null_mut());
            }
            f();
            libc::fflush(stdout);
            // `_exit` so none of the parent runtime's at-exit handlers run.
            libc::_exit(0);
        }

        // ---- parent ----
        libc::close(wr);
        let mut file = std::fs::File::from_raw_fd(rd);
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf);
        drop(file);

        let mut status: c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        let term = if libc::WIFEXITED(status) {
            Term::Exited(libc::WEXITSTATUS(status))
        } else if libc::WIFSIGNALED(status) {
            Term::Signaled(libc::WTERMSIG(status))
        } else {
            Term::Unknown
        };
        (term, buf)
    }
}

/// Calls `driver` once per input, all inside a SINGLE forked child, and returns
/// the concatenated stdout bytes. Batching keeps the fork count low while still
/// comparing the exact byte stream `printf` produces.
pub fn driver_batch_raw(imp: &Impl, inputs: &[Vec<u8>]) -> (Term, Vec<u8>) {
    // NUL-terminate up front so the child does no allocation.
    let bufs: Vec<Vec<u8>> = inputs
        .iter()
        .map(|i| {
            let mut b = i.clone();
            b.push(0);
            b
        })
        .collect();
    fork_capture(|| {
        for b in &bufs {
            unsafe { (imp.driver)(b.as_ptr() as *const c_char) };
        }
    })
}

/// Differential batch `driver` run: asserts the C and Rust `.so` emit
/// byte-identical stdout for the whole batch, then returns the per-input output
/// lines (each including its trailing `'\n'`).
///
/// On mismatch the offending input is pinpointed by re-running the batch one
/// input at a time.
pub fn diff_driver_lines(inputs: &[Vec<u8>], ctx: &str) -> Vec<Vec<u8>> {
    let (c, r) = both();
    let (tc, oc) = driver_batch_raw(c, inputs);
    let (tr, or) = driver_batch_raw(r, inputs);
    assert_eq!(tc, tr, "driver batch termination mismatch [{ctx}]");
    assert_eq!(tc, Term::Exited(0), "driver batch died [{ctx}]");

    if oc != or {
        // Pinpoint the first diverging input.
        for (i, inp) in inputs.iter().enumerate() {
            let one = std::slice::from_ref(inp).to_vec();
            let (_, a) = driver_batch_raw(c, &one);
            let (_, b) = driver_batch_raw(r, &one);
            if a != b {
                panic!(
                    "driver stdout mismatch [{ctx}] at input #{i}\n  input = {:?}\n  bytes = {:?}\n  C    = {:?}\n  RUST = {:?}",
                    String::from_utf8_lossy(inp),
                    inp,
                    String::from_utf8_lossy(&a),
                    String::from_utf8_lossy(&b),
                );
            }
        }
        panic!(
            "driver batch stdout mismatch [{ctx}] but no single input diverges\n  C    = {:?}\n  RUST = {:?}",
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or),
        );
    }

    let lines: Vec<Vec<u8>> = oc.split_inclusive(|b| *b == b'\n').map(|s| s.to_vec()).collect();
    assert_eq!(
        lines.len(),
        inputs.len(),
        "[{ctx}] driver must print exactly one line per call; got {} lines for {} inputs: {:?}",
        lines.len(),
        inputs.len(),
        String::from_utf8_lossy(&oc)
    );
    lines
}

/// Differential single `driver` call; returns the output line (with `'\n'`).
pub fn diff_driver(input: &[u8], ctx: &str) -> Vec<u8> {
    let inputs = vec![input.to_vec()];
    diff_driver_lines(&inputs, ctx).pop().unwrap()
}

/// Differential `call_fma` call.
pub fn diff_call_fma(data: &[i32], len: c_int, ctx: &str) -> c_int {
    let (c, r) = both();
    let vc = unsafe { (c.call_fma)(data.as_ptr(), len) };
    let vr = unsafe { (r.call_fma)(data.as_ptr(), len) };
    assert_eq!(
        vc, vr,
        "call_fma mismatch [{ctx}] len={len} data={:?}",
        &data[..data.len().min(16)]
    );
    vc
}

/// Differential `fma_array` call on disjoint buffers: returns the shared `out`.
pub fn diff_fma_array(
    out_init: &[i32],
    mul1: &[i32],
    mul2: &[i32],
    add: &[i32],
    len: c_int,
    ctx: &str,
) -> Vec<i32> {
    let (c, r) = both();
    let mut oc = out_init.to_vec();
    let mut or = out_init.to_vec();
    unsafe {
        (c.fma_array)(oc.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
        (r.fma_array)(or.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
    }
    assert_eq!(oc, or, "fma_array mismatch [{ctx}] len={len}");
    oc
}

// ---------------------------------------------------------------------------
// Crash comparison (for the C code's unchecked dereferences)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Term {
    Exited(i32),
    Signaled(i32),
    Unknown,
}

impl Term {
    pub fn is_signal(self) -> bool {
        matches!(self, Term::Signaled(_))
    }
}

/// Runs `f` in a forked child and reports how the child terminated. Used to
/// compare the *fault* behaviour of the two libraries (the C code performs
/// several unchecked dereferences, so "segfaults identically" is the
/// observable contract).
pub fn term_of<F: FnOnce()>(f: F) -> Term {
    fork_capture(f).0
}

/// [`term_of`] with the host runtime's fault handlers reset to `SIG_DFL`, so the
/// reported signal is the one the kernel raised. Use this for every row that
/// compares faulting behaviour.
pub fn term_of_raw<F: FnOnce()>(f: F) -> Term {
    fork_capture_raw_signals(f).0
}

/// Runs the same risky operation against both implementations and asserts they
/// terminate in the exact same way (same exit code, or same fatal signal).
pub fn assert_same_term<F>(ctx: &str, mut op: F)
where
    F: FnMut(&'static Impl),
{
    let (c, r) = both();
    let tc = term_of_raw(|| op(c));
    let tr = term_of_raw(|| op(r));
    assert_eq!(
        tc, tr,
        "[{ctx}] termination differs: C = {tc:?}, RUST = {tr:?}"
    );
}

/// Like [`assert_same_term`], but for operations whose fault is a NULL
/// dereference *inside the Rust code*. In the shipping configuration this is
/// identical to [`assert_same_term`]; with `debug_assertions` on, rustc's
/// null-check turns the fault into an abort (see
/// [`rust_has_debug_assertions`]), which this accounts for while still requiring
/// that both libraries reject the input fatally.
pub fn assert_same_term_null_deref<F>(ctx: &str, mut op: F)
where
    F: FnMut(&'static Impl),
{
    let (c, r) = both();
    let tc = term_of_raw(|| op(c));
    let tr = term_of_raw(|| op(r));
    assert_eq!(
        tc,
        Term::Signaled(libc::SIGSEGV),
        "[{ctx}] C must SIGSEGV on a NULL dereference, got {tc:?}"
    );
    assert_eq!(
        tr,
        expected_rust_null_deref_term(tc),
        "[{ctx}] RUST termination unexpected (debug_assertions={}): C = {tc:?}, RUST = {tr:?}",
        rust_has_debug_assertions()
    );
    assert!(tr.is_signal(), "[{ctx}] RUST must reject fatally");
}
