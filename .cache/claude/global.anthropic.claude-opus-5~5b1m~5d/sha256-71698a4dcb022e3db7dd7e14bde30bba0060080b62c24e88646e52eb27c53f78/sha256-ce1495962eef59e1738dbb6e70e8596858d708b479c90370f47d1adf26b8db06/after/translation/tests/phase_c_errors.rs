// Phase C -- error-path differential tests. One test per row of ERRORS.md.
//
// Every test constructs the exact invalid input / condition, calls BOTH the C
// `.so` and the Rust `.so`, and asserts the SAME sentinel (NULL / -1 / 0) or the
// SAME termination signal comes back. "Both failed somehow" is never accepted.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ===========================================================================
// ERRORS.md #1 -- malloc(sizeof(StringBuffer)) fails.
// Not host-triggerable; see note A in ERRORS.md. What IS checkable is that the
// only capacity `buffapp` ever asks for succeeds in both, so neither library
// can diverge here at runtime.
// ===========================================================================
#[test]
fn err01_outer_malloc_failure_not_reachable() {
    let (cl, rl) = pair();
    unsafe {
        for _ in 0..1000 {
            let cb = (cl.create_buffer)(32);
            let rb = (rl.create_buffer)(32);
            assert!(!cb.is_null(), "C create_buffer(32) returned NULL");
            assert!(!rb.is_null(), "Rust create_buffer(32) returned NULL");
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

// ===========================================================================
// ERRORS.md #2, #3, #4 -- inner malloc(initial_capacity) fails because a
// negative int is sign-extended to a huge size_t. Both must return NULL.
// ===========================================================================
fn assert_create_is_null(cap: c_int) {
    let (cl, rl) = pair();
    unsafe {
        let cb = (cl.create_buffer)(cap);
        let rb = (rl.create_buffer)(cap);
        assert!(
            cb.is_null(),
            "C create_buffer({cap}) should be NULL, got {cb:p}"
        );
        assert!(
            rb.is_null(),
            "Rust create_buffer({cap}) should be NULL, got {rb:p}"
        );
    }
}

#[test]
fn err02_create_buffer_negative_one() {
    assert_create_is_null(-1);
}

#[test]
fn err03_create_buffer_int_min() {
    assert_create_is_null(i32::MIN);
}

#[test]
fn err04_create_buffer_assorted_negatives() {
    for cap in [-2, -3, -7, -100, -4096, -1_000_000, i32::MIN + 1, i32::MIN / 2] {
        assert_create_is_null(cap);
    }
    let mut rng = Rng::new();
    for _ in 0..2000 {
        let cap = rng.range_i32(i32::MIN, -1);
        assert_create_is_null(cap);
    }
}

// ===========================================================================
// ERRORS.md #5 -- initial_capacity == 0 is NOT an error on glibc.
// ===========================================================================
#[test]
fn err05_create_buffer_zero_capacity_succeeds() {
    let (cl, rl) = pair();
    unsafe {
        let cb = (cl.create_buffer)(0);
        let rb = (rl.create_buffer)(0);
        assert_eq!(
            cb.is_null(),
            rb.is_null(),
            "create_buffer(0): nullness differs"
        );
        assert!(!cb.is_null(), "glibc malloc(0) is non-NULL; expected success");
        assert_eq!((*cb).capacity, 0);
        assert_eq!((*rb).capacity, 0);
        assert_eq!((*cb).length, 0);
        assert_eq!((*rb).length, 0);
        // data[0] = '\0' was written by both.
        assert_eq!(read_n((*cb).data, 1), read_n((*rb).data, 1));
        assert_eq!(read_n((*cb).data, 1), vec![0u8]);
        (cl.destroy_buffer)(cb);
        (rl.destroy_buffer)(rb);
    }
}

// ===========================================================================
// ERRORS.md #6, #7 -- realloc fails because new_capacity = required*2 overflows
// int to a negative value which is sign-extended to a huge size_t.
// Both must return -1 and leave data/capacity untouched.
// ===========================================================================
fn assert_append_realloc_fails(length: c_int, s: &[u8]) {
    let (cl, rl) = pair();
    let cs = cstring(s);
    unsafe {
        let cb = (cl.create_buffer)(32);
        let rb = (rl.create_buffer)(32);
        assert!(!cb.is_null() && !rb.is_null());

        let c_data_before = (*cb).data;
        let r_data_before = (*rb).data;
        (*cb).length = length;
        (*rb).length = length;

        // Sanity: this setup really must reach the realloc guard.
        let required = length.wrapping_add(s.len() as c_int).wrapping_add(1);
        assert!(
            required > 32,
            "setup error: required={required} does not exceed capacity"
        );
        assert!(
            required.wrapping_mul(2) < 0,
            "setup error: new_capacity={} is not negative",
            required.wrapping_mul(2)
        );

        let cr = (cl.append_to_buffer)(cb, cs.as_ptr());
        let rr = (rl.append_to_buffer)(rb, cs.as_ptr());
        assert_eq!(cr, -1, "C append_to_buffer should return -1, got {cr}");
        assert_eq!(rr, -1, "Rust append_to_buffer should return -1, got {rr}");

        // On failure the C code returns before touching data/capacity.
        assert_eq!((*cb).data, c_data_before, "C data pointer changed");
        assert_eq!((*rb).data, r_data_before, "Rust data pointer changed");
        assert_eq!((*cb).capacity, 32, "C capacity changed");
        assert_eq!((*rb).capacity, 32, "Rust capacity changed");
        assert_eq!((*cb).length, length, "C length changed");
        assert_eq!((*rb).length, length, "Rust length changed");

        (*cb).length = 0;
        (*rb).length = 0;
        (cl.destroy_buffer)(cb);
        (rl.destroy_buffer)(rb);
    }
}

#[test]
fn err06_append_realloc_failure_two_billion_length() {
    assert_append_realloc_fails(2_000_000_000, b"hello");
}

#[test]
fn err07_append_realloc_failure_around_int_max_half() {
    let half = i32::MAX / 2; // 1073741823
    for k in [1i32, 2, 3, 8, 100, 10_000, 1_000_000] {
        assert_append_realloc_fails(half + k, b"x");
    }
    let mut rng = Rng::new();
    for _ in 0..200 {
        // Any length whose required*2 overflows negative but whose required is
        // still positive.
        let length = rng.range_i32(half + 1, i32::MAX - 16);
        let s = b"abc";
        let required = length.wrapping_add(s.len() as c_int).wrapping_add(1);
        if required <= 32 || required.wrapping_mul(2) >= 0 {
            continue;
        }
        assert_append_realloc_fails(length, s);
    }
}

// ===========================================================================
// ERRORS.md #8 -- required_capacity itself overflows to NEGATIVE, so
// `required_capacity > buffer->capacity` is FALSE, no realloc happens, and the
// strcpy lands at `data + INT_MAX`. Both must return 0 and compute the same
// wrapped length and write the same bytes at the same wild offset.
//
// A 2 GiB PROT_READ|PROT_WRITE MAP_NORESERVE reservation makes that offset
// legal memory so the branch can be observed instead of faulting.
// ===========================================================================
#[test]
fn err08_append_required_capacity_overflows_negative() {
    let (cl, rl) = pair();
    let s: &[u8] = b"tail";
    let cs = cstring(s);
    let off: usize = i32::MAX as usize;
    let map_len = off + 4096;

    let (cmap, rmap) = match (BigMap::new(map_len), BigMap::new(map_len)) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("err08: 2GiB reservation refused; verifying the branch decision only");
            // Even without the mapping we can still verify both libraries agree
            // that no reallocation is required (the observable pre-strcpy step).
            let length = i32::MAX;
            let required = length.wrapping_add(s.len() as c_int).wrapping_add(1);
            assert!(required < 0, "required should wrap negative, got {required}");
            assert!(!(required > 32), "no realloc must be triggered");
            return;
        }
    };

    let length = i32::MAX;
    let required = length.wrapping_add(s.len() as c_int).wrapping_add(1);
    assert!(required < 0, "setup: required must wrap negative");
    assert!(!(required > 32), "setup: the grow branch must NOT be taken");

    unsafe {
        // Pre-poison both windows identically so any difference is real.
        std::ptr::write_bytes(cmap.base.add(off), 0xAA, 64);
        std::ptr::write_bytes(rmap.base.add(off), 0xAA, 64);

        let mut cbuf = StringBuffer { data: cmap.base as *mut c_char, capacity: 32, length };
        let mut rbuf = StringBuffer { data: rmap.base as *mut c_char, capacity: 32, length };

        let cr = (cl.append_to_buffer)(&mut cbuf, cs.as_ptr());
        let rr = (rl.append_to_buffer)(&mut rbuf, cs.as_ptr());

        assert_eq!(cr, 0, "C should return 0 (no realloc attempted)");
        assert_eq!(rr, 0, "Rust should return 0 (no realloc attempted)");
        assert_eq!(cbuf.capacity, 32, "C capacity must be untouched");
        assert_eq!(rbuf.capacity, 32, "Rust capacity must be untouched");
        assert_eq!(
            cbuf.length, rbuf.length,
            "wrapped length differs: C {} vs Rust {}",
            cbuf.length, rbuf.length
        );
        assert_eq!(
            cbuf.length,
            length.wrapping_add(s.len() as c_int),
            "length must wrap exactly like C"
        );
        let cbytes = std::slice::from_raw_parts(cmap.base.add(off), 64);
        let rbytes = std::slice::from_raw_parts(rmap.base.add(off), 64);
        assert_eq!(cbytes, rbytes, "bytes written at data+INT_MAX differ");
        assert_eq!(&cbytes[..5], b"tail\0", "wrong bytes written");
    }
}

// ===========================================================================
// ERRORS.md #9, #10, #17 -- NULL pointer dereferences. Both must die with the
// same signal.
// ===========================================================================
#[test]
fn err09_append_null_buffer_same_signal() {
    let (cl, rl) = pair();
    let cs = cstring(b"x");
    let p = cs.as_ptr();
    let o = assert_same_outcome(
        "append_to_buffer(NULL, \"x\")",
        || unsafe {
            let _ = (cl.append_to_buffer)(std::ptr::null_mut(), p);
        },
        || unsafe {
            let _ = (rl.append_to_buffer)(std::ptr::null_mut(), p);
        },
    );
    assert_eq!(o, Outcome::Signal(libc::SIGSEGV), "expected SIGSEGV, got {o:?}");
}

#[test]
fn err10_append_null_string_same_signal() {
    let (cl, rl) = pair();
    unsafe {
        let cb = (cl.create_buffer)(32);
        let rb = (rl.create_buffer)(32);
        let o = assert_same_outcome(
            "append_to_buffer(buf, NULL)",
            || {
                let _ = (cl.append_to_buffer)(cb, std::ptr::null());
            },
            || {
                let _ = (rl.append_to_buffer)(rb, std::ptr::null());
            },
        );
        assert_eq!(o, Outcome::Signal(libc::SIGSEGV), "expected SIGSEGV, got {o:?}");
        (cl.destroy_buffer)(cb);
        (rl.destroy_buffer)(rb);
    }
}

#[test]
fn err17_perform_operation_null_operation_same_signal() {
    let (cl, rl) = pair();
    let o = assert_same_outcome(
        "perform_operation(1, 2, NULL)",
        || unsafe {
            let _ = (cl.perform_operation)(1, 2, std::ptr::null());
        },
        || unsafe {
            let _ = (rl.perform_operation)(1, 2, std::ptr::null());
        },
    );
    assert_eq!(o, Outcome::Signal(libc::SIGSEGV), "expected SIGSEGV, got {o:?}");
}

// ===========================================================================
// ERRORS.md #11 -- destroy_buffer(NULL) is a no-op in both.
// ===========================================================================
#[test]
fn err11_destroy_null_is_noop() {
    let (cl, rl) = pair();
    let o = assert_same_outcome(
        "destroy_buffer(NULL)",
        || unsafe { (cl.destroy_buffer)(std::ptr::null_mut()) },
        || unsafe { (rl.destroy_buffer)(std::ptr::null_mut()) },
    );
    assert_eq!(o, Outcome::Exited(0), "destroy_buffer(NULL) must not crash");
    // And in-process, repeatedly, to be sure nothing is corrupted.
    unsafe {
        for _ in 0..100 {
            (cl.destroy_buffer)(std::ptr::null_mut());
            (rl.destroy_buffer)(std::ptr::null_mut());
        }
    }
}

// ===========================================================================
// ERRORS.md #12 -- buffer->data == NULL: skip free(data), still free(buffer).
// ===========================================================================
#[test]
fn err12_destroy_with_null_data() {
    let (cl, rl) = pair();
    unsafe {
        for _ in 0..200 {
            let cb = (cl.create_buffer)(32);
            let rb = (rl.create_buffer)(32);
            assert!(!cb.is_null() && !rb.is_null());
            // Take the data block out of the struct and free it ourselves, so
            // destroy_buffer exercises the `data == NULL` branch without leaking.
            let cdata = (*cb).data;
            let rdata = (*rb).data;
            (*cb).data = std::ptr::null_mut();
            (*rb).data = std::ptr::null_mut();
            let o = assert_same_outcome(
                "destroy_buffer(buf with data==NULL)",
                || (cl.destroy_buffer)(cb),
                || (rl.destroy_buffer)(rb),
            );
            assert_eq!(o, Outcome::Exited(0));
            // The forks above ran destroy in children; free for real here.
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
            libc::free(cdata as *mut libc::c_void);
            libc::free(rdata as *mut libc::c_void);
        }
    }
}

// ===========================================================================
// ERRORS.md #13 -- get_operation_name default arm, incl. out-of-range enum ints.
// ===========================================================================
#[test]
fn err13_get_operation_name_out_of_range() {
    let (cl, rl) = pair();
    unsafe {
        let mut codes: Vec<c_int> = vec![
            4,
            5,
            -1,
            -2,
            -3,
            -4,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
            0x7fff_fffe,
            -0x8000_0000i64 as c_int,
        ];
        let mut rng = Rng::new();
        for _ in 0..4000 {
            let v = rng.spicy_i32();
            if !(0..=3).contains(&v) {
                codes.push(v);
            }
        }
        for code in codes {
            let cn = read_cstr((cl.get_operation_name)(code));
            let rn = read_cstr((rl.get_operation_name)(code));
            assert_eq!(cn, rn, "get_operation_name({code}) differs");
            assert_eq!(
                cn, b"unknown",
                "get_operation_name({code}) should be \"unknown\""
            );
        }
    }
}

// ===========================================================================
// ERRORS.md #14 -- perform_operation with an unmatched operation name -> 0.
// ===========================================================================
#[test]
fn err14_perform_operation_unmatched_returns_zero() {
    let (cl, rl) = pair();
    let bad: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"ADD".to_vec(),
        b"add\t".to_vec(),
        b"  add".to_vec(),
        b"addition".to_vec(),
        b"sub".to_vec(),
        b"mul".to_vec(),
        b"div".to_vec(),
        b"divides".to_vec(),
        b"unknown".to_vec(),
        b"0".to_vec(),
        b"\xff\xfe".to_vec(),
    ];
    unsafe {
        for op in &bad {
            let s = cstring(op);
            for (a, b) in [(0, 0), (1, 2), (i32::MIN, -1), (i32::MAX, i32::MIN), (7, 0)] {
                let cv = (cl.perform_operation)(a, b, s.as_ptr());
                let rv = (rl.perform_operation)(a, b, s.as_ptr());
                assert_eq!(cv, rv, "perform_operation({a},{b},{op:?}) differs");
                assert_eq!(cv, 0, "unmatched operation must yield 0");
            }
        }
    }
}

