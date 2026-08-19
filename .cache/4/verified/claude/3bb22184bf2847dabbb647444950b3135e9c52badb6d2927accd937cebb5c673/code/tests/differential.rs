//! Differential test suite: the C shared object vs the Rust shared object.
//!
//! Both libraries are loaded with `libloading` and driven **only** through their
//! exported C-ABI symbols (`printLine`, `printIntLine`, `bad`, `good`, `main`),
//! exactly as an external consumer would. No Rust function is ever called
//! directly, so the `#[no_mangle]` export wrappers are under test too.
//!
//! * Phase A artifacts: `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`
//! * Phase B (valid paths): `cfg_*` tests, one per `CONFIGS.md` row
//! * Phase C (error paths): `err_*` tests, one per `ERRORS.md` row
//! * Phase D (symbol parity): `symbol_parity_c_so_vs_rust_so`
//!
//! `bad`, `good` and `main` read `stdin`. C's `FILE *stdin` and Rust's buffered
//! stdin both carry stream state, so those rows run in a freshly exec'd child
//! process (`zz_child_runner`, re-execing this same test binary) with `stdin`
//! bound to a file holding the row's input. Within a row the calls are batched
//! so the shared-stream / shared-stdout behaviour is exercised as well.

#![allow(clippy::missing_safety_doc)]

use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ===========================================================================
// Paths & artifact building
// ===========================================================================

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// `target/<profile>/` — derived from the test binary's own location
/// (`target/<profile>/deps/differential-<hash>`).
fn target_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn tmp_dir() -> &'static Path {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let d = target_dir().join("difftmp");
        fs::create_dir_all(&d).expect("create difftmp");
        d
    })
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_file(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    tmp_dir().join(format!("{}-{}-{}", tag, std::process::id(), n))
}

/// The C shared object, compiled from the untouched `c_src/src/main.c`.
///
/// `c_src/CMakeLists.txt` uses `add_executable`, so cmake cannot emit a `.so`
/// for us; the same translation unit is compiled here with `-fPIC -shared` and
/// otherwise-default flags (cmake sets no `CMAKE_BUILD_TYPE`, hence no `-O`),
/// which is what `SYMBOLS.md` documents. Nothing under `c_src/` is modified.
fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = Path::new(MANIFEST_DIR).join("c_src/src/main.c");
        assert!(src.is_file(), "missing C source: {}", src.display());
        let out = target_dir().join("libdriver_c.so");
        let st = Command::new("gcc")
            .args(["-fPIC", "-shared", "-o"])
            .arg(&out)
            .arg(&src)
            .arg("-lm")
            .status()
            .expect("spawn gcc");
        assert!(st.success(), "gcc failed to build the C shared object");
        out
    })
}

/// Profile name of the currently running test binary (`debug` / `release`).
fn profile_name() -> String {
    target_dir()
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "debug".to_string())
}

/// Builds the `cdylib` and `bin` targets into a *separate* target directory and
/// returns that directory.
///
/// This is done UNCONDITIONALLY and never reuses whatever happens to sit in
/// `target/<profile>/`. `cargo test --test differential` does not rebuild the
/// `lib`/`bin` targets, so trusting a pre-existing `target/<profile>/libdriver.so`
/// silently tests a STALE shared object — a divergence introduced in `src/` would
/// then go unnoticed (exactly what `mutation_check.sh` caught). Using a dedicated
/// `CARGO_TARGET_DIR` also avoids contending for the lock the parent cargo
/// invocation holds.
fn ensure_rust_artifacts() -> &'static Path {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let profile = profile_name();
        let alt = Path::new(MANIFEST_DIR).join("target/difftest-artifacts");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(cargo);
        cmd.current_dir(MANIFEST_DIR)
            .env("CARGO_TARGET_DIR", &alt)
            .env_remove("RUSTFLAGS")
            .args(["build", "--offline", "--no-default-features", "--lib", "--bins"]);
        if profile == "release" {
            cmd.arg("--release");
        }
        let st = cmd
            .stdout(Stdio::null())
            .status()
            .expect("spawn cargo to build the cdylib/bin");
        assert!(st.success(), "`cargo build --lib --bins` failed");
        let dir = alt.join(&profile);

        // Staleness guard: the artifacts must be newer than every Rust source.
        let newest_src = ["src/imp.rs", "src/lib.rs", "src/main.rs", "Cargo.toml"]
            .iter()
            .filter_map(|p| fs::metadata(Path::new(MANIFEST_DIR).join(p)).ok())
            .filter_map(|m| m.modified().ok())
            .max()
            .expect("stat sources");
        for artifact in ["libdriver.so", "driver"] {
            let p = dir.join(artifact);
            let built = fs::metadata(&p)
                .and_then(|m| m.modified())
                .unwrap_or_else(|e| panic!("missing artifact {}: {e}", p.display()));
            assert!(
                built >= newest_src,
                "{} is STALE (older than src/) — the suite would be testing outdated code",
                p.display()
            );
        }
        dir
    })
}

/// The Rust `cdylib` (`crate-type = ["cdylib"]`).
fn rust_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let p = ensure_rust_artifacts().join("libdriver.so");
        assert!(p.is_file(), "missing Rust cdylib at {}", p.display());
        p
    })
}

/// The C executable produced by `c_src/CMakeLists.txt` (built with cmake as
/// documented). Falls back to compiling it directly with the same default
/// flags if the cmake build tree is absent.
fn c_exe_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let cmake_built = Path::new(MANIFEST_DIR).join("c_src/build/driver");
        if cmake_built.is_file() {
            return cmake_built;
        }
        let out = target_dir().join("driver_c");
        let st = Command::new("gcc")
            .arg("-o")
            .arg(&out)
            .arg(Path::new(MANIFEST_DIR).join("c_src/src/main.c"))
            .arg("-lm")
            .status()
            .expect("spawn gcc");
        assert!(st.success(), "gcc failed to build the C executable");
        out
    })
}

/// The Rust executable (`[[bin]] name = "driver"`).
fn rust_exe_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let p = ensure_rust_artifacts().join("driver");
        assert!(p.is_file(), "missing Rust binary at {}", p.display());
        p
    })
}

// ===========================================================================
// Deterministic pseudo-random generator (fixed seed => reproducible rows)
// ===========================================================================

/// xorshift64* — small, deterministic, and identical on every platform.
struct Gen(u64);

