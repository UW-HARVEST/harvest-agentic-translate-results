// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md, plus the generic FFI boundaries (NULL
// pointers, zero / oversized lengths, out-of-range `mode` values one step past
// the documented range and far outside it).
//
// Rows that require `malloc()` to fail are driven through the `fault_child`
// helper, which is spawned with an LD_PRELOAD interposer that makes one exact
// allocation size fail (see tests/fixtures/fail_malloc.c).

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

// ===========================================================================
// ERRORS.md row 3 — safe_add permission denial
// ===========================================================================

#[test]
fn row03_safe_add_insufficient_permissions() {
    const READ: c_int = 0o400;
    const WRITE: c_int = 0o200;

    let mut denied: Vec<c_int> = vec![
        0, 0o100, 0o200, 0o400, 0o300, 0o500, 0o077, 0o111, 0o222, 0o444, 0o177, 0o477, 0o277,
        i32::MAX & !0o600,
        i32::MIN,
    ];
    let mut rng = Rng::new(0x9001);
    while denied.len() < 200 {
        let p = rng.i32();
        if (p & (READ | WRITE)) != (READ | WRITE) {
            denied.push(p);
        }
    }

    for &perms in &denied {
        assert_ne!(
            perms & (READ | WRITE),
            READ | WRITE,
            "test bug: {perms:o} actually grants rw"
        );
        let a = rng.i32();
        let b = rng.i32();
        let (c, r) = both();
        let (cv, cout) = capture(|| unsafe { (c.safe_add)(a, b, perms) });
        let (rv, rout) = capture(|| unsafe { (r.safe_add)(a, b, perms) });
        assert_eq!(cv, 0, "C must return 0 on denial (perms={perms:o})");
        assert_eq!(
            cout, b"Insufficient permissions for addition\n",
            "C denial message changed"
        );
        assert_eq!(rv, cv, "safe_add({a},{b},{perms:o}) return differs");
        assert_eq!(
            cout,
            rout,
            "safe_add({a},{b},{perms:o}) stdout differs: {} vs {}",
            show(&cout),
            show(&rout)
        );
    }
}

// ===========================================================================
// ERRORS.md row 4 — copy_and_sum(NULL, count)
// ===========================================================================

#[test]
fn row04_copy_and_sum_null_source() {
    for &count in &[0, 1, 3, -1, 7, i32::MAX, i32::MIN, 1024] {
        let (c, r) = both();
        let (cv, cout) = capture(|| unsafe { (c.copy_and_sum)(std::ptr::null_mut(), count) });
        let (rv, rout) = capture(|| unsafe { (r.copy_and_sum)(std::ptr::null_mut(), count) });
        assert_eq!(cv, -1, "C must return -1 for NULL src (count={count})");
        assert_eq!(cout, b"Source pointer is NULL\n", "C message changed");
        assert_eq!(rv, cv, "copy_and_sum(NULL,{count}) return differs");
        assert_eq!(
            cout,
            rout,
            "copy_and_sum(NULL,{count}) stdout differs: {} vs {}",
            show(&cout),
            show(&rout)
        );
    }
}

// ===========================================================================
// ERRORS.md row 5 — copy_and_sum allocation failure (oversized length)
// ===========================================================================

#[test]
fn row05_copy_and_sum_allocation_failure() {
    // Any negative count sign-extends to a size_t near 2^64 => malloc fails.
    let mut counts: Vec<c_int> = vec![-1, -2, -3, -100, -65536, i32::MIN, i32::MIN + 1, -(1 << 30)];
    let mut rng = Rng::new(0x9002);
    for _ in 0..60 {
        let v = rng.i32();
        counts.push(if v >= 0 { -(v / 2) - 1 } else { v });
    }

    for &count in &counts {
        assert!(count < 0);
        let mut buf: Vec<c_int> = vec![1, 2, 3, 4];
        let (c, r) = both();
        let (cv, cout) = capture(|| unsafe { (c.copy_and_sum)(buf.as_mut_ptr(), count) });
        let (rv, rout) = capture(|| unsafe { (r.copy_and_sum)(buf.as_mut_ptr(), count) });
        assert_eq!(cv, -1, "C must return -1 for count={count}");
        assert_eq!(cout, b"Memory allocation failed\n", "C message changed");
        assert_eq!(rv, cv, "copy_and_sum(buf,{count}) return differs");
        assert_eq!(
            cout,
            rout,
            "copy_and_sum(buf,{count}) stdout differs: {} vs {}",
            show(&cout),
            show(&rout)
        );
    }
}

/// Same row, but with the allocation of a *valid* size failing, proving both
/// implementations request the same number of bytes (`count * sizeof(int)`).
#[test]
fn row05b_copy_and_sum_allocation_failure_injected() {
    let report = child_report("cas", 12); // count 3 * sizeof(int)
    assert_c_section_contains(&report, &["Memory allocation failed", "RET=-1"]);
    assert_sections_match(&report);
}