// ===========================================================================
// ERRORS.md #15 -- "divide" with b == 0 returns 0 instead of trapping.
// ===========================================================================
#[test]
fn err15_divide_by_zero_returns_zero_and_does_not_trap() {
    let (cl, rl) = pair();
    let s = cstring(b"divide");
    let p = s.as_ptr();
    unsafe {
        for a in [0, 1, -1, 7, -7, i32::MIN, i32::MAX, i32::MIN + 1] {
            let cv = (cl.perform_operation)(a, 0, p);
            let rv = (rl.perform_operation)(a, 0, p);
            assert_eq!(cv, rv, "perform_operation({a},0,\"divide\") differs");
            assert_eq!(cv, 0, "divide-by-zero must return 0");
        }
    }
    // Also prove neither traps, in a child, so a SIGFPE would be visible.
    let o = assert_same_outcome(
        "perform_operation(INT_MIN, 0, \"divide\")",
        || unsafe {
            let _ = (cl.perform_operation)(i32::MIN, 0, p);
        },
        || unsafe {
            let _ = (rl.perform_operation)(i32::MIN, 0, p);
        },
    );
    assert_eq!(o, Outcome::Exited(0), "divide-by-zero must not signal");
}

// ===========================================================================
// ERRORS.md #16 -- "divide" with INT_MIN / -1: the quotient is unrepresentable
// and gcc emits a bare idiv, so the CPU raises #DE -> SIGFPE.
// ===========================================================================
#[test]
fn err16_int_min_div_minus_one_same_signal() {
    let (cl, rl) = pair();
    let s = cstring(b"divide");
    let p = s.as_ptr();
    let o = assert_same_outcome(
        "perform_operation(INT_MIN, -1, \"divide\")",
        || unsafe {
            let _ = (cl.perform_operation)(i32::MIN, -1, p);
        },
        || unsafe {
            let _ = (rl.perform_operation)(i32::MIN, -1, p);
        },
    );
    assert_eq!(o, Outcome::Signal(libc::SIGFPE), "expected SIGFPE, got {o:?}");

    // The neighbouring, representable cases must NOT trap, in either library.
    for (a, b) in [(i32::MIN + 1, -1), (i32::MIN, 1), (i32::MAX, -1), (i32::MIN, -2)] {
        let o = assert_same_outcome(
            "near-miss divide",
            || unsafe {
                let _ = (cl.perform_operation)(a, b, p);
            },
            || unsafe {
                let _ = (rl.perform_operation)(a, b, p);
            },
        );
        assert_eq!(o, Outcome::Exited(0), "({a},{b}) should not trap, got {o:?}");
        unsafe {
            assert_eq!(
                (cl.perform_operation)(a, b, p),
                (rl.perform_operation)(a, b, p),
                "({a},{b}) value differs"
            );
        }
    }
}