impl Gen {
    fn new() -> Self {
        Gen(0x243F_6A88_85A3_08D3)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n`.
    fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ===========================================================================
// In-process stdout capture (for the entry points that do not read stdin)
// ===========================================================================

/// Redirects fd 1 to a fresh file, runs `f`, flushes C stdio, restores fd 1 and
/// returns everything that was written. Works for both libraries: the C `.so`
/// writes through glibc `stdio` (flushed with `fflush(NULL)`) and the Rust
/// `.so` writes straight to fd 1 through its own `std`.
fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    let path = tmp_file("cap");
    let file = fs::File::create(&path).expect("create capture file");

    // Make sure nothing of *our* buffered output lands in the capture.
    std::io::stdout().flush().ok();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    assert!(unsafe { libc::dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { libc::close(saved) };
    drop(file);

    let bytes = fs::read(&path).expect("read capture file");
    fs::remove_file(&path).ok();
    bytes
}

// ===========================================================================
// Library handles
// ===========================================================================

/// Both libraries, kept loaded for the whole process.
struct Libs {
    c: &'static Library,
    rust: &'static Library,
}

fn libs() -> &'static Libs {
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(|| {
        let c = unsafe { Library::new(c_so_path()) }.expect("dlopen C .so");
        let rust = unsafe { Library::new(rust_so_path()) }.expect("dlopen Rust .so");
        Libs {
            c: Box::leak(Box::new(c)),
            rust: Box::leak(Box::new(rust)),
        }
    })
}

type PrintLineFn = unsafe extern "C" fn(*const c_char);
type PrintIntLineFn = unsafe extern "C" fn(c_int);
type VoidFn = unsafe extern "C" fn();
type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    unsafe { lib.get(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol {:?} not found: {e}",
            String::from_utf8_lossy(name.strip_suffix(b"\0").unwrap_or(name))
        )
    })
}

// ===========================================================================
// Comparison helper
// ===========================================================================

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for (i, chunk) in bytes.split(|&b| b == b'\n').enumerate() {
        if i > 20 {
            s.push_str("    ...\n");
            break;
        }
        s.push_str(&format!("    [{i}] {:?}\n", String::from_utf8_lossy(chunk)));
    }
    s
}

/// Asserts byte equality and, on mismatch, reports the first differing line
/// together with the input that produced it.
fn assert_same(row: &str, ctx: &str, c_out: &[u8], rust_out: &[u8]) {
    // A row that produced nothing at all would pass trivially; only the
    // explicit "prints nothing" cases are allowed to, and they opt in with the
    // `[empty-ok]` marker in their context string.
    assert!(
        !c_out.is_empty() || ctx.contains("[empty-ok]"),
        "[{row}] the C library produced no output at all — {ctx}"
    );
    if c_out == rust_out {
        return;
    }
    let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
    let r_lines: Vec<&[u8]> = rust_out.split(|&b| b == b'\n').collect();
    let mut first = None;
    for i in 0..c_lines.len().max(r_lines.len()) {
        let a = c_lines.get(i).copied().unwrap_or(b"<missing>");
        let b = r_lines.get(i).copied().unwrap_or(b"<missing>");
        if a != b {
            first = Some((i, a.to_vec(), b.to_vec()));
            break;
        }
    }
    let detail = match first {
        Some((i, a, b)) => format!(
            "first difference at output line {i}:\n      C    = {:?}\n      Rust = {:?}\n",
            String::from_utf8_lossy(&a),
            String::from_utf8_lossy(&b)
        ),
        None => String::new(),
    };
    panic!(
        "[{row}] MISMATCH\n  context: {ctx}\n  {detail}  C output ({} bytes):\n{}  Rust output ({} bytes):\n{}",
        c_out.len(),
        show(c_out),
        rust_out.len(),
        show(rust_out),
    );
}

// ===========================================================================
// Child-process harness for the stdin-reading entry points
// ===========================================================================

const E_LIB: &str = "DIFF_CHILD_LIB";
const E_FN: &str = "DIFF_CHILD_FN";
const E_REPEAT: &str = "DIFF_CHILD_REPEAT";
const E_OUT: &str = "DIFF_CHILD_OUT";
const E_ARGC: &str = "DIFF_CHILD_ARGC";
const E_ARGV: &str = "DIFF_CHILD_ARGV";

fn running_as_child() -> bool {
    std::env::var_os(E_LIB).is_some()
}

/// How the child's fd 0 is set up.
#[derive(Clone, Copy)]
enum StdinKind<'a> {
    /// A regular file pre-filled with these bytes.
    Data(&'a [u8]),
    /// A write-only file: every `read(0, …)` fails with `EBADF`, so `fgets`
    /// returns NULL because of an *error* rather than end-of-file.
    WriteOnly,
}

struct ChildOut {
    out: Vec<u8>,
    rc: Vec<i32>,
}

/// Re-execs this test binary as `zz_child_runner`, which `dlopen`s `lib` and
/// calls the exported `func` `repeat` times with fd 0 bound per `stdin_kind`.
fn run_child(
    lib: &Path,
    func: &str,
    repeat: usize,
    stdin_kind: StdinKind<'_>,
    argc: Option<c_int>,
    argv: Option<&[&str]>,
) -> ChildOut {
    let in_path = tmp_file("in");
    let out_path = tmp_file("out");
    let rc_path = PathBuf::from(format!("{}.rc", out_path.display()));

    let stdin_file = match stdin_kind {
        StdinKind::Data(data) => {
            fs::write(&in_path, data).expect("write child stdin");
            fs::File::open(&in_path).expect("open child stdin")
        }
        StdinKind::WriteOnly => fs::File::create(&in_path).expect("create write-only stdin"),
    };

    let mut cmd = Command::new(std::env::current_exe().expect("current_exe"));
    cmd.args(["zz_child_runner", "--exact", "--nocapture", "--test-threads=1"])
        .env(E_LIB, lib)
        .env(E_FN, func)
        .env(E_REPEAT, repeat.to_string())
        .env(E_OUT, &out_path)
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(a) = argc {
        cmd.env(E_ARGC, a.to_string());
    }
    match argv {
        Some(items) => {
            cmd.env(E_ARGV, items.join("\u{1}"));
        }
        None => {
            cmd.env(E_ARGV, "\u{2}null");
        }
    }

    let status = cmd.status().expect("spawn child runner");
    let out = fs::read(&out_path).unwrap_or_default();
    let rc: Vec<i32> = fs::read_to_string(&rc_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect();
    assert!(
        status.success(),
        "child runner failed for {} / {func} (status {status:?})",
        lib.display()
    );
    fs::remove_file(&in_path).ok();
    fs::remove_file(&out_path).ok();
    fs::remove_file(&rc_path).ok();
    ChildOut { out, rc }
}

/// Runs a row against BOTH `.so` files and asserts byte-identical stdout and
/// identical `main` return codes.
fn diff_stdin_row(row: &str, ctx: &str, func: &str, repeat: usize, stdin_data: &[u8]) {
    let c = run_child(
        c_so_path(),
        func,
        repeat,
        StdinKind::Data(stdin_data),
        None,
        Some(&["driver"]),
    );
    let r = run_child(
        rust_so_path(),
        func,
        repeat,
        StdinKind::Data(stdin_data),
        None,
        Some(&["driver"]),
    );
    assert_same(row, ctx, &c.out, &r.out);
    assert_eq!(c.rc, r.rc, "[{row}] return codes differ — {ctx}");
    // Guard against a vacuous pass: every entry point must have printed
    // at least one line per invocation.
    let produced = c.out.iter().filter(|&&b| b == b'\n').count();
    assert!(
        produced >= repeat,
        "[{row}] suspiciously little output: {produced} lines for {repeat} calls — {ctx}"
    );
}

/// Number of `fgets(buf, CHAR_ARRAY_SIZE, stdin)` calls needed to drain `data`:
/// each call takes bytes up to and including the next `\n`, but never more than
/// `CHAR_ARRAY_SIZE - 1` == 19 of them.
fn fgets_chunks(data: &[u8]) -> usize {
    let mut i = 0usize;
    let mut n = 0usize;
    while i < data.len() {
        let mut taken = 0usize;
        while i < data.len() && taken < 19 {
            let c = data[i];
            i += 1;
            taken += 1;
            if c == b'\n' {
                break;
            }
        }
        n += 1;
    }
    n
}

/// `fgets` calls performed per invocation of each exported entry point.
fn reads_per_call(func: &str) -> usize {
    match func {
        // main -> goodB2G (1) + bad (1); the composed sequences likewise.
        "main" | "good_then_bad" | "bad_then_good" => 2,
        // bad -> 1; good -> goodG2B (0) + goodB2G (1)
        _ => 1,
    }
}

/// Drives `func` enough times to drain `data`, plus two extra calls so the
/// end-of-input (`fgets` returns NULL) path is exercised in the same run.
fn diff_stdin_auto(row: &str, ctx: &str, func: &str, data: &[u8]) {
    let per = reads_per_call(func);
    let repeat = fgets_chunks(data) / per + 2;
    let ctx = format!("{ctx}; {} bytes of stdin, {repeat} x {func}()", data.len());
    diff_stdin_row(row, &ctx, func, repeat, data);
}

/// Convenience: one `fgets` per input line (every line must be short enough —
/// <= 19 bytes including the newline — that a single `fgets` takes exactly one).
fn diff_lines(row: &str, func: &str, lines: &[Vec<u8>]) {
    for (i, l) in lines.iter().enumerate() {
        assert!(
            l.len() <= 19 && l.ends_with(b"\n"),
            "[{row}] generated line {i} is not a single-fgets line: {:?}",
            String::from_utf8_lossy(l)
        );
    }
    let mut data = Vec::new();
    for l in lines {
        data.extend_from_slice(l);
    }
    let ctx = format!(
        "{} lines, first={:?}, last={:?}",
        lines.len(),
        lines.first().map(|l| String::from_utf8_lossy(l).to_string()),
        lines.last().map(|l| String::from_utf8_lossy(l).to_string())
    );
    diff_stdin_auto(row, &ctx, func, &data);
}

// ---------------------------------------------------------------------------
// The child entry point. A no-op unless the DIFF_CHILD_* env vars are set.
// ---------------------------------------------------------------------------

#[test]
fn zz_child_runner() {
    let Ok(libpath) = std::env::var(E_LIB) else {
        return; // ordinary suite run: nothing to do
    };
    let func = std::env::var(E_FN).expect(E_FN);
    let repeat: usize = std::env::var(E_REPEAT).expect(E_REPEAT).parse().expect("repeat");
    let out_path = std::env::var(E_OUT).expect(E_OUT);
    let argc: c_int = std::env::var(E_ARGC)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let argv_spec = std::env::var(E_ARGV).unwrap_or_else(|_| "driver".to_string());

    // Build argv (NULL when the spec is the sentinel).
    let argv_null = argv_spec == "\u{2}null";
    let owned: Vec<CString> = if argv_null {
        Vec::new()
    } else {
        argv_spec
            .split('\u{1}')
            .map(|s| CString::new(s).unwrap())
            .collect()
    };
    let mut argv_ptrs: Vec<*mut c_char> = owned.iter().map(|c| c.as_ptr() as *mut c_char).collect();
    argv_ptrs.push(std::ptr::null_mut());
    let argv: *mut *mut c_char = if argv_null {
        std::ptr::null_mut()
    } else {
        argv_ptrs.as_mut_ptr()
    };

    let lib = unsafe { Library::new(&libpath) }.expect("dlopen in child");
    let lib: &'static Library = Box::leak(Box::new(lib));

    let mut rcs: Vec<i32> = Vec::new();

    // Redirect fd 1 to the capture file so libtest's own chatter stays out.
    std::io::stdout().flush().ok();
    unsafe { libc::fflush(std::ptr::null_mut()) };
    let file = fs::File::create(&out_path).expect("create child out");
    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0);
    assert!(unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0);

    unsafe {
        match func.as_str() {
            "bad" => {
                let f: Symbol<VoidFn> = sym(lib, b"bad\0");
                for _ in 0..repeat {
                    f();
                }
            }
            "good" => {
                let f: Symbol<VoidFn> = sym(lib, b"good\0");
                for _ in 0..repeat {
                    f();
                }
            }
            "main" => {
                let f: Symbol<MainFn> = sym(lib, b"main\0");
                for _ in 0..repeat {
                    rcs.push(f(argc, argv) as i32);
                }
            }
            "good_then_bad" => {
                let g: Symbol<VoidFn> = sym(lib, b"good\0");
                let b: Symbol<VoidFn> = sym(lib, b"bad\0");
                for _ in 0..repeat {
                    g();
                    b();
                }
            }
            "bad_then_good" => {
                let g: Symbol<VoidFn> = sym(lib, b"good\0");
                let b: Symbol<VoidFn> = sym(lib, b"bad\0");
                for _ in 0..repeat {
                    b();
                    g();
                }
            }
            other => panic!("unknown child function {other:?}"),
        }
    }

    unsafe { libc::fflush(std::ptr::null_mut()) };
    assert!(unsafe { libc::dup2(saved, 1) } >= 0);
    unsafe { libc::close(saved) };
    drop(file);

    if !rcs.is_empty() {
        let mut s = String::new();
        for rc in &rcs {
            s.push_str(&format!("{rc}\n"));
        }
        fs::write(format!("{out_path}.rc"), s).expect("write rc file");
    }
}

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

fn nm_defined(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Global text/data/weak symbols only (upper-case kinds).
            if kind.chars().all(|c| c.is_ascii_uppercase()) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`
/// under the exact same name.
#[test]
fn symbol_parity_c_so_vs_rust_so() {
    if running_as_child() {
        return;
    }
    let c = nm_defined(c_so_path());
    let r = nm_defined(rust_so_path());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();

    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\nRust = {r:?}"
    );
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so but not by the C .so: {extra:?}"
    );
    assert_eq!(
        c,
        vec!["bad", "good", "main", "printIntLine", "printLine"],
        "the C .so's exported surface changed; SYMBOLS.md needs updating"
    );

