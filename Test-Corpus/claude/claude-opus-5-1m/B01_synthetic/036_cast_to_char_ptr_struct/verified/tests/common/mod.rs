//! Shared differential-testing harness.
//!
//! Everything here loads the **C** and the **Rust** implementations as shared
//! libraries and calls them through `dlopen`/`dlsym`, exactly as an external
//! consumer would. No Rust function of the crate under test is ever called
//! directly, so the `#[no_mangle]` export wrappers are part of what is tested.

#![allow(dead_code)]

use std::io::Write;
use std::os::raw::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture (declared directly to avoid pulling in
// the `libc` crate).
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` drains *every* C stdio stream, including the `stdout`
    /// buffer of the loaded C `.so` (it shares this process's libc).
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Paths / building
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/debug` (derived from the running test binary at
/// `target/debug/deps/<name>-<hash>`).
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
    }

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// The profile this test binary was built with, taken from its own path
/// (`target/debug/deps/...` vs `target/release/deps/...`), so the artifacts the
/// harness builds land in the same directory it looks them up in.
fn profile_args() -> &'static [&'static str] {
    let dir = target_profile_dir();
    match dir.file_name().and_then(|n| n.to_str()) {
        Some("release") => &["--release"],
        _ => &[],
    }
}

fn run(cmd: &mut Command) -> String {
    let what = format!("{cmd:?}");
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn {what}: {e}"));
    if !out.status.success() {
        panic!(
            "command failed: {what}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Builds `c_src/src/main.c` as a shared library.
///
/// `c_src/CMakeLists.txt` declares `add_executable(driver src/main.c)`; the
/// same single translation unit is compiled with `-shared -fPIC` so its
/// exported symbols can be compared and called. Nothing inside `c_src/` is
/// modified — all output goes to `target/cbuild/`.
pub fn c_lib() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let out_dir = manifest_dir().join("target/cbuild");
        std::fs::create_dir_all(&out_dir).expect("mkdir target/cbuild");
        let so = out_dir.join("libcdriver.so");
        run(Command::new("gcc").args([
            "-shared",
            "-fPIC",
            "-O2",
            "-o",
            so.to_str().unwrap(),
            manifest_dir().join("c_src/src/main.c").to_str().unwrap(),
        ]));
        assert!(so.is_file(), "C .so not produced: {}", so.display());
        so
    })
    .as_path()
}

/// Builds the C executable exactly the way `c_src/CMakeLists.txt` says to,
/// with the build tree kept outside `c_src/`.
pub fn c_exe() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let build = manifest_dir().join("target/cexe");
        run(Command::new("cmake").args([
            "-S",
            manifest_dir().join("c_src").to_str().unwrap(),
            "-B",
            build.to_str().unwrap(),
        ]));
        run(Command::new("cmake").args(["--build", build.to_str().unwrap()]));
        let exe = build.join("driver");
        assert!(exe.is_file(), "C exe not produced: {}", exe.display());
        exe
    })
    .as_path()
}

/// Builds and returns the Rust `cdylib`.
///
/// `cargo test` does not build the `cdylib` artifact (the lib target has
/// `test = false`), so the harness builds it explicitly. The build lock is
/// already released by the time test binaries run.
pub fn rust_lib() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        run(Command::new(cargo())
            .args(["build", "--offline", "--lib"])
            .args(profile_args())
            .current_dir(manifest_dir()));
        let so = target_profile_dir().join("libdriver.so");
        assert!(so.is_file(), "Rust .so not produced: {}", so.display());
        so
    })
    .as_path()
}

/// Builds and returns the Rust executable (mirrors `add_executable`).
pub fn rust_exe() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        run(Command::new(cargo())
            .args(["build", "--offline", "--bin", "driver"])
            .args(profile_args())
            .current_dir(manifest_dir()));
        let exe = target_profile_dir().join("driver");
        assert!(exe.is_file(), "Rust exe not produced: {}", exe.display());
        exe
    })
    .as_path()
}

/// The `examples/call_symbol.rs` helper, used to invoke the exported `main` in
/// a fresh process (stdin buffering inside each library is not resettable from
/// the outside, so `main` gets one process per input).
pub fn helper() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        run(Command::new(cargo())
            .args(["build", "--offline", "--example", "call_symbol"])
            .args(profile_args())
            .current_dir(manifest_dir()));
        let exe = target_profile_dir().join("examples/call_symbol");
        assert!(exe.is_file(), "helper not produced: {}", exe.display());
        exe
    })
    .as_path()
}

