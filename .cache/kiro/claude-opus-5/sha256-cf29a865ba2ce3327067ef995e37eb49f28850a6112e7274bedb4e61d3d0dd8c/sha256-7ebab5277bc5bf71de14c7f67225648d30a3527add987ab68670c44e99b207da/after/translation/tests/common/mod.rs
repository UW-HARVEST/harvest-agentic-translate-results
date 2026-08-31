//! Shared harness for the differential tests.
//!
//! Both implementations are reached through their shared objects only: a helper
//! process (`examples/driver_runner.rs`) `dlopen`s the requested `.so`, looks up
//! the exported `driver` symbol with `libloading`, calls it for every input, and
//! writes the results to its stdout. Running the calls in a child process keeps
//! the captured bytes free of anything the test harness itself prints to
//! file descriptor 1, and keeps the tests safe to run in parallel.

// Each integration-test binary includes this module and uses a different part
// of it, so unused-item warnings here are expected.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use libloading::{Library, Symbol};

pub type DriverFn = unsafe extern "C" fn(f64);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The C shared library produced by `c_src/CMakeLists.txt`.
pub fn c_library_path() -> PathBuf {
    let root = workspace_root();
    for candidate in [
        "c_src/build/libdriver.so",
        "c_src/build/Release/libdriver.so",
        "c_src/build/Debug/libdriver.so",
    ] {
        let p = root.join(candidate);
        if p.exists() {
            return p;
        }
    }
    root.join("c_src/build/libdriver.so")
}

/// Directory holding the current test binary (…/target/<profile>/deps).
fn test_bin_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent().expect("deps dir").to_path_buf()
}

/// …/target/<profile>
fn profile_dir() -> PathBuf {
    let deps = test_bin_dir();
    if deps.file_name().map(|n| n == "deps").unwrap_or(false) {
        deps.parent().expect("profile dir").to_path_buf()
    } else {
        deps
    }
}

/// The Rust `cdylib` under test. Whichever profile the tests were built with is
/// preferred, so `cargo test` and `cargo test --release` both work.
pub fn rust_library_path() -> PathBuf {
    let here = profile_dir().join("libdriver.so");
    if here.exists() {
        return here;
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for candidate in ["release/libdriver.so", "debug/libdriver.so"] {
        let p = base.join(candidate);
        if p.exists() {
            return p;
        }
    }
    here
}

/// The helper process built from `examples/driver_runner.rs`.
///
/// `cargo test` builds examples automatically, but `cargo test --test <name>`
/// does not, so the helper is built on demand the first time it is missing.
fn runner_path() -> PathBuf {
    static READY: OnceLock<PathBuf> = OnceLock::new();
    READY
        .get_or_init(|| {
            let found = find_runner();
            if found.exists() {
                return found;
            }
            let profile = profile_dir();
            let release = profile.file_name().map(|n| n == "release").unwrap_or(false);
            let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or("cargo".into()));
            cmd.arg("build")
                .arg("--example")
                .arg("driver_runner")
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped());
            if release {
                cmd.arg("--release");
            }
            // Avoid inheriting the parent cargo's state, which can confuse the
            // nested invocation.
            for key in [
                "RUSTC_WORKSPACE_WRAPPER",
                "CARGO_MAKEFLAGS",
                "RUSTUP_TOOLCHAIN",
            ] {
                cmd.env_remove(key);
            }
            let status = cmd.output();
            match status {
                Ok(out) if out.status.success() => {}
                Ok(out) => eprintln!(
                    "note: could not build driver_runner automatically:\n{}",
                    String::from_utf8_lossy(&out.stderr)
                ),
                Err(e) => eprintln!("note: could not run cargo to build driver_runner: {e}"),
            }
            find_runner()
        })
        .clone()
}

fn find_runner() -> PathBuf {
    let examples = profile_dir().join("examples");
    let direct = examples.join("driver_runner");
    if direct.exists() {
        return direct;
    }
    // Cargo may only leave the hash-suffixed artefact behind; pick the newest.
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(&examples) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("driver_runner") || name.contains('.') {
                continue;
            }
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
                best = Some((mtime, path));
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or(direct)
}

fn check_prerequisites() {
    let c = c_library_path();
    assert!(
        c.exists(),
        "missing C shared library at {c:?}\n\
         build it with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    let r = rust_library_path();
    assert!(
        r.exists(),
        "missing Rust cdylib at {r:?}; run `cargo build` / `cargo build --release`"
    );
    let runner = runner_path();
    assert!(
        runner.exists(),
        "missing helper binary at {runner:?}; run `cargo build --example driver_runner`"
    );
}

/// Confirms both libraries actually export `driver` and can be `dlopen`ed.
pub fn assert_driver_symbol_exported(path: &Path) {
    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        let sym: Result<Symbol<DriverFn>, _> = lib.get(b"driver\0");
        assert!(sym.is_ok(), "{path:?} does not export `driver`");
    }
}

