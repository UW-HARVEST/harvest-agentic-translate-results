//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are always reached through `dlopen` + `dlsym`
//! (`libloading`), never by calling a Rust function directly, so the
//! `#[no_mangle] extern "C"` export wrappers are part of what is under test.

#![allow(dead_code)]

use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
pub type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;

/// Path of the C reference shared library compiled by `build.rs` from
/// `c_src/src/main.c` with the flags `c_src/CMakeLists.txt` uses (`-fPIC`, no
/// optimisation).
pub fn c_so_default() -> PathBuf {
    PathBuf::from(env!("C_REF_SO"))
}

/// Same C source, compiled `-O2` (gcc vectorises the `fma_array` loop there).
pub fn c_so_o2() -> PathBuf {
    PathBuf::from(env!("C_REF_SO_O2"))
}

/// Every C build variant every row is checked against.
pub fn c_so_variants() -> Vec<(&'static str, PathBuf)> {
    vec![("O0", c_so_default()), ("O2", c_so_o2())]
}

/// `target/<profile>/` — the directory holding the Rust cdylib and the
/// `driver` binary. Derived from the running test executable, which lives in
/// `target/<profile>/deps/`.
pub fn target_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target dir")
        .to_path_buf()
}

/// The Rust `cdylib` under test.
///
/// The integration tests deliberately do **not** link the library crate (they
/// only `dlopen` it), which means `cargo test` has no dependency edge that would
/// rebuild the `cdylib` after a source change. A stale `.so` would silently
/// "verify" the previous translation, so the freshness check below is a hard
/// error rather than a warning. Use `scripts/verify_all.sh` (or
/// `cargo build --all-targets` before `cargo test`) to keep it current.
pub fn rust_so() -> PathBuf {
    let p = target_dir().join("libfma_array.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {} -- run `cargo build --all-targets` first",
        p.display()
    );
    assert_so_fresh(&p);
    p
}

fn assert_so_fresh(so: &Path) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![manifest.join("src")];
    let mut files = vec![manifest.join("Cargo.toml"), manifest.join("build.rs")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                files.push(p);
            }
        }
    }
    for f in files {
        if let Ok(t) = std::fs::metadata(&f).and_then(|m| m.modified()) {
            if newest.as_ref().is_none_or(|(_, n)| t > *n) {
                newest = Some((f, t));
            }
        }
    }
    if let Some((f, t)) = newest {
        assert!(
            t <= so_mtime,
            "STALE Rust cdylib: {} was modified after {} was built.\n\
             The integration tests only dlopen the cdylib, so cargo will not \
             rebuild it automatically.\n\
             Run `cargo build --all-targets` (or scripts/verify_all.sh) and \
             re-run the tests.",
            f.display(),
            so.display()
        );
    }
}

/// The `soprobe` helper (an example target, so it may live in `examples/`).
pub fn soprobe() -> PathBuf {
    let t = target_dir();
    for cand in [t.join("examples").join("soprobe"), t.join("soprobe")] {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "soprobe helper not built; run `cargo build --examples` (looked in {})",
        t.display()
    );
}

/// The C `driver` executable. Prefers the CMake build output, falling back to
/// the copy `build.rs` produces with the identical compiler flags.
pub fn c_driver_exe() -> PathBuf {
    let build = PathBuf::from(env!("C_REF_SO"))
        .parent()
        .expect("c_src/build")
        .to_path_buf();
    for cand in [build.join("driver"), build.join("driver_ref")] {
        if cand.exists() {
            return cand;
        }
    }
    panic!("C driver executable not found in {}", build.display());
}

/// The Rust `driver` executable (`[[bin]] name = "driver"`).
pub fn rust_driver_exe() -> PathBuf {
    let p = target_dir().join("driver");
    assert!(p.exists(), "Rust driver not found at {}", p.display());
    p
}

// ---------------------------------------------------------------------------
// Loaded library wrapper
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: String,
    // Field order matters: the symbols borrow from `lib`, so keep `lib` last
    // for drop order... instead of self-referencing, the raw function pointers
    // are copied out at load time and the library is leaked for the process
    // lifetime, which is what a real consumer of a plugin would do.
    pub fma_array: FmaArrayFn,
    pub call_fma: CallFmaFn,
    _lib: &'static libloading::Library,
}

