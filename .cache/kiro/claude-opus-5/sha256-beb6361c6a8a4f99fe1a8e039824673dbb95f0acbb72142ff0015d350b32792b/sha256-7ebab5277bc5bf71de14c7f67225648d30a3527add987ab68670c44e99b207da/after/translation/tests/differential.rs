//! Differential tests for the fully-defined parts of the API.
//!
//! Ordered lowest-level first, following the call hierarchy in
//! `c_src/src/driver.c`:
//!
//! ```text
//! driver(int) -> good() -> printIntPtrLine(const int *)
//!             -> bad()  -> printIntPtrLine(const int *)   [undefined behaviour]
//! ```
//!
//! Every call goes through `dlopen`/`dlsym` on both shared objects; the `bad()`
//! branch is covered in `tests/undefined_behaviour.rs`.

mod common;

use common::{BOTH, Impl, Op, assert_identical, run, run_worker_if_child, show};

/// Child-side worker. Does nothing in the parent run.
#[test]
fn difftest_worker() {
    run_worker_if_child();
}

// ---------------------------------------------------------------------------
// Level 1: printIntPtrLine(const int *)
// ---------------------------------------------------------------------------

/// A broad sweep of `int` values, one call per process.
#[test]
fn print_int_ptr_line_matches() {
    let values: [i32; 19] = [
        0,
        5,
        1,
        -1,
        42,
        -42,
        7,
        10,
        100,
        -100,
        999,
        1_000_000,
        123_456_789,
        -123_456_789,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -0,
    ];

    for v in values {
        let ops = [Op::Print(v)];
        let bytes = assert_identical(&ops, &format!("printIntPtrLine({v})"));
        // The C is `printf("%d\n", *p)`: decimal digits and one newline, nothing else.
        assert_eq!(
            bytes,
            format!("{v}\n").into_bytes(),
            "printIntPtrLine({v}) wrote {}",
            show(&bytes)
        );
    }
}

/// Values around every decimal-width boundary, where a formatting difference
/// would show up first.
#[test]
fn print_int_ptr_line_digit_boundaries_match() {
    let mut values = vec![0i32];
    let mut p: i64 = 1;
    while p <= i32::MAX as i64 {
        for delta in [-1i64, 0, 1] {
            for sign in [1i64, -1] {
                let v = sign * (p + delta);
                if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                    values.push(v as i32);
                }
            }
        }
        p *= 10;
    }
    values.sort_unstable();
    values.dedup();

    // One process runs the whole sweep, which also compares the buffering.
    let ops: Vec<Op> = values.iter().map(|v| Op::Print(*v)).collect();
    let bytes = assert_identical(&ops, "printIntPtrLine digit boundaries");
    let expected: String = values.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(bytes, expected.as_bytes());
}

/// Many calls in one process: catches a difference in `stdio` buffering, or a
/// per-call flush on one side only.
#[test]
fn print_int_ptr_line_long_sequence_matches() {
    let values: Vec<i32> = (0..256).map(|i| i * 7 - 900).collect();
    let ops: Vec<Op> = values.iter().map(|v| Op::Print(*v)).collect();
    let bytes = assert_identical(&ops, "printIntPtrLine long sequence");
    let expected: String = values.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(bytes, expected.as_bytes());
}

/// Enough output to cross the default 4 KiB `stdio` buffer several times, so a
/// mismatch in flush points would become visible as reordering or truncation.
#[test]
fn print_int_ptr_line_crosses_stdio_buffer() {
    let values: Vec<i32> = (0..2000).map(|i| 1_000_000 + i).collect();
    let ops: Vec<Op> = values.iter().map(|v| Op::Print(*v)).collect();
    let bytes = assert_identical(&ops, "printIntPtrLine past the stdio buffer");
    let expected: String = values.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(bytes.len(), expected.len());
    assert_eq!(bytes, expected.as_bytes());
}

/// A pointer into the middle of an array rather than to a standalone local:
/// the callee must read exactly that element and nothing adjacent.
#[test]
fn print_int_ptr_line_array_element_matches() {
    let arr = vec![-7i32, 0, 5, 300_000, i32::MIN, i32::MAX, -1];
    for i in 0..arr.len() {
        let ops = [Op::PrintArrayElem(arr.clone(), i)];
        let bytes = assert_identical(&ops, &format!("printIntPtrLine(&arr[{i}])"));
        assert_eq!(bytes, format!("{}\n", arr[i]).into_bytes());
    }
}

