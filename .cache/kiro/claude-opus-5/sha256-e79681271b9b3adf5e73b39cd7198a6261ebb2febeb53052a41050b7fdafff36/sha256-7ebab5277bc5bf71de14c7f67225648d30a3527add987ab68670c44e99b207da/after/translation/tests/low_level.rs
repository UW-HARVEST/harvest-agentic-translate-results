//! Differential tests for the leaf functions of the library, in call-hierarchy
//! order: `get_operation_name`, `create_buffer`, `append_to_buffer`,
//! `destroy_buffer`, then `perform_operation`.

mod common;

use common::{buffer_bytes, cstr_bytes, load_pair, INTERESTING};
use std::ffi::{c_char, c_int};

#[test]
fn get_operation_name_matches() {
    let p = load_pair();

    let mut codes: Vec<c_int> = (-16..=16).collect();
    codes.extend_from_slice(INTERESTING);

    for &code in &codes {
        // SAFETY: both symbols return a pointer to a static NUL-terminated
        // string for every input.
        unsafe {
            let c = cstr_bytes((p.c.get_operation_name)(code));
            let r = cstr_bytes((p.rs.get_operation_name)(code));
            assert_eq!(
                c, r,
                "get_operation_name({code}): C={:?} Rust={:?}",
                String::from_utf8_lossy(&c),
                String::from_utf8_lossy(&r)
            );
        }
    }
}

#[test]
fn get_operation_name_pointers_are_stable() {
    // The C version returns pointers into .rodata, so repeated calls with the
    // same op code yield the same address. The Rust version must behave the
    // same way, otherwise a caller that caches the pointer would break.
    let p = load_pair();
    for code in [0, 1, 2, 3, 4, -1] {
        unsafe {
            let a = (p.rs.get_operation_name)(code);
            let b = (p.rs.get_operation_name)(code);
            assert_eq!(a, b, "Rust get_operation_name({code}) returned two addresses");

            let ca = (p.c.get_operation_name)(code);
            let cb = (p.c.get_operation_name)(code);
            assert_eq!(ca, cb);
        }
    }
}

