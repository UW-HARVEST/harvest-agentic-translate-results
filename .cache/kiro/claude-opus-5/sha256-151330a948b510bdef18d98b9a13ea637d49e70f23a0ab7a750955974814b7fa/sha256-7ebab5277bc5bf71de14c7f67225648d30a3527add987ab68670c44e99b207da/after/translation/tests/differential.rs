//! Differential tests: `c_src/src/driver.c` versus the Rust cdylib, compared
//! in-process through `dlopen`/`dlsym` (never by calling the Rust crate
//! directly, so the `#[no_mangle]` wrappers are part of what is tested).
//!
//! Call hierarchy, from `include/driver.h` + `src/driver.c`:
//!
//! ```text
//! printLine(const char *)   <- leaf; the only thing that writes output
//!   ^-- good()              <- passes the literal "string"
//!   ^-- bad()               <- passes an *uninitialized* char * (CWE-457)
//!         ^-- driver(int)   <- the only symbol declared in driver.h
//! ```
//!
//! Tests run leaf-first. Everything reachable without `bad()` is asserted
//! byte-for-byte here. The `bad()` path is deliberately not exercised against
//! the C library in this process: its C behaviour is a function of leftover
//! stack contents and, in some call sequences, it dereferences a wild pointer
//! and takes the whole process down with SIGSEGV. It is characterised in
//! `tests/external_caller.rs`, which runs each scenario in a separate process.

mod harness;

use harness::{assert_same, capture_fd1, load_both, load_c, load_rust, show};
use std::ffi::{CString, c_char, c_int};

// ---------------------------------------------------------------------------
// 0. Both objects load and expose the whole C API surface.
// ---------------------------------------------------------------------------

/// Loading succeeds only if `printLine`, `bad`, `good` and `driver` all resolve
/// by their exact C names in both shared objects.
#[test]
fn t00_both_libraries_export_the_full_api() {
    let (c, rust) = load_both();
    println!("C   : {}", c.path.display());
    println!("Rust: {}", rust.path.display());
}

// ---------------------------------------------------------------------------
// 1. printLine — the leaf function.
// ---------------------------------------------------------------------------

/// `printLine(NULL)` must take the guard branch and write nothing at all.
#[test]
fn t01_print_line_null_writes_nothing() {
    let (c, rust) = load_both();

    let c_out = capture_fd1(|| c.print_line(std::ptr::null()));
    let rust_out = capture_fd1(|| rust.print_line(std::ptr::null()));

    assert_same("printLine(NULL)", &c_out, &rust_out);
    assert!(
        c_out.is_empty(),
        "C printLine(NULL) unexpectedly wrote {}",
        show(&c_out)
    );
}

/// A representative set of byte strings pushed through `printf("%s\n", line)`.
fn print_line_cases() -> Vec<(&'static str, Vec<u8>)> {
    let mut cases: Vec<(&'static str, Vec<u8>)> = vec![
        ("empty", b"".to_vec()),
        ("the literal used by good()", b"string".to_vec()),
        ("single space", b" ".to_vec()),
        ("a newline", b"\n".to_vec()),
        ("trailing newline", b"abc\n".to_vec()),
        ("embedded newlines", b"a\nb\nc".to_vec()),
        ("CR LF", b"a\r\n".to_vec()),
        ("tab and vertical tab", b"a\tb\x0bc".to_vec()),
        // `line` is the %s *argument*, not the format: these must be copied
        // verbatim rather than interpreted.
        ("percent s", b"%s".to_vec()),
        ("percent d", b"%d".to_vec()),
        ("percent n", b"%n".to_vec()),
        ("percent percent", b"%%".to_vec()),
        ("format soup", b"%s %d %p %n %99999s %%".to_vec()),
        ("backslashes", b"a\\nb\\\\c".to_vec()),
        ("quotes", b"\"'`".to_vec()),
        ("high bytes", vec![0x80, 0xfe, 0xff, 0x7f, 0x01]),
        ("all printable ASCII", (0x20u8..0x7f).collect::<Vec<u8>>()),
        ("all non-NUL bytes", (1u8..=255).collect::<Vec<u8>>()),
        ("utf8", "héllo — wörld 🦀".as_bytes().to_vec()),
        ("long 4095", vec![b'x'; 4095]),
        ("long 4096 (page)", vec![b'y'; 4096]),
        ("long 4097", vec![b'z'; 4097]),
        ("long 65536", vec![b'q'; 65536]),
        ("long 1 MiB", vec![b'm'; 1 << 20]),
    ];
    // Control bytes one at a time (excluding NUL, which terminates the string).
    for b in 1u8..0x20 {
        cases.push(("control byte", vec![b]));
    }
    cases
}

