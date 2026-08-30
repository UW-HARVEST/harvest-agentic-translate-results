//! Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//!
//! Both libraries are loaded with `libloading` and driven ONLY through their
//! exported C symbols -- the Rust functions are never called directly, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.
//!
//! Every call is made in a child process (`examples/runner`) because several
//! rows of `ERRORS.md` make the library dereference an invalid pointer and die
//! from `SIGSEGV`. Comparing `(stdout, exit code, signal)` turns a crash into a
//! comparable observation instead of a lost test run.
//!
//! Phase A artifacts: `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`.

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Fixed seed so every randomized row is reproducible.
const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

/// SplitMix64: the same generator the runner uses, so seeds line up.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// A non-zero `i32` -- the `driver()` "truthy" domain.
    fn next_nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }
}

/// `target/<profile>` directory, derived from the running test binary
/// (`target/debug/deps/differential-<hash>`).
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent() // deps/
        .and_then(Path::parent) // debug/
        .expect("target/<profile>")
        .to_path_buf()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn runner_path() -> PathBuf {
    let p = target_profile_dir().join("examples").join("runner");
    assert!(
        p.is_file(),
        "runner example not built at {p:?} -- run `cargo build --example runner` \
         (cargo test normally builds it automatically)"
    );
    p
}

/// The C shared library built by `c_src/CMakeLists.txt`.
fn c_lib() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C library missing at {p:?} -- build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Every Rust `cdylib` that exists, so the default (debug) build AND the
/// `release` build (which sets `panic = "abort"`) are both verified.
fn rust_libs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let target = target_profile_dir()
        .parent()
        .expect("target dir")
        .to_path_buf();
    for profile in ["debug", "release"] {
        let p = target.join(profile).join("libdriver.so");
        if p.is_file() {
            out.push(p);
        }
    }
    assert!(!out.is_empty(), "no Rust libdriver.so found under {target:?}");

    // Freshness guard. `cargo test` builds the *test* targets but does NOT
    // re-emit the `cdylib` artifact, so without this check the whole suite can
    // silently pass against a stale `.so` from an earlier build -- which is
    // exactly how a real divergence hid here once. Fail loudly instead.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let newest_src = ["src/lib.rs", "Cargo.toml"]
        .iter()
        .filter_map(|f| std::fs::metadata(manifest.join(f)).ok()?.modified().ok())
        .max()
        .expect("source mtime");
    for p in &out {
        let so = std::fs::metadata(p)
            .and_then(|m| m.modified())
            .expect("so mtime");
        assert!(
            so >= newest_src,
            "STALE ARTIFACT: {p:?} is older than src/lib.rs or Cargo.toml.\n\
             `cargo test` does not rebuild the cdylib. Run:\n  \
             cargo build && cargo build --release\n\
             (or use ./verify_all.sh, which sequences the builds correctly)."
        );
    }
    out
}

/// What a child-process invocation observably did.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8_lossy(&self.stdout);
        let shown: String = if s.len() > 400 {
            format!("{}... ({} bytes total)", &s[..400], self.stdout.len())
        } else {
            s.into_owned()
        };
        write!(
            f,
            "Outcome {{ code: {:?}, signal: {:?}, stdout: {:?} }}",
            self.code, self.signal, shown
        )
    }
}

impl Outcome {
    fn crashed(&self) -> bool {
        self.signal.is_some()
    }
    fn lines(&self) -> Vec<&[u8]> {
        self.stdout
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .collect()
    }
}