/// The same pointer read repeatedly must not be mutated by the callee - the C
/// parameter is `const int *`.
#[test]
fn print_int_ptr_line_does_not_modify_its_argument() {
    let arr = vec![11i32, 22, 33];
    let ops = vec![
        Op::PrintArrayElem(arr.clone(), 1),
        Op::PrintArrayElem(arr.clone(), 1),
        Op::PrintArrayElem(arr.clone(), 1),
    ];
    let bytes = assert_identical(&ops, "printIntPtrLine is read-only");
    assert_eq!(bytes, b"22\n22\n22\n");
}

// ---------------------------------------------------------------------------
// Level 2: good()
// ---------------------------------------------------------------------------

/// `good()` takes the address of a local set to 5 and prints it.
#[test]
fn good_matches() {
    let bytes = assert_identical(&[Op::Good], "good()");
    assert_eq!(bytes, b"5\n", "good() should print 5, got {}", show(&bytes));
}

/// `good()` is stateless: repeated calls must behave identically.
#[test]
fn good_is_repeatable() {
    let ops = vec![Op::Good; 64];
    let bytes = assert_identical(&ops, "good() x64");
    assert_eq!(bytes, "5\n".repeat(64).into_bytes());
}

// ---------------------------------------------------------------------------
// Level 3: driver(int)
// ---------------------------------------------------------------------------

/// Every nonzero selector must route to `good()`: C truthiness is "not zero",
/// not "equal to 1".
#[test]
fn driver_nonzero_matches() {
    let selectors: [i32; 16] = [
        1,
        -1,
        2,
        -2,
        7,
        255,
        256,
        1024,
        0x1000,
        0x0001_0000,
        0x0100_0000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -2_147_483_647,
    ];

    for s in selectors {
        let bytes = assert_identical(&[Op::Driver(s)], &format!("driver({s})"));
        assert_eq!(
            bytes,
            b"5\n",
            "driver({s}) should take the good() path, got {}",
            show(&bytes)
        );
    }
}

/// Selectors whose low byte or low half is zero. A translation that narrowed
/// the `int` before testing truthiness would take the `bad()` path here and
/// diverge loudly.
#[test]
fn driver_truthiness_is_not_narrowed() {
    for s in [
        0x0000_0100i32,
        0x0000_ff00,
        0x0001_0000,
        0x0100_0000,
        i32::MIN,
        0x7fff_ff00,
    ] {
        let c = run(Impl::C, &[Op::Driver(s)]);
        let rust = run(Impl::Rust, &[Op::Driver(s)]);
        assert_eq!(
            c.bytes, rust.bytes,
            "driver({s:#x}) differs: C {:?} / Rust {:?}",
            c.status, rust.status
        );
        assert_eq!(c.status, rust.status, "driver({s:#x}) status differs");
        assert_eq!(
            c.bytes,
            b"5\n",
            "driver({s:#x}) must be truthy in C, got {}",
            show(&c.bytes)
        );
    }
}

/// Repeated and interleaved calls across all three defined entry points in a
/// single process, so buffering and call order are compared together.
#[test]
fn mixed_call_sequence_matches() {
    let ops = vec![
        Op::Driver(1),
        Op::Good,
        Op::Print(-99),
        Op::Driver(i32::MIN),
        Op::Print(0),
        Op::Driver(255),
        Op::PrintArrayElem(vec![1, 2, 3], 2),
        Op::Good,
        Op::Print(i32::MAX),
        Op::Driver(-1),
    ];
    let bytes = assert_identical(&ops, "mixed sequence");
    assert_eq!(bytes, b"5\n5\n-99\n5\n0\n5\n3\n5\n2147483647\n5\n");
}

/// Alternating selectors, all nonzero, to confirm `driver` keeps no state.
#[test]
fn driver_is_stateless() {
    let selectors: Vec<i32> = (1..=50).map(|i| if i % 2 == 0 { i } else { -i }).collect();
    let ops: Vec<Op> = selectors.iter().map(|s| Op::Driver(*s)).collect();
    let bytes = assert_identical(&ops, "driver stateless");
    assert_eq!(bytes, "5\n".repeat(selectors.len()).into_bytes());
}

// ---------------------------------------------------------------------------
// Output plumbing
// ---------------------------------------------------------------------------

/// The library must write through the *caller's* `stdio` buffer, not one of its
/// own. Otherwise output from the caller's `printf` and output from the library
/// would flush in a different order and this interleaving would come out
/// scrambled for one of the two implementations.
#[test]
fn output_interleaves_with_the_callers_printf() {
    let ops = vec![
        Op::HostPrint("A ".into()),
        Op::Print(1),
        Op::HostPrint("B ".into()),
        Op::Good,
        Op::HostPrint("C ".into()),
        Op::Driver(9),
        Op::HostPrint("D ".into()),
        Op::Print(-2),
    ];
    let bytes = assert_identical(&ops, "interleaving with the caller's stdio");
    assert_eq!(
        bytes,
        b"A 1\nB 5\nC 5\nD -2\n",
        "output ordering relative to the caller's printf is wrong: {}",
        show(&bytes)
    );
}

