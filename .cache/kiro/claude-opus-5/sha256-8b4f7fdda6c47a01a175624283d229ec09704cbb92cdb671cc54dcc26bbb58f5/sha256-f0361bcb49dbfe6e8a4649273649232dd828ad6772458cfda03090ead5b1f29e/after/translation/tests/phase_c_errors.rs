//! Phase C — error / rejection-path differential tests, gated on `ERRORS.md`.
//! One test per row; row numbers refer to that table.
//!
//! Rows whose trigger kills the process (SIGFPE / SIGSEGV) are compared
//! out-of-process via `fork()`, asserting the *same* termination signal rather
//! than merely "both failed".

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn free(p: *mut c_void);
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// How a forked child ended: `Exited(code)` or `Signalled(signo)`.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(c_int),
    Signalled(c_int),
}

/// Runs `f` in a forked child and reports how the child terminated.
fn run_isolated(f: impl FnOnce()) -> Outcome {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    let w = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    let sig = status & 0x7f;
    if sig == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signalled(sig)
    }
}

const SIGFPE: c_int = 8;
const SIGSEGV: c_int = 11;

// ==========================================================================
// Rows 1-4 — is_valid_operation rejects.
// ==========================================================================
fn assert_rejected(ch: c_char, why: &str) {
    let cv = unsafe { (c().is_valid_operation)(ch) };
    let rv = unsafe { (r().is_valid_operation)(ch) };
    assert_eq!(cv, rv, "is_valid_operation({ch}) [{why}] C={cv} RUST={rv}");
    assert_eq!(cv, 0, "is_valid_operation({ch}) [{why}] must reject, C returned {cv}");
}

#[test]
fn err01_is_valid_operation_nul() {
    assert_rejected(0, "row 1: NUL short-circuits the &&");
}

#[test]
fn err02_is_valid_operation_below_range() {
    for ch in 1..49i32 {
        assert_rejected(ch as c_char, "row 2: nonzero but < '1'");
    }
}

#[test]
fn err03_is_valid_operation_above_range() {
    for ch in 54..=127i32 {
        assert_rejected(ch as c_char, "row 3: > '5'");
    }
}

#[test]
fn err04_is_valid_operation_negative_char() {
    for ch in -128..=-1i32 {
        assert_rejected(ch as c_char, "row 4: signed char < 0");
    }
    // And the accepting band must still be accepted identically.
    for ch in 49..=53i32 {
        let cv = unsafe { (c().is_valid_operation)(ch as c_char) };
        let rv = unsafe { (r().is_valid_operation)(ch as c_char) };
        assert_eq!(cv, rv, "is_valid_operation({ch})");
        assert_eq!(cv, 1, "is_valid_operation({ch}) must accept");
    }
}

// ==========================================================================
// Rows 5-6 — divide/modulo by zero return the sentinel 0.
// ==========================================================================
#[test]
fn err05_divide_by_zero_returns_zero() {
    let mut rng = Rng::new(0xE05);
    let cases: Vec<c_int> = BOUNDARIES
        .iter()
        .copied()
        .chain((0..2000).map(|_| rng.next_i32_mixed()))
        .collect();
    for a in cases {
        for &u in &[0, -1, c_int::MAX] {
            let cv = unsafe { (c().divide_operation)(a, 0, u) };
            let rv = unsafe { (r().divide_operation)(a, 0, u) };
            assert_eq!(cv, rv, "divide_operation({a}, 0, {u})");
            assert_eq!(cv, 0, "divide_operation({a}, 0) must return the 0 sentinel");
        }
    }
}

#[test]
fn err06_modulo_by_zero_returns_zero() {
    let mut rng = Rng::new(0xE06);
    let cases: Vec<c_int> = BOUNDARIES
        .iter()
        .copied()
        .chain((0..2000).map(|_| rng.next_i32_mixed()))
        .collect();
    for a in cases {
        for &u in &[0, -1, c_int::MAX] {
            let cv = unsafe { (c().modulo_operation)(a, 0, u) };
            let rv = unsafe { (r().modulo_operation)(a, 0, u) };
            assert_eq!(cv, rv, "modulo_operation({a}, 0, {u})");
            assert_eq!(cv, 0, "modulo_operation({a}, 0) must return the 0 sentinel");
        }
    }
}