// ---------------------------------------------------------------------------
// Symbol tables
// ---------------------------------------------------------------------------

/// The globally-defined, non-Rust-mangled, non-toolchain symbols of a shared
/// object, as reported by `nm -D --defined-only`, sorted and deduplicated.
///
/// Only `T`/`W` (text) and `D`/`B`/`R` (data) *global* entries are kept, and
/// Rust's own `_ZN…`/`__rust…`/`rust_…` runtime plus the standard ELF and libc
/// scaffolding are filtered out, so the result is the API surface a C consumer
/// can actually link against.
pub fn exported_symbols(so: &Path) -> Vec<String> {
    let out = run(Command::new("nm").args([
        "-D",
        "--defined-only",
        so.to_str().unwrap(),
    ]));
    let mut syms: Vec<String> = out
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (kind, name) = match (it.next(), it.next(), it.next()) {
                // "<addr> <kind> <name>"
                (Some(_), Some(k), Some(n)) => (k, n),
                // "         <kind> <name>" (weak/undefined-address form)
                (Some(k), Some(n), None) => (k, n),
                _ => return None,
            };
            if !matches!(kind, "T" | "W" | "D" | "B" | "R" | "G") {
                return None;
            }
            if is_toolchain_symbol(name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Symbols that come from the compiler/runtime rather than from the translated
/// source, and so are not part of the API surface being compared.
fn is_toolchain_symbol(name: &str) -> bool {
    const PREFIXES: [&str; 12] = [
        "_ZN",         // Rust/C++ mangled
        "_R",          // Rust v0 mangled
        "__rust",      // Rust runtime
        "rust_",       // Rust runtime
        "_ITM_",       // transactional memory stubs
        "__gmon",      // profiling
        "__cxa",       // C++/exit machinery
        "_fini",       //
        "_init",       //
        "__bss_start", //
        "_edata",      //
        "_end",        //
    ];
    PREFIXES.iter().any(|p| name.starts_with(p))
}

// ---------------------------------------------------------------------------
// In-process stdout capture (for `driver`, which takes no input)
// ---------------------------------------------------------------------------

/// Serializes fd-1 redirection within a test binary. Cargo runs different test
/// binaries one at a time, so a per-binary lock is enough.
fn capture_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// everything written to it, flushing C stdio first so that buffered `printf`
/// output is included.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "harvest-capture-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let file_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    unsafe {
        // Flush anything already pending so it is not attributed to this call.
        fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file_fd, 1) >= 0, "dup2 onto stdout failed");

        f();

        // Drain both the C stdio buffer and this process's Rust buffer.
        fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();

        assert!(dup2(saved, 1) >= 0, "restore stdout failed");
        close(saved);
    }

    drop(file);
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Loading the two libraries
// ---------------------------------------------------------------------------

/// A loaded implementation, plus the symbols under test.
pub struct Impl {
    pub name: &'static str,
    _lib: libloading::Library,
    driver: libloading::Symbol<'static, unsafe extern "C" fn(c_int)>,
    /// `driver` seen through a 64-bit parameter type, to probe what the callee
    /// does with the unused upper half of the argument register.
    driver_i64: libloading::Symbol<'static, unsafe extern "C" fn(i64)>,
    main: libloading::Symbol<'static, unsafe extern "C" fn() -> c_int>,
}

impl Impl {
    pub fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            // The symbols borrow from `lib`, which lives as long as this
            // struct; the transmute detaches the lifetime so both can be
            // stored together.
            let driver: libloading::Symbol<unsafe extern "C" fn(c_int)> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display()));
            let driver_i64: libloading::Symbol<unsafe extern "C" fn(i64)> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display()));
            let main: libloading::Symbol<unsafe extern "C" fn() -> c_int> = lib
                .get(b"main\0")
                .unwrap_or_else(|e| panic!("dlsym main in {}: {e}", path.display()));
            Impl {
                name,
                driver: std::mem::transmute(driver),
                driver_i64: std::mem::transmute(driver_i64),
                main: std::mem::transmute(main),
                _lib: lib,
            }
        }
    }

    /// Calls the exported `driver(int)` and returns exactly what it wrote to
    /// file descriptor 1.
    pub fn driver(&self, floors: c_int) -> Vec<u8> {
        capture_stdout(|| unsafe { (self.driver)(floors) })
    }

    /// Calls the exported `driver` through a 64-bit parameter type.
    pub fn driver_wide(&self, wide: i64) -> Vec<u8> {
        capture_stdout(|| unsafe { (self.driver_i64)(wide) })
    }

    /// Does the library export `print_hex`? (It must not: it is `static` in C.)
    pub fn exports_print_hex(&self) -> bool {
        unsafe {
            self._lib
                .get::<unsafe extern "C" fn(*const u8, c_int)>(b"print_hex\0")
                .is_ok()
        }
    }
}

