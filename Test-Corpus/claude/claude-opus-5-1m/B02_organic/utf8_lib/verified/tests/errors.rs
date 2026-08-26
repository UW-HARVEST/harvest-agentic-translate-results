//! Phase C -- error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Every test constructs the exact invalid
//! input/condition described by the row, calls BOTH the C `.so` and the Rust
//! `.so`, and asserts they produce the *same* rejection (same returned offset,
//! same NULL, same fatal signal) -- not merely "both failed somehow".

mod common;

use common::*;
use std::ffi::c_char;

/// Prefixes used to exercise the rejection at offset 0 *and* at a non-zero
/// offset (the latter drives `w_utf8_filter`'s prefix `memcpy`).
const PREFIXES: [&[u8]; 5] = [
    b"",
    b"a",
    b"abcdefgh",
    b"\xC3\xA9x\xE2\x82\xAC",
    b"\xF0\x9F\x98\x80\x7F",
];

/// Assert that `seq`, appended to each prefix, is rejected exactly at the
/// prefix boundary -- by BOTH implementations -- and that `w_utf8_filter` agrees
/// for both `replacement` values.
#[track_caller]
fn assert_rejects(seq: &[u8]) {
    for pre in PREFIXES {
        let mut v = pre.to_vec();
        v.extend_from_slice(seq);
        let buf = cstr(&v);
        let off = cmp_drop(&buf);
        assert_eq!(
            off,
            pre.len(),
            "expected rejection at offset {} but got {off}; buffer = [{}]",
            pre.len(),
            hex(&buf)
        );
        cmp_filter(&buf, 0);
        cmp_filter(&buf, 1);
    }
}

/// Assert that `seq` is fully accepted (no embedded NUL allowed).
#[track_caller]
fn assert_accepts(seq: &[u8]) {
    assert!(!seq.contains(&0));
    for pre in PREFIXES {
        let mut v = pre.to_vec();
        v.extend_from_slice(seq);
        let buf = cstr(&v);
        let off = cmp_drop(&buf);
        assert_eq!(
            off,
            pre.len() + seq.len(),
            "expected full acceptance; buffer = [{}]",
            hex(&buf)
        );
        cmp_filter(&buf, 0);
        cmp_filter(&buf, 1);
    }
}

// ===========================================================================
// row 1 -- w_utf8_drop(NULL) => assert() => SIGABRT
// ===========================================================================
#[test]
fn abort_on_null_drop() {
    let p = pair();
    let cf = p.c.utf8_drop;
    let rf = p.rs.utf8_drop;
    let c = in_child(true, move || unsafe {
        let _ = cf(std::ptr::null());
        libc::_exit(77);
    });
    let r = in_child(true, move || unsafe {
        let _ = rf(std::ptr::null());
        libc::_exit(77);
    });
    assert_eq!(
        c,
        Exit::Signal(libc::SIGABRT),
        "C w_utf8_drop(NULL) should abort, got {c:?}"
    );
    assert_eq!(r, c, "Rust w_utf8_drop(NULL) must abort exactly like C");
}

// ===========================================================================
// row 2 -- w_utf8_filter(NULL, _) => assert() => SIGABRT
// ===========================================================================
#[test]
fn abort_on_null_filter() {
    let p = pair();
    for repl in [0u32, 1, 0xFF] {
        let cf = p.c.utf8_filter;
        let rf = p.rs.utf8_filter;
        let c = in_child(true, move || unsafe {
            let _ = cf(std::ptr::null(), repl);
            libc::_exit(77);
        });
        let r = in_child(true, move || unsafe {
            let _ = rf(std::ptr::null(), repl);
            libc::_exit(77);
        });
        assert_eq!(
            c,
            Exit::Signal(libc::SIGABRT),
            "C w_utf8_filter(NULL,{repl}) should abort, got {c:?}"
        );
        assert_eq!(
            r, c,
            "Rust w_utf8_filter(NULL,{repl}) must abort exactly like C"
        );
    }
}