// ==========================================================================
// Rows 7-8 — INT_MIN / -1 traps. Compared out-of-process.
// ==========================================================================
#[test]
fn err07_divide_int_min_by_minus_one_traps_identically() {
    let cf = c().divide_operation;
    let rf = r().divide_operation;
    let co = run_isolated(move || {
        let v = unsafe { cf(c_int::MIN, -1, 0) };
        std::hint::black_box(v);
    });
    let ro = run_isolated(move || {
        let v = unsafe { rf(c_int::MIN, -1, 0) };
        std::hint::black_box(v);
    });
    assert_eq!(co, ro, "divide_operation(INT_MIN, -1): C={co:?} RUST={ro:?}");
    assert_eq!(
        co,
        Outcome::Signalled(SIGFPE),
        "row 7: the C code is expected to raise SIGFPE here"
    );
}

#[test]
fn err08_modulo_int_min_by_minus_one_traps_identically() {
    let cf = c().modulo_operation;
    let rf = r().modulo_operation;
    let co = run_isolated(move || {
        let v = unsafe { cf(c_int::MIN, -1, 0) };
        std::hint::black_box(v);
    });
    let ro = run_isolated(move || {
        let v = unsafe { rf(c_int::MIN, -1, 0) };
        std::hint::black_box(v);
    });
    assert_eq!(co, ro, "modulo_operation(INT_MIN, -1): C={co:?} RUST={ro:?}");
    assert_eq!(
        co,
        Outcome::Signalled(SIGFPE),
        "row 8: the C code is expected to raise SIGFPE here"
    );
}

// ==========================================================================
// Row 9 — select_operation on out-of-range enum values (never NULL).
// ==========================================================================
#[test]
fn err09_select_operation_out_of_range_enum() {
    let mut probes: Vec<c_int> = vec![
        0,
        6,
        7,
        -1,
        -5,
        -6,
        255,
        256,
        1 << 16,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX,
        c_int::MAX - 1,
    ];
    let mut rng = Rng::new(0xE09);
    for _ in 0..3000 {
        let v = rng.next_i32_mixed();
        if !(1..=5).contains(&v) {
            probes.push(v);
        }
    }
    for op in probes {
        let cp = unsafe { (c().select_operation)(op) };
        let rp = unsafe { (r().select_operation)(op) };
        assert!(!cp.is_null(), "select_operation({op}) must not be NULL in C");
        assert!(!rp.is_null(), "select_operation({op}) must not be NULL in Rust");
        let cid = c().op_identity(cp);
        let rid = r().op_identity(rp);
        assert_eq!(cid, rid, "select_operation({op}) identity C={cid} RUST={rid}");
        assert_eq!(cid, "add_operation", "select_operation({op}) default arm");
    }
}

// ==========================================================================
// Row 10 — get_operation_priority does not validate its enum.
// ==========================================================================
#[test]
fn err10_get_operation_priority_no_validation() {
    let mut probes: Vec<c_int> = vec![0, 6, -1, -6, c_int::MIN, c_int::MAX, 214748365, -214748365];
    let mut rng = Rng::new(0xE10);
    for _ in 0..3000 {
        probes.push(rng.next_i32_mixed());
    }
    for op in probes {
        let cv = unsafe { (c().get_operation_priority)(op) };
        let rv = unsafe { (r().get_operation_priority)(op) };
        assert_eq!(cv, rv, "get_operation_priority({op}) C={cv} RUST={rv}");
    }
}

// ==========================================================================
// Rows 11-13 — allocate_results failure / degenerate counts.
// ==========================================================================
fn diff_alloc_nullness(count: c_int, row: &str) -> bool {
    let cp = unsafe { (c().allocate_results)(count) };
    let rp = unsafe { (r().allocate_results)(count) };
    let cn = cp.is_null();
    let rn = rp.is_null();
    assert_eq!(
        cn, rn,
        "{row}: allocate_results({count}) NULL-ness C={cn} RUST={rn}"
    );
    unsafe {
        free(cp as *mut c_void);
        free(rp as *mut c_void);
    }
    cn
}

#[test]
fn err11_allocate_results_negative_count_returns_null() {
    for count in [-1, -2, -10, -1000, c_int::MIN, c_int::MIN + 1, -89478485] {
        let was_null = diff_alloc_nullness(count, "row 11");
        assert!(
            was_null,
            "row 11: allocate_results({count}) sign-extends to a huge size_t; calloc must fail"
        );
    }
}

#[test]
fn err12_allocate_results_zero_count() {
    // glibc's calloc(0, 24) yields a unique non-NULL pointer; the point of the
    // row is that both sides agree, whatever the allocator does.
    let was_null = diff_alloc_nullness(0, "row 12");
    assert!(!was_null, "row 12: glibc calloc(0, 24) returns non-NULL");
}