/// Run the runner against `lib` with `args`, capturing stdout into a pipe
/// (fully buffered -- CONFIGS row 33).
fn run(lib: &Path, args: &[String]) -> Outcome {
    let out = Command::new(runner_path())
        .arg(lib)
        .args(args)
        .output()
        .expect("spawn runner");
    // A dlopen/symbol-resolution failure or a runner `panic!` is a harness bug,
    // never a legitimate result -- surface it loudly instead of comparing it.
    if out.status.code() == Some(101) || out.status.code() == Some(2) {
        panic!(
            "runner harness error for {lib:?} {args:?}:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Outcome {
        stdout: out.stdout,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

/// Build an argv from an op plus a list of integers.
fn argv_ints(op: &str, values: &[i32]) -> Vec<String> {
    let mut v = vec![op.to_string()];
    v.extend(values.iter().map(|x| x.to_string()));
    v
}

/// The core assertion: C and Rust must be byte-identical on stdout AND agree on
/// exit code and terminating signal, for every Rust build profile present.
#[track_caller]
fn assert_same(args: &[String]) -> Outcome {
    let c = run(&c_lib(), args);
    for rl in rust_libs() {
        let r = run(&rl, args);
        assert_eq!(
            c.stdout,
            r.stdout,
            "stdout differs for args {args:?}\n  C  ({:?}) = {c:?}\n  RUST ({rl:?}) = {r:?}",
            c_lib()
        );
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "exit status differs for args {args:?}\n  C = {c:?}\n  RUST ({rl:?}) = {r:?}"
        );
    }
    c
}

// ===========================================================================
// PHASE A -- symbol surface
// ===========================================================================

/// SYMBOLS.md: every dynamic symbol the C `.so` defines must also be defined by
/// the Rust `.so`, with the exact same name. Enforced mechanically with `nm -D`.
#[test]
fn symbols_00_nm_parity() {
    fn defined(lib: &Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(lib)
            .output()
            .expect("nm not available");
        assert!(out.status.success(), "nm failed on {lib:?}");
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = defined(&c_lib());
    assert!(
        !c.is_empty(),
        "no symbols read from the C .so -- is it built?"
    );
    // Sanity-check the artifact against the source: these are the four external
    // functions in c_src/src/driver.c.
    for expected in ["printIntPtrLine", "good", "bad", "driver"] {
        assert!(
            c.contains(&expected.to_string()),
            "C .so unexpectedly lacks {expected}"
        );
    }

    for rl in rust_libs() {
        let r = defined(&rl);
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "Rust .so {rl:?} is missing {} C symbol(s): {missing:?}",
            missing.len()
        );
    }
}

/// All four symbols must be resolvable through `dlsym` and callable, in both
/// libraries. (The runner resolves every symbol before dispatching, so a
/// successful `symbols` op proves the whole surface is reachable.)
#[test]
fn symbols_01_all_resolvable_via_dlsym() {
    let o = assert_same(&argv(&["symbols"]));
    assert_eq!(o.stdout, b"printIntPtrLine good bad driver\n");
}

// ===========================================================================
// PHASE B -- valid-path differential tests (one test per CONFIGS.md row)
// ===========================================================================

/// CONFIGS rows 1-6: hand-picked value shapes with known `%d` traps.
#[test]
fn cfg_01_06_print_int_ptr_line_value_traps() {
    // row 1: 0 | row 2: 5 | row 3: -1 (0xFFFFFFFF)
    // row 4: INT_MAX | row 5: INT_MIN | row 6: 0x8000_0000 reinterpreted
    let values = [0i32, 5, -1, i32::MAX, i32::MIN, 0x8000_0000u32 as i32];
    let o = assert_same(&argv_ints("print", &values));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "0\n5\n-1\n2147483647\n-2147483648\n-2147483648\n",
        "C reference output changed"
    );
}

/// CONFIGS row 7: every decimal-width boundary, both signs.
#[test]
fn cfg_07_print_decimal_width_sweep() {
    let mut values: Vec<i32> = Vec::new();
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        for v in [p - 1, p, p + 1] {
            if v <= i32::MAX as i64 {
                values.push(v as i32);
                values.push(-(v as i32));
            }
        }
        p *= 10;
    }
    values.push(i32::MAX);
    values.push(i32::MIN);
    assert!(values.len() >= 50);
    assert_same(&argv_ints("print", &values));
}

/// CONFIGS row 8: randomized full-range `i32` through a stack local.
#[test]
fn cfg_08_print_randomized_stack() {
    let mut rng = Rng::new(SEED);
    let values: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    let o = assert_same(&argv_ints("print", &values));
    assert_eq!(o.lines().len(), 256, "expected one line per input");
}

/// CONFIGS row 9: heap-allocated storage.
#[test]
fn cfg_09_print_randomized_heap() {
    let mut rng = Rng::new(SEED ^ 9);
    let values: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    let o = assert_same(&argv_ints("print_heap", &values));
    assert_eq!(o.lines().len(), 256);
}