// ===========================================================================
// rows 1 + 2 (continued) -- the abort *message* must match too
// ===========================================================================
#[test]
fn abort_message_matches() {
    let p = pair();

    // w_utf8_drop
    let cf = p.c.utf8_drop;
    let rf = p.rs.utf8_drop;
    let (cs, cerr) = in_child_capture_stderr(move || unsafe {
        let _ = cf(std::ptr::null());
        libc::_exit(77);
    });
    let (rs, rerr) = in_child_capture_stderr(move || unsafe {
        let _ = rf(std::ptr::null());
        libc::_exit(77);
    });
    assert_eq!(cs, Exit::Signal(libc::SIGABRT));
    assert_eq!(rs, cs);
    assert_eq!(
        String::from_utf8_lossy(&cerr),
        String::from_utf8_lossy(&rerr),
        "w_utf8_drop(NULL): __assert_fail arguments (file/line/function/expression) must match"
    );
    assert!(
        cerr.windows(14).any(|w| w == b"string != NULL"),
        "unexpected C assert text: {}",
        String::from_utf8_lossy(&cerr)
    );
    assert!(
        cerr.windows(13).any(|w| w == b"w_utf8_drop: "),
        "C assert text should name the function: {}",
        String::from_utf8_lossy(&cerr)
    );

    // w_utf8_filter
    let cf = p.c.utf8_filter;
    let rf = p.rs.utf8_filter;
    let (cs, cerr) = in_child_capture_stderr(move || unsafe {
        let _ = cf(std::ptr::null(), 1);
        libc::_exit(77);
    });
    let (rs, rerr) = in_child_capture_stderr(move || unsafe {
        let _ = rf(std::ptr::null(), 1);
        libc::_exit(77);
    });
    assert_eq!(cs, Exit::Signal(libc::SIGABRT));
    assert_eq!(rs, cs);
    assert_eq!(
        String::from_utf8_lossy(&cerr),
        String::from_utf8_lossy(&rerr),
        "w_utf8_filter(NULL, _): __assert_fail arguments must match"
    );
    assert!(
        cerr.windows(15).any(|w| w == b"w_utf8_filter: "),
        "C assert text should name the function: {}",
        String::from_utf8_lossy(&cerr)
    );
}

// ===========================================================================
// row 3 -- malloc() failure => NULL
// ===========================================================================
const NULL_RESULT: i32 = 1;
const NON_NULL_RESULT: i32 = 0;

/// ~32 MiB of address-space head-room is plenty: `exhaust_allocator` drains it
/// with a descending ladder of allocation sizes in a handful of iterations.
const HEADROOM: usize = 32 * 1024 * 1024;

/// A 4 KiB input whose *first* byte is invalid, so `w_utf8_drop` returns offset
/// 0 and `w_utf8_filter` is forced down the `malloc(strlen + 1)` path.
fn alloc_test_input() -> Vec<u8> {
    let mut v = vec![b'a'; 4096];
    v[0] = 0x80;
    v.push(0);
    v
}

/// Run `f(base, repl)` in a forked child in which every allocation fails.
/// `reserve` is the exact number of bytes that is `malloc`ed *before* the
/// allocator is drained and released again just before the call, so that the
/// library's first `malloc(reserve)` still succeeds while everything after it
/// (i.e. `realloc`) fails.  `reserve == 0` drains everything, so even the first
/// `malloc` fails.
fn run_under_alloc_failure(
    f: FilterFn,
    base: *const c_char,
    repl: u32,
    reserve: usize,
) -> Exit {
    in_child(true, move || unsafe {
        let keep = if reserve > 0 {
            libc::malloc(reserve)
        } else {
            std::ptr::null_mut()
        };
        if reserve > 0 && keep.is_null() {
            libc::_exit(65);
        }
        exhaust_allocator(HEADROOM);
        if !keep.is_null() {
            // hands back exactly one chunk of exactly the size the library is
            // about to request, so malloc() succeeds and realloc() cannot grow
            libc::free(keep);
        }
        let r = f(base, repl);
        libc::_exit(if r.is_null() {
            NULL_RESULT
        } else {
            NON_NULL_RESULT
        });
    })
}

#[test]
fn malloc_failure_returns_null() {
    let p = pair();
    let v = alloc_test_input();
    let base = v.as_ptr() as *const c_char;

    for repl in [0u32, 1] {
        let c = run_under_alloc_failure(p.c.utf8_filter, base, repl, 0);
        let r = run_under_alloc_failure(p.rs.utf8_filter, base, repl, 0);
        assert_ne!(
            c,
            Exit::Code(EXIT_SETRLIMIT_FAILED),
            "setrlimit failed in the C child"
        );
        assert_ne!(
            r,
            Exit::Code(EXIT_SETRLIMIT_FAILED),
            "setrlimit failed in the Rust child"
        );
        assert_eq!(
            c,
            Exit::Code(NULL_RESULT),
            "C w_utf8_filter must return NULL when malloc fails (repl={repl}), got {c:?}"
        );
        assert_eq!(
            r, c,
            "Rust w_utf8_filter must return NULL exactly like C when malloc fails (repl={repl})"
        );
    }
}