    // No non-libc undefined symbols in the Rust .so.
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    let unresolved: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| {
            // Everything legitimately provided by libc / libgcc / the loader.
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("_ITM_")
                || s.starts_with("_Unwind_")
                || s == "__gmon_start__")
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has undefined non-libc symbols: {unresolved:?}"
    );
}

/// Sanity: `dlsym` really resolves the *library's* symbols (in particular
/// `main`, which the test binary also defines) and both libraries actually run.
#[test]
fn sanity_harness_resolves_library_symbols() {
    if running_as_child() {
        return;
    }
    for (name, so) in [("C", c_so_path()), ("Rust", rust_so_path())] {
        let got = run_child(so, "main", 1, StdinKind::Data(b"4\n5\n"), None, Some(&["driver"]));
        let text = String::from_utf8_lossy(&got.out).to_string();
        assert!(
            text.starts_with("Calling good()...\n"),
            "{name} .so: dlsym(\"main\") did not reach the library's own main; got {text:?}"
        );
        assert_eq!(got.rc, vec![0], "{name} .so: main should return 0");
    }
}

// ===========================================================================
// Input generators (deterministic; every generated line fits a single fgets)
// ===========================================================================

/// `CHAR_ARRAY_SIZE - 1` — the most bytes one `fgets` call can take.
const FGETS_MAX: usize = 19;

/// Turns text into a `\n`-terminated line that exactly one `fgets` consumes.
fn line(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(b'\n');
    assert!(
        v.len() <= FGETS_MAX,
        "generated line too long for one fgets ({} > {FGETS_MAX}): {s:?}",
        v.len()
    );
    v
}

fn lines_of(strs: &[&str]) -> Vec<Vec<u8>> {
    strs.iter().map(|s| line(s)).collect()
}

/// C whitespace that does **not** terminate an `fgets` line.
const WS: [char; 5] = [' ', '\t', '\u{b}', '\u{c}', '\r'];

fn sign(g: &mut Gen) -> &'static str {
    *g.pick(&["", "+", "-"])
}

fn ws_prefix(g: &mut Gen, max: usize) -> String {
    let n = g.below(max as u64 + 1) as usize;
    (0..n).map(|_| *g.pick(&WS)).collect()
}

/// Random decimal integer, e.g. `-1234567`.
fn gen_decimal_int(g: &mut Gen) -> String {
    let digits = 1 + g.below(9) as u32;
    let m = 10u64.pow(digits);
    format!("{}{}", sign(g), g.below(m))
}

/// Random decimal fraction, e.g. `+12.3456789`.
fn gen_decimal_frac(g: &mut Gen) -> String {
    let ip = g.below(1_000_000);
    let fd = g.below(9) as u32;
    let fp = g.below(10u64.pow(fd.max(1)));
    if fd == 0 {
        // exercise the trailing-point and leading-point forms too
        if g.bool() {
            format!("{}{}.", sign(g), ip)
        } else {
            format!("{}.{}", sign(g), fp)
        }
    } else {
        format!("{}{}.{:0width$}", sign(g), ip, fp, width = fd as usize)
    }
}

/// Random scientific notation, e.g. `-1.2345e-38`.
fn gen_scientific(g: &mut Gen) -> String {
    let ip = g.below(10);
    let fd = g.below(6) as u32;
    let frac = if fd == 0 {
        String::new()
    } else {
        format!(".{:0width$}", g.below(10u64.pow(fd)), width = fd as usize)
    };
    let e = if g.bool() { "e" } else { "E" };
    let es = sign(g);
    let ev = g.below(46);
    format!("{}{}{}{}{}{}", sign(g), ip, frac, e, es, ev)
}

/// Random hexadecimal float, e.g. `-0X1a.3fp-4`.
fn gen_hex_float(g: &mut Gen) -> String {
    const HEX: [char; 22] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'A', 'B',
        'C', 'D', 'E', 'F',
    ];
    let prefix = if g.bool() { "0x" } else { "0X" };
    let nint = g.below(5) as usize; // 0..4
    let nfrac = g.below(5) as usize; // 0..4
    let ints: String = (0..nint).map(|_| *g.pick(&HEX)).collect();
    let fracs: String = (0..nfrac).map(|_| *g.pick(&HEX)).collect();
    let mut s = String::new();
    s.push_str(sign(g));
    s.push_str(prefix);
    s.push_str(&ints);
    if nfrac > 0 || g.bool() {
        s.push('.');
        s.push_str(&fracs);
    }
    if g.bool() {
        s.push(if g.bool() { 'p' } else { 'P' });
        s.push_str(sign(g));
        s.push_str(&g.below(40).to_string());
    }
    s
}

/// Random `f32` bit pattern rendered as a decimal literal (`{:e}` is the
/// shortest round-tripping form, so it stays well inside 19 bytes).
fn gen_f32_bits(g: &mut Gen) -> String {
    let v = f32::from_bits(g.next_u32());
    if v.is_nan() {
        return (if g.bool() { "nan" } else { "-nan" }).to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "inf".into() } else { "-inf".into() };
    }
    format!("{:e}", v)
}

/// Random non-numeric noise (never contains `\n` or NUL).
fn gen_garbage(g: &mut Gen) -> String {
    const CH: [char; 34] = [
        'a', 'b', 'c', 'd', 'g', 'h', 'q', 'z', 'A', 'B', 'Z', 'Q', '_', '-', '+', '*', '/', '!',
        '?', '#', '$', '%', '&', '(', ')', '[', ']', '{', '}', '<', '>', ',', ';', ' ',
    ];
    let n = g.below(13) as usize;
    (0..n).map(|_| *g.pick(&CH)).collect()
}

/// Draws from every `atof` subject-sequence form (axis C) — the "all shapes"
/// generator used by the `good`/`main` rows.
fn gen_any(g: &mut Gen) -> String {
    let ws = ws_prefix(g, 2);
    let body = match g.below(9) {
        0 => gen_decimal_int(g),
        1 => gen_decimal_frac(g),
        2 => gen_scientific(g),
        3 => gen_hex_float(g),
        4 => gen_f32_bits(g),
        5 => gen_garbage(g),
        6 => (*g.pick(&["inf", "INF", "Infinity", "-inf", "+INFINITY", "nan", "-NAN", "nan(9)"])).to_string(),
        7 => (*g.pick(&["0", "-0", "0.0", "-0.0", "0e0", "1e-46", "-1e-46", "0.000001", "1e-6", "1e-7"])).to_string(),
        _ => (*g.pick(&[".", "5.", ".5", "1e", "1e+", "1E-", "0x", "0X", "0x.", "0x1p", "0x1p+", "1.2.3", "12abc", "--5", "+ 5", "e5", "0xg", ""])).to_string(),
    };
    let mut s = format!("{ws}{body}");
    while s.len() + 1 > FGETS_MAX {
        s.pop();
    }
    s
}

fn gen_n(n: usize, g: &mut Gen, f: impl Fn(&mut Gen) -> String) -> Vec<Vec<u8>> {
    (0..n).map(|_| line(&f(g))).collect()
}

// ===========================================================================
// Phase B — CONFIGS.md rows C-01 .. C-06 (printLine / printIntLine)
// These entry points do not touch stdin, so they run in-process against both
// dlopen'ed libraries with fd 1 captured.
// ===========================================================================

/// Runs `f` against `printIntLine` of both libraries and compares.
fn diff_print_int(row: &str, ctx: &str, vals: &[i32]) {
    let l = libs();
    let cf: Symbol<PrintIntLineFn> = sym(l.c, b"printIntLine\0");
    let rf: Symbol<PrintIntLineFn> = sym(l.rust, b"printIntLine\0");
    let c_out = capture_fd1(|| {
        for v in vals {
            unsafe { cf(*v as c_int) };
        }
    });
    let r_out = capture_fd1(|| {
        for v in vals {
            unsafe { rf(*v as c_int) };
        }
    });
    assert_same(row, ctx, &c_out, &r_out);
}

/// Runs `printLine` of both libraries over `payloads` (raw, NUL-free) bytes.
fn diff_print_line(row: &str, ctx: &str, payloads: &[Vec<u8>]) {
    let l = libs();
    let cf: Symbol<PrintLineFn> = sym(l.c, b"printLine\0");
    let rf: Symbol<PrintLineFn> = sym(l.rust, b"printLine\0");
    let cstrs: Vec<CString> = payloads
        .iter()
        .map(|p| CString::new(p.clone()).expect("payload must not contain NUL"))
        .collect();
    let c_out = capture_fd1(|| {
        for s in &cstrs {
            unsafe { cf(s.as_ptr()) };
        }
    });
    let r_out = capture_fd1(|| {
        for s in &cstrs {
            unsafe { rf(s.as_ptr()) };
        }
    });
    assert_same(row, ctx, &c_out, &r_out);
}

