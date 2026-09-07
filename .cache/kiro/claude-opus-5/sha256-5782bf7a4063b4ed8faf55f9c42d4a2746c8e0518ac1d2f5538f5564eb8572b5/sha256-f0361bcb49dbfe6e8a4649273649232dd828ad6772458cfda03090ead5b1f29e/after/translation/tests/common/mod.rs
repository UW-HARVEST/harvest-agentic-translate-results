//! Differential test harness: C `libdriver.so` vs Rust `libdriver.so`.
//!
//! Both libraries are loaded with `libloading` and called **only** through
//! their exported symbols, exactly as an external consumer would — the Rust
//! `#[no_mangle]` wrappers are therefore under test too. Rust functions are
//! never called directly.
//!
//! # Why this harness is shaped the way it is
//!
//! `bad()` reproduces the C's CWE-457 defect: it forwards an *uninitialized*
//! `char *` to `printLine`, so the bytes that reach stdout are raw stack
//! residue. A naive comparison therefore measures the harness rather than the
//! library. Two artifacts must be eliminated (see CONFIGS.md):
//!
//! 1. **argv/env contents.** The stale pointer aims into the top-of-stack
//!    `argv`/`envp` string area. The two libraries' natural paths differ in
//!    length, which shifts that area and changes the printed bytes. Both `.so`s
//!    are therefore copied to **equal-length paths**, and both children are
//!    spawned with **identical argv and an identical environment**.
//! 2. **ASLR.** When the residue *is* a stack address, its bytes are the
//!    randomized address itself, so two processes disagree by construction.
//!    Randomization is disabled in the child via `personality(ADDR_NO_RANDOMIZE)`
//!    before `exec`.
//!
//! Each case runs one child process per library. The child redirects fd 1 to a
//! file, runs an op program, flushes C stdio, and `_exit`s, so the captured
//! bytes are exactly the library's output with no test-harness chatter.
//!
//! `RTLD_LAZY` and `RTLD_NOW` are both exercised: the C `.so` uses lazy PLT
//! binding, so the first intra-library call runs `_dl_runtime_resolve`, which
//! overwrites the very stack word `bad()` reads. That is observable, so it is a
//! configuration axis rather than an implementation detail.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, OsStr};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use libloading::os::unix::Library;

// ---------------------------------------------------------------------------
// libc bits we need (declared directly; no `libc` crate dependency)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn _exit(status: c_int) -> !;
    fn personality(persona: u64) -> c_int;
}

const ADDR_NO_RANDOMIZE: u64 = 0x0004_0000;
const RTLD_LAZY: c_int = 1;
const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;

// Exported signatures of the library under test.
type FnVoid = unsafe extern "C" fn();
type FnInt = unsafe extern "C" fn(c_int);
type FnStr = unsafe extern "C" fn(*const c_char);

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

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
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

// ---------------------------------------------------------------------------
// The op program: a compact, ASCII-safe encoding passed through the environment
// ---------------------------------------------------------------------------
//
//   d:<int>          driver(<int>)
//   b                bad()
//   g                good()
//   pn               printLine(NULL)
//   p:<hex>          printLine(<hex-decoded bytes>, NUL terminated)
//   pr:<hex>:<n>     printLine(<byte> repeated n times, NUL terminated)
//
// Ops are joined with ','. The encoding is kept short on purpose: the child's
// environment lives on the stack, and a huge variable would shift the very
// bytes `bad()` reads. `pr` exists so the 1 MiB case costs a few characters.

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}

/// Calls `f(arg)` and then `g()` back-to-back from one frame, with **no
/// intervening work**, so the slot `f` spills its parameter into is still
/// untouched when `g` runs.
///
/// This is how `bad()`'s slot offset is verified independently of the loaded
/// object's footprint: the residue is placed there by the *previous library
/// call* rather than by `dlopen`, so a faithful translation must read back
/// exactly what the C reads back. `#[inline(never)]` keeps the frame from being
/// merged into the interpreter loop.
#[inline(never)]
extern "C" fn tramp_str_then_void(f: FnStr, arg: *const c_char, g: FnVoid) {
    unsafe { f(arg) };
    unsafe { g() };
}

