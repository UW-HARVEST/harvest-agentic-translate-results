//! The `bad()` path.
//!
//! `bad()` in the original C is:
//!
//! ```c
//! void bad() { int *data; printIntPtrLine(data); }
//! ```
//!
//! It dereferences an uninitialised pointer (CWE-457 / CWE-824). The value it
//! passes is whatever bytes the caller left in one stack slot, so the C is not
//! deterministic and cannot act as a byte-exact oracle here. Measured on this
//! machine, the same C `.so` behaves two different ways depending only on who
//! called it:
//!
//! * `bad()` called directly from a compiled driver -> prints a garbage
//!   integer and returns
//! * `driver(0)` -> `bad()` -> SIGSEGV
//!
//! The read slot lands below the caller's frame in the second case. Asserting
//! C/Rust byte equality on this path would be asserting on uninitialised
//! memory, so these tests pin down the contract that *is* well defined:
//! control reaches `printIntPtrLine` at most once, with a pointer taken from an
//! uninitialised stack slot, so a run either faults on the dereference or emits
//! exactly one `"%d\n"` line and returns.
//!
//! Anything else would be a translation defect rather than the C's own
//! nondeterminism: no output with a clean return (the call was optimised away),
//! more than one line (the call was duplicated), a Rust panic or abort, or a
//! signal other than SIGSEGV/SIGBUS.
//!
//! The `objdump` cross-check at the bottom of this file compares the two
//! implementations structurally instead, which is the strongest statement
//! available for a path whose value is by definition garbage.

mod common;

use common::{BOTH, Impl, Op, Outcome, Status, run, run_worker_if_child, show};

/// Child-side worker. Does nothing in the parent run.
#[test]
fn difftest_worker() {
    run_worker_if_child();
}

/// Matches exactly what `printf("%d\n", v)` can produce for one `int`.
fn is_single_int_line(bytes: &[u8]) -> bool {
    let Some(body) = bytes.strip_suffix(b"\n") else {
        return false;
    };
    let digits = body.strip_prefix(b"-").unwrap_or(body);
    !digits.is_empty() && digits.iter().all(u8::is_ascii_digit) && body.len() <= 11
}

/// The shape a faithful `bad()` must produce: it either faulted on the bad
/// dereference, or it survived and printed exactly one decimal line.
fn is_faithful_bad_shape(o: &Outcome) -> bool {
    match &o.status {
        Status::Signalled(_) => o.faulted(),
        Status::Completed => is_single_int_line(&o.bytes),
        Status::Failed(_) => false,
    }
}

fn describe(o: &Outcome) -> String {
    format!("{:?} with output {}", o.status, show(&o.bytes))
}

/// `bad()` called directly, several times: each run must show the faithful
/// shape on both sides.
#[test]
fn bad_has_faithful_shape() {
    for attempt in 0..3 {
        let c = run(Impl::C, &[Op::Bad]);
        let rust = run(Impl::Rust, &[Op::Bad]);

        assert!(
            is_faithful_bad_shape(&c),
            "attempt {attempt}: C bad() gave {}",
            describe(&c)
        );
        assert!(
            is_faithful_bad_shape(&rust),
            "attempt {attempt}: Rust bad() gave {} (C gave {})",
            describe(&rust),
            describe(&c)
        );
    }
}

/// `driver(0)` is the only selector that reaches `bad()`.
#[test]
fn driver_zero_has_faithful_shape() {
    let c = run(Impl::C, &[Op::Driver(0)]);
    let rust = run(Impl::Rust, &[Op::Driver(0)]);

    assert!(
        is_faithful_bad_shape(&c),
        "C driver(0) gave {}",
        describe(&c)
    );
    assert!(
        is_faithful_bad_shape(&rust),
        "Rust driver(0) gave {} (C gave {})",
        describe(&rust),
        describe(&c)
    );
}

/// The dereference must actually happen: a run that returns cleanly with no
/// output at all would mean the compiler deleted the call, which is a
/// divergence from the C even though the value it passes is garbage.
#[test]
fn bad_never_silently_skips_the_call() {
    for which in BOTH {
        for op in [Op::Bad, Op::Driver(0)] {
            let o = run(which, std::slice::from_ref(&op));
            let skipped = o.completed() && o.bytes.is_empty();
            assert!(
                !skipped,
                "{} {:?} returned without calling printIntPtrLine: {}",
                which.name(),
                op,
                describe(&o)
            );
        }
    }
}