/// C-01 — `printIntLine` over the whole `i32` range plus every boundary.
#[test]
fn cfg_01_print_int_line_random() {
    if running_as_child() {
        return;
    }
    let mut vals: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        -2,
        9,
        -9,
        10,
        -10,
        50,
        100,
        -100,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i16::MAX as i32,
        i16::MIN as i32,
        1_000_000_000,
        -1_000_000_000,
    ];
    let mut g = Gen::new();
    for _ in 0..20_000 {
        vals.push(g.next_u32() as i32);
    }
    diff_print_int("C-01", &format!("{} int values", vals.len()), &vals);
}

/// C-02 — `printLine` over random printable-ASCII strings, length 0..=257.
#[test]
fn cfg_02_print_line_random_ascii() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let payloads: Vec<Vec<u8>> = (0..2_000)
        .map(|_| {
            let n = g.below(258) as usize;
            (0..n).map(|_| 0x20u8 + (g.below(95) as u8)).collect()
        })
        .collect();
    diff_print_line("C-02", "2000 random ASCII strings, len 0..=257", &payloads);
}

/// C-03 — `printLine` over random **non-UTF-8** byte strings.
#[test]
fn cfg_03_print_line_random_bytes() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let payloads: Vec<Vec<u8>> = (0..2_000)
        .map(|_| {
            let n = g.below(258) as usize;
            // 0x01..=0xFF (NUL would terminate the C string)
            (0..n).map(|_| 1u8 + (g.below(255) as u8)).collect()
        })
        .collect();
    // Sanity: the corpus really does contain invalid UTF-8.
    assert!(
        payloads.iter().any(|p| std::str::from_utf8(p).is_err()),
        "corpus should contain non-UTF-8 payloads"
    );
    diff_print_line("C-03", "2000 random raw byte strings, len 0..=257", &payloads);
}

/// C-04 — `printLine` with printf conversion specifiers in the *data* and with
/// embedded control characters.
#[test]
fn cfg_04_print_line_format_and_ctrl() {
    if running_as_child() {
        return;
    }
    let cases: Vec<Vec<u8>> = [
        "%d", "%s", "%n", "%%", "%1000000d", "%p", "%x", "%999999999s", "%.*f", "%hhn",
        "a%db%sc%nd", "100%", "%", "%%%%", "%c%c%c",
        "tab\there", "cr\rhere", "nl\nhere", "vt\u{b}here", "ff\u{c}here",
        "multi\nline\npayload", "\n", "\n\n\n", "trailing\n", "\r\n",
        "\u{1}\u{2}\u{3}\u{7f}", "mixed \u{7f}%d\t\r\n end",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    diff_print_line("C-04", "printf specifiers + control chars", &cases);
}

/// C-05 — `printLine` with oversized payloads that cross stdio buffer sizes.
#[test]
fn cfg_05_print_line_large() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for n in [1usize, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 65_536] {
        cases.push((0..n).map(|_| 0x21u8 + (g.below(94) as u8)).collect());
    }
    diff_print_line("C-05", "sizes 1..65536 bytes", &cases);
}

/// C-06 — axis H: many randomly interleaved `printLine` / `printIntLine` calls
/// on the single shared `stdout` stream.
#[test]
fn cfg_06_interleaved_print_calls() {
    if running_as_child() {
        return;
    }
    let l = libs();
    let c_pl: Symbol<PrintLineFn> = sym(l.c, b"printLine\0");
    let c_pi: Symbol<PrintIntLineFn> = sym(l.c, b"printIntLine\0");
    let r_pl: Symbol<PrintLineFn> = sym(l.rust, b"printLine\0");
    let r_pi: Symbol<PrintIntLineFn> = sym(l.rust, b"printIntLine\0");

    // Script of operations: Some(text) => printLine, None => printIntLine(int)
    let mut g = Gen::new();
    let mut script: Vec<(Option<CString>, i32)> = Vec::new();
    for _ in 0..4_000 {
        if g.bool() {
            let n = g.below(40) as usize;
            let bytes: Vec<u8> = (0..n).map(|_| 0x21u8 + (g.below(94) as u8)).collect();
            script.push((Some(CString::new(bytes).unwrap()), 0));
        } else {
            script.push((None, g.next_u32() as i32));
        }
    }

    let run = |pl: &Symbol<PrintLineFn>, pi: &Symbol<PrintIntLineFn>| {
        for (text, num) in &script {
            match text {
                Some(s) => unsafe { pl(s.as_ptr()) },
                None => unsafe { pi(*num as c_int) },
            }
        }
    };
    let c_out = capture_fd1(|| run(&c_pl, &c_pi));
    let r_out = capture_fd1(|| run(&r_pl, &r_pi));
    assert_same("C-06", "4000 interleaved printLine/printIntLine calls", &c_out, &r_out);
}

// ===========================================================================
// Phase B — CONFIGS.md rows C-07 .. C-21 (`bad`)
// ===========================================================================

/// C-07 — random decimal integers.
#[test]
fn cfg_07_bad_decimal_ints() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-07", "bad", &gen_n(600, &mut g, gen_decimal_int));
}

/// C-08 — random decimal fractions (incl. `5.` and `.5` forms).
#[test]
fn cfg_08_bad_decimal_fractions() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-08", "bad", &gen_n(600, &mut g, gen_decimal_frac));
}

/// C-09 — explicit sign plus random leading C-whitespace runs.
#[test]
fn cfg_09_bad_sign_and_whitespace() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let v = gen_n(600, &mut g, |g| {
        let ws = ws_prefix(g, 4);
        let body = if g.bool() {
            gen_decimal_int(g)
        } else {
            gen_decimal_frac(g)
        };
        let mut s = format!("{ws}{body}");
        while s.len() + 1 > FGETS_MAX {
            s.pop();
        }
        s
    });
    diff_lines("C-09", "bad", &v);
}

/// C-10 — scientific notation across the whole exponent range.
#[test]
fn cfg_10_bad_scientific() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-10", "bad", &gen_n(600, &mut g, gen_scientific));
}

/// C-11 — hexadecimal floats: `0x`/`0X`, optional `.`frac, optional `p`±exp.
#[test]
fn cfg_11_bad_hex_floats() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-11", "bad", &gen_n(600, &mut g, gen_hex_float));
}

/// C-12 — every case permutation of `inf` / `infinity` / `nan` / `nan(chars)`.
#[test]
fn cfg_12_bad_inf_nan() {
    if running_as_child() {
        return;
    }
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for word in ["inf", "infinity", "nan"] {
        // all 2^len case permutations for the short words, sampled for the long
        let n = word.len().min(8);
        let total = 1usize << n;
        let step = if total > 128 { total / 128 } else { 1 };
        let mut k = 0usize;
        while k < total {
            let cased: String = word
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i < n && (k >> i) & 1 == 1 {
                        c.to_ascii_uppercase()
                    } else {
                        c
                    }
                })
                .collect();
            for pre in ["", "+", "-"] {
                cases.push(line(&format!("{pre}{cased}")));
            }
            k += step;
        }
    }
    for extra in [
        "nan(1)", "-nan(1)", "NAN(abc_9)", "nan()", "-NaN(x)", "inf1", "infinit",
        "infinityx", "nanx", "  inf", "\tnan", "+inf ", "-INF\t",
    ] {
        cases.push(line(extra));
    }
    diff_lines("C-12", "bad", &cases);
}

/// C-13 — unparseable input: `atof` performs no conversion and returns 0.0,
/// which drives the unguarded division by zero.
#[test]
fn cfg_13_bad_unparseable() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let mut cases = gen_n(500, &mut g, gen_garbage);
    for s in ["", " ", "\t", "\u{b}", "\u{c}", "\r", "     ", " \t\u{b}\u{c}\r", "abc", "-", "+", "--", "+-", ".", "-.", "+.", "e", "E", "x"] {
        cases.push(line(s));
    }
    diff_lines("C-13", "bad", &cases);
}

/// C-14 — partial-parse prefixes: strtod consumes the longest valid prefix.
#[test]
fn cfg_14_bad_partial_parse() {
    if running_as_child() {
        return;
    }
    let cases = lines_of(&[
        ".", "5.", ".5", "-.5", "+.5", "5.e", "5.e3", "1e", "1e+", "1e-", "1E", "1E-", "1e+x",
        "0x", "0X", "0x.", "0X.", "0x.p", "0x1p", "0x1p+", "0x1p-", "0x1P", "0xp1", "0xg", "0x g",
        "1.2.3", "1..2", "12abc", "12e3abc", "--5", "++5", "+-5", "-+5", "+ 5", "- 5", "e5", "E5",
        "0", "00", "000000000000000000", "0.", ".0", "0e", "0e0", "0x0", "0x0p0", "0x00.00p0",
        "1_000", "1,000", "1 000", " 12 34", "3.14foo", "0b101", "0o17", "008", "09",
    ]);
    diff_lines("C-14", "bad", &cases);
}

/// C-15 — `double` -> `float` cast underflow / overflow edges.
#[test]
fn cfg_15_bad_float_cast_edges() {
    if running_as_child() {
        return;
    }
    let mags = [
        "1e-45", "1e-46", "7e-46", "1.4e-45", "1e-44", "1.17e-38", "1e-38", "1e-39", "1e-40",
        "3.4e38", "3.4028235e38", "3.4028236e38", "3.5e38", "1e39", "1e40", "1e60", "1e-60",
        "1e308", "1e309", "1e-308", "1e-320", "1e-400", "1e400", "2.2e-308", "5e-324", "1e-323",
    ];
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for m in mags {
        cases.push(line(m));
        cases.push(line(&format!("-{m}")));
        cases.push(line(&format!("+{m}")));
    }
    diff_lines("C-15", "bad", &cases);
}