/// Every case must produce identical bytes from both implementations.
#[test]
fn t02_print_line_matches_for_many_inputs() {
    let (c, rust) = load_both();

    for (name, bytes) in print_line_cases() {
        let s = CString::new(bytes.clone()).expect("no interior NUL in case data");
        let p = s.as_ptr();

        let c_out = capture_fd1(|| c.print_line(p));
        let rust_out = capture_fd1(|| rust.print_line(p));

        let label = format!("printLine [{name}, {} bytes]", bytes.len());
        assert_same(&label, &c_out, &rust_out);

        // Cross-check the C side against the documented semantics of
        // `printf("%s\n", line)`: the argument's bytes, then one '\n'.
        let mut expected = bytes.clone();
        expected.push(b'\n');
        assert_eq!(
            c_out, expected,
            "C printLine deviated from printf(\"%s\\n\") for [{name}]"
        );
    }
}

/// `%s` stops at the first NUL: bytes after it must not be emitted by either
/// implementation.
#[test]
fn t03_print_line_stops_at_first_nul() {
    let (c, rust) = load_both();

    let buf: [c_char; 8] = [
        b'a' as c_char,
        b'b' as c_char,
        0,
        b'c' as c_char,
        b'd' as c_char,
        0,
        b'e' as c_char,
        0,
    ];
    let p = buf.as_ptr();

    let c_out = capture_fd1(|| c.print_line(p));
    let rust_out = capture_fd1(|| rust.print_line(p));

    assert_same("printLine(\"ab\\0cd\\0e\\0\")", &c_out, &rust_out);
    assert_eq!(c_out, b"ab\n".to_vec(), "C did not stop at the first NUL");
}

/// Repeated calls must keep producing the same bytes (no first-call-only state,
/// e.g. a lazily initialised buffer or a one-shot static).
#[test]
fn t04_print_line_is_repeatable() {
    let (c, rust) = load_both();
    let s = CString::new("string").unwrap();
    let p = s.as_ptr();

    for round in 0..8 {
        let c_out = capture_fd1(|| {
            c.print_line(p);
            c.print_line(p);
        });
        let rust_out = capture_fd1(|| {
            rust.print_line(p);
            rust.print_line(p);
        });
        assert_same(&format!("2x printLine, round {round}"), &c_out, &rust_out);
    }
}

/// Interleaved across libraries inside one capture. This catches divergent
/// stream handling: if one side wrote via `write(2)` and the other via stdio,
/// or if the two used different `FILE *` buffers, the lines would reorder.
#[test]
fn t05_print_line_interleaves_without_reordering() {
    let (c, rust) = load_both();
    let a = CString::new("from-c").unwrap();
    let b = CString::new("from-rust").unwrap();

    let mixed = capture_fd1(|| {
        c.print_line(a.as_ptr());
        rust.print_line(b.as_ptr());
        c.print_line(a.as_ptr());
        rust.print_line(b.as_ptr());
    });

    assert_eq!(
        mixed,
        b"from-c\nfrom-rust\nfrom-c\nfrom-rust\n".to_vec(),
        "interleaved C/Rust output was reordered or malformed: {}",
        show(&mixed)
    );
}

// ---------------------------------------------------------------------------
// 2. good() — assigns "string", then delegates to printLine.
// ---------------------------------------------------------------------------

#[test]
fn t06_good_matches() {
    let (c, rust) = load_both();

    let c_out = capture_fd1(|| c.good());
    let rust_out = capture_fd1(|| rust.good());

    assert_same("good()", &c_out, &rust_out);
    assert_eq!(
        c_out,
        b"string\n".to_vec(),
        "C good() wrote {}",
        show(&c_out)
    );
}

#[test]
fn t07_good_repeated_matches() {
    let (c, rust) = load_both();

    let c_out = capture_fd1(|| (0..64).for_each(|_| c.good()));
    let rust_out = capture_fd1(|| (0..64).for_each(|_| rust.good()));

    assert_same("64x good()", &c_out, &rust_out);
    assert_eq!(c_out.len(), 64 * 7, "unexpected C output length for 64x good()");
}

// ---------------------------------------------------------------------------
// 3. driver(int) — the public entry point from driver.h.
//
// `driver(0)` reaches bad(); see the module header. Only the non-zero branch is
// compared here.
// ---------------------------------------------------------------------------

/// The `if (useGood)` test is a plain C truth test, so every non-zero value
/// must take the `good()` branch — not just 1, and not only values with a set
/// low byte or a clear sign bit.
#[test]
fn t08_driver_nonzero_matches_for_all_interesting_ints() {
    let (c, rust) = load_both();

    let mut values: Vec<c_int> = vec![
        1,
        -1,
        2,
        -2,
        7,
        42,
        -42,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        0x0000_0100,
        0x0001_0000,
        0x7fff_0000,
        0x00ff_ff00,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
        i32::from(i16::MIN),
        i32::from(i16::MAX),
        65536,
        -65536,
    ];
    // Every single-bit value: catches an implementation that only inspects the
    // low byte, or only the sign bit, instead of the whole int.
    for bit in 0..32 {
        values.push(1i32.wrapping_shl(bit));
    }
    values.sort_unstable();
    values.dedup();
    assert!(!values.contains(&0), "the zero case belongs to the bad() path");

    for v in values {
        let c_out = capture_fd1(|| c.driver(v));
        let rust_out = capture_fd1(|| rust.driver(v));

        assert_same(&format!("driver({v})"), &c_out, &rust_out);
        assert_eq!(
            c_out,
            b"string\n".to_vec(),
            "C driver({v}) should have taken the good() branch, got {}",
            show(&c_out)
        );
    }
}

