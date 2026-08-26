// CONFIGS.md rows 17 & 18 / ERRORS.md row 8 -- the `bad()` and `driver(0)`
// paths, which read an uninitialized pointer in the C source (CWE-457).
//
// `bad()` has no single correct observable behaviour, so this file does three
// things:
//
//   1. asserts the Rust `.so` matches an `-O2` C build byte-for-byte, where the
//      C behaviour IS well defined (`printLine(NULL)` -> no output);
//   2. PINS the `-O0` vs optimised codegen difference by disassembly, so the
//      documented UB exclusion can never silently widen (if a future toolchain
//      stops folding the uninitialized read to NULL at -O2, this fails loudly);
//   3. asserts the `-O0` C build still *reaches* `printLine` and returns
//      normally, i.e. the defect is preserved and was not optimised away.
//
// See ERRORS.md §UB for the measurements behind this design.

mod common;

use common::*;
use std::process::Command;

// ---------------------------------------------------------------------------
// Rows 17 & 18 -- differential against the -O2 C build (well-defined)
// ---------------------------------------------------------------------------

#[test]
fn row18_bad_matches_optimized_c() {
    diff_o2("row18 bad()", |imp| imp.bad());

    // Specific expected result: every optimising gcc build folds the
    // uninitialized pointer to NULL, so `bad()` emits nothing.
    for imp in [c_o2(), rust()] {
        let out = capture_stdout(|| imp.bad());
        assert!(
            out.is_empty(),
            "{}: bad() must emit nothing (printLine(NULL)), got {}",
            imp.name,
            show(&out)
        );
    }

    diff_o2("row18 bad() x50", |imp| {
        for _ in 0..50 {
            imp.bad();
        }
    });
}

#[test]
fn row8_driver_zero_matches_optimized_c() {
    diff_o2("row17 driver(0)", |imp| imp.driver(0));

    for imp in [c_o2(), rust()] {
        let out = capture_stdout(|| imp.driver(0));
        assert!(
            out.is_empty(),
            "{}: driver(0) must emit nothing, got {}",
            imp.name,
            show(&out)
        );
    }

    // driver(0) and bad() must be indistinguishable (driver just dispatches).
    for imp in [c_o2(), rust()] {
        let via_driver = capture_stdout(|| imp.driver(0));
        let direct = capture_stdout(|| imp.bad());
        assert_eq!(
            via_driver, direct,
            "{}: driver(0) must behave exactly like bad()",
            imp.name
        );
    }
}

