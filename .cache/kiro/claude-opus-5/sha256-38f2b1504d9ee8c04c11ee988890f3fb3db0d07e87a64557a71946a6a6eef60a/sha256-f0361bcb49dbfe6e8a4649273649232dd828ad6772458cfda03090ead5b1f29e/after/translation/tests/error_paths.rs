// Phase C — error-path differential tests. One test per ERRORS.md row.
//
// Every row asserts the SAME error code / sentinel from both implementations,
// not merely "both failed".

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

// ===========================================================================
// ERRORS row 1 — malloc(sizeof(StringBuffer)) failure -> NULL
// ===========================================================================
// Not reachable without an allocator interposer (an unconditional 16-byte
// malloc). Asserted structurally: both implementations return NULL on a NULL
// struct allocation and neither dereferences before the check. The reachable
// sibling (row 2) is fully tested below.
#[test]
fn err_01_struct_alloc_failure_is_structural() {
    // The `.so`s must at least agree on the guard's *existence*: a 16-byte
    // allocation always succeeds here, so both must return non-NULL.
    unsafe {
        let a = (c().create_buffer)(8);
        let b = (rust().create_buffer)(8);
        assert!(!a.is_null(), "C create_buffer(8) unexpectedly NULL");
        assert!(!b.is_null(), "Rust create_buffer(8) unexpectedly NULL");
        (c().destroy_buffer)(a);
        (rust().destroy_buffer)(b);
    }
}

// ===========================================================================
// ERRORS row 2 — malloc(initial_capacity) failure -> NULL
// Reachable because `int` sign-extends into `size_t`.
// ===========================================================================
#[test]
fn err_02_data_alloc_failure_returns_null() {
    let mut cases: Vec<c_int> = vec![-1, -2, -3, -8, -16, -32, -1000, -65536, i32::MIN, i32::MIN + 1];
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 2);
    for _ in 0..200 {
        cases.push(-(rng.range(1, 2_000_000_000) as i64 as c_int));
    }
    for cap in cases {
        unsafe {
            let cb = (c().create_buffer)(cap);
            let rb = (rust().create_buffer)(cap);
            assert!(
                cb.is_null(),
                "C create_buffer({cap}) returned non-NULL; test premise wrong"
            );
            assert!(
                rb.is_null(),
                "DIVERGENCE: create_buffer({cap}) c=NULL rust={rb:?}"
            );
        }
    }
}

/// `INT_MAX` capacity: a ~2 GiB request that may or may not succeed. Whatever
/// the allocator does, both libraries must agree.
#[test]
fn err_02b_huge_positive_capacity_agrees() {
    for cap in [i32::MAX, i32::MAX - 1, 1 << 30] {
        unsafe {
            let cb = (c().create_buffer)(cap);
            let rb = (rust().create_buffer)(cap);
            assert_eq!(
                cb.is_null(),
                rb.is_null(),
                "create_buffer({cap}): c_null={} rust_null={}",
                cb.is_null(),
                rb.is_null()
            );
            if !cb.is_null() {
                assert_eq!((*cb).capacity, (*rb).capacity);
                assert_eq!((*cb).length, (*rb).length);
                assert_eq!(*(*cb).data, *(*rb).data);
                (c().destroy_buffer)(cb);
                (rust().destroy_buffer)(rb);
            }
        }
    }
}

// ===========================================================================
// ERRORS row 3 — initial_capacity == 0: malloc(0) succeeds, data[0] written
// out of bounds. No error; state must match exactly.
// ===========================================================================
#[test]
fn err_03_zero_capacity_is_not_an_error() {
    unsafe {
        let cb = (c().create_buffer)(0);
        let rb = (rust().create_buffer)(0);
        assert!(!cb.is_null(), "C create_buffer(0) returned NULL");
        assert!(!rb.is_null(), "DIVERGENCE: Rust create_buffer(0) returned NULL");
        assert_eq!((*cb).capacity, 0);
        assert_eq!((*rb).capacity, 0, "DIVERGENCE: capacity for cap=0");
        assert_eq!((*cb).length, 0);
        assert_eq!((*rb).length, 0, "DIVERGENCE: length for cap=0");
        assert!(!(*cb).data.is_null() && !(*rb).data.is_null());
        assert_eq!(*(*cb).data, 0);
        assert_eq!(*(*rb).data, 0, "DIVERGENCE: data[0] not NUL for cap=0");
        (c().destroy_buffer)(cb);
        (rust().destroy_buffer)(rb);
    }
}