#[test]
fn err13_allocate_results_huge_count() {
    // INT_MAX * 24 bytes. Both sides call the same calloc with the same
    // arguments, so the only invariant that must hold is that they agree.
    for count in [c_int::MAX, c_int::MAX - 1, 1 << 30, 100_000_000] {
        diff_alloc_nullness(count, "row 13");
    }
}

// ==========================================================================
// Row 14 — NULL history discards the caller's count.
// ==========================================================================
#[test]
fn err14_null_history_discards_caller_count() {
    for start in [1, 5, 9, 10, 11, 12, 1000, c_int::MAX, -1, c_int::MIN] {
        let mut ch: *mut ComputationResult = std::ptr::null_mut();
        let mut rh: *mut ComputationResult = std::ptr::null_mut();
        let mut cc = start;
        let mut rc = start;
        let cv = unsafe { (c().perform_computation_with_history)(11, 22, 1, &mut ch, &mut cc) };
        let rv = unsafe { (r().perform_computation_with_history)(11, 22, 1, &mut rh, &mut rc) };
        assert_eq!(cv, rv, "row 14: ret with start={start}");
        assert_eq!(cc, rc, "row 14: count with start={start} C={cc} RUST={rc}");
        assert_eq!(cc, 1, "row 14: count is reset to 0 then incremented");
        assert_eq!(ch.is_null(), rh.is_null());
        assert_eq!(
            unsafe { raw_bytes(ch, 10) },
            unsafe { raw_bytes(rh, 10) },
            "row 14: buffer bytes with start={start}"
        );
        unsafe {
            free(ch as *mut c_void);
            free(rh as *mut c_void);
        }
    }
}