/// The C implementation (ground truth).
pub fn c_impl() -> &'static Impl {
    static I: OnceLock<Impl> = OnceLock::new();
    I.get_or_init(|| Impl::load("C", c_lib()))
}

/// The Rust implementation under test.
pub fn rust_impl() -> &'static Impl {
    static I: OnceLock<Impl> = OnceLock::new();
    I.get_or_init(|| Impl::load("Rust", rust_lib()))
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Calls the exported `driver` once per value in a fresh process that has
/// `dlopen`ed `so`, and returns one output line per value.
///
/// Batching keeps this fast while still guaranteeing that the captured bytes
/// come only from the loaded library.
pub fn driver_batch(so: &Path, values: &[i64], wide: bool) -> Vec<Vec<u8>> {
    let mode = if wide { "driver_wide_batch" } else { "driver_batch" };
    let mut input = String::with_capacity(values.len() * 12);
    for v in values {
        input.push_str(&v.to_string());
        input.push('\n');
    }

    let mut child = Command::new(helper())
        .arg(so)
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn call_symbol helper");
    child
        .stdin
        .take()
        .expect("helper stdin")
        .write_all(input.as_bytes())
        .expect("write helper stdin");
    let out = child.wait_with_output().expect("helper wait");
    assert!(
        out.status.success() && out.stderr.is_empty(),
        "helper {mode} for {} failed: status={:?} stderr={}",
        so.display(),
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    let lines: Vec<Vec<u8>> = out
        .stdout
        .split_inclusive(|&b| b == b'\n')
        .map(|l| l.to_vec())
        .collect();
    assert_eq!(
        lines.len(),
        values.len(),
        "{} produced {} lines for {} values",
        so.display(),
        lines.len(),
        values.len()
    );
    lines
}

/// Phase B/C core check for `driver` over a batch of arguments: both `.so`s
/// must emit identical bytes for every one of them.
///
/// `wide` selects the `fn(i64)`-typed view of the same symbol.
#[track_caller]
pub fn assert_driver_batch_same(values: &[i64], wide: bool, ctx: &str) -> Vec<Vec<u8>> {
    if values.is_empty() {
        return Vec::new();
    }
    let c = driver_batch(c_lib(), values, wide);
    let r = driver_batch(rust_lib(), values, wide);
    for (i, (cl, rl)) in c.iter().zip(r.iter()).enumerate() {
        let v = values[i];
        assert_eq!(
            cl,
            rl,
            "driver({v}) [{:#018x}] diverged ({ctx}, batch index {i}{})\n  C   : {}\n  Rust: {}",
            v as u64,
            if wide { ", 64-bit argument" } else { "" },
            show(cl),
            show(rl)
        );
        // Sanity: the C really produced a 33-byte hex line, i.e. the harness
        // is not comparing two empty buffers.
        assert_eq!(
            cl.len(),
            33,
            "unexpected C output length for driver({v}) ({ctx})"
        );
    }
    c
}

/// Phase B/C core check for a single `driver` argument.
#[track_caller]
pub fn assert_driver_same(floors: c_int, ctx: &str) -> Vec<u8> {
    assert_driver_batch_same(&[floors as i64], false, ctx)
        .into_iter()
        .next()
        .unwrap()
}

/// The result of invoking an exported `main` (or an executable) once.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub status: i32,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ stdout: \"{}\", status: {} }}", show(&self.stdout), self.status)
    }
}

/// Invokes the exported `main` of `so` in a fresh process, feeding `stdin`.
///
/// A new process per call is required because each library buffers stdin
/// internally (glibc `FILE*` / Rust `Stdin`), and that buffer cannot be reset
/// from the outside.
pub fn run_main_via_so(so: &Path, stdin: &[u8]) -> Run {
    let mut child = Command::new(helper())
        .arg(so)
        .arg("main")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn call_symbol helper");
    child
        .stdin
        .take()
        .expect("helper stdin")
        .write_all(stdin)
        .or_else(|e| match e.kind() {
            // The callee may stop reading early; that is not an error.
            std::io::ErrorKind::BrokenPipe => Ok(()),
            _ => Err(e),
        })
        .expect("write helper stdin");
    let out = child.wait_with_output().expect("helper wait");
    assert!(
        out.stderr.is_empty(),
        "helper for {} wrote to stderr: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    Run {
        stdout: out.stdout,
        status: out.status.code().unwrap_or(-1),
    }
}

