//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input or
//! condition the C rejects, calls BOTH `.so` exports, and asserts they produce
//! the SAME rejection — the exact 18-byte `An error occurred\n` sentinel, the
//! same `errno` left behind, and the same (unchanged) internal state — not
//! merely "both failed somehow".

mod common;
use common::*;

/// Inputs where `strtol` performs no conversion at all, so `endp == str`.
const NO_CONVERSION: [&[u8]; 20] = [
    b"",
    b"abc",
    b"   ",
    b" \t\n\x0b\x0c\r",
    b"+",
    b"-",
    b".",
    b"x10",
    b"++1",
    b"--1",
    b"+-1",
    b"e5",
    b"NaN",
    b"inf",
    b"#1",
    b"/5",
    b":9",
    b"\x7f",
    b" + 1",
    b"- 1",
];

/// `ERANGE`, magnitude above `LONG_MAX`.
const ERANGE_POS: [&[u8]; 6] = [
    b"99999999999999999999",
    b"9223372036854775808",
    b"9223372036854775809",
    b"+9223372036854775808",
    b"18446744073709551616",
    b"  1000000000000000000000000",
];

/// `ERANGE`, magnitude below `LONG_MIN`.
const ERANGE_NEG: [&[u8]; 5] = [
    b"-99999999999999999999",
    b"-9223372036854775809",
    b"-9223372036854775810",
    b"-18446744073709551616",
    b"  -1000000000000000000000000",
];

/// Converted cleanly (`errno == 0`) but below `INT_MIN`.
const BELOW_INT_MIN: [&[u8]; 6] = [
    b"-2147483649",
    b"-2147483650",
    b"-3000000000",
    b"-9223372036854775808",
    b"-9223372036854775807",
    b"  -4294967296",
];

/// Converted cleanly (`errno == 0`) but above `INT_MAX`.
const ABOVE_INT_MAX: [&[u8]; 7] = [
    b"2147483648",
    b"2147483649",
    b"3000000000",
    b"9223372036854775807",
    b"+2147483648",
    b"  4294967296",
    b"0000000000002147483648",
];

/// Shared per-row assertion: the C must reject, and the Rust must reject
/// identically (same bytes AND same `errno`).
#[track_caller]
fn expect_same_rejection(row: &str, p: &Pair, input: &[u8]) {
    let ((c, c_errno), (r, r_errno)) = p.driver_step_errno(input, 0x5A5A);
    let shown = String::from_utf8_lossy(input).escape_debug().to_string();
    assert_eq!(
        c, ERROR_LINE,
        "{row}: C did NOT reject {shown:?} (got {:?})",
        String::from_utf8_lossy(&c)
    );
    same(&format!("{row} driver({shown:?})"), &c, &r);
    assert_eq!(
        c_errno, r_errno,
        "{row}: errno diverged after driver({shown:?}): C={c_errno} Rust={r_errno}"
    );
}

/// ERRORS row 1 — `endp == str`, no conversion performed.
#[test]
fn row01_no_conversion() {
    let p = pair();
    for input in NO_CONVERSION {
        expect_same_rejection("row01", &p, input);
    }
}

/// ERRORS row 2 — `errno == ERANGE`, above `LONG_MAX`.
#[test]
fn row02_erange_positive() {
    let p = pair();
    for input in ERANGE_POS {
        expect_same_rejection("row02", &p, input);
    }
    // The C leaves ERANGE in errno; confirm that is what we are observing, and
    // that both libraries agree on the exact value.
    let ((c, c_errno), (r, r_errno)) = p.driver_step_errno(b"99999999999999999999", 0);
    assert_eq!(c, ERROR_LINE);
    same("row02 erange bytes", &c, &r);
    assert_eq!(c_errno, libc::ERANGE, "C should leave ERANGE in errno");
    assert_eq!(r_errno, libc::ERANGE, "Rust should leave ERANGE in errno");
}

/// ERRORS row 3 — `errno == ERANGE`, below `LONG_MIN`.
#[test]
fn row03_erange_negative() {
    let p = pair();
    for input in ERANGE_NEG {
        expect_same_rejection("row03", &p, input);
    }
    let ((c, c_errno), (_r, r_errno)) = p.driver_step_errno(b"-99999999999999999999", 0);
    assert_eq!(c, ERROR_LINE);
    assert_eq!(c_errno, libc::ERANGE);
    assert_eq!(r_errno, libc::ERANGE);
}

/// ERRORS row 4 — `tmp < INT_MIN` with `errno == 0`.
#[test]
fn row04_below_int_min() {
    let p = pair();
    for input in BELOW_INT_MIN {
        expect_same_rejection("row04", &p, input);
    }
    // This row is distinct from row 3: the conversion SUCCEEDED, so errno must
    // still be the 0 that line 67 wrote, not ERANGE.
    let ((c, c_errno), (_r, r_errno)) = p.driver_step_errno(b"-3000000000", 0x5A5A);
    assert_eq!(c, ERROR_LINE);
    assert_eq!(c_errno, 0, "C: clean conversion must leave errno == 0");
    assert_eq!(r_errno, 0, "Rust: clean conversion must leave errno == 0");
}