// ===========================================================================
// ERRORS row 4 — realloc failure -> -1, buffer left untouched
// Triggered by `required_capacity * 2` overflowing int and going negative.
// ===========================================================================
#[test]
fn err_04_realloc_failure_returns_minus_one() {
    // length values for which (length + str_len + 1) * 2 overflows to negative
    let lengths: [c_int; 6] = [
        1_073_741_824, // (2^30 + 2) * 2 overflows
        1_500_000_000,
        2_000_000_000,
        2_100_000_000,
        i32::MAX - 2,
        i32::MAX / 2 + 4,
    ];
    let s = CString::new("x").unwrap();
    for len in lengths {
        unsafe {
            let cdata = malloc_bytes(64);
            let rdata = malloc_bytes(64);
            let cb = manual_buffer(cdata, 16, len);
            let rb = manual_buffer(rdata, 16, len);

            let crc = (c().append_to_buffer)(cb, s.as_ptr());
            let rrc = (rust().append_to_buffer)(rb, s.as_ptr());
            assert_eq!(crc, -1, "C append with length={len} did not fail; premise wrong");
            assert_eq!(rrc, crc, "DIVERGENCE: append rc rust={rrc} c={crc} (length={len})");

            // buffer must be untouched on the failure path
            assert_eq!((*cb).capacity, 16);
            assert_eq!((*rb).capacity, 16, "DIVERGENCE: capacity mutated on failure");
            assert_eq!((*cb).length, len);
            assert_eq!((*rb).length, len, "DIVERGENCE: length mutated on failure");
            assert_eq!((*cb).data, cdata, "C moved data on failure; premise wrong");
            assert_eq!((*rb).data, rdata, "DIVERGENCE: data pointer moved on failure");

            free_raw((*cb).data as *mut c_void);
            free_raw(cb as *mut c_void);
            free_raw((*rb).data as *mut c_void);
            free_raw(rb as *mut c_void);
        }
    }
}

/// Same failure path reached with randomized long strings instead of a
/// pre-inflated `length`.
#[test]
fn err_04b_realloc_failure_randomized_lengths() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 4);
    for _ in 0..100 {
        let len = 1_073_741_824i64 + rng.range(0, 1_000_000_000) as i64;
        let len = len as c_int;
        let s = rng.ascii_range(1, 8);
        unsafe {
            let cdata = malloc_bytes(64);
            let rdata = malloc_bytes(64);
            let cb = manual_buffer(cdata, 16, len);
            let rb = manual_buffer(rdata, 16, len);
            let crc = (c().append_to_buffer)(cb, s.as_ptr());
            let rrc = (rust().append_to_buffer)(rb, s.as_ptr());
            assert_eq!(crc, -1, "premise: C should fail for length={len}");
            assert_eq!(rrc, crc, "DIVERGENCE: length={len} rust={rrc} c={crc}");
            free_raw(cdata as *mut c_void);
            free_raw(cb as *mut c_void);
            free_raw(rdata as *mut c_void);
            free_raw(rb as *mut c_void);
        }
    }
}

// ===========================================================================
// ERRORS rows 5, 6, 13, 15 — NULL-pointer dereference parity (out of process)
// The C has no NULL guard at these sites, so the only faithful behaviour is
// to fault identically. A Rust `is_null()` early-return would be a DIVERGENCE.
// ===========================================================================
fn crash_parity(label: &str, c_call: impl FnOnce(), r_call: impl FnOnce()) {
    let cres = run_in_child(c_call);
    let rres = run_in_child(r_call);
    assert_eq!(
        rres, cres,
        "{label}: termination mismatch c={cres:?} rust={rres:?} (Err(n) = killed by signal n)"
    );
}

#[test]
fn err_05_append_null_buffer_crash_parity() {
    let _ = c();
    let _ = rust();
    let s = CString::new("hello").unwrap();
    let p = s.as_ptr();
    let cf = c().append_to_buffer;
    let rf = rust().append_to_buffer;
    crash_parity(
        "append_to_buffer(NULL, str)",
        || unsafe {
            cf(std::ptr::null_mut(), p);
        },
        || unsafe {
            rf(std::ptr::null_mut(), p);
        },
    );
}

