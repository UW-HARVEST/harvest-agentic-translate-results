// Phase C — error-path differential tests. One test per ERRORS.md row.
// Run with `--test-threads=1` (the crash rows fork children).

mod common;

use common::*;
use std::ffi::{c_char, c_int, CStr, CString};

const SIGSEGV: c_int = 11;
const SIGFPE: c_int = 8;

fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn name_of(p: *const c_char) -> Vec<u8> {
    assert!(!p.is_null());
    unsafe { CStr::from_ptr(p).to_bytes().to_vec() }
}

// --- E1 -------------------------------------------------------------------
#[test]
fn e1_create_buffer_negative_capacity() {
    let p = pair();
    for cap in [-1i32, -2, -7, -1024, -1_000_000_000] {
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            assert!(bc.is_null(), "C create_buffer({cap}) should be NULL");
            assert_eq!(
                bc.is_null(),
                br.is_null(),
                "create_buffer({cap}): C null={} RUST null={}",
                bc.is_null(),
                br.is_null()
            );
        }
    }
}

// --- E2 -------------------------------------------------------------------
#[test]
fn e2_create_buffer_int_min() {
    let p = pair();
    for cap in [i32::MIN, i32::MIN + 1] {
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            assert!(bc.is_null(), "C create_buffer({cap}) should be NULL");
            assert_eq!(bc.is_null(), br.is_null(), "create_buffer({cap})");
        }
    }
}

