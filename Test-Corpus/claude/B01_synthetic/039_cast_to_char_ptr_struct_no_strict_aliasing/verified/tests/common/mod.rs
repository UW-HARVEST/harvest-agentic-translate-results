//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Both shared libraries are always driven through their **exported C ABI
//! symbols loaded with `libloading`** — never by calling Rust functions
//! directly:
//!
//! * `examples/so_runner.rs` is a tiny `libloading` host used as a subprocess so
//!   that every `main` test case gets pristine stdin/stdout stream state (the C
//!   program calls `main` once per process).
//! * `tests/inprocess.rs` loads both libraries into one process with
//!   `libloading` and calls `driver` through the FFI boundary directly.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// paths
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory containing the built test/bench binaries' siblings, i.e.
/// `target/<profile>/`.
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>  ->  .../target/<profile>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Newest mtime over the Rust sources of the translation.
///
/// `Cargo.toml` is deliberately NOT included: cargo does not rebuild for a
/// comment-only manifest edit, so using it here would report false staleness.
fn newest_source_mtime() -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    for f in ["src/imp.rs", "src/lib.rs", "src/main.rs"] {
        if let Some(t) = mtime(&manifest_dir().join(f)) {
            if t > newest {
                newest = t;
            }
        }
    }
    newest
}

/// Picks the freshest of the candidate artifact locations and refuses to run
/// against a stale one.
///
/// This matters: `cargo build` *uplifts* `deps/libdriver.so` to
/// `target/<profile>/libdriver.so`, but `cargo test` rebuilds it in `deps/`
/// **without** uplifting — so the uplifted copy can silently be an older build,
/// and the whole differential suite would then be testing yesterday's `.so`.
fn freshest(what: &str, candidates: &[PathBuf]) -> PathBuf {
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for c in candidates {
        if let Some(t) = mtime(c) {
            if best.as_ref().map(|(_, bt)| t > *bt).unwrap_or(true) {
                best = Some((c.clone(), t));
            }
        }
    }
    let (path, t) = best.unwrap_or_else(|| {
        panic!("{what} not found in any of {candidates:?} — run `cargo build` first")
    });
    let src = newest_source_mtime();
    assert!(
        t >= src,
        "{what} at {path:?} is STALE (built {t:?}, sources changed {src:?}) — \
         run `cargo build` before the tests"
    );
    path
}

/// Defined dynamic symbols of an ELF shared object, via `nm -D --defined-only`.
pub fn defined_symbols(lib: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(lib)
        .output();
    let mut v: Vec<String> = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    };
    v.sort();
    v.dedup();
    v
}

pub fn rust_lib_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(rust_lib_path_uncached).clone()
}

fn rust_lib_path_uncached() -> PathBuf {
    let dir = target_profile_dir();
    let candidates = [dir.join("libdriver.so"), dir.join("deps/libdriver.so")];
    // `cargo build --all-targets` can also produce a `cfg(test)` build of the
    // lib in `deps/`; only accept a copy that actually exports the C ABI.
    let usable: Vec<PathBuf> = candidates
        .iter()
        .filter(|p| {
            let s = defined_symbols(p);
            s.iter().any(|x| x == "driver") && s.iter().any(|x| x == "main")
        })
        .cloned()
        .collect();
    let pool: &[PathBuf] = if usable.is_empty() { &candidates } else { &usable };
    freshest("Rust cdylib libdriver.so", pool)
}

pub fn runner_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(runner_path_uncached).clone()
}

fn runner_path_uncached() -> PathBuf {
    let p = target_profile_dir().join("examples/so_runner");
    assert!(
        p.is_file(),
        "so_runner example not found at {p:?} — run `cargo build --examples` first"
    );
    if let (Some(t), Some(s)) = (mtime(&p), mtime(&manifest_dir().join("examples/so_runner.rs"))) {
        assert!(t >= s, "so_runner at {p:?} is STALE — run `cargo build --examples`");
    }
    p
}

pub fn rust_exe_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(rust_exe_path_uncached).clone()
}

fn rust_exe_path_uncached() -> PathBuf {
    // Only the uplifted binary: `deps/driver-<hash>` is ambiguous, because
    // `cargo build --all-targets` also puts the lib/bin *libtest harness* there
    // under the very same name pattern.
    freshest("Rust executable", &[target_profile_dir().join("driver")])
}

