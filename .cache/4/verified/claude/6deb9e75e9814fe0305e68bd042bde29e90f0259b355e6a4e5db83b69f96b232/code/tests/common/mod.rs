// Shared differential-test harness.
//
// Both the C implementation (c_src/build/libdriver.so) and the Rust
// translation (target/<profile>/libdriver.so) are loaded with `libloading`
// and called ONLY through their exported `driver` symbol, exactly as an
// external C consumer would. No Rust function is ever called directly, so the
// `#[unsafe(no_mangle)] pub extern "C"` wrapper is part of what is under test.
//
// `driver` returns `void` and writes to stdout, so "comparing outputs" means
// capturing file descriptor 1 around each call and comparing the raw bytes.

#![allow(dead_code)]

use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

/// Signature of the only public entry point: `void driver(double f);`
pub type DriverFn = unsafe extern "C" fn(f64);

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    /// `fflush(NULL)` flushes *all* open output streams, which includes the
    /// stdout buffer used by whichever `.so` just ran. Both `.so`s import the
    /// same `printf@GLIBC_2.2.5`, so there is a single buffer to flush.
    fn fflush(stream: *mut c_void) -> i32;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    /// `driver` resolved from the C shared object.
    pub c: DriverFn,
    /// `driver` resolved from the Rust shared object.
    pub rust: DriverFn,
    // Keep the handles alive for the lifetime of the process so the resolved
    // function pointers stay valid.
    _c_lib: Library,
    _rust_lib: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test executable
/// (`target/<profile>/deps/<name>-<hash>`) so it is correct for any profile.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    target_profile_dir().join("libdriver.so")
}

/// `cargo test` builds the integration-test binaries but does **not** relink
/// `target/<profile>/libdriver.so`, because nothing links the `cdylib`. Without
/// this step the tests would happily compare against a stale `.so` and pass
/// vacuously after a change to `src/lib.rs`. So build it explicitly, then assert
/// it is not stale.
fn ensure_rust_so_fresh() {
    let profile_dir = target_profile_dir();
    let is_release = profile_dir.file_name().is_some_and(|n| n == "release");

    let mut cmd = std::process::Command::new(
        std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
    );
    cmd.arg("build").arg("--lib");
    if is_release {
        cmd.arg("--release");
    }
    // Propagate the feature selection the test binary was compiled with so the
    // `.so` under test matches the tests exercising it.
    cmd.arg("--no-default-features");
    for f in enabled_features() {
        cmd.arg("--features").arg(f);
    }
    cmd.current_dir(manifest_dir());
    // Keep cargo's own output off the fd we are about to capture.
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());

    match cmd.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => panic!(
            "failed to rebuild the Rust cdylib under test:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => panic!("could not invoke cargo to rebuild the cdylib: {e}"),
    }

    // Staleness guard: even if the rebuild above silently no-ops, a `.so` older
    // than the source it is supposed to implement must not be tested.
    let so = rust_so_path();
    let src = manifest_dir().join("src/lib.rs");
    if let (Ok(so_m), Ok(src_m)) = (
        std::fs::metadata(&so).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) {
        assert!(
            so_m >= src_m,
            "{} is OLDER than src/lib.rs — the tests would compare against a \
             stale library and pass vacuously. Run `cargo build` first.",
            so.display()
        );
    }
}

/// Features this test binary was compiled with, from cargo's `CARGO_FEATURE_*`
/// environment. (This crate currently declares no features, so this is empty.)
fn enabled_features() -> Vec<String> {
    let mut v: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| k.strip_prefix("CARGO_FEATURE_").map(|f| f.to_lowercase()))
        .collect();
    v.sort();
    v
}