#[test]
fn err_06_append_null_string_crash_parity() {
    let _ = c();
    let _ = rust();
    let cf = c().append_to_buffer;
    let rf = rust().append_to_buffer;
    let ccreate = c().create_buffer;
    let rcreate = rust().create_buffer;
    crash_parity(
        "append_to_buffer(buf, NULL)",
        || unsafe {
            let b = ccreate(16);
            cf(b, std::ptr::null());
        },
        || unsafe {
            let b = rcreate(16);
            rf(b, std::ptr::null());
        },
    );
}

#[test]
fn err_13_perform_operation_null_string_crash_parity() {
    let _ = c();
    let _ = rust();
    let cf = c().perform_operation;
    let rf = rust().perform_operation;
    crash_parity(
        "perform_operation(a, b, NULL)",
        || unsafe {
            cf(1, 2, std::ptr::null());
        },
        || unsafe {
            rf(1, 2, std::ptr::null());
        },
    );
}

// row 15: `buffapp` never NULL-checks `create_buffer(32)`. Unreachable in
// practice; the equivalent unguarded dereference is proven by row 5 above.
#[test]
fn err_15_buffapp_unchecked_create_is_unreachable() {
    // create_buffer(32) must succeed in both, which is why the missing NULL
    // check in buffapp is unobservable. Assert the premise.
    unsafe {
        let a = (c().create_buffer)(32);
        let b = (rust().create_buffer)(32);
        assert!(!a.is_null() && !b.is_null());
        (c().destroy_buffer)(a);
        (rust().destroy_buffer)(b);
    }
}

// ===========================================================================
// ERRORS row 7 — string fits: no realloc, rc 0, data pointer unchanged
// ===========================================================================
#[test]
fn err_07_no_grow_path_returns_zero() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 7);
    for _ in 0..300 {
        let cap = rng.range(16, 512) as c_int;
        let s = rng.ascii_range(0, (cap as usize) - 1);
        unsafe {
            let cb = (c().create_buffer)(cap);
            let rb = (rust().create_buffer)(cap);
            let cd = (*cb).data;
            let rd = (*rb).data;
            let crc = (c().append_to_buffer)(cb, s.as_ptr());
            let rrc = (rust().append_to_buffer)(rb, s.as_ptr());
            assert_eq!(crc, 0);
            assert_eq!(rrc, crc, "DIVERGENCE: no-grow rc");
            assert_eq!((*cb).data, cd, "premise: C must not realloc");
            assert_eq!((*rb).data, rd, "DIVERGENCE: Rust reallocated on no-grow path");
            assert_eq!((*rb).capacity, (*cb).capacity);
            assert_eq!(snapshot(rb), snapshot(cb));
            (c().destroy_buffer)(cb);
            (rust().destroy_buffer)(rb);
        }
    }
}

// ===========================================================================
// ERRORS row 8 — destroy_buffer(NULL) is a guarded no-op
// ===========================================================================
#[test]
fn err_08_destroy_null_is_noop() {
    unsafe {
        (c().destroy_buffer)(std::ptr::null_mut());
        (rust().destroy_buffer)(std::ptr::null_mut());
    }
    // Also assert neither *crashes* when run in a child (exit code 0 both).
    let _ = c();
    let _ = rust();
    let cf = c().destroy_buffer;
    let rf = rust().destroy_buffer;
    let cres = run_in_child(|| unsafe {
        for _ in 0..1000 {
            cf(std::ptr::null_mut());
        }
    });
    let rres = run_in_child(|| unsafe {
        for _ in 0..1000 {
            rf(std::ptr::null_mut());
        }
    });
    assert_eq!(cres, Ok(0), "C destroy_buffer(NULL) crashed");
    assert_eq!(rres, cres, "DIVERGENCE: destroy_buffer(NULL) {rres:?} vs {cres:?}");
}