/// CONFIGS row 10: writable static (`.data`) storage.
#[test]
fn cfg_10_print_randomized_static() {
    let mut rng = Rng::new(SEED ^ 10);
    let values: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    let o = assert_same(&argv_ints("print_static", &values));
    assert_eq!(o.lines().len(), 256);
}

/// CONFIGS row 11: read-only static (`.rodata`) storage.
#[test]
fn cfg_11_print_rodata() {
    let o = assert_same(&argv(&["print_rodata"]));
    assert_eq!(o.stdout, b"1234567\n");
}

/// CONFIGS rows 12-15: array element access -- first, middle, last, and a full
/// walk. The `last` index is the off-by-one / out-of-range-index trap.
#[test]
fn cfg_12_15_print_array_indices() {
    let mut rng = Rng::new(SEED ^ 12);
    let arr: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();

    for (row, mode, expect_lines) in [
        (12, "first", 1usize),
        (13, "mid", 1),
        (14, "last", 1),
        (15, "all", 64),
    ] {
        let mut args = vec!["print_array".to_string(), mode.to_string()];
        args.extend(arr.iter().map(|x| x.to_string()));
        let o = assert_same(&args);
        assert_eq!(
            o.lines().len(),
            expect_lines,
            "CONFIGS row {row} ({mode}): wrong line count"
        );
    }

    // Cross-check the C against the array contents: mode `all` must reproduce
    // the array in order, proving the pointer arithmetic (not just that C and
    // Rust agree with each other).
    let mut args = vec!["print_array".to_string(), "all".to_string()];
    args.extend(arr.iter().map(|x| x.to_string()));
    let o = run(&c_lib(), &args);
    let got: Vec<i32> = o
        .lines()
        .iter()
        .map(|l| String::from_utf8_lossy(l).parse().unwrap())
        .collect();
    assert_eq!(got, arr);
}

/// CONFIGS row 16 / ERRORS row 4: a misaligned pointer is NOT rejected on
/// x86-64; the C reads 4 bytes little-endian at the odd address.
#[test]
fn cfg_16_print_misaligned_pointer() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..32 {
        let bytes: Vec<i32> = (0..8).map(|_| (rng.next_u64() & 0xFF) as i32).collect();
        let o = assert_same(&argv_ints("print_misaligned", &bytes));
        assert!(!o.crashed(), "misaligned read must not fault on x86-64");

        // Confirm the read really is 4 bytes little-endian starting at byte 1.
        let expect = i32::from_le_bytes([
            bytes[1] as u8,
            bytes[2] as u8,
            bytes[3] as u8,
            bytes[4] as u8,
        ]);
        let got: i32 = String::from_utf8_lossy(o.lines()[0]).parse().unwrap();
        assert_eq!(got, expect, "not a 4-byte little-endian read at buf+1");
    }
}

/// CONFIGS row 17 / ERRORS row 5: the last 4 valid bytes of a mapping, with the
/// next page unmapped. Proves the read width is exactly `sizeof(int)` == 4 and
/// never 8 -- an 8-byte read here would fault.
#[test]
fn cfg_17_print_page_end_boundary() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..16 {
        let v = rng.next_i32();
        let o = assert_same(&argv_ints("print_page_end", &[v]));
        assert!(!o.crashed(), "reading the last 4 valid bytes must not fault");
        assert_eq!(o.stdout, format!("{v}\n").into_bytes());
    }
}

/// CONFIGS row 18: 512 buffered writes in one process, crossing the stdio
/// `BUFSIZ` boundary many times.
#[test]
fn cfg_18_print_burst_buffering() {
    let o = assert_same(&argv(&["print_burst", "0x5EED1234ABCD0001", "512"]));
    assert_eq!(o.lines().len(), 512);
    assert!(o.stdout.len() > 4096, "burst should exceed one stdio buffer");
}

/// CONFIGS row 19: `good()` prints exactly `5\n`.
#[test]
fn cfg_19_good_single_call() {
    let o = assert_same(&argv(&["good", "1"]));
    assert_eq!(o.stdout, b"5\n");
}

/// CONFIGS row 20: `good()` 256 times -- no drift from a reused stack slot.
#[test]
fn cfg_20_good_repeated() {
    let o = assert_same(&argv(&["good", "256"]));
    assert_eq!(o.stdout, "5\n".repeat(256).into_bytes());
}