/// Same, for a `void`-taking first call (`good(); bad();`).
#[inline(never)]
extern "C" fn tramp_void_then_void(f: FnVoid, g: FnVoid) {
    unsafe { f() };
    unsafe { g() };
}

/// Executes one op program against an already-loaded library.
///
/// Every call goes through a symbol looked up in the `.so` by name.
unsafe fn run_ops(lib: &Library, program: &str) {
    let driver = unsafe { lib.get::<FnInt>(b"driver\0").expect("driver") };
    let bad = unsafe { lib.get::<FnVoid>(b"bad\0").expect("bad") };
    let good = unsafe { lib.get::<FnVoid>(b"good\0").expect("good") };
    let print_line = unsafe { lib.get::<FnStr>(b"printLine\0").expect("printLine") };

    for op in program.split(',').filter(|s| !s.is_empty()) {
        if let Some(rest) = op.strip_prefix("d:") {
            let v: i64 = rest.parse().expect("int arg");
            unsafe { driver(v as c_int) };
        } else if op == "b" {
            unsafe { bad() };
        } else if op == "g" {
            unsafe { good() };
        } else if op == "pn" {
            unsafe { print_line(std::ptr::null()) };
        } else if let Some(rest) = op.strip_prefix("tp:") {
            // printLine(<bytes>) immediately followed by bad(), same frame.
            let mut buf = unhex(rest);
            buf.push(0);
            tramp_str_then_void(*print_line, buf.as_ptr() as *const c_char, *bad);
        } else if op == "tg" {
            // good() immediately followed by bad(), same frame.
            tramp_void_then_void(*good, *bad);
        } else if op == "tn" {
            // printLine(NULL) immediately followed by bad(), same frame.
            tramp_str_then_void(*print_line, std::ptr::null(), *bad);
        } else if let Some(rest) = op.strip_prefix("pr:") {
            let (bh, n) = rest.split_once(':').expect("pr:<hex>:<n>");
            let byte = unhex(bh)[0];
            let n: usize = n.parse().unwrap();
            let mut buf = vec![byte; n];
            buf.push(0);
            unsafe { print_line(buf.as_ptr() as *const c_char) };
        } else if let Some(rest) = op.strip_prefix("p:") {
            let mut buf = unhex(rest);
            buf.push(0);
            unsafe { print_line(buf.as_ptr() as *const c_char) };
        } else {
            panic!("bad op {op:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Child mode
// ---------------------------------------------------------------------------

const ENV_LIB: &str = "DIFF_CHILD_LIB";
const ENV_OPS: &str = "DIFF_CHILD_OPS";
const ENV_OUT: &str = "DIFF_CHILD_OUT";
const ENV_MODE: &str = "DIFF_CHILD_MODE";

/// The child worker. A no-op unless the parent set `DIFF_CHILD_LIB`, so it is
/// harmless when `cargo test` runs it as an ordinary test.
#[test]
fn zz_child_worker() {
    let Ok(libpath) = std::env::var(ENV_LIB) else {
        return; // parent mode: nothing to do
    };
    let ops = std::env::var(ENV_OPS).unwrap_or_default();
    let outpath = std::env::var(ENV_OUT).unwrap();
    let flags = match std::env::var(ENV_MODE).unwrap_or_default().as_str() {
        "now" => RTLD_NOW | RTLD_LOCAL,
        _ => RTLD_LAZY | RTLD_LOCAL,
    };

    // Push any test-harness chatter out to the real fd 1 before redirecting.
    std::io::stdout().flush().ok();

    let out = std::fs::File::create(&outpath).expect("create out");
    unsafe { dup2(out.as_raw_fd(), 1) };

    let lib = unsafe { Library::open(Some(&libpath), flags) }.expect("dlopen");
    unsafe { run_ops(&lib, &ops) };

    // Flush C stdio (shared with the loaded library) into the redirected fd,
    // then leave immediately so no harness output can follow.
    unsafe { fflush(std::ptr::null_mut()) };
    unsafe { _exit(0) };
}

// ---------------------------------------------------------------------------
// Parent side: fixture + differential runner
// ---------------------------------------------------------------------------

pub struct Fixture {
    dir: PathBuf,
}

/// Copies both `.so`s to **equal-length** paths: `<tmp>/a/lib.so` (C) and
/// `<tmp>/b/lib.so` (Rust). Equal length is what keeps the top-of-stack
/// `argv` area — which `bad()` reads — identical between the two children.
///
/// `DIFF_LIB_A` / `DIFF_LIB_B` override either side. That exists for the
/// *control* experiments in `tests/phase_b_residue_control.rs`, which compare
/// two behaviourally identical C builds through this very harness in order to
/// establish which divergences are attributable to the translation and which
/// are artifacts of a shared object's load footprint.
pub fn fixture() -> Fixture {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_so = match std::env::var("DIFF_LIB_A") {
        Ok(p) => PathBuf::from(p).canonicalize().expect("DIFF_LIB_A"),
        Err(_) => root.join("../c_src/build/libdriver.so").canonicalize().expect(
            "C library missing — build it: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        ),
    };
    let rust_so = match std::env::var("DIFF_LIB_B") {
        Ok(p) => PathBuf::from(p).canonicalize().expect("DIFF_LIB_B"),
        Err(_) => find_rust_so(root).expect("Rust cdylib missing — run `cargo build --release`"),
    };
    fixture_pair(&c_so, &rust_so)
}

/// Builds a fixture over an explicit pair of libraries. Used by the attribution
/// controls, which must not mutate process-global environment variables: test
/// binaries run their tests in parallel threads, so `set_var` would leak into
/// unrelated fixtures in the same process.
pub fn fixture_pair(a: &Path, b: &Path) -> Fixture {
    let c_so = a.canonicalize().unwrap_or_else(|e| panic!("side A {a:?}: {e}"));
    let rust_so = b.canonicalize().unwrap_or_else(|e| panic!("side B {b:?}: {e}"));

    // One directory per Fixture instance. Tests in a single test binary run in
    // parallel threads by default, so a pid-only name would be shared between
    // concurrently running fixtures and they would delete each other's `.so`
    // and capture files. A per-instance counter keeps them isolated.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let unique = format!(
        "{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let dir = std::env::temp_dir().join(format!("driver_diff_{unique}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("a")).unwrap();
    std::fs::create_dir_all(dir.join("b")).unwrap();
    std::fs::copy(&c_so, dir.join("a/lib.so")).unwrap();
    std::fs::copy(&rust_so, dir.join("b/lib.so")).unwrap();

    // Equal-length paths are the whole point; assert it rather than assume it.
    assert_eq!(
        dir.join("a/lib.so").as_os_str().len(),
        dir.join("b/lib.so").as_os_str().len(),
        "library paths must have equal length"
    );
    Fixture { dir }
}

fn find_rust_so(root: &Path) -> Option<PathBuf> {
    for p in [
        root.join("target/release/libdriver.so"),
        root.join("target/debug/libdriver.so"),
    ] {
        if p.exists() {
            return Some(p.canonicalize().unwrap());
        }
    }
    None
}

impl Fixture {
    fn spawn(&self, side: &str, mode: &str, ops: &str) -> Vec<u8> {
        let lib = self.dir.join(side).join("lib.so");
        let out = self.dir.join(side).join("out.bin");
        let _ = std::fs::remove_file(&out);

        let exe = std::env::current_exe().unwrap();
        let mut cmd = Command::new(&exe);
        cmd.args([
            "--exact",
            "common::zz_child_worker",
            "--test-threads=1",
            "--nocapture",
        ]);

        // An identical, minimal environment for both children: the environment
        // block sits on the stack, and `bad()` can read into it.
        cmd.env_clear();
        cmd.env(ENV_LIB, OsStr::from_bytes(lib.as_os_str().as_bytes()));
        cmd.env(ENV_OPS, ops);
        cmd.env(ENV_OUT, OsStr::from_bytes(out.as_os_str().as_bytes()));
        cmd.env(ENV_MODE, mode);

        cmd.stdout(Stdio::null()).stderr(Stdio::null());

        // Disable ASLR so both children see an identical stack layout.
        unsafe {
            cmd.pre_exec(|| {
                personality(ADDR_NO_RANDOMIZE);
                Ok(())
            })
        };

        let status = cmd.status().expect("spawn child");
        assert!(
            status.success(),
            "child for {side} (mode={mode}, ops={ops}) failed: {status:?}"
        );
        std::fs::read(&out).unwrap_or_else(|e| {
            panic!(
                "child for {side} (mode={mode}, ops={ops}) produced no capture file \
                 at {out:?}: {e}. The child worker did not run -- a missing file must \
                 never be reported as empty output, or every comparison passes vacuously."
            )
        })
    }

    /// Runs `ops` against both libraries and asserts byte-identical stdout.
    pub fn assert_same(&self, row: &str, mode: &str, ops: &str) {
        let c = self.spawn("a", mode, ops);
        let r = self.spawn("b", mode, ops);
        assert_eq!(
            c,
            r,
            "\nROW {row}: C and Rust stdout differ\n  mode = {mode}\n  ops  = {ops}\n\
             \n  C    ({} bytes) = {}\n  Rust ({} bytes) = {}\n",
            c.len(),
            summarize(&c),
            r.len(),
            summarize(&r)
        );
    }

    /// Raw stdout of the **C** library for `ops`. Used to pin an expected
    /// result absolutely, rather than only mirroring C against Rust.
    pub fn c_output(&self, mode: &str, ops: &str) -> Vec<u8> {
        self.spawn("a", mode, ops)
    }

    /// Raw stdout of the **Rust** library for `ops`.
    pub fn rust_output(&self, mode: &str, ops: &str) -> Vec<u8> {
        self.spawn("b", mode, ops)
    }

    /// Comparison for programs containing *unprimed* `bad()` calls.
    ///
    /// An unprimed direct `bad()` reads a slot that was last written by
    /// `dlopen`, so the pointer it forwards is a function of the loaded object's
    /// load footprint (dependency list, `.dynsym` size, segment sizes) rather
    /// than of any translated code. `tests/phase_b_residue_control.rs` shows
    /// this mechanically: two *behaviourally identical C builds* disagree on
    /// exactly these lines, so no translation can match them.
    ///
    /// Everything else is still required to be byte-identical. This asserts:
    ///
    ///  * the two outputs have the same number of lines (so `bad()` took the
    ///    same NULL-vs-non-NULL branch the same number of times on both sides);
    ///  * at most `n_unprimed_bad` lines differ; and
    ///  * after removing those lines, the outputs are byte-identical.
    ///
    /// The slot offset that `bad()` actually reads is verified separately, and
    /// exactly, by the `tp:`/`tg:`/`tn` trampoline rows, where the residue is
    /// written by the previous *library* call instead of by the loader.
    pub fn assert_same_except_loader_residue(
        &self,
        row: &str,
        mode: &str,
        ops: &str,
        n_unprimed_bad: usize,
    ) {
        let c = self.spawn("a", mode, ops);
        let r = self.spawn("b", mode, ops);

        let cl: Vec<&[u8]> = c.split(|b| *b == b'\n').collect();
        let rl: Vec<&[u8]> = r.split(|b| *b == b'\n').collect();
        assert_eq!(
            cl.len(),
            rl.len(),
            "\nROW {row}: line count differs (mode={mode}, ops={ops})\n  C    = {}\n  Rust = {}\n",
            summarize(&c),
            summarize(&r)
        );

        let differing: Vec<usize> = (0..cl.len()).filter(|i| cl[*i] != rl[*i]).collect();
        assert!(
            differing.len() <= n_unprimed_bad,
            "\nROW {row}: {} lines differ but only {n_unprimed_bad} unprimed bad() \
             call(s) can legitimately differ (mode={mode}, ops={ops})\n  C    = {}\n  Rust = {}\n",
            differing.len(),
            summarize(&c),
            summarize(&r)
        );

        let keep_c: Vec<&[u8]> = (0..cl.len())
            .filter(|i| !differing.contains(i))
            .map(|i| cl[i])
            .collect();
        let keep_r: Vec<&[u8]> = (0..rl.len())
            .filter(|i| !differing.contains(i))
            .map(|i| rl[i])
            .collect();
        assert_eq!(
            keep_c, keep_r,
            "\nROW {row}: output differs outside the loader-residue lines \
             (mode={mode}, ops={ops})\n"
        );
    }
}

fn summarize(b: &[u8]) -> String {
    let shown: Vec<u8> = b.iter().copied().take(96).collect();
    let tail = if b.len() > 96 { "..." } else { "" };
    format!("{}{tail} | {:?}{tail}", hex(&shown), String::from_utf8_lossy(&shown))
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}


