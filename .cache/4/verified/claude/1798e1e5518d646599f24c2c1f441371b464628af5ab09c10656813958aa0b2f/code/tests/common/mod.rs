//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are consumed *only* through their shared objects, loaded
//! at run time with `libloading`. Nothing in `tests/` ever calls a Rust function
//! from this crate directly, so the `#[no_mangle] extern "C"` wrappers in
//! `src/lib.rs` are part of what is under test.

#![allow(dead_code)]

use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// Fixed seed so every randomized row is reproducible.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------- PRNG -----

/// SplitMix64 — tiny, deterministic, no external dependency.
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

    /// Uniform-enough value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn range_usize(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below((hi_inclusive - lo + 1) as u64) as usize
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    pub fn i8(&mut self) -> i8 {
        self.byte() as i8
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// --------------------------------------------------------------- paths -----

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test executable
/// (`target/<profile>/deps/<test>-<hash>`).
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn require(p: PathBuf, what: &str) -> PathBuf {
    assert!(
        p.exists(),
        "missing {what}: {}\n\
         Build the C side with:\n  \
           cd c_src && mkdir -p build && cd build && \
           cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && \
           gcc -shared -fPIC -o libdriver_c.so ../src/main.c\n\
         Build the Rust side with:\n  cargo build",
        p.display()
    );
    p
}

/// The C shared object (`gcc -shared -fPIC` over `c_src/src/main.c`).
pub fn c_so_path() -> PathBuf {
    require(
        manifest_dir().join("c_src/build/libdriver_c.so"),
        "C shared object",
    )
}

/// The Rust `cdylib`.
pub fn rust_so_path() -> PathBuf {
    require(target_profile_dir().join("libdriver.so"), "Rust cdylib")
}

/// The C executable built by `c_src/CMakeLists.txt`.
pub fn c_exe_path() -> PathBuf {
    require(manifest_dir().join("c_src/build/driver"), "C executable")
}

/// The Rust executable.
pub fn rust_exe_path() -> PathBuf {
    require(target_profile_dir().join("driver"), "Rust executable")
}

/// Helper that dlopens a `.so` and calls one exported symbol; used for the
/// stdin-consuming `main` export, which needs a fresh process per input.
pub fn ffi_runner_path() -> PathBuf {
    require(
        target_profile_dir().join("examples/ffi_runner"),
        "ffi_runner example (built by `cargo test`/`cargo build --examples`)",
    )
}

// ------------------------------------------------------------- loading -----

/// A dlopen'd driver library plus typed handles to its five exports.
pub struct Lib {
    _lib: libloading::Library,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub print_hex_char_line: unsafe extern "C" fn(c_char),
    /// Same slot as `print_hex_char_line`, but typed to pass a full-width `int`
    /// so we can push out-of-`char`-range values across the ABI (ERRORS.md E8).
    pub print_hex_char_line_as_int: unsafe extern "C" fn(c_int),
    pub bad: unsafe extern "C" fn(),
    pub good: unsafe extern "C" fn(),
    pub main: unsafe extern "C" fn() -> c_int,
    pub name: &'static str,
}

impl Lib {
    fn open(path: &Path, name: &'static str) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            macro_rules! sym {
                ($t:ty, $n:expr) => {{
                    let s: libloading::Symbol<$t> = lib
                        .get($n)
                        .unwrap_or_else(|e| panic!("dlsym {} in {name}: {e}",
                            String::from_utf8_lossy($n)));
                    *s
                }};
            }
            let print_line = sym!(unsafe extern "C" fn(*const c_char), b"printLine");
            let print_hex_char_line = sym!(unsafe extern "C" fn(c_char), b"printHexCharLine");
            let print_hex_char_line_as_int =
                sym!(unsafe extern "C" fn(c_int), b"printHexCharLine");
            let bad = sym!(unsafe extern "C" fn(), b"bad");
            let good = sym!(unsafe extern "C" fn(), b"good");
            let main = sym!(unsafe extern "C" fn() -> c_int, b"main");
            Lib {
                _lib: lib,
                print_line,
                print_hex_char_line,
                print_hex_char_line_as_int,
                bad,
                good,
                main,
                name,
            }
        }
    }
}

/// The C and Rust libraries, both dlopen'd, ready to be diffed.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

pub fn load_pair() -> Pair {
    assert_serial_test_harness();
    Pair {
        c: Lib::open(&c_so_path(), "C .so"),
        rust: Lib::open(&rust_so_path(), "Rust .so"),
    }
}

// ------------------------------------------------------ stdout capture -----

// fd 1 redirection is process-global; serialize it (tests inside one binary run
// on several threads).
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

/// Redirecting fd 1 affects the whole process, including libtest's own progress
/// output ("test foo ... ok"), which another test thread would otherwise write
/// into our capture window and corrupt the comparison. The harness must run
/// single-threaded; `.cargo/config.toml` sets `RUST_TEST_THREADS=1` so a plain
/// `cargo test` does the right thing, and this check fails loudly (instead of
/// flakily) if that is ever lost.
pub fn assert_serial_test_harness() {
    let via_env = std::env::var("RUST_TEST_THREADS").as_deref() == Ok("1");
    let mut args = std::env::args();
    let mut via_arg = false;
    while let Some(a) = args.next() {
        if a == "--test-threads" {
            via_arg = args.next().as_deref() == Some("1");
        } else if a == "--test-threads=1" {
            via_arg = true;
        }
    }
    assert!(
        via_env || via_arg,
        "these differential tests redirect fd 1 process-wide and must run \
         single-threaded.\nRun them as:\n  \
         cargo test -- --test-threads=1\nor keep RUST_TEST_THREADS=1 \
         (set for you by .cargo/config.toml)."
    );
}

