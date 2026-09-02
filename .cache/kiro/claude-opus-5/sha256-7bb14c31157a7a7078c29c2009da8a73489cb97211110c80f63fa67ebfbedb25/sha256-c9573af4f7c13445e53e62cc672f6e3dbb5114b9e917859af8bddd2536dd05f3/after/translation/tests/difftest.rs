//! Differential test harness: C `libdriver.so` vs Rust `libdriver.so`.
//!
//! Both libraries are loaded with `libloading` and invoked *only* through their
//! exported `driver` symbol. The Rust function is never called directly, so the
//! `#[unsafe(no_mangle)] extern "C"` wrapper is exercised exactly as an external
//! consumer would exercise it.
//!
//! Structure: this binary is both the test driver and its own child process.
//! `driver` can terminate the process with `SIGFPE` (see `ERRORS.md`), which is
//! unobservable from inside a normal test, so every case is run in a freshly
//! spawned child (`std::env::current_exe()` + `DIFF_CHILD_LIB`) with stdout
//! captured as a pipe. The parent compares raw stdout bytes and the exact exit
//! status / termination signal.

use std::env;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// child mode
// ---------------------------------------------------------------------------

/// Child entry point: load the `.so` named by `DIFF_CHILD_LIB`, then call
/// `driver(x, y)` for every `x,y` pair given on stdin as `x y` lines.
fn child_main(lib_path: String) -> ! {
    use std::io::Read;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("child: read stdin");

    // Loaded, and kept alive, for the whole child lifetime.
    let lib = unsafe { libloading::Library::new(&lib_path) }
        .unwrap_or_else(|e| panic!("child: dlopen {lib_path}: {e}"));
    let driver: libloading::Symbol<unsafe extern "C" fn(std::ffi::c_int, std::ffi::c_int)> =
        unsafe { lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("child: dlsym driver in {lib_path}: {e}"));

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let x: i32 = it.next().expect("child: missing x").parse().expect("child: x");
        let y: i32 = it.next().expect("child: missing y").parse().expect("child: y");
        unsafe { driver(x, y) };
    }

    // Flush the C `stdout` FILE stream (printf's buffer), not Rust's.
    unsafe { libc_fflush_all() };
    std::process::exit(0);
}

unsafe extern "C" {
    #[link_name = "fflush"]
    fn c_fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
}

/// `fflush(NULL)` — flush every open output stream.
unsafe fn libc_fflush_all() {
    unsafe { c_fflush(std::ptr::null_mut()) };
}

// ---------------------------------------------------------------------------
// parent: running a batch of pairs against one library
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct RunResult {
    stdout: Vec<u8>,
    stderr_len: usize,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for RunResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RunResult {{ code: {:?}, signal: {:?}, stdout: {:?} }}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout)
        )
    }
}

