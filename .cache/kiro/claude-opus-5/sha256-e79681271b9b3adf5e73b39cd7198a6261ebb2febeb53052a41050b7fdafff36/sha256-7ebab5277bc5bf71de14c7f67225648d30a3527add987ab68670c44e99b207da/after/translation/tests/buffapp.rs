//! Differential tests for the public entry point `buffapp`, which composes
//! every lower-level function and also writes to stdout. Both the return value
//! and the exact bytes printed are compared.

mod common;

use common::{capture_stdout, load_pair, Impl, INTERESTING};
use std::ffi::c_int;

/// Mirror of the C control flow, used only to detect the inputs where the C
/// build would execute `INT_MIN / -1` and take a SIGFPE. Those inputs are
/// undefined behaviour in C and therefore out of scope for a byte-for-byte
/// comparison.
fn traps_in_c(p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> bool {
    fn op_name(code: c_int) -> &'static str {
        match code {
            0 => "add",
            1 => "subtract",
            2 => "multiply",
            3 => "divide",
            _ => "unknown",
        }
    }
    fn apply(a: c_int, b: c_int, op: &str) -> Option<c_int> {
        Some(match op {
            "add" => a.wrapping_add(b),
            "subtract" => a.wrapping_sub(b),
            "multiply" => a.wrapping_mul(b),
            "divide" => {
                if b == 0 {
                    0
                } else if a == c_int::MIN && b == -1 {
                    return None;
                } else {
                    a.wrapping_div(b)
                }
            }
            _ => 0,
        })
    }

    let op1 = op_name(p1.wrapping_rem(4));
    let Some(i1) = apply(p1, p2, op1) else {
        return true;
    };
    let op2 = op_name(p3.wrapping_rem(4));
    let Some(i2) = apply(p3, p4, op2) else {
        return true;
    };
    let i3 = i1.wrapping_mul(i2);
    let result = i1.wrapping_add(i2);
    i3 != 0 && result == c_int::MIN && i3 == -1
}

