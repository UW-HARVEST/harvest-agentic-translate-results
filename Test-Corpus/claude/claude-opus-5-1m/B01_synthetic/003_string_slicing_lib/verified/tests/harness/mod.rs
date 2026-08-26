//! Shared differential-test harness.
//!
//! Both the C library and the Rust library are loaded as **shared objects** with
//! `libloading` and driven exclusively through their exported `slice` symbol, so
//! the `#[no_mangle] extern "C"` wrapper is part of what is under test.  No Rust
//! function of the crate is ever called directly.
//!
//! `slice()` reports its result through two channels:
//!   * the `int` return value (`0` = success, `1` = rejection), and
//!   * bytes written to **stdout** via `printf`/`puts`.
//! Both channels are compared byte-for-byte, together with the caller-visible
//! memory (the string buffer and the two `int`s) to prove neither library
//! mutates its arguments.
//!
//! stdout is captured at the *file-descriptor* level (`dup`/`dup2` onto a temp
//! file) because the libraries use C stdio, not Rust's `println!`.  Capturing is
//! process-global, so these test binaries deliberately use `harness = false`
//! and run every check sequentially from `main()`; nothing may print while a
//! capture session is live, hence divergences are accumulated as strings and
//! reported after the session is dropped.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

/// `int slice(char *mystr, int *start_ptr, int *stop_ptr)`
pub type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub slice: SliceFn,
    _lib: libloading::Library,
}