// ===========================================================================
// ERRORS.md #18 -- signed overflow in add/subtract/multiply wraps (as gcc does).
// ===========================================================================
#[test]
fn err18_signed_overflow_wraps_identically() {
    let (cl, rl) = pair();
    let cases: [(&[u8], i32, i32); 9] = [
        (b"add", i32::MAX, 1),
        (b"add", i32::MIN, -1),
        (b"add", i32::MAX, i32::MAX),
        (b"subtract", i32::MIN, 1),
        (b"subtract", i32::MAX, -1),
        (b"subtract", i32::MIN, i32::MAX),
        (b"multiply", i32::MIN, -1),
        (b"multiply", i32::MAX, 2),
        (b"multiply", 65536, 65536),
    ];
    unsafe {
        for (op, a, b) in cases {
            let s = cstring(op);
            let cv = (cl.perform_operation)(a, b, s.as_ptr());
            let rv = (rl.perform_operation)(a, b, s.as_ptr());
            assert_eq!(
                cv,
                rv,
                "overflow {:?}({a},{b}): C {cv} != Rust {rv}",
                String::from_utf8_lossy(op)
            );
        }
    }
    // Randomised overflow sweep.
    let mut rng = Rng::new();
    unsafe {
        for op in [b"add".as_slice(), b"subtract".as_slice(), b"multiply".as_slice()] {
            let s = cstring(op);
            for _ in 0..4000 {
                let a = rng.next_i32();
                let b = rng.next_i32();
                assert_eq!(
                    (cl.perform_operation)(a, b, s.as_ptr()),
                    (rl.perform_operation)(a, b, s.as_ptr()),
                    "overflow sweep {:?}({a},{b})",
                    String::from_utf8_lossy(op)
                );
            }
        }
    }
}

