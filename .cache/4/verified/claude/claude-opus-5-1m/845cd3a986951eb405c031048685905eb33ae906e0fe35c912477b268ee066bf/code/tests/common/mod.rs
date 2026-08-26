//! Shared differential-testing harness.
//!
//! Both the C shared library (built from the unmodified `c_src/src/*.c`) and the
//! Rust `cdylib` are loaded with `libloading`; every call in every test goes
//! through `dlsym`ed symbols, never through Rust code linked into the test.
//!
//! ## Global-state discipline
//!
//! Both libraries keep the tokenizer/analyzer state in process globals, and some
//! of it (`total_*_processed`) can never be reset.  Tests therefore
//!
//! * hold `common::lock()` for their whole body, and
//! * apply **every** operation to *both* libraries in the same order,
//!
//! which keeps the two libraries in lock-step no matter in which order the test
//! harness schedules the tests.  Assertions only ever compare C against Rust,
//! never against an absolute expectation.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

// ---------------------------------------------------------------------------
// C types (mirrors of include/tokenizer.h and include/analyzer.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CToken {
    pub ttype: c_int,
    pub value: [c_char; MAX_TOKEN_LENGTH],
    pub length: usize,
    pub line: c_int,
    pub column: c_int,
}

impl CToken {
    pub fn zeroed() -> CToken {
        CToken {
            ttype: 0,
            value: [0; MAX_TOKEN_LENGTH],
            length: 0,
            line: 0,
            column: 0,
        }
    }

    /// The `token.value` C string.  Everything past the NUL is uninitialised in
    /// the C build and must never be compared.
    pub fn value_bytes(&self) -> Vec<u8> {
        let raw: &[u8] =
            unsafe { std::slice::from_raw_parts(self.value.as_ptr() as *const u8, self.value.len()) };
        match raw.iter().position(|&b| b == 0) {
            Some(i) => raw[..i].to_vec(),
            None => raw.to_vec(),
        }
    }

    pub fn view(&self) -> TokenView {
        TokenView {
            ttype: self.ttype,
            value: self.value_bytes(),
            length: self.length,
            line: self.line,
            column: self.column,
        }
    }
}

/// The observable part of a `token_t`.
#[derive(Clone, PartialEq, Eq)]
pub struct TokenView {
    pub ttype: c_int,
    pub value: Vec<u8>,
    pub length: usize,
    pub line: c_int,
    pub column: c_int,
}

impl std::fmt::Debug for TokenView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Token {{ type: {}, value: {:?}, length: {}, line: {}, column: {} }}",
            self.ttype,
            String::from_utf8_lossy(&self.value),
            self.length,
            self.line,
            self.column
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct COps {
    pub next_token: Option<extern "C" fn() -> CToken>,
    pub peek_token: Option<extern "C" fn() -> CToken>,
    pub reset: Option<extern "C" fn()>,
    pub load_text: Option<extern "C" fn(*const c_char) -> c_int>,
    pub get_stats: Option<extern "C" fn(*mut usize, *mut usize, *mut usize)>,
}

impl COps {
    pub fn null() -> COps {
        COps {
            next_token: None,
            peek_token: None,
            reset: None,
            load_text: None,
            get_stats: None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct CResult {
    pub word_count: usize,
    pub number_count: usize,
    pub keyword_count: usize,
    pub operator_count: usize,
    pub comment_count: usize,
    pub string_count: usize,
    pub line_count: usize,
    pub char_count: usize,
}

// token_type_t
pub const TOKEN_EOF: c_int = 0;
pub const TOKEN_WORD: c_int = 1;
pub const TOKEN_NUMBER: c_int = 2;
pub const TOKEN_PUNCTUATION: c_int = 3;
pub const TOKEN_WHITESPACE: c_int = 4;
pub const TOKEN_NEWLINE: c_int = 5;
pub const TOKEN_IDENTIFIER: c_int = 6;
pub const TOKEN_KEYWORD: c_int = 7;
pub const TOKEN_OPERATOR: c_int = 8;
pub const TOKEN_STRING: c_int = 9;
pub const TOKEN_COMMENT: c_int = 10;
pub const TOKEN_ERROR: c_int = 11;

// ---------------------------------------------------------------------------
// libc bits used by the harness itself
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(p: *mut c_void);
}

pub fn c_free(p: *mut c_char) {
    unsafe { free(p as *mut c_void) }
}

// ---------------------------------------------------------------------------
// The loaded API of one library
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    _lib: libloading::Library,
    pub tokenizer_next_token: extern "C" fn() -> CToken,
    pub tokenizer_peek_token: extern "C" fn() -> CToken,
    pub tokenizer_reset: extern "C" fn(),
    pub tokenizer_load_text: extern "C" fn(*const c_char) -> c_int,
    pub tokenizer_get_stats: extern "C" fn(*mut usize, *mut usize, *mut usize),
    pub get_tokenizer_ops: extern "C" fn() -> COps,
    pub analyzer_init: extern "C" fn(COps),
    pub analyze_text: extern "C" fn(*const c_char) -> CResult,
    pub print_token_distribution: extern "C" fn(),
    pub calculate_complexity_score: extern "C" fn() -> c_int,
    pub find_patterns: extern "C" fn(*const c_char),
    pub print_menu: extern "C" fn(),
    pub print_analysis_result: extern "C" fn(CResult),
    pub interactive_tokenizer: extern "C" fn(COps),
    pub read_file: extern "C" fn(*const c_char) -> *mut c_char,
    /// Only the Rust build has an explicit buffer-drain entry point; the C build
    /// is drained with `fflush(NULL)`.
    pub flush_stdout: Option<extern "C" fn()>,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $t:ty) => {{
        let s: libloading::Symbol<$t> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        *s
    }};
}

