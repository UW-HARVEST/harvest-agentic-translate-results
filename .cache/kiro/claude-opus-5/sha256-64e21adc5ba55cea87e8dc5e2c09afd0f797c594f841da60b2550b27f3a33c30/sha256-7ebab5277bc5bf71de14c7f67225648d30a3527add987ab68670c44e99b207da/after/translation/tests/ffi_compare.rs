//! Differential test: loads both the C `libdriver.so` and the Rust
//! `libdriver.so` through `libloading` and compares the exact bytes each one
//! writes to stdout for the same `driver(x, y)` inputs.
//!
//! `driver` returns `void` and communicates only via stdout, so the comparison
//! has to observe the process' output. To keep the byte stream pristine the
//! test binary re-executes *itself* (`harness = false`, so there is no libtest
//! chatter): the child is told which shared object to `dlopen` and which
//! arguments to pass, it calls the symbol, and the parent captures stdout.
//!
//! Only the exported `driver` symbol is ever called - never a Rust function
//! directly - so the `#[no_mangle]` wrapper is covered as well.

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;

const ENV_LIB: &str = "DRIVER_CHILD_LIB";
const ENV_X: &str = "DRIVER_CHILD_X";
const ENV_Y: &str = "DRIVER_CHILD_Y";

/// Wall-clock budget for a single child invocation, in seconds.
const CHILD_TIMEOUT_SECS: &str = "20";