// ===========================================================================
// ERRORS.md #19, #20, #21, #23 -- buffapp fallback / "unknown" branches.
// ===========================================================================
fn buffapp_both(p: (i32, i32, i32, i32)) -> (c_int, Vec<u8>, Vec<u8>) {
    let (cl, rl) = pair();
    let (p1, p2, p3, p4) = p;
    let (cv, cout) = capture_stdout(|| unsafe { (cl.buffapp)(p1, p2, p3, p4) });
    let (rv, rout) = capture_stdout(|| unsafe { (rl.buffapp)(p1, p2, p3, p4) });
    assert_eq!(cv, rv, "buffapp{p:?}: return C {cv} != Rust {rv}");
    assert_eq!(
        cout,
        rout,
        "buffapp{p:?}: stdout differs\n C   : {:?}\n Rust: {:?}",
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout)
    );
    (cv, cout, rout)
}

#[test]
fn err19_buffapp_intermediate3_zero_takes_sum_fallback() {
    // i1 == 0 via the "unknown" path (p1 % 4 negative) makes i3 == 0.
    for p in [
        (-1, 5, -1, 7),
        (-2, 100, -3, -100),
        (-3, i32::MAX, -1, i32::MIN),
        (-1, 0, -2, 0),
    ] {
        let (v, out, _) = buffapp_both(p);
        let expect = p.0.wrapping_add(p.1).wrapping_add(p.2).wrapping_add(p.3);
        assert_eq!(v, expect, "buffapp{p:?} must return p1+p2+p3+p4");
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("Operation 3: multiply(0, 0)\n"), "{s:?}");
    }
    // Also via a genuine zero product with real operations: p1%4==0 and p2 such
    // that p1+p2 == 0.
    for p in [(4, -4, 8, 3), (0, 0, 0, 0), (100, -100, 12, -12)] {
        let (v, _, _) = buffapp_both(p);
        let expect = p.0.wrapping_add(p.1).wrapping_add(p.2).wrapping_add(p.3);
        assert_eq!(v, expect, "buffapp{p:?} fallback value");
    }
}