/// Same row reached *through* `complexmode`'s mode-3 arm, where `copy_and_sum`
/// is a candidate for inlining.  This is a real regression test: at
/// `opt-level > 0` LLVM removed the (non-escaping) `malloc`/`free` pair here, so
/// the `NULL` branch became unreachable and the release build returned the sum
/// (21) while the C returns -1 and prints `Memory allocation failed`.
#[test]
fn row05c_copy_and_sum_allocation_failure_through_complexmode() {
    let report = child_report("cm3", 12); // 3 * sizeof(int)
    assert_c_section_contains(
        &report,
        &["Memory allocation failed", "Result: -1", "RET=-1"],
    );
    assert_sections_match(&report);

    // the other modes' allocations must survive optimisation as well
    for (scenario, size) in [("cm2", 64u64), ("cm1", 40), ("cm4", 40), ("cm3", 40)] {
        let report = child_report(scenario, size);
        assert_sections_match(&report);
    }
}

// ===========================================================================
// ERRORS.md rows 6, 7, 8 — compare_operations NULL arguments
// ===========================================================================

fn cmp_null_case(name: &str, a: Option<&[u8]>, b: Option<&[u8]>) {
    let pa = a.map_or(std::ptr::null(), |s| s.as_ptr() as *const c_char);
    let pb = b.map_or(std::ptr::null(), |s| s.as_ptr() as *const c_char);
    let (c, r) = both();
    let (cv, cout) = capture(|| unsafe { (c.compare_operations)(pa, pb) });
    let (rv, rout) = capture(|| unsafe { (r.compare_operations)(pa, pb) });
    assert_eq!(cv, -1, "[{name}] C must return -1");
    assert_eq!(
        cout, b"One or both operation strings are NULL\n",
        "[{name}] C message changed"
    );
    assert_eq!(rv, cv, "[{name}] return differs");
    assert_eq!(
        cout,
        rout,
        "[{name}] stdout differs: {} vs {}",
        show(&cout),
        show(&rout)
    );
}

#[test]
fn row06_compare_operations_null_first() {
    cmp_null_case("op1=NULL", None, Some(b"none\0"));
    cmp_null_case("op1=NULL,empty op2", None, Some(b"\0"));
}

#[test]
fn row07_compare_operations_null_second() {
    cmp_null_case("op2=NULL", Some(b"none\0"), None);
    cmp_null_case("empty op1,op2=NULL", Some(b"\0"), None);
}

#[test]
fn row08_compare_operations_both_null() {
    cmp_null_case("both NULL", None, None);
}

// ===========================================================================
// ERRORS.md row 10 — complexmode invalid mode (default: arm)
// ===========================================================================

#[test]
fn row10_complexmode_invalid_mode() {
    // Every "enum-like" int that has no matching case, including one step
    // outside the valid 1..=4 window and the extremes of the C int range.
    let mut modes: Vec<c_int> = vec![
        0, 5, 6, -1, -2, 100, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 0x1_0000, -0x1_0000,
    ];
    let mut rng = Rng::new(0x9003);
    while modes.len() < 250 {
        let m = rng.i32();
        if !(1..=4).contains(&m) {
            modes.push(m);
        }
    }

    for &mode in &modes {
        let v1 = rng.i32();
        let v2 = rng.i32();
        let v3 = rng.i32();
        let (c, r) = both();
        let (cv, cout) = capture(|| unsafe { (c.complexmode)(mode, v1, v2, v3) });
        let (rv, rout) = capture(|| unsafe { (r.complexmode)(mode, v1, v2, v3) });
        assert_eq!(cv, -1, "C must return -1 for mode={mode}");
        // "Invalid mode" only; the trailing "Operation performed:" line must be
        // suppressed because `operation` stayed "none".
        assert_eq!(cout, b"Invalid mode\n", "C output changed for mode={mode}");
        assert_eq!(rv, cv, "complexmode({mode},..) return differs");
        assert_eq!(
            cout,
            rout,
            "complexmode({mode},..) stdout differs: {} vs {}",
            show(&cout),
            show(&rout)
        );
    }
}

// ===========================================================================
// ERRORS.md row 12 — copy_and_sum(count == 0) boundary
// ===========================================================================

#[test]
fn row12_copy_and_sum_zero_count() {
    let mut buf: Vec<c_int> = vec![7, 8, 9];
    let (c, r) = both();
    let (cv, cout) = capture(|| unsafe { (c.copy_and_sum)(buf.as_mut_ptr(), 0) });
    let (rv, rout) = capture(|| unsafe { (r.copy_and_sum)(buf.as_mut_ptr(), 0) });
    assert_eq!(cv, 0, "glibc malloc(0) succeeds, so the C returns 0");
    assert!(cout.is_empty(), "C printed unexpectedly: {}", show(&cout));
    assert_eq!(rv, cv);
    assert_eq!(cout, rout);
}

// ===========================================================================
// Generic FFI boundaries not tied to a single ERRORS.md row
// ===========================================================================