impl Lib {
    pub fn open(path: &Path, name: &str) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        // Leak so the extracted function pointers stay valid for the whole
        // test process.
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));

        let fma_array: libloading::Symbol<FmaArrayFn> = unsafe { lib.get(b"fma_array\0") }
            .unwrap_or_else(|e| panic!("dlsym(fma_array) in {}: {e}", path.display()));
        let call_fma: libloading::Symbol<CallFmaFn> = unsafe { lib.get(b"call_fma\0") }
            .unwrap_or_else(|e| panic!("dlsym(call_fma) in {}: {e}", path.display()));

        Lib {
            name: name.to_string(),
            fma_array: *fma_array,
            call_fma: *call_fma,
            _lib: lib,
        }
    }
}

/// Every (C variant, Rust) pair each row must agree on.
pub fn pairs() -> Vec<(Lib, Lib)> {
    let mut v = Vec::new();
    for (tag, p) in c_so_variants() {
        let c = Lib::open(&p, &format!("C[{tag}]"));
        let r = Lib::open(&rust_so(), "Rust");
        v.push((c, r));
    }
    v
}

/// Runs `f` on a thread with a large stack.
///
/// `call_fma` allocates `3 * 4 * len` bytes of variable-length arrays **on the
/// caller's stack**, and libtest gives each test thread only 2 MiB, so any
/// `len` above ~170 000 would blow the test thread's stack before it ever
/// reached the code under test. Large-`len` rows therefore run here.
pub fn with_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    let handle = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(f)
        .expect("spawn big-stack thread");
    if let Err(e) = handle.join() {
        std::panic::resume_unwind(e);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seeds keep every run reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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

    /// Full-range `int`.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn vec_i32(&mut self, n: usize) -> Vec<i32> {
        (0..n).map(|_| self.next_i32()).collect()
    }

    /// Values drawn from the interesting extremes of `int`.
    pub fn extreme_i32(&mut self) -> i32 {
        const POOL: [i32; 12] = [
            0,
            1,
            -1,
            2,
            -2,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            0x7FFF,
            -0x8000,
            0x1_0000,
        ];
        *self.pick(&POOL)
    }

    pub fn vec_extreme_i32(&mut self, n: usize) -> Vec<i32> {
        (0..n).map(|_| self.extreme_i32()).collect()
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// One `fma_array` case. `out_len` may exceed `len` so the canary tail can be
/// checked; `alias` picks which read-only pointers share a buffer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alias {
    None,
    Mul1EqMul2,
    Mul2EqAdd,
    AllInputs,
}

/// Factory for a fresh stdin handle (each child process needs its own).
pub type StdinFactory = Box<dyn Fn() -> Stdio>;

/// Runs `fma_array` in both libraries on identical scratch buffers and returns
/// the two resulting `out` buffers (full length, including any canary tail).
///
/// The argument list mirrors the C function's own five parameters plus the two
/// libraries and the aliasing selector, so it is intentionally wide.
#[allow(clippy::too_many_arguments)]
pub fn run_fma_array(
    c: &Lib,
    r: &Lib,
    out_template: &[i32],
    mul1: &[i32],
    mul2: &[i32],
    add: &[i32],
    len: c_int,
    alias: Alias,
) -> (Vec<i32>, Vec<i32>) {
    let mut results = Vec::new();
    for lib in [c, r] {
        let mut out = out_template.to_vec();
        let mut a = mul1.to_vec();
        let mut b = mul2.to_vec();
        let mut d = add.to_vec();
        unsafe {
            let (p1, p2, p3) = match alias {
                Alias::None => (a.as_ptr(), b.as_ptr(), d.as_ptr()),
                Alias::Mul1EqMul2 => (a.as_ptr(), a.as_ptr(), d.as_ptr()),
                Alias::Mul2EqAdd => (a.as_ptr(), b.as_ptr(), b.as_ptr()),
                Alias::AllInputs => (a.as_ptr(), a.as_ptr(), a.as_ptr()),
            };
            (lib.fma_array)(out.as_mut_ptr(), p1, p2, p3, len);
        }
        // Keep the inputs alive until after the call.
        std::hint::black_box((&mut a, &mut b, &mut d));
        results.push(out);
    }
    let r2 = results.pop().unwrap();
    let c2 = results.pop().unwrap();
    (c2, r2)
}

