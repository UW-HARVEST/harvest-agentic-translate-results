//! Shared harness for differential testing of the C and Rust `StaticAlias`
//! shared libraries.
//!
//! Both libraries are exercised only through the symbols they export: the tests
//! either `dlopen` them directly with `libloading`, or drive them through the
//! `driver_runner` example, which also uses `libloading`. The Rust functions are
//! never called as Rust functions, so the `#[no_mangle]` wrappers are part of
//! what is under test.
//!
//! Two mechanisms are used because the two entry points have different
//! observables:
//!
//! * `static_alias` returns a pointer, so the tests need to inspect it in
//!   process. That is done with `libloading` directly.
//! * `driver` writes to `stdout` via C `printf`. Capturing file descriptor 1
//!   in process is unreliable under `cargo test`, whose harness writes progress
//!   output to the same descriptor from other threads, so those comparisons run
//!   in a child process whose stdout is a pipe.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Locating the artifacts
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workspace>/c_src/build/libStaticAlias.so`
pub fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libStaticAlias.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The directory holding this profile's build artifacts.
///
/// The test executable lives in `<target>/<profile>/deps/`, so the cdylib and
/// the `examples/` directory are one level up.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("test exe is under <target>/<profile>/deps/")
        .to_path_buf()
}

/// `--release` if the test binary itself was built with the release profile.
fn profile_args() -> &'static [&'static str] {
    let dir = profile_dir();
    match dir.file_name().and_then(|s| s.to_str()) {
        Some("debug") => &[],
        _ => &["--release"],
    }
}

/// `cargo test` does not necessarily build the `cdylib` or the examples, so any
/// artifact the tests need is built on demand.
fn ensure_built(path: &Path, cargo_args: &[&str]) {
    if path.is_file() {
        return;
    }
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .arg("build")
        .args(cargo_args)
        .args(profile_args())
        .current_dir(manifest_dir())
        .status()
        .unwrap_or_else(|e| panic!("run cargo build {cargo_args:?}: {e}"));
    assert!(status.success(), "cargo build {cargo_args:?} failed");
    assert!(
        path.is_file(),
        "{} still missing after cargo build {cargo_args:?}",
        path.display()
    );
}

/// The Rust `cdylib`, built for the same profile as the test binary.
pub fn rust_so_path() -> PathBuf {
    static SO: OnceLock<PathBuf> = OnceLock::new();
    SO.get_or_init(|| {
        let p = profile_dir().join("libStaticAlias.so");
        ensure_built(&p, &["--lib"]);
        p
    })
    .clone()
}

/// The `driver_runner` example, built on demand if `cargo test` did not already
/// build it.
fn runner_path() -> &'static Path {
    static RUNNER: OnceLock<PathBuf> = OnceLock::new();
    RUNNER.get_or_init(|| {
        let p = profile_dir().join("examples/driver_runner");
        ensure_built(&p, &["--example", "driver_runner"]);
        p
    })
}

// ---------------------------------------------------------------------------
// In-process loading with fresh static state
// ---------------------------------------------------------------------------

/// `static_alias` keeps mutable state in a function-local `static`, and that
/// state lives as long as the library stays loaded. `dlopen` de-duplicates by
/// file identity, so re-opening the same path hands back the already-loaded
/// image together with its accumulated state.
///
/// To get a genuinely fresh `inner == 1`, each library is copied to a unique
/// temporary file and that copy is opened instead.
struct FreshLib {
    lib: Library,
    path: PathBuf,
}

impl FreshLib {
    fn open(src: &Path, tag: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);

        let dir = std::env::temp_dir().join(format!(
            "staticalias-difftest-{}-{}",
            std::process::id(),
            n
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(format!("lib{tag}StaticAlias.so"));
        std::fs::copy(src, &path)
            .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), path.display()));

        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));

        FreshLib { lib, path }
    }
}

impl Drop for FreshLib {
    fn drop(&mut self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
type DriverFn = unsafe extern "C" fn(c_int, c_int);

/// One implementation (C or Rust), loaded with freshly initialised statics.
pub struct Impl {
    lib: FreshLib,
    pub name: &'static str,
}

impl Impl {
    pub fn static_alias(&self, outer: *mut c_int) -> *mut c_int {
        let f: Symbol<StaticAliasFn> = unsafe { self.lib.lib.get(b"static_alias\0") }
            .unwrap_or_else(|e| panic!("{}: symbol static_alias: {e}", self.name));
        unsafe { f(outer) }
    }

    pub fn driver(&self, initial_value: c_int, iterations: c_int) {
        let f: Symbol<DriverFn> = unsafe { self.lib.lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{}: symbol driver: {e}", self.name));
        unsafe { f(initial_value, iterations) }
    }
}

/// A matched pair of freshly loaded C and Rust libraries.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

impl Pair {
    pub fn new() -> Self {
        Pair {
            c: Impl {
                lib: FreshLib::open(&c_so_path(), "C"),
                name: "C",
            },
            rust: Impl {
                lib: FreshLib::open(&rust_so_path(), "Rust"),
                name: "Rust",
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Observing `static_alias` in process
// ---------------------------------------------------------------------------

/// Where the pointer returned by `static_alias` points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The caller's own object was returned (the `else` branch).
    Outer,
    /// A pointer into the library's own storage was returned (the `if` branch,
    /// `&inner`). Addresses differ between the two libraries, so only the
    /// classification is compared.
    Internal,
}

/// Everything observable about a single `static_alias` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AliasObs {
    pub target: Target,
    /// `*returned_pointer` after the call.
    pub ret_value: c_int,
    /// `*outer` after the call.
    pub outer_after: c_int,
    /// Whether the returned pointer equals the internal pointer seen on the
    /// previous `if`-branch call; `None` when there was no previous one or when
    /// the caller's object was returned.
    pub ret_ptr_stable: Option<bool>,
}

/// Calls `static_alias` on one implementation with a fresh cell holding `outer`
/// and records everything observable. `prev_internal` carries the internal
/// address seen earlier, if any.
pub fn observe_alias(imp: &Impl, outer: c_int, prev_internal: &mut Option<*mut c_int>) -> AliasObs {
    let mut cell: c_int = outer;
    let outer_ptr: *mut c_int = &mut cell;

    let ret = imp.static_alias(outer_ptr);
    assert!(!ret.is_null(), "{} returned NULL", imp.name);

    let target = if ret == outer_ptr {
        Target::Outer
    } else {
        Target::Internal
    };

    let ret_value = unsafe { *ret };
    let outer_after = cell;

    let ret_ptr_stable = if target == Target::Internal {
        let stable = prev_internal.map(|p| p == ret);
        *prev_internal = Some(ret);
        stable
    } else {
        None
    };

    AliasObs {
        target,
        ret_value,
        outer_after,
        ret_ptr_stable,
    }
}

/// Feeds an identical sequence of `outer` values to both implementations and
/// asserts every observation matches.
///
/// The two libraries are stepped in lockstep so their internal `inner` state
/// evolves through exactly the same history.
pub fn compare_alias_sequence(inputs: &[c_int]) {
    let pair = Pair::new();
    let mut c_prev: Option<*mut c_int> = None;
    let mut rust_prev: Option<*mut c_int> = None;

    for (i, &v) in inputs.iter().enumerate() {
        let c_obs = observe_alias(&pair.c, v, &mut c_prev);
        let rust_obs = observe_alias(&pair.rust, v, &mut rust_prev);
        assert_eq!(
            c_obs, rust_obs,
            "static_alias mismatch at step {i} (outer = {v}); sequence so far: {:?}",
            &inputs[..=i]
        );
    }
}

// ---------------------------------------------------------------------------
// Out-of-process script comparison (covers stdout bytes)
// ---------------------------------------------------------------------------

/// A step in a runner script.
#[derive(Debug, Clone, Copy)]
pub enum Step {
    /// `driver(initial_value, iterations)`
    Driver(c_int, c_int),
    /// `static_alias(&outer)`, reported as a canonical observation line.
    Alias(c_int),
}

impl Step {
    fn encode(&self) -> String {
        match *self {
            Step::Driver(iv, it) => format!("d:{iv}:{it}"),
            Step::Alias(v) => format!("a:{v}"),
        }
    }
}

fn run_script(so: &Path, script: &[Step]) -> Vec<u8> {
    let out = Command::new(runner_path())
        .arg(so)
        .args(script.iter().map(Step::encode))
        .output()
        .expect("spawn driver_runner");
    assert!(
        out.status.success(),
        "driver_runner failed for {}: status {:?}\nstderr: {}",
        so.display(),
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// Runs the same script against the C library and the Rust library, each in its
/// own process, and asserts the two stdout streams are byte-identical.
pub fn compare_script(script: &[Step]) {
    let c_out = run_script(&c_so_path(), script);
    let rust_out = run_script(&rust_so_path(), script);

    if c_out != rust_out {
        panic!(
            "output mismatch for script {script:?}\n--- C ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}\n--- first difference ---\n{}",
            c_out.len(),
            String::from_utf8_lossy(&c_out),
            rust_out.len(),
            String::from_utf8_lossy(&rust_out),
            first_difference(&c_out, &rust_out),
        );
    }
}

fn first_difference(a: &[u8], b: &[u8]) -> String {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return format!("byte {i}: C = {:?}, Rust = {:?}", a[i] as char, b[i] as char);
        }
    }
    format!("common prefix of {n} bytes; lengths {} vs {}", a.len(), b.len())
}

/// Convenience: a single `driver` call.
pub fn compare_driver(initial_value: c_int, iterations: c_int) {
    compare_script(&[Step::Driver(initial_value, iterations)]);
}

/// Convenience: several `driver` calls sharing one library instance, so the
/// accumulated static state is carried between them.
pub fn compare_driver_sequence(calls: &[(c_int, c_int)]) {
    let script: Vec<Step> = calls
        .iter()
        .map(|&(iv, it)| Step::Driver(iv, it))
        .collect();
    compare_script(&script);
}

// ---------------------------------------------------------------------------
// Symbol table comparison
// ---------------------------------------------------------------------------

/// Names of the dynamic symbols *defined* by a shared object
/// (`nm -D --defined-only`).
pub fn nm_defined(path: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    names.sort();
    names.dedup();
    names
}