/// `create_result_string(NULL, val)` is *not* rejected by the C — it forwards
/// the NULL to `snprintf("%s")`.  Assert both agree on that too (this is the
/// "one step past the obvious guard" case).
#[test]
fn boundary_create_result_string_null_op_is_not_rejected() {
    let (c, r) = both();
    for &val in &[0, -1, i32::MIN, i32::MAX, 5] {
        let (cs, cout) = capture(|| unsafe {
            let p = (c.create_result_string)(std::ptr::null(), val);
            assert!(!p.is_null(), "C returned NULL unexpectedly");
            let s = cstr_bytes(p);
            libc_free(p);
            s
        });
        let (rs, rout) = capture(|| unsafe {
            let p = (r.create_result_string)(std::ptr::null(), val);
            assert!(!p.is_null(), "Rust returned NULL unexpectedly");
            let s = cstr_bytes(p);
            libc_free(p);
            s
        });
        assert_eq!(cs, rs, "create_result_string(NULL,{val}) text differs");
        assert_eq!(cout, rout);
        assert!(
            cs.windows(6).any(|w| w == b"(null)"),
            "expected glibc's (null) rendering, got {}",
            show(&cs)
        );
    }
}

/// The `mode` parameter is an `int`: a C caller can pass any bit pattern,
/// including values that no `case` label covers.  Sweep the whole switch
/// neighbourhood and assert identical classification (valid arm vs `default:`).
#[test]
fn boundary_mode_enum_sweep() {
    for mode in -8..=12 {
        let (c, r) = both();
        let (cv, cout) = capture(|| unsafe { (c.complexmode)(mode, 11, 13, 17) });
        let (rv, rout) = capture(|| unsafe { (r.complexmode)(mode, 11, 13, 17) });
        assert_eq!(rv, cv, "mode={mode} return differs");
        assert_eq!(cout, rout, "mode={mode} stdout differs");
        if !(1..=4).contains(&mode) {
            assert_eq!(cv, -1);
            assert_eq!(cout, b"Invalid mode\n");
        }
    }
}

/// Zero and one-element lengths at the copy_and_sum boundary, and a length one
/// past the buffer's real size is *not* tested (that is out-of-bounds UB in the
/// C; see ERRORS.md notes).  Here: 0 and 1 with the same buffer.
#[test]
fn boundary_copy_and_sum_zero_and_one() {
    let mut rng = Rng::new(0x9004);
    for _ in 0..100 {
        let v = vec![rng.i32(), rng.i32()];
        for &count in &[0, 1, 2] {
            let (c, r) = both();
            let mut b1 = v.clone();
            let mut b2 = v.clone();
            let (cv, cout) = capture(|| unsafe { (c.copy_and_sum)(b1.as_mut_ptr(), count) });
            let (rv, rout) = capture(|| unsafe { (r.copy_and_sum)(b2.as_mut_ptr(), count) });
            assert_eq!(rv, cv, "copy_and_sum({v:?},{count}) return differs");
            assert_eq!(cout, rout);
        }
    }
}

// ===========================================================================
// malloc-failure rows (1, 2, 9, 11) — driven through the LD_PRELOAD helper
// ===========================================================================

#[test]
fn row01_create_result_string_malloc_failure() {
    // 64 == `malloc(64 * sizeof(char))` in create_result_string
    let report = child_report("crs", 64);
    assert_c_section_contains(&report, &["RET_PTR=<NULL>"]);
    assert_sections_match(&report);
}

#[test]
fn row02_multiply_with_log_inner_malloc_failure() {
    let report = child_report("mwl", 64);
    // returns 0 (not 42) and leaves *log_msg NULL
    assert_c_section_contains(&report, &["RET=0 LOG=<NULL>"]);
    assert_sections_match(&report);
}

#[test]
fn row09_complexmode_tracker_malloc_failure() {
    // 40 == sizeof(Result) { int; char[32]; int; }
    for scenario in ["cm1", "cm2", "cm3", "cm4", "cm9"] {
        let report = child_report(scenario, 40);
        assert_c_section_contains(
            &report,
            &["Failed to allocate result tracker", "RET=-1"],
        );
        assert_sections_match(&report);
    }
}

#[test]
fn row11_complexmode_mode2_log_creation_failure() {
    // tracker malloc(40) succeeds, log-string malloc(64) fails
    let report = child_report("cm2", 64);
    assert_c_section_contains(
        &report,
        &[
            "Log message creation failed",
            "Operation performed: multiplication",
            "RET=0",
        ],
    );
    assert!(
        !c_section(&report).contains("Mode 2: Operation"),
        "the log-print branch must not run:\n{report}"
    );
    assert_sections_match(&report);
}

/// Sanity check on the harness itself: with the interposer *disarmed* the same
/// scenarios must take the happy path, so a passing malloc-failure test cannot
/// be an artefact of a broken helper.
#[test]
fn row09_11_control_run_without_injection() {
    let report = child_report("cm2", 0);
    assert_c_section_contains(
        &report,
        &["Mode 2: Operation: multiply, Value: 42", "RET=42"],
    );
    assert_sections_match(&report);

    let report = child_report("crs", 0);
    assert_c_section_contains(&report, &["RET_PTR=Operation: multiply, Value: 42"]);
    assert_sections_match(&report);
}

// The helper-process plumbing (`child_report`, `assert_sections_match`, …)
// lives in tests/common/mod.rs so the heap-poison suite can reuse it.