impl Api {
    pub fn load(name: &'static str, path: &Path) -> Api {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e));
        let flush_stdout: Option<extern "C" fn()> = unsafe {
            lib.get::<extern "C" fn()>(b"text_analyzer_flush_stdout\0")
                .ok()
                .map(|s| *s)
        };
        Api {
            name,
            tokenizer_next_token: sym!(lib, "tokenizer_next_token", extern "C" fn() -> CToken),
            tokenizer_peek_token: sym!(lib, "tokenizer_peek_token", extern "C" fn() -> CToken),
            tokenizer_reset: sym!(lib, "tokenizer_reset", extern "C" fn()),
            tokenizer_load_text: sym!(
                lib,
                "tokenizer_load_text",
                extern "C" fn(*const c_char) -> c_int
            ),
            tokenizer_get_stats: sym!(
                lib,
                "tokenizer_get_stats",
                extern "C" fn(*mut usize, *mut usize, *mut usize)
            ),
            get_tokenizer_ops: sym!(lib, "get_tokenizer_ops", extern "C" fn() -> COps),
            analyzer_init: sym!(lib, "analyzer_init", extern "C" fn(COps)),
            analyze_text: sym!(lib, "analyze_text", extern "C" fn(*const c_char) -> CResult),
            print_token_distribution: sym!(lib, "print_token_distribution", extern "C" fn()),
            calculate_complexity_score: sym!(
                lib,
                "calculate_complexity_score",
                extern "C" fn() -> c_int
            ),
            find_patterns: sym!(lib, "find_patterns", extern "C" fn(*const c_char)),
            print_menu: sym!(lib, "print_menu", extern "C" fn()),
            print_analysis_result: sym!(lib, "print_analysis_result", extern "C" fn(CResult)),
            interactive_tokenizer: sym!(lib, "interactive_tokenizer", extern "C" fn(COps)),
            read_file: sym!(lib, "read_file", extern "C" fn(*const c_char) -> *mut c_char),
            flush_stdout,
            _lib: lib,
        }
    }

    pub fn flush(&self) {
        if let Some(f) = self.flush_stdout {
            f();
        }
        unsafe {
            fflush(std::ptr::null_mut());
        }
    }

    // -- convenience wrappers -------------------------------------------------

    pub fn load_text(&self, text: &[u8]) -> c_int {
        let s = cstring(text);
        (self.tokenizer_load_text)(s.as_ptr() as *const c_char)
    }

    pub fn next(&self) -> TokenView {
        (self.tokenizer_next_token)().view()
    }

    pub fn peek(&self) -> TokenView {
        (self.tokenizer_peek_token)().view()
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        let mut l = usize::MAX;
        let mut t = usize::MAX;
        let mut c = usize::MAX;
        (self.tokenizer_get_stats)(&mut l, &mut t, &mut c);
        (l, t, c)
    }

    pub fn analyze(&self, text: &[u8]) -> CResult {
        let s = cstring(text);
        (self.analyze_text)(s.as_ptr() as *const c_char)
    }

    pub fn find(&self, pattern: &[u8]) -> Vec<u8> {
        let s = cstring(pattern);
        self.captured(|| (self.find_patterns)(s.as_ptr() as *const c_char))
    }

    /// Tokenize the whole buffer, returning every token including the final EOF.
    pub fn drain_tokens(&self) -> Vec<TokenView> {
        let mut out = Vec::new();
        loop {
            let t = self.next();
            let done = t.ttype == TOKEN_EOF;
            out.push(t);
            if done {
                break;
            }
            if out.len() > 200_000 {
                panic!("{}: tokenizer does not terminate", self.name);
            }
        }
        out
    }