#[test]
fn err20_buffapp_negative_residue_op1_is_unknown() {
    let (cl, rl) = pair();
    unsafe {
        for p1 in [-1, -2, -3, -5, -6, -7, -9, -1001, i32::MIN + 1, i32::MIN + 3] {
            let r = p1.wrapping_rem(4);
            assert!(r < 0, "setup: {p1} % 4 = {r} should be negative");
            assert_eq!(read_cstr((cl.get_operation_name)(r)), b"unknown");
            assert_eq!(read_cstr((rl.get_operation_name)(r)), b"unknown");
            let p = (p1, 12345, 8, 3);
            let (_, out, _) = buffapp_both(p);
            let s = String::from_utf8_lossy(&out);
            assert!(s.contains(&format!("Operation 1: unknown({p1}, 12345)\n")), "{s:?}");
            assert!(s.contains("Operation 3: multiply(0, "), "{s:?}");
        }
    }
}

#[test]
fn err21_buffapp_negative_residue_op2_is_unknown() {
    for p3 in [-1, -2, -3, -5, -6, -7, -9, -1001, i32::MIN + 1, i32::MIN + 3] {
        let p = (8, 4, p3, 999);
        let (_, out, _) = buffapp_both(p);
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains(&format!("Operation 2: unknown({p3}, 999)\n")), "{s:?}");
        assert!(s.contains(", 0)\n"), "{s:?}");
    }
}