/// Asserts that both libraries produce byte-identical `out` buffers.
#[allow(clippy::too_many_arguments)]
pub fn assert_fma_array_eq(
    c: &Lib,
    r: &Lib,
    label: &str,
    out_template: &[i32],
    mul1: &[i32],
    mul2: &[i32],
    add: &[i32],
    len: c_int,
    alias: Alias,
) {
    let (cv, rv) = run_fma_array(c, r, out_template, mul1, mul2, add, len, alias);
    if cv != rv {
        let first = cv
            .iter()
            .zip(rv.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        panic!(
            "fma_array mismatch [{label}] {} vs {}: len={len} alias={alias:?}\n  \
             first differing index {first}: C={} Rust={}\n  \
             mul1[{first}]={:?} mul2[{first}]={:?} add[{first}]={:?}\n  \
             C   out={:?}\n  Rust out={:?}",
            c.name,
            r.name,
            cv[first],
            rv[first],
            mul1.get(first),
            mul2.get(first),
            add.get(first),
            &cv[..cv.len().min(32)],
            &rv[..rv.len().min(32)],
        );
    }
}

/// Asserts that both libraries' `call_fma` return the same value.
pub fn assert_call_fma_eq(c: &Lib, r: &Lib, label: &str, data: &[i32], len: c_int) {
    let mut cd = data.to_vec();
    let mut rd = data.to_vec();
    let cv = unsafe { (c.call_fma)(cd.as_ptr(), len) };
    let rv = unsafe { (r.call_fma)(rd.as_ptr(), len) };
    std::hint::black_box((&mut cd, &mut rd));
    assert_eq!(
        cv, rv,
        "call_fma mismatch [{label}] {} vs {}: len={len} data(head)={:?}, data(tail)={:?}",
        c.name,
        r.name,
        &data[..data.len().min(8)],
        &data[data.len().saturating_sub(8)..],
    );
}

// ---------------------------------------------------------------------------
// Child-process helpers (for `main` and for crash comparison)
// ---------------------------------------------------------------------------

pub struct RunOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl RunOutcome {
    pub fn describe(&self) -> String {
        format!(
            "code={:?} signal={:?} stdout={:?} stderr={:?}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn finish(mut child: std::process::Child, stdin_data: Option<&[u8]>, per_byte: bool) -> RunOutcome {
    use std::io::Write;
    if let Some(data) = stdin_data {
        let mut si = child.stdin.take().expect("stdin pipe");
        if per_byte {
            for b in data {
                // Ignore EPIPE: the child may stop reading early (exactly what
                // the C does when it has consumed 100 integers).
                if si.write_all(&[*b]).is_err() {
                    break;
                }
                let _ = si.flush();
            }
        } else {
            let _ = si.write_all(data);
        }
        drop(si);
    }
    let out = child.wait_with_output().expect("wait_with_output");
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;
    RunOutcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

/// Runs `soprobe <lib> <op> [args]`, feeding `stdin_data` if given.
pub fn probe(lib: &Path, op: &str, args: &[String], stdin_data: Option<&[u8]>) -> RunOutcome {
    probe_chunked(lib, op, args, stdin_data, false)
}

pub fn probe_chunked(
    lib: &Path,
    op: &str,
    args: &[String],
    stdin_data: Option<&[u8]>,
    per_byte: bool,
) -> RunOutcome {
    let mut cmd = Command::new(soprobe());
    cmd.arg(lib)
        .arg(op)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    let child = cmd.spawn().expect("spawn soprobe");
    finish(child, stdin_data, per_byte)
}

/// Runs a standalone executable with the given stdin.
pub fn run_exe(exe: &Path, stdin_data: &[u8], per_byte: bool) -> RunOutcome {
    let child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    finish(child, Some(stdin_data), per_byte)
}

/// Runs an executable (or `soprobe <lib> main`) with a caller-supplied stdin
/// handle, so unusual stdin flavours (`/dev/null`, a write-only descriptor, a
/// seekable regular file) can be compared too.
pub fn run_with_stdio(exe: &Path, args: &[String], stdin: Stdio) -> RunOutcome {
    let child = Command::new(exe)
        .args(args)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    finish(child, None, false)
}

/// Opens `path` write-only, which makes every `read(0, ...)` fail with EBADF.
pub fn write_only_stdin(path: &Path) -> Stdio {
    let f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .expect("open write-only stdin");
    Stdio::from(f)
}

/// Drives the exported `main` of both `.so`s with the same stdin and asserts
/// byte-identical stdout plus identical exit status.
pub fn assert_main_eq(c_lib: &Path, c_name: &str, stdin_data: &[u8], per_byte: bool) {
    let c = probe_chunked(c_lib, "main", &[], Some(stdin_data), per_byte);
    let r = probe_chunked(&rust_so(), "main", &[], Some(stdin_data), per_byte);
    assert_eq!(
        (c.stdout.clone(), c.code, c.signal),
        (r.stdout.clone(), r.code, r.signal),
        "main() mismatch ({c_name} .so vs Rust .so)\n  stdin={:?}\n  C   : {}\n  Rust: {}",
        String::from_utf8_lossy(&truncate(stdin_data)),
        c.describe(),
        r.describe(),
    );
}

/// Same, for the two standalone driver executables.
pub fn assert_driver_eq(stdin_data: &[u8], per_byte: bool) {
    let c = run_exe(&c_driver_exe(), stdin_data, per_byte);
    let r = run_exe(&rust_driver_exe(), stdin_data, per_byte);
    assert_eq!(
        (c.stdout.clone(), c.code, c.signal),
        (r.stdout.clone(), r.code, r.signal),
        "driver mismatch\n  stdin={:?}\n  C   : {}\n  Rust: {}",
        String::from_utf8_lossy(&truncate(stdin_data)),
        c.describe(),
        r.describe(),
    );
}

fn truncate(d: &[u8]) -> Vec<u8> {
    if d.len() <= 400 {
        d.to_vec()
    } else {
        let mut v = d[..400].to_vec();
        v.extend_from_slice(b"...");
        v
    }
}

// ---------------------------------------------------------------------------
// stdin corpus generation for `main`
// ---------------------------------------------------------------------------

pub const WS: [u8; 6] = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];

/// The exact decimal boundary magnitudes glibc's `%d` treats specially.
pub const BOUNDARY_TOKENS: [&str; 22] = [
    "0",
    "-0",
    "+0",
    "1",
    "-1",
    "2147483646",
    "2147483647",
    "2147483648",
    "2147483649",
    "-2147483647",
    "-2147483648",
    "-2147483649",
    "4294967295",
    "4294967296",
    "9223372036854775806",
    "9223372036854775807",
    "9223372036854775808",
    "9223372036854775809",
    "-9223372036854775807",
    "-9223372036854775808",
    "-9223372036854775809",
    "18446744073709551616",
];

/// Tokens on which the *first* `scanf("%d")` conversion fails outright (no
/// digit is ever consumed), so `main` breaks with `i == 0` and prints `0`.
pub const NON_NUMERIC_TOKENS: [&str; 11] =
    ["abc", ".", ",", "-", "+", "-x", "e5", "--5", "+-3", "#", "_7"];

/// Tokens that *do* have a decimal prefix `%d` converts successfully; the
/// leftover suffix makes the *next* conversion fail.
pub const NUMERIC_PREFIX_TOKENS: [&str; 6] =
    ["0x1f", "3.9", "12abc", "007x", "1,2", "2147483647x"];

/// Everything that makes `scanf("%d")` return something other than 1 at some
/// point in the stream.
pub const BAD_TOKENS: [&str; 17] = [
    "abc",
    ".",
    ",",
    "-",
    "+",
    "-x",
    "e5",
    "--5",
    "+-3",
    "#",
    "_7",
    "0x1f",
    "3.9",
    "12abc",
    "007x",
    "1,2",
    "2147483647x",
];

pub fn random_ws(rng: &mut Rng) -> Vec<u8> {
    let n = 1 + rng.below(3);
    (0..n).map(|_| *rng.pick(&WS)).collect()
}

/// Joins tokens with random whitespace runs, optionally adding leading and
/// trailing whitespace.
pub fn join_tokens(rng: &mut Rng, tokens: &[String], leading: bool, trailing: bool) -> Vec<u8> {
    let mut v = Vec::new();
    if leading {
        v.extend(random_ws(rng));
    }
    for (i, t) in tokens.iter().enumerate() {
        if i > 0 {
            v.extend(random_ws(rng));
        }
        v.extend_from_slice(t.as_bytes());
    }
    if trailing {
        v.extend(random_ws(rng));
    }
    v
}
