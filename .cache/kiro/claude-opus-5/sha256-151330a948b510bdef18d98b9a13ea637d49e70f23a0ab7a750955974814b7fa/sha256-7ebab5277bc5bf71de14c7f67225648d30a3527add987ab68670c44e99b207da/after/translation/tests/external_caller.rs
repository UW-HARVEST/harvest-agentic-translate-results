//! End-to-end equivalence from a *C* caller, one fresh process per scenario.
//!
//! `tests/differential.rs` compares the two libraries in-process via
//! `libloading`. This file adds the complementary view: a small C program
//! (`tests/c_caller/main.c`) `dlopen`s the library it is handed, runs one
//! scenario and exits. Both libraries go through the same binary, the same call
//! sites and a fresh process, so the comparison is apples-to-apples and matches
//! how a real consumer of `driver.h` would use the library.
//!
//! Scenarios are split in two:
//!
//! * `MUST_MATCH` — reachable without `bad()`. Asserted byte-for-byte, plus the
//!   exit status and termination signal.
//! * `UB_PATH` — reaches `bad()`, which reads an uninitialized `char *`. The C
//!   result there is decided by stack-frame aliasing rather than by the program;
//!   see `ub_path_characterisation` for the measurements. Those are checked for
//!   reproducibility and reported, not asserted equal.

mod harness;

use harness::show;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Scenarios whose behaviour is fully determined by the C source.
const MUST_MATCH: &[(&str, &str)] = &[
    ("printLine_null", ""),
    ("printLine", ""),
    ("printLine", "string"),
    ("printLine", "hello world"),
    ("printLine", "%s %d %n %%"),
    ("printLine", "tab\tand\\backslash"),
    ("printLine", "héllo — wörld 🦀"),
    ("good", ""),
    ("good_x8", ""),
    ("driver", "1"),
    ("driver", "-1"),
    ("driver", "2"),
    ("driver", "42"),
    ("driver", "2147483647"),
    ("driver", "-2147483648"),
    ("driver", "0x100"),
    ("driver", "0x10000"),
    ("driver", "0x7fff0000"),
];

/// Scenarios that reach `bad()`.
const UB_PATH: &[(&str, &str)] = &[
    ("bad", ""),
    ("bad_x8", ""),
    ("driver", "0"),
    ("bad_after_churn", ""),
    ("printLine_then_bad", "MARKER"),
    ("printLine_then_driver0", "MARKER"),
    ("good_then_driver0", ""),
    ("driver1_then_bad", ""),
    ("driver0_then_bad", ""),
    ("driver_alternating", ""),
    ("mixed", ""),
];

// ---------------------------------------------------------------------------
// plumbing
// ---------------------------------------------------------------------------

/// Compiles `tests/c_caller/main.c` once per test binary. `None` means no C
/// compiler is available, in which case the tests here report as skipped.
fn c_caller() -> Option<&'static Path> {
    static EXE: OnceLock<Option<PathBuf>> = OnceLock::new();
    EXE.get_or_init(|| {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/c_caller/main.c");
        assert!(src.exists(), "missing {}", src.display());

        let out_dir = std::env::current_exe()
            .ok()?
            .parent()?
            .parent()?
            .join("c_caller_harness");
        std::fs::create_dir_all(&out_dir).ok()?;
        let exe = out_dir.join("c_caller");

        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        match Command::new(&cc)
            .args(["-O0", "-g"])
            .arg(&src)
            .arg("-o")
            .arg(&exe)
            .arg("-ldl")
            .status()
        {
            Ok(s) if s.success() => Some(exe),
            Ok(s) => panic!("`{cc}` failed to build the C caller harness: {s}"),
            Err(e) => {
                eprintln!("SKIP: cannot run C compiler `{cc}`: {e}");
                None
            }
        }
    })
    .as_deref()
}

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    status: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Display for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", show(&self.stdout))?;
        match (self.status, self.signal) {
            (_, Some(sig)) => write!(f, " [killed by signal {sig}]"),
            (Some(0), None) => Ok(()),
            (code, None) => write!(f, " [exit {code:?}]"),
        }
    }
}

fn run(lib: &Path, scenario: &str, arg: &str) -> Run {
    let exe = c_caller().expect("C caller harness");
    let out = Command::new(exe)
        .arg(lib)
        .arg(scenario)
        .arg(arg)
        .output()
        .unwrap_or_else(|e| panic!("spawning {} failed: {e}", exe.display()));

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;

    Run {
        stdout: out.stdout,
        status: out.status.code(),
        signal,
    }
}

fn label(scenario: &str, arg: &str) -> String {
    if arg.is_empty() {
        scenario.to_string()
    } else {
        format!("{scenario} {arg:?}")
    }
}