/// `driver(nonzero)` must be indistinguishable from a direct `good()` call in
/// both libraries — i.e. the delegation, not just the output, lines up.
#[test]
fn t09_driver_nonzero_delegates_to_good() {
    let (c, rust) = load_both();

    for v in [1, -1, 2, i32::MAX, i32::MIN] {
        let c_driver = capture_fd1(|| c.driver(v));
        let rust_driver = capture_fd1(|| rust.driver(v));
        assert_same(&format!("driver({v})"), &c_driver, &rust_driver);

        let c_good = capture_fd1(|| c.good());
        assert_eq!(
            c_driver,
            c_good,
            "C driver({v}) differs from a direct good(): {} vs {}",
            show(&c_driver),
            show(&c_good)
        );

        let rust_good = capture_fd1(|| rust.good());
        assert_eq!(
            rust_driver,
            rust_good,
            "Rust driver({v}) differs from a direct good(): {} vs {}",
            show(&rust_driver),
            show(&rust_good)
        );
    }
}

/// A long mixed sequence over the non-zero branch: any state drift between the
/// two libraries (buffering, a cached pointer, a one-shot static) shows up as a
/// divergence here.
#[test]
fn t10_driver_long_sequence_matches() {
    let (c, rust) = load_both();

    // Deterministic pseudo-random values, identical for both sides, never zero.
    let mut state: u32 = 0x1234_5678;
    let mut seq: Vec<c_int> = Vec::with_capacity(300);
    while seq.len() < 300 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let v = state as c_int;
        if v != 0 {
            seq.push(v);
        }
    }

    let c_out = capture_fd1(|| seq.iter().for_each(|&v| c.driver(v)));
    let rust_out = capture_fd1(|| seq.iter().for_each(|&v| rust.driver(v)));

    assert_same("driver() over a 300-call sequence", &c_out, &rust_out);
    assert_eq!(c_out.len(), 300 * 7);
}

/// Exports must keep working across `dlclose`/`dlopen`, as they would for any
/// external caller that loads the library more than once.
#[test]
fn t11_reload_and_call_again_matches() {
    for round in 0..3 {
        let c = load_c();
        let rust = load_rust();

        let c_out = capture_fd1(|| {
            c.driver(1);
            c.good();
        });
        let rust_out = capture_fd1(|| {
            rust.driver(1);
            rust.good();
        });

        assert_same(&format!("after reload {round}"), &c_out, &rust_out);
        drop(rust);
        drop(c);
    }
}

// ---------------------------------------------------------------------------
// 4. bad() / driver(0) — the CWE-457 path.
// ---------------------------------------------------------------------------

/// The Rust side of the `bad()` path must at least be *self-consistent*: the
/// same bytes on every call, from `bad()` and from `driver(0)` alike. Only the
/// Rust library is called here; see the module header for why the C library is
/// left to `tests/external_caller.rs`.
#[test]
fn t12_rust_bad_path_is_deterministic() {
    let rust = load_rust();

    let baseline = capture_fd1(|| rust.bad());
    println!("Rust bad() -> {}", show(&baseline));

    for round in 0..16 {
        let again = capture_fd1(|| rust.bad());
        assert_eq!(
            baseline,
            again,
            "Rust bad() is not deterministic (round {round}): {} vs {}",
            show(&baseline),
            show(&again)
        );
    }

    // Same bytes whether reached directly or through the public entry point.
    let via_driver = capture_fd1(|| rust.driver(0));
    assert_eq!(
        baseline,
        via_driver,
        "Rust driver(0) differs from a direct bad(): {} vs {}",
        show(&baseline),
        show(&via_driver)
    );

    // Unaffected by preceding calls, unlike the C original.
    let after_churn = capture_fd1(|| {
        let s = CString::new("unrelated preceding output").unwrap();
        rust.good();
        rust.print_line(s.as_ptr());
        rust.driver(1);
        rust.bad()
    });
    assert!(
        after_churn.ends_with(&baseline),
        "Rust bad() changed after preceding calls: {}",
        show(&after_churn)
    );

    // Whatever the value is, it must be a single complete line: `printLine`
    // appends exactly one '\n' and the C control flow reaches it exactly once.
    assert_eq!(
        baseline.iter().filter(|&&b| b == b'\n').count(),
        1,
        "Rust bad() emitted more than one line: {}",
        show(&baseline)
    );
}