/// ERRORS row 5 — `tmp > INT_MAX` with `errno == 0`.
#[test]
fn row05_above_int_max() {
    let p = pair();
    for input in ABOVE_INT_MAX {
        expect_same_rejection("row05", &p, input);
    }
    let ((c, c_errno), (_r, r_errno)) = p.driver_step_errno(b"3000000000", 0x5A5A);
    assert_eq!(c, ERROR_LINE);
    assert_eq!(c_errno, 0, "C: clean conversion must leave errno == 0");
    assert_eq!(r_errno, 0, "Rust: clean conversion must leave errno == 0");
}

/// ERRORS rows 6 + 14 — on rejection, `driver` prints only the sentinel, makes
/// no `run` call, and leaves `the_house` untouched (so `x` is never read).
#[test]
fn row06_error_path_leaves_state_untouched() {
    let p = pair();
    let all: Vec<&[u8]> = NO_CONVERSION
        .iter()
        .chain(ERANGE_POS.iter())
        .chain(ERANGE_NEG.iter())
        .chain(BELOW_INT_MIN.iter())
        .chain(ABOVE_INT_MAX.iter())
        .copied()
        .collect();

    for input in all {
        // Snapshot the state via a no-op `run`.
        let (before_c, before_r) = p.run_step(0);
        same("row06 pre-probe", &before_c, &before_r);
        let before = parse_last_state(&before_c).expect("parse state");

        // Reject.
        let (c, r) = p.driver_step_raw(input);
        assert_eq!(
            c,
            ERROR_LINE,
            "row06: expected rejection for {:?}",
            String::from_utf8_lossy(input)
        );
        same("row06 rejection", &c, &r);

        // Re-probe: `run(0)` prints the state first, before mutating it, so the
        // first line must be exactly what the previous probe's last line said.
        let (after_c, after_r) = p.run_step(0);
        same("row06 post-probe", &after_c, &after_r);
        let first_line_state = {
            let s = std::str::from_utf8(&after_c).unwrap();
            let first = s.lines().next().unwrap();
            parse_last_state(first.as_bytes()).expect("parse state")
        };
        // `run(0)` prints the state BEFORE mutating it, so the first line of a
        // fresh probe must equal the last line of the previous probe. Anything
        // else means the rejected `driver` touched `the_house`.
        assert_eq!(
            first_line_state, before,
            "row06: state moved across a rejected driver({:?})",
            String::from_utf8_lossy(input)
        );
    }
}

/// ERRORS row 7 — `driver(NULL)`: the C never null-checks, so `strtol`
/// dereferences it. Both libraries must die the same way. Run in a forked child
/// so the test process survives.
#[test]
fn row07_null_pointer_identical_fault() {
    let p = pair();
    let (cf, rf) = p.raw_drivers();

    let c_status = fork_and_call(&move || unsafe { cf(std::ptr::null()) });
    let r_status = fork_and_call(&move || unsafe { rf(std::ptr::null()) });

    assert_eq!(
        describe(c_status),
        describe(r_status),
        "driver(NULL) terminated differently: C={} Rust={}",
        describe(c_status),
        describe(r_status)
    );
    // Document what actually happens rather than assuming.
    eprintln!("row07: driver(NULL) => {} (both)", describe(c_status));
}

fn fork_and_call(f: &dyn Fn()) -> i32 {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: silence output, run the call, exit cleanly if it returns.
            let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 1);
                libc::dup2(devnull, 2);
            }
            f();
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        status
    }
}

fn describe(status: i32) -> String {
    if libc::WIFSIGNALED(status) {
        format!("signal {}", libc::WTERMSIG(status))
    } else if libc::WIFEXITED(status) {
        format!("exit {}", libc::WEXITSTATUS(status))
    } else {
        format!("raw {status}")
    }
}

/// ERRORS row 8 — a stale non-zero `errno` on entry must NOT cause a rejection
/// (line 67 clears it). A translation missing `errno = 0` fails here.
#[test]
fn row08_stale_errno_does_not_reject() {
    let p = pair();
    for pre in [libc::ERANGE, libc::EINVAL, libc::ENOENT, 1, 4095, i32::MAX] {
        for input in [b"0".as_slice(), b"1", b"-1", b"2147483647", b"-2147483648"] {
            let ((c, c_errno), (r, r_errno)) = p.driver_step_errno(input, pre);
            assert_ne!(
                c,
                ERROR_LINE,
                "row08: stale errno {pre} made C reject {:?}",
                String::from_utf8_lossy(input)
            );
            same(
                &format!("row08 pre_errno={pre} driver({:?})", String::from_utf8_lossy(input)),
                &c,
                &r,
            );
            assert_eq!(c_errno, r_errno, "row08: errno diverged (pre={pre})");
            assert_eq!(c_errno, 0, "row08: C must have cleared errno");
        }
    }
}