// ===========================================================================
// row 4 -- realloc() failure => NULL
// ===========================================================================
#[test]
fn realloc_failure_returns_null() {
    let p = pair();
    let v = alloc_test_input();
    let base = v.as_ptr() as *const c_char;
    // strlen + 1 -- exactly what w_utf8_filter's malloc() asks for
    let reserve = v.len();

    // replacement enabled => the very first rejected byte hits `repl < 3` and
    // calls realloc(copy, size + REPLACEMENT_INC), which must fail.
    let c = run_under_alloc_failure(p.c.utf8_filter, base, 1, reserve);
    let r = run_under_alloc_failure(p.rs.utf8_filter, base, 1, reserve);
    assert_ne!(c, Exit::Code(65), "reserve malloc failed in the C child");
    assert_ne!(r, Exit::Code(65), "reserve malloc failed in the Rust child");
    assert_ne!(
        c,
        Exit::Code(EXIT_SETRLIMIT_FAILED),
        "setrlimit failed in the C child"
    );
    assert_eq!(
        c,
        Exit::Code(NULL_RESULT),
        "C w_utf8_filter must return NULL when realloc fails, got {c:?}"
    );
    assert_eq!(
        r, c,
        "Rust w_utf8_filter must return NULL exactly like C when realloc fails"
    );

    // Control: same input and same allocator state, but replacement disabled =>
    // no realloc at all, so BOTH must succeed.  This proves the previous
    // assertion really exercised the realloc branch and not the malloc one.
    let c0 = run_under_alloc_failure(p.c.utf8_filter, base, 0, reserve);
    let r0 = run_under_alloc_failure(p.rs.utf8_filter, base, 0, reserve);
    assert_eq!(
        c0,
        Exit::Code(NON_NULL_RESULT),
        "control: C should succeed without replacement, got {c0:?}"
    );
    assert_eq!(r0, c0, "control: Rust must match C without replacement");
}

