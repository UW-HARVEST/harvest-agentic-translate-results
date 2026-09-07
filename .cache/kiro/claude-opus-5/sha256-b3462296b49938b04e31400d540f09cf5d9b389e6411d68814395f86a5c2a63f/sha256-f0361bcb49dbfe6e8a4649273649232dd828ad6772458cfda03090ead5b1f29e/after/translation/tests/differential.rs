//! Differential test: C `libdriver.so` vs Rust `libdriver.so`, both loaded with
//! `libloading` and called only through their exported `extern "C"` symbols.
//!
//! Why this test target uses `harness = false`:
//!
//! The C library keeps `static house_t the_house` at file scope and mutates it on
//! every call, with no reset path. Output therefore depends on the whole call
//! history of the process. To make a configuration reproducible we run it in a
//! freshly `exec`'d process: this binary re-executes itself in "worker" mode,
//! where it `dlopen`s exactly one of the two libraries, performs a scripted call
//! sequence, and lets the library's own `printf` write to the inherited stdout
//! pipe. The parent then compares the two byte streams. No fd juggling, no
//! shared state, no ordering dependence between rows.
//!
//! Both libraries import `printf@GLIBC` (see SYMBOLS.md), so the comparison
//! covers `%d` / `%.1f` formatting produced by the same glibc in both cases.

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Library location
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so` produced by `c_src/CMakeLists.txt`.
fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\ntried: {candidates:?}"
    );
}

/// The Rust `cdylib`. Located next to the test executable (`target/<profile>/`),
/// which keeps debug and release runs from picking up each other's artifact.
///
/// `cargo test` does not build the `cdylib` artifact of the crate under test
/// (nothing links it, since we deliberately go through `dlopen` instead), so if
/// it is absent we build it here rather than failing.
fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| if p.file_name().is_some_and(|n| n == "deps") { p.parent() } else { Some(p) })
        .expect("profile dir")
        .to_path_buf();
    let cand = profile_dir.join("libdriver.so");
    if cand.exists() {
        return cand;
    }
    let release = profile_dir.file_name().is_some_and(|n| n == "release");
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.arg("build").current_dir(manifest_dir());
    if release {
        cmd.arg("--release");
    }
    let st = cmd.status().expect("cargo build for the cdylib");
    assert!(st.success(), "`cargo build` for the cdylib failed");
    assert!(cand.exists(), "cdylib still missing after cargo build: {}", cand.display());
    cand
}

// ---------------------------------------------------------------------------
// Call scripting
// ---------------------------------------------------------------------------

/// One scripted call. `wide` selects the ABI shape: `false` calls the symbol
/// through `extern "C" fn(c_int)`, `true` through `extern "C" fn(i64)` so that a
/// value with a dirty high half is handed to a callee that declares `int`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Call {
    entry: Entry,
    value: i64,
    wide: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Run,
    Driver,
}

impl Entry {
    fn symbol(self) -> &'static [u8] {
        match self {
            Entry::Run => b"run\0",
            Entry::Driver => b"driver\0",
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Entry::Run => "run",
            Entry::Driver => "driver",
        }
    }
}

fn run_call(v: i32) -> Call {
    Call { entry: Entry::Run, value: v as i64, wide: false }
}
fn driver_call(v: i32) -> Call {
    Call { entry: Entry::Driver, value: v as i64, wide: false }
}
fn run_wide(v: i64) -> Call {
    Call { entry: Entry::Run, value: v, wide: true }
}
fn driver_wide(v: i64) -> Call {
    Call { entry: Entry::Driver, value: v, wide: true }
}

fn encode(seq: &[Call]) -> Vec<String> {
    seq.iter()
        .map(|c| format!("{}{}:{}", c.entry.tag(), if c.wide { "w" } else { "" }, c.value))
        .collect()
}

fn decode(args: &[String]) -> Vec<Call> {
    args.iter()
        .map(|a| {
            let (name, val) = a.split_once(':').expect("malformed call spec");
            let (tag, wide) = match name.strip_suffix('w') {
                Some(t) => (t, true),
                None => (name, false),
            };
            let entry = match tag {
                "run" => Entry::Run,
                "driver" => Entry::Driver,
                other => panic!("unknown entry point {other:?}"),
            };
            Call { entry, value: val.parse().expect("bad value"), wide }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Worker: dlopen one library, execute the sequence, let it print to stdout
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

fn worker(lib_path: &str, seq: &[Call]) -> ! {
    // SAFETY: we load a known-good shared library and call symbols whose C
    // signatures are `void run(int)` / `void driver(int)`.
    unsafe {
        let lib = libloading::Library::new(lib_path)
            .unwrap_or_else(|e| panic!("dlopen {lib_path}: {e}"));
        for call in seq {
            if call.wide {
                let f: libloading::Symbol<unsafe extern "C" fn(i64)> =
                    lib.get(call.entry.symbol()).expect("symbol");
                f(call.value);
            } else {
                let f: libloading::Symbol<unsafe extern "C" fn(c_int)> =
                    lib.get(call.entry.symbol()).expect("symbol");
                f(call.value as c_int);
            }
        }
        // Flush every stdio stream (NULL == all) so the library's buffered
        // output reaches the pipe before the process dies.
        fflush(std::ptr::null_mut());
        std::process::exit(0);
    }
}

// ---------------------------------------------------------------------------
// Parent: spawn one worker per library and diff the bytes
// ---------------------------------------------------------------------------

struct Harness {
    exe: PathBuf,
    c: PathBuf,
    rust: PathBuf,
    passed: usize,
    failed: Vec<String>,
}

impl Harness {
    /// Run the scripted sequence against one library in a pristine process.
    /// Returns `None` (and records a failure) if the worker did not exit 0 --
    /// e.g. a missing export, or a Rust panic/abort where the C simply wraps.
    fn spawn_checked(&mut self, row: &str, lib: &Path, seq: &[Call]) -> Option<Vec<u8>> {
        let out = Command::new(&self.exe)
            .arg("--worker")
            .arg(lib)
            .args(encode(seq))
            .output()
            .expect("spawn worker");
        if out.status.success() {
            return Some(out.stdout);
        }
        let msg = format!(
            "{row}: worker for {} exited with {:?}\n  sequence: {:?}\n  stderr:\n{}",
            lib.display(),
            out.status,
            encode(seq),
            String::from_utf8_lossy(&out.stderr),
        );
        eprintln!("FAIL {msg}");
        self.failed.push(msg);
        None
    }

    /// Infallible variant for the internal cross-checks that have already had
    /// their worker exit status validated by a preceding `check`.
    fn spawn(&self, lib: &Path, seq: &[Call]) -> Vec<u8> {
        let out = Command::new(&self.exe)
            .arg("--worker")
            .arg(lib)
            .args(encode(seq))
            .output()
            .expect("spawn worker");
        assert!(
            out.status.success(),
            "worker for {} exited with {:?}\nstderr:\n{}",
            lib.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    }

    /// Run one configuration against both libraries and require byte equality.
    fn check(&mut self, row: &str, detail: &str, seq: &[Call]) {
        let c_path = self.c.clone();
        let r_path = self.rust.clone();
        let Some(c_out) = self.spawn_checked(row, &c_path, seq) else { return };
        let Some(r_out) = self.spawn_checked(row, &r_path, seq) else { return };
        if c_out == r_out {
            self.passed += 1;
            return;
        }
        let msg = format!(
            "{row}: {detail}\n  sequence: {:?}\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}\n  first diff at byte {}",
            encode(seq),
            c_out.len(),
            String::from_utf8_lossy(&c_out),
            r_out.len(),
            String::from_utf8_lossy(&r_out),
            c_out
                .iter()
                .zip(r_out.iter())
                .position(|(a, b)| a != b)
                .map(|p| p.to_string())
                .unwrap_or_else(|| format!("length only ({} vs {})", c_out.len(), r_out.len())),
        );
        eprintln!("FAIL {msg}");
        self.failed.push(msg);
    }

    /// Same as `check`, but also asserts stdout is non-empty -- guards against a
    /// "both printed nothing, so both match" false pass.
    fn check_nonempty(&mut self, row: &str, detail: &str, seq: &[Call]) {
        let before = self.failed.len();
        self.check(row, detail, seq);
        if self.failed.len() == before {
            let c_path = self.c.clone();
            if self.spawn(&c_path, seq).is_empty() {
                self.failed.push(format!("{row}: {detail}: C produced no output at all"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Rng(0x5eed_1234_5678_9abc)
    }
    fn next_u64(&mut self) -> u64 {
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
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md rows
// ---------------------------------------------------------------------------

const I32_MAX: i32 = i32::MAX;
const I32_MIN: i32 = i32::MIN;

fn phase_b(h: &mut Harness, rng: &mut Rng) {
    // Row 1-3: fresh state, trivial values, low-level entry point.
    h.check_nonempty("CONFIGS row 1", "run(0), fresh state", &[run_call(0)]);
    h.check_nonempty("CONFIGS row 2", "run(1), fresh state", &[run_call(1)]);
    h.check_nonempty("CONFIGS row 3", "run(-1), fresh state", &[run_call(-1)]);

    // Row 4-5: extreme magnitudes.
    h.check_nonempty("CONFIGS row 4", "run(INT_MAX), bedroom add overflows", &[run_call(I32_MAX)]);
    h.check_nonempty("CONFIGS row 5", "run(INT_MIN)", &[run_call(I32_MIN)]);

    // Row 6: exact overflow boundary of `5 + extra_bedrooms`.
    h.check_nonempty("CONFIGS row 6a", "run(INT_MAX-5): last non-wrapping", &[run_call(I32_MAX - 5)]);
    h.check_nonempty("CONFIGS row 6b", "run(INT_MAX-4): first wrapping", &[run_call(I32_MAX - 4)]);

    // Row 7: bedrooms lands exactly on 0 / -1.
    h.check_nonempty("CONFIGS row 7a", "run(-5): bedrooms -> 0", &[run_call(-5)]);
    h.check_nonempty("CONFIGS row 7b", "run(-6): bedrooms -> -1", &[run_call(-6)]);

    // Row 8: 64 randomized full-range values, each in a pristine process.
    for i in 0..64 {
        let v = rng.i32_full();
        h.check(&format!("CONFIGS row 8[{i}]"), &format!("run({v}) fresh"), &[run_call(v)]);
    }

    // Row 9: 64 randomized small values, each in a pristine process.
    for i in 0..64 {
        let v = rng.range(-1000, 1000);
        h.check(&format!("CONFIGS row 9[{i}]"), &format!("run({v}) fresh"), &[run_call(v)]);
    }

    // Row 10-13: the convenience wrapper, which runs the pass twice.
    h.check_nonempty("CONFIGS row 10", "driver(0)", &[driver_call(0)]);
    h.check_nonempty("CONFIGS row 11a", "driver(1)", &[driver_call(1)]);
    h.check_nonempty("CONFIGS row 11b", "driver(-1)", &[driver_call(-1)]);
    h.check_nonempty("CONFIGS row 12", "driver(INT_MAX), compounded overflow", &[driver_call(I32_MAX)]);
    h.check_nonempty("CONFIGS row 13", "driver(INT_MIN), compounded underflow", &[driver_call(I32_MIN)]);

    // Row 14: 64 randomized full-range values through `driver`.
    for i in 0..64 {
        let v = rng.i32_full();
        h.check(&format!("CONFIGS row 14[{i}]"), &format!("driver({v}) fresh"), &[driver_call(v)]);
    }

    // Row 15: composed-pipeline cross-check. `run(v); run(v)` must be
    // byte-identical to `driver(v)` in BOTH libraries, and the two libraries
    // must agree with each other.
    for i in 0..16 {
        let v = if i < 4 { [0, 1, I32_MAX, I32_MIN][i] } else { rng.i32_full() };
        let two_runs = [run_call(v), run_call(v)];
        let one_driver = [driver_call(v)];
        let row = format!("CONFIGS row 15[{i}]");
        h.check(&row, &format!("run({v}) twice, accumulated"), &two_runs);
        let c_path = h.c.clone();
        let r_path = h.rust.clone();
        let Some(c_runs) = h.spawn_checked(&row, &c_path, &two_runs) else { continue };
        let Some(c_drv) = h.spawn_checked(&row, &c_path, &one_driver) else { continue };
        let Some(r_runs) = h.spawn_checked(&row, &r_path, &two_runs) else { continue };
        let Some(r_drv) = h.spawn_checked(&row, &r_path, &one_driver) else { continue };
        if c_runs != c_drv {
            h.failed.push(format!(
                "CONFIGS row 15[{i}]: C's own run(v);run(v) != driver(v) -- test model is wrong"
            ));
        } else if r_runs != r_drv {
            h.failed.push(format!(
                "CONFIGS row 15[{i}]: Rust run({v});run({v}) != driver({v}) while C agrees\n  \
                 runs: {:?}\n  driver: {:?}",
                String::from_utf8_lossy(&r_runs),
                String::from_utf8_lossy(&r_drv)
            ));
        } else {
            h.passed += 1;
        }
    }

    // Row 16: accumulated state, 16 small randomized values in one process.
    let seq: Vec<Call> = (0..16).map(|_| run_call(rng.range(-1000, 1000))).collect();
    h.check_nonempty("CONFIGS row 16", "16x run(small), accumulated state", &seq);

    // Row 17: accumulated state, 16 full-range values (repeated wrap-around).
    let seq: Vec<Call> = (0..16).map(|_| run_call(rng.i32_full())).collect();
    h.check_nonempty("CONFIGS row 17", "16x run(full-range), accumulated", &seq);

    // Row 18: 8 x driver = 16 passes, accumulated.
    let seq: Vec<Call> = (0..8).map(|_| driver_call(rng.i32_full())).collect();
    h.check_nonempty("CONFIGS row 18", "8x driver(full-range), accumulated", &seq);

    // Row 19: mixed entry points, accumulated.
    let seq: Vec<Call> = (0..32)
        .map(|_| {
            let v = rng.i32_full();
            if rng.bool() { run_call(v) } else { driver_call(v) }
        })
        .collect();
    h.check_nonempty("CONFIGS row 19", "32 mixed run/driver calls, accumulated", &seq);

    // Row 20: long sequence -- floors grows 1..4 digits, bathrooms reaches
    // 4098.5, exercising `%d` field growth and `%.1f` at large magnitude.
    let seq: Vec<Call> = (0..4096).map(|_| run_call(0)).collect();
    h.check_nonempty("CONFIGS row 20", "4096x run(0), long accumulation", &seq);

    // Row 21: engineered repeated crossing of INT_MAX.
    let seq: Vec<Call> = (0..64).map(|_| run_call(I32_MAX / 2)).collect();
    h.check_nonempty("CONFIGS row 21", "64x run(INT_MAX/2), repeated overflow", &seq);

    // Row 22: engineered repeated crossing of INT_MIN.
    let seq: Vec<Call> = (0..64).map(|_| run_call(I32_MIN / 2)).collect();
    h.check_nonempty("CONFIGS row 22", "64x run(INT_MIN/2), repeated underflow", &seq);

    // Row 23: land bedrooms exactly on INT_MAX, then INT_MIN, then print the
    // widest `%d` field. bedrooms starts at 5:
    //   +(INT_MAX-5)         -> INT_MAX
    //   +1                   -> INT_MIN (wrap)
    //   +0                   -> INT_MIN printed again
    h.check_nonempty(
        "CONFIGS row 23",
        "bedrooms -> INT_MAX -> INT_MIN -> INT_MIN",
        &[run_call(I32_MAX - 5), run_call(1), run_call(0)],
    );

    // Row 24-25: ABI truncation -- a 64-bit argument handed to an `int` callee.
    let wide_vals: [i64; 8] = [
        0x1_0000_0000,
        0xFFFF_FFFF_0000_0007u64 as i64,
        i64::MAX,
        i64::MIN,
        -1,
        0x7FFF_FFFF_8000_0000u64 as i64,
        0xDEAD_BEEF_0000_0000u64 as i64,
        0x0000_0001_7FFF_FFFF,
    ];
    for (i, &v) in wide_vals.iter().enumerate() {
        h.check(
            &format!("CONFIGS row 24[{i}]"),
            &format!("run called as fn(i64) with {v:#x}"),
            &[run_wide(v)],
        );
        h.check(
            &format!("CONFIGS row 25[{i}]"),
            &format!("driver called as fn(i64) with {v:#x}"),
            &[driver_wide(v)],
        );
    }

    // Row 26: 256 randomized mixed-length sequences, each in its own process
    // pair. This is the property-style sweep over the whole surface.
    for i in 0..256 {
        let len = rng.range(1, 12) as usize;
        let seq: Vec<Call> = (0..len)
            .map(|_| {
                let v = if rng.bool() { rng.i32_full() } else { rng.range(-64, 64) };
                if rng.bool() { run_call(v) } else { driver_call(v) }
            })
            .collect();
        h.check(&format!("CONFIGS row 26[{i}]"), "randomized mixed sequence", &seq);
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows
// ---------------------------------------------------------------------------

fn phase_c(h: &mut Harness, rng: &mut Rng) {
    // Row 1: run(INT_MAX). There is no error return to compare (both functions
    // are `void`), so the observable contract is: the process must survive and
    // print the same wrapped value. A Rust translation that panicked on overflow
    // would abort here and the worker's non-zero exit would fail the assert in
    // `spawn`.
    h.check_nonempty("ERRORS row 1", "run(INT_MAX): signed overflow, no rejection", &[run_call(I32_MAX)]);

    // Row 2: run(INT_MIN).
    h.check_nonempty("ERRORS row 2", "run(INT_MIN): signed underflow input", &[run_call(I32_MIN)]);

    // Row 3-4: the same through `driver`, i.e. applied twice.
    h.check_nonempty("ERRORS row 3", "driver(INT_MAX): overflow twice", &[driver_call(I32_MAX)]);
    h.check_nonempty("ERRORS row 4", "driver(INT_MIN): underflow twice", &[driver_call(I32_MIN)]);

    // Row 5: one step past the 32-bit parameter range.
    for (i, &v) in [
        0x1_0000_0000i64,
        0x1_0000_0001,
        -0x1_0000_0000,
        i64::MAX,
        i64::MIN,
        -1,
        0x0000_0000_8000_0000,
        0xFFFF_FFFF_7FFF_FFFFu64 as i64,
    ]
    .iter()
    .enumerate()
    {
        h.check(
            &format!("ERRORS row 5[{i}]"),
            &format!("wide argument {v:#x} truncated to int"),
            &[run_wide(v), driver_wide(v)],
        );
    }

    // Row 6: the parameter is a bare `int`, not an enum -- there is no invalid
    // variant and no rejection branch. Sweep the boundaries exhaustively plus a
    // randomized sample of the full 2^32 space; every value must be accepted
    // identically by both libraries.
    let mut boundary: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        -2,
        5,
        -5,
        -6,
        I32_MAX,
        I32_MAX - 1,
        I32_MAX - 4,
        I32_MAX - 5,
        I32_MAX - 6,
        I32_MIN,
        I32_MIN + 1,
        I32_MIN + 4,
        I32_MIN + 5,
        I32_MIN + 6,
        i16::MAX as i32,
        i16::MAX as i32 + 1,
        i16::MIN as i32,
        i16::MIN as i32 - 1,
        u16::MAX as i32,
        u16::MAX as i32 + 1,
        i8::MAX as i32,
        i8::MAX as i32 + 1,
        i8::MIN as i32,
        i8::MIN as i32 - 1,
        0x0100_0000,
        0x7F00_0000,
        -0x0100_0000,
    ];
    for _ in 0..96 {
        boundary.push(rng.i32_full());
    }
    for (i, &v) in boundary.iter().enumerate() {
        h.check(
            &format!("ERRORS row 6[{i}]"),
            &format!("int value {v} is accepted, never rejected"),
            &[run_call(v), driver_call(v)],
        );
    }

    // Row 7: the `floors++` counter. True overflow needs 2^31-3 calls, which is
    // not runnable; drive 4096 increments and require exact agreement, which is
    // what is actually observable.
    let seq: Vec<Call> = (0..4096).map(|_| run_call(0)).collect();
    h.check_nonempty("ERRORS row 7", "4096 floors++ increments", &seq);

    // Row 8: there is no pointer/length/buffer parameter to pass NULL or an
    // oversized length for. Assert that structurally rather than claiming to
    // have tested something that does not exist: the C `.so` must export exactly
    // the two int-taking entry points, and must not import the string/memory
    // machinery a buffer-taking API would need.
    let c_defined = nm_defined(&h.c);
    let c_undefined = nm_undefined(&h.c);
    let mut problems = Vec::new();
    if c_defined != vec!["driver".to_string(), "run".to_string()] {
        problems.push(format!("unexpected C API surface: {c_defined:?}"));
    }
    for banned in ["memcpy", "memmove", "strlen", "strcpy", "malloc", "free"] {
        if c_undefined.iter().any(|s| s == banned) {
            problems.push(format!(
                "C library imports {banned}, so it may after all take a buffer/pointer; \
                 ERRORS.md row 8 must be revisited"
            ));
        }
    }
    if problems.is_empty() {
        h.passed += 1;
    } else {
        for p in problems {
            h.failed.push(format!("ERRORS row 8: {p}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Phase D — symbol parity, checked from inside the test
// ---------------------------------------------------------------------------

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b) = (it.next(), it.next());
            let (kind, name) = match (a, b, it.next()) {
                (Some(k), Some(n), None) => (k, n),
                (Some(_addr), Some(k), Some(n)) => (k, n),
                _ => return None,
            };
            // Keep only strong global text/data; drop weak CRT hooks.
            if kind == "T" || kind == "U" || kind == "D" || kind == "B" {
                Some(name.split('@').next().unwrap().to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_defined(path: &Path) -> Vec<String> {
    nm(path, "--defined-only")
}
fn nm_undefined(path: &Path) -> Vec<String> {
    nm(path, "--undefined-only")
}

fn phase_d(h: &mut Harness) {
    let c_syms = nm_defined(&h.c);
    let r_syms = nm_defined(&h.rust);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    if missing.is_empty() {
        h.passed += 1;
    } else {
        let msg = format!("SYMBOLS: Rust .so is missing C-exported symbols: {missing:?}");
        eprintln!("FAIL {msg}");
        h.failed.push(msg);
    }

    // The C internals are `static`; Rust must not leak them either.
    for internal in ["the_house", "add_floor", "add_bedrooms", "add_floor_to_the_house", "print_the_house"] {
        if r_syms.iter().any(|s| s == internal) {
            h.failed.push(format!(
                "SYMBOLS: Rust .so exports {internal}, which is `static` (unexported) in C"
            ));
        }
    }

    // Every Rust import must be libc / the unwinder -- no dangling references.
    let allowed_prefixes = ["_Unwind_", "__", "_ITM_"];
    let libc_syms: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "printf",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific", "pthread_setspecific",
        "read", "readlink", "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall",
        "sysconf", "write", "writev", "exit", "fflush", "puts", "fwrite", "signal", "sigaction",
        "sigaltstack", "poll", "pipe2", "dlsym", "mprotect", "getpid", "raise", "environ",
    ];
    let mut foreign = Vec::new();
    for s in nm_undefined(&h.rust) {
        if allowed_prefixes.iter().any(|p| s.starts_with(p)) || libc_syms.contains(&s.as_str()) {
            continue;
        }
        foreign.push(s);
    }
    if foreign.is_empty() {
        h.passed += 1;
    } else {
        h.failed.push(format!(
            "SYMBOLS: Rust .so has undefined non-libc symbols: {foreign:?}"
        ));
    }

    // `printf` must be imported by both: that is what makes `%d`/`%.1f`
    // formatting identical by construction rather than by reimplementation.
    if !nm_undefined(&h.rust).iter().any(|s| s == "printf") {
        h.failed.push(
            "SYMBOLS: Rust .so does not import printf; formatting is reimplemented and must be \
             audited digit-by-digit"
                .to_string(),
        );
    } else {
        h.passed += 1;
    }
}

// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && args[1] == "--worker" {
        let lib = args[2].clone();
        let seq = decode(&args[3..]);
        worker(&lib, &seq);
    }

    // The libtest-style flags cargo passes to a `harness = false` target.
    if args.iter().any(|a| a == "--list") {
        println!("differential: test");
        return;
    }

    let exe = std::env::current_exe().expect("current_exe");
    let c = c_so();
    let rust = rust_so();
    println!("C    .so: {}", c.display());
    println!("Rust .so: {}", rust.display());

    let mut h = Harness { exe, c, rust, passed: 0, failed: Vec::new() };
    let mut rng = Rng::new();

    println!("\n== Phase D: symbol parity ==");
    phase_d(&mut h);
    println!("== Phase B: valid-path differential (CONFIGS.md) ==");
    phase_b(&mut h, &mut rng);
    println!("== Phase C: error-path differential (ERRORS.md) ==");
    phase_c(&mut h, &mut rng);

    println!("\nchecks passed: {}", h.passed);
    if h.failed.is_empty() {
        println!("all differential checks matched byte-for-byte");
    } else {
        println!("FAILURES: {}", h.failed.len());
        for f in &h.failed {
            println!("---\n{f}");
        }
        std::process::exit(1);
    }
}