fn run_batch(self_exe: &Path, lib: &Path, pairs: &[(i32, i32)]) -> RunResult {
    let mut input = String::with_capacity(pairs.len() * 24);
    for (x, y) in pairs {
        input.push_str(&format!("{x} {y}\n"));
    }

    let mut child = Command::new(self_exe)
        .env("DIFF_CHILD_LIB", lib)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn child");
    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(input.as_bytes())
        .ok(); // a child that dies on SIGFPE may close the pipe early (EPIPE)
    let out = child.wait_with_output().expect("wait child");

    RunResult {
        stdout: out.stdout,
        stderr_len: out.stderr.len(),
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (fixed seed -> reproducible rows)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn i32_full(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    /// Non-zero `i32`, full range.
    fn i32_nonzero(&mut self) -> i32 {
        loop {
            let v = self.i32_full();
            if v != 0 {
                return v;
            }
        }
    }
}

const SEED: u64 = 0x5EED_1234_ABCD;
const I32_MIN: i32 = i32::MIN;
const I32_MAX: i32 = i32::MAX;

/// Would this pair make the C library die? (`ERRORS.md` rows 1-3.)
fn is_fatal(x: i32, y: i32) -> bool {
    y == 0 || (x == I32_MIN && y == -1)
}

// ---------------------------------------------------------------------------
// row builders (Phase B — CONFIGS.md)
// ---------------------------------------------------------------------------

const N: usize = 400;

fn row_pairs(row: usize, rng: &mut Rng) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    match row {
        // 1: x == 0, y == 1
        1 => v.push((0, 1)),
        // 2: x == 0, y random non-zero
        2 => {
            for _ in 0..N {
                v.push((0, rng.i32_nonzero()));
            }
        }
        // 3: x > 0, y > 0, exactly divisible
        3 => {
            for _ in 0..N {
                let y = rng.range(1, 46_340) as i32; // 46340^2 < I32_MAX
                let k = rng.range(1, (I32_MAX as i64) / y as i64) as i32;
                v.push((y.wrapping_mul(k), y));
            }
        }
        // 4: x > 0, y > 0, not divisible
        4 => {
            let mut n = 0;
            while n < N {
                let y = rng.range(2, I32_MAX as i64) as i32;
                let x = rng.range(1, I32_MAX as i64) as i32;
                if x % y != 0 {
                    v.push((x, y));
                    n += 1;
                }
            }
        }
        // 5: x > 0, y < 0
        5 => {
            for _ in 0..N {
                v.push((
                    rng.range(1, I32_MAX as i64) as i32,
                    rng.range(I32_MIN as i64, -1) as i32,
                ));
            }
        }
        // 6: x < 0, y > 0
        6 => {
            for _ in 0..N {
                v.push((
                    rng.range(I32_MIN as i64, -1) as i32,
                    rng.range(1, I32_MAX as i64) as i32,
                ));
            }
        }
        // 7: x < 0, y < 0  (skip the INT_MIN/-1 fatal pair)
        7 => {
            let mut n = 0;
            while n < N {
                let x = rng.range(I32_MIN as i64, -1) as i32;
                let y = rng.range(I32_MIN as i64, -1) as i32;
                if !is_fatal(x, y) {
                    v.push((x, y));
                    n += 1;
                }
            }
        }
        // 8: y == 1, x full range
        8 => {
            v.push((I32_MIN, 1));
            v.push((I32_MAX, 1));
            for _ in 0..N {
                v.push((rng.i32_full(), 1));
            }
        }
        // 9: y == -1, x full range excluding INT_MIN
        9 => {
            v.push((I32_MIN + 1, -1));
            v.push((I32_MAX, -1));
            for _ in 0..N {
                let x = rng.i32_full();
                v.push((if x == I32_MIN { I32_MIN + 1 } else { x }, -1));
            }
        }
        // 10: |x| < |y|, all four sign quadrants
        10 => {
            for _ in 0..N {
                let mag_y = rng.range(2, I32_MAX as i64);
                let mag_x = rng.range(0, mag_y - 1);
                let sx = if rng.next_u64() & 1 == 0 { 1 } else { -1 };
                let sy = if rng.next_u64() & 1 == 0 { 1 } else { -1 };
                v.push(((mag_x * sx) as i32, (mag_y * sy) as i32));
            }
        }
        // 11: x == INT_MAX, y random non-zero
        11 => {
            for _ in 0..N {
                v.push((I32_MAX, rng.i32_nonzero()));
            }
        }
        // 12: x == INT_MIN (y != 0, != -1), plus INT_MIN+1 / INT_MAX-1 neighbours
        12 => {
            for _ in 0..N {
                let mut y = rng.i32_nonzero();
                if y == -1 {
                    y = -2;
                }
                v.push((I32_MIN, y));
            }
            for _ in 0..N / 4 {
                let y = rng.i32_nonzero();
                v.push((I32_MIN + 1, y));
                v.push((I32_MAX - 1, y));
            }
        }
        // 13: y == INT_MAX / INT_MIN, x full range
        13 => {
            for _ in 0..N {
                v.push((rng.i32_full(), I32_MAX));
                v.push((rng.i32_full(), I32_MIN));
            }
            v.push((I32_MIN, I32_MIN));
            v.push((I32_MAX, I32_MAX));
            v.push((I32_MIN, I32_MAX));
            v.push((I32_MAX, I32_MIN));
        }
        // 14: bulk uniform full-range sweep
        14 => {
            let mut n = 0;
            while n < 4000 {
                let x = rng.i32_full();
                let y = rng.i32_full();
                if !is_fatal(x, y) {
                    v.push((x, y));
                    n += 1;
                }
            }
        }
        // 15: small magnitudes, dense near zero
        15 => {
            for x in -64i32..=64 {
                for y in -64i32..=64 {
                    if y != 0 {
                        v.push((x, y));
                    }
                }
            }
        }
        // 16: |y| an exact power of two, x full range
        16 => {
            for p in 0..=30u32 {
                let mag = 1i32 << p;
                for sign in [1i32, -1] {
                    let y = mag * sign;
                    for _ in 0..8 {
                        let x = rng.i32_full();
                        if !is_fatal(x, y) {
                            v.push((x, y));
                        }
                    }
                    v.push((I32_MAX, y));
                    if !is_fatal(I32_MIN, y) {
                        v.push((I32_MIN, y));
                    }
                }
            }
        }
        // 17: exhaustive extremal grid
        17 => {
            let vals = [
                I32_MIN,
                I32_MIN + 1,
                -2,
                -1,
                0,
                1,
                2,
                I32_MAX - 1,
                I32_MAX,
            ];
            for &x in &vals {
                for &y in &vals {
                    if !is_fatal(x, y) {
                        v.push((x, y));
                    }
                }
            }
        }
        // 18: maximum-width printf output
        18 => {
            v.push((I32_MIN, 1)); // quotient "-2147483648"
            v.push((I32_MIN + 1, I32_MAX)); // remainder "-2147483647"
            v.push((I32_MAX, 1)); // quotient "2147483647"
            v.push((I32_MAX, I32_MIN)); // remainder "2147483647"
            v.push((I32_MIN, I32_MIN)); // quotient 1, remainder 0
        }
        // 19: many interleaved calls in one process
        19 => {
            for _ in 0..1000 {
                let (x, y) = match rng.next_u64() % 4 {
                    0 => (rng.i32_full(), 1),
                    1 => (rng.range(-9, 9) as i32, rng.range(1, 9) as i32),
                    2 => (I32_MIN, rng.range(2, 100) as i32),
                    _ => (rng.i32_full(), rng.i32_nonzero()),
                };
                if !is_fatal(x, y) {
                    v.push((x, y));
                }
            }
        }
        _ => unreachable!("unknown row {row}"),
    }
    v
}

// ---------------------------------------------------------------------------
// parent driver
// ---------------------------------------------------------------------------

struct Report {
    passed: usize,
    failed: Vec<String>,
}

impl Report {
    fn check(&mut self, name: &str, ok: bool, detail: impl FnOnce() -> String) {
        if ok {
            self.passed += 1;
            println!("  PASS  {name}");
        } else {
            let d = detail();
            println!("  FAIL  {name}\n        {d}");
            self.failed.push(format!("{name}: {d}"));
        }
    }
}

fn locate_libs() -> (PathBuf, Vec<PathBuf>) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = manifest
        .parent()
        .expect("workspace parent")
        .join("c_src/build/libdriver.so");
    assert!(
        c_so.is_file(),
        "C shared library not found at {}; build it with \
         `cd c_src && mkdir -p build && cd build && cmake .. \
         -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`",
        c_so.display()
    );

    let mut rust_sos = Vec::new();
    for profile in ["debug", "release"] {
        let p = manifest.join("target").join(profile).join("libdriver.so");
        if p.is_file() {
            rust_sos.push(p);
        }
    }
    assert!(
        !rust_sos.is_empty(),
        "no Rust libdriver.so found under {}/target/{{debug,release}}",
        manifest.display()
    );
    (c_so, rust_sos)
}

