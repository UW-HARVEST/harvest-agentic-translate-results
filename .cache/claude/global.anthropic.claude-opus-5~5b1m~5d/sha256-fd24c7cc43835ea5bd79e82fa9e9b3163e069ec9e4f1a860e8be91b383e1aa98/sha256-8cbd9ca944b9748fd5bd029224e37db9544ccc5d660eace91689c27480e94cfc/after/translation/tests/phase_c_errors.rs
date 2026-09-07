//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input or
//! condition, calls BOTH `.so`s through `dlsym`, and asserts the *same*
//! rejection: identical return value (sentinel/error code) and identical bytes
//! on stdout.

mod common;

use common::*;
use std::ffi::CString;

// ---------------------------------------------------------------------------
// ERRORS row 1 — create_result_string: malloc(64) returns NULL
// ---------------------------------------------------------------------------
#[test]
fn err01_create_result_string_malloc_failure() {
    // Encoded as 1 when the callee returned NULL, 0 otherwise. The body runs in
    // a forked child with a drained allocator and uses only pre-resolved raw
    // function pointers, so it performs no allocation of its own.
    for val in [7i32, 0, -1, i32::MIN] {
        let body = move |i: Impl| -> i64 {
            let f = raw(i).create_result_string;
            let p = unsafe { f(c"multiply".as_ptr(), val) };
            // Deliberately leaked; the child is about to `_exit`.
            p.is_null() as i64
        };
        // Guard against a vacuous pass: the C child must really have taken the
        // `str == NULL` branch, i.e. reported 1 and printed nothing.
        let c = run_oom(&format!("crs-oom-{val}"), Impl::C, body);
        assert_eq!(
            c.ret, 1,
            "the drained-heap child did not reach create_result_string's NULL \
             branch (malloc still succeeded); test would be vacuous"
        );
        assert!(c.stdout.is_empty(), "C printed {:?}", show(&c.stdout));
        diff_oom(&format!("crs-oom-{val}"), body);
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 2 — safe_add rejects when perms lack READ_PERM|WRITE_PERM
// ---------------------------------------------------------------------------
#[test]
fn err02_safe_add_insufficient_permissions() {
    let denying: [i32; 14] = [
        0,
        0o400,
        0o200,
        0o100,
        0o444,
        0o222,
        0o111,
        0o177,
        0o066,
        !0o600,
        0o400 | 0o100,
        0o200 | 0o100,
        i32::MIN,
        i32::MAX & !0o200,
    ];
    for perms in denying {
        assert_ne!(perms & 0o600, 0o600, "test bug: {perms:o} grants 0600");
        for (a, b) in [(1, 2), (i32::MAX, i32::MAX), (i32::MIN, -1), (0, 0), (-5, 5)] {
            let (expected_zero, _) = capture(|| safe_add(Impl::C, a, b, perms));
            assert_eq!(expected_zero, 0, "C must return the 0 sentinel");
            diff(&format!("safe_add({a},{b},{perms:o}) denied"), |i| {
                safe_add(i, a, b, perms)
            });
        }
    }
    // One step past the boundary in each direction: exactly 0600 is accepted,
    // 0600 with either required bit cleared is rejected.
    diff("safe_add boundary 0600", |i| safe_add(i, 3, 4, 0o600));
    diff("safe_add boundary 0400", |i| safe_add(i, 3, 4, 0o400));
    diff("safe_add boundary 0200", |i| safe_add(i, 3, 4, 0o200));
}

// ---------------------------------------------------------------------------
// ERRORS row 3 — multiply_with_log when create_result_string returns NULL
// ---------------------------------------------------------------------------
#[test]
fn err03_multiply_with_log_null_log() {
    // Encoding: ret*4 + is_null*2 + untouched. Under a drained heap the C must
    // return 0 with *log_msg == NULL, i.e. encoding 2.
    for (a, b) in [(6i32, 7i32), (0, 0), (i32::MIN, -1)] {
        let body = move |i: Impl| multiply_with_log_encoded(i, a, b);
        let c = run_oom("mwl-oom", Impl::C, body);
        // ret == 0 and *log_msg == NULL and the out-param WAS written.
        assert_eq!(
            c.ret, 2,
            "the drained-heap child did not reach multiply_with_log's NULL-log \
             branch (expected encoding 2 = ret 0 + log NULL + written)"
        );
        assert!(c.stdout.is_empty(), "C printed {:?}", show(&c.stdout));
        diff_oom("mwl-oom", body);
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 4 — multiply_with_log with log_msg == NULL is UB in the C.
// Documented, not executed (it would SIGSEGV the test process). We instead
// assert that both implementations store through the out-parameter
// unconditionally, which is the observable half of that behaviour.
// ---------------------------------------------------------------------------
#[test]
fn err04_multiply_with_log_always_stores_out_param() {
    for (a, b) in [(0, 0), (3, 4), (i32::MAX, 2)] {
        diff(
            &format!("multiply_with_log({a},{b}) writes out-param"),
            |i| {
                let (r, log) = multiply_with_log(i, a, b);
                // `log == None` would mean the out-parameter was left at the
                // poison value, i.e. never written. The C always writes it.
                (r, log.is_some(), log)
            },
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 5 — copy_and_sum with src == NULL
// ---------------------------------------------------------------------------
#[test]
fn err05_copy_and_sum_null_src() {
    for count in [0i32, 1, 3, 64, 1024] {
        let (r, _) = capture(|| copy_and_sum(Impl::C, std::ptr::null_mut(), count));
        assert_eq!(r, -1, "C must return the -1 sentinel");
        diff(&format!("copy_and_sum(NULL, {count})"), |i| {
            copy_and_sum(i, std::ptr::null_mut(), count)
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 6 — NULL src AND invalid count: the NULL check must win
// ---------------------------------------------------------------------------
#[test]
fn err06_copy_and_sum_null_src_and_bad_count() {
    for count in [-1i32, -2, -1024, i32::MIN, i32::MIN + 1, i32::MAX] {
        diff(&format!("copy_and_sum(NULL, {count})"), |i| {
            copy_and_sum(i, std::ptr::null_mut(), count)
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 7 — copy_and_sum where malloc(count * 4) fails (count < 0)
// ---------------------------------------------------------------------------
#[test]
fn err07_copy_and_sum_alloc_failure() {
    let mut buf: Vec<i32> = vec![1, 2, 3, 4];
    let p = buf.as_mut_ptr();
    for count in [-1i32, -2, -3, -4, -8, -1024, i32::MIN, i32::MIN + 1, -0x4000_0000] {
        let (r, _) = capture(|| copy_and_sum(Impl::C, p, count));
        assert_eq!(r, -1, "C must return -1 for count={count}");
        diff(&format!("copy_and_sum(valid, {count})"), |i| {
            copy_and_sum(i, p, count)
        });
    }
    // NOTE: large *positive* counts are deliberately NOT exercised here. Under
    // Linux overcommit, `malloc(count * 4)` for e.g. count == INT_MAX (8 GiB)
    // succeeds, and the C then `memcpy`s 8 GiB out of a 4-element array and
    // SIGSEGVs. That is genuine UB in the C, identical in both libraries, and
    // running it would kill the test process rather than compare anything.
    // The malloc-returns-NULL branch is fully covered by the negative counts
    // above, which make the `size_t` byte count exceed the address space.
}

// ---------------------------------------------------------------------------
// ERRORS row 8 — count == 0 boundary (not an error: returns 0, prints nothing)
// ---------------------------------------------------------------------------
#[test]
fn err08_copy_and_sum_zero_count() {
    let mut buf: Vec<i32> = vec![9, 9, 9];
    let p = buf.as_mut_ptr();
    let (r, out) = capture(|| copy_and_sum(Impl::C, p, 0));
    assert_eq!(r, 0);
    assert!(out.is_empty(), "C printed {:?} for count==0", show(&out));
    diff("copy_and_sum(valid, 0)", |i| copy_and_sum(i, p, 0));
    // One step either side of the boundary.
    diff("copy_and_sum(valid, 1)", |i| copy_and_sum(i, p, 1));
    diff("copy_and_sum(valid, -1)", |i| copy_and_sum(i, p, -1));
}

// ---------------------------------------------------------------------------
// ERRORS row 9 — count beyond the logical length (over-read inside the alloc)
// ---------------------------------------------------------------------------
#[test]
fn err09_copy_and_sum_over_read() {
    let mut buf: Vec<i32> = (0..8192)
        .map(|k| (k as i32).wrapping_mul(0x9E37_79B9u32 as i32))
        .collect();
    let p = buf.as_mut_ptr();
    for count in [3i32, 5, 17, 100, 4096, 8192] {
        diff(&format!("copy_and_sum(over-read {count})"), |i| {
            copy_and_sum(i, p, count)
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 10/11/12 — compare_operations NULL combinations
// ---------------------------------------------------------------------------
#[test]
fn err10_compare_operations_null_first() {
    let valid = CString::new("none").unwrap();
    let (r, _) = capture(|| compare_operations(Impl::C, std::ptr::null(), valid.as_ptr()));
    assert_eq!(r, -1);
    for other in ["", "none", "zzz"] {
        let s = CString::new(other).unwrap();
        diff(&format!("compare_operations(NULL, {other:?})"), |i| {
            compare_operations(i, std::ptr::null(), s.as_ptr())
        });
    }
}

#[test]
fn err11_compare_operations_null_second() {
    for other in ["", "none", "zzz"] {
        let s = CString::new(other).unwrap();
        diff(&format!("compare_operations({other:?}, NULL)"), |i| {
            compare_operations(i, s.as_ptr(), std::ptr::null())
        });
    }
}

#[test]
fn err12_compare_operations_both_null() {
    let (r, _) = capture(|| compare_operations(Impl::C, std::ptr::null(), std::ptr::null()));
    assert_eq!(r, -1);
    diff("compare_operations(NULL, NULL)", |i| {
        compare_operations(i, std::ptr::null(), std::ptr::null())
    });
}

// ---------------------------------------------------------------------------
// ERRORS row 13 — the return value is raw strcmp, not normalised to +/-1
// ---------------------------------------------------------------------------
#[test]
fn err13_compare_operations_raw_strcmp_value() {
    // -1 is also the NULL-rejection sentinel, so the two must not be confused:
    // find an input pair whose genuine strcmp value is negative and check both
    // libraries agree on the exact magnitude, whatever glibc returns.
    let pairs = [
        ("a", "b"),
        ("b", "a"),
        ("a", "z"),
        ("z", "a"),
        ("abc", "abd"),
        ("abd", "abc"),
        ("", "\u{1}"),
        ("A", "a"),
    ];
    for (x, y) in pairs {
        let a = CString::new(x).unwrap();
        let b = CString::new(y).unwrap();
        let (cv, _) = capture(|| compare_operations(Impl::C, a.as_ptr(), b.as_ptr()));
        let (rv, _) = capture(|| compare_operations(Impl::Rust, a.as_ptr(), b.as_ptr()));
        assert_eq!(cv, rv, "raw strcmp value mismatch for ({x:?}, {y:?})");
        diff(&format!("compare_operations({x:?}, {y:?}) raw"), |i| {
            compare_operations(i, a.as_ptr(), b.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 14 — complexmode when malloc(sizeof(Result)) fails
// ---------------------------------------------------------------------------
#[test]
fn err14_complexmode_tracker_alloc_failure() {
    for mode in [1i32, 2, 3, 4, 0, 99] {
        let body = move |i: Impl| {
            let f = raw(i).complexmode;
            unsafe { f(mode, 3, 4, 5) as i64 }
        };
        let c = run_oom(&format!("cm-oom-{mode}"), Impl::C, body);
        assert_eq!(
            c.ret, -1,
            "the drained-heap child did not reach complexmode's tracker-alloc \
             failure for mode {mode}; test would be vacuous"
        );
        assert_eq!(
            c.stdout, b"Failed to allocate result tracker\n",
            "C stdout was {:?}",
            show(&c.stdout)
        );
        diff_oom(&format!("cm-oom-{mode}"), body);
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 15 — the `default:` arm: every out-of-range `mode`
// ---------------------------------------------------------------------------
#[test]
fn err15_complexmode_invalid_mode() {
    // Explicit values incl. one step past each end of the valid 1..=4 range.
    let modes: Vec<i32> = vec![
        0,
        5,
        -1,
        6,
        100,
        -100,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        // Out-of-range values a C `enum` would accept but that have no variant.
        1 << 8,
        1 << 16,
        1 << 24,
        i32::MIN + 4,
    ];
    for mode in modes {
        let (r, _) = capture(|| complexmode(Impl::C, mode, 1, 2, 3));
        assert_eq!(r, -1, "C must return -1 for invalid mode {mode}");
        diff(&format!("complexmode({mode}, 1, 2, 3) invalid"), |i| {
            complexmode(i, mode, 1, 2, 3)
        });
    }
    // Exhaustive sweep of a window straddling the whole valid range.
    for mode in -32i32..=32 {
        diff(&format!("complexmode({mode}, 7, 11, 13) sweep"), |i| {
            complexmode(i, mode, 7, 11, 13)
        });
    }
    // ...and a wide random sweep of arbitrary ints as `mode`.
    let mut rng = Rng::with_seed(SEED ^ 0xF15);
    for _ in 0..500 {
        let mode = rng.any_i32();
        let (a, b, c) = (rng.any_i32(), rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode({mode}, {a}, {b}, {c}) random"), |i| {
            complexmode(i, mode, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 16 — mode 1 never takes safe_add's rejection branch (perms=0644)
// ---------------------------------------------------------------------------
#[test]
fn err16_complexmode_mode1_never_rejected() {
    let (r, out) = capture(|| complexmode(Impl::C, 1, 20, 22, 0));
    assert_eq!(r, 42);
    assert_eq!(
        out,
        b"Mode 1: Addition\nResult: 42\nOperation performed: addition\n",
        "C stdout was {:?}",
        show(&out)
    );
    assert!(
        !String::from_utf8_lossy(&out).contains("Insufficient"),
        "the rejection branch must be unreachable from complexmode"
    );
    let mut rng = Rng::with_seed(SEED ^ 16);
    for _ in 0..200 {
        let (a, b) = (rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode(1, {a}, {b}, 0)"), |i| {
            complexmode(i, 1, a, b, 0)
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 17 — mode 4 always takes the `else` branch (0644 lacks 0100)
// ---------------------------------------------------------------------------
#[test]
fn err17_complexmode_mode4_takes_else_branch() {
    assert_eq!(
        check_permissions(Impl::C, 0o644, 0o100),
        0,
        "0644 must not grant EXEC_PERM"
    );
    assert_eq!(check_permissions(Impl::Rust, 0o644, 0o100), 0);
    // 5*7+9 == 44 via the multiply branch, 5+7+9 == 21 via the else branch.
    let (r, _) = capture(|| complexmode(Impl::C, 4, 5, 7, 9));
    assert_eq!(r, 21, "C must take the addition (else) branch, not 5*7+9");
    diff("complexmode(4, 5, 7, 9)", |i| complexmode(i, 4, 5, 7, 9));
    let mut rng = Rng::with_seed(SEED ^ 17);
    for _ in 0..200 {
        let (a, b, c) = (rng.any_i32(), rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode(4, {a}, {b}, {c})"), |i| {
            complexmode(i, 4, a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 18 — mode 2's "log message creation failed" branch is dead
// because snprintf always writes the "Operation: " prefix
// ---------------------------------------------------------------------------
#[test]
fn err18_complexmode_mode2_log_never_empty() {
    let (r, out) = capture(|| complexmode(Impl::C, 2, 6, 7, 0));
    assert_eq!(r, 42);
    assert_eq!(
        out,
        b"Mode 2: Operation: multiply, Value: 42\nOperation performed: multiplication\n",
        "C stdout was {:?}",
        show(&out)
    );
    let mut rng = Rng::with_seed(SEED ^ 18);
    for _ in 0..200 {
        let (a, b) = (rng.any_i32(), rng.any_i32());
        diff(&format!("complexmode(2, {a}, {b}, 0)"), |i| {
            complexmode(i, 2, a, b, 0)
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 19 — signed overflow must wrap identically
// ---------------------------------------------------------------------------
#[test]
fn err19_signed_overflow_wraps_identically() {
    // safe_add
    for (a, b) in [(i32::MAX, 1), (i32::MIN, -1), (i32::MAX, i32::MAX), (i32::MIN, i32::MIN)] {
        diff(&format!("safe_add({a},{b},0644) ovf"), |i| {
            safe_add(i, a, b, 0o644)
        });
    }
    // multiply_with_log
    for (a, b) in [(i32::MAX, 2), (i32::MIN, -1), (65536, 65536), (i32::MIN, i32::MIN)] {
        diff(&format!("multiply_with_log({a},{b}) ovf"), |i| {
            multiply_with_log(i, a, b)
        });
    }
    // copy_and_sum accumulator
    for fill in [i32::MAX, i32::MIN] {
        let mut v = vec![fill; 5];
        let p = v.as_mut_ptr();
        diff(&format!("copy_and_sum([{fill};5],5) ovf"), |i| {
            copy_and_sum(i, p, 5)
        });
    }
    // complexmode, every mode
    for mode in [1i32, 2, 3, 4] {
        for (a, b, c) in [
            (i32::MAX, i32::MAX, i32::MAX),
            (i32::MIN, i32::MIN, i32::MIN),
            (i32::MIN, -1, i32::MIN),
            (i32::MAX, 1, -1),
        ] {
            diff(&format!("complexmode({mode},{a},{b},{c}) ovf"), |i| {
                complexmode(i, mode, a, b, c)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 20 — check_permissions is total (no rejection path)
// ---------------------------------------------------------------------------
#[test]
fn err20_check_permissions_is_total() {
    // required == 0 is always satisfied, even for perms == 0.
    assert_eq!(check_permissions(Impl::C, 0, 0), 1);
    assert_eq!(check_permissions(Impl::Rust, 0, 0), 1);
    let mut rng = Rng::with_seed(SEED ^ 20);
    for _ in 0..1000 {
        let (p, r) = (rng.any_i32(), rng.any_i32());
        let (cv, _) = capture(|| check_permissions(Impl::C, p, r));
        assert!(cv == 0 || cv == 1, "C returned {cv}, not a boolean");
        diff(&format!("check_permissions({p},{r})"), |i| {
            check_permissions(i, p, r)
        });
    }
    // The function must print nothing, ever.
    let (_, out) = capture(|| check_permissions(Impl::C, 0o644, 0o600));
    assert!(out.is_empty());
    let (_, out) = capture(|| check_permissions(Impl::Rust, 0o644, 0o600));
    assert!(out.is_empty());
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks not tied to a single ERRORS row.
// ---------------------------------------------------------------------------
#[test]
fn err_generic_boundaries() {
    // Every documented sentinel, reached through the lowest-level entry points.
    // NULL pointer arguments:
    diff("copy_and_sum(NULL, 3)", |i| {
        copy_and_sum(i, std::ptr::null_mut(), 3)
    });
    diff("compare_operations(NULL, NULL)", |i| {
        compare_operations(i, std::ptr::null(), std::ptr::null())
    });

    // Zero and oversized lengths:
    let mut v = vec![1i32, 2, 3];
    let p = v.as_mut_ptr();
    // (Large positive counts would `memcpy` past the buffer and SIGSEGV in the
    // C too — see the note in `err07_copy_and_sum_alloc_failure`.)
    for count in [0i32, 3, 4, i32::MIN, i32::MIN + 1, -1] {
        diff(&format!("copy_and_sum(valid, {count}) bounds"), |i| {
            copy_and_sum(i, p, count)
        });
    }

    // `mode` treated as an out-of-range C enum value across the FFI boundary:
    // C enums accept any int, so these are all real inputs.
    for mode in [
        i32::MIN,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        0x7FFF_FFFF,
        0x0000_0100,
        0x1234_5678,
        -0x1234_5678,
    ] {
        diff(&format!("complexmode({mode}) enum-range"), |i| {
            complexmode(i, mode, -3, 11, 7)
        });
    }

    // Permission masks one step past each meaningful boundary.
    for perms in [0o577, 0o600, 0o601, 0o677, 0o1600, -0o601] {
        diff(&format!("safe_add(_,_,{perms:o}) bounds"), |i| {
            safe_add(i, 12, 34, perms)
        });
        diff(&format!("check_permissions({perms:o}, 0600)"), |i| {
            check_permissions(i, perms, 0o600)
        });
        diff(&format!("check_permissions({perms:o}, 0100)"), |i| {
            check_permissions(i, perms, 0o100)
        });
    }
}