#[test]
fn create_buffer_matches() {
    let p = load_pair();

    // Positive capacities that malloc always satisfies.
    for cap in [1, 2, 4, 8, 16, 31, 32, 33, 64, 100, 4096, 65536] {
        unsafe {
            let cb = (p.c.create_buffer)(cap);
            let rb = (p.rs.create_buffer)(cap);
            assert!(!cb.is_null() && !rb.is_null(), "create_buffer({cap}) failed");

            assert_eq!((*cb).capacity, (*rb).capacity, "capacity for {cap}");
            assert_eq!((*cb).length, (*rb).length, "length for {cap}");
            assert_eq!(*(*cb).data, *(*rb).data, "data[0] for {cap}");
            assert_eq!(*(*cb).data, 0, "data[0] must be NUL for {cap}");

            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

#[test]
fn create_buffer_allocation_failure_matches() {
    let p = load_pair();
    // A negative capacity is converted to `size_t` by sign extension, so the
    // inner malloc is guaranteed to fail and both must return NULL.
    for cap in [-1, -2, -1024, c_int::MIN] {
        unsafe {
            let cb = (p.c.create_buffer)(cap);
            let rb = (p.rs.create_buffer)(cap);
            assert!(cb.is_null(), "C create_buffer({cap}) should be NULL");
            assert!(rb.is_null(), "Rust create_buffer({cap}) should be NULL");
        }
    }
}

/// Drive `append_to_buffer` with the same script of strings on both sides and
/// compare the full observable state after every step.
fn append_script(cap: c_int, strings: &[&str]) {
    let p = load_pair();
    unsafe {
        let cb = (p.c.create_buffer)(cap);
        let rb = (p.rs.create_buffer)(cap);
        assert!(!cb.is_null() && !rb.is_null());

        for (i, s) in strings.iter().enumerate() {
            let owned = std::ffi::CString::new(*s).unwrap();
            let ptr = owned.as_ptr() as *const c_char;

            let rc_c = (p.c.append_to_buffer)(cb, ptr);
            let rc_r = (p.rs.append_to_buffer)(rb, ptr);
            assert_eq!(rc_c, rc_r, "step {i} ({s:?}) return value, cap={cap}");

            assert_eq!(
                (*cb).length,
                (*rb).length,
                "step {i} ({s:?}) length, cap={cap}"
            );
            assert_eq!(
                (*cb).capacity,
                (*rb).capacity,
                "step {i} ({s:?}) capacity, cap={cap}"
            );

            let bc = buffer_bytes(cb);
            let br = buffer_bytes(rb);
            assert_eq!(
                bc,
                br,
                "step {i} ({s:?}) contents, cap={cap}: C={:?} Rust={:?}",
                String::from_utf8_lossy(&bc),
                String::from_utf8_lossy(&br)
            );
        }

        (p.c.destroy_buffer)(cb);
        (p.rs.destroy_buffer)(rb);
    }
}

#[test]
fn append_to_buffer_empty_string() {
    append_script(32, &["", "", ""]);
}

#[test]
fn append_to_buffer_no_growth_needed() {
    append_script(32, &["abc", "de", "f"]);
}

#[test]
fn append_to_buffer_triggers_growth() {
    // 38 bytes + NUL > 32, so the first append must realloc to 78.
    append_script(32, &["Starting computation with 4 parameters\n"]);
}

#[test]
fn append_to_buffer_exact_capacity_boundary() {
    // 31 payload bytes + NUL == 32 exactly: `required > capacity` is false, so
    // no realloc happens. One more byte must force one.
    append_script(32, &["0123456789012345678901234567890", "x"]);
}

#[test]
fn append_to_buffer_repeated_growth() {
    let long = "abcdefghijklmnopqrstuvwxyz0123456789";
    let script: Vec<&str> = vec![long; 24];
    append_script(1, &script);
    append_script(8, &script);
    append_script(32, &script);
}

#[test]
fn append_to_buffer_binary_ish_and_high_bytes() {
    // Bytes above 0x7F must be copied verbatim by both implementations.
    append_script(4, &["\u{1}\u{2}\u{7f}", "caf\u{e9}", "\u{fffd}"]);
}

#[test]
fn append_to_buffer_tiny_capacities() {
    for cap in [1, 2, 3, 4, 5, 7, 8, 16] {
        append_script(cap, &["a", "bb", "ccc", "dddd", "eeeeeeeeeeeeeeee"]);
    }
}

#[test]
fn destroy_buffer_accepts_null() {
    let p = load_pair();
    // The C guard is `if (buffer)`, so NULL is a no-op on both sides.
    unsafe {
        (p.c.destroy_buffer)(std::ptr::null_mut());
        (p.rs.destroy_buffer)(std::ptr::null_mut());
    }
}

#[test]
fn destroy_buffer_frees_cross_checked_buffers() {
    let p = load_pair();
    // Both libraries use the process allocator, so a buffer made by one can be
    // released by the other; this pins down that shared-ownership contract.
    unsafe {
        let cb = (p.c.create_buffer)(64);
        assert!(!cb.is_null());
        (p.rs.destroy_buffer)(cb);

        let rb = (p.rs.create_buffer)(64);
        assert!(!rb.is_null());
        (p.c.destroy_buffer)(rb);
    }
}

const OPERATIONS: &[&str] = &[
    "add",
    "subtract",
    "multiply",
    "divide",
    "unknown",
    "",
    "ADD",
    "add ",
    " add",
    "addx",
    "ad",
    "subtrac",
    "subtracts",
    "multiply\u{1}",
    "divide0",
    "div",
];

#[test]
fn perform_operation_matches() {
    let p = load_pair();

    for op in OPERATIONS {
        let owned = std::ffi::CString::new(*op).unwrap();
        let ptr = owned.as_ptr() as *const c_char;

        for &a in INTERESTING {
            for &b in INTERESTING {
                // `INT_MIN / -1` traps on x86 in the C build (signed overflow
                // in idiv), so that single pair is out of scope.
                if *op == "divide" && a == c_int::MIN && b == -1 {
                    continue;
                }
                unsafe {
                    let rc = (p.c.perform_operation)(a, b, ptr);
                    let rr = (p.rs.perform_operation)(a, b, ptr);
                    assert_eq!(rc, rr, "perform_operation({a}, {b}, {op:?})");
                }
            }
        }
    }
}

#[test]
fn perform_operation_uses_names_from_get_operation_name() {
    // Feed each implementation the pointer produced by the *other* one, so the
    // string constants themselves are compared through the real call path.
    let p = load_pair();
    for code in -8..=8 {
        unsafe {
            let c_name = (p.c.get_operation_name)(code);
            let r_name = (p.rs.get_operation_name)(code);
            for &(a, b) in &[(7, 3), (-7, 3), (7, -3), (0, 0), (1, 0), (i32::MAX, 2)] {
                let v1 = (p.c.perform_operation)(a, b, c_name);
                let v2 = (p.rs.perform_operation)(a, b, c_name);
                let v3 = (p.c.perform_operation)(a, b, r_name);
                let v4 = (p.rs.perform_operation)(a, b, r_name);
                assert_eq!(v1, v2, "code={code} a={a} b={b} (C name)");
                assert_eq!(v1, v3, "code={code} a={a} b={b} (name provenance)");
                assert_eq!(v1, v4, "code={code} a={a} b={b} (Rust name)");
            }
        }
    }
}

#[test]
fn perform_operation_division_truncates_toward_zero() {
    let p = load_pair();
    let owned = std::ffi::CString::new("divide").unwrap();
    let ptr = owned.as_ptr() as *const c_char;
    for &(a, b) in &[
        (7, 2),
        (-7, 2),
        (7, -2),
        (-7, -2),
        (1, 3),
        (-1, 3),
        (i32::MIN, 2),
        (i32::MIN, 3),
        (i32::MAX, -1),
        (5, 0),
        (-5, 0),
        (0, 0),
    ] {
        unsafe {
            assert_eq!(
                (p.c.perform_operation)(a, b, ptr),
                (p.rs.perform_operation)(a, b, ptr),
                "divide({a}, {b})"
            );
        }
    }
}

#[test]
fn create_buffer_zero_capacity_matches() {
    let p = load_pair();
    // malloc(0) returns a non-NULL minimal chunk, and the C code then writes
    // data[0] = '\0' with no capacity check. Both sides must agree.
    unsafe {
        let cb = (p.c.create_buffer)(0);
        let rb = (p.rs.create_buffer)(0);
        assert_eq!(cb.is_null(), rb.is_null(), "NULL-ness for capacity 0");
        if !cb.is_null() {
            assert_eq!((*cb).capacity, (*rb).capacity);
            assert_eq!((*cb).length, (*rb).length);
            assert_eq!(*(*cb).data, *(*rb).data);
            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

#[test]
fn perform_operation_short_and_prefix_strings() {
    let p = load_pair();
    // Single characters and proper prefixes of each operation name: these are
    // where a hand-rolled comparison could read past the NUL terminator or
    // accept a partial match that strcmp would reject.
    let probes = [
        "a", "ad", "s", "su", "sub", "subt", "m", "mu", "mul", "d", "di", "div", "divid", "u",
        "un", "unknow", "\0abc", "A", "z",
    ];
    for op in probes {
        let owned = std::ffi::CString::new(op.replace('\0', "")).unwrap();
        let ptr = owned.as_ptr() as *const c_char;
        for &(a, b) in &[(9, 4), (-9, 4), (0, 0), (i32::MAX, 2), (i32::MIN, 7)] {
            unsafe {
                assert_eq!(
                    (p.c.perform_operation)(a, b, ptr),
                    (p.rs.perform_operation)(a, b, ptr),
                    "perform_operation({a}, {b}, {op:?})"
                );
            }
        }
    }
}

#[test]
fn append_to_buffer_after_external_length_reset() {
    let p = load_pair();
    // `buffapp` pokes `log_buffer->length = 0` directly before appending, so
    // appending over an already-populated buffer is a real code path.
    unsafe {
        let cb = (p.c.create_buffer)(32);
        let rb = (p.rs.create_buffer)(32);

        let first = std::ffi::CString::new("hello world, this is a longer string").unwrap();
        (p.c.append_to_buffer)(cb, first.as_ptr());
        (p.rs.append_to_buffer)(rb, first.as_ptr());
        assert_eq!(buffer_bytes(cb), buffer_bytes(rb));
        assert_eq!((*cb).capacity, (*rb).capacity);

        // Reset the length the way buffapp does, then append again; the write
        // now lands at offset 0 of an already-grown allocation.
        (*cb).length = 0;
        (*rb).length = 0;

        let second = std::ffi::CString::new("short").unwrap();
        assert_eq!(
            (p.c.append_to_buffer)(cb, second.as_ptr()),
            (p.rs.append_to_buffer)(rb, second.as_ptr())
        );
        assert_eq!((*cb).length, (*rb).length);
        assert_eq!((*cb).capacity, (*rb).capacity);
        assert_eq!(buffer_bytes(cb), buffer_bytes(rb));

        (p.c.destroy_buffer)(cb);
        (p.rs.destroy_buffer)(rb);
    }
}

#[test]
fn string_buffer_layout_matches_c() {
    // The test mirror of StringBuffer is used to read fields out of buffers
    // allocated by the C library, so its layout must match C's exactly.
    assert_eq!(
        std::mem::size_of::<common::StringBuffer>(),
        std::mem::size_of::<*mut c_char>() + 2 * std::mem::size_of::<c_int>(),
        "unexpected StringBuffer size"
    );

    // Cross-read: fill the fields via the C library, then confirm the Rust
    // library agrees on where they live by mutating through the same mirror.
    let p = load_pair();
    unsafe {
        let cb = (p.c.create_buffer)(48);
        assert_eq!((*cb).capacity, 48, "capacity read at the wrong offset");
        assert_eq!((*cb).length, 0, "length read at the wrong offset");
        let s = std::ffi::CString::new("0123456789").unwrap();
        (p.rs.append_to_buffer)(cb, s.as_ptr());
        assert_eq!((*cb).length, 10, "Rust wrote length at the wrong offset");
        assert_eq!((*cb).capacity, 48, "capacity should not have changed");
        (p.c.destroy_buffer)(cb);
    }
}