/// C-16 — values placing `100.0 / data` exactly on / next to the `int` limits,
/// where the x86-64 `cvttsd2si` conversion flips to the indefinite value.
#[test]
fn cfg_16_bad_int_range_edges() {
    if running_as_child() {
        return;
    }
    // 100 / 2^31 = 4.656612873077393e-8 ; 100 / (2^31-1) is a hair larger.
    let mags = [
        "4.656612873e-8",
        "4.6566128731e-8",
        "4.656612874e-8",
        "4.656612872e-8",
        "4.65661287e-8",
        "4.7e-8",
        "4.6e-8",
        "5e-8",
        "4e-8",
        "1e-7",
        "1e-8",
        "1e-9",
        "0x1.p-24",
        "0x1p-24",
        "0x1p-25",
        "0x1p-23",
        "1",
        "2",
        "3",
        "100",
        "-100",
        "0.5",
        "-0.5",
        "1e-30",
        "-1e-30",
        "1e30",
        "-1e30",
        "2147483647",
        "-2147483648",
    ];
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for m in mags {
        cases.push(line(m));
        cases.push(line(&format!("-{m}")));
    }
    diff_lines("C-16", "bad", &cases);
}

/// C-17 — 600 random `f32` bit patterns rendered as decimal text (covers every
/// exponent, subnormals, negative zero, inf and NaN).
#[test]
fn cfg_17_bad_random_float_bits() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-17", "bad", &gen_n(600, &mut g, gen_f32_bits));
}

/// C-18 — the `fgets` truncation boundary: lines around 19/20 bytes leave a
/// remainder in the stream that the *next* call picks up.
#[test]
fn cfg_18_bad_fgets_truncation() {
    if running_as_child() {
        return;
    }
    let mut data: Vec<u8> = Vec::new();
    // exact byte lengths (including the newline) around CHAR_ARRAY_SIZE
    for total in [1usize, 2, 5, 17, 18, 19, 20, 21, 22, 25, 39, 40, 41, 60] {
        // numeric digits so the truncated prefix still parses
        let digits = total - 1;
        let mut s = String::new();
        for i in 0..digits {
            s.push((b'1' + (i % 9) as u8) as char);
        }
        data.extend_from_slice(s.as_bytes());
        data.push(b'\n');
    }
    // the same again but with a decimal point placed right at the cut
    for total in [19usize, 20, 21, 22] {
        let mut s: String = (0..total - 2).map(|i| (b'1' + (i % 9) as u8) as char).collect();
        s.insert(18.min(s.len()), '.');
        data.extend_from_slice(s.as_bytes());
        data.push(b'\n');
    }
    diff_stdin_auto("C-18", "line lengths straddling CHAR_ARRAY_SIZE", "bad", &data);
}

/// C-19 — final line not `\n`-terminated, and `\r\n` line endings.
#[test]
fn cfg_19_bad_no_trailing_newline_and_crlf() {
    if running_as_child() {
        return;
    }
    for (tag, data) in [
        ("no trailing newline", b"12\n34\n56".to_vec()),
        ("single line, no newline", b"7".to_vec()),
        ("only newline", b"\n".to_vec()),
        ("crlf", b"12\r\n34\r\n".to_vec()),
        ("cr only", b"12\r34\r".to_vec()),
        ("lone cr no nl", b"\r".to_vec()),
        ("blank lines", b"\n\n\n1\n\n".to_vec()),
        ("crlf 19 bytes", b"1234567890123456\r\n".to_vec()),
        ("crlf at cut", b"123456789012345678\r\n9\n".to_vec()),
    ] {
        diff_stdin_auto("C-19", tag, "bad", &data);
    }
}

/// C-20 — NUL bytes embedded in the input line.
#[test]
fn cfg_20_bad_embedded_nul() {
    if running_as_child() {
        return;
    }
    for (tag, data) in [
        ("nul first", b"\0123\n".to_vec()),
        ("nul mid", b"12\03\n".to_vec()),
        ("nul last", b"123\0\n".to_vec()),
        ("nul only", b"\0\n".to_vec()),
        ("nuls then num", b"\0\0\0\n7\n".to_vec()),
        ("num nul num", b"5\0 9\n".to_vec()),
        ("nul no newline", b"42\0".to_vec()),
        ("nul in ws", b" \0 4\n".to_vec()),
    ] {
        diff_stdin_auto("C-20", tag, "bad", &data);
    }
}

/// C-21 — axis H: 400 consecutive `bad()` calls sharing one `stdin` position
/// and one `stdout` stream.
#[test]
fn cfg_21_bad_repeated_shared_stream() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-21", "bad", &gen_n(400, &mut g, gen_any));
}

// ===========================================================================
// Phase B — CONFIGS.md rows C-22 .. C-27 (`good` = goodG2B + goodB2G)
// ===========================================================================

/// C-22 — `good` with no input at all: `goodG2B` still prints `50`, then
/// `goodB2G`'s `fgets` fails.
#[test]
fn cfg_22_good_eof() {
    if running_as_child() {
        return;
    }
    diff_stdin_row("C-22", "empty stdin, 3 x good()", "good", 3, b"");
}

/// C-23 — 600 random values across every `atof` form, through `good`.
#[test]
fn cfg_23_good_random_values() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-23", "good", &gen_n(600, &mut g, gen_any));
}

/// C-24 — the `fabs(data) > 0.000001` guard boundary in `goodB2G`.
#[test]
fn cfg_24_good_guard_boundary() {
    if running_as_child() {
        return;
    }
    let mags = [
        "0.000001",
        "0.0000010000001",
        "0.00000100001",
        "0.0000009999999",
        "9.99999e-7",
        "1e-6",
        "1.0000001e-6",
        "0.9999999e-6",
        "1e-7",
        "1e-5",
        "0",
        "0.0",
        "0e0",
        "00.00",
        "1e-46",
        "1e-45",
        "0x1p-20",
        "0x1p-19",
        "0x1.0p-20",
        "1e-8",
        "2e-6",
        "1.000001e-6",
    ];
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for m in mags {
        cases.push(line(m));
        cases.push(line(&format!("-{m}")));
        cases.push(line(&format!("+{m}")));
    }
    diff_lines("C-24", "good", &cases);
}

/// C-25 — `inf` / `nan` / hex / unparseable input through the guard. NaN is the
/// interesting one: every comparison with NaN is false, so `goodB2G` takes the
/// *else* branch.
#[test]
fn cfg_25_good_inf_nan_hex() {
    if running_as_child() {
        return;
    }
    let cases = lines_of(&[
        "inf", "-inf", "+inf", "INF", "Infinity", "-INFINITY", "infx",
        "nan", "-nan", "+nan", "NAN", "NaN", "nan(0)", "-nan(z)", "nanq",
        "0x1", "0x0", "-0x0", "0x1p-30", "-0x1p-30", "0xA.8p2", "0X.1", "0x",
        "abc", "", " ", "-", ".", "1e", "0x.", "--1", "1e400", "-1e400", "1e-400",
    ]);
    diff_lines("C-25", "good", &cases);
}

/// C-26 — `>19`-byte lines so `goodB2G` only ever sees a truncated prefix.
#[test]
fn cfg_26_good_fgets_truncation() {
    if running_as_child() {
        return;
    }
    let mut data: Vec<u8> = Vec::new();
    for total in [19usize, 20, 21, 25, 40, 41] {
        let s: String = (0..total - 1).map(|i| (b'1' + (i % 9) as u8) as char).collect();
        data.extend_from_slice(s.as_bytes());
        data.push(b'\n');
    }
    // a line whose 19-byte prefix is "0.00000000000000000" -> below the guard
    data.extend_from_slice(b"0.000000000000000001234\n");
    // a line whose 19-byte prefix ends mid-exponent
    data.extend_from_slice(b"1.2345678901234567e5\n");
    diff_stdin_auto("C-26", "truncated lines", "good", &data);
}

/// C-27 — axis H: 300 consecutive `good()` calls sharing the stream.
#[test]
fn cfg_27_good_repeated_shared_stream() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-27", "good", &gen_n(300, &mut g, gen_any));
}

// ===========================================================================
// Phase B — CONFIGS.md rows C-28 .. C-33 (`main`, two reads per call)
// ===========================================================================

/// C-28 — `main` with no stdin at all: both `fgets` calls fail.
#[test]
fn cfg_28_main_eof() {
    if running_as_child() {
        return;
    }
    diff_stdin_row("C-28", "empty stdin, 3 x main()", "main", 3, b"");
}

/// C-29 — exactly one line available: `good()` consumes it, `bad()` hits EOF.
#[test]
fn cfg_29_main_single_line() {
    if running_as_child() {
        return;
    }
    for l in ["5", "0", "-0", "abc", "", "1e-9", "inf", "nan", "0.0000005"] {
        let data = format!("{l}\n").into_bytes();
        diff_stdin_row("C-29", &format!("single line {l:?}"), "main", 1, &data);
    }
    // and one line with no trailing newline at all
    diff_stdin_row("C-29", "single line, no newline", "main", 1, b"7");
}

/// C-30 — 400 random line PAIRS through the real two-read pipeline.
#[test]
fn cfg_30_main_random_line_pairs() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    diff_lines("C-30", "main", &gen_n(800, &mut g, gen_any));
}