/// Build the C shared object if it is missing or older than `driver.c`.
fn ensure_c_so_fresh() {
    let so = c_so_path();
    let src = manifest_dir().join("c_src/src/driver.c");

    let stale = match (
        std::fs::metadata(&so).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) {
        (Ok(so_m), Ok(src_m)) => so_m < src_m,
        _ => true, // missing => build it
    };
    if !stale {
        return;
    }

    let build_dir = manifest_dir().join("c_src/build");
    std::fs::create_dir_all(&build_dir).expect("create c_src/build");

    let run = |prog: &str, args: &[&str]| {
        let out = std::process::Command::new(prog)
            .args(args)
            .current_dir(&build_dir)
            .output()
            .unwrap_or_else(|e| panic!("could not run {prog}: {e}"));
        assert!(
            out.status.success(),
            "{prog} {args:?} failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run("cmake", &["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"]);
    run("cmake", &["--build", "."]);

    assert!(
        so.exists(),
        "C shared library still missing at {} after building",
        so.display()
    );
}

fn load() -> Libs {
    ensure_c_so_fresh();
    ensure_rust_so_fresh();

    let c_path = c_so_path();
    let rust_path = rust_so_path();

    assert!(
        c_path.exists(),
        "C shared library not built at {}\n\
         build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        c_path.display()
    );
    assert!(
        rust_so_path().exists(),
        "Rust shared library not built at {}\n build it with:\n  cargo build",
        rust_path.display()
    );

    unsafe {
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust_lib = Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

        // Resolve the exported symbol by name from each object. A missing
        // symbol here is itself a translation failure.
        let c_sym: Symbol<DriverFn> = c_lib
            .get(b"driver\0")
            .expect("symbol `driver` missing from the C .so");
        let rust_sym: Symbol<DriverFn> = rust_lib
            .get(b"driver\0")
            .expect("symbol `driver` missing from the Rust .so");

        let c = *c_sym;
        let rust = *rust_sym;

        Libs {
            c,
            rust,
            _c_lib: c_lib,
            _rust_lib: rust_lib,
        }
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(load)
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// `cargo test` runs test functions on parallel threads, but fd 1 is a
/// process-wide resource, so redirection must be serialized.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn unique_tmp_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        n
    ))
}

/// Redirect fd 1 to a temp file, invoke `f` once per input, then restore fd 1
/// and return the exact bytes that were written.
pub fn capture(f: DriverFn, inputs: &[f64]) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = unique_tmp_path();

    let bytes = {
        let file = fs::File::create(&path)
            .unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
        let file_fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };

        unsafe {
            // Flush anything already buffered so it does not land in our file.
            fflush(std::ptr::null_mut());

            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file_fd, 1) >= 0, "dup2 onto stdout failed");

            for &x in inputs {
                f(x);
            }

            // Force the library's buffered output out to our file *before*
            // restoring fd 1, otherwise it would be flushed to the real stdout.
            fflush(std::ptr::null_mut());

            assert!(dup2(saved, 1) >= 0, "restoring stdout failed");
            close(saved);
        }

        drop(file);
        fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    };

    let _ = fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Differential assertion
// ---------------------------------------------------------------------------

fn describe(x: f64) -> String {
    let bits = x.to_bits();
    let class = if x.is_nan() {
        let quiet = (bits >> 51) & 1 == 1;
        if quiet {
            "quiet NaN"
        } else {
            "signaling NaN"
        }
    } else if x.is_infinite() {
        "infinity"
    } else if x == 0.0 {
        "zero"
    } else if x.is_subnormal() {
        "subnormal"
    } else {
        "normal"
    };
    format!("bits=0x{bits:016x} class={class} value={x:?}")
}

