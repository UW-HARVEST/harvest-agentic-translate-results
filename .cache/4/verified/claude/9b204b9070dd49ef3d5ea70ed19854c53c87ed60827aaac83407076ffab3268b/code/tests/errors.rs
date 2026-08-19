// Phase C — error/rejection-path differential tests.
//
// One test per row of ERRORS.md (E1..E10) plus the generic FFI boundary rows
// (G1..G7).  Every test constructs the exact invalid input/condition, calls
// BOTH `.so`s through their exported symbols, and asserts they reject it in the
// SAME way — identical captured bytes, or identical `waitpid` termination
// signal for the inputs that trigger the out-of-bounds write.

mod common;

use common::*;

// ===========================================================================
// E1 / G1 — printLine(NULL): the `line != NULL` guard fails -> no output
// ===========================================================================

#[test]
fn err_e1_print_line_null() {
    // Exactly zero bytes must be produced by both.
    assert_same_and_eq("E1", &[Op::PrintLineNull], b"");

    // And it must stay silent when repeated and when surrounded by real output,
    // i.e. the guard must not swallow or emit anything extra.
    assert_same_and_eq(
        "E1",
        &[Op::PrintLineNull, Op::PrintLineNull, Op::PrintLineNull],
        b"",
    );
    assert_same_and_eq(
        "E1",
        &[
            Op::PrintLine(cbuf(b"before")),
            Op::PrintLineNull,
            Op::PrintLine(cbuf(b"after")),
        ],
        b"before\nafter\n",
    );

    // The null pointer must also not crash either implementation.
    let h = harness();
    let oc = child_print_line(&h.c, None);
    let or = child_print_line(&h.rust, None);
    assert_eq!(oc, or, "printLine(NULL): C {oc:?} vs Rust {or:?}");
    assert_eq!(oc, Outcome::Exited(0), "printLine(NULL) must not crash");
}

// ===========================================================================
// E2 — printLine(""): guard passes, degenerate content -> a single "\n"
// ===========================================================================

#[test]
fn err_e2_print_line_empty() {
    assert_same_and_eq("E2", &[Op::PrintLine(cbuf(b""))], b"\n");
}

// ===========================================================================
// E3 — driver(100): first value that fails `data < 100`
// ===========================================================================

#[test]
fn err_e3_driver_at_bound_100() {
    assert_same_and_eq("E3", &[Op::Driver(100)], b"\n");
}

// ===========================================================================
// E4 — driver(101): one step past the bound
// ===========================================================================

#[test]
fn err_e4_driver_past_bound_101() {
    assert_same_and_eq("E4", &[Op::Driver(101)], b"\n");
}

// ===========================================================================
// E5 — driver(INT_MAX): maximal oversized length
// ===========================================================================

#[test]
fn err_e5_driver_int_max() {
    assert_same_and_eq("E5", &[Op::Driver(i32::MAX)], b"\n");
    assert_same_and_eq("E5", &[Op::Driver(i32::MAX - 1)], b"\n");
}

// ===========================================================================
// E6 — driver(99): largest value passing the guard; `dest[99]` is the last
//      in-bounds byte (the off-by-one boundary)
// ===========================================================================

#[test]
fn err_e6_driver_at_99_boundary() {
    let mut expected = vec![b'A'; 99];
    expected.push(b'\n');
    assert_same_and_eq("E6", &[Op::Driver(99)], &expected);

    // Repeating it must not accumulate corruption in either implementation.
    let mut twice = expected.clone();
    twice.extend_from_slice(&expected);
    assert_same_and_eq("E6", &[Op::Driver(99), Op::Driver(99)], &twice);
}

// ===========================================================================
// E7 / G2 — driver(0): zero length
// ===========================================================================

#[test]
fn err_e7_driver_zero_len() {
    assert_same_and_eq("E7", &[Op::Driver(0)], b"\n");
}

// ===========================================================================
// E8 — driver(-1): `data` sign-extends into a huge `size_t` for `strncpy`,
//      writing out of bounds past `dest`.  Both must die the same way.
// ===========================================================================

#[test]
fn err_e8_driver_negative_crashes() {
    let outcome = assert_same_outcome("E8", -1);
    match outcome {
        Outcome::Signaled(sig) => assert!(
            sig == SIGSEGV || sig == SIGBUS,
            "driver(-1) expected SIGSEGV/SIGBUS, got signal {sig}"
        ),
        Outcome::Exited(code) => panic!(
            "driver(-1) is an out-of-bounds write and must terminate by signal, \
             but both implementations exited with code {code}"
        ),
    }
}