fn newer(a: &Path, b: &Path) -> bool {
    let ma = std::fs::metadata(a).and_then(|m| m.modified());
    let mb = std::fs::metadata(b).and_then(|m| m.modified());
    match (ma, mb) {
        (Ok(ta), Ok(tb)) => ta > tb,
        _ => true,
    }
}

/// Builds (once per test binary) the C shared library from the *unmodified*
/// `c_src/src/main.c`, using the same compile option `c_src/CMakeLists.txt`
/// applies to the translation unit. Output goes to `cbuild/` — nothing inside
/// `c_src/` is written except the `build/` dir CMake itself uses.
pub fn c_lib_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = manifest_dir();
        let src = root.join("c_src/src/main.c");
        let outdir = root.join("cbuild");
        std::fs::create_dir_all(&outdir).expect("mkdir cbuild");
        let out = outdir.join("libdriver_c.so");
        if !out.is_file() || newer(&src, &out) {
            let st = Command::new("gcc")
                .args(["-shared", "-fPIC", "-fno-strict-aliasing", "-o"])
                .arg(&out)
                .arg(&src)
                .status()
                .expect("run gcc");
            assert!(st.success(), "gcc failed building the C shared library");
        }
        out
    })
    .clone()
}

/// The C executable. Prefers the CMake build product
/// (`c_src/build/driver`, produced by `add_executable`); otherwise compiles an
/// equivalent one into `cbuild/` with the same flags.
pub fn c_exe_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = manifest_dir();
        let cmake_built = root.join("c_src/build/driver");
        let src = root.join("c_src/src/main.c");
        if cmake_built.is_file() && !newer(&src, &cmake_built) {
            return cmake_built;
        }
        let outdir = root.join("cbuild");
        std::fs::create_dir_all(&outdir).expect("mkdir cbuild");
        let out = outdir.join("driver_c");
        if !out.is_file() || newer(&src, &out) {
            let st = Command::new("gcc")
                .args(["-fno-strict-aliasing", "-o"])
                .arg(&out)
                .arg(&src)
                .status()
                .expect("run gcc");
            assert!(st.success(), "gcc failed building the C executable");
        }
        out
    })
    .clone()
}

pub fn tmp_dir() -> PathBuf {
    let d = manifest_dir().join("cbuild/tmp");
    std::fs::create_dir_all(&d).expect("mkdir cbuild/tmp");
    d
}

// ---------------------------------------------------------------------------
// process running
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Out {
    pub stdout: Vec<u8>,
    pub status: Option<i32>,
    /// terminating signal, if the process was killed (e.g. `SIGPIPE` = 13)
    pub signal: Option<i32>,
    pub stderr: Vec<u8>,
}