type DriverFn = unsafe extern "C" fn(c_int, c_int);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/`, derived from the location of this test binary
/// (`target/<profile>/deps/ffi_compare-<hash>`).
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn rust_lib() -> PathBuf {
    profile_dir().join("libdriver.so")
}

/// `cargo test` builds the crate as a test/rlib target but does not necessarily
/// emit the `cdylib`, so make sure the shared object the test loads exists and
/// is up to date. `cargo build --lib` does not build any test target, so this
/// cannot recurse.
fn ensure_rust_lib() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.arg("build").arg("--lib").current_dir(manifest_dir());
    if profile_dir().file_name().and_then(|s| s.to_str()) == Some("release") {
        cmd.arg("--release");
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => eprintln!(
            "warning: `cargo build --lib` failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => eprintln!("warning: could not run cargo build: {e}"),
    }
    rust_lib()
}

fn c_lib() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so")
}

// ---------------------------------------------------------------------------
// Child mode
// ---------------------------------------------------------------------------

/// Runs inside the re-executed child: dlopen the requested library, look up
/// `driver`, call it. Everything it prints goes straight to the inherited
/// stdout pipe.
fn run_child(lib: String) -> ! {
    let x: c_int = std::env::var(ENV_X).unwrap().parse().unwrap();
    let y: c_int = std::env::var(ENV_Y).unwrap().parse().unwrap();

    // Scope the library so it is unloaded (and C stdio buffers are flushed by
    // the runtime at exit) deterministically.
    unsafe {
        let library = libloading::Library::new(&lib)
            .unwrap_or_else(|e| panic!("dlopen({lib}) failed: {e}"));
        let driver: libloading::Symbol<DriverFn> = library
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym(driver) in {lib} failed: {e}"));
        driver(x, y);
    }

    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Parent mode
// ---------------------------------------------------------------------------

struct Outcome {
    stdout: Vec<u8>,
    status: Option<i32>,
}

fn invoke(lib: &Path, x: c_int, y: c_int) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    // `timeout` guards against a translation bug that turns a terminating loop
    // into an infinite one; without it a single case could hang the suite.
    let out = Command::new("timeout")
        .arg(CHILD_TIMEOUT_SECS)
        .arg(&exe)
        .env(ENV_LIB, lib)
        .env(ENV_X, x.to_string())
        .env(ENV_Y, y.to_string())
        .output()
        .expect("spawn child");

    if !out.stderr.is_empty() {
        eprintln!(
            "[child stderr] {} ({x}, {y}): {}",
            lib.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    Outcome {
        stdout: out.stdout,
        status: out.status.code(),
    }
}

fn describe(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

/// Compares C vs Rust for one input pair. Returns `Ok(())` or a description of
/// the mismatch.
fn compare(x: c_int, y: c_int) -> Result<(), String> {
    let c = invoke(&c_lib(), x, y);
    let r = invoke(&rust_lib(), x, y);

    if c.status != Some(0) {
        return Err(format!(
            "C child for ({x}, {y}) exited with {:?} (124 = timeout)",
            c.status
        ));
    }
    if r.status != Some(0) {
        return Err(format!(
            "Rust child for ({x}, {y}) exited with {:?} (124 = timeout)",
            r.status
        ));
    }
    if c.stdout != r.stdout {
        // Locate the first divergence to make the report actionable.
        let at = c
            .stdout
            .iter()
            .zip(r.stdout.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c.stdout.len().min(r.stdout.len()));
        return Err(format!(
            "driver({x}, {y}) output differs at byte {at}\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
            c.stdout.len(),
            describe(&c.stdout),
            r.stdout.len(),
            describe(&r.stdout),
        ));
    }
    Ok(())
}

/// Input pairs that make `driver` terminate.
///
/// The C original loops forever whenever `x > 0 && y < 0`: the `y == 0` test
/// never fires, so `y` is decremented without bound. Those inputs are excluded
/// - the reference implementation has no observable output to match. Every
/// other combination in the sweep terminates.
fn terminates(x: c_int, y: c_int) -> bool {
    !(x > 0 && y < 0)
}

fn sweep_cases() -> Vec<(c_int, c_int)> {
    let mut cases = Vec::new();

    // Dense sweep around zero and around the `x == 1 && y == 4` special case,
    // including negatives to cover the loop-guard and `x > 0` / `y == 0` edges.
    for x in -4..=14 {
        for y in -4..=14 {
            if terminates(x, y) {
                cases.push((x, y));
            }
        }
    }

    // Hand-picked interesting points: the `goto label2` trigger and its
    // neighbours, the `x < 3` backward-jump threshold, larger magnitudes.
    for &pair in &[
        (1, 4),
        (0, 4),
        (2, 4),
        (1, 3),
        (1, 5),
        (3, 3),
        (3, 4),
        (4, 3),
        (2, 2),
        (17, 5),
        (5, 17),
        (33, 31),
        (64, 64),
        (100, 7),
        (7, 100),
        (128, 129),
        (-100, 100),
        (0, 250),
        (250, 0),
        (300, 300),
        (0, -1),
        (-1, 0),
        (-1, -1),
        (0, 0),
        (i32::MIN, 0),
        (i32::MIN, 5),
        (0, i32::MIN),
        (-5, i32::MIN),
    ] {
        if terminates(pair.0, pair.1) {
            cases.push(pair);
        }
    }

    cases.sort_unstable();
    cases.dedup();
    cases
}

/// Every dynamic symbol the C `.so` defines must also be defined by the Rust
/// `.so`, under the identical name.
fn compare_exports() -> Result<(), String> {
    fn defined_symbols(lib: &Path) -> Result<Vec<String>, String> {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(lib)
            .output()
            .map_err(|e| format!("running nm on {}: {e}", lib.display()))?;
        if !out.status.success() {
            return Err(format!(
                "nm -D {} failed: {}",
                lib.display(),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut syms: Vec<String> = text
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let (_addr, kind, name) = (parts.next()?, parts.next()?, parts.next()?);
                // Code / data definitions, ignoring linker bookkeeping.
                matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "w")
                    .then(|| name.to_string())
            })
            .filter(|n| !n.starts_with("_") && n != "driver_LTX_preloaded_symbols")
            .collect();
        syms.sort();
        syms.dedup();
        Ok(syms)
    }

    let c_syms = defined_symbols(&c_lib())?;
    let rust_syms = defined_symbols(&rust_lib())?;

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    if !missing.is_empty() {
        return Err(format!(
            "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C   : {c_syms:?}\n  Rust: {rust_syms:?}"
        ));
    }
    println!("  exports checked: C defines {:?}", c_syms);
    Ok(())
}

fn main() {
    if let Ok(lib) = std::env::var(ENV_LIB) {
        run_child(lib);
    }

    let c = c_lib();
    let r = ensure_rust_lib();
    assert!(
        c.exists(),
        "C shared library not found at {} - build it with cmake first",
        c.display()
    );
    assert!(
        r.exists(),
        "Rust shared library not found at {} - `cargo build` should produce it",
        r.display()
    );
    println!("C   lib: {}", c.display());
    println!("Rust lib: {}", r.display());

    let mut failures: Vec<String> = Vec::new();

    println!("\ntest exported_symbols_match ...");
    match compare_exports() {
        Ok(()) => println!("ok"),
        Err(e) => {
            println!("FAILED");
            failures.push(e);
        }
    }

    let cases = sweep_cases();
    println!("\ntest driver_matches_c ... ({} input pairs)", cases.len());
    let mut checked = 0usize;
    for (x, y) in cases {
        match compare(x, y) {
            Ok(()) => checked += 1,
            Err(e) => failures.push(e),
        }
    }
    println!("  {checked} pairs matched byte-for-byte");

    if failures.is_empty() {
        println!("\ntest result: ok. all comparisons matched");
    } else {
        println!("\ntest result: FAILED. {} problem(s):", failures.len());
        for f in &failures {
            println!("---\n{f}");
        }
        std::process::exit(1);
    }
}