fn main() {
    if let Ok(lib) = env::var("DIFF_CHILD_LIB") {
        child_main(lib);
    }

    let self_exe = env::current_exe().expect("current_exe");
    let (c_so, rust_sos) = locate_libs();

    println!("C  library : {}", c_so.display());
    for r in &rust_sos {
        println!("Rust lib   : {}", r.display());
    }

    let mut report = Report {
        passed: 0,
        failed: Vec::new(),
    };

    for rust_so in &rust_sos {
        let profile = rust_so
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        println!("\n=== Rust profile: {profile} ===");

        // ---------------- Phase B: CONFIGS.md rows 1..=19 ----------------
        println!("\n-- Phase B: valid-path differential (CONFIGS.md) --");
        for row in 1..=19usize {
            // Same seed per row for both libraries: identical input vectors.
            let pairs = row_pairs(row, &mut Rng::new(SEED.wrapping_add(row as u64)));
            let c = run_batch(&self_exe, &c_so, &pairs);
            let r = run_batch(&self_exe, rust_so, &pairs);
            let name = format!("configs_row_{row:02}[{profile}] ({} inputs)", pairs.len());
            report.check(
                &name,
                c.stdout == r.stdout && c.code == r.code && c.signal == r.signal,
                || {
                    let mismatch = c
                        .stdout
                        .iter()
                        .zip(r.stdout.iter())
                        .position(|(a, b)| a != b);
                    format!(
                        "status C={:?}/{:?} Rust={:?}/{:?}; stdout {} vs {} bytes; \
                         first byte mismatch at {:?}\n        C   ...{}\n        Rust...{}",
                        c.code,
                        c.signal,
                        r.code,
                        r.signal,
                        c.stdout.len(),
                        r.stdout.len(),
                        mismatch,
                        excerpt(&c.stdout, mismatch),
                        excerpt(&r.stdout, mismatch),
                    )
                },
            );
        }

        // ---------------- Phase C: ERRORS.md rows 1..=3 ----------------
        println!("\n-- Phase C: error-path differential (ERRORS.md) --");
        let error_rows: [(&str, i32, i32); 3] = [
            ("error_row_1_div_by_zero_nonzero_numer", 5, 0),
            ("error_row_2_zero_over_zero", 0, 0),
            ("error_row_3_int_min_over_minus_one", I32_MIN, -1),
        ];
        for (name, x, y) in error_rows {
            let pairs = [(x, y)];
            let c = run_batch(&self_exe, &c_so, &pairs);
            let r = run_batch(&self_exe, rust_so, &pairs);
            // Same *specific* signal, not merely "both failed".
            let ok = c.signal == r.signal
                && c.signal == Some(8) // SIGFPE
                && c.code == r.code
                && c.stdout == r.stdout
                && c.stdout.is_empty();
            report.check(&format!("{name}[{profile}]"), ok, || {
                format!(
                    "expected both to die with SIGFPE(8) and print nothing; \
                     got C={:?} Rust={:?}",
                    c, r
                )
            });
        }

        // Extra error-adjacent boundaries: y == 0 across a spread of numerators,
        // and INT_MIN/-1 neighbours that must NOT fault.
        for x in [I32_MIN, I32_MIN + 1, -1, 0, 1, I32_MAX - 1, I32_MAX] {
            let pairs = [(x, 0)];
            let c = run_batch(&self_exe, &c_so, &pairs);
            let r = run_batch(&self_exe, rust_so, &pairs);
            report.check(
                &format!("error_div_by_zero_x_{x}[{profile}]"),
                c.signal == Some(8) && r.signal == Some(8) && c.stdout == r.stdout,
                || format!("C={c:?} Rust={r:?}"),
            );
        }
        for (x, y) in [
            (I32_MIN + 1, -1),
            (I32_MIN, -2),
            (I32_MIN, 1),
            (I32_MAX, -1),
            (-1, -1),
        ] {
            let pairs = [(x, y)];
            let c = run_batch(&self_exe, &c_so, &pairs);
            let r = run_batch(&self_exe, rust_so, &pairs);
            report.check(
                &format!("error_nonfatal_neighbour_{x}_{y}[{profile}]"),
                c.signal.is_none() && r.signal.is_none() && c.stdout == r.stdout,
                || format!("C={c:?} Rust={r:?}"),
            );
        }

        // Batch that faults partway through: the fault must occur after the
        // same prefix of output in both libraries.
        {
            let pairs = [(9, 4), (-9, 4), (7, 0), (1, 1)];
            let c = run_batch(&self_exe, &c_so, &pairs);
            let r = run_batch(&self_exe, rust_so, &pairs);
            report.check(
                &format!("error_fault_midbatch[{profile}]"),
                c.signal == Some(8) && r.signal == Some(8) && c.stdout == r.stdout,
                || format!("C={c:?} Rust={r:?}"),
            );
        }

        // Full-domain "no valid variant" analogue for the two int parameters:
        // exhaustive sweep of one parameter over a contiguous window while the
        // other is extremal, catching value-dependent divergence.
        {
            let mut pairs = Vec::new();
            for y in -300i32..=300 {
                if y != 0 {
                    pairs.push((I32_MIN, y));
                    pairs.push((I32_MAX, y));
                }
            }
            pairs.retain(|&(x, y)| !is_fatal(x, y));
            let c = run_batch(&self_exe, &c_so, &pairs);
            let r = run_batch(&self_exe, rust_so, &pairs);
            report.check(
                &format!("extremal_numer_dense_denom_sweep[{profile}] ({} inputs)", pairs.len()),
                c.stdout == r.stdout && c.code == r.code && c.signal == r.signal,
                || format!("C={:?}/{:?} Rust={:?}/{:?}", c.code, c.signal, r.code, r.signal),
            );
        }
    }

    println!("\n=====================================================");
    println!("passed: {}, failed: {}", report.passed, report.failed.len());
    if !report.failed.is_empty() {
        println!("\nFAILURES:");
        for f in &report.failed {
            println!("  - {f}");
        }
        std::process::exit(1);
    }
    println!("ALL DIFFERENTIAL TESTS PASSED");
}

fn excerpt(buf: &[u8], at: Option<usize>) -> String {
    let at = at.unwrap_or(0);
    let start = at.saturating_sub(40);
    let end = (at + 40).min(buf.len());
    String::from_utf8_lossy(&buf[start..end]).replace('\n', "\\n")
}