// ===========================================================================
// rows 5..20 -- the validation-clause rejection table
// ===========================================================================
#[test]
fn rejection_table() {
    // ---- row 5: valid_1 fails and no multi-byte form matches -----------
    // every byte >= 0x80 standing alone is rejected (all multi-byte forms are
    // truncated by the terminator)
    for b in 0x80u8..=0xFF {
        assert_rejects(&[b]);
    }

    // ---- row 6: lone continuation byte 0x80..0xBF ---------------------
    for b in 0x80u8..=0xBF {
        assert_rejects(&[b, 0x41]);
        assert_rejects(&[b, 0x80, 0x80, 0x80]);
    }

    // ---- row 7: valid_2 clause 1, (s[0] & 0xE0) != 0xC0 ---------------
    for lead in 0x80u8..=0xFF {
        if (lead & 0xE0) != 0xC0 {
            // followed by one continuation byte: valid_2's mask test fails, and
            // valid_3/valid_4 are truncated => rejection
            assert_rejects(&[lead, 0x80]);
        }
    }

    // ---- row 8: valid_2 clause 2, overlong leads 0xC0 / 0xC1 ----------
    for lead in [0xC0u8, 0xC1] {
        for b1 in 0u8..=0xFF {
            assert_rejects(&[lead, b1, 0x41]);
        }
    }

    // ---- row 9: valid_2 clause 3, bad/absent continuation -------------
    for lead in 0xC2u8..=0xDF {
        for b1 in 0u8..=0xFF {
            if (b1 & 0xC0) == 0x80 {
                assert_accepts(&[lead, b1]);
            } else if b1 == 0 {
                assert_rejects(&[lead, 0]); // truncation by the terminator
            } else {
                assert_rejects(&[lead, b1, 0x41]);
            }
        }
    }

    // ---- row 10: valid_3 clause 1, (s[0] & 0xF0) != 0xE0 --------------
    for lead in 0x80u8..=0xBF {
        assert_rejects(&[lead, 0x80, 0x80, 0x80]);
    }
    // a lead that fails valid_3 clause 1 but is picked up by valid_4
    assert_accepts(&[0xF1, 0x80, 0x80, 0x80]);
    // a lead that fails valid_3 clause 1 but is picked up by valid_2
    assert_accepts(&[0xC2, 0x80]);

    // ---- row 11: valid_3 clause 2, (s[1] & 0xC0) != 0x80 --------------
    for lead in 0xE0u8..=0xEF {
        for b1 in 0u8..=0xFF {
            if (b1 & 0xC0) != 0x80 {
                if b1 == 0 {
                    assert_rejects(&[lead, 0]);
                } else {
                    assert_rejects(&[lead, b1, 0x80, 0x41]);
                }
            }
        }
    }

    // ---- row 12: valid_3 clause 3, (s[2] & 0xC0) != 0x80 --------------
    for lead in 0xE0u8..=0xEF {
        // pick a second byte that satisfies this lead's own guards
        let b1 = match lead {
            0xE0 => 0xA0,
            0xED => 0x9F,
            _ => 0x80,
        };
        for b2 in 0u8..=0xFF {
            if (b2 & 0xC0) == 0x80 {
                assert_accepts(&[lead, b1, b2]);
            } else if b2 == 0 {
                assert_rejects(&[lead, b1, 0]);
            } else {
                assert_rejects(&[lead, b1, b2, 0x41]);
            }
        }
    }

    // ---- row 13: valid_3 clause 4, overlong 0xE0 0x80..0x9F -----------
    for b1 in 0x80u8..=0x9F {
        assert_rejects(&[0xE0, b1, 0x80]);
        assert_rejects(&[0xE0, b1, 0xBF]);
    }
    for b1 in 0xA0u8..=0xBF {
        assert_accepts(&[0xE0, b1, 0x80]);
    }

    // ---- row 14: valid_3 clause 5, surrogates 0xED 0xA0..0xBF ---------
    for b1 in 0xA0u8..=0xBF {
        assert_rejects(&[0xED, b1, 0x80]);
        assert_rejects(&[0xED, b1, 0xBF]);
    }
    for b1 in 0x80u8..=0x9F {
        assert_accepts(&[0xED, b1, 0x80]);
    }

    // ---- row 16: valid_4 clause 1, leads 0xF8..0xFF -------------------
    for lead in 0xF8u8..=0xFF {
        assert_rejects(&[lead, 0x80, 0x80, 0x80]);
        assert_rejects(&[lead, 0x90, 0x80, 0x80]);
    }

    // ---- row 17: valid_4 clause 2, leads 0xF5..0xF7 -------------------
    // (these DO satisfy `(s[0] & 0xF8) == 0xF0`, hence a separate row)
    for lead in 0xF5u8..=0xF7 {
        assert_eq!(lead & 0xF8, 0xF0, "row 17 precondition");
        for b1 in 0x80u8..=0xBF {
            assert_rejects(&[lead, b1, 0x80, 0x80]);
        }
    }

    // ---- row 18: valid_4 clauses 3/4/5, bad or absent continuations ---
    for lead in 0xF0u8..=0xF4 {
        let b1 = match lead {
            0xF0 => 0x90,
            0xF4 => 0x8F,
            _ => 0x80,
        };
        assert_rejects(&[lead]); // truncated after 1 byte
        assert_rejects(&[lead, b1]); // truncated after 2 bytes
        assert_rejects(&[lead, b1, 0x80]); // truncated after 3 bytes
        assert_accepts(&[lead, b1, 0x80, 0x80]);
        for bad in [0x00u8, 0x01, 0x41, 0x7F, 0xC0, 0xE0, 0xF0, 0xFF] {
            if bad == 0 {
                assert_rejects(&[lead, b1, 0x80, 0]);
                assert_rejects(&[lead, b1, 0, 0x80]);
            } else {
                assert_rejects(&[lead, b1, 0x80, bad, 0x41]);
                assert_rejects(&[lead, b1, bad, 0x80, 0x41]);
            }
        }
    }

    // ---- row 19: valid_4 clause 6, overlong 0xF0 0x80..0x8F -----------
    for b1 in 0x80u8..=0x8F {
        assert_rejects(&[0xF0, b1, 0x80, 0x80]);
    }
    for b1 in 0x90u8..=0xBF {
        assert_accepts(&[0xF0, b1, 0x80, 0x80]);
    }

    // ---- row 20: valid_4 clause 7, 0xF4 0x90..0xBF (> U+10FFFF) -------
    for b1 in 0x90u8..=0xBF {
        assert_rejects(&[0xF4, b1, 0x80, 0x80]);
    }
    for b1 in 0x80u8..=0x8F {
        assert_accepts(&[0xF4, b1, 0x80, 0x80]);
    }
}