// ===========================================================================
// E8b — SUPPLEMENTARY: `data` must be *sign*-extended into `size_t`.
//
// Rationale: E8/E9/E10 can only observe "the child died from signal N".  Both a
// faithful sign-extension (`(size_t)-1` == 2^64-1) and a buggy zero-extension
// (`(size_t)(unsigned)-1` == 2^32-1) produce an unbounded upward write that runs
// off the top of the stack, so the two are *observationally equivalent* through
// the FFI boundary: no black-box differential test can separate them (telling
// 2^32 from 2^64 would require >4 GiB of writable memory above `dest`).
//
// The C source is unambiguous about which one it is -- gcc emits `movslq` for
// the `strncpy` length and `cltq` for the `dest[data]` index -- so the width
// conversion is verified here at the machine-code level instead, which is the
// one place the distinction is visible.  Skipped (not failed) when `objdump` is
// unavailable or when the C baseline cannot be measured.
// ===========================================================================

#[test]
fn err_e8b_negative_data_is_sign_extended_not_zero_extended() {
    if !cfg!(debug_assertions) {
        eprintln!("E8b: skipped (optimized build may share one sign-extension)");
        return;
    }
    fn sign_extensions(so: &std::path::Path) -> Option<usize> {
        let out = std::process::Command::new("objdump")
            .args(["-d", "--no-show-raw-insn", "--disassemble=driver"])
            .arg(so)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        Some(
            text.lines()
                .filter(|l| {
                    l.contains("movslq") || l.contains("movsxd") || l.contains("cltq")
                        || l.contains("cdqe")
                })
                .count(),
        )
    }

    let (c_so, rust_so) = so_paths();
    let (Some(c_n), Some(rust_n)) = (sign_extensions(&c_so), sign_extensions(&rust_so)) else {
        eprintln!("E8b: skipped (objdump unavailable)");
        return;
    };
    if c_n == 0 {
        eprintln!("E8b: skipped (could not measure the C baseline; c_n = 0)");
        return;
    }
    assert!(
        rust_n >= c_n,
        "driver(): the C code sign-extends the `int data` argument {c_n}x \
         (movslq/cltq) before using it as a `size_t` length and as an array \
         index, but the Rust code only does so {rust_n}x. A zero-extension \
         would make negative `data` produce a 2^32-byte overwrite instead of \
         the C's 2^64-byte overwrite."
    );
}

// ===========================================================================
// E9 — driver(INT_MIN): most extreme negative
// ===========================================================================

#[test]
fn err_e9_driver_int_min_crashes() {
    let outcome = assert_same_outcome("E9", i32::MIN);
    assert!(
        matches!(outcome, Outcome::Signaled(s) if s == SIGSEGV || s == SIGBUS),
        "driver(INT_MIN) expected to be killed by SIGSEGV/SIGBUS, got {outcome:?}"
    );
}

// ===========================================================================
// E10 — driver, sweep of negative values (incl. exactly -100 / -99)
// ===========================================================================

#[test]
fn err_e10_driver_negative_sweep_crashes() {
    let mut vals: Vec<i32> = vec![-1, -2, -99, -100, -101, -1000, -(1 << 20), i32::MIN + 1];
    let mut rng = Rng::new(SEED ^ 0xE10);
    for _ in 0..24 {
        vals.push(rng.range(i32::MIN as i64, -1) as i32);
    }
    for d in vals {
        let outcome = assert_same_outcome("E10", d);
        assert!(
            matches!(outcome, Outcome::Signaled(s) if s == SIGSEGV || s == SIGBUS),
            "driver({d}) expected to be killed by SIGSEGV/SIGBUS, got {outcome:?}"
        );
    }
}

// ===========================================================================
// G3 — oversized lengths
// ===========================================================================

#[test]
fn err_g3_driver_oversized_lengths() {
    for d in [100i32, 101, 1000, 100_000, 1 << 20, 1 << 30, i32::MAX] {
        assert_same_and_eq("G3", &[Op::Driver(d)], b"\n");
    }
}

// ===========================================================================
// G4 — one step either side of the documented valid range
// ===========================================================================

#[test]
fn err_g4_driver_one_step_past_range() {
    // 99 = last value taking the copy path, 100 = first value skipping it.
    let mut ninety_nine = vec![b'A'; 99];
    ninety_nine.push(b'\n');
    assert_same_and_eq("G4", &[Op::Driver(99)], &ninety_nine);
    assert_same_and_eq("G4", &[Op::Driver(100)], b"\n");

    // 0 = first value taking the copy path, -1 = first value that is UB
    // (covered for crash-parity in E8).
    assert_same_and_eq("G4", &[Op::Driver(0)], b"\n");
    let mut one = vec![b'A'; 1];
    one.push(b'\n');
    assert_same_and_eq("G4", &[Op::Driver(1)], &one);
}