/// Runs `f` with fd 1 pointed at a fresh temp file and returns everything that
/// was written. Flushes both C `FILE*` streams and Rust's `Stdout` afterwards so
/// nothing is left behind in a buffer.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Make sure nothing of ours is pending before we steal fd 1.
    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "driver_capture_{}_{:p}.out",
        std::process::id(),
        &dir as *const _
    ));
    let file = std::fs::File::create(&path).expect("create capture file");

    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(
        unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0,
        "dup2 onto fd 1 failed"
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Flush the callee's buffers *while fd 1 is still the temp file*:
    //  - libc `fflush(NULL)` covers the C `.so`'s `printf`/`puts`.
    //  - the Rust `.so` uses a line-buffered `Stdout`; every write ends in '\n'.
    unsafe { libc::fflush(std::ptr::null_mut()) };

    assert!(unsafe { libc::dup2(saved, 1) } >= 0, "restore fd 1 failed");
    unsafe { libc::close(saved) };
    drop(file);

    let data = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
    data
}

/// Captures `f(&Lib)` for the C library and for the Rust library and asserts the
/// two byte streams are identical.
pub fn assert_same<F>(pair: &Pair, what: &str, f: F)
where
    F: Fn(&Lib),
{
    let c_out = capture_stdout(|| f(&pair.c));
    let rust_out = capture_stdout(|| f(&pair.rust));
    if c_out != rust_out {
        panic!(
            "OUTPUT MISMATCH [{what}]\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
            c_out.len(),
            escape(&c_out),
            rust_out.len(),
            escape(&rust_out)
        );
    }
}

// ----------------------------------------------------------- processes -----

#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Option<i32>,
}

/// Runs a command with `input` on stdin and pipes for stdout/stderr.
pub fn run_with_stdin(cmd: &mut Command, input: &[u8]) -> Run {
    use std::io::Write;
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut si = child.stdin.take().expect("stdin");
        // A short write is fine: the program only reads the first number, so it
        // may exit before draining stdin (EPIPE is expected and ignored).
        let _ = si.write_all(input);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

/// `main` via the `.so`: `ffi_runner <lib.so> main`, with `input` on stdin.
pub fn run_main_via_so(so: &Path, input: &[u8]) -> Run {
    let runner = ffi_runner_path();
    let mut cmd = Command::new(runner);
    cmd.arg(so).arg("main");
    run_with_stdin(&mut cmd, input)
}

/// `main` via the standalone executable, with `input` on stdin.
pub fn run_main_via_exe(exe: &Path, input: &[u8]) -> Run {
    let mut cmd = Command::new(exe);
    run_with_stdin(&mut cmd, input)
}

fn diff_runs(what: &str, how: &str, input: &[u8], c: &Run, r: &Run) {
    if c.stdout != r.stdout || c.stderr != r.stderr || c.status != r.status {
        panic!(
            "MISMATCH [{what}] via {how}\n  stdin ({} bytes): {}\n\
             \x20 C   : status={:?} stdout={} stderr={}\n\
             \x20 Rust: status={:?} stdout={} stderr={}",
            input.len(),
            escape(input),
            c.status,
            escape(&c.stdout),
            escape(&c.stderr),
            r.status,
            escape(&r.stdout),
            escape(&r.stderr),
        );
    }
}

/// Full differential check of one stdin input: through the two `.so`s (dlopen +
/// `main`) *and* through the two standalone executables (CONFIGS.md C28/C30).
pub fn assert_same_stdin(what: &str, input: &[u8]) {
    let c_so = run_main_via_so(&c_so_path(), input);
    let r_so = run_main_via_so(&rust_so_path(), input);
    diff_runs(what, "dlopen(.so) + main", input, &c_so, &r_so);

    let c_exe = run_main_via_exe(&c_exe_path(), input);
    let r_exe = run_main_via_exe(&rust_exe_path(), input);
    diff_runs(what, "standalone executable", input, &c_exe, &r_exe);

    // The two invocation styles must also agree with each other.
    assert_eq!(
        c_so.stdout,
        c_exe.stdout,
        "C .so and C exe disagree for stdin {}",
        escape(input)
    );
    assert_eq!(
        r_so.stdout,
        r_exe.stdout,
        "Rust .so and Rust exe disagree for stdin {}",
        escape(input)
    );
}

/// Cheaper variant that only goes through the two `.so`s.
pub fn assert_same_stdin_so_only(what: &str, input: &[u8]) {
    let c_so = run_main_via_so(&c_so_path(), input);
    let r_so = run_main_via_so(&rust_so_path(), input);
    diff_runs(what, "dlopen(.so) + main", input, &c_so, &r_so);
}

// ------------------------------------------------------------- helpers -----

pub fn escape(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes.iter().take(400) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 400 {
        s.push_str(&format!("\"... (+{} bytes)", bytes.len() - 400));
    } else {
        s.push('"');
    }
    s
}

/// Builds a NUL-terminated buffer for passing `bytes` as a `const char *`.
pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0), "interior NUL");
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}

/// The six bytes `isspace()` accepts in the C locale.
pub const C_WHITESPACE: [u8; 6] = [b'\t', b'\n', 0x0b, 0x0c, b'\r', b' '];