// ===========================================================================
// row 15 -- valid_3's `0xEF` clause is unreachable and must stay that way
// ===========================================================================
#[test]
fn valid3_ef_clause_unreachable() {
    // Clause 2 already forces (s[1] & 0xC0) == 0x80, i.e. s[1] <= 0xBF, so the
    // extra `s[0] != 0xEF || s[1] <= 0xBF` guard can never reject anything.
    for b1 in 0u8..=0xFF {
        assert!(!((b1 & 0xC0) == 0x80 && b1 > 0xBF), "clause must be dead");
    }
    // Behavioural proof: every 0xEF sequence with well-formed continuations is
    // accepted by both implementations (including the noncharacters U+FFFE/F).
    for b1 in 0x80u8..=0xBF {
        for b2 in 0x80u8..=0xBF {
            assert_accepts(&[0xEF, b1, b2]);
        }
    }
    // ...and rejected for exactly the same reason as any other 0xE_ lead when
    // the continuation bytes are malformed.
    for b1 in 0u8..=0xFF {
        if (b1 & 0xC0) != 0x80 && b1 != 0 {
            assert_rejects(&[0xEF, b1, 0x80, 0x41]);
        }
    }
}

// ===========================================================================
// row 21 -- zero-length input
// ===========================================================================
#[test]
fn empty_string() {
    let buf = cstr(b"");
    assert_eq!(cmp_drop(&buf), 0, "drop(\"\") must point at the terminator");
    for repl in [0u32, 1, 2, 0xFF, 0x100] {
        let out = cmp_filter(&buf, repl);
        assert!(!out.null, "filter(\"\") must not return NULL");
        assert!(out.bytes.is_empty());
    }
}

// ===========================================================================
// row 22 -- replacement == 0 drops the offending byte
// ===========================================================================
#[test]
fn replacement_false_drops() {
    let mut r = Rng::new(0x2222_2222_0000_0022);
    for _ in 0..3000 {
        let mut v = Vec::new();
        let mut expect = Vec::new();
        for _ in 0..(1 + r.below(20)) {
            let before = v.len();
            push_valid_any(&mut v, &mut r);
            expect.extend_from_slice(&v[before..]);
            if r.below(2) == 0 {
                v.push(invalid_byte(&mut r)); // dropped, contributes nothing
            }
        }
        let buf = cstr(&v);
        let out = cmp_filter(&buf, 0);
        assert!(!out.null);
        assert_eq!(
            out.bytes,
            expect,
            "replacement=0 must drop invalid bytes silently\n  in = [{}]",
            hex(&buf)
        );
    }
}

// ===========================================================================
// row 23 -- replacement != 0 emits U+FFFD (EF BF BD)
// ===========================================================================
#[test]
fn replacement_true_emits_fffd() {
    let mut r = Rng::new(0x2323_2323_0000_0023);
    for _ in 0..3000 {
        let mut v = Vec::new();
        let mut expect = Vec::new();
        for _ in 0..(1 + r.below(20)) {
            let before = v.len();
            push_valid_any(&mut v, &mut r);
            expect.extend_from_slice(&v[before..]);
            if r.below(2) == 0 {
                v.push(invalid_byte(&mut r));
                expect.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
            }
        }
        let buf = cstr(&v);
        let out = cmp_filter(&buf, 1);
        assert!(!out.null);
        assert_eq!(
            out.bytes,
            expect,
            "replacement=1 must emit EF BF BD per rejected byte\n  in = [{}]",
            hex(&buf)
        );
    }
}