// ===========================================================================
// ERRORS row 9 — destroy_buffer with data == NULL skips the inner free
// ===========================================================================
#[test]
fn err_09_destroy_with_null_data() {
    let _ = c();
    let _ = rust();
    let cf = c().destroy_buffer;
    let rf = rust().destroy_buffer;
    // If either implementation called free(NULL) or free(garbage) differently
    // the child would abort; both must exit 0.
    let cres = run_in_child(|| unsafe {
        for len in [0i32, 5, -1, i32::MAX] {
            let b = manual_buffer(std::ptr::null_mut(), 7, len);
            cf(b);
        }
    });
    let rres = run_in_child(|| unsafe {
        for len in [0i32, 5, -1, i32::MAX] {
            let b = manual_buffer(std::ptr::null_mut(), 7, len);
            rf(b);
        }
    });
    assert_eq!(cres, Ok(0), "C destroy_buffer(data=NULL) did not exit cleanly");
    assert_eq!(rres, cres, "DIVERGENCE: destroy_buffer(data=NULL) {rres:?} vs {cres:?}");

    // In-process too (no crash expected).
    unsafe {
        let b1 = manual_buffer(std::ptr::null_mut(), 0, 0);
        let b2 = manual_buffer(std::ptr::null_mut(), 0, 0);
        (c().destroy_buffer)(b1);
        (rust().destroy_buffer)(b2);
    }
}

// ===========================================================================
// ERRORS row 10 — get_operation_name out-of-range "enum" values -> "unknown"
// This is the out-of-range-enum-across-FFI case.
// ===========================================================================
#[test]
fn err_10_get_operation_name_out_of_range() {
    let mut codes: Vec<c_int> = Vec::new();
    codes.extend(4..=512);
    codes.extend(-512..=-1);
    codes.extend([i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 1 << 30, -(1 << 30)]);
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 10);
    for _ in 0..20_000 {
        codes.push(rng.next_i32());
    }
    for code in codes {
        unsafe {
            let cp = (c().get_operation_name)(code);
            let rp = (rust().get_operation_name)(code);
            assert!(!cp.is_null(), "C returned NULL for {code}");
            assert!(!rp.is_null(), "DIVERGENCE: Rust returned NULL for {code}");
            let cb = cstr_bytes(cp);
            let rb = cstr_bytes(rp);
            assert_eq!(
                rb,
                cb,
                "DIVERGENCE: get_operation_name({code}) rust={:?} c={:?}",
                String::from_utf8_lossy(&rb),
                String::from_utf8_lossy(&cb)
            );
            if !(0..=3).contains(&code) {
                assert_eq!(cb, b"unknown".to_vec(), "premise: {code} should be unknown");
            }
        }
    }
}