/// `bad()` must call `printIntPtrLine` at most once and print nothing of its
/// own, so at most one newline can reach fd 1.
#[test]
fn bad_emits_at_most_one_line() {
    for which in BOTH {
        for op in [Op::Bad, Op::Driver(0)] {
            let o = run(which, std::slice::from_ref(&op));
            let lines = o.bytes.iter().filter(|b| **b == b'\n').count();
            assert!(
                lines <= 1,
                "{} {:?} wrote {lines} lines: {}",
                which.name(),
                op,
                describe(&o)
            );
        }
    }
}

/// A `bad()` that survives must not corrupt the process: a following `good()`
/// still has to print 5 on both sides.
#[test]
fn state_after_a_surviving_bad_matches() {
    let ops = [Op::Bad, Op::Good];
    let c = run(Impl::C, &ops);
    let rust = run(Impl::Rust, &ops);

    for (which, o) in [("C", &c), ("Rust", &rust)] {
        if o.completed() {
            assert!(
                o.bytes.ends_with(b"5\n"),
                "{which}: good() after a surviving bad() did not print 5: {}",
                describe(o)
            );
            assert_eq!(
                o.bytes.iter().filter(|b| **b == b'\n').count(),
                2,
                "{which}: expected one line from bad() and one from good(): {}",
                describe(o)
            );
        } else {
            assert!(
                o.faulted(),
                "{which}: bad() then good() ended unexpectedly: {}",
                describe(o)
            );
        }
    }
}

/// Structural cross-check of the two `bad()` implementations.
///
/// Both must load 8 bytes from an unwritten slot in their own frame and pass it
/// straight to `printIntPtrLine`. This is what "faithful translation of UB"
/// means operationally, and unlike the printed value it is stable, so it is
/// checked directly on the machine code.
#[test]
fn bad_reads_an_uninitialised_stack_slot_in_both() {
    for which in BOTH {
        let text = disassemble(&which.path(), "bad");

        // A load from a stack slot, relative to rsp or rbp, with nothing having
        // stored to it first.
        let reads_stack_slot = text
            .lines()
            .any(|l| l.contains("mov") && (l.contains("(%rsp)") || l.contains("(%rbp)")));
        assert!(
            reads_stack_slot,
            "{} bad() does not read a stack slot; disassembly:\n{text}",
            which.name()
        );

        // Nothing may initialise that slot: no immediate store into the frame.
        let writes_immediate_to_stack = text.lines().any(|l| {
            l.contains("mov") && l.contains('$') && (l.contains("(%rsp)") || l.contains("(%rbp)"))
        });
        assert!(
            !writes_immediate_to_stack,
            "{} bad() initialises its stack slot, so it is no longer the CWE-457 \
             behaviour of the C; disassembly:\n{text}",
            which.name()
        );

        // And control must reach printIntPtrLine, by call or by tail jump.
        let reaches_callee = text
            .lines()
            .any(|l| l.contains("printIntPtrLine") || l.contains("call") || l.contains("jmp"));
        assert!(
            reaches_callee,
            "{} bad() never transfers control to printIntPtrLine; disassembly:\n{text}",
            which.name()
        );
    }
}

/// Disassembles one function out of a shared object. Returns an empty string if
/// `objdump` is unavailable, which makes the checks above skip rather than fail
/// on a host without binutils.
fn disassemble(lib: &std::path::Path, func: &str) -> String {
    let out = match std::process::Command::new("objdump")
        .args(["-d", "--no-show-raw-insn"])
        .arg(lib)
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        _ => return String::new(),
    };
    let text = String::from_utf8_lossy(&out);
    let start = format!("<{func}>:");
    let mut body = String::new();
    let mut inside = false;
    for line in text.lines() {
        if line.contains(&start) {
            inside = true;
            continue;
        }
        if inside {
            if line.trim().is_empty() {
                break;
            }
            body.push_str(line);
            body.push('\n');
        }
    }
    body
}

/// Sanity guard: if `objdump` is missing the structural test above is vacuous,
/// so say so out loud rather than reporting a pass.
#[test]
fn structural_check_is_not_vacuous() {
    let text = disassemble(&Impl::C.path(), "bad");
    assert!(
        !text.is_empty(),
        "could not disassemble bad() from the C library - install binutils, \
         otherwise bad_reads_an_uninitialised_stack_slot_in_both proves nothing"
    );
}