/// C-31 — one long line: `goodB2G` truncates at 19 bytes and `bad` then reads
/// the *remainder of the same line*.
#[test]
fn cfg_31_main_truncation_carryover() {
    if running_as_child() {
        return;
    }
    for (tag, data) in [
        ("38-byte numeric line", b"12345678901234567890123456789012345678\n".to_vec()),
        ("exponent split at 19", b"1.234567890123456e12\n".to_vec()),
        ("point split at 19", b"1234567890123456789.5\n".to_vec()),
        ("zeros then digits", b"0.00000000000000000001\n".to_vec()),
        ("no newline, 30 bytes", b"123456789012345678901234567890".to_vec()),
        ("60 bytes of 9s", vec![b'9'; 60]),
        ("garbage then number", b"abcdefghijklmnopqrs42\n".to_vec()),
    ] {
        diff_stdin_auto("C-31", tag, "main", &data);
    }
}

/// C-32 — axis G: `main` ignores `argc`/`argv`, including nonsense values.
#[test]
fn cfg_32_main_argc_argv_variants() {
    if running_as_child() {
        return;
    }
    let data: &[u8] = b"3\n4\n";
    let variants: [(&str, Option<c_int>, Option<&[&str]>); 7] = [
        ("argc=1, argv=[driver]", Some(1), Some(&["driver"])),
        ("argc=2, argv=[driver,x]", Some(2), Some(&["driver", "x"])),
        ("argc=0, argv=NULL", Some(0), None),
        ("argc=1, argv=NULL", Some(1), None),
        ("argc=INT_MAX, argv=NULL", Some(c_int::MAX), None),
        ("argc=-1, argv=NULL", Some(-1), None),
        ("argc=INT_MIN, argv=NULL", Some(c_int::MIN), None),
    ];
    for (tag, argc, argv) in variants {
        let c = run_child(c_so_path(), "main", 1, StdinKind::Data(data), argc, argv);
        let r = run_child(rust_so_path(), "main", 1, StdinKind::Data(data), argc, argv);
        assert_same("C-32", tag, &c.out, &r.out);
        assert_eq!(c.rc, r.rc, "[C-32] main return code differs — {tag}");
        assert_eq!(c.rc, vec![0], "[C-32] C main must return 0 — {tag}");
    }
}

/// C-33 — raw byte fuzz: 300 random stdin blobs (random lengths, NULs, `\r`,
/// `\n`, high bytes, often no trailing newline).
#[test]
fn cfg_33_main_raw_byte_fuzz() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    for case in 0..300 {
        let n = g.below(70) as usize;
        let blob: Vec<u8> = (0..n)
            .map(|_| match g.below(10) {
                0 => b'\n',
                1 => b'\r',
                2 => 0u8,
                3 => b' ',
                4..=7 => b'0' + (g.below(10) as u8),
                8 => *g.pick(&[b'.', b'-', b'+', b'e', b'E', b'x', b'X', b'p', b'P']),
                _ => g.below(256) as u8,
            })
            .collect();
        diff_stdin_auto("C-33", &format!("fuzz case {case} ({n} bytes)"), "main", &blob);
    }
}

// ===========================================================================
// Phase B — CONFIGS.md rows C-34 .. C-35 (composed low-level sequences)
// ===========================================================================

/// C-34 — `good()` then `bad()` through the two separate exports: the same read
/// order `main` uses, but driven from outside.
#[test]
fn cfg_34_good_then_bad_sequence() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let lines = gen_n(400, &mut g, gen_any);
    let mut data = Vec::new();
    for l in &lines {
        data.extend_from_slice(l);
    }
    diff_stdin_auto("C-34", "200 x (good(); bad())", "good_then_bad", &data);
}

/// C-35 — `bad()` then `good()`: a sequence `main` never performs, reachable
/// only through the low-level exports.
#[test]
fn cfg_35_bad_then_good_sequence() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let lines = gen_n(400, &mut g, gen_any);
    let mut data = Vec::new();
    for l in &lines {
        data.extend_from_slice(l);
    }
    diff_stdin_auto("C-35", "200 x (bad(); good())", "bad_then_good", &data);
}

// ===========================================================================
// Phase B — CONFIGS.md rows C-36 .. C-37 (whole executables)
// ===========================================================================

/// Runs an executable with `stdin` from `data`, capturing stdout via a pipe.
fn run_exe_pipe(exe: &Path, data: &[u8]) -> (Vec<u8>, Option<i32>) {
    let in_path = tmp_file("exein");
    fs::write(&in_path, data).expect("write exe stdin");
    let f = fs::File::open(&in_path).expect("open exe stdin");
    let out = Command::new(exe)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("spawn exe");
    fs::remove_file(&in_path).ok();
    (out.stdout, out.status.code())
}

/// Runs an executable with `stdout` redirected to a regular file.
fn run_exe_file(exe: &Path, data: &[u8]) -> (Vec<u8>, Option<i32>) {
    let in_path = tmp_file("exein");
    let out_path = tmp_file("exeout");
    fs::write(&in_path, data).expect("write exe stdin");
    let fin = fs::File::open(&in_path).expect("open exe stdin");
    let fout = fs::File::create(&out_path).expect("create exe stdout");
    let st = Command::new(exe)
        .stdin(Stdio::from(fin))
        .stdout(Stdio::from(fout))
        .stderr(Stdio::null())
        .status()
        .expect("spawn exe");
    let bytes = fs::read(&out_path).unwrap_or_default();
    fs::remove_file(&in_path).ok();
    fs::remove_file(&out_path).ok();
    (bytes, st.code())
}

/// C-36 — end-to-end: the cmake-built C `driver` vs the cargo-built Rust
/// `driver`, over 400 random stdin blobs; stdout **and** exit status compared.
#[test]
fn cfg_36_executable_end_to_end() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"\n".to_vec(),
        b"\n\n".to_vec(),
        b"0\n0\n".to_vec(),
        b"2\n2\n".to_vec(),
        b"abc\ndef\n".to_vec(),
        b"1e-9\n1e-9\n".to_vec(),
        b"inf\nnan\n".to_vec(),
        b"0.000001\n0.000001\n".to_vec(),
        b"12345678901234567890123456789\n".to_vec(),
        b"\0\n\0\n".to_vec(),
    ];
    for _ in 0..400 {
        let mut blob = Vec::new();
        let lines = 1 + g.below(3) as usize;
        for _ in 0..lines {
            blob.extend_from_slice(&line(&gen_any(&mut g)));
        }
        if g.below(4) == 0 {
            blob.pop(); // drop the final newline
        }
        inputs.push(blob);
    }
    for (i, data) in inputs.iter().enumerate() {
        let (c_out, c_code) = run_exe_pipe(c_exe_path(), data);
        let (r_out, r_code) = run_exe_pipe(rust_exe_path(), data);
        let ctx = format!("input #{i} = {:?}", String::from_utf8_lossy(data));
        assert_same("C-36", &ctx, &c_out, &r_out);
        assert_eq!(c_code, r_code, "[C-36] exit status differs — {ctx}");
        assert_eq!(c_code, Some(0), "[C-36] C driver should exit 0 — {ctx}");
    }
}

/// C-37 — axis I: `stdout` as a pipe vs as a regular file. C stdio buffers
/// differently in each case; the emitted bytes must nevertheless be identical
/// both between the two destinations and between C and Rust.
#[test]
fn cfg_37_stdout_pipe_vs_file() {
    if running_as_child() {
        return;
    }
    let mut g = Gen::new();
    let mut inputs: Vec<Vec<u8>> = vec![b"".to_vec(), b"2\n2\n".to_vec(), b"0\n0\n".to_vec()];
    for _ in 0..57 {
        let mut blob = Vec::new();
        for _ in 0..(1 + g.below(3)) {
            blob.extend_from_slice(&line(&gen_any(&mut g)));
        }
        inputs.push(blob);
    }
    for (i, data) in inputs.iter().enumerate() {
        let ctx = format!("input #{i} = {:?}", String::from_utf8_lossy(data));
        let (c_pipe, _) = run_exe_pipe(c_exe_path(), data);
        let (c_file, _) = run_exe_file(c_exe_path(), data);
        let (r_pipe, _) = run_exe_pipe(rust_exe_path(), data);
        let (r_file, _) = run_exe_file(rust_exe_path(), data);
        assert_same("C-37", &format!("C pipe vs C file — {ctx}"), &c_pipe, &c_file);
        assert_same("C-37", &format!("Rust pipe vs Rust file — {ctx}"), &r_pipe, &r_file);
        assert_same("C-37", &format!("C pipe vs Rust pipe — {ctx}"), &c_pipe, &r_pipe);
        assert_same("C-37", &format!("C file vs Rust file — {ctx}"), &c_file, &r_file);
    }
}

// ===========================================================================
// Phase C — one test per ERRORS.md row.
//
// Each row asserts (a) C and Rust agree byte-for-byte and (b) the result is the
// *specific* sentinel the C code produces (the exact diagnostic string, or the
// x86-64 integer-indefinite value -2147483648), never merely "both failed".
// ===========================================================================

/// Expected output of one `bad()` call whose `data` ends up as 0 / out of range.
const INDEFINITE: &str = "-2147483648\n";
const FGETS_FAILED: &str = "fgets() failed.\n";
const DIV_BY_ZERO: &str = "This would result in a divide by zero\n";

/// Runs `func` `repeat` times against both `.so` files and additionally asserts
/// the exact expected byte string.
fn diff_expect(row: &str, ctx: &str, func: &str, repeat: usize, data: &[u8], expected: &str) {
    let c = run_child(
        c_so_path(),
        func,
        repeat,
        StdinKind::Data(data),
        None,
        Some(&["driver"]),
    );
    let r = run_child(
        rust_so_path(),
        func,
        repeat,
        StdinKind::Data(data),
        None,
        Some(&["driver"]),
    );
    assert_same(row, ctx, &c.out, &r.out);
    assert_eq!(
        String::from_utf8_lossy(&c.out),
        expected,
        "[{row}] the C library's own result changed — {ctx}"
    );
    assert_eq!(
        String::from_utf8_lossy(&r.out),
        expected,
        "[{row}] Rust does not produce the C sentinel — {ctx}"
    );
}