/// A long interleaved run, crossing the `stdio` buffer while alternating
/// between caller and library writes.
#[test]
fn long_interleaving_matches() {
    let mut ops = Vec::new();
    let mut expected = String::new();
    for i in 0..500 {
        ops.push(Op::HostPrint(format!("<{i}> ")));
        expected.push_str(&format!("<{i}> "));
        ops.push(Op::Print(i));
        expected.push_str(&format!("{i}\n"));
    }
    let bytes = assert_identical(&ops, "long interleaving");
    assert_eq!(bytes, expected.as_bytes());
}

// ---------------------------------------------------------------------------
// Pointer edge cases
// ---------------------------------------------------------------------------

/// `printIntPtrLine(NULL)`. The C does no validation - it dereferences and
/// faults. A translation that added a null check, returned early, or panicked
/// would diverge, so both sides must fault the same way.
#[test]
fn print_int_ptr_line_null_faults_identically() {
    let c = run(Impl::C, &[Op::PrintNull]);
    let rust = run(Impl::Rust, &[Op::PrintNull]);

    assert!(
        c.faulted(),
        "expected the C to fault on a NULL argument, got {:?} / {}",
        c.status,
        show(&c.bytes)
    );
    assert_eq!(
        c.status, rust.status,
        "NULL argument: C ended as {:?} but Rust as {:?} (Rust output {})",
        c.status,
        rust.status,
        show(&rust.bytes)
    );
    assert_eq!(c.bytes, rust.bytes, "NULL argument: output differs");
}

/// A misaligned `const int *`. The C simply dereferences it, which x86-64
/// permits; both sides must read the same value.
#[test]
fn print_int_ptr_line_misaligned_matches() {
    for v in [0i32, 5, -1, 123_456, i32::MIN, i32::MAX] {
        let ops = [Op::PrintUnaligned(v)];
        let c = run(Impl::C, &ops);
        let rust = run(Impl::Rust, &ops);
        assert_eq!(
            c.status, rust.status,
            "misaligned pointer to {v}: status differs"
        );
        assert_eq!(
            c.bytes,
            rust.bytes,
            "misaligned pointer to {v}: C wrote {} but Rust wrote {}",
            show(&c.bytes),
            show(&rust.bytes)
        );
        if c.completed() {
            assert_eq!(c.bytes, format!("{v}\n").into_bytes());
        }
    }
}

/// A fault on one call must not change how a later call behaves, so run the
/// surviving cases together and confirm the sequence still matches.
#[test]
fn misaligned_then_normal_matches() {
    let ops = [
        Op::PrintUnaligned(7),
        Op::Print(8),
        Op::Good,
        Op::PrintUnaligned(-9),
    ];
    let bytes = assert_identical(&ops, "misaligned then normal");
    assert_eq!(bytes, b"7\n8\n5\n-9\n");
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

/// Every symbol the C library exports must resolve in the Rust library too,
/// under the exact same name.
#[test]
fn rust_exports_every_c_symbol() {
    let c = unsafe { libloading::Library::new(Impl::C.path()) }.expect("dlopen C");
    let rust = unsafe { libloading::Library::new(Impl::Rust.path()) }.expect("dlopen Rust");

    for name in ["printIntPtrLine", "bad", "good", "driver"] {
        let sym = [name.as_bytes(), b"\0"].concat();
        let in_c: Result<libloading::Symbol<*const ()>, _> = unsafe { c.get(&sym) };
        assert!(in_c.is_ok(), "C library unexpectedly missing `{name}`");
        let in_rust: Result<libloading::Symbol<*const ()>, _> = unsafe { rust.get(&sym) };
        assert!(
            in_rust.is_ok(),
            "Rust library does not export `{name}` - add a #[no_mangle] wrapper"
        );
    }
}

/// Guards the harness itself: a child that fails to do its job must be
/// reported, not silently compared as two empty outputs.
#[test]
fn harness_detects_real_output() {
    for which in BOTH {
        let outcome = run(which, &[Op::Print(1234)]);
        assert!(
            outcome.completed(),
            "{} worker did not complete: {:?}",
            which.name(),
            outcome.status
        );
        assert_eq!(
            outcome.bytes,
            b"1234\n",
            "{} worker produced {}",
            which.name(),
            show(&outcome.bytes)
        );
    }
}