impl Out {
    pub fn show(&self) -> String {
        format!(
            "status={:?} signal={:?} stdout={:?} stderr={:?}",
            self.status,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// How the child's stdin is provided (axis A5 of `CONFIGS.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdinKind {
    /// anonymous pipe, closed after the payload is written
    Pipe,
    /// regular file containing the payload
    File,
    /// `/dev/null` (character device, immediate EOF)
    Null,
    /// a write-only descriptor: every `read(0)` fails with `EBADF`
    WriteOnlyFd,
    /// a directory descriptor: every `read(0)` fails with `EISDIR`
    Directory,
    /// fd 0 closed before `exec`
    Closed,
}

/// How the child's stdout is provided (axis A6 of `CONFIGS.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdoutKind {
    Pipe,
    File,
    /// `/dev/full`: every `write` fails with `ENOSPC` (the C ignores
    /// `printf`'s return value)
    DevFull,
    /// a pipe whose read end is already closed: the first `write` raises
    /// `SIGPIPE`
    ClosedPipe,
}

extern "C" {
    fn close(fd: i32) -> i32;
    fn pipe(fds: *mut i32) -> i32;
}

fn unique(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    tmp_dir().join(format!(
        "{tag}-{}-{}-{n}",
        std::process::id(),
        std::thread::current().id().as_u64_hack()
    ))
}

trait IdHack {
    fn as_u64_hack(&self) -> u64;
}
impl IdHack for std::thread::ThreadId {
    fn as_u64_hack(&self) -> u64 {
        // ThreadId has no stable accessor; its Debug form is stable enough for
        // building unique temp file names.
        let s = format!("{self:?}");
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse().unwrap_or(0)
    }
}

pub fn run(
    program: &Path,
    args: &[&str],
    input: &[u8],
    sk: StdinKind,
    ok: StdoutKind,
) -> Out {
    run_env(program, args, &[], input, sk, ok)
}

/// Like [`run`], plus extra environment variables (axis: the C program never
/// calls `setlocale`, so nothing in the environment may change its behaviour).
pub fn run_env(
    program: &Path,
    args: &[&str],
    env: &[(&str, &str)],
    input: &[u8],
    sk: StdinKind,
    ok: StdoutKind,
) -> Out {
    let mut cmd = Command::new(program);
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.stderr(Stdio::piped());

    // ---- stdin ----
    let mut stdin_file_path = None;
    match sk {
        StdinKind::Pipe => {
            cmd.stdin(Stdio::piped());
        }
        StdinKind::File => {
            let p = unique("stdin");
            std::fs::write(&p, input).expect("write stdin file");
            let f = std::fs::File::open(&p).expect("open stdin file");
            cmd.stdin(Stdio::from(f));
            stdin_file_path = Some(p);
        }
        StdinKind::Null => {
            cmd.stdin(Stdio::null());
        }
        StdinKind::WriteOnlyFd => {
            let p = unique("wonly");
            let f = std::fs::File::create(&p).expect("create write-only file");
            cmd.stdin(Stdio::from(f));
            stdin_file_path = Some(p);
        }
        StdinKind::Directory => {
            let f = std::fs::File::open(tmp_dir()).expect("open directory");
            cmd.stdin(Stdio::from(f));
        }
        StdinKind::Closed => {
            cmd.stdin(Stdio::null());
            // SAFETY: only calls the async-signal-safe `close(2)` in the child
            // between fork and exec.
            unsafe {
                use std::os::unix::process::CommandExt;
                cmd.pre_exec(|| {
                    close(0);
                    Ok(())
                });
            }
        }
    }

    // ---- stdout ----
    let mut stdout_file_path = None;
    match ok {
        StdoutKind::Pipe => {
            cmd.stdout(Stdio::piped());
        }
        StdoutKind::File => {
            let p = unique("stdout");
            let f = std::fs::File::create(&p).expect("create stdout file");
            cmd.stdout(Stdio::from(f));
            stdout_file_path = Some(p);
        }
        StdoutKind::DevFull => {
            let f = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .expect("open /dev/full");
            cmd.stdout(Stdio::from(f));
        }
        StdoutKind::ClosedPipe => {
            use std::os::fd::FromRawFd;
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
            // close the read end *before* the child runs, so its very first
            // write deterministically raises SIGPIPE
            unsafe { close(fds[0]) };
            // SAFETY: `fds[1]` is a fresh, owned descriptor.
            let wr = unsafe { std::os::fd::OwnedFd::from_raw_fd(fds[1]) };
            cmd.stdout(Stdio::from(wr));
        }
    }

    // Spawning thousands of short-lived processes can transiently hit EAGAIN /
    // EMFILE on a loaded machine; retry so that a resource hiccup can never be
    // mistaken for a behavioural divergence.
    let mut child = {
        let mut attempt = 0;
        loop {
            match cmd.spawn() {
                Ok(c) => break c,
                Err(e) if attempt < 20 => {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_millis(25 * attempt as u64));
                    if attempt == 20 {
                        panic!("spawn {program:?} failed after {attempt} retries: {e}");
                    }
                }
                Err(e) => panic!("spawn {program:?}: {e}"),
            }
        }
    };

    // Feed the pipe from a helper thread so a child that never drains stdin
    // cannot deadlock us.
    let writer = if sk == StdinKind::Pipe {
        let mut si = child.stdin.take().expect("piped stdin");
        let data = input.to_vec();
        Some(std::thread::spawn(move || {
            let _ = si.write_all(&data);
            let _ = si.flush();
            // dropping `si` closes the pipe -> EOF for the child
        }))
    } else {
        None
    };