/// ERRORS row 1 — `printLine(NULL)`: the `if (line != NULL)` guard fails, so
/// nothing at all is printed (not even a newline).
#[test]
fn err_01_print_line_null() {
    if running_as_child() {
        return;
    }
    let l = libs();
    let cf: Symbol<PrintLineFn> = sym(l.c, b"printLine\0");
    let rf: Symbol<PrintLineFn> = sym(l.rust, b"printLine\0");
    let c_out = capture_fd1(|| unsafe { cf(std::ptr::null()) });
    let r_out = capture_fd1(|| unsafe { rf(std::ptr::null()) });
    assert_same("E-01", "printLine(NULL) [empty-ok]", &c_out, &r_out);
    assert!(c_out.is_empty(), "C printLine(NULL) printed {c_out:?}");
    assert!(r_out.is_empty(), "Rust printLine(NULL) printed {r_out:?}");

    // NULL interleaved with real strings: only the non-NULL ones appear.
    let good = CString::new("kept").unwrap();
    let c_out = capture_fd1(|| unsafe {
        cf(std::ptr::null());
        cf(good.as_ptr());
        cf(std::ptr::null());
    });
    let r_out = capture_fd1(|| unsafe {
        rf(std::ptr::null());
        rf(good.as_ptr());
        rf(std::ptr::null());
    });
    assert_same("E-01", "NULL interleaved with non-NULL", &c_out, &r_out);
    assert_eq!(String::from_utf8_lossy(&c_out), "kept\n");
}

/// ERRORS row 2 — `bad()` with `stdin` already at EOF: `fgets` returns NULL,
/// `data` stays `0.0F`, and the unguarded division still runs.
#[test]
fn err_02_bad_fgets_eof() {
    if running_as_child() {
        return;
    }
    let expected = format!("{FGETS_FAILED}{INDEFINITE}");
    diff_expect("E-02", "empty stdin, 1 x bad()", "bad", 1, b"", &expected);
    // repeated calls keep failing the same way
    let expected3 = expected.repeat(3);
    diff_expect("E-02", "empty stdin, 3 x bad()", "bad", 3, b"", &expected3);
}

/// ERRORS row 3 — `goodB2G`'s `fgets` failing: `goodG2B` has already printed
/// `50`, then the diagnostic and the guard's else branch follow.
#[test]
fn err_03_good_fgets_eof() {
    if running_as_child() {
        return;
    }
    let expected = format!("50\n{FGETS_FAILED}{DIV_BY_ZERO}");
    diff_expect("E-03", "empty stdin, 1 x good()", "good", 1, b"", &expected);
    diff_expect(
        "E-03",
        "empty stdin, 2 x good()",
        "good",
        2,
        b"",
        &expected.repeat(2),
    );
}

/// ERRORS row 4 — `fabs(data) > 0.000001` rejects the input.
#[test]
fn err_04_good_b2g_guard_rejects() {
    if running_as_child() {
        return;
    }
    // NOTE: `1e`, `1e+`, `5.` and `.5` are deliberately NOT here — strtod
    // consumes the longest valid prefix, so they yield 1.0 / 5.0 / 0.5 and the
    // guard ACCEPTS them (verified against the C binary).
    let rejected = [
        "0", "-0", "0.0", "-0.0", "0e0", "0x0", "-0x0", "1e-7", "-1e-7", "0.000001", "-0.000001",
        "1e-6", "-1e-6", "0.0000005", "1e-45", "-1e-45", "1e-60", "abc", "", " ", "\t", ".", "-",
        "+", "0x", "0x.", "--5", "+ 5", "e5", "4.7e-8", "-4.7e-8",
    ];
    for s in rejected {
        let data = format!("{s}\n").into_bytes();
        let expected = format!("50\n{DIV_BY_ZERO}");
        diff_expect("E-04", &format!("input {s:?}"), "good", 1, &data, &expected);
    }
    // Just past the boundary the guard must ACCEPT (control cases; expected
    // values taken from the C implementation, which is the ground truth).
    for (s, want) in [
        ("0.0000011", "90909093\n"),
        ("2e-6", "50000000\n"),
        ("-2e-6", "-50000000\n"),
        ("1e", "100\n"),
        ("5.", "20\n"),
        (".5", "200\n"),
    ] {
        let data = format!("{s}\n").into_bytes();
        let expected = format!("50\n{want}");
        diff_expect("E-04", &format!("control (accepted) {s:?}"), "good", 1, &data, &expected);
    }
}

/// ERRORS row 5 — NaN through the guard: every comparison with NaN is false, so
/// `fabs(NaN) > 0.000001` is FALSE and the *else* branch runs.
#[test]
fn err_05_good_b2g_nan_guard() {
    if running_as_child() {
        return;
    }
    for s in ["nan", "-nan", "+nan", "NAN", "NaN", "nan(1)", "-NAN(abc)", "  nan", "\tnan"] {
        let data = format!("{s}\n").into_bytes();
        let expected = format!("50\n{DIV_BY_ZERO}");
        diff_expect("E-05", &format!("input {s:?}"), "good", 1, &data, &expected);
    }
}

/// ERRORS row 6 — the FLAW: `bad()` has no guard, so `100.0 / 0.0` happens and
/// `(int)+inf` yields the x86-64 integer-indefinite value.
#[test]
fn err_06_bad_divide_by_zero() {
    if running_as_child() {
        return;
    }
    // `1e` / `5.` / `.5` are NOT zero (strtod takes the longest valid prefix),
    // so they belong in the control list below, not here.
    for s in [
        "0", "-0", "0.0", "-0.0", "0e0", "00000", "0x0", "-0x0", "0x0p0", "1e-46", "-1e-46",
        "1e-60", "-1e-60", "abc", "", " ", ".", "-", "+", "0x", "0x.", "--5", "e5", "%d",
    ] {
        let data = format!("{s}\n").into_bytes();
        diff_expect("E-06", &format!("input {s:?}"), "bad", 1, &data, INDEFINITE);
    }
    // Controls: partial-parse forms that DO convert, so no divide by zero.
    for (s, want) in [("1e", "100\n"), ("1e+", "100\n"), ("1E-", "100\n"), ("5.", "20\n"), (".5", "200\n")] {
        let data = format!("{s}\n").into_bytes();
        diff_expect("E-06", &format!("control {s:?}"), "bad", 1, &data, want);
    }
}

/// ERRORS row 7 — `100.0/data` outside `int` range, or NaN: same indefinite
/// value, again via undefined behaviour that x86-64 makes deterministic.
#[test]
fn err_07_bad_out_of_int_range() {
    if running_as_child() {
        return;
    }
    for s in [
        "1e-30", "-1e-30", "1e-9", "-1e-9", "1e-20", "4e-8", "-4e-8", "1e-38", "-1e-38", "1e-45",
        "-1e-45", "nan", "-nan", "NAN", "nan(2)", "4.656612e-8", "-4.656612e-8", "0x1p-30",
        "-0x1p-30",
    ] {
        let data = format!("{s}\n").into_bytes();
        diff_expect("E-07", &format!("input {s:?}"), "bad", 1, &data, INDEFINITE);
    }
    // Controls that must stay INSIDE range and therefore NOT be indefinite.
    for (s, want) in [
        ("1", "100\n"),
        ("2", "50\n"),
        ("-1", "-100\n"),
        ("inf", "0\n"),
        ("-inf", "0\n"),
        ("1e30", "0\n"),
        ("4.7e-8", "2127659559\n"),
        ("-4.7e-8", "-2127659559\n"),
    ] {
        let data = format!("{s}\n").into_bytes();
        diff_expect("E-07", &format!("control {s:?}"), "bad", 1, &data, want);
    }
}

/// ERRORS row 8 — the `CHAR_ARRAY_SIZE - 1` == 19 byte `fgets` limit: longer
/// lines are truncated and the remainder stays in the stream.
#[test]
fn err_08_fgets_truncation_boundary() {
    if running_as_child() {
        return;
    }
    // 19 bytes total ("1" + 17 zeros + "\n") is consumed whole by one fgets.
    let exactly_19 = b"100000000000000000\n".to_vec();
    assert_eq!(exactly_19.len(), 19);
    assert_eq!(fgets_chunks(&exactly_19), 1);
    diff_expect("E-08", "exactly 19 bytes", "bad", 1, &exactly_19, "0\n");

    // 20 bytes: fgets takes the first 19 ("1000000000000000000", no newline),
    // leaving "\n" for the next call.
    let exactly_20 = b"1000000000000000000\n".to_vec();
    assert_eq!(exactly_20.len(), 20);
    assert_eq!(fgets_chunks(&exactly_20), 2);
    diff_expect(
        "E-08",
        "exactly 20 bytes -> 2 fgets chunks",
        "bad",
        2,
        &exactly_20,
        &format!("0\n{INDEFINITE}"),
    );

    // 21+ bytes: the tail digits form the *next* value.
    let long = b"22222222222222222225\n".to_vec();
    assert_eq!(long.len(), 21);
    assert_eq!(fgets_chunks(&long), 2);
    diff_expect(
        "E-08",
        "21 bytes: prefix then remainder",
        "bad",
        2,
        &long,
        "0\n20\n",
    );

    // A 40-byte line becomes three chunks.
    let very_long: Vec<u8> = {
        let mut v = vec![b'3'; 39];
        v.push(b'\n');
        v
    };
    assert_eq!(fgets_chunks(&very_long), 3);
    diff_stdin_auto("E-08", "40-byte line", "bad", &very_long);
}