/// ERRORS row 9 — after a rejection that left `ERANGE` behind, the next call
/// with a valid input must still succeed.
#[test]
fn row09_errno_reset_between_calls() {
    let p = pair();
    for _ in 0..8 {
        let (c, r) = p.driver_step_raw(b"99999999999999999999");
        assert_eq!(c, ERROR_LINE);
        same("row09 failing call", &c, &r);

        let (c, r) = p.driver_step_raw(b"11");
        assert_ne!(c, ERROR_LINE, "row09: C failed to recover after ERANGE");
        same("row09 recovering call", &c, &r);
    }
}

/// ERRORS row 10 — oversized inputs. No length check exists; `strtol` reports
/// `ERANGE`.
#[test]
fn row10_oversized_input() {
    let p = pair();
    for len in [4096usize, 100_000] {
        let mut s = Vec::with_capacity(len);
        s.push(b'1');
        s.extend(std::iter::repeat(b'0').take(len - 1));
        let ((c, c_errno), (r, r_errno)) = p.driver_step_errno(&s, 0);
        assert_eq!(c, ERROR_LINE, "row10: C accepted a {len}-digit number");
        same(&format!("row10 len={len}"), &c, &r);
        assert_eq!(c_errno, r_errno, "row10: errno diverged at len={len}");

        // Same length, but negative.
        let mut neg = vec![b'-'];
        neg.extend_from_slice(&s);
        let ((c, c_errno), (r, r_errno)) = p.driver_step_errno(&neg, 0);
        assert_eq!(c, ERROR_LINE);
        same(&format!("row10 neg len={len}"), &c, &r);
        assert_eq!(c_errno, r_errno);

        // Oversized but VALID: a huge run of leading zeros then a small value.
        let mut padded: Vec<u8> = std::iter::repeat(b'0').take(len).collect();
        padded.extend_from_slice(b"42");
        let ((c, _), (r, _)) = p.driver_step_errno(&padded, 0);
        assert_ne!(c, ERROR_LINE, "row10: C should accept zero-padded 42");
        same(&format!("row10 padded len={len}"), &c, &r);
    }
}

/// ERRORS row 11 — the zero-length boundary: a valid pointer to an immediate
/// `NUL`.
#[test]
fn row11_zero_length_input() {
    let p = pair();
    let ((c, c_errno), (r, r_errno)) = p.driver_step_errno(b"", 0);
    assert_eq!(c, ERROR_LINE, "row11: C must reject the empty string");
    same("row11 driver(\"\")", &c, &r);
    assert_eq!(c_errno, r_errno);
    assert_eq!(c_errno, 0, "row11: no conversion attempted, errno stays 0");
}

/// ERRORS row 12 — extreme / overflowing `extra_bedrooms` through `run`. The C
/// wraps; the Rust must wrap identically and must never panic.
#[test]
fn row12_run_overflowing_extremes() {
    let p = pair();
    // Drive bedrooms to a known place, then overflow it deliberately.
    let (probe, _) = p.run_step(0);
    let (_, bedrooms, _) = parse_last_state(&probe).expect("parse state");

    let deltas = [
        i32::MAX,
        i32::MIN,
        i32::MAX.wrapping_sub(bedrooms),      // land exactly on INT_MAX
        1,                                    // then overflow by one
        i32::MIN,
        -1,
        i32::MAX,
        i32::MAX,
    ];
    for (i, &d) in deltas.iter().enumerate() {
        let (c, r) = p.run_step(d);
        assert!(is_four_house_lines(&c), "row12: bad C output for run({d})");
        same(&format!("row12 run({d}) #{i}"), &c, &r);
    }
}

/// ERRORS row 13 — arbitrary `int` bit patterns across the FFI boundary. This
/// API declares no enums (`grep -c enum c_src/**` == 0), so the analogue is an
/// unconstrained `int`: the C validates nothing, so every value must be
/// accepted by both, including ones no sane caller would pass.
#[test]
fn row13_arbitrary_int_across_ffi() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xE0);
    let mut vals = vec![
        0i32,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        0x7FFF_FFFF,
        -0x8000_0000,
        0x5555_5555,
        0xAAAA_AAAAu32 as i32,
        0xDEAD_BEEFu32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x0000_FFFF,
        0xFFFF_0000u32 as i32,
    ];
    while vals.len() < 64 {
        vals.push(rng.next_i32());
    }
    for (i, &n) in vals.iter().enumerate() {
        let (c, r) = p.run_step(n);
        assert!(is_four_house_lines(&c), "row13: bad C output for run({n})");
        same(&format!("row13 run({n:#x}) #{i}"), &c, &r);
    }
}