fn unique_tmp_path(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-diff-{}-{}-{}-{}",
        std::process::id(),
        tag,
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// Runs `library`'s exported `driver` over `inputs` in a child process and
/// returns the exact bytes it wrote to stdout.
///
/// `env` is applied to the child before it loads the library, which lets the
/// tests vary the ambient state that `printf`'s conversions depend on (see
/// `DRIVER_LOCALE` and `DRIVER_ROUNDING` in `examples/driver_runner.rs`).
pub fn run_library_with_env(
    library: &Path,
    inputs: &[f64],
    env: &[(&str, &str)],
) -> Result<Vec<u8>, String> {
    check_prerequisites();

    let mut listing = String::with_capacity(inputs.len() * 17);
    for f in inputs {
        listing.push_str(&format!("{:x}\n", f.to_bits()));
    }
    let input_path = unique_tmp_path("in");
    std::fs::write(&input_path, listing.as_bytes()).expect("write input list");

    let mut cmd = Command::new(runner_path());
    cmd.arg(library).arg(&input_path);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("spawn driver_runner");
    let _ = std::fs::remove_file(&input_path);

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "driver_runner failed for {library:?}: status {:?}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Runs `library`'s exported `driver` over `inputs` in a child process and
/// returns the exact bytes it wrote to stdout.
pub fn run_library(library: &Path, inputs: &[f64]) -> Vec<u8> {
    run_library_with_env(library, inputs, &[]).unwrap_or_else(|e| panic!("{e}"))
}

/// Bytes written by the C implementation for `inputs`.
pub fn c_run(inputs: &[f64]) -> Vec<u8> {
    run_library(&c_library_path(), inputs)
}

/// Bytes written by the Rust implementation for `inputs`.
pub fn rust_run(inputs: &[f64]) -> Vec<u8> {
    run_library(&rust_library_path(), inputs)
}

/// Bytes written by the C implementation for a single input.
pub fn c_output(f: f64) -> Vec<u8> {
    c_run(&[f])
}

/// Bytes written by the Rust implementation for a single input.
pub fn rust_output(f: f64) -> Vec<u8> {
    rust_run(&[f])
}

/// Asserts byte-identical output for a single input.
#[track_caller]
pub fn assert_same(f: f64) {
    assert_same_all([f]);
}

/// Asserts byte-identical output across a batch of inputs.
///
/// `driver` emits exactly one newline-terminated line per call, so a line index
/// in the captured stream maps straight back to the input that produced it.
#[track_caller]
pub fn assert_same_all<I: IntoIterator<Item = f64>>(inputs: I) {
    let inputs: Vec<f64> = inputs.into_iter().collect();
    if inputs.is_empty() {
        return;
    }
    let c = c_run(&inputs);
    let r = rust_run(&inputs);
    if c == r {
        // Every call must have produced exactly one line.
        assert_eq!(
            c.iter().filter(|b| **b == b'\n').count(),
            inputs.len(),
            "expected one line of output per input"
        );
        return;
    }

    let cl: Vec<&[u8]> = c.split(|b| *b == b'\n').collect();
    let rl: Vec<&[u8]> = r.split(|b| *b == b'\n').collect();
    let mut diffs = Vec::new();
    let mut total = 0usize;
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or(b"<missing>");
        let b = rl.get(i).copied().unwrap_or(b"<missing>");
        if a != b {
            total += 1;
            if diffs.len() < 20 {
                diffs.push(format!(
                    "  input {i} (bits {}):\n    C   = {:?}\n    Rust= {:?}",
                    inputs
                        .get(i)
                        .map(|f| format!("{:#018x}", f.to_bits()))
                        .unwrap_or_else(|| "?".to_string()),
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b)
                ));
            }
        }
    }

    panic!(
        "{total} of {} inputs mismatched (C {} bytes, Rust {} bytes); first {}:\n{}",
        inputs.len(),
        c.len(),
        r.len(),
        diffs.len(),
        diffs.join("\n")
    );
}

/// Compares the two implementations with extra ambient state applied to the
/// child process. Returns `Ok(())` on a match, `Err` describing the divergence
/// otherwise. A child that refuses the requested state (e.g. an uninstalled
/// locale) yields `Err` too, so callers can distinguish "unsupported" from
/// "mismatch" by inspecting the message.
pub fn compare_with_env(inputs: &[f64], env: &[(&str, &str)]) -> Result<(), String> {
    let c = run_library_with_env(&c_library_path(), inputs, env)?;
    let r = run_library_with_env(&rust_library_path(), inputs, env)?;
    if c == r {
        return Ok(());
    }
    let cl: Vec<&[u8]> = c.split(|b| *b == b'\n').collect();
    let rl: Vec<&[u8]> = r.split(|b| *b == b'\n').collect();
    let mut diffs = Vec::new();
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or(b"<missing>");
        let b = rl.get(i).copied().unwrap_or(b"<missing>");
        if a != b && diffs.len() < 10 {
            diffs.push(format!(
                "  input {i} (bits {}):\n    C   = {:?}\n    Rust= {:?}",
                inputs
                    .get(i)
                    .map(|f| format!("{:#018x}", f.to_bits()))
                    .unwrap_or_else(|| "?".to_string()),
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b)
            ));
        }
    }
    Err(format!("mismatch with env {env:?}:\n{}", diffs.join("\n")))
}

/// Whether the child refused the requested ambient state rather than producing
/// differing output.
pub fn is_unsupported(err: &str) -> bool {
    err.contains("setlocale") || err.contains("fesetround") || err.contains("rounding mode")
}

/// Same as [`assert_same_all`] but streams the inputs through in fixed-size
/// chunks, so multi-million-input sweeps stay within a bounded memory budget.
#[track_caller]
pub fn assert_same_chunked<I: IntoIterator<Item = f64>>(inputs: I, chunk: usize) {
    let chunk = chunk.max(1);
    let mut buf = Vec::with_capacity(chunk);
    for f in inputs {
        buf.push(f);
        if buf.len() == chunk {
            assert_same_all(buf.drain(..).collect::<Vec<_>>());
        }
    }
    if !buf.is_empty() {
        assert_same_all(buf);
    }
}

/// Deterministic 64-bit PRNG (splitmix64) so failures are reproducible.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