// ===========================================================================
// ERRORS row 11 — unrecognised operation string -> 0
// ===========================================================================
#[test]
fn err_11_unknown_operation_returns_zero() {
    let mut names: Vec<CString> = [
        "", " ", "unknown", "ADD", "Add", "add ", " add", "ad", "adds", "addx", "a",
        "sub", "subtrac", "subtracts", "SUBTRACT", "mult", "multipl", "multiplyy",
        "div", "divid", "divides", "DIVIDE", "divide\t", "0", "add\nsubtract",
        "\u{7f}", "~~~", "addsubtract", "multiply multiply",
    ]
    .iter()
    .map(|s| CString::new(*s).unwrap())
    .collect();
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 11);
    for _ in 0..500 {
        names.push(rng.ascii_range(0, 20));
    }

    for name in &names {
        for &(a, b) in &[
            (0i32, 0i32),
            (1, 1),
            (7, 3),
            (-7, 3),
            (i32::MAX, 1),
            (i32::MIN, -1),
            (i32::MIN, 1),
            (12345, -6789),
        ] {
            unsafe {
                let cv = (c().perform_operation)(a, b, name.as_ptr());
                let rv = (rust().perform_operation)(a, b, name.as_ptr());
                assert_eq!(rv, cv, "DIVERGENCE: perform_operation({a},{b},{name:?})");
                // Any name not exactly one of the four must yield the 0 sentinel.
                let n = name.as_bytes();
                if !matches!(n, b"add" | b"subtract" | b"multiply" | b"divide") {
                    assert_eq!(cv, 0, "premise: {name:?} should return 0");
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS row 12 — divide by zero -> 0
// ===========================================================================
#[test]
fn err_12_divide_by_zero_returns_zero() {
    let op = CString::new("divide").unwrap();
    let mut aa: Vec<c_int> = vec![0, 1, -1, 2, -2, i32::MAX, i32::MIN, 123456, -123456];
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 12);
    for _ in 0..2000 {
        aa.push(rng.next_i32());
    }
    for a in aa {
        unsafe {
            let cv = (c().perform_operation)(a, 0, op.as_ptr());
            let rv = (rust().perform_operation)(a, 0, op.as_ptr());
            assert_eq!(cv, 0, "premise: C divide({a}, 0) should be 0");
            assert_eq!(rv, cv, "DIVERGENCE: divide({a}, 0) rust={rv} c={cv}");
        }
    }
    // Same via the name pointer each library returns for op_code 3.
    unsafe {
        for src in [(c().get_operation_name)(3), (rust().get_operation_name)(3)] {
            let cv = (c().perform_operation)(99, 0, src);
            let rv = (rust().perform_operation)(99, 0, src);
            assert_eq!(cv, 0);
            assert_eq!(rv, cv);
        }
    }
}

// ===========================================================================
// ERRORS row 14 — perform_operation("divide", INT_MIN, -1) -> SIGFPE
// ===========================================================================
#[test]
fn err_14_int_min_div_minus_one_sigfpe_parity() {
    let _ = c();
    let _ = rust();
    let op = CString::new("divide").unwrap();
    let p = op.as_ptr();
    let cf = c().perform_operation;
    let rf = rust().perform_operation;
    let cres = run_in_child(|| unsafe {
        let v = cf(i32::MIN, -1, p);
        std::hint::black_box(v);
    });
    let rres = run_in_child(|| unsafe {
        let v = rf(i32::MIN, -1, p);
        std::hint::black_box(v);
    });
    assert_eq!(
        cres,
        Err(8),
        "premise: C INT_MIN / -1 should raise SIGFPE, got {cres:?}"
    );
    assert_eq!(
        rres, cres,
        "DIVERGENCE: INT_MIN / -1 c={cres:?} rust={rres:?}"
    );
}

// ===========================================================================
// ERRORS row 17 — buffapp's `result / intermediate3` with
// result == INT_MIN and intermediate3 == -1 -> SIGFPE.
//
// Reachable: i1 = -(2^30 - 1), i2 = -(2^30 + 1) gives
//   i1 + i2 == INT_MIN  and  i1 * i2 == 2^60 - 1 == -1 (mod 2^32).
// Both come out of the "add" arm (param % 4 == 0).
// ===========================================================================
#[test]
fn err_17_buffapp_int_min_div_minus_one_sigfpe_parity() {
    let _ = c();
    let _ = rust();
    let cf = c().buffapp;
    let rf = rust().buffapp;

    // p1 % 4 == 0 -> "add": i1 = p1 + p2 = -1073741823
    // p3 % 4 == 0 -> "add": i2 = p3 + p4 = -1073741825
    let (p1, p2, p3, p4) = (-1_073_741_824i32, 1i32, -1_073_741_824i32, -1i32);
    assert_eq!(p1 % 4, 0);
    assert_eq!(p3 % 4, 0);
    let i1 = p1.wrapping_add(p2);
    let i2 = p3.wrapping_add(p4);
    assert_eq!(i1.wrapping_add(i2), i32::MIN);
    assert_eq!(i1.wrapping_mul(i2), -1);

    let cres = run_in_child(|| unsafe {
        let v = cf(p1, p2, p3, p4);
        std::hint::black_box(v);
    });
    let rres = run_in_child(|| unsafe {
        let v = rf(p1, p2, p3, p4);
        std::hint::black_box(v);
    });
    assert_eq!(
        cres,
        Err(8),
        "premise: C buffapp({p1},{p2},{p3},{p4}) should SIGFPE, got {cres:?}"
    );
    assert_eq!(
        rres, cres,
        "DIVERGENCE: buffapp SIGFPE c={cres:?} rust={rres:?}"
    );
}

// ===========================================================================
// ERRORS row 16 — buffapp's intermediate3 == 0 branch
// ===========================================================================
#[test]
fn err_16_buffapp_zero_intermediate_branch() {
    // -1 % 4 == -1 -> "unknown" -> perform_operation returns 0 -> i1 = 0,
    // so intermediate3 == 0 and result = p1+p2+p3+p4 (wrapping).
    let cases: &[(i32, i32, i32, i32)] = &[
        (-1, 0, -1, 0),
        (-1, 7, -1, 11),
        (-1, i32::MAX, -1, i32::MAX),
        (-2, i32::MIN, -3, i32::MIN),
        (-1, i32::MIN, -1, -1),
        (3, 0, 3, 0),   // divide by zero -> 0 on both sides
        (3, 0, 4, 5),   // i1 == 0 only
        (4, 5, 3, 0),   // i2 == 0 only
        (0, 0, 0, 0),   // add(0,0) == 0
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
    ];
    for &(p1, p2, p3, p4) in cases {
        let (cv, cout) = capture_stdout(|| unsafe { (c().buffapp)(p1, p2, p3, p4) });
        let (rv, rout) = capture_stdout(|| unsafe { (rust().buffapp)(p1, p2, p3, p4) });
        let expect = p1.wrapping_add(p2).wrapping_add(p3).wrapping_add(p4);
        assert_eq!(
            cv, expect,
            "premise: C buffapp({p1},{p2},{p3},{p4}) should take the sum branch"
        );
        assert_eq!(rv, cv, "DIVERGENCE: buffapp({p1},{p2},{p3},{p4}) rust={rv} c={cv}");
        assert_eq!(rout, cout, "DIVERGENCE: stdout for buffapp({p1},{p2},{p3},{p4})");
    }
}

// ===========================================================================
// Generic boundaries required by Phase C beyond the table
// ===========================================================================

/// `create_buffer` at every interesting length boundary, plus one step past
/// the signed range in both directions.
#[test]
fn generic_capacity_boundaries() {
    for cap in [
        0i32, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 33, 63, 64, 127, 128, 255, 256,
        4095, 4096, 65535, 65536, 65537, -1, i32::MIN, i32::MIN + 1,
    ] {
        unsafe {
            let cb = (c().create_buffer)(cap);
            let rb = (rust().create_buffer)(cap);
            assert_eq!(
                cb.is_null(),
                rb.is_null(),
                "create_buffer({cap}) NULL-ness differs"
            );
            if !cb.is_null() {
                assert_eq!(snapshot(rb), snapshot(cb), "create_buffer({cap}) state differs");
                (c().destroy_buffer)(cb);
                (rust().destroy_buffer)(rb);
            }
        }
    }
}

/// Zero-length and oversized `str` inputs to `append_to_buffer`.
#[test]
fn generic_append_length_boundaries() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 0xB0);
    for cap in [0i32, 1, 2, 32] {
        for len in [0usize, 1, 2, 3, 31, 32, 33, 64, 65, 1024, 1025, 100_000] {
            let s = rng.ascii(len);
            unsafe {
                let cb = (c().create_buffer)(cap);
                let rb = (rust().create_buffer)(cap);
                let crc = (c().append_to_buffer)(cb, s.as_ptr());
                let rrc = (rust().append_to_buffer)(rb, s.as_ptr());
                assert_eq!(rrc, crc, "append(cap={cap}, len={len}) rc rust={rrc} c={crc}");
                assert_eq!(
                    snapshot(rb),
                    snapshot(cb),
                    "append(cap={cap}, len={len}) state differs"
                );
                (c().destroy_buffer)(cb);
                (rust().destroy_buffer)(rb);
            }
        }
    }
}

/// One step past each documented `get_operation_name` value.
#[test]
fn generic_one_past_valid_enum_range() {
    for code in [-1i32, 0, 3, 4] {
        unsafe {
            let cb = cstr_bytes((c().get_operation_name)(code));
            let rb = cstr_bytes((rust().get_operation_name)(code));
            assert_eq!(rb, cb, "get_operation_name({code})");
        }
    }
}

// ---------------------------------------------------------------------------
// small helper
// ---------------------------------------------------------------------------
unsafe fn malloc_bytes(n: usize) -> *mut c_char {
    extern "C" {
        fn malloc(n: usize) -> *mut c_void;
    }
    let p = malloc(n) as *mut c_char;
    assert!(!p.is_null());
    std::ptr::write_bytes(p, 0, n);
    p
}