// ==========================================================================
// Rows 15-16 — the record is silently dropped at/over capacity.
// ==========================================================================
fn diff_pcwh_full(start: c_int, op: c_int, a: c_int, b: c_int, row: &str) {
    let cp = unsafe { (c().allocate_results)(10) };
    let rp = unsafe { (r().allocate_results)(10) };
    assert!(!cp.is_null() && !rp.is_null());
    let before = unsafe { raw_bytes(cp, 10) };

    let mut ch = cp;
    let mut rh = rp;
    let mut cc = start;
    let mut rc = start;
    let cv = unsafe { (c().perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
    let rv = unsafe { (r().perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

    assert_eq!(cv, rv, "{row}: ret start={start} op={op} a={a} b={b}");
    assert_eq!(cc, rc, "{row}: count start={start} C={cc} RUST={rc}");
    assert_eq!(cc, start, "{row}: count must be left unchanged at/over capacity");
    let cafter = unsafe { raw_bytes(cp, 10) };
    let rafter = unsafe { raw_bytes(rp, 10) };
    assert_eq!(cafter, rafter, "{row}: buffer bytes");
    assert_eq!(cafter, before, "{row}: buffer must be untouched (record dropped)");
    unsafe {
        free(cp as *mut c_void);
        free(rp as *mut c_void);
    }
}

#[test]
fn err15_pcwh_at_capacity_drops_record() {
    let mut rng = Rng::new(0xE15);
    for op in [1, 2, 3, 4, 5, 0, 6, -1, c_int::MIN, c_int::MAX] {
        for _ in 0..30 {
            let a = rng.next_i32_mixed();
            let mut b = rng.next_i32_mixed();
            if a == c_int::MIN && b == -1 {
                b = 3;
            }
            diff_pcwh_full(10, op, a, b, "row 15");
        }
    }
}

#[test]
fn err16_pcwh_over_capacity_drops_record() {
    let mut rng = Rng::new(0xE16);
    for start in [11, 12, 20, 1000, c_int::MAX - 1, c_int::MAX] {
        for op in [1, 4, 5, 0, -3] {
            for _ in 0..10 {
                let a = rng.next_i32_mixed();
                let mut b = rng.next_i32_mixed();
                if a == c_int::MIN && b == -1 {
                    b = 3;
                }
                diff_pcwh_full(start, op, a, b, "row 16");
            }
        }
    }
}

// ==========================================================================
// Row 17 — negative count passes `< 10`, so the C code writes at a negative
// offset. Given a padded buffer the write lands in owned memory, so the exact
// out-of-bounds behaviour can be compared safely.
// ==========================================================================
#[test]
fn err17_pcwh_negative_count_writes_before_buffer() {
    const PAD: usize = 16;
    const TOTAL: usize = PAD + 10;
    let mut rng = Rng::new(0xE17);

    for start in -(PAD as c_int)..0 {
        for op in [1, 2, 3, 4, 5, 0, -7] {
            let a = rng.next_i32_mixed();
            let mut b = rng.next_i32_mixed();
            if a == c_int::MIN && b == -1 {
                b = 3;
            }

            let mut cbuf = vec![
                ComputationResult { value: 0, timestamp: 0, status: 0 };
                TOTAL
            ];
            let mut rbuf = cbuf.clone();

            // history points PAD elements in, so offsets -PAD..-1 stay inside.
            let mut ch = unsafe { cbuf.as_mut_ptr().add(PAD) };
            let mut rh = unsafe { rbuf.as_mut_ptr().add(PAD) };
            let mut cc = start;
            let mut rc = start;

            let cv = unsafe { (c().perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
            let rv = unsafe { (r().perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

            assert_eq!(cv, rv, "row 17: ret start={start} op={op} a={a} b={b}");
            assert_eq!(cc, rc, "row 17: count start={start} C={cc} RUST={rc}");
            assert_eq!(cc, start + 1, "row 17: negative count still increments");
            let cb = unsafe { raw_bytes(cbuf.as_ptr(), TOTAL) };
            let rb = unsafe { raw_bytes(rbuf.as_ptr(), TOTAL) };
            assert_eq!(
                cb, rb,
                "row 17: the out-of-bounds write must land at the same offset \
                 (start={start} op={op})"
            );
            // The write really did land before the nominal buffer start.
            assert_ne!(
                &cb[..PAD * SIZEOF_COMPUTATION_RESULT],
                &vec![0u8; PAD * SIZEOF_COMPUTATION_RESULT][..],
                "row 17: expected a write in the padding region"
            );
        }
    }
}

// ==========================================================================
// Row 18 — out-of-range enum across the FFI boundary into pcwh.
// ==========================================================================
#[test]
fn err18_pcwh_out_of_range_enum() {
    let mut rng = Rng::new(0xE18);
    let mut ops: Vec<c_int> = vec![0, 6, 7, -1, -5, 1000, c_int::MIN, c_int::MAX];
    for _ in 0..500 {
        let v = rng.next_i32_mixed();
        if !(1..=5).contains(&v) {
            ops.push(v);
        }
    }
    for op in ops {
        let a = rng.next_i32_mixed();
        let b = rng.next_i32_mixed();

        let cp = unsafe { (c().allocate_results)(10) };
        let rp = unsafe { (r().allocate_results)(10) };
        let mut ch = cp;
        let mut rh = rp;
        let mut cc: c_int = 0;
        let mut rc: c_int = 0;
        let cv = unsafe { (c().perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
        let rv = unsafe { (r().perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };
        assert_eq!(cv, rv, "row 18: ret op={op} a={a} b={b}");
        assert_eq!(
            cv,
            a.wrapping_add(b),
            "row 18: default arm must be add_operation (op={op})"
        );
        assert_eq!(cc, rc);
        assert_eq!(unsafe { raw_bytes(cp, 10) }, unsafe { raw_bytes(rp, 10) });
        unsafe {
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }
    }
}

// ==========================================================================
// Row 19 — NULL pointer arguments. Compared out-of-process.
// ==========================================================================
#[test]
fn err19_pcwh_null_pointer_args_fault_identically() {
    type Pcwh = unsafe extern "C" fn(
        c_int,
        c_int,
        c_int,
        *mut *mut ComputationResult,
        *mut c_int,
    ) -> c_int;

    // (label, history is NULL, history_count is NULL)
    let cases: [(&str, bool, bool); 3] = [
        ("history=NULL", true, false),
        ("history_count=NULL", false, true),
        ("both NULL", true, true),
    ];

    for (label, hnull, cnull) in cases {
        let run = |f: Pcwh| -> Outcome {
            run_isolated(move || {
                let mut storage: *mut ComputationResult = std::ptr::null_mut();
                let mut count: c_int = 0;
                let hp = if hnull { std::ptr::null_mut() } else { &mut storage as *mut _ };
                let cp = if cnull { std::ptr::null_mut() } else { &mut count as *mut _ };
                let v = unsafe { f(1, 2, 1, hp, cp) };
                std::hint::black_box(v);
            })
        };
        let co = run(c().perform_computation_with_history);
        let ro = run(r().perform_computation_with_history);
        assert_eq!(co, ro, "row 19 [{label}]: C={co:?} RUST={ro:?}");
        assert_eq!(
            co,
            Outcome::Signalled(SIGSEGV),
            "row 19 [{label}]: expected SIGSEGV from the unchecked dereference"
        );
    }
}

// ==========================================================================
// Row 20 — divide/modulo by zero still appends a record.
// ==========================================================================
#[test]
fn err20_pcwh_zero_divisor_still_records() {
    for op in [4, 5] {
        for &a in BOUNDARIES.iter() {
            for start in [0, 4, 9] {
                let cp = unsafe { (c().allocate_results)(10) };
                let rp = unsafe { (r().allocate_results)(10) };
                let mut ch = cp;
                let mut rh = rp;
                let mut cc = start;
                let mut rc = start;
                let cv = unsafe { (c().perform_computation_with_history)(a, 0, op, &mut ch, &mut cc) };
                let rv = unsafe { (r().perform_computation_with_history)(a, 0, op, &mut rh, &mut rc) };
                assert_eq!(cv, rv, "row 20: ret op={op} a={a} start={start}");
                assert_eq!(cv, 0, "row 20: zero-divisor sentinel");
                assert_eq!(cc, rc);
                assert_eq!(cc, start + 1, "row 20: the 0 record is still appended");
                let cb = unsafe { raw_bytes(cp, 10) };
                let rb = unsafe { raw_bytes(rp, 10) };
                assert_eq!(cb, rb, "row 20: buffer bytes op={op} a={a} start={start}");
                // value == 0 and status == STATUS_SUCCESS in the written slot.
                let slot = unsafe { *cp.offset(start as isize) };
                assert_eq!(slot.value, 0, "row 20: recorded value");
                assert_eq!(slot.status, 0, "row 20: STATUS_SUCCESS");
                unsafe {
                    free(cp as *mut c_void);
                    free(rp as *mut c_void);
                }
            }
        }
    }
}

// ==========================================================================
// Rows 21-25 (mathop rejection paths) live in tests/phase_c_mathop.rs: they
// capture fd 1, which must not race with libtest's own progress output.
// ==========================================================================

// Row 26 — get_computation_timestamp has no failure path.
#[test]
fn err26_get_computation_timestamp_no_failure_path() {
    for _ in 0..50 {
        let cv = unsafe { (c().get_computation_timestamp)() };
        let rv = unsafe { (r().get_computation_timestamp)() };
        assert_eq!(cv, rv, "row 26: get_computation_timestamp");
        assert_ne!(cv, -1, "row 26: time() itself did not fail");
    }
}

// ==========================================================================
// Generic FFI boundary sweep required by Phase C beyond the table: every
// exported entry point fed out-of-range enum values and boundary integers.
// ==========================================================================
#[test]
fn generic_out_of_range_enum_sweep() {
    let mut probes: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -100000,
        -6,
        -5,
        -1,
        0,
        6,
        7,
        8,
        100,
        65535,
        65536,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    let mut rng = Rng::new(0xBEEF);
    for _ in 0..2000 {
        probes.push(rng.next_i32_mixed());
    }

    for op in probes {
        // get_operation_priority
        assert_eq!(
            unsafe { (c().get_operation_priority)(op) },
            unsafe { (r().get_operation_priority)(op) },
            "get_operation_priority({op})"
        );
        // select_operation identity
        assert_eq!(
            c().op_identity(unsafe { (c().select_operation)(op) }),
            r().op_identity(unsafe { (r().select_operation)(op) }),
            "select_operation({op})"
        );
        // is_valid_operation over the low byte
        let ch = (op & 0xff) as u8 as i8 as c_char;
        assert_eq!(
            unsafe { (c().is_valid_operation)(ch) },
            unsafe { (r().is_valid_operation)(ch) },
            "is_valid_operation({ch})"
        );
        // perform_computation_with_history with that enum
        let cp = unsafe { (c().allocate_results)(10) };
        let rp = unsafe { (r().allocate_results)(10) };
        let mut ch2 = cp;
        let mut rh2 = rp;
        let mut cc: c_int = 3;
        let mut rc: c_int = 3;
        let a = rng.next_i32_mixed();
        let mut b = rng.next_i32_mixed();
        if a == c_int::MIN && b == -1 {
            b = 3;
        }
        assert_eq!(
            unsafe { (c().perform_computation_with_history)(a, b, op, &mut ch2, &mut cc) },
            unsafe { (r().perform_computation_with_history)(a, b, op, &mut rh2, &mut rc) },
            "pcwh(a={a}, b={b}, op={op})"
        );
        assert_eq!(cc, rc);
        assert_eq!(unsafe { raw_bytes(cp, 10) }, unsafe { raw_bytes(rp, 10) });
        unsafe {
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }
    }
}