fn run(imp: &Impl, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> (c_int, Vec<u8>) {
    let mut rc: c_int = 0;
    let out = capture_stdout(|| {
        // SAFETY: `buffapp` takes four ints and only writes to stdout.
        rc = unsafe { (imp.buffapp)(p1, p2, p3, p4) };
    });
    (rc, out)
}

#[track_caller]
fn compare(pair: &common::Pair, p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    if traps_in_c(p1, p2, p3, p4) {
        return;
    }

    // fd 1 is process-global. `capture_stdout` drains both buffering layers and
    // holds a mutex, but a sibling test thread finishing mid-capture can still
    // push a libtest progress line into the redirect window. A genuine
    // translation mismatch is deterministic while a contaminated capture is
    // not, so a mismatch is only reported once it reproduces.
    let mut last: Option<(c_int, Vec<u8>, c_int, Vec<u8>)> = None;
    for _ in 0..3 {
        let (rc_c, out_c) = run(&pair.c, p1, p2, p3, p4);
        let (rc_r, out_r) = run(&pair.rs, p1, p2, p3, p4);
        if rc_c == rc_r && out_c == out_r {
            return;
        }
        last = Some((rc_c, out_c, rc_r, out_r));
    }

    let (rc_c, out_c, rc_r, out_r) = last.unwrap();
    assert_eq!(
        rc_c, rc_r,
        "buffapp({p1}, {p2}, {p3}, {p4}) return value: C={rc_c} Rust={rc_r}"
    );
    assert_eq!(
        out_c,
        out_r,
        "buffapp({p1}, {p2}, {p3}, {p4}) stdout differs (reproduced 3x)\n\
         --- C ---\n{}\n--- Rust ---\n{}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r)
    );
}

#[test]
fn buffapp_small_grid() {
    let pair = load_pair();
    // Covers every combination of op codes (p1 % 4 and p3 % 4), including the
    // negative remainders that select "unknown".
    for p1 in -5..=5 {
        for p2 in -3..=3 {
            for p3 in -5..=5 {
                for p4 in -3..=3 {
                    compare(&pair, p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn buffapp_extreme_values() {
    let pair = load_pair();
    // Wide values exercise the widest `%d` renderings, which is where a
    // fixed-size `temp[64]` scratch buffer would be most stressed.
    for &p1 in INTERESTING {
        for &p2 in INTERESTING {
            compare(&pair, p1, p2, 0, 0);
            compare(&pair, 0, 0, p1, p2);
            compare(&pair, p1, p2, p2, p1);
        }
    }
}

#[test]
fn buffapp_widest_log_lines() {
    let pair = load_pair();
    // Deliberately pick op codes that produce the longest operation names and
    // 11-character operands on every log line.
    let wide = [c_int::MIN, c_int::MIN + 1, -2_000_000_000, 2_000_000_000, c_int::MAX];
    for &a in &wide {
        for &b in &wide {
            for &c in &wide {
                for &d in &wide {
                    compare(&pair, a, b, c, d);
                }
            }
        }
    }
    // `subtract` (8 chars) with maximal operands on both operation lines.
    for &a in &wide {
        for &b in &wide {
            compare(&pair, a.wrapping_sub(a.wrapping_rem(4)).wrapping_add(1), b, 1, b);
        }
    }
}

#[test]
fn buffapp_pseudorandom_sweep() {
    let pair = load_pair();
    // xorshift64* keeps the sweep deterministic and reproducible.
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        (s.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32 as c_int
    };
    for _ in 0..600 {
        let (a, b, c, d) = (next(), next(), next(), next());
        compare(&pair, a, b, c, d);
        // Also probe small values, where the divide/zero branches are reachable.
        compare(&pair, a % 9, b % 5, c % 9, d % 5);
    }
}

#[test]
fn buffapp_zero_intermediate_fallback() {
    let pair = load_pair();
    // intermediate3 == 0 takes the `param1 + param2 + param3 + param4` branch.
    // p1 % 4 == 0 -> add, so p1 = -p2 makes intermediate1 zero.
    for &(a, b) in &[(4, -4), (8, -8), (0, 0), (100, -100), (-4, 4), (-8, 8)] {
        for &(c, d) in &[(1, 1), (2, 3), (3, 0), (7, 7), (-1, -1)] {
            compare(&pair, a, b, c, d);
        }
    }
}

#[test]
fn buffapp_repeated_calls_are_stateless() {
    let pair = load_pair();
    // The buffer is created and destroyed per call, so results must not drift.
    let (first_rc, first_out) = run(&pair.rs, 6, 7, 9, 3);
    for _ in 0..25 {
        let (rc, out) = run(&pair.rs, 6, 7, 9, 3);
        assert_eq!(rc, first_rc, "Rust buffapp return drifted across calls");
        assert_eq!(out, first_out, "Rust buffapp stdout drifted across calls");
    }
    let (c_rc, c_out) = run(&pair.c, 6, 7, 9, 3);
    assert_eq!(c_rc, first_rc);
    assert_eq!(c_out, first_out);
}

#[test]
fn buffapp_capture_is_not_vacuous() {
    // Guards the harness itself: if the fd redirection silently captured
    // nothing, every stdout assertion above would pass for the wrong reason.
    let pair = load_pair();
    let (_, out_c) = run(&pair.c, 6, 7, 9, 3);
    let (_, out_r) = run(&pair.rs, 6, 7, 9, 3);

    assert!(!out_c.is_empty(), "captured no stdout from the C library");
    assert!(!out_r.is_empty(), "captured no stdout from the Rust library");

    let text = String::from_utf8(out_c.clone()).expect("log is ASCII");
    assert!(text.starts_with("Computation Log:\n"), "unexpected log header: {text:?}");
    assert!(text.contains("Starting computation with 4 parameters\n"));
    assert!(text.contains("Operation 1: "));
    assert!(text.contains("Operation 2: "));
    assert!(text.contains("Operation 3: multiply("));
    assert!(text.contains("Final result: "));
    assert!(text.ends_with('\n'));
    assert_eq!(out_c, out_r);

    // The log must be reproduced exactly, trailing blank line included.
    let expected = "Computation Log:\nStarting computation with 4 parameters\n\
                    Operation 1: multiply(6, 7)\nOperation 2: subtract(9, 3)\n\
                    Operation 3: multiply(42, 6)\nFinal result: 0\n\n";
    assert_eq!(text, expected, "golden log mismatch");
}
