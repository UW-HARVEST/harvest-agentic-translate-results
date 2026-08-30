//! Phase C — error-path differential tests, one sub-test per `ERRORS.md` row.
//!
//! Each row constructs the exact invalid input the C code rejects and asserts
//! both implementations return the *same* sentinel (`1`) **and** print the
//! *same* diagnostic. Asserting the message, not just "both failed", is what
//! catches a translation that picks the wrong branch — the three rejections
//! are only distinguishable by their text.

mod common;

use common::*;

const N: usize = 200;

const ERR_START: &[u8] = b"Error: start is off the end of the string!\n";
const ERR_STOP_END: &[u8] = b"Error: stop is off the end of the string!\n";
const ERR_ORDER: &[u8] = b"Error: stop must come after start!\n";

/// Asserts C and Rust agree *and* that they produced the specific rejection.
#[track_caller]
fn assert_rejects(ctx: &str, payload: &[u8], start: Option<i32>, stop: Option<i32>, msg: &[u8]) {
    let out = assert_same_str(ctx, payload, start, stop);
    assert_eq!(out.ret, 1, "{ctx}: expected the rejection sentinel 1");
    assert_eq!(
        out.stdout,
        msg,
        "{ctx}: wrong rejection branch\n  expected: {:?}\n  actual:   {:?}",
        String::from_utf8_lossy(msg),
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn phase_c_errors() {
    silence_panic_hook();
    let mut s = Suite::new("phase_c_errors");

    // -- E1: start strictly greater than len ------------------------------
    s.row("E1 e1_start_past_end", || {
        let mut rng = Rng::new(0xE1);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            // Any value in (len, INT_MAX].
            let over = rng.range(1, 1000);
            let start = (len + over) as i32;
            assert_rejects(
                &format!("E1/#{i}/len={len}/start={start}"),
                &p,
                Some(start),
                None,
                ERR_START,
            );
        }
    });

    // -- E2: negative start (int → size_t conversion makes it huge) -------
    s.row("E2 e2_start_negative", || {
        let mut rng = Rng::new(0xE2);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            let start = -(rng.range(1, 100_000) as i32);
            assert_rejects(
                &format!("E2/#{i}/len={len}/start={start}"),
                &p,
                Some(start),
                None,
                ERR_START,
            );
        }
        // The two extremes explicitly.
        assert_rejects("E2/minus1", b"hello", Some(-1), None, ERR_START);
        assert_rejects("E2/int_min", b"hello", Some(i32::MIN), None, ERR_START);
        // Negative start also wins over an otherwise-valid stop.
        assert_rejects("E2/with_stop", b"hello", Some(-1), Some(3), ERR_START);
    });

    // -- E3: stop strictly greater than len -------------------------------
    s.row("E3 e3_stop_past_end", || {
        let mut rng = Rng::new(0xE3);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            let over = rng.range(1, 1000);
            let stop = (len + over) as i32;
            // With a NULL start and with an explicit valid start.
            assert_rejects(
                &format!("E3/#{i}/null_start/len={len}/stop={stop}"),
                &p,
                None,
                Some(stop),
                ERR_STOP_END,
            );
            let start = rng.range(0, len) as i32;
            assert_rejects(
                &format!("E3/#{i}/start={start}/len={len}/stop={stop}"),
                &p,
                Some(start),
                Some(stop),
                ERR_STOP_END,
            );
        }
    });

    // -- E4: negative stop reports "off the end", NOT "must come after" ----
    // This is the subtlest branch in the library: the range check converts to
    // `size_t` (so a negative wraps to huge and trips first), while the
    // ordering check below it is a plain signed comparison.
    s.row("E4 e4_stop_negative", || {
        let mut rng = Rng::new(0xE4);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            let stop = -(rng.range(1, 100_000) as i32);
            assert_rejects(
                &format!("E4/#{i}/null_start/len={len}/stop={stop}"),
                &p,
                None,
                Some(stop),
                ERR_STOP_END,
            );
            let start = rng.range(0, len) as i32;
            assert_rejects(
                &format!("E4/#{i}/start={start}/stop={stop}"),
                &p,
                Some(start),
                Some(stop),
                ERR_STOP_END,
            );
        }
        assert_rejects("E4/minus1", b"hello", None, Some(-1), ERR_STOP_END);
        assert_rejects("E4/int_min", b"hello", None, Some(i32::MIN), ERR_STOP_END);
    });

    // -- E5: stop < start (both in range) ---------------------------------
    s.row("E5 e5_stop_before_start", || {
        let mut rng = Rng::new(0xE5);
        for i in 0..N {
            let len = rng.range(1, 64);
            let p = rng.payload(len, ASCII);
            let start = rng.range(1, len) as i32;
            let stop = rng.range(0, start as usize - 1) as i32;
            assert_rejects(
                &format!("E5/#{i}/len={len}/{start}>{stop}"),
                &p,
                Some(start),
                Some(stop),
                ERR_ORDER,
            );
        }
    });

    // -- E6: stop == start (the check is `<=`, so empty slices are rejected)
    s.row("E6 e6_stop_equals_start", || {
        let mut rng = Rng::new(0xE6);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            let v = rng.range(0, len) as i32;
            assert_rejects(
                &format!("E6/#{i}/len={len}/both={v}"),
                &p,
                Some(v),
                Some(v),
                ERR_ORDER,
            );
        }
    });

    // -- E7: NULL start (⇒ 0) with stop == 0 ------------------------------
    s.row("E7 e7_null_start_zero_stop", || {
        let mut rng = Rng::new(0xE7);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            assert_rejects(
                &format!("E7/#{i}/len={len}"),
                &p,
                None,
                Some(0),
                ERR_ORDER,
            );
        }
    });

    // -- E8: empty string with stop == 0 ----------------------------------
    s.row("E8 e8_empty_string_zero_stop", || {
        assert_rejects("E8/null_start", b"", None, Some(0), ERR_ORDER);
        assert_rejects("E8/zero_start", b"", Some(0), Some(0), ERR_ORDER);
    });

    // -- E9: empty string with start == 1 (one past the only valid value) --
    s.row("E9 e9_empty_string_start_one", || {
        assert_rejects("E9/stop_null", b"", Some(1), None, ERR_START);
        assert_rejects("E9/stop_zero", b"", Some(1), Some(0), ERR_START);
        assert_rejects("E9/stop_one", b"", Some(1), Some(1), ERR_START);
    });

    // -- E10: start == len (accepted) but any in-range stop then fails -----
    s.row("E10 e10_start_at_len_with_stop", || {
        let mut rng = Rng::new(0xE10);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            let stop = rng.range(0, len) as i32;
            assert_rejects(
                &format!("E10/#{i}/len={len}/stop={stop}"),
                &p,
                Some(len as i32),
                Some(stop),
                ERR_ORDER,
            );
        }
    });

    // -- E11: ordering — the start check runs before anything about stop ---
    s.row("E11 e11_both_invalid_start_wins", || {
        let mut rng = Rng::new(0xE11);
        for i in 0..N {
            let len = rng.range(0, 32);
            let p = rng.payload(len, ASCII);
            let bad_start = (len + rng.range(1, 50)) as i32;
            let bad_stop = (len + rng.range(1, 50)) as i32;
            assert_rejects(
                &format!("E11/#{i}/both_over"),
                &p,
                Some(bad_start),
                Some(bad_stop),
                ERR_START,
            );
            // Also with a negative stop, and with both negative.
            assert_rejects(
                &format!("E11/#{i}/over_start_neg_stop"),
                &p,
                Some(bad_start),
                Some(-1),
                ERR_START,
            );
            assert_rejects(
                &format!("E11/#{i}/both_neg"),
                &p,
                Some(-1),
                Some(-1),
                ERR_START,
            );
        }
    });

    // -- E12: ordering — stop's range check runs before the ordering check --
    s.row("E12 e12_stop_range_before_order", || {
        let mut rng = Rng::new(0xE12);
        for i in 0..N {
            let len = rng.range(1, 64);
            let p = rng.payload(len, ASCII);
            let start = rng.range(0, len) as i32;
            // A negative stop is both out of range AND <= start; the range
            // message must win.
            assert_rejects(
                &format!("E12/#{i}/neg_stop"),
                &p,
                Some(start),
                Some(-1),
                ERR_STOP_END,
            );
            // A too-large stop is out of range but > start; also the range
            // message (this direction is unambiguous, kept as a control).
            assert_rejects(
                &format!("E12/#{i}/over_stop"),
                &p,
                Some(start),
                Some(len as i32 + 1),
                ERR_STOP_END,
            );
        }
    });

    // -- E13: mystr == NULL — the C code has no null check ----------------
    s.row("E13 e13_null_string_faults", || {
        let c = run_slice_in_child(Impl::C, std::ptr::null_mut(), None, None);
        let r = run_slice_in_child(Impl::Rust, std::ptr::null_mut(), None, None);
        assert_eq!(
            c, r,
            "E13: NULL mystr must terminate both implementations identically \
             (C={c:?}, Rust={r:?})"
        );
        // Sanity-check the child harness itself against a valid input, so a
        // pass above cannot come from the fork machinery silently failing.
        let mut ok = *b"hello\0";
        let c_ok = run_slice_in_child(Impl::C, ok.as_mut_ptr() as *mut i8, None, None);
        let r_ok = run_slice_in_child(Impl::Rust, ok.as_mut_ptr() as *mut i8, None, None);
        assert_eq!(c_ok, ChildOutcome::Exited(0), "child harness control (C)");
        assert_eq!(r_ok, ChildOutcome::Exited(0), "child harness control (Rust)");
        assert!(
            matches!(c, ChildOutcome::Signalled(_)),
            "E13: expected a fatal signal for NULL mystr, got {c:?}"
        );
    });

    // -- G2: INT_MAX bounds ------------------------------------------------
    s.row("G2 g2_int_max_bounds", || {
        for payload in [&b""[..], &b"a"[..], &b"hello world"[..]] {
            assert_rejects("G2/start", payload, Some(i32::MAX), None, ERR_START);
            assert_rejects("G2/stop", payload, None, Some(i32::MAX), ERR_STOP_END);
            assert_rejects("G2/both", payload, Some(i32::MAX), Some(i32::MAX), ERR_START);
            assert_rejects("G2/stop_only", payload, Some(0), Some(i32::MAX), ERR_STOP_END);
        }
    });

    // -- G3: INT_MIN bounds ------------------------------------------------
    s.row("G3 g3_int_min_bounds", || {
        for payload in [&b""[..], &b"a"[..], &b"hello world"[..]] {
            assert_rejects("G3/start", payload, Some(i32::MIN), None, ERR_START);
            assert_rejects("G3/stop", payload, None, Some(i32::MIN), ERR_STOP_END);
            assert_rejects("G3/both", payload, Some(i32::MIN), Some(i32::MIN), ERR_START);
            assert_rejects("G3/stop_only", payload, Some(0), Some(i32::MIN), ERR_STOP_END);
        }
    });

    // -- G4: the full zero-length matrix ----------------------------------
    s.row("G4 g4_empty_string_matrix", || {
        let bounds = [None, Some(-1), Some(0), Some(1), Some(2), Some(i32::MIN), Some(i32::MAX)];
        for a in bounds {
            for b in bounds {
                // Only agreement is asserted here; which branch fires is
                // covered by E7–E9 above.
                assert_same_str(&format!("G4/{a:?}/{b:?}"), b"", a, b);
            }
        }
    });

    // -- G5: one step past the range on both sides, many lengths ----------
    s.row("G5 g5_off_by_one_matrix", || {
        let mut rng = Rng::new(0x6501);
        for len in 0usize..=16 {
            let p = rng.payload(len, ASCII);
            let l = len as i32;
            let bounds = [None, Some(-1), Some(0), Some(l - 1), Some(l), Some(l + 1)];
            for a in bounds {
                for b in bounds {
                    assert_same_str(&format!("G5/len={len}/{a:?}/{b:?}"), &p, a, b);
                }
            }
        }
    });

    // -- G6: arbitrary int bit patterns across the FFI boundary -----------
    // `slice` takes no enums, so the analogue of "an enum value with no valid
    // variant" is an arbitrary `int` in the bound pointers. Fuzz the full
    // `i32` range, mixing wild values with in-range ones.
    s.row("G6 g6_full_int_range_fuzz", || {
        let mut rng = Rng::new(0x6666);
        for i in 0..4000 {
            let len = rng.range(0, 24);
            let p = rng.payload(len, ASCII);

            let pick = |rng: &mut Rng| -> Option<i32> {
                match rng.below(6) {
                    0 => None,
                    1 => Some(rng.i32()),                    // anywhere in i32
                    2 => Some(rng.range(0, len) as i32),     // in range
                    3 => Some(len as i32 + rng.below(4) as i32), // near the boundary
                    4 => Some(-(rng.below(4) as i32)),       // near zero, negative
                    _ => Some(if rng.below(2) == 0 { i32::MIN } else { i32::MAX }),
                }
            };
            let a = pick(&mut rng);
            let b = pick(&mut rng);
            assert_same_str(&format!("G6/#{i}/len={len}/{a:?}/{b:?}"), &p, a, b);
        }
    });

    // -- G7: arguments are never written through --------------------------
    // `assert_same` checks this on every single call in the whole suite; this
    // row makes the guarantee explicit for the mutable `char *` in particular.
    s.row("G7 g7_arguments_not_mutated", || {
        let mut rng = Rng::new(0x7777);
        for i in 0..N {
            let len = rng.range(1, 40);
            let p = rng.payload(len, ASCII);
            for (a, b) in [
                (None, None),
                (Some(0), Some(len as i32)),
                (Some(-1), None),
                (Some(len as i32 + 1), Some(-5)),
            ] {
                let out = assert_same_str(&format!("G7/#{i}"), &p, a, b);
                let mut expect = p.clone();
                expect.push(0);
                assert_eq!(out.buf_after, expect, "G7: mystr was modified");
                assert_eq!(out.start_after, a, "G7: *start_ptr was modified");
                assert_eq!(out.stop_after, b, "G7: *stop_ptr was modified");
            }
        }
    });

    // -- G8: aliased bound pointers (`slice(s, &n, &n)`) -------------------
    // A real C caller can pass the same object twice. `start == stop` then
    // always trips the ordering check, unless the value is out of range and
    // the start check fires first.
    s.row("G8 g8_aliased_bound_pointers", || {
        let mut rng = Rng::new(0x8888);
        for i in 0..N {
            let len = rng.range(0, 40);
            let p = rng.payload(len, ASCII);
            for value in [
                0,
                (len / 2) as i32,
                len as i32,
                len as i32 + 1,
                -1,
                i32::MIN,
                i32::MAX,
                rng.i32(),
            ] {
                let out = assert_same_aliased(&format!("G8/#{i}/len={len}/v={value}"), &p, value);
                assert_eq!(out.ret, 1, "G8: aliased bounds always reject");
                let expected: &[u8] = if (value as usize) > len {
                    ERR_START
                } else {
                    ERR_ORDER
                };
                assert_eq!(
                    out.stdout, expected,
                    "G8/len={len}/v={value}: wrong branch"
                );
            }
        }
    });

    s.finish();
}