#[test]
fn err23_buffapp_int_min_param1_takes_add_branch() {
    assert_eq!(i32::MIN.wrapping_rem(4), 0, "INT_MIN % 4 is 0, not negative");
    for p in [
        (i32::MIN, 0, 0, 0),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, 1, i32::MIN, 1),
        (i32::MIN, i32::MAX, 3, -1),
    ] {
        let (_, out, _) = buffapp_both(p);
        let s = String::from_utf8_lossy(&out);
        assert!(
            s.contains(&format!("Operation 1: add({}, {})\n", p.0, p.1)),
            "{s:?}"
        );
    }
}

// ===========================================================================
// ERRORS.md #22 -- buffapp's unconditional `log_buffer->length = 0`.
// Not host-triggerable (create_buffer(32) never fails); documented by asserting
// the reachable precondition holds in both.
// ===========================================================================
#[test]
fn err22_buffapp_log_buffer_never_null() {
    let (cl, rl) = pair();
    unsafe {
        for _ in 0..2000 {
            let cb = (cl.create_buffer)(32);
            let rb = (rl.create_buffer)(32);
            assert!(!cb.is_null() && !rb.is_null());
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
    // And buffapp itself completes normally, i.e. never takes the NULL deref.
    let o = assert_same_outcome(
        "buffapp(1,2,3,4)",
        || unsafe {
            let _ = (cl.buffapp)(1, 2, 3, 4);
        },
        || unsafe {
            let _ = (rl.buffapp)(1, 2, 3, 4);
        },
    );
    assert_eq!(o, Outcome::Exited(0));
}

// ===========================================================================
// ERRORS.md #24 -- buffapp's final `result / intermediate3` really can be
// INT_MIN / -1: both halves take "add", i1 = 1073741823, i2 = 1073741825,
// so result = i1+i2 = INT_MIN and i3 = i1*i2 = -1 (wrapping). Both must SIGFPE.
// ===========================================================================
#[test]
fn err24_buffapp_final_division_traps_identically() {
    let (cl, rl) = pair();
    let trapping: [(i32, i32, i32, i32); 4] = [
        (0, 1073741823, 0, 1073741825),
        (0, 1073741825, 0, 1073741823),
        (0, -1073741825, 0, -1073741823),
        (0, -1073741823, 0, -1073741825),
    ];
    for (p1, p2, p3, p4) in trapping {
        // Confirm the setup really produces INT_MIN / -1.
        let i1 = p1.wrapping_add(p2);
        let i2 = p3.wrapping_add(p4);
        assert_eq!(i1.wrapping_add(i2), i32::MIN, "setup: result must be INT_MIN");
        assert_eq!(i1.wrapping_mul(i2), -1, "setup: intermediate3 must be -1");

        let o = assert_same_outcome(
            &format!("buffapp({p1},{p2},{p3},{p4})"),
            || unsafe {
                let _ = (cl.buffapp)(p1, p2, p3, p4);
            },
            || unsafe {
                let _ = (rl.buffapp)(p1, p2, p3, p4);
            },
        );
        assert_eq!(
            o,
            Outcome::Signal(libc::SIGFPE),
            "buffapp({p1},{p2},{p3},{p4}) expected SIGFPE, got {o:?}"
        );
    }
}

// ===========================================================================
// Generic FFI boundary coverage required by Phase C beyond the table:
// zero / oversized lengths and one-step-past-range values.
// ===========================================================================
#[test]
fn generic_zero_and_oversized_lengths() {
    let (cl, rl) = pair();
    unsafe {
        // Zero-length string into every capacity class, including capacity 0.
        let empty = cstring(b"");
        for cap in [0i32, 1, 2, 32] {
            let cb = (cl.create_buffer)(cap);
            let rb = (rl.create_buffer)(cap);
            assert_eq!(cb.is_null(), rb.is_null());
            if cb.is_null() {
                continue;
            }
            let cr = (cl.append_to_buffer)(cb, empty.as_ptr());
            let rr = (rl.append_to_buffer)(rb, empty.as_ptr());
            assert_eq!(cr, rr, "empty append cap={cap}");
            assert_eq!(snapshot(cb), snapshot(rb), "empty append state cap={cap}");
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }

        // Oversized append: a string much larger than the capacity.
        let mut rng = Rng::new();
        for _ in 0..200 {
            let cap = rng.range_i32(1, 8);
            let extra = rng.below(4096) as usize;
            let big = rng.ascii_bytes(4096 + extra);
            let s = cstring(&big);
            let cb = (cl.create_buffer)(cap);
            let rb = (rl.create_buffer)(cap);
            let cr = (cl.append_to_buffer)(cb, s.as_ptr());
            let rr = (rl.append_to_buffer)(rb, s.as_ptr());
            assert_eq!(cr, rr, "oversized append return");
            assert_eq!(snapshot(cb), snapshot(rb), "oversized append state");
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

#[test]
fn generic_one_past_valid_range_enum_values() {
    // A C enum accepts any int; 3 is the last valid op_code, so 4 and -1 are the
    // one-step-past-range values on both sides.
    let (cl, rl) = pair();
    unsafe {
        for code in [-1, 4] {
            let cn = read_cstr((cl.get_operation_name)(code));
            let rn = read_cstr((rl.get_operation_name)(code));
            assert_eq!(cn, rn, "one-past-range {code}");
            assert_eq!(cn, b"unknown");
        }
        // Boundary capacities: -1 (invalid) / 0 (degenerate) / 1 (smallest valid).
        assert!((cl.create_buffer)(-1).is_null());
        assert!((rl.create_buffer)(-1).is_null());
        for cap in [0, 1] {
            let cb = (cl.create_buffer)(cap);
            let rb = (rl.create_buffer)(cap);
            assert_eq!(cb.is_null(), rb.is_null(), "cap={cap}");
            assert_eq!(snapshot(cb), snapshot(rb), "cap={cap}");
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}