// ===========================================================================
// G5 — out-of-range enum values across the FFI boundary
//
// The library declares no `enum` type: the complete parameter surface is
// `int` and `const char *`.  The analogue of an out-of-range enum is therefore
// an `int` with no meaningful interpretation; the entire i32 domain is swept
// for parity here so the check is not silently skipped.
// ===========================================================================

#[test]
fn err_g5_no_enum_full_int_domain_parity() {
    // Non-negative half: compare captured bytes (defined behaviour).
    let mut rng = Rng::new(SEED ^ 0x6500_u64);
    let mut vals: Vec<i32> = vec![0, 1, 98, 99, 100, 101, i32::MAX];
    for _ in 0..256 {
        vals.push(rng.range(0, i32::MAX as i64) as i32);
    }
    for d in vals {
        let expected: Vec<u8> = if d < 100 {
            let mut v = vec![b'A'; d as usize];
            v.push(b'\n');
            v
        } else {
            b"\n".to_vec()
        };
        assert_same_and_eq("G5", &[Op::Driver(d)], &expected);
    }

    // Negative half: compare termination signal (undefined behaviour).
    for d in [-1i32, -7, -128, -32768, i32::MIN] {
        assert_same_outcome("G5", d);
    }
}

// ===========================================================================
// G6 — printLine with '%' bearing strings.
//
// gcc folds the C `printf("%s\n", line)` into `puts(line)`; the Rust
// translation keeps a real `printf` with `line` as the *argument*.  If the Rust
// side ever passed `line` as the *format*, these inputs would diverge (or
// crash on `%n`).
// ===========================================================================

#[test]
fn err_g6_print_line_percent_not_format() {
    let cases: &[&[u8]] = &[
        b"%s",
        b"%n",
        b"%n%n%n%n",
        b"%s%s%s%s%s%s%s%s",
        b"%99999999d",
        b"%",
        b"%%",
        b"100%",
        b"a%sb%dc%nd",
        b"%p %p %p %p %p %p %p %p %p %p",
        b"%.2147483647f",
        b"%hhn",
    ];
    for c in cases {
        let buf = cbuf(c);
        let mut expected = c.to_vec();
        expected.push(b'\n');
        assert_same_and_eq("G6", &[Op::PrintLine(buf)], &expected);
    }

    // ...and it must not crash either side (a format-string bug on `%n` would).
    let h = harness();
    for c in cases {
        let buf = cbuf(c);
        let oc = child_print_line(&h.c, Some(buf.clone()));
        let or = child_print_line(&h.rust, Some(buf));
        assert_eq!(
            oc,
            or,
            "printLine({:?}): C {oc:?} vs Rust {or:?}",
            String::from_utf8_lossy(c)
        );
        assert_eq!(oc, Outcome::Exited(0), "printLine must not crash on {c:?}");
    }
}

// ===========================================================================
// G7 — printLine with embedded NUL / newline / non-ASCII high bytes
// ===========================================================================

#[test]
fn err_g7_print_line_embedded_bytes() {
    // Embedded NUL: everything from it on is invisible.
    let mut raw = b"visible".to_vec();
    raw.push(0);
    raw.extend_from_slice(b"HIDDEN");
    assert_same_and_eq("G7", &[Op::PrintLine(cbuf(&raw))], b"visible\n");

    // Leading NUL -> empty line.
    let mut lead = vec![0u8];
    lead.extend_from_slice(b"HIDDEN");
    assert_same_and_eq("G7", &[Op::PrintLine(cbuf(&lead))], b"\n");

    // Embedded newlines are passed through verbatim.
    assert_same_and_eq("G7", &[Op::PrintLine(cbuf(b"a\nb\nc"))], b"a\nb\nc\n");
    assert_same_and_eq("G7", &[Op::PrintLine(cbuf(b"\r\n"))], b"\r\n\n");

    // High / non-UTF-8 bytes are byte-transparent.
    let high: Vec<u8> = (0x80u8..=0xff).collect();
    let mut expected = high.clone();
    expected.push(b'\n');
    assert_same_and_eq("G7", &[Op::PrintLine(cbuf(&high))], &expected);

    // Every non-NUL byte value, one at a time.
    for b in 1u16..=255 {
        let expected = [b as u8, b'\n'];
        assert_same_and_eq("G7", &[Op::PrintLine(cbuf(&[b as u8]))], &expected);
    }
}