/// CONFIGS rows 21-27: `driver()` truthiness over the hand-picked shapes,
/// including the byte- and half-word-truncation traps.
#[test]
fn cfg_21_27_driver_truthy_shapes() {
    let cases: [(u32, i32); 7] = [
        (21, 1),
        (22, 2),
        (23, -1),
        (24, i32::MAX),
        (25, i32::MIN),
        (26, 0x0000_0100),        // low BYTE zero -> a `as u8` bug would call bad()
        (27, 0x7fff_0000u32 as i32), // low 16 bits zero -> `as u16` bug
    ];
    for (row, v) in cases {
        let o = assert_same(&argv_ints("driver", &[v]));
        assert_eq!(
            o.stdout, b"5\n",
            "CONFIGS row {row}: driver({v}) must take the good() path"
        );
    }
    // ...and all of them in one process, to check ordering too.
    let values: Vec<i32> = cases.iter().map(|&(_, v)| v).collect();
    let o = assert_same(&argv_ints("driver", &values));
    assert_eq!(o.stdout, "5\n".repeat(7).into_bytes());
}

/// CONFIGS row 28: randomized non-zero `useGood` -- every value must print `5`.
#[test]
fn cfg_28_driver_randomized_nonzero() {
    let mut rng = Rng::new(SEED ^ 28);
    let values: Vec<i32> = (0..256).map(|_| rng.next_nonzero_i32()).collect();
    let o = assert_same(&argv_ints("driver", &values));
    assert_eq!(
        o.stdout,
        "5\n".repeat(256).into_bytes(),
        "every non-zero useGood must dispatch to good()"
    );
}

/// CONFIGS row 30: `driver(1)` 256 times, buffered ordering.
#[test]
fn cfg_30_driver_repeated() {
    let values = vec![1i32; 256];
    let o = assert_same(&argv_ints("driver", &values));
    assert_eq!(o.stdout, "5\n".repeat(256).into_bytes());
}

/// CONFIGS row 32: the composed pipeline -- `driver`, `good` and
/// `printIntPtrLine` interleaved over a randomized 256-step script in ONE
/// process. Ordering and buffering of the composed sequence are invisible to
/// per-wrapper tests.
#[test]
fn cfg_32_composed_pipeline() {
    for seed in ["0x5EED1234ABCD0001", "0xDEADBEEF", "0x1"] {
        let o = assert_same(&argv(&["pipeline", seed, "256"]));
        assert!(!o.crashed(), "the good-path pipeline must never fault");
        assert!(o.lines().len() >= 256, "one line per step at minimum");
    }
}