/// The result of calling an exported `main` several times in one process.
#[derive(PartialEq, Eq)]
pub struct RepeatedRun {
    /// Concatenation of everything the library printed across all calls.
    pub stdout: Vec<u8>,
    /// The value each call returned, in order (reported on stderr).
    pub returns: Vec<i32>,
}

impl std::fmt::Debug for RepeatedRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ stdout: \"{}\", returns: {:?} }}",
            show(&self.stdout),
            self.returns
        )
    }
}

/// Calls the exported `main` of `so` `n` times in one process, all reading the
/// same stdin stream.
pub fn run_main_n_via_so(so: &Path, stdin: &[u8], n: usize) -> RepeatedRun {
    let mut child = Command::new(helper())
        .arg(so)
        .arg("main_n")
        .arg(n.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn call_symbol helper");
    let mut sink = child.stdin.take().expect("helper stdin");
    let write_result = sink.write_all(stdin);
    drop(sink);
    match write_result {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(e) => panic!("write helper stdin: {e}"),
    }
    let out = child.wait_with_output().expect("helper wait");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let returns = stderr
        .split_whitespace()
        .map(|t| {
            t.parse::<i32>()
                .unwrap_or_else(|e| panic!("helper stderr {stderr:?} is not return codes: {e}"))
        })
        .collect::<Vec<i32>>();
    assert_eq!(
        returns.len(),
        n,
        "{} reported {} return values for {n} calls (stderr: {stderr:?})",
        so.display(),
        returns.len()
    );
    RepeatedRun {
        stdout: out.stdout,
        returns,
    }
}

/// Calls the exported `main` of `so` `count` times with a **growing file** as
/// stdin: `text` is appended to it after every call, so the stream hits
/// end-of-file and then has more data available.
pub fn run_main_growing_via_so(so: &Path, initial: &[u8], text: &str, count: usize) -> RepeatedRun {
    let path = std::env::temp_dir().join(format!(
        "harvest-growing-{}-{}.in",
        std::process::id(),
        so.file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&path, initial).expect("write the initial stdin file");
    let stdin = std::fs::File::open(&path).expect("open the stdin file");

    let child = Command::new(helper())
        .arg(so)
        .arg("main_growing")
        .arg(count.to_string())
        .arg(&path)
        .arg(text)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn call_symbol helper");
    let out = child.wait_with_output().expect("helper wait");
    let _ = std::fs::remove_file(&path);

    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let returns = stderr
        .split_whitespace()
        .map(|t| {
            t.parse::<i32>()
                .unwrap_or_else(|e| panic!("helper stderr {stderr:?} is not return codes: {e}"))
        })
        .collect::<Vec<i32>>();
    assert_eq!(
        returns.len(),
        count,
        "{} reported {} return values for {count} calls (stderr: {stderr:?})",
        so.display(),
        returns.len()
    );
    RepeatedRun {
        stdout: out.stdout,
        returns,
    }
}

/// Phase C check for a stdin stream that grows after reaching end-of-file.
#[track_caller]
pub fn assert_main_growing_same(
    initial: &[u8],
    text: &str,
    count: usize,
    ctx: &str,
) -> RepeatedRun {
    let c = run_main_growing_via_so(c_lib(), initial, text, count);
    let r = run_main_growing_via_so(rust_lib(), initial, text, count);
    assert_eq!(
        c,
        r,
        "main() x{count} on a growing stdin diverged (initial {:?}, appending {text:?}) ({ctx})\n  \
         C   : {c:?}\n  Rust: {r:?}",
        show(initial)
    );
    c
}

/// Phase B/C check for repeated `main` calls: identical output *and* identical
/// per-call return values.
#[track_caller]
pub fn assert_main_n_same(stdin: &[u8], n: usize, ctx: &str) -> RepeatedRun {
    let c = run_main_n_via_so(c_lib(), stdin, n);
    let r = run_main_n_via_so(rust_lib(), stdin, n);
    assert_eq!(
        c,
        r,
        "main() x{n} diverged for stdin {:?} ({ctx})\n  C   : {c:?}\n  Rust: {r:?}",
        show(stdin)
    );
    assert_eq!(
        c.stdout.len(),
        33 * n,
        "unexpected C output length for stdin {:?} ({ctx})",
        show(stdin)
    );
    c
}

/// stdout, stderr and exit status of one helper invocation.
#[derive(PartialEq, Eq)]
pub struct ModeRun {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: i32,
}

impl std::fmt::Debug for ModeRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ stdout: \"{}\", stderr: \"{}\", status: {} }}",
            show(&self.stdout),
            show(&self.stderr),
            self.status
        )
    }
}