fn c_lib() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "build the C library first: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_lib() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    for cand in [
        deps.join("libdriver.so"),
        deps.parent().unwrap().join("libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!("Rust cdylib not found next to {}", exe.display());
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

/// Every fully-defined scenario must agree on stdout, exit status and signal.
#[test]
fn e2e_defined_behaviour_is_identical() {
    if c_caller().is_none() {
        return;
    }
    let (c, r) = (c_lib(), rust_lib());

    let mut failures = Vec::new();
    for (scenario, arg) in MUST_MATCH {
        let a = run(&c, scenario, arg);
        let b = run(&r, scenario, arg);
        let name = label(scenario, arg);

        if a == b {
            println!("  ok  {name} -> {a}");
        } else {
            failures.push(format!("  {name}\n      C   : {a}\n      Rust: {b}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} C-caller scenarios diverged:\n{}",
        failures.len(),
        MUST_MATCH.len(),
        failures.join("\n")
    );
}

/// The `MUST_MATCH` assertions are only meaningful if both libraries are stable
/// under them, so this pins that down separately: a future flake is then
/// reported as non-determinism rather than as a translation defect.
#[test]
fn e2e_defined_behaviour_is_reproducible() {
    if c_caller().is_none() {
        return;
    }
    let (c, r) = (c_lib(), rust_lib());

    for (scenario, arg) in MUST_MATCH {
        for (which, lib) in [("C", &c), ("Rust", &r)] {
            let first = run(lib, scenario, arg);
            for round in 1..4 {
                let again = run(lib, scenario, arg);
                assert!(
                    first == again,
                    "{which} is not reproducible for `{}` (round {round}): {first} vs {again}",
                    label(scenario, arg)
                );
            }
        }
    }
}

/// Records what the two libraries do on the `bad()` path.
///
/// `bad()` in C is:
///
/// ```c
/// char *data;        // never assigned  -- CWE-457
/// printLine(data);
/// ```
///
/// The value read is whatever occupies one particular stack slot. On gcc -O0
/// x86_64, `bad` reads `[rbp-0x8]`, which is exactly where `printLine` and
/// `good` save their own `char *` when called at the same depth, so the measured
/// C behaviour is:
///
/// | preceding call sequence              | C output                          |
/// |--------------------------------------|-----------------------------------|
/// | nothing (`bad` first in the process) | `"\n"` (slot points at a NUL)     |
/// | `driver(0)` first                    | `""` (slot is zero, i.e. NULL)    |
/// | `printLine("MARKER"); bad()`         | `"MARKER"` printed **twice**      |
/// | `good(); driver(0)`                  | `"string"` printed twice          |
/// | `printLine("MARKER"); driver(0)`     | raw machine-code bytes            |
/// | `driver(1); bad()`                   | **SIGSEGV**                       |
/// | `driver(0)`/`driver(1)` alternating  | **varies between runs**           |
///
/// So the C output on this path is not a property of the program: it is decided
/// by frame geometry, it ranges over empty / blank line / an unrelated earlier
/// string / raw code bytes, and it can crash the process. Matching it would
/// require re-emitting all four functions with gcc's exact frame layout in
/// assembly, and no single value the translation could pick matches every
/// context.
///
/// The translation emits a deterministic empty line, which is what the C library
/// does whenever `bad()` is the first thing a process calls. This test measures
/// both sides, asserts that the Rust one is deterministic and never terminates
/// abnormally, and reports the C one — including how many distinct results the C
/// library produces for the same input.
#[test]
fn ub_path_characterisation() {
    if c_caller().is_none() {
        return;
    }
    let (c, r) = (c_lib(), rust_lib());
    const ROUNDS: usize = 5;

    /// Distinct outcomes observed over `ROUNDS` runs of the same scenario.
    fn outcomes(lib: &Path, scenario: &str, arg: &str) -> Vec<Run> {
        let mut seen: Vec<Run> = Vec::new();
        for _ in 0..ROUNDS {
            let run = run(lib, scenario, arg);
            if !seen.contains(&run) {
                seen.push(run);
            }
        }
        seen
    }

    println!("bad() path — the C result is stack-dependent (CWE-457); see doc comment\n");
    let (mut agree, mut c_unstable) = (0usize, 0usize);

    for (scenario, arg) in UB_PATH {
        let c_seen = outcomes(&c, scenario, arg);
        let rust_seen = outcomes(&r, scenario, arg);

        // The Rust side must be deterministic and must never crash, whatever
        // the C original does.
        assert_eq!(
            rust_seen.len(),
            1,
            "Rust is not deterministic for `{}`: {}",
            label(scenario, arg),
            rust_seen
                .iter()
                .map(Run::to_string)
                .collect::<Vec<_>>()
                .join(" / ")
        );
        let rust_out = &rust_seen[0];
        assert_eq!(
            (rust_out.status, rust_out.signal),
            (Some(0), None),
            "Rust terminated abnormally for `{}`: {rust_out}",
            label(scenario, arg)
        );

        if c_seen.len() > 1 {
            c_unstable += 1;
        }
        let verdict = if c_seen.len() == 1 && &c_seen[0] == rust_out {
            agree += 1;
            "same"
        } else if c_seen.len() > 1 {
            "FLAKY"
        } else {
            "DIFF"
        };

        println!(
            "  {verdict:<5} {:<34} C: {:<46} Rust: {rust_out}",
            label(scenario, arg),
            c_seen
                .iter()
                .map(Run::to_string)
                .collect::<Vec<_>>()
                .join("  |  ")
        );
    }

    println!(
        "\n  over {ROUNDS} runs each: {agree}/{n} scenarios agree, {c_unstable}/{n} are \
         non-deterministic in C itself.\n  \
         The C output on this path is decided by stack-frame aliasing, not by the program, \
         so it is not matchable by any translation.",
        n = UB_PATH.len()
    );
}

/// Confirms the measurement that anchors the translation's choice: with
/// `driver(0)` as the first call in a fresh process, the C library reaches
/// `printLine` with a slot that is either NULL or a pointer to a NUL byte, so it
/// writes nothing or a single blank line — never `"string"`. If this ever
/// changes, the table in `ub_path_characterisation` needs revisiting.
#[test]
fn ub_path_c_driver_zero_is_blank_or_empty() {
    if c_caller().is_none() {
        return;
    }
    let out = run(&c_lib(), "driver", "0");
    assert_eq!(
        (out.status, out.signal),
        (Some(0), None),
        "C driver(0) terminated abnormally: {out}"
    );
    assert!(
        out.stdout.is_empty() || out.stdout == b"\n",
        "unexpected C driver(0) output {}",
        show(&out.stdout)
    );
}