// --- E3 -------------------------------------------------------------------
#[test]
fn e3_create_buffer_int_max() {
    let p = pair();
    for cap in [i32::MAX, i32::MAX - 1, 1 << 30] {
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            assert_eq!(
                bc.is_null(),
                br.is_null(),
                "create_buffer({cap}): C null={} RUST null={}",
                bc.is_null(),
                br.is_null()
            );
            if !bc.is_null() {
                assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "create_buffer({cap})");
            }
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- E4 -------------------------------------------------------------------
#[test]
fn e4_create_buffer_zero_capacity() {
    let p = pair();
    unsafe {
        let bc = (p.c.create_buffer)(0);
        let br = (p.rs.create_buffer)(0);
        assert!(!bc.is_null(), "malloc(0) is non-NULL on glibc");
        assert_eq!(bc.is_null(), br.is_null());
        let sc = p.c.snapshot(bc);
        assert_eq!(sc, p.rs.snapshot(br));
        assert_eq!(sc.capacity, 0);
        assert_eq!(sc.length, 0);
        assert_eq!(sc.bytes, Vec::<u8>::new());
        (p.c.destroy_buffer)(bc);
        (p.rs.destroy_buffer)(br);
    }
}

// --- E5 (documented as unreachable) ---------------------------------------
#[test]
fn e5_struct_malloc_failure_unreachable() {
    // `malloc(sizeof(StringBuffer))` (16 bytes) cannot be made to fail through
    // the public API, so `lib.c:37` is unreachable from a differential test.
    // Recorded here so the row is not silently dropped; the Rust translation
    // contains the identical `if buffer.is_null() { return NULL }` guard.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .unwrap();
    assert!(
        src.contains("if buffer.is_null()") && src.contains("return core::ptr::null_mut()"),
        "Rust create_buffer must keep the struct-malloc NULL guard"
    );
}

// --- E6 -------------------------------------------------------------------
/// `new_capacity = required * 2` overflows `int` to a negative value, which
/// sign-extends to a huge `size_t`, so `realloc` fails -> `-1` (`lib.c:62`).
#[test]
fn e6_append_realloc_failure() {
    let p = pair();
    let s = cs("abcde");
    unsafe {
        let bc = (p.c.create_buffer)(32);
        let br = (p.rs.create_buffer)(32);
        (*bc).length = 2_000_000_000;
        (*br).length = 2_000_000_000;

        let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
        let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
        assert_eq!(rc, -1, "C append_to_buffer must return -1");
        assert_eq!(rc, rr, "append_to_buffer realloc failure: C={rc} RUST={rr}");
        // Buffer must be left untouched.
        assert_eq!((*bc).capacity, (*br).capacity);
        assert_eq!((*bc).capacity, 32, "capacity must not change on failure");
        assert_eq!((*bc).length, (*br).length);
        assert_eq!((*bc).length, 2_000_000_000);

        (*bc).length = 0;
        (*br).length = 0;
        (p.c.destroy_buffer)(bc);
        (p.rs.destroy_buffer)(br);
    }
}

// --- E7 -------------------------------------------------------------------
#[test]
fn e7_append_realloc_failure_randomized() {
    let p = pair();
    let mut rng = Rng::new(0xE7_5EED);
    for _ in 0..300 {
        // Any length whose `(length + str_len + 1) * 2` overflows int.
        let length = (i32::MAX / 2) + 1 + (rng.range(0, 1_000_000_000) as i32).abs() % (i32::MAX / 2);
        let slen = rng.range(0, 16) as usize;
        let s = cs(&"q".repeat(slen));
        unsafe {
            let bc = (p.c.create_buffer)(16);
            let br = (p.rs.create_buffer)(16);
            (*bc).length = length;
            (*br).length = length;
            let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
            let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
            assert_eq!(rc, rr, "length={length} slen={slen}: C={rc} RUST={rr}");
            assert_eq!(rc, -1, "length={length} slen={slen} should fail");
            assert_eq!((*bc).capacity, (*br).capacity);
            assert_eq!((*bc).length, (*br).length);
            (*bc).length = 0;
            (*br).length = 0;
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- E8 -------------------------------------------------------------------
#[test]
fn e8_append_empty_string_no_growth() {
    let p = pair();
    let s = cs("");
    for cap in [1i32, 2, 8, 32, 4096] {
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
            let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
            assert_eq!((rc, rr), (0, 0), "cap={cap}");
            let sc = p.c.snapshot(bc);
            assert_eq!(sc, p.rs.snapshot(br), "cap={cap}");
            assert_eq!(sc.capacity, cap, "cap={cap}: growth branch not taken");
            assert_eq!(sc.length, 0);
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- E9 / E10 -------------------------------------------------------------
#[test]
fn e9_e10_growth_boundary_exact() {
    let p = pair();
    let mut rng = Rng::new(0xE910_5EED);
    for _ in 0..400 {
        let cap = rng.range(1, 512) as i32;
        for (delta, expect_growth) in [(0i32, false), (1, true)] {
            // required = str_len + 1; pick str_len so required == cap + delta
            let str_len = (cap + delta - 1) as usize;
            let s = cs(&"w".repeat(str_len));
            unsafe {
                let bc = (p.c.create_buffer)(cap);
                let br = (p.rs.create_buffer)(cap);
                let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
                let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
                assert_eq!((rc, rr), (0, 0), "cap={cap} delta={delta}");
                let sc = p.c.snapshot(bc);
                assert_eq!(sc, p.rs.snapshot(br), "cap={cap} delta={delta}");
                if expect_growth {
                    assert_eq!(sc.capacity, (cap + 1) * 2, "cap={cap}: expected growth");
                } else {
                    assert_eq!(sc.capacity, cap, "cap={cap}: expected no growth");
                }
                (p.c.destroy_buffer)(bc);
                (p.rs.destroy_buffer)(br);
            }
        }
    }
}

// --- E11 ------------------------------------------------------------------
#[test]
fn e11_append_null_buffer_crashes_identically() {
    let p = pair();
    let oc = run_isolated(|| unsafe {
        let s = cs("x");
        Some((p.c.append_to_buffer)(std::ptr::null_mut(), s.as_ptr()))
    });
    let or = run_isolated(|| unsafe {
        let s = cs("x");
        Some((p.rs.append_to_buffer)(std::ptr::null_mut(), s.as_ptr()))
    });
    assert_eq!(oc.signal, Some(SIGSEGV), "C should SIGSEGV, got {oc:?}");
    assert_eq!(oc.signal, or.signal, "C={:?} RUST={:?}", oc.signal, or.signal);
    assert_eq!(oc.exit_code, or.exit_code);
    assert_eq!(oc.ret, or.ret);
    assert_eq!(oc.stdout, or.stdout);
}

// --- E12 ------------------------------------------------------------------
#[test]
fn e12_append_null_string_crashes_identically() {
    let p = pair();
    let oc = run_isolated(|| unsafe {
        let b = (p.c.create_buffer)(32);
        Some((p.c.append_to_buffer)(b, std::ptr::null()))
    });
    let or = run_isolated(|| unsafe {
        let b = (p.rs.create_buffer)(32);
        Some((p.rs.append_to_buffer)(b, std::ptr::null()))
    });
    assert_eq!(oc.signal, Some(SIGSEGV), "C should SIGSEGV, got {oc:?}");
    assert_eq!(oc.signal, or.signal, "C={:?} RUST={:?}", oc.signal, or.signal);
    assert_eq!(oc.exit_code, or.exit_code);
    assert_eq!(oc.ret, or.ret);
}

// --- E13 ------------------------------------------------------------------
#[test]
fn e13_destroy_null_is_noop() {
    let p = pair();
    let oc = isolated_void(|| unsafe { (p.c.destroy_buffer)(std::ptr::null_mut()) });
    let or = isolated_void(|| unsafe { (p.rs.destroy_buffer)(std::ptr::null_mut()) });
    assert_eq!(oc.exit_code, Some(0), "C destroy_buffer(NULL) must be a no-op: {oc:?}");
    assert_eq!((oc.exit_code, oc.signal, oc.stdout.clone()), (or.exit_code, or.signal, or.stdout.clone()));
    // Also repeatedly, in-process.
    unsafe {
        for _ in 0..100 {
            (p.c.destroy_buffer)(std::ptr::null_mut());
            (p.rs.destroy_buffer)(std::ptr::null_mut());
        }
    }
}

// --- E14 ------------------------------------------------------------------
#[test]
fn e14_destroy_with_null_data() {
    let p = pair();
    let oc = isolated_void(|| unsafe {
        let b = (p.c.create_buffer)(32);
        (*b).data = std::ptr::null_mut(); // leaks the payload, exactly as C would
        (p.c.destroy_buffer)(b);
    });
    let or = isolated_void(|| unsafe {
        let b = (p.rs.create_buffer)(32);
        (*b).data = std::ptr::null_mut();
        (p.rs.destroy_buffer)(b);
    });
    assert_eq!(oc.exit_code, Some(0), "C path must not crash: {oc:?}");
    assert_eq!((oc.exit_code, oc.signal), (or.exit_code, or.signal));
    assert_eq!(oc.stdout, or.stdout);
}

// --- E15 / E16 ------------------------------------------------------------
#[test]
fn e15_e16_get_operation_name_one_past_range() {
    let p = pair();
    for op in [4i32, -1] {
        unsafe {
            let nc = name_of((p.c.get_operation_name)(op));
            let nr = name_of((p.rs.get_operation_name)(op));
            assert_eq!(nc, nr, "get_operation_name({op})");
            assert_eq!(nc, b"unknown".to_vec(), "get_operation_name({op})");
        }
    }
}

// --- E17 ------------------------------------------------------------------
/// Out-of-range "enum" ints crossing the FFI boundary: a C enum accepts any int.
#[test]
fn e17_get_operation_name_out_of_range_enum_values() {
    let p = pair();
    let mut cases: Vec<i32> = vec![
        -3, -2, -1, 4, 5, 6, 42, 255, 256, 65536, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
    ];
    let mut rng = Rng::new(0xE17_5EED);
    for _ in 0..5000 {
        let v = rng.next_i32();
        if !(0..4).contains(&v) {
            cases.push(v);
        }
    }
    for op in cases {
        unsafe {
            let nc = name_of((p.c.get_operation_name)(op));
            let nr = name_of((p.rs.get_operation_name)(op));
            assert_eq!(nc, nr, "get_operation_name({op})");
            assert_eq!(nc, b"unknown".to_vec(), "get_operation_name({op})");
        }
    }
}

// --- E18 ------------------------------------------------------------------
#[test]
fn e18_perform_operation_unrecognised_strings() {
    let p = pair();
    let mut rng = Rng::new(0xE18_5EED);
    let mut names: Vec<String> = vec![
        "".into(),
        " ".into(),
        "ADD".into(),
        "Add".into(),
        "add ".into(),
        " add".into(),
        "ad".into(),
        "addd".into(),
        "sub".into(),
        "subtract1".into(),
        "multipl".into(),
        "divid".into(),
        "divide ".into(),
        "unknown".into(),
        "\u{1}\u{2}\u{3}".into(),
        "\u{7f}".into(),
    ];
    // Random byte strings (excluding the four recognised names).
    for _ in 0..500 {
        let len = rng.range(1, 12) as usize;
        let s: String = (0..len).map(|_| char::from(rng.range(1, 127) as u8)).collect();
        if !matches!(s.as_str(), "add" | "subtract" | "multiply" | "divide") {
            names.push(s);
        }
    }
    for name in names {
        let c = cs(&name);
        for _ in 0..8 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            unsafe {
                let rc = (p.c.perform_operation)(a, b, c.as_ptr());
                let rr = (p.rs.perform_operation)(a, b, c.as_ptr());
                assert_eq!(rc, rr, "perform_operation({a}, {b}, {name:?})");
                assert_eq!(rc, 0, "perform_operation({a}, {b}, {name:?}) must be 0");
            }
        }
    }
}

// --- E19 ------------------------------------------------------------------
#[test]
fn e19_divide_by_zero_returns_zero() {
    let p = pair();
    let op = cs("divide");
    let mut rng = Rng::new(0xE19_5EED);
    let mut vals: Vec<i32> = vec![0, 1, -1, 2, -2, i32::MAX, i32::MIN];
    for _ in 0..2000 {
        vals.push(rng.next_i32());
    }
    for a in vals {
        unsafe {
            let rc = (p.c.perform_operation)(a, 0, op.as_ptr());
            let rr = (p.rs.perform_operation)(a, 0, op.as_ptr());
            assert_eq!(rc, rr, "divide({a}, 0)");
            assert_eq!(rc, 0);
        }
    }
}

// --- E20 ------------------------------------------------------------------
/// `INT_MIN / -1` is signed-overflow UB; gcc emits a bare `idiv`, which traps.
/// The Rust translation must trap the same way.
#[test]
fn e20_divide_int_min_by_minus_one() {
    let p = pair();
    let oc = run_isolated(|| unsafe {
        let op = cs("divide");
        Some((p.c.perform_operation)(i32::MIN, -1, op.as_ptr()))
    });
    let or = run_isolated(|| unsafe {
        let op = cs("divide");
        Some((p.rs.perform_operation)(i32::MIN, -1, op.as_ptr()))
    });
    assert_eq!(
        (oc.signal, oc.exit_code, oc.ret),
        (or.signal, or.exit_code, or.ret),
        "INT_MIN / -1: C={oc:?} RUST={or:?}"
    );
    // Whatever the C does (trap or a value), the Rust must match; on x86-64
    // gcc -O0 this is SIGFPE.
    if oc.signal.is_some() {
        assert_eq!(oc.signal, Some(SIGFPE), "expected SIGFPE, got {oc:?}");
    }
}

// --- E21 ------------------------------------------------------------------
#[test]
fn e21_perform_operation_null_operation_crashes_identically() {
    let p = pair();
    let oc = run_isolated(|| unsafe { Some((p.c.perform_operation)(3, 4, std::ptr::null())) });
    let or = run_isolated(|| unsafe { Some((p.rs.perform_operation)(3, 4, std::ptr::null())) });
    assert_eq!(oc.signal, Some(SIGSEGV), "C should SIGSEGV, got {oc:?}");
    assert_eq!(
        (oc.signal, oc.exit_code, oc.ret),
        (or.signal, or.exit_code, or.ret),
        "perform_operation(3, 4, NULL): C={oc:?} RUST={or:?}"
    );
}

// --- E22 ------------------------------------------------------------------
#[test]
fn e22_signed_overflow_wraps_identically() {
    let p = pair();
    let cases: &[(&str, i32, i32)] = &[
        ("add", i32::MAX, 1),
        ("add", i32::MAX, i32::MAX),
        ("add", i32::MIN, -1),
        ("add", i32::MIN, i32::MIN),
        ("subtract", i32::MIN, 1),
        ("subtract", i32::MAX, -1),
        ("subtract", i32::MIN, i32::MAX),
        ("multiply", i32::MIN, -1),
        ("multiply", i32::MAX, 2),
        ("multiply", i32::MIN, 2),
        ("multiply", 65536, 65536),
        ("multiply", i32::MAX, i32::MAX),
        ("multiply", i32::MIN, i32::MIN),
    ];
    for &(op, a, b) in cases {
        let c = cs(op);
        unsafe {
            let rc = (p.c.perform_operation)(a, b, c.as_ptr());
            let rr = (p.rs.perform_operation)(a, b, c.as_ptr());
            assert_eq!(rc, rr, "{op}({a}, {b}): C={rc} RUST={rr}");
        }
    }
    // Randomized overflow sweep.
    let mut rng = Rng::new(0xE22_5EED);
    for op in ["add", "subtract", "multiply"] {
        let c = cs(op);
        for _ in 0..3000 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            unsafe {
                let rc = (p.c.perform_operation)(a, b, c.as_ptr());
                let rr = (p.rs.perform_operation)(a, b, c.as_ptr());
                assert_eq!(rc, rr, "{op}({a}, {b})");
            }
        }
    }
}

// --- E23 ------------------------------------------------------------------
#[test]
fn e23_buffapp_zero_intermediate3_fallback() {
    // intermediate3 == 0 -> `result = param1 + param2 + param3 + param4`.
    for (a, b, c, d) in [
        (4i32, -4i32, 5i32, 3i32),   // op1 = add, sums to 0
        (8, -8, 9, 1),               // ditto
        (-5, 3, 6, 2),               // op1 = unknown (negative remainder) -> 0
        (4, 6, 7, 0),                // op2 = divide by 0 -> 0
        (0, 0, 0, 0),                // both 0
        (5, 5, 4, -4),               // op2 = add, sums to 0
        (6, 0, 2, 0),                // op1 = multiply by 0
        (-1, 12, -2, 34),            // both operations unknown
    ] {
        diff_buffapp(a, b, c, d);
    }
}

// --- E24 ------------------------------------------------------------------
/// `result / intermediate3` with `result == INT_MIN` and `intermediate3 == -1`.
/// Reachable because the multiply wraps: 1073741823 * 1073741825 == -1 (mod 2^32)
/// while 1073741823 + 1073741825 == INT_MIN (mod 2^32).
#[test]
fn e24_buffapp_division_overflow() {
    let p = pair();
    let (a, b, c, d) = (0i32, 1_073_741_823i32, 0i32, 1_073_741_825i32);
    let oc = isolated_int(|| unsafe { (p.c.buffapp)(a, b, c, d) });
    let or = isolated_int(|| unsafe { (p.rs.buffapp)(a, b, c, d) });
    assert_eq!(
        (oc.signal, oc.exit_code, oc.ret),
        (or.signal, or.exit_code, or.ret),
        "buffapp({a},{b},{c},{d}): C={oc:?} RUST={or:?}"
    );
    assert_eq!(oc.stdout, or.stdout, "buffapp({a},{b},{c},{d}) stdout differs");
    if oc.signal.is_some() {
        assert_eq!(oc.signal, Some(SIGFPE), "expected SIGFPE, got {oc:?}");
    }
    // The mirrored operand order, and the negative solutions.
    for (a, b, c, d) in [
        (0i32, 1_073_741_825i32, 0i32, 1_073_741_823i32),
        (0, -1_073_741_825, 0, -1_073_741_823),
        (0, -1_073_741_823, 0, -1_073_741_825),
    ] {
        let oc = isolated_int(|| unsafe { (p.c.buffapp)(a, b, c, d) });
        let or = isolated_int(|| unsafe { (p.rs.buffapp)(a, b, c, d) });
        assert_eq!(
            (oc.signal, oc.exit_code, oc.ret, oc.stdout.clone()),
            (or.signal, or.exit_code, or.ret, or.stdout.clone()),
            "buffapp({a},{b},{c},{d}): C={:?} RUST={:?}",
            (oc.signal, oc.exit_code, oc.ret),
            (or.signal, or.exit_code, or.ret)
        );
    }
}

// --- E25 ------------------------------------------------------------------
#[test]
fn e25_buffapp_negative_remainder_unknown_ops() {
    let p = pair();
    let mut rng = Rng::new(0xE25_5EED);
    // Every negative remainder for param1 and param3.
    for r1 in [-1i32, -2, -3] {
        for r3 in [-1i32, -2, -3] {
            for _ in 0..12 {
                let k1 = rng.range(1, 1000) as i32;
                let k3 = rng.range(1, 1000) as i32;
                let a = -4 * k1 + r1;
                let c = -4 * k3 + r3;
                let b = rng.interesting_i32();
                let d = rng.interesting_i32();
                assert_eq!(a.wrapping_rem(4), r1);
                assert_eq!(c.wrapping_rem(4), r3);
                // Both operations resolve to "unknown" -> both intermediates 0
                // -> intermediate3 == 0 -> the sum fallback.
                let oc = isolated_int(|| unsafe { (p.c.buffapp)(a, b, c, d) });
                let or = isolated_int(|| unsafe { (p.rs.buffapp)(a, b, c, d) });
                assert_eq!(oc.ret, or.ret, "buffapp({a},{b},{c},{d})");
                assert_eq!(oc.stdout, or.stdout, "buffapp({a},{b},{c},{d}) stdout");
                assert_eq!((oc.signal, oc.exit_code), (or.signal, or.exit_code));
                let out = String::from_utf8_lossy(&oc.stdout);
                assert!(out.contains("Operation 1: unknown"), "{out}");
                assert!(out.contains("Operation 2: unknown"), "{out}");
                assert_eq!(
                    oc.ret,
                    Some(a.wrapping_add(b).wrapping_add(c).wrapping_add(d)),
                    "fallback sum expected"
                );
            }
        }
    }
}

// --- E26 (documented as unreachable) --------------------------------------
#[test]
fn e26_buffapp_null_log_buffer_unreachable() {
    // `buffapp` always calls `create_buffer(32)`, which cannot fail, so the
    // unchecked dereference at `lib.c:116` is unreachable. Assert instead that
    // the Rust keeps the same unchecked dereference (no added NULL guard, which
    // would change behaviour if create_buffer ever did fail).
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .unwrap();
    let buffapp = src.split("pub unsafe extern \"C\" fn buffapp").nth(1).unwrap();
    let body = &buffapp[..buffapp.find("sprintf(temp_ptr, FMT_START").unwrap()];
    assert!(
        body.contains("(*log_buffer).length = 0"),
        "Rust buffapp must dereference log_buffer unconditionally, like the C"
    );
    assert!(
        !body.contains("log_buffer.is_null()"),
        "Rust buffapp must NOT add a NULL check the C does not have"
    );
    // And the happy path still works.
    let p = pair();
    let oc = isolated_int(|| unsafe { (p.c.buffapp)(4, 5, 6, 7) });
    let or = isolated_int(|| unsafe { (p.rs.buffapp)(4, 5, 6, 7) });
    assert_eq!(oc.ret, or.ret);
    assert_eq!(oc.stdout, or.stdout);
}

// --- generic boundary sweep, beyond the table -----------------------------
#[test]
fn generic_null_pointer_matrix() {
    let p = pair();
    // Both NULL.
    let oc = run_isolated(|| unsafe {
        Some((p.c.append_to_buffer)(std::ptr::null_mut(), std::ptr::null()))
    });
    let or = run_isolated(|| unsafe {
        Some((p.rs.append_to_buffer)(std::ptr::null_mut(), std::ptr::null()))
    });
    assert_eq!(
        (oc.signal, oc.exit_code, oc.ret),
        (or.signal, or.exit_code, or.ret),
        "append_to_buffer(NULL, NULL): C={oc:?} RUST={or:?}"
    );
}

#[test]
fn generic_zero_and_oversized_lengths() {
    let p = pair();
    // capacity 0 and negative capacities already covered; sweep the transition.
    for cap in [-2i32, -1, 0, 1, 2] {
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            assert_eq!(bc.is_null(), br.is_null(), "create_buffer({cap})");
            if !bc.is_null() {
                assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "create_buffer({cap})");
                (p.c.destroy_buffer)(bc);
                (p.rs.destroy_buffer)(br);
            }
        }
    }
    // Oversized single append: a string longer than any plausible capacity.
    let big = cs(&"m".repeat(200_000));
    unsafe {
        let bc = (p.c.create_buffer)(1);
        let br = (p.rs.create_buffer)(1);
        let rc = (p.c.append_to_buffer)(bc, big.as_ptr());
        let rr = (p.rs.append_to_buffer)(br, big.as_ptr());
        assert_eq!((rc, rr), (0, 0));
        assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br));
        (p.c.destroy_buffer)(bc);
        (p.rs.destroy_buffer)(br);
    }
}

#[test]
fn generic_one_past_valid_op_range_through_perform() {
    let p = pair();
    // `get_operation_name(op)` for op == 3 and op == 4 feeding perform_operation:
    // 3 -> "divide" (guarded), 4 -> "unknown" (returns 0).
    for op in [3i32, 4, -1, i32::MIN, i32::MAX] {
        unsafe {
            let nc = (p.c.get_operation_name)(op);
            let nr = (p.rs.get_operation_name)(op);
            for (a, b) in [(10i32, 3i32), (10, 0), (-10, 3), (0, 0), (i32::MAX, i32::MIN)] {
                let rc = (p.c.perform_operation)(a, b, nc);
                let rr = (p.rs.perform_operation)(a, b, nr);
                assert_eq!(rc, rr, "op={op} a={a} b={b}");
            }
        }
    }
}