    /// Run `f` with the process' `stdout` redirected into a temporary file and
    /// return everything the library wrote.
    pub fn captured(&self, f: impl FnOnce()) -> Vec<u8> {
        capture_stdout(|| {
            f();
            self.flush();
        })
    }

    /// Like [`Api::captured`] but returns `(stdout, stderr)`.
    pub fn captured_both(&self, f: impl FnOnce()) -> (Vec<u8>, Vec<u8>) {
        capture_out_err(|| {
            f();
            self.flush();
        })
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Serialises the tests of one test binary: both libraries hold global state.
pub fn lock() -> MutexGuard<'static, ()> {
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/`
pub fn target_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test binary>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target dir")
        .to_path_buf()
}

pub fn c_lib_path() -> PathBuf {
    crate_dir().join("c_src/build/libtextanalyzer_c.so")
}

pub fn c_driver_path() -> PathBuf {
    crate_dir().join("c_src/build/driver")
}

pub fn rust_lib_path() -> PathBuf {
    target_dir().join("libtext_analyzer.so")
}

pub fn rust_driver_path() -> PathBuf {
    target_dir().join("driver")
}

/// Builds the C shared library / executable from the unmodified sources if they
/// are missing or out of date.
pub fn ensure_c_artifacts() {
    let root = crate_dir().join("c_src");
    let build = root.join("build");
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");

    let sources: Vec<PathBuf> = ["tokenizer.c", "analyzer.c", "main.c"]
        .iter()
        .map(|f| root.join("src").join(f))
        .collect();
    let newest = sources
        .iter()
        .map(|p| std::fs::metadata(p).unwrap().modified().unwrap())
        .max()
        .unwrap();

    let so = c_lib_path();
    let needs_so = match std::fs::metadata(&so) {
        Ok(m) => m.modified().unwrap() < newest,
        Err(_) => true,
    };
    if needs_so {
        let mut cmd = std::process::Command::new("gcc");
        cmd.arg("-shared")
            .arg("-fPIC")
            .arg("-O2")
            .arg("-I")
            .arg(root.join("include"))
            .arg("-o")
            .arg(&so);
        for s in &sources {
            cmd.arg(s);
        }
        let st = cmd.status().expect("run gcc");
        assert!(st.success(), "building {} failed", so.display());
    }

    let exe = c_driver_path();
    let needs_exe = match std::fs::metadata(&exe) {
        Ok(m) => m.modified().unwrap() < newest,
        Err(_) => true,
    };
    if needs_exe {
        let mut cmd = std::process::Command::new("gcc");
        cmd.arg("-O2")
            .arg("-I")
            .arg(root.join("include"))
            .arg("-o")
            .arg(&exe);
        for s in &sources {
            cmd.arg(s);
        }
        let st = cmd.status().expect("run gcc");
        assert!(st.success(), "building {} failed", exe.display());
    }
}

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| {
        ensure_c_artifacts();
        let rust = ensure_rust_lib();
        Pair {
            c: Api::load("C", &c_lib_path()),
            rust: Api::load("Rust", &rust),
        }
    })
}

// ---------------------------------------------------------------------------
// the out-of-process runner (examples/ffi_runner.rs)
// ---------------------------------------------------------------------------

pub fn runner_path() -> PathBuf {
    target_dir().join("examples").join("ffi_runner")
}

/// `"release"` when the tests themselves were built with `--release`.
fn profile_name() -> String {
    target_dir()
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "debug".to_string())
}

/// Runs `cargo build <args>` in the crate root, for the same profile the tests
/// were built with, trying `--offline` first.
fn cargo_build(args: &[&str], produced: PathBuf) -> PathBuf {
    let profile = profile_name();
    for extra in [&["--offline"][..], &[][..]] {
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.arg("build")
            .args(extra)
            .args(args)
            .current_dir(crate_dir());
        if profile != "debug" {
            cmd.args(["--profile", &profile]);
        }
        if let Ok(out) = cmd.output() {
            if out.status.success() && produced.exists() {
                return produced;
            }
        }
    }
    panic!("`cargo build {:?}` did not produce {}", args, produced.display());
}

static RUST_LIB: OnceLock<PathBuf> = OnceLock::new();

/// Builds the Rust `cdylib` (once per test process).
///
/// `cargo test --test <name>` does **not** rebuild the `cdylib`, because the
/// integration test does not link against it - it is only ever `dlopen`ed.
/// Without this, a stale `.so` from an earlier build would be compared.
pub fn ensure_rust_lib() -> PathBuf {
    RUST_LIB
        .get_or_init(|| cargo_build(&["--lib"], rust_lib_path()))
        .clone()
}

static DRIVER: OnceLock<PathBuf> = OnceLock::new();

/// Builds the Rust `driver` binary (once per test process, so that a stale
/// binary from an earlier build can never be compared).
pub fn ensure_rust_driver() -> PathBuf {
    DRIVER
        .get_or_init(|| cargo_build(&["--bin", "driver"], rust_driver_path()))
        .clone()
}

static RUNNER: OnceLock<PathBuf> = OnceLock::new();

/// Builds `examples/ffi_runner` (a plain `cargo test` builds examples,
/// `cargo test --test <name>` does not) - once per test process.
pub fn ensure_runner() -> PathBuf {
    RUNNER
        .get_or_init(|| cargo_build(&["--example", "ffi_runner"], runner_path()))
        .clone()
}

pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Option<i32>,
    /// The signal that killed the process, if any (`SIGSEGV` = 11).
    pub signal: Option<i32>,
}

/// Replays `script` against one library in a fresh process.
pub fn run_runner(lib: &Path, script: &str, stdin: &[u8]) -> Run {
    use std::process::{Command, Stdio};
    let runner = ensure_runner();
    let n = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let script_path = std::env::temp_dir().join(format!("ta_script_{}_{}.txt", std::process::id(), n));
    std::fs::write(&script_path, script).expect("write script");

    let mut child = Command::new(&runner)
        .arg(lib)
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {}", runner.display(), e));

    {
        let mut si = child.stdin.take().expect("stdin");
        let data = stdin.to_vec();
        std::thread::spawn(move || {
            let _ = si.write_all(&data);
        });
    }
    let out = child.wait_with_output().expect("wait runner");
    let _ = std::fs::remove_file(&script_path);
    use std::os::unix::process::ExitStatusExt;
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Replays `script` against both libraries and asserts byte-identical output.
pub fn diff_runner(script: &str, stdin: &[u8]) -> Run {
    ensure_c_artifacts();
    let c = run_runner(&c_lib_path(), script, stdin);
    let r = run_runner(&rust_lib_path(), script, stdin);
    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "runner stdout differs\nscript:\n{}\nstdin: {}",
        script,
        show(stdin)
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "runner stderr differs\nscript:\n{}\nstdin: {}",
        script,
        show(stdin)
    );
    assert_eq!(
        c.status, r.status,
        "runner exit status differs\nscript:\n{}",
        script
    );
    assert_eq!(
        c.signal, r.signal,
        "runner death signal differs\nscript:\n{}",
        script
    );
    c
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// NUL-terminated copy of `bytes`.
pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    s
}

fn temp_capture_file(tag: &str) -> (std::fs::File, PathBuf) {
    let n = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "ta_cap_{}_{}_{}.txt",
        std::process::id(),
        tag,
        n
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp capture file");
    (file, path)
}

fn slurp(mut file: std::fs::File, path: PathBuf) -> Vec<u8> {
    let mut buf = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek capture file");
    file.read_to_end(&mut buf).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&path);
    buf
}

/// Redirects fd 1 into a temporary file for the duration of `f`.
///
/// The test harness prints its own `test ... ok` progress lines to fd 1 from
/// another thread; holding this binary's `Stdout` lock keeps them out of the
/// captured file.  The `.so`s link their own copy of `std`, so their output is
/// not affected by the lock (and the C build uses `printf` anyway).
pub fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    let stdout_guard = std::io::stdout().lock();
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let (file, path) = temp_capture_file("out");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }

    let buf = slurp(file, path);
    drop(stdout_guard);
    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    buf
}

/// Redirects fd 1 and fd 2 into separate temporary files for the duration of
/// `f` and returns `(stdout, stderr)`.
pub fn capture_out_err(f: impl FnOnce()) -> (Vec<u8>, Vec<u8>) {
    // see capture_stdout: keeps the harness' progress lines and other threads'
    // panic messages out of the captured files
    let stdout_guard = std::io::stdout().lock();
    let stderr_guard = std::io::stderr().lock();
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let (ofile, opath) = temp_capture_file("out");
    let (efile, epath) = temp_capture_file("err");
    let saved_out = unsafe { dup(1) };
    let saved_err = unsafe { dup(2) };
    assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
    assert!(unsafe { dup2(ofile.as_raw_fd(), 1) } >= 0, "dup2(1) failed");
    assert!(unsafe { dup2(efile.as_raw_fd(), 2) } >= 0, "dup2(2) failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
    }

    let out = slurp(ofile, opath);
    let err = slurp(efile, epath);
    drop(stdout_guard);
    drop(stderr_guard);
    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    (out, err)
}

static CAPTURE_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// deterministic RNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
}

// ---------------------------------------------------------------------------
// random text generators
// ---------------------------------------------------------------------------

pub const KEYWORDS: [&str; 31] = [
    "if", "else", "while", "for", "return", "int", "char", "float", "double", "void", "struct",
    "typedef", "const", "static", "extern", "auto", "register", "sizeof", "break", "continue",
    "switch", "case", "default", "do", "goto", "enum", "union", "signed", "unsigned", "long",
    "short",
];

pub const OPERATOR_CHARS: &[u8] = b"+-*/%=<>!&|^~?:";
pub const PUNCT_CHARS: &[u8] = b"(){}[];,.";
pub const TWO_CHAR_OPS: [&str; 11] = ["==", "!=", "<=", ">=", "&&", "||", "++", "--", "->", "<<", ">>"];

/// Bytes that hit interesting tokenizer branches.
pub fn interesting_bytes() -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(b"abcXYZ_09");
    v.extend_from_slice(b" \t\n\r\x0b\x0c");
    v.extend_from_slice(OPERATOR_CHARS);
    v.extend_from_slice(PUNCT_CHARS);
    v.extend_from_slice(b"\"'\\#@$`");
    v.extend_from_slice(&[0x01, 0x7f, 0x80, 0xa0, 0xc3, 0xff]);
    v
}

/// Random byte soup over `interesting_bytes()` (never contains a NUL).
pub fn random_soup(rng: &mut Rng, max_len: usize) -> Vec<u8> {
    let alphabet = interesting_bytes();
    let len = rng.below(max_len + 1);
    let mut v = Vec::with_capacity(len);
    for _ in 0..len {
        if rng.chance(40) {
            // a fully random non-NUL byte
            let mut b = rng.byte();
            if b == 0 {
                b = 1;
            }
            v.push(b);
        } else {
            v.push(*rng.pick(&alphabet));
        }
    }
    v
}

/// Random C-like source text.
pub fn random_source(rng: &mut Rng, max_items: usize) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    let items = rng.below(max_items + 1);
    for _ in 0..items {
        match rng.below(12) {
            0 => v.extend_from_slice(rng.pick(&KEYWORDS).as_bytes()),
            1 => {
                let n = rng.range(1, 8);
                for i in 0..n {
                    let c = if i == 0 {
                        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_"
                            [rng.below(53)]
                    } else {
                        b"abcdefghijklmnopqrstuvwxyz0123456789_"[rng.below(37)]
                    };
                    v.push(c);
                }
            }
            2 => {
                let n = rng.range(1, 6);
                for _ in 0..n {
                    v.push(b'0' + rng.below(10) as u8);
                }
                if rng.chance(3) {
                    v.push(b'.');
                    for _ in 0..rng.range(1, 3) {
                        v.push(b'0' + rng.below(10) as u8);
                    }
                }
                if rng.chance(6) {
                    v.push(b'.');
                    v.push(b'0' + rng.below(10) as u8);
                }
            }
            3 => {
                let q = if rng.chance(2) { b'"' } else { b'\'' };
                v.push(q);
                for _ in 0..rng.below(10) {
                    match rng.below(10) {
                        0 => {
                            v.push(b'\\');
                            v.push(*rng.pick(b"nt\"'\\0"));
                        }
                        1 => v.push(b' '),
                        _ => v.push(b'a' + rng.below(26) as u8),
                    }
                }
                if !rng.chance(5) {
                    v.push(q);
                }
            }
            4 => {
                v.extend_from_slice(b"//");
                for _ in 0..rng.below(20) {
                    v.push(*rng.pick(b"abc 123/*="));
                }
                v.push(b'\n');
            }
            5 => {
                v.extend_from_slice(b"/*");
                for _ in 0..rng.below(20) {
                    v.push(*rng.pick(b"abc 12*/\n"));
                }
                if !rng.chance(4) {
                    v.extend_from_slice(b"*/");
                }
            }
            6 => v.extend_from_slice(rng.pick(&TWO_CHAR_OPS).as_bytes()),
            7 => v.push(*rng.pick(OPERATOR_CHARS)),
            8 => v.push(*rng.pick(PUNCT_CHARS)),
            9 => v.push(b'\n'),
            10 => v.push(*rng.pick(b" \t")),
            _ => v.push(*rng.pick(b"#@$`\\")),
        }
        if rng.chance(3) {
            v.push(*rng.pick(b" \n\t"));
        }
    }
    v
}