// ---------------------------------------------------------------------------
// Forked-child runner for the undefined-behaviour row (E13)
// ---------------------------------------------------------------------------

/// How a forked child terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ChildOutcome {
    Exited(i32),
    Signalled(i32),
}

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// Calls `slice` in a forked child and reports how the child died.
///
/// `slice(NULL, …)` dereferences a null pointer, which cannot be observed
/// in-process without taking the test runner down with it. Forking isolates the
/// fault so the C and Rust termination modes can be compared directly.
fn run_slice_in_child(
    which: Impl,
    mystr: *mut i8,
    start: Option<i32>,
    stop: Option<i32>,
) -> ChildOutcome {
    let f = slice_fn(which);
    let mut start_val = start.unwrap_or(0);
    let mut stop_val = stop.unwrap_or(0);
    let start_p: *mut i32 = if start.is_some() {
        &mut start_val
    } else {
        std::ptr::null_mut()
    };
    let stop_p: *mut i32 = if stop.is_some() {
        &mut stop_val
    } else {
        std::ptr::null_mut()
    };

    // Flush first: buffered parent output must not be duplicated by the child.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let _ = std::io::Write::flush(&mut std::io::stderr());

    // SAFETY: after `fork` the child performs only the FFI call and `_exit`,
    // never returning into the test harness or running destructors.
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            let rc = f(mystr as *mut std::os::raw::c_char, start_p, stop_p);
            _exit(if rc == 0 { 0 } else { 10 + rc });
        }
        let mut status: i32 = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid() failed");

        // Decode without libc's macros: low 7 bits are the signal, and
        // 0x7f in them means "exited" with the code in bits 8..16.
        let sig = status & 0x7f;
        if sig == 0 {
            ChildOutcome::Exited((status >> 8) & 0xff)
        } else {
            ChildOutcome::Signalled(sig)
        }
    }
}