    // Watchdog: a translation stuck in a loop (or waiting for input the C does
    // not wait for) must make the test FAIL, never hang the suite.
    let limit = std::time::Duration::from_secs(60);
    let reader = child.stdout.take().map(|mut so| {
        std::thread::spawn(move || {
            let mut v = Vec::new();
            let _ = so.read_to_end(&mut v);
            v
        })
    });
    let err_reader = child.stderr.take().map(|mut se| {
        std::thread::spawn(move || {
            let mut v = Vec::new();
            let _ = se.read_to_end(&mut v);
            v
        })
    });
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "{program:?} {args:?} did not terminate within {limit:?} \
                         (stdin {sk:?} = {:?}, stdout {ok:?}) — hung process",
                        esc(input)
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    };
    let out = std::process::Output {
        status,
        stdout: reader.map(|t| t.join().expect("stdout reader")).unwrap_or_default(),
        stderr: err_reader
            .map(|t| t.join().expect("stderr reader"))
            .unwrap_or_default(),
    };
    if let Some(t) = writer {
        let _ = t.join();
    }

    let stdout = match (&ok, &stdout_file_path) {
        (StdoutKind::File, Some(p)) => {
            let mut v = Vec::new();
            std::fs::File::open(p)
                .expect("reopen stdout file")
                .read_to_end(&mut v)
                .expect("read stdout file");
            let _ = std::fs::remove_file(p);
            v
        }
        _ => out.stdout,
    };
    if let Some(p) = stdin_file_path {
        let _ = std::fs::remove_file(p);
    }

    use std::os::unix::process::ExitStatusExt;
    Out {
        stdout,
        status: out.status.code(),
        signal: out.status.signal(),
        stderr: out.stderr,
    }
}

/// Like [`run`], but the child's stdin pipe is **kept open** for `hold` after the
/// payload is written, and the child must still exit within `limit`.
///
/// This is how the "does the implementation wait for EOF?" property is checked:
/// `scanf("%d")` returns as soon as a non-digit terminates the number, so a
/// translation that slurps stdin to EOF would hang here while the C does not.
pub fn run_holding_stdin(
    program: &Path,
    args: &[&str],
    input: &[u8],
    hold: std::time::Duration,
    limit: std::time::Duration,
) -> (Out, std::time::Duration) {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {program:?}: {e}"));

    let mut si = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    // Detached on purpose: joining it would make the test wait for `hold`, which
    // is exactly the delay this test proves the child does NOT wait for.
    std::thread::spawn(move || {
        let _ = si.write_all(&data);
        let _ = si.flush();
        std::thread::sleep(hold);
        drop(si); // now EOF
    });

    let mut so = child.stdout.take().expect("piped stdout");
    let reader = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = so.read_to_end(&mut v);
        v
    });

    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    panic!(
                        "{program:?} did not exit within {limit:?} while stdin stayed open \
                         (input {:?}) — it is waiting for EOF, which the C does not",
                        esc(input)
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    };
    let elapsed = start.elapsed();
    let stdout = reader.join().expect("reader thread");
    let mut stderr = Vec::new();
    if let Some(mut se) = child.stderr.take() {
        let _ = se.read_to_end(&mut stderr);
    }

    use std::os::unix::process::ExitStatusExt;
    (
        Out {
            stdout,
            status: status.code(),
            signal: status.signal(),
            stderr,
        },
        elapsed,
    )
}

/// Invoke `int main()` of `lib` inside a fresh `libloading` host process.
pub fn call_main(lib: &Path, input: &[u8], sk: StdinKind, ok: StdoutKind) -> Out {
    let runner = runner_path();
    let libs = lib.to_str().unwrap().to_string();
    run(&runner, &[&libs, "main"], input, sk, ok)
}

/// Invoke `void driver(int)` once inside a fresh `libloading` host process.
pub fn call_driver(lib: &Path, value: i32) -> Out {
    let runner = runner_path();
    let libs = lib.to_str().unwrap().to_string();
    let v = format!("{value}");
    run(
        &runner,
        &[&libs, "driver", &v],
        b"",
        StdinKind::Null,
        StdoutKind::Pipe,
    )
}

/// Invoke `void driver(int)` once per value, all in ONE `libloading` host
/// process (axis A7: repeated calls / stream state).
pub fn call_driver_batch(lib: &Path, values: &[i32], ok: StdoutKind) -> Out {
    let runner = runner_path();
    let libs = lib.to_str().unwrap().to_string();
    let mut input = String::new();
    for v in values {
        input.push_str(&format!("{v}\n"));
    }
    run(
        &runner,
        &[&libs, "driver-batch"],
        input.as_bytes(),
        StdinKind::Pipe,
        ok,
    )
}

// ---------------------------------------------------------------------------
// assertions
// ---------------------------------------------------------------------------

pub fn esc(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s
}

/// Independent oracle for the `driver` output: the little-endian image of
/// `house_t { floors, 3, 2.0 }` as lowercase hex plus '\n'. Used only as an
/// extra cross-check; the C library remains the ground truth.
pub fn expected_image(floors: i32) -> Vec<u8> {
    let mut s = String::new();
    for b in floors.to_le_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s.push_str("03000000"); // bedrooms = 3
    s.push_str("0000000000000040"); // bathrooms = 2.0
    s.push('\n');
    s.into_bytes()
}