/// Run the same inputs through the C `.so` and the Rust `.so` and require the
/// emitted bytes to be identical.
pub fn assert_same(row: &str, inputs: &[f64]) {
    assert!(!inputs.is_empty(), "{row}: empty input set");

    let libs = libs();
    let c_out = capture(libs.c, inputs);
    let rust_out = capture(libs.rust, inputs);

    // Guard against a vacuous pass: if the fd redirection silently failed both
    // captures would be empty and every comparison would trivially succeed.
    assert!(
        !c_out.is_empty(),
        "{row}: captured no output from the C .so — the stdout capture is broken, \
         so this comparison would be vacuous"
    );

    let c_lines: Vec<&[u8]> = split_lines(&c_out);
    let rust_lines: Vec<&[u8]> = split_lines(&rust_out);

    assert_eq!(
        c_lines.len(),
        inputs.len(),
        "{row}: C .so emitted {} lines for {} inputs (expected exactly one line \
         per call).\nIf the extra lines look like libtest progress output, the \
         capture was contaminated by a parallel test thread — run with \
         `--test-threads=1` (normally supplied automatically by \
         .cargo/config.toml's RUST_TEST_THREADS=1).",
        c_lines.len(),
        inputs.len()
    );

    if c_out != rust_out {
        // Locate the first divergence and attribute it to a specific input.
        for (i, (cl, rl)) in c_lines.iter().zip(rust_lines.iter()).enumerate() {
            if cl != rl {
                panic!(
                    "{row}: output differs at input #{i} ({})\n  C   : {:?}\n  Rust: {:?}",
                    describe(inputs[i]),
                    String::from_utf8_lossy(cl),
                    String::from_utf8_lossy(rl),
                );
            }
        }
        panic!(
            "{row}: output differs in line count: C emitted {} lines, Rust emitted {} \
             lines for {} inputs",
            c_lines.len(),
            rust_lines.len(),
            inputs.len()
        );
    }

    assert_eq!(
        rust_lines.len(),
        inputs.len(),
        "{row}: Rust .so emitted {} lines for {} inputs",
        rust_lines.len(),
        inputs.len()
    );
}

fn split_lines(buf: &[u8]) -> Vec<&[u8]> {
    // Output is exactly one `\n`-terminated line per call, so a trailing empty
    // fragment after the final newline is expected and dropped.
    let mut v: Vec<&[u8]> = buf.split(|&b| b == b'\n').collect();
    if let Some(last) = v.last() {
        if last.is_empty() {
            v.pop();
        }
    }
    v
}

/// Convenience wrapper for a single value.
pub fn assert_same_one(row: &str, x: f64) {
    assert_same(row, &[x]);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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

    /// Uniform in [0, 1).
    pub fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in [-1, 1).
    pub fn next_signed_unit(&mut self) -> f64 {
        self.next_unit() * 2.0 - 1.0
    }

    /// An arbitrary 64-bit pattern reinterpreted as a `double`. Reaches every
    /// IEEE-754 class, including non-canonical NaN payloads.
    pub fn next_bit_pattern(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    pub fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Random 52-bit mantissa field.
    pub fn next_mantissa(&mut self) -> u64 {
        self.next_u64() & 0x000F_FFFF_FFFF_FFFF
    }
}

/// Build a `double` from its IEEE-754 fields.
pub fn from_fields(sign: bool, biased_exp: u64, mantissa: u64) -> f64 {
    let bits = ((sign as u64) << 63) | ((biased_exp & 0x7FF) << 52) | (mantissa & 0x000F_FFFF_FFFF_FFFF);
    f64::from_bits(bits)
}

/// Both signs of every magnitude in `mags`.
pub fn with_both_signs(mags: &[f64]) -> Vec<f64> {
    let mut v = Vec::with_capacity(mags.len() * 2);
    for &m in mags {
        v.push(m);
        v.push(-m);
    }
    v
}

/// `x` together with `n` ULP steps either side of it.
pub fn ulp_neighbourhood(x: f64, n: i32) -> Vec<f64> {
    let mut v = vec![x];
    let mut up = x;
    let mut down = x;
    for _ in 0..n {
        up = next_after(up, f64::INFINITY);
        down = next_after(down, f64::NEG_INFINITY);
        v.push(up);
        v.push(down);
    }
    v
}

/// Minimal `nextafter` over the bit representation.
pub fn next_after(x: f64, toward: f64) -> f64 {
    if x.is_nan() || toward.is_nan() {
        return x + toward;
    }
    if x == toward {
        return toward;
    }
    if x == 0.0 {
        // Step off zero into the smallest subnormal with the target's sign.
        return if toward > 0.0 {
            f64::from_bits(1)
        } else {
            -f64::from_bits(1)
        };
    }
    let bits = x.to_bits();
    let going_up = toward > x;
    let magnitude_grows = (x > 0.0) == going_up;
    let next = if magnitude_grows {
        bits + 1
    } else {
        bits - 1
    };
    f64::from_bits(next)
}