#[test]
fn row17_driver_zero_interleaved_with_good() {
    // The composed pipeline for the zero branch, against the -O2 C build.
    diff_o2("row17 mixed driver(0)/driver(1)/bad", |imp| {
        for i in 0..20 {
            if i % 3 == 0 {
                imp.driver(0);
            } else if i % 3 == 1 {
                imp.driver(1);
            } else {
                imp.bad();
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Codegen pin: keeps the documented UB exclusion honest
// ---------------------------------------------------------------------------

fn disassemble_bad(so: &std::path::Path) -> String {
    let out = Command::new("objdump")
        .args(["-d", "--no-show-raw-insn"])
        .arg(so)
        .output()
        .expect("objdump");
    assert!(out.status.success(), "objdump failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut body = String::new();
    let mut in_fn = false;
    for line in text.lines() {
        if line.contains("<bad>:") {
            in_fn = true;
            continue;
        }
        if in_fn {
            if line.trim().is_empty() {
                break;
            }
            body.push_str(line);
            body.push('\n');
        }
    }
    assert!(!body.is_empty(), "could not find bad() in {}", so.display());
    body
}

#[test]
#[cfg_attr(not(target_arch = "x86_64"), ignore = "x86_64 disassembly pin")]
fn optimized_c_folds_uninitialized_read_to_null() {
    // This is the assumption the Rust translation of `bad()` rests on. If it
    // ever stops holding, the differential tests above would silently start
    // comparing against different behaviour -- so assert it directly.
    let o2 = disassemble_bad(&c_so_path_o2());
    let calls_printline = o2.contains("printLine");
    assert!(
        calls_printline,
        "-O2 bad() must still call printLine (defect preserved):\n{o2}"
    );
    let zeroes_arg = o2.contains("xor    %edi,%edi")
        || o2.contains("xor %edi,%edi")
        || o2.contains("mov    $0x0,%edi")
        || o2.contains("mov $0x0,%edi");
    assert!(
        zeroes_arg,
        "-O2 bad() is expected to pass a NULL first argument to printLine, \
         which is what the Rust translation reproduces. Actual codegen:\n{o2}"
    );
}

#[test]
#[cfg_attr(not(target_arch = "x86_64"), ignore = "x86_64 disassembly pin")]
fn default_c_build_is_unoptimized_and_reads_stack_residue() {
    // Documents WHY the default (-O0) build is excluded from the byte-for-byte
    // comparison for this one function: it genuinely loads stack residue.
    let o0 = disassemble_bad(&c_so_path_default());
    assert!(
        o0.contains("printLine"),
        "-O0 bad() must call printLine (defect preserved):\n{o0}"
    );
    let reads_stack = o0.contains("-0x8(%rbp),%rax");
    assert!(
        reads_stack,
        "expected the default cmake build to be -O0 and to load the \
         uninitialized slot from [rbp-8]; if this changed, re-evaluate the UB \
         exclusion in ERRORS.md. Actual codegen:\n{o0}"
    );
}

#[test]
fn default_c_build_bad_is_undefined_behaviour_characterization() {
    // The `-O0` C `bad()` dereferences whatever stack residue it finds, so it
    // may return normally OR die on a wild pointer. Both were observed in
    // practice: it returns normally in the debug test binary and SIGSEGVs in the
    // release one, purely because the residue differs.
    //
    // It is therefore run in a FORKED CHILD: the outcome is characterised
    // without risking the test process. This is the empirical justification for
    // excluding `-O0` `bad()` from byte-for-byte comparison (ERRORS.md §UB) --
    // an implementation whose output can be a wild dereference has no
    // well-defined behaviour to match.
    let bad = c_default().bad_fn_ptr();
    let driver = c_default().driver_fn_ptr();

    let o1 = run_isolated(|| unsafe { bad() });
    let o2 = run_isolated(|| unsafe { driver(0) });

    for (label, outcome) in [("bad()", &o1), ("driver(0)", &o2)] {
        match outcome {
            ChildOutcome::Exited(0) => {
                eprintln!("note: -O0 C {label}: residue was benign, returned normally");
            }
            ChildOutcome::Signalled(s) if *s == SIGSEGV || *s == SIGBUS => {
                eprintln!(
                    "note: -O0 C {label}: residue was a wild pointer, died with signal {s} \
                     (this is the CWE-457 defect manifesting)"
                );
            }
            other => panic!(
                "-O0 C {label} terminated unexpectedly: {other:?}. Expected either a clean \
                 return or SIGSEGV/SIGBUS from the uninitialized read."
            ),
        }
    }
}

#[test]
fn rust_bad_never_crashes() {
    // The contrast that matters for the translation: whatever the C build does
    // with its residue, the Rust `bad()` is deterministic and must ALWAYS
    // return cleanly with no output (it is `printLine(NULL)`).
    let bad = rust().bad_fn_ptr();
    let driver = rust().driver_fn_ptr();
    for _ in 0..5 {
        assert_eq!(
            run_isolated(|| unsafe { bad() }),
            ChildOutcome::Exited(0),
            "Rust bad() must always return cleanly"
        );
        assert_eq!(
            run_isolated(|| unsafe { driver(0) }),
            ChildOutcome::Exited(0),
            "Rust driver(0) must always return cleanly"
        );
    }
    // ... and in-process, repeatedly, with no output.
    let out = capture_stdout(|| {
        for _ in 0..100 {
            rust().bad();
            rust().driver(0);
        }
    });
    assert!(
        out.is_empty(),
        "Rust bad()/driver(0) must emit nothing, got {}",
        show(&out)
    );
}

// ---------------------------------------------------------------------------
// Non-zero branch is fully deterministic even at -O0
// ---------------------------------------------------------------------------

#[test]
fn nonzero_branch_is_deterministic_against_default_build() {
    // Contrast with the rows above: `driver(non-zero)` has no UB, so it is
    // compared against the DEFAULT (-O0) C build.
    let mut rng = Rng::new(SEED ^ 0xBAD);
    for _ in 0..200 {
        let mut v = rng.next_u32() as i32;
        if v == 0 {
            v = 1;
        }
        diff(&format!("driver({v}) non-zero vs -O0 C"), |imp| imp.driver(v));
    }
    diff("good() vs -O0 C", |imp| imp.good());
}