/// Runs the helper in an arbitrary mode with the given stdin, capturing stdout,
/// stderr and the exit status. Used by the "host" modes, where the helper acts
/// as a C program embedding the library and reads/writes through libc's own
/// `stdin`/`stdout`.
pub fn run_main_via_so_mode(so: &Path, stdin: &[u8], mode: &str) -> ModeRun {
    let mut child = Command::new(helper())
        .arg(so)
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn call_symbol helper");
    let mut sink = child.stdin.take().expect("helper stdin");
    match sink.write_all(stdin) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(e) => panic!("write helper stdin: {e}"),
    }
    drop(sink);
    let out = child.wait_with_output().expect("helper wait");
    ModeRun {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code().unwrap_or(-1),
    }
}

/// Invokes the exported `main` of `so`, delivering stdin in separate chunks
/// with a pause between them, so that the conversion has to span several
/// `read` calls.
pub fn run_main_chunked_via_so(so: &Path, chunks: &[&[u8]], pause_ms: u64) -> Run {
    let mut child = Command::new(helper())
        .arg(so)
        .arg("main")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn call_symbol helper");
    let mut sink = child.stdin.take().expect("helper stdin");
    for chunk in chunks {
        match sink.write_all(chunk).and_then(|()| sink.flush()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => break,
            Err(e) => panic!("write helper stdin: {e}"),
        }
        std::thread::sleep(std::time::Duration::from_millis(pause_ms));
    }
    drop(sink);
    let out = child.wait_with_output().expect("helper wait");
    assert!(
        out.stderr.is_empty(),
        "helper for {} wrote to stderr: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    Run {
        stdout: out.stdout,
        status: out.status.code().unwrap_or(-1),
    }
}

/// Phase B check for stdin delivered in chunks.
#[track_caller]
pub fn assert_main_chunked_same(chunks: &[&[u8]], pause_ms: u64, ctx: &str) -> Run {
    let c = run_main_chunked_via_so(c_lib(), chunks, pause_ms);
    let r = run_main_chunked_via_so(rust_lib(), chunks, pause_ms);
    assert_eq!(
        c, r,
        "main() diverged for chunked stdin {chunks:?} ({ctx})\n  C   : {c:?}\n  Rust: {r:?}"
    );
    c
}

/// Runs a linked executable with the given stdin.
pub fn run_exe(exe: &Path, stdin: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin)
        .or_else(|e| match e.kind() {
            std::io::ErrorKind::BrokenPipe => Ok(()),
            _ => Err(e),
        })
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    Run {
        stdout: out.stdout,
        status: out.status.code().unwrap_or(-1),
    }
}

/// Phase B/C core check for the exported `main`: identical stdout *and*
/// identical return value.
#[track_caller]
pub fn assert_main_same(stdin: &[u8], ctx: &str) -> Run {
    let c = run_main_via_so(c_lib(), stdin);
    let r = run_main_via_so(rust_lib(), stdin);
    assert_eq!(
        c,
        r,
        "main() diverged for stdin {:?} ({ctx})\n  C   : {c:?}\n  Rust: {r:?}",
        show(stdin)
    );
    assert_eq!(
        c.stdout.len(),
        33,
        "unexpected C output length for stdin {:?}",
        show(stdin)
    );
    c
}

/// Phase B check at the executable level (`add_executable` vs `[[bin]]`).
#[track_caller]
pub fn assert_exe_same(stdin: &[u8], ctx: &str) -> Run {
    let c = run_exe(c_exe(), stdin);
    let r = run_exe(rust_exe(), stdin);
    assert_eq!(
        c,
        r,
        "executables diverged for stdin {:?} ({ctx})\n  C   : {c:?}\n  Rust: {r:?}",
        show(stdin)
    );
    c
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style runs)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    /// `seed` is folded with a constant so different rows get different, but
    /// still reproducible, streams.
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
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

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}