// ===========================================================================
// row 24 -- non-normalized `_Bool` values across the FFI boundary
// ===========================================================================
#[test]
fn non_normalized_bool() {
    // The C ABI passes `_Bool` in the low byte and gcc emits `cmpb $0x0,...`,
    // so the *entire* 32-bit value space is a legal input from C's point of
    // view.  Both implementations must agree for all of it.
    let seq: &[u8] = b"ok\x80mid\xC0end\xF5!";
    let buf = cstr(seq);
    let want_true = cmp_filter(&buf, 1).bytes;
    let want_false = cmp_filter(&buf, 0).bytes;
    assert_ne!(want_true, want_false);

    let mut r = Rng::new(0x2424_2424_0000_0024);
    for _ in 0..4096 {
        let v = r.next_u32();
        let out = cmp_filter(&buf, v);
        let want = if (v & 0xFF) != 0 {
            &want_true
        } else {
            &want_false
        };
        assert_eq!(
            &out.bytes, want,
            "replacement={v:#010x} (low byte {:#04x})",
            v & 0xFF
        );
    }
    // exhaustive over the low byte, with assorted junk in the upper bits
    for hi in [0x0000_0000u32, 0x0000_0100, 0xDEAD_BE00, 0xFFFF_FF00] {
        for lo in 0u32..=0xFF {
            let v = hi | lo;
            let out = cmp_filter(&buf, v);
            let want = if lo != 0 { &want_true } else { &want_false };
            assert_eq!(&out.bytes, want, "replacement={v:#010x}");
        }
    }
}

// ===========================================================================
// row 26 -- invalid byte at offset 0 => memcpy(copy, string, 0)
// ===========================================================================
#[test]
fn invalid_at_offset_zero() {
    for b in 0x80u8..=0xFF {
        let buf = cstr(&[b]);
        assert_eq!(cmp_drop(&buf), 0);
        let o0 = cmp_filter(&buf, 0);
        let o1 = cmp_filter(&buf, 1);
        assert!(!o0.null && !o1.null);
        assert_eq!(o0.bytes, Vec::<u8>::new());
        assert_eq!(o1.bytes, vec![0xEF, 0xBF, 0xBD]);
    }
    // invalid byte at 0 followed by a long valid tail
    let mut r = Rng::new(0x2626_2626_0000_0026);
    for _ in 0..2000 {
        let mut v = vec![invalid_byte(&mut r)];
        for _ in 0..r.below(40) {
            push_valid_any(&mut v, &mut r);
        }
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), 0);
        cmp_filter(&buf, 0);
        cmp_filter(&buf, 1);
    }
}

// ===========================================================================
// row 27 -- fully valid input takes the strdup() early return
// ===========================================================================
#[test]
fn strdup_path() {
    let p = pair();
    let mut r = Rng::new(0x2727_2727_0000_0027);
    for _ in 0..2000 {
        let mut v = Vec::new();
        for _ in 0..r.below(40) {
            push_valid_any(&mut v, &mut r);
        }
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), v.len());
        for repl in [0u32, 1] {
            let out = cmp_filter(&buf, repl);
            assert!(!out.null, "strdup path must not return NULL");
            assert_eq!(out.bytes, v);
        }
        // the returned buffer must be a *fresh* allocation, not the input
        let base = buf.as_ptr() as *const c_char;
        let a = unsafe { (p.c.utf8_filter)(base, 0) };
        let b = unsafe { (p.rs.utf8_filter)(base, 0) };
        assert!(!a.is_null() && !b.is_null());
        assert_ne!(a as *const c_char, base);
        assert_ne!(b as *const c_char, base);
        unsafe {
            libc::free(a as *mut libc::c_void);
            libc::free(b as *mut libc::c_void);
        }
    }
}

// ===========================================================================
// row 28 -- everything past the first NUL is ignored
// ===========================================================================
#[test]
fn bytes_after_nul_ignored() {
    let mut r = Rng::new(0x2828_2828_0000_0028);
    for _ in 0..3000 {
        // visible part
        let mut visible = Vec::new();
        for _ in 0..r.below(20) {
            push_valid_any(&mut visible, &mut r);
            if r.below(3) == 0 {
                visible.push(invalid_byte(&mut r));
            }
        }
        // full buffer = visible + NUL + junk + NUL
        let mut full = visible.clone();
        full.push(0);
        for _ in 0..(1 + r.below(24)) {
            full.push(r.nonzero_byte());
        }
        full.push(0);

        let vis_buf = cstr(&visible);
        assert_eq!(
            cmp_drop(&full),
            cmp_drop(&vis_buf),
            "drop must stop at the first NUL"
        );
        for repl in [0u32, 1] {
            let want = cmp_filter(&vis_buf, repl).bytes;
            let got = cmp_filter(&full, repl).bytes;
            assert_eq!(got, want, "filter must stop at the first NUL");
        }
    }
}
