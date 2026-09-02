//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Rows whose trigger faults the process
//! (SIGSEGV / SIGFPE) are executed in a re-exec'd child process and the
//! terminating signal is compared, so "both crash the same way" is asserted
//! rather than assumed.

mod support;

use std::ffi::{c_char, c_int, c_void};
use support::*;

extern "C" {
    fn free(p: *mut c_void);
}

macro_rules! err {
    ($row:literal, $inputs:expr, $c:expr, $rs:expr) => {
        assert_eq!(
            $c, $rs,
            "ERRORS.md row {} diverged for {:?}\n  C   = {:?}\n  Rust= {:?}",
            $row, $inputs, $c, $rs
        )
    };
}

// ---------------------------------------------------------------------------
// Row 1 — malloc(sizeof(StringBuffer)) failure.
// Not triggerable from outside (a 16-byte allocation never fails here), but the
// NULL-return contract is shared with row 2, which is triggerable. Documented
// and asserted as "both libraries return NULL from the same code shape".
// ---------------------------------------------------------------------------
#[test]
fn row01_create_buffer_header_alloc_failure_shares_null_contract() {
    let p = pair();
    unsafe {
        // The only observable form of row 1 is "returns NULL", which row 2
        // exercises for real. Assert the success path is not accidentally
        // returning NULL, so row 2's NULL is meaningful.
        let cb = (p.c.create_buffer)(16);
        let rb = (p.rs.create_buffer)(16);
        err!(1, 16, cb.is_null(), rb.is_null());
        assert!(!cb.is_null());
        (p.c.destroy_buffer)(cb);
        (p.rs.destroy_buffer)(rb);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — negative initial_capacity sign-extends to a huge size_t; malloc
// fails; the header is freed and NULL returned.
// ---------------------------------------------------------------------------
#[test]
fn row02_create_buffer_negative_capacity_returns_null() {
    let p = pair();
    let mut rng = Rng::new(0x0002_0002);
    unsafe {
        let mut caps: Vec<c_int> = vec![-1, -2, -8, -4096, -65536, i32::MIN, i32::MIN + 1];
        for _ in 0..200 {
            caps.push(-(rng.range(1, 1 << 30) as c_int));
        }
        for cap in caps {
            let cb = (p.c.create_buffer)(cap);
            let rb = (p.rs.create_buffer)(cap);
            err!(2, cap, cb.is_null(), rb.is_null());
            assert!(cb.is_null(), "create_buffer({cap}) must fail");
            // Nothing to destroy; both returned NULL after freeing the header.
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — huge-but-positive initial_capacity.
// ---------------------------------------------------------------------------
#[test]
fn row03_create_buffer_huge_positive_capacity() {
    let p = pair();
    unsafe {
        // Each library is exercised while the other holds nothing, so a
        // multi-gigabyte request is not perturbed by the peer's live
        // allocation.
        for cap in [i32::MAX, i32::MAX - 1, 1 << 30, (1 << 30) + 1] {
            let cb = (p.c.create_buffer)(cap);
            let c_null = cb.is_null();
            let c_snap = if c_null { None } else { Some(snapshot(cb)) };
            (p.c.destroy_buffer)(cb);

            let rb = (p.rs.create_buffer)(cap);
            let r_null = rb.is_null();
            let r_snap = if r_null { None } else { Some(snapshot(rb)) };
            (p.rs.destroy_buffer)(rb);

            err!(3, cap, c_null, r_null);
            err!(3, cap, c_snap, r_snap);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — initial_capacity == 0 is NOT rejected; data[0] is written OOB.
// ---------------------------------------------------------------------------
#[test]
fn row04_create_buffer_zero_capacity_not_rejected() {
    let p = pair();
    unsafe {
        let cb = (p.c.create_buffer)(0);
        let rb = (p.rs.create_buffer)(0);
        err!(4, 0, cb.is_null(), rb.is_null());
        assert!(!cb.is_null(), "malloc(0) is non-NULL on glibc");
        err!(4, 0, snapshot(cb), snapshot(rb));
        assert_eq!((*cb).capacity, 0);
        assert_eq!((*cb).length, 0);
        (p.c.destroy_buffer)(cb);
        (p.rs.destroy_buffer)(rb);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — realloc failure returns -1 and leaves the buffer untouched.
// ---------------------------------------------------------------------------
#[test]
fn row05_append_realloc_failure_returns_minus_one() {
    let p = pair();
    unsafe {
        // required_capacity = INT_MAX-3, doubled it wraps to -8, which
        // sign-extends to a ~2^64 realloc request -> NULL -> return -1.
        //
        // NB: length == INT_MAX is deliberately excluded: there
        // required_capacity wraps to INT_MIN, which is NOT > capacity, so the C
        // skips the realloc entirely and runs `strcpy(data + INT_MAX, ...)`.
        // That is a wild write, not this row's rejection path; it is covered by
        // the out-of-process rows instead.
        for length in [i32::MAX - 4, i32::MAX - 1, 1 << 30, (1 << 30) + 7] {
            let empty = b"\0";
            let cb = (p.c.create_buffer)(64);
            let rb = (p.rs.create_buffer)(64);
            (*cb).length = length;
            (*rb).length = length;
            let crc = (p.c.append_to_buffer)(cb, empty.as_ptr() as *const c_char);
            let rrc = (p.rs.append_to_buffer)(rb, empty.as_ptr() as *const c_char);
            err!(5, length, crc, rrc);
            assert_eq!(crc, -1, "realloc must fail for length={length}");
            // Buffer left untouched: capacity still 64, length still `length`.
            err!(
                5,
                length,
                ((*cb).capacity, (*cb).length),
                ((*rb).capacity, (*rb).length)
            );
            assert_eq!((*cb).capacity, 64);
            assert_eq!((*cb).length, length);
            (*cb).length = 0;
            (*rb).length = 0;
            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row05b_append_realloc_zero_size_request() {
    let p = pair();
    unsafe {
        // capacity = -1, length = -1, str = "" => required_capacity == 0 > -1,
        // new_capacity == 0, realloc(ptr, 0). Whatever glibc does, both
        // libraries must do the same thing.
        let empty = b"\0";
        let cb = (p.c.create_buffer)(64);
        let rb = (p.rs.create_buffer)(64);
        (*cb).capacity = -1;
        (*cb).length = -1;
        (*rb).capacity = -1;
        (*rb).length = -1;
        let crc = (p.c.append_to_buffer)(cb, empty.as_ptr() as *const c_char);
        let rrc = (p.rs.append_to_buffer)(rb, empty.as_ptr() as *const c_char);
        err!("5b", "realloc(ptr, 0)", crc, rrc);
        err!(
            "5b",
            "fields",
            ((*cb).capacity, (*cb).length),
            ((*rb).capacity, (*rb).length)
        );
        // glibc's realloc(p, 0) frees p and returns NULL, so `data` now dangles
        // in both buffers identically. Neutralise it before destroying so the
        // test does not double-free.
        if crc == -1 {
            (*cb).data = std::ptr::null_mut();
            (*rb).data = std::ptr::null_mut();
        }
        (*cb).length = 0;
        (*rb).length = 0;
        (p.c.destroy_buffer)(cb);
        (p.rs.destroy_buffer)(rb);
    }
}

// ---------------------------------------------------------------------------
// Rows 6, 7, 14, 15 — faulting inputs, compared out-of-process.
// ---------------------------------------------------------------------------
#[test]
fn row06_append_null_buffer_faults_identically() {
    let c = run_crash_case("append_null_buffer", "c");
    let r = run_crash_case("append_null_buffer", "rust");
    err!(6, "append_to_buffer(NULL, \"x\")", c, r);
    assert_eq!(c, Outcome::Signal(libc_sigsegv()), "expected SIGSEGV, got {c:?}");
}

#[test]
fn row07_append_null_str_faults_identically() {
    let c = run_crash_case("append_null_str", "c");
    let r = run_crash_case("append_null_str", "rust");
    err!(7, "append_to_buffer(buf, NULL)", c, r);
    assert_eq!(c, Outcome::Signal(libc_sigsegv()), "expected SIGSEGV, got {c:?}");
}

#[test]
fn row14_divide_int_min_by_minus_one_faults_identically() {
    let c = run_crash_case("divide_int_min", "c");
    let r = run_crash_case("divide_int_min", "rust");
    err!(14, "perform_operation(INT_MIN, -1, \"divide\")", c, r);
    assert_eq!(c, Outcome::Signal(libc_sigfpe()), "expected SIGFPE, got {c:?}");
}

#[test]
fn row15_perform_operation_null_operation_faults_identically() {
    let c = run_crash_case("perform_null_op", "c");
    let r = run_crash_case("perform_null_op", "rust");
    err!(15, "perform_operation(1, 2, NULL)", c, r);
    assert_eq!(c, Outcome::Signal(libc_sigsegv()), "expected SIGSEGV, got {c:?}");
}

/// ERRORS.md row 20 — `length == INT_MAX` makes `required_capacity` wrap to
/// `INT_MIN`, which is *not* `> capacity`, so the C skips the realloc guard
/// entirely and performs `strcpy(data + INT_MAX, str)`. Both libraries must
/// fault the same way.
#[test]
fn row20_append_length_int_max_wraps_past_the_growth_guard() {
    let c = run_crash_case("append_length_int_max", "c");
    let r = run_crash_case("append_length_int_max", "rust");
    err!(20, "append with length == INT_MAX", c, r);
    assert_eq!(c, Outcome::Signal(libc_sigsegv()), "expected SIGSEGV, got {c:?}");
}

/// ERRORS.md row 21 — a large negative `length` also bypasses the growth guard
/// and writes below the allocation.
#[test]
fn row21_append_large_negative_length_writes_below_allocation() {
    let c = run_crash_case("append_length_negative", "c");
    let r = run_crash_case("append_length_negative", "rust");
    err!(21, "append with length == INT_MIN/2", c, r);
    assert_eq!(c, Outcome::Signal(libc_sigsegv()), "expected SIGSEGV, got {c:?}");
}

/// Control: the out-of-process harness must report a clean exit for a case
/// that does not fault, otherwise the rows above would pass vacuously.
#[test]
fn crash_harness_control_case_exits_cleanly() {
    assert_eq!(run_crash_case("ok", "c"), Outcome::Exit(0));
    assert_eq!(run_crash_case("ok", "rust"), Outcome::Exit(0));
}

fn libc_sigsegv() -> i32 {
    11
}
fn libc_sigfpe() -> i32 {
    8
}

// ---------------------------------------------------------------------------
// Row 8 — no-growth append of an empty string.
// ---------------------------------------------------------------------------
#[test]
fn row08_append_empty_string_no_growth() {
    let p = pair();
    unsafe {
        for cap in [2, 8, 32, 1024] {
            let empty = b"\0";
            let cb = (p.c.create_buffer)(cap);
            let rb = (p.rs.create_buffer)(cap);
            let crc = (p.c.append_to_buffer)(cb, empty.as_ptr() as *const c_char);
            let rrc = (p.rs.append_to_buffer)(rb, empty.as_ptr() as *const c_char);
            err!(8, cap, (crc, snapshot(cb)), (rrc, snapshot(rb)));
            assert_eq!(crc, 0);
            assert_eq!((*cb).capacity, cap, "no realloc for empty string");
            assert_eq!((*cb).length, 0);
            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — destroy_buffer(NULL) is a no-op.
// ---------------------------------------------------------------------------
#[test]
fn row09_destroy_buffer_null_is_noop() {
    let p = pair();
    unsafe {
        // Called repeatedly: if either library dereferenced NULL the test
        // binary would die, which the harness reports as a failure.
        for _ in 0..1000 {
            (p.c.destroy_buffer)(std::ptr::null_mut());
            (p.rs.destroy_buffer)(std::ptr::null_mut());
        }
    }
    err!(9, "destroy_buffer(NULL)", (), ());
}

// ---------------------------------------------------------------------------
// Row 10 — destroy_buffer with a NULL `data` field skips free(data).
// ---------------------------------------------------------------------------
#[test]
fn row10_destroy_buffer_null_data_field() {
    let p = pair();
    unsafe {
        for _ in 0..500 {
            let cb = (p.c.create_buffer)(8);
            let rb = (p.rs.create_buffer)(8);
            // Release `data` ourselves and NULL the field, so destroy_buffer
            // must take the `if (buffer->data)` false branch.
            free((*cb).data as *mut c_void);
            free((*rb).data as *mut c_void);
            (*cb).data = std::ptr::null_mut();
            (*rb).data = std::ptr::null_mut();
            err!(10, "pre-destroy", snapshot(cb), snapshot(rb));
            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — out-of-range "enum" values for get_operation_name.
// This is the out-of-range-enum-across-FFI case: C enums/ints accept any value.
// ---------------------------------------------------------------------------
#[test]
fn row11_get_operation_name_out_of_range_codes() {
    let p = pair();
    let mut rng = Rng::new(0x0011_0011);
    unsafe {
        // One step past each end of the documented 0..=3 range, the residues a
        // negative `% 4` produces, and the integer extremes.
        let mut codes: Vec<c_int> = vec![
            -1,
            -2,
            -3,
            -4,
            -5,
            4,
            5,
            6,
            7,
            100,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
        ];
        for _ in 0..4000 {
            codes.push(rng.next_i32());
        }
        for code in codes {
            let c = cstr_bytes((p.c.get_operation_name)(code));
            let r = cstr_bytes((p.rs.get_operation_name)(code));
            err!(11, code, c, r);
            if !(0..=3).contains(&code) {
                assert_eq!(
                    c.as_deref(),
                    Some(&b"unknown"[..]),
                    "code {code} must map to \"unknown\""
                );
            }
        }
        // The returned pointers must also be usable by the *other* library.
        for code in [-1, 4, i32::MIN, i32::MAX] {
            let c_ptr = (p.c.get_operation_name)(code);
            let r_ptr = (p.rs.get_operation_name)(code);
            err!(
                11,
                (code, "cross-fed"),
                (p.c.perform_operation)(7, 3, r_ptr),
                (p.rs.perform_operation)(7, 3, c_ptr)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — unmatched operation string falls through to `return 0`.
// ---------------------------------------------------------------------------
#[test]
fn row12_perform_operation_unmatched_operation_returns_zero() {
    let p = pair();
    let mut rng = Rng::new(0x0012_0012);
    unsafe {
        let fixed: Vec<&[u8]> = vec![
            b"\0",
            b"unknown\0",
            b"ADD\0",
            b"Add\0",
            b"add \0",
            b" add\0",
            b"addd\0",
            b"ad\0",
            b"subtrac\0",
            b"multiply \0",
            b"divide\t\0",
            b"DIVIDE\0",
            b"\x80\x81\0",
            b"\xff\0",
            b"\x01\0",
        ];
        for op in fixed {
            for (a, b) in [(0, 0), (1, 2), (i32::MIN, -1), (i32::MAX, i32::MAX)] {
                let ptr = op.as_ptr() as *const c_char;
                let c = (p.c.perform_operation)(a, b, ptr);
                let r = (p.rs.perform_operation)(a, b, ptr);
                err!(12, (op, a, b), c, r);
                assert_eq!(c, 0, "unmatched operation {op:?} must return 0");
            }
        }
        // b"add\0extra\0" compares EQUAL to "add" (strcmp stops at the first
        // NUL), so it takes the add branch rather than returning 0. Verify both
        // libraries agree on that.
        let tricky = b"add\0extra\0";
        let c = (p.c.perform_operation)(3, 4, tricky.as_ptr() as *const c_char);
        let r = (p.rs.perform_operation)(3, 4, tricky.as_ptr() as *const c_char);
        err!(12, "embedded NUL", c, r);
        assert_eq!(c, 7, "embedded NUL: strcmp matches \"add\"");
        let tricky2 = b"subtract\0x\0";
        let c = (p.c.perform_operation)(3, 4, tricky2.as_ptr() as *const c_char);
        let r = (p.rs.perform_operation)(3, 4, tricky2.as_ptr() as *const c_char);
        err!(12, "embedded NUL 2", c, r);
        assert_eq!(c, -1, "embedded NUL: strcmp matches \"subtract\"");

        for _ in 0..4000 {
            let s = rng.cstring_len(0, 20);
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let ptr = s.as_ptr() as *const c_char;
            let c = (p.c.perform_operation)(a, b, ptr);
            let r = (p.rs.perform_operation)(a, b, ptr);
            err!(12, (&s, a, b), c, r);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — divide with b == 0 returns 0 without dividing.
// ---------------------------------------------------------------------------
#[test]
fn row13_perform_operation_divide_by_zero_returns_zero() {
    let p = pair();
    let mut rng = Rng::new(0x0013_0013);
    unsafe {
        let div = b"divide\0";
        let ptr = div.as_ptr() as *const c_char;
        let mut vals: Vec<c_int> = vec![0, 1, -1, i32::MIN, i32::MAX];
        for _ in 0..3000 {
            vals.push(rng.interesting_i32());
        }
        for a in vals {
            let c = (p.c.perform_operation)(a, 0, ptr);
            let r = (p.rs.perform_operation)(a, 0, ptr);
            err!(13, a, c, r);
            assert_eq!(c, 0, "divide({a}, 0) must return 0");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — signed-overflow arithmetic must wrap identically.
// ---------------------------------------------------------------------------
#[test]
fn row16_signed_overflow_wraps_identically() {
    let p = pair();
    unsafe {
        let cases: [(&[u8], c_int, c_int, c_int); 8] = [
            (b"add\0", i32::MAX, 1, i32::MIN),
            (b"add\0", i32::MIN, -1, i32::MAX),
            (b"add\0", i32::MAX, i32::MAX, -2),
            (b"subtract\0", i32::MIN, 1, i32::MAX),
            (b"subtract\0", i32::MAX, -1, i32::MIN),
            (b"multiply\0", i32::MIN, -1, i32::MIN),
            (b"multiply\0", i32::MAX, i32::MAX, 1),
            (b"multiply\0", i32::MIN, i32::MIN, 0),
        ];
        for (op, a, b, want) in cases {
            let ptr = op.as_ptr() as *const c_char;
            let c = (p.c.perform_operation)(a, b, ptr);
            let r = (p.rs.perform_operation)(a, b, ptr);
            err!(16, (op, a, b), c, r);
            assert_eq!(c, want, "C wrapping result for {op:?} {a} {b}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — buffapp does not NULL-check create_buffer(32). Unreachable (a
// 32-byte allocation does not fail); asserted as "create_buffer(32) succeeds",
// which is what makes the missing check harmless in practice.
// ---------------------------------------------------------------------------
#[test]
fn row17_buffapp_unchecked_create_buffer_is_unreachable() {
    let p = pair();
    unsafe {
        for _ in 0..1000 {
            let cb = (p.c.create_buffer)(32);
            let rb = (p.rs.create_buffer)(32);
            err!(17, 32, cb.is_null(), rb.is_null());
            assert!(!cb.is_null() && !rb.is_null());
            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}



// ---------------------------------------------------------------------------
// Generic FFI boundary sweep (beyond the table): NULL, zero, oversized.
// ---------------------------------------------------------------------------
#[test]
fn generic_boundary_sweep() {
    let p = pair();
    unsafe {
        // Zero and one-past-range capacities. Sequenced so a multi-gigabyte
        // request from one library is not perturbed by the other's live block.
        for cap in [0, 1, -1, i32::MIN, i32::MAX] {
            let cb = (p.c.create_buffer)(cap);
            let c = (cb.is_null(), if cb.is_null() { None } else { Some(snapshot(cb)) });
            (p.c.destroy_buffer)(cb);
            let rb = (p.rs.create_buffer)(cap);
            let r = (rb.is_null(), if rb.is_null() { None } else { Some(snapshot(rb)) });
            (p.rs.destroy_buffer)(rb);
            err!("generic", cap, c, r);
        }
        // destroy_buffer must tolerate NULL from a failed create_buffer.
        let cb = (p.c.create_buffer)(-1);
        let rb = (p.rs.create_buffer)(-1);
        (p.c.destroy_buffer)(cb);
        (p.rs.destroy_buffer)(rb);
    }
}

// ---------------------------------------------------------------------------
// The re-exec'd helper used by rows 6, 7, 14, 15.
// ---------------------------------------------------------------------------
mod crash_helper {
    use super::support::*;
    use std::ffi::{c_char, c_int};

    #[test]
    #[ignore = "re-exec'd by the out-of-process error-path tests"]
    fn helper() {
        let case = match std::env::var("BUFFAPP_CRASH_CASE") {
            Ok(v) => v,
            Err(_) => return, // invoked directly, e.g. by `--include-ignored`
        };
        let which = std::env::var("BUFFAPP_CRASH_IMPL").unwrap_or_else(|_| "c".into());
        let p = pair();
        let imp = if which == "rust" { &p.rs } else { &p.c };
        unsafe {
            match case.as_str() {
                "append_null_buffer" => {
                    let s = b"x\0";
                    let rc =
                        (imp.append_to_buffer)(std::ptr::null_mut(), s.as_ptr() as *const c_char);
                    // Not reached; keep the value observable so nothing is
                    // optimized away.
                    std::process::exit(100 + (rc & 1));
                }
                "append_null_str" => {
                    let buf = (imp.create_buffer)(32);
                    let rc = (imp.append_to_buffer)(buf, std::ptr::null());
                    std::process::exit(100 + (rc & 1));
                }
                "perform_null_op" => {
                    let rc = (imp.perform_operation)(1, 2, std::ptr::null());
                    std::process::exit(100 + (rc & 1));
                }
                "divide_int_min" => {
                    let div = b"divide\0";
                    let rc = (imp.perform_operation)(
                        c_int::MIN,
                        -1,
                        div.as_ptr() as *const c_char,
                    );
                    std::process::exit(100 + (rc & 1));
                }
                "append_length_int_max" => {
                    let buf = (imp.create_buffer)(64);
                    (*buf).length = c_int::MAX;
                    let s = b"x\0";
                    let rc = (imp.append_to_buffer)(buf, s.as_ptr() as *const c_char);
                    std::process::exit(100 + (rc & 1));
                }
                "append_length_negative" => {
                    let buf = (imp.create_buffer)(64);
                    // required_capacity stays hugely negative => not > capacity
                    // => no realloc => strcpy far below `data`.
                    (*buf).length = c_int::MIN / 2;
                    let s = b"x\0";
                    let rc = (imp.append_to_buffer)(buf, s.as_ptr() as *const c_char);
                    std::process::exit(100 + (rc & 1));
                }
                "ok" => {
                    let buf = (imp.create_buffer)(32);
                    let s = b"hello\0";
                    let rc = (imp.append_to_buffer)(buf, s.as_ptr() as *const c_char);
                    (imp.destroy_buffer)(buf);
                    assert_eq!(rc, 0);
                    std::process::exit(0);
                }
                other => panic!("unknown crash case {other}"),
            }
        }
    }
}