pub fn assert_main_case(input: &[u8], sk: StdinKind, ok: StdoutKind, label: &str) {
    let c = call_main(&c_lib_path(), input, sk, ok);
    let r = call_main(&rust_lib_path(), input, sk, ok);
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs for stdin {:?} ({sk:?}/{ok:?})\n  C   : {}\n  Rust: {}",
        esc(input),
        c.show(),
        r.show()
    );
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "[{label}] exit status differs for stdin {:?} ({sk:?}/{ok:?})\n  C   : {}\n  Rust: {}",
        esc(input),
        c.show(),
        r.show()
    );
    if matches!(ok, StdoutKind::Pipe | StdoutKind::File | StdoutKind::DevFull) {
        assert_eq!(
            c.status,
            Some(0),
            "[{label}] the C main must always return 0 (stdin {:?})",
            esc(input)
        );
    }
}

/// Differential check of the "must not wait for EOF" property (both libraries,
/// stdin held open).
pub fn assert_main_case_holding_stdin(
    input: &[u8],
    hold: std::time::Duration,
    limit: std::time::Duration,
    label: &str,
) {
    let runner = runner_path();
    let cl = c_lib_path();
    let rl = rust_lib_path();
    let cls = cl.to_str().unwrap().to_string();
    let rls = rl.to_str().unwrap().to_string();
    let (c, ct) = run_holding_stdin(&runner, &[&cls, "main"], input, hold, limit);
    let (r, rt) = run_holding_stdin(&runner, &[&rls, "main"], input, hold, limit);
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs for held-open stdin {:?}\n  C   : {}\n  Rust: {}",
        esc(input),
        c.show(),
        r.show()
    );
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "[{label}] exit status differs for held-open stdin {:?}",
        esc(input)
    );
    eprintln!(
        "[{label}] stdin {:?}: C exited after {ct:?}, Rust after {rt:?}",
        esc(input)
    );
}

pub fn assert_exe_case(input: &[u8], sk: StdinKind, ok: StdoutKind, label: &str) {
    let c = run(&c_exe_path(), &[], input, sk, ok);
    let r = run(&rust_exe_path(), &[], input, sk, ok);
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] executable stdout differs for stdin {:?} ({sk:?}/{ok:?})\n  C   : {}\n  Rust: {}",
        esc(input),
        c.show(),
        r.show()
    );
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "[{label}] executable exit status/signal differs for stdin {:?}\n  C   : {}\n  Rust: {}",
        esc(input),
        c.show(),
        r.show()
    );
}

/// Differential check of the executables with extra `argv` entries (the C
/// `int main()` ignores them).
pub fn assert_exe_case_args(input: &[u8], args: &[&str], label: &str) {
    let c = run(
        &c_exe_path(),
        args,
        input,
        StdinKind::Pipe,
        StdoutKind::Pipe,
    );
    let r = run(
        &rust_exe_path(),
        args,
        input,
        StdinKind::Pipe,
        StdoutKind::Pipe,
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] executable stdout differs with args {args:?}\n  C   : {}\n  Rust: {}",
        c.show(),
        r.show()
    );
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "[{label}] executable status differs with args {args:?}"
    );
}

pub fn assert_driver_batch(values: &[i32], ok: StdoutKind, label: &str) {
    let c = call_driver_batch(&c_lib_path(), values, ok);
    let r = call_driver_batch(&rust_lib_path(), values, ok);
    if c.stdout != r.stdout {
        // narrow the report down to the first differing line
        let cl: Vec<&[u8]> = c.stdout.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r.stdout.split(|&b| b == b'\n').collect();
        for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
            if a != b {
                panic!(
                    "[{label}] driver({}) differs: C={:?} Rust={:?}",
                    values.get(i).copied().unwrap_or_default(),
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b)
                );
            }
        }
        panic!(
            "[{label}] driver batch output length differs: C={} Rust={}",
            c.stdout.len(),
            r.stdout.len()
        );
    }
    assert_eq!(c.status, r.status, "[{label}] driver batch exit status");

    // extra cross-check against the independent oracle
    let mut want = Vec::new();
    for v in values {
        want.extend_from_slice(&expected_image(*v));
    }
    assert_eq!(
        c.stdout,
        want,
        "[{label}] the C output disagrees with the struct-image oracle"
    );
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*) for property-style rows
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// uniform in `0..n`
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}