impl Lib {
    fn load(name: &'static str, path: PathBuf) -> Lib {
        assert!(
            path.is_file(),
            "shared object for `{name}` not found at {}\n\
             build the C side with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             build the Rust cdylib with:\n  \
             cargo build   (or `cargo build --release` when testing --release)\n\
             `./run_verification.sh` does both in the right order.",
            path.display()
        );
        // SAFETY: dlopen of a plain C library with no initialisers of interest.
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let slice: SliceFn = unsafe {
            let sym: libloading::Symbol<SliceFn> = lib
                .get(b"slice\0")
                .unwrap_or_else(|e| panic!("`slice` missing from {}: {e}", path.display()));
            *sym
        };
        Lib {
            name,
            path,
            slice,
            _lib: lib,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libString_Slice.so`
pub fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libString_Slice.so")
}

/// `target/<profile>/libString_Slice.so`, derived from this test executable's
/// own location so it follows `--release` / custom profiles automatically.
pub fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_SLICE_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("exe parent").to_path_buf();
    if dir.file_name().map(|f| f == "deps").unwrap_or(false) {
        dir.pop();
    }
    dir.join("libString_Slice.so")
}

pub fn load_pair() -> (Lib, Lib) {
    let c = Lib::load("C", c_so_path());
    let r = Lib::load("Rust", rust_so_path());
    (c, r)
}

// ---------------------------------------------------------------------------
// stdout capture session
// ---------------------------------------------------------------------------

static SEQ: AtomicU64 = AtomicU64::new(0);

pub struct Session {
    writer: File,
    reader: File,
    path: PathBuf,
    saved: c_int,
}

impl Session {
    pub fn new() -> Session {
        // Drain both stdio layers before hijacking fd 1.
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        unsafe { fflush(std::ptr::null_mut()) };

        let path = std::env::temp_dir().join(format!(
            "slice_capture_{}_{}.bin",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        let writer = File::create(&path).expect("create capture file");
        let reader = File::open(&path).expect("open capture file");
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        let rc = unsafe { dup2(writer.as_raw_fd(), 1) };
        assert!(rc >= 0, "dup2 onto fd 1 failed");
        Session {
            writer,
            reader,
            path,
            saved,
        }
    }

    /// Run `f` with fd 1 redirected and return `(return value, bytes written)`.
    pub fn call(&mut self, f: impl FnOnce() -> c_int) -> (c_int, Vec<u8>) {
        let ret = f();
        unsafe { fflush(std::ptr::null_mut()) };
        let mut out = Vec::new();
        self.reader.read_to_end(&mut out).expect("read capture");
        (ret, out)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
        let _ = self.writer.flush();
        let _ = std::fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Arguments / observations
// ---------------------------------------------------------------------------

/// How an `int *` argument is supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arg {
    /// `NULL` — selects the library's default (`start = 0` / `stop = len`).
    Null,
    /// A readable `int` holding this value.
    Val(i32),
    /// A deliberately unreadable address; dereferencing it must not happen.
    Wild(usize),
}

impl Arg {
    pub fn of(v: Option<i32>) -> Arg {
        match v {
            None => Arg::Null,
            Some(v) => Arg::Val(v),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Obs {
    pub ret: c_int,
    pub out: Vec<u8>,
    /// The string buffer after the call (must be unmodified).
    pub buf: Vec<u8>,
    /// `*start_ptr` after the call, if it was readable.
    pub start_after: Option<i32>,
    /// `*stop_ptr` after the call, if it was readable.
    pub stop_after: Option<i32>,
}

/// NUL-terminate `bytes` for use as a C string.
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

fn esc(bytes: &[u8]) -> String {
    let mut s = String::new();
    for (i, b) in bytes.iter().enumerate() {
        if i == 96 {
            s.push_str("...");
            break;
        }
        match *b {
            b'\n' => s.push_str("\\n"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(*b as char),
            other => s.push_str(&format!("\\x{other:02x}")),
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

pub struct Diff<'a> {
    c: &'a Lib,
    r: &'a Lib,
    session: Session,
    pub cases: usize,
    pub errors: Vec<String>,
}

impl<'a> Diff<'a> {
    fn new(c: &'a Lib, r: &'a Lib) -> Diff<'a> {
        Diff {
            c,
            r,
            session: Session::new(),
            cases: 0,
            errors: Vec::new(),
        }
    }

    fn invoke(&mut self, lib: &Lib, content: &[u8], start: Arg, stop: Arg) -> Obs {
        let mut buf = content.to_vec();
        // Sentinels let us notice a stray write into the int cells.
        let mut s_cell: c_int = match start {
            Arg::Val(v) => v,
            _ => 0x5A5A_5A5A,
        };
        let mut e_cell: c_int = match stop {
            Arg::Val(v) => v,
            _ => 0x5A5A_5A5A,
        };
        let sp: *mut c_int = match start {
            Arg::Null => std::ptr::null_mut(),
            Arg::Val(_) => &mut s_cell,
            Arg::Wild(a) => a as *mut c_int,
        };
        let ep: *mut c_int = match stop {
            Arg::Null => std::ptr::null_mut(),
            Arg::Val(_) => &mut e_cell,
            Arg::Wild(a) => a as *mut c_int,
        };
        let f = lib.slice;
        let p = buf.as_mut_ptr() as *mut c_char;
        let (ret, out) = self.session.call(|| unsafe { f(p, sp, ep) });
        Obs {
            ret,
            out,
            buf,
            start_after: matches!(start, Arg::Val(_)).then_some(s_cell),
            stop_after: matches!(stop, Arg::Val(_)).then_some(e_cell),
        }
    }

    /// Core comparison: call both `.so`s with identical inputs, require
    /// identical return value, identical stdout bytes and untouched arguments.
    pub fn cmp(&mut self, label: &str, content: &[u8], start: Arg, stop: Arg) {
        self.cases += 1;
        let (c, r) = (self.c, self.r);
        let a = self.invoke(c, content, start, stop);
        let b = self.invoke(r, content, start, stop);
        if a != b {
            if self.errors.len() < 12 {
                self.errors.push(format!(
                    "  [{label}] str=\"{}\" (strlen={}) start={:?} stop={:?}\n\
                     \x20     C   : ret={} out=\"{}\" buf=\"{}\" *start={:?} *stop={:?}\n\
                     \x20     Rust: ret={} out=\"{}\" buf=\"{}\" *start={:?} *stop={:?}",
                    esc(content),
                    content.iter().position(|&b| b == 0).unwrap_or(content.len()),
                    start,
                    stop,
                    a.ret,
                    esc(&a.out),
                    esc(&a.buf),
                    a.start_after,
                    a.stop_after,
                    b.ret,
                    esc(&b.out),
                    esc(&b.buf),
                    b.start_after,
                    b.stop_after,
                ));
            }
        }
        // The input buffer must survive the call unchanged in *both* libraries.
        if a.buf != content {
            if self.errors.len() < 12 {
                self.errors
                    .push(format!("  [{label}] C mutated the input buffer"));
            }
        }
    }

    /// Convenience wrapper taking `Option<i32>` indices.
    pub fn cmp_v(&mut self, label: &str, content: &[u8], start: Option<i32>, stop: Option<i32>) {
        self.cmp(label, content, Arg::of(start), Arg::of(stop));
    }

    /// Replay a whole *sequence* of calls against one library and then against
    /// the other, comparing the concatenated transcripts.  This is what catches
    /// hidden per-library state (a cached length, a static buffer, ...) that a
    /// call-by-call comparison would hide because it always alternates.
    pub fn transcript(&mut self, label: &str, cases: &[(Vec<u8>, Arg, Arg)]) {
        self.cases += cases.len();
        let (c, r) = (self.c, self.r);
        let mut a = Vec::new();
        let mut b = Vec::new();
        for (content, s, e) in cases {
            let o = self.invoke(c, content, *s, *e);
            a.push((o.ret, o.out));
        }
        for (content, s, e) in cases {
            let o = self.invoke(r, content, *s, *e);
            b.push((o.ret, o.out));
        }
        for (i, ((ra, oa), (rb, ob))) in a.iter().zip(b.iter()).enumerate() {
            if ra != rb || oa != ob {
                if self.errors.len() < 12 {
                    let (content, s, e) = &cases[i];
                    self.errors.push(format!(
                        "  [{label}] transcript step {i}: str=\"{}\" start={:?} stop={:?}\n\
                         \x20     C   : ret={ra} out=\"{}\"\n\
                         \x20     Rust: ret={rb} out=\"{}\"",
                        esc(content),
                        s,
                        e,
                        esc(oa),
                        esc(ob)
                    ));
                }
            }
        }
    }

    /// Assert an exact expected `(ret, stdout)` from the C side, then require
    /// Rust to match C.  Used by the error-path rows so the table's documented
    /// C behaviour is pinned down, not merely "both did the same thing".
    pub fn cmp_expect(
        &mut self,
        label: &str,
        content: &[u8],
        start: Arg,
        stop: Arg,
        want_ret: c_int,
        want_out: &[u8],
    ) {
        self.cmp(label, content, start, stop);
        let (c, r) = (self.c, self.r);
        let a = self.invoke(c, content, start, stop);
        let b = self.invoke(r, content, start, stop);
        for (who, o) in [("C", &a), ("Rust", &b)] {
            if o.ret != want_ret || o.out != want_out {
                if self.errors.len() < 12 {
                    self.errors.push(format!(
                        "  [{label}] {who} != documented behaviour: want ret={want_ret} out=\"{}\"; got ret={} out=\"{}\"",
                        esc(want_out),
                        o.ret,
                        esc(&o.out)
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal sequential runner (harness = false)
// ---------------------------------------------------------------------------

pub struct Runner {
    pub c: Lib,
    pub r: Lib,
    pub failed: usize,
    pub rows: usize,
    pub total_cases: usize,
}

impl Runner {
    pub fn new(title: &str) -> Runner {
        let (c, r) = load_pair();
        println!("\n=== {title} ===");
        println!("  C    .so: {}", c.path.display());
        println!("  Rust .so: {}", r.path.display());
        Runner {
            c,
            r,
            failed: 0,
            rows: 0,
            total_cases: 0,
        }
    }

    /// Run one table row.  `body` performs as many differential comparisons as
    /// it likes; the row passes only if every one of them matched.
    pub fn row(&mut self, id: &str, body: impl FnOnce(&mut Diff)) {
        self.rows += 1;
        let (errors, cases) = {
            let mut d = Diff::new(&self.c, &self.r);
            body(&mut d);
            (std::mem::take(&mut d.errors), d.cases)
            // `d` (and its Session) is dropped here, restoring fd 1 before we print.
        };
        self.total_cases += cases;
        if errors.is_empty() {
            println!("ok    {id}  ({cases} cases)");
        } else {
            self.failed += 1;
            println!("FAIL  {id}  ({cases} cases, {} divergence(s))", errors.len());
            for e in &errors {
                println!("{e}");
            }
        }
        let _ = std::io::stdout().flush();
    }

    /// A row that needs full control (e.g. `fork`) and reports its own verdict.
    pub fn raw_row(&mut self, id: &str, body: impl FnOnce(&Lib, &Lib) -> Result<String, String>) {
        self.rows += 1;
        let res = body(&self.c, &self.r);
        match res {
            Ok(note) => println!("ok    {id}  ({note})"),
            Err(e) => {
                self.failed += 1;
                println!("FAIL  {id}\n  {e}");
            }
        }
        let _ = std::io::stdout().flush();
    }

    pub fn finish(self) {
        println!(
            "\n{} row(s), {} differential case(s), {} failure(s)",
            self.rows, self.total_cases, self.failed
        );
        let _ = std::io::stdout().flush();
        if self.failed != 0 {
            std::process::exit(1);
        }
    }
}

/// Fork, call `slice(NULL, NULL, NULL)` in the child, and return the raw
/// `waitpid` status so the two libraries' fault behaviour can be compared.
pub fn null_string_status(lib: &Lib) -> c_int {
    let f = lib.slice;
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            _exit(0);
        }
        let mut st: c_int = -1;
        let rc = waitpid(pid, &mut st, 0);
        assert!(rc == pid, "waitpid failed");
        st
    }
}

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn clearerr(stream: *mut c_void);
    static stdout: *mut c_void;
}

/// Call `slice` with **fd 1 closed**, i.e. an unwritable stdout, and report
/// `(return value, errno, fflush result)`.  The C code never checks what
/// `printf` returns, so it must still hand back its usual sentinel; this proves
/// the Rust translation does not grow error handling the C never had.
///
/// The stream's error indicator is cleared before and after so that the two
/// libraries — which share this process' `stdout` FILE — cannot influence each
/// other through sticky stdio state.
pub fn call_with_unwritable_stdout(
    lib: &Lib,
    content: &[u8],
    start: Arg,
    stop: Arg,
) -> (c_int, c_int, c_int) {
    let mut buf = content.to_vec();
    let mut s_cell: c_int = match start {
        Arg::Val(v) => v,
        _ => 0,
    };
    let mut e_cell: c_int = match stop {
        Arg::Val(v) => v,
        _ => 0,
    };
    let sp: *mut c_int = match start {
        Arg::Null => std::ptr::null_mut(),
        Arg::Val(_) => &mut s_cell,
        Arg::Wild(a) => a as *mut c_int,
    };
    let ep: *mut c_int = match stop {
        Arg::Null => std::ptr::null_mut(),
        Arg::Val(_) => &mut e_cell,
        Arg::Wild(a) => a as *mut c_int,
    };
    let f = lib.slice;
    let p = buf.as_mut_ptr() as *mut c_char;
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
        clearerr(stdout);
        let saved = dup(1);
        assert!(saved >= 0);
        close(1);
        *__errno_location() = 0;
        let ret = f(p, sp, ep);
        let flushed = fflush(stdout);
        let err = *__errno_location();
        dup2(saved, 1);
        close(saved);
        clearerr(stdout);
        *__errno_location() = 0;
        (ret, err, flushed)
    }
}

pub fn wif_signaled(status: c_int) -> bool {
    (status & 0x7f) != 0 && (status & 0x7f) != 0x7f
}
pub fn wtermsig(status: c_int) -> c_int {
    status & 0x7f
}
pub fn wexitstatus(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEE_CE66_D000_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish value in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    pub fn byte_ascii(&mut self) -> u8 {
        0x20 + self.below(0x5f) as u8 // ' '..='~'
    }
    /// Any non-NUL byte (so `strlen` == requested length).
    pub fn byte_nonzero(&mut self) -> u8 {
        1 + self.below(255) as u8
    }
}

/// Random printable-ASCII C string of `len` bytes (plus NUL).
pub fn rand_ascii(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len).map(|_| rng.byte_ascii()).collect();
    v.push(0);
    v
}

/// Random C string of `len` non-NUL bytes (full 0x01..=0xff range, plus NUL).
pub fn rand_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len).map(|_| rng.byte_nonzero()).collect();
    v.push(0);
    v
}