/// CONFIGS rows 33-34: stdout shape. Row 33 (pipe) is how every other test
/// runs. Row 34 re-runs the two output-heavy rows with stdout redirected to a
/// regular FILE, which glibc buffers using the file's `st_blksize`.
#[test]
fn cfg_33_34_stdout_pipe_vs_regular_file() {
    let dir = std::env::temp_dir().join(format!("driver-difftest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    for args in [
        argv(&["print_burst", "0x5EED1234ABCD0001", "512"]),
        argv(&["pipeline", "0x5EED1234ABCD0001", "256"]),
    ] {
        // row 33: pipe (captured by Command::output)
        let via_pipe = run(&c_lib(), &args);

        // row 34: regular file, for C and for every Rust profile
        let mut outputs = Vec::new();
        let mut libs = vec![c_lib()];
        libs.extend(rust_libs());
        for (i, lib) in libs.iter().enumerate() {
            let path = dir.join(format!("out-{i}.txt"));
            let f = std::fs::File::create(&path).expect("create temp file");
            let st = Command::new(runner_path())
                .arg(lib)
                .args(&args)
                .stdout(f)
                .status()
                .expect("spawn runner");
            assert_eq!(st.code(), Some(0), "unexpected status for {lib:?}");
            outputs.push(std::fs::read(&path).expect("read temp file"));
        }

        for (i, o) in outputs.iter().enumerate().skip(1) {
            assert_eq!(
                &outputs[0], o,
                "regular-file stdout differs for {:?} with args {args:?}",
                libs[i]
            );
        }
        assert_eq!(
            via_pipe.stdout, outputs[0],
            "C output must not depend on whether stdout is a pipe or a file"
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}

// ===========================================================================
// PHASE C -- error-path differential tests (one test per ERRORS.md row)
// ===========================================================================

/// ERRORS row 1: `printIntPtrLine(NULL)`. There is no null check in the C, so
/// the null dereference must kill the process with the SAME signal in both,
/// having printed nothing.
#[test]
fn err_01_print_int_ptr_line_null() {
    let o = assert_same(&argv(&["print_raw_ptr", "0"]));
    assert!(o.crashed(), "NULL deref must raise a fatal signal, got {o:?}");
    assert_eq!(o.signal, Some(libc_sigsegv()), "expected SIGSEGV");
    assert!(o.stdout.is_empty(), "nothing may be printed before the fault");
}

/// ERRORS row 2: a non-null but unmapped low address.
#[test]
fn err_02_print_int_ptr_line_unmapped_low() {
    let o = assert_same(&argv(&["print_raw_ptr", "0x1"]));
    assert_eq!(o.signal, Some(libc_sigsegv()));
    assert!(o.stdout.is_empty());
}

/// ERRORS row 3: a non-canonical / unmapped high address.
#[test]
fn err_03_print_int_ptr_line_noncanonical() {
    let o = assert_same(&argv(&["print_raw_ptr", "0xdeadbeefdeadbee0"]));
    assert_eq!(o.signal, Some(libc_sigsegv()));
    assert!(o.stdout.is_empty());
}

/// ERRORS row 4: misaligned but valid -- accepted, not rejected.
/// (Full value-level verification lives in `cfg_16_print_misaligned_pointer`.)
#[test]
fn err_04_print_int_ptr_line_misaligned() {
    let o = assert_same(&argv_ints("print_misaligned", &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x11]));
    assert!(!o.crashed(), "unaligned loads are legal on x86-64");
    assert_eq!(o.code, Some(0));
    let expect = i32::from_le_bytes([0xBB, 0xCC, 0xDD, 0xEE]);
    assert_eq!(o.stdout, format!("{expect}\n").into_bytes());
}

/// ERRORS row 5: last 4 valid bytes of a mapping -- accepted.
#[test]
fn err_05_print_int_ptr_line_page_end() {
    let o = assert_same(&argv(&["print_page_end", "424242"]));
    assert!(!o.crashed());
    assert_eq!(o.stdout, b"424242\n");
}

/// ERRORS row 6: one step past the end of the mapping -- rejected by the MMU.
/// This is the "one past a valid range" boundary case.
#[test]
fn err_06_print_int_ptr_line_past_end() {
    let o = assert_same(&argv(&["print_past_end"]));
    assert_eq!(o.signal, Some(libc_sigsegv()));
    assert!(o.stdout.is_empty());
}

/// ERRORS row 7 / CONFIGS row 31: `bad()` -- the CWE-457 defect.
///
/// `int *data;` is read uninitialised and dereferenced. There is no defined
/// behaviour to match byte-for-byte here, and the C is not even self-consistent
/// (calling `bad()` directly and reaching it through `driver(0)` print
/// different garbage, because the stack slot holds different leftovers). So the
/// assertion is that the DEFECT IS PRESERVED rather than silently fixed:
///
///  * the outcome is either a fatal fault or a single garbage integer line --
///    exactly the two things an uninitialised pointer dereference can do;
///  * it is NOT the sanitized value `0`, and NOT `5` (which would mean the
///    translation had quietly replaced `bad()` with `good()`'s behaviour);
///  * `good()` in the very same process still prints `5`, proving the library
///    works and that `bad()`'s garbage is genuinely an uninitialised read.
#[test]
fn err_07_bad_is_undefined_behaviour() {
    let mut libs = vec![("C", c_lib())];
    for rl in rust_libs() {
        libs.push(("RUST", rl));
    }

    for (tag, lib) in &libs {
        let mut faulted = 0;
        let mut printed = 0;
        // Every distinct observable outcome of reading the indeterminate slot,
        // gathered across BOTH call paths that reach `bad()`.
        let mut outcomes: std::collections::BTreeSet<String> = Default::default();
        for op in [["bad"].as_slice(), ["driver", "0"].as_slice()] {
            for _ in 0..12 {
                let o = run(lib, &argv(op));
                outcomes.insert(if o.crashed() {
                    format!("signal:{:?}", o.signal)
                } else {
                    String::from_utf8_lossy(&o.stdout).into_owned()
                });
            }
        }
        // THE anti-sanitization gate. `int *data;` is indeterminate, so what
        // `bad()` does depends on whatever the stack slot happens to hold --
        // and it demonstrably differs between the two call paths (in C,
        // `bad()` directly and `driver(0)` print different garbage, because the
        // leftover stack contents differ). A translation that "fixed" the
        // defect -- `data = null` (always SIGSEGV), or `data = &0` (always
        // prints 0), or any other deterministic substitute -- would collapse
        // this to a SINGLE outcome. Requiring more than one outcome is what
        // makes "the defect is preserved" a real assertion rather than a hope.
        assert!(
            outcomes.len() > 1,
            "{tag} {lib:?}: bad() produced the single deterministic outcome {outcomes:?} \
             across both call paths -- the uninitialised read has been replaced by a \
             defined value, i.e. CWE-457 was silently fixed instead of preserved"
        );

        for _ in 0..12 {
            let o = run(lib, &argv(&["bad"]));
            if o.crashed() {
                assert_eq!(
                    o.signal,
                    Some(libc_sigsegv()),
                    "{tag} {lib:?}: bad() faulted with an unexpected signal: {o:?}"
                );
                assert!(o.stdout.is_empty());
                faulted += 1;
            } else {
                assert_eq!(o.code, Some(0), "{tag} {lib:?}: unexpected exit: {o:?}");
                let lines = o.lines();
                assert_eq!(lines.len(), 1, "{tag} {lib:?}: bad() prints one line: {o:?}");
                let text = String::from_utf8_lossy(lines[0]).to_string();
                text.parse::<i32>().unwrap_or_else(|_| {
                    panic!("{tag} {lib:?}: bad() printed a non-integer {text:?}")
                });
                assert_ne!(
                    text, "5",
                    "{tag} {lib:?}: bad() printed 5 -- the CWE-457 defect has been \
                     replaced by good()'s behaviour instead of being preserved"
                );
                printed += 1;
            }
        }
        assert_eq!(
            faulted + printed,
            12,
            "{tag} {lib:?}: unaccounted outcomes"
        );
    }

    // Both implementations must exhibit the SAME KIND of undefined behaviour:
    // an uninitialised-pointer dereference, i.e. never a clean, defined result.
    // And `good()` must still be correct in both, so the garbage above cannot
    // be blamed on a broken library.
    let o = assert_same(&argv(&["good", "1"]));
    assert_eq!(o.stdout, b"5\n");
}

/// ERRORS row 8 / CONFIGS row 29: `driver(0)` must dispatch to `bad()`, i.e.
/// inherit the UB -- and in particular must NOT print `5`.
#[test]
fn err_08_driver_zero_dispatches_to_bad() {
    let mut libs = vec![("C", c_lib())];
    for rl in rust_libs() {
        libs.push(("RUST", rl));
    }
    for (tag, lib) in &libs {
        for _ in 0..8 {
            let o = run(lib, &argv(&["driver", "0"]));
            if o.crashed() {
                assert_eq!(o.signal, Some(libc_sigsegv()), "{tag} {lib:?}: {o:?}");
                continue;
            }
            let lines = o.lines();
            assert_eq!(lines.len(), 1, "{tag} {lib:?}: {o:?}");
            assert_ne!(
                String::from_utf8_lossy(lines[0]),
                "5",
                "{tag} {lib:?}: driver(0) must take the bad() branch, not good()"
            );
        }
    }
}

/// ERRORS row 9: out-of-range "enum-like" `int` values across the FFI boundary.
/// `useGood` is a plain `int` with no range check, so C accepts ANY `int`;
/// truthiness is evaluated on the full 32-bit value. Every non-zero value --
/// including ones with a zero low byte or zero low half-word -- must reach
/// `good()`, and only exactly `0` must reach `bad()`.
#[test]
fn err_09_driver_out_of_range_int_values() {
    let truthy: [i32; 14] = [
        1,
        2,
        3,
        -1,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        0x0000_0100,                 // low byte zero
        0x0001_0000,                 // low 16 bits zero
        0x7fff_ff00u32 as i32,       // low byte zero, large
        0x7fff_0000u32 as i32,       // low 16 bits zero, large
        0x0100_0000,                 // low 24 bits zero
    ];
    // All in one process, so ordering is compared as well as content.
    let o = assert_same(&argv_ints("driver", &truthy));
    assert_eq!(
        o.stdout,
        "5\n".repeat(truthy.len()).into_bytes(),
        "every non-zero int must be truthy for the C `if (useGood)`"
    );

    // Each one individually too, so a failure names the exact value.
    for v in truthy {
        let o = assert_same(&argv_ints("driver", &[v]));
        assert_eq!(o.stdout, b"5\n", "driver({v}) diverged");
    }

    // The sole falsy value is exactly 0 -- covered by err_08.
}

/// ERRORS row 10: dirty high 32 bits. The caller declares `driver` as taking an
/// `i64` and passes a value whose low 32 bits are zero but whose high bits are
/// not. The C ABI says the callee reads only the low 32 bits, so this must
/// truncate to `0` and take the `bad()` branch -- NOT print `5`.
#[test]
fn err_10_driver_dirty_high_bits() {
    // Low 32 bits == 0 => must behave exactly like driver(0).
    let mut libs = vec![("C", c_lib())];
    for rl in rust_libs() {
        libs.push(("RUST", rl));
    }
    for (tag, lib) in &libs {
        for _ in 0..6 {
            let o = run(lib, &argv(&["driver_dirty", "0x100000000"]));
            if o.crashed() {
                assert_eq!(o.signal, Some(libc_sigsegv()), "{tag} {lib:?}: {o:?}");
                continue;
            }
            let lines = o.lines();
            assert_eq!(lines.len(), 1, "{tag} {lib:?}: {o:?}");
            assert_ne!(
                String::from_utf8_lossy(lines[0]),
                "5",
                "{tag} {lib:?}: 0x1_0000_0000 must truncate to 0 and reach bad()"
            );
        }
    }

    // Low 32 bits != 0 => the good() path, fully deterministic, so compare
    // byte-for-byte. These also confirm the upper half is ignored rather than
    // participating in the truthiness test.
    for arg in [
        "0x100000001",
        "0xFFFFFFFF00000001",
        "0x7FFFFFFF7FFFFFFF",
        "0x0000000100000100", // dirty high half AND zero low byte
    ] {
        let o = assert_same(&argv(&["driver_dirty", arg]));
        assert_eq!(o.stdout, b"5\n", "driver_dirty({arg}) diverged");
    }
}

/// ERRORS row 11: `good()` is nullary and cannot fail -- it must always print
/// `5\n` and exit 0, across many repetitions and process launches.
#[test]
fn err_11_good_cannot_fail() {
    for n in [1u32, 2, 7, 64, 300] {
        let o = assert_same(&argv(&["good", &n.to_string()]));
        assert_eq!(o.code, Some(0));
        assert!(!o.crashed());
        assert_eq!(o.stdout, "5\n".repeat(n as usize).into_bytes());
    }
}

/// Generic FFI boundary sweep that every C API deserves, beyond the table:
/// zero, one, and many; plus a dense sweep of small `useGood` values so no
/// single value in the low range can diverge unnoticed.
#[test]
fn err_12_generic_boundary_sweep() {
    // "many": a long argv of mixed truthy values.
    let mut values: Vec<i32> = (1..=200).collect();
    values.extend((-200..0).rev());
    let o = assert_same(&argv_ints("driver", &values));
    assert_eq!(o.stdout, "5\n".repeat(values.len()).into_bytes());

    // "zero-length": an op with no values at all must be a no-op in both.
    let o = assert_same(&argv(&["print"]));
    assert!(o.stdout.is_empty());
    assert_eq!(o.code, Some(0));

    let o = assert_same(&argv(&["driver"]));
    assert!(o.stdout.is_empty());
    assert_eq!(o.code, Some(0));

    // "oversized": a large burst, to be sure nothing degrades with volume.
    let o = assert_same(&argv(&["print_burst", "0xABCDEF", "4096"]));
    assert_eq!(o.lines().len(), 4096);
}

// ---------------------------------------------------------------------------

/// `SIGSEGV` == 11 on Linux.
fn libc_sigsegv() -> i32 {
    11
}