/// ERRORS row 9 — extreme / out-of-range `int` values over the FFI boundary.
#[test]
fn err_09_print_int_line_extremes() {
    if running_as_child() {
        return;
    }
    let vals = [
        0i32,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -2147483648,
        2147483647,
    ];
    diff_print_int("E-09", "int extremes", &vals);

    // And the exact rendering, so a wrong sign/width would be caught.
    let l = libs();
    let cf: Symbol<PrintIntLineFn> = sym(l.c, b"printIntLine\0");
    let out = capture_fd1(|| unsafe {
        cf(i32::MIN as c_int);
        cf(i32::MAX as c_int);
        cf(0);
        cf(-1);
    });
    assert_eq!(
        String::from_utf8_lossy(&out),
        "-2147483648\n2147483647\n0\n-1\n"
    );
}

/// ERRORS row 10 — the zero-length boundary: `printLine("")` is not NULL, so
/// the guard passes and exactly one newline is printed.
#[test]
fn err_10_print_line_empty() {
    if running_as_child() {
        return;
    }
    let empty = CString::new("").unwrap();
    let l = libs();
    let cf: Symbol<PrintLineFn> = sym(l.c, b"printLine\0");
    let rf: Symbol<PrintLineFn> = sym(l.rust, b"printLine\0");
    let c_out = capture_fd1(|| unsafe { cf(empty.as_ptr()) });
    let r_out = capture_fd1(|| unsafe { rf(empty.as_ptr()) });
    assert_same("E-10", "printLine(\"\")", &c_out, &r_out);
    assert_eq!(String::from_utf8_lossy(&c_out), "\n");
    assert_eq!(String::from_utf8_lossy(&r_out), "\n");
}

/// ERRORS row 11 — oversized / non-UTF-8 / format-specifier payloads.
#[test]
fn err_11_print_line_oversized_nonutf8() {
    if running_as_child() {
        return;
    }
    let mut cases: Vec<Vec<u8>> = Vec::new();
    cases.push(vec![b'A'; 65_536]); // oversized
    cases.push((1u8..=255).collect()); // every non-NUL byte, invalid UTF-8
    cases.push((0x80u8..=0xFF).collect()); // pure continuation bytes
    cases.push(b"%d %s %n %p %999999999d".to_vec()); // format specifiers in data
    cases.push(vec![0xC3]); // truncated UTF-8 sequence
    cases.push(vec![0xED, 0xA0, 0x80]); // encoded surrogate
    cases.push(vec![0xF4, 0x90, 0x80, 0x80]); // beyond U+10FFFF
    for c in &cases {
        assert!(!c.contains(&0));
    }
    diff_print_line("E-11", "oversized + non-UTF-8 + format specifiers", &cases);

    // Non-UTF-8 bytes must be emitted verbatim, not replaced with U+FFFD.
    let raw: Vec<u8> = (0x80u8..=0xFF).collect();
    let cstr = CString::new(raw.clone()).unwrap();
    let l = libs();
    let rf: Symbol<PrintLineFn> = sym(l.rust, b"printLine\0");
    let out = capture_fd1(|| unsafe { rf(cstr.as_ptr()) });
    let mut want = raw.clone();
    want.push(b'\n');
    assert_eq!(out, want, "Rust printLine must not transcode raw bytes");
}

/// ERRORS row 12 — `main` ignores `argc`/`argv` entirely and always returns 0,
/// including for nonsense values that no real `exec` would produce.
#[test]
fn err_12_main_ignores_argv() {
    if running_as_child() {
        return;
    }
    // stdin is empty, so BOTH fgets calls (goodB2G's and bad's) fail.
    let full = format!(
        "Calling good()...\n50\n{FGETS_FAILED}{DIV_BY_ZERO}Finished good()\nCalling bad()...\n{FGETS_FAILED}{INDEFINITE}Finished bad()\n"
    );
    let variants: [(&str, Option<c_int>, Option<&[&str]>); 6] = [
        ("argc=0, argv=NULL", Some(0), None),
        ("argc=1, argv=NULL", Some(1), None),
        ("argc=-1, argv=NULL", Some(-1), None),
        ("argc=INT_MIN, argv=NULL", Some(c_int::MIN), None),
        ("argc=INT_MAX, argv=NULL", Some(c_int::MAX), None),
        ("argc=1, argv=[driver]", Some(1), Some(&["driver"])),
    ];
    for (tag, argc, argv) in variants {
        let c = run_child(c_so_path(), "main", 1, StdinKind::Data(b""), argc, argv);
        let r = run_child(rust_so_path(), "main", 1, StdinKind::Data(b""), argc, argv);
        assert_same("E-12", tag, &c.out, &r.out);
        assert_eq!(String::from_utf8_lossy(&c.out), full, "[E-12] C changed — {tag}");
        assert_eq!(String::from_utf8_lossy(&r.out), full, "[E-12] Rust differs — {tag}");
        assert_eq!(c.rc, vec![0], "[E-12] C main must return 0 — {tag}");
        assert_eq!(r.rc, vec![0], "[E-12] Rust main must return 0 — {tag}");
    }
}

/// ERRORS row 13 — `fgets` failing because of a read *error* (fd 0 is
/// write-only, so `read` fails with `EBADF`) rather than end-of-file.
#[test]
fn err_13_fgets_read_error() {
    if running_as_child() {
        return;
    }
    for (func, repeat, expected) in [
        ("bad", 2, format!("{FGETS_FAILED}{INDEFINITE}").repeat(2)),
        ("good", 2, format!("50\n{FGETS_FAILED}{DIV_BY_ZERO}").repeat(2)),
        (
            "main",
            1,
            format!(
                "Calling good()...\n50\n{FGETS_FAILED}{DIV_BY_ZERO}Finished good()\nCalling bad()...\n{FGETS_FAILED}{INDEFINITE}Finished bad()\n"
            ),
        ),
    ] {
        let c = run_child(c_so_path(), func, repeat, StdinKind::WriteOnly, None, Some(&["driver"]));
        let r = run_child(rust_so_path(), func, repeat, StdinKind::WriteOnly, None, Some(&["driver"]));
        assert_same("E-13", &format!("write-only fd 0, {func}"), &c.out, &r.out);
        assert_eq!(
            String::from_utf8_lossy(&c.out),
            expected,
            "[E-13] C result changed for {func}"
        );
        assert_eq!(
            String::from_utf8_lossy(&r.out),
            expected,
            "[E-13] Rust differs for {func}"
        );
    }
}

// ===========================================================================
// Phase B — CONFIGS.md row C-38 (f32 double-rounding boundaries)
//
// `data` is a `float`, but C reaches it via `(float)atof(s)` -- i.e. decimal ->
// `double` -> `float`, TWO roundings. A translation that parsed straight into
// `f32` (one rounding) would agree on almost every input yet differ on decimals
// that sit between an `f32` midpoint and the nearest `double`. Such inputs need
// ~17 significant digits, which still fits in the 19-byte `fgets` window, so
// they are reachable and must be covered.
//
// Construction: for `v` where `100.0/v` is an exact integer, take the midpoint
// `m` between `v` and the next `f32` up, and print `m` with 16/17 significant
// digits. `strtod` rounds that text to exactly `m`, then `(float)` rounds
// half-to-even down to `v` (so `100.0/v` is the exact integer); a single-rounding
// parser would instead land on the next `f32` up and truncate to integer - 1.
// ===========================================================================

fn next_f32_up(v: f32) -> f32 {
    f32::from_bits(v.to_bits() + 1)
}

/// C-38 — decimals engineered to sit on `f32` rounding midpoints.
#[test]
fn cfg_38_bad_double_rounding_boundaries() {
    if running_as_child() {
        return;
    }
    // Each v is exactly representable in f32 and 100.0/v is an exact integer.
    let exact: [f32; 14] = [
        10.0, 6.25, 5.0, 4.0, 3.125, 2.5, 2.0, 1.5625, 1.25, 1.0, 0.8, 0.78125, 0.625, 0.5,
    ];
    let mut cases: Vec<Vec<u8>> = Vec::new();
    let mut interesting = 0usize;

    for v in exact {
        let up = next_f32_up(v);
        let mid = (v as f64 + up as f64) / 2.0;
        // Midpoint printed with increasing precision; the longest that still
        // fits the 19-byte fgets window is the interesting one.
        for prec in 6..=17usize {
            let s = format!("{:.*}", prec, mid);
            if s.len() + 1 <= FGETS_MAX {
                cases.push(line(&s));
                if prec >= 15 {
                    interesting += 1;
                }
            }
        }
        // The plain value and its f32 neighbours, for contrast.
        cases.push(line(&format!("{v}")));
        for s in [format!("{:.10}", up as f64), format!("{:.9}", v as f64)] {
            if s.len() + 1 <= FGETS_MAX {
                cases.push(line(&s));
            }
        }
    }

    // Random f32 midpoints across a wide exponent range, printed at 17 digits.
    let mut g = Gen::new();
    for _ in 0..400 {
        // keep the magnitude in a range where 17 significant digits still fit
        let v = f32::from_bits((g.next_u32() & 0x007F_FFFF) | (0x3F00_0000 | ((g.below(8) as u32) << 23)));
        if !v.is_finite() || v == 0.0 {
            continue;
        }
        let mid = (v as f64 + next_f32_up(v) as f64) / 2.0;
        for prec in [15usize, 16, 17] {
            let s = format!("{:.*}", prec, mid);
            if s.len() + 1 <= FGETS_MAX {
                cases.push(line(&s));
            }
        }
    }

    assert!(
        interesting >= 14,
        "row C-38 must include >=15-digit midpoint inputs (got {interesting})"
    );
    diff_lines("C-38", "bad", &cases);
    diff_lines("C-38", "good", &cases);
}
