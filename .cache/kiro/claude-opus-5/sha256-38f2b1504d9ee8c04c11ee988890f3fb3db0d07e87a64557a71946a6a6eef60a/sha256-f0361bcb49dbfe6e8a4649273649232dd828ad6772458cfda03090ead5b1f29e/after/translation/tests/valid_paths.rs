// Phase B — valid-path differential tests.
//
// Every row of CONFIGS.md rows 1-26 and 34. Both implementations are driven
// exclusively through their `.so` exports.

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// CONFIGS rows 1-4: get_operation_name valid switch arms
// ---------------------------------------------------------------------------
#[test]
fn cfg_01_04_get_operation_name_valid_arms() {
    let expected: [(c_int, &[u8]); 4] = [
        (0, b"add"),
        (1, b"subtract"),
        (2, b"multiply"),
        (3, b"divide"),
    ];
    for (code, want) in expected {
        unsafe {
            let cb = cstr_bytes((c().get_operation_name)(code));
            let rb = cstr_bytes((rust().get_operation_name)(code));
            assert_eq!(cb, want, "C returned unexpected name for {code}");
            assert_eq!(
                rb, cb,
                "get_operation_name({code}): rust={:?} c={:?}",
                String::from_utf8_lossy(&rb),
                String::from_utf8_lossy(&cb)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 5-10: perform_operation, each valid arm, randomized operands
// ---------------------------------------------------------------------------

fn perform_diff(a: c_int, b: c_int, op: &CString) {
    unsafe {
        let cv = (c().perform_operation)(a, b, op.as_ptr());
        let rv = (rust().perform_operation)(a, b, op.as_ptr());
        assert_eq!(
            rv, cv,
            "perform_operation({a}, {b}, {:?}): rust={rv} c={cv}",
            op
        );
    }
}

#[test]
fn cfg_05_07_perform_operation_add_sub_mul_randomized() {
    let mut rng = Rng::default_seeded();
    let ops = [
        CString::new("add").unwrap(),
        CString::new("subtract").unwrap(),
        CString::new("multiply").unwrap(),
    ];
    // Boundary corners first, then randomized.
    let corners = [0i32, 1, -1, 2, -2, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for op in &ops {
        for &a in &corners {
            for &b in &corners {
                perform_diff(a, b, op);
            }
        }
        for _ in 0..3000 {
            perform_diff(rng.next_i32(), rng.next_i32(), op);
            perform_diff(rng.next_small_i32(), rng.next_small_i32(), op);
        }
    }
}

#[test]
fn cfg_08_10_perform_operation_divide_randomized() {
    let mut rng = Rng::default_seeded();
    let op = CString::new("divide").unwrap();

    // row 8: exact division
    for _ in 0..1000 {
        let b = loop {
            let v = rng.next_small_i32();
            if v != 0 {
                break v;
            }
        };
        let q = rng.next_small_i32();
        let a = q.wrapping_mul(b);
        if a == i32::MIN && b == -1 {
            continue; // ERRORS.md row 14 (SIGFPE) — covered out of process
        }
        perform_diff(a, b, &op);
    }

    // rows 9 + 10: truncating division, all sign combinations
    let corners = [1i32, -1, 2, -2, 3, -3, 7, -7, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for &a in &corners {
        for &b in &corners {
            if a == i32::MIN && b == -1 {
                continue;
            }
            perform_diff(a, b, &op);
        }
    }
    for _ in 0..4000 {
        let a = if rng.next_u64() & 1 == 0 {
            rng.next_i32()
        } else {
            rng.next_small_i32()
        };
        let b = loop {
            let v = if rng.next_u64() & 1 == 0 {
                rng.next_i32()
            } else {
                rng.next_small_i32()
            };
            if v != 0 {
                break v;
            }
        };
        if a == i32::MIN && b == -1 {
            continue;
        }
        perform_diff(a, b, &op);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 11: operation string is a pointer returned by the OTHER library
// ---------------------------------------------------------------------------
#[test]
fn cfg_11_perform_operation_with_cross_library_name_pointer() {
    let mut rng = Rng::default_seeded();
    for code in 0..4 {
        unsafe {
            let from_c = (c().get_operation_name)(code);
            let from_rust = (rust().get_operation_name)(code);
            for _ in 0..200 {
                let a = rng.next_small_i32();
                let mut b = rng.next_small_i32();
                if code == 3 && (b == 0 || (a == i32::MIN && b == -1)) {
                    b = 3;
                }
                // C name pointer -> both impls
                let c1 = (c().perform_operation)(a, b, from_c);
                let r1 = (rust().perform_operation)(a, b, from_c);
                // Rust name pointer -> both impls
                let c2 = (c().perform_operation)(a, b, from_rust);
                let r2 = (rust().perform_operation)(a, b, from_rust);
                assert_eq!(r1, c1, "code={code} a={a} b={b} (C name ptr)");
                assert_eq!(r2, c2, "code={code} a={a} b={b} (Rust name ptr)");
                assert_eq!(c1, c2, "code={code}: name pointers disagree in C");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 12-16: create_buffer / destroy_buffer
// ---------------------------------------------------------------------------
fn create_diff(cap: c_int) {
    unsafe {
        let cb = (c().create_buffer)(cap);
        let rb = (rust().create_buffer)(cap);
        let cs = snapshot(cb);
        let rs = snapshot(rb);
        assert_eq!(
            rs, cs,
            "create_buffer({cap}): rust={rs:?} c={cs:?}"
        );
        if !cb.is_null() {
            (c().destroy_buffer)(cb);
        }
        if !rb.is_null() {
            (rust().destroy_buffer)(rb);
        }
    }
}

#[test]
fn cfg_12_16_create_buffer_shapes() {
    create_diff(0); // row 12
    create_diff(1); // row 13
    create_diff(32); // row 14
    for cap in [2, 3, 4, 8, 16, 31, 33, 63, 64, 1023, 1024, 65535, 65536] {
        create_diff(cap);
    }
    let mut rng = Rng::default_seeded();
    for _ in 0..500 {
        create_diff(rng.range(1, 65536) as c_int); // row 15
    }
    // row 16: many create/destroy round trips, no crash or divergence
    for _ in 0..2000 {
        create_diff(rng.range(0, 4096) as c_int);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 17-23: append_to_buffer, one append, each grow/no-grow shape
// ---------------------------------------------------------------------------

/// Create a buffer in each library, append the same string, compare
/// return code + full observable state.
fn append_once_diff(cap: c_int, s: &CString, label: &str) {
    unsafe {
        let cb = (c().create_buffer)(cap);
        let rb = (rust().create_buffer)(cap);
        assert!(!cb.is_null() && !rb.is_null(), "{label}: create failed");
        let c_data_before = (*cb).data;
        let r_data_before = (*rb).data;

        let crc = (c().append_to_buffer)(cb, s.as_ptr());
        let rrc = (rust().append_to_buffer)(rb, s.as_ptr());
        assert_eq!(rrc, crc, "{label}: return code rust={rrc} c={crc}");

        let cs = snapshot(cb);
        let rs = snapshot(rb);
        assert_eq!(rs, cs, "{label}: state rust={rs:?} c={cs:?}");

        // Whether the data pointer was reallocated must match too.
        let c_moved = (*cb).data != c_data_before;
        let r_moved = (*rb).data != r_data_before;
        assert_eq!(
            r_moved, c_moved,
            "{label}: realloc-happened mismatch rust={r_moved} c={c_moved}"
        );

        (c().destroy_buffer)(cb);
        (rust().destroy_buffer)(rb);
    }
}

#[test]
fn cfg_17_23_append_single_shapes() {
    let mut rng = Rng::default_seeded();

    // row 17: no-grow, generous capacity
    for _ in 0..300 {
        let cap = rng.range(64, 256) as c_int;
        let len = rng.range(0, 32);
        append_once_diff(cap, &rng.ascii(len), "row17");
    }

    // row 18: required_capacity == capacity exactly (no grow)
    for cap in [1i32, 2, 4, 8, 32, 33, 100, 4096] {
        let s = rng.ascii((cap - 1) as usize);
        append_once_diff(cap, &s, "row18-exact-fit");
    }

    // row 19: required_capacity == capacity + 1 (grow)
    for cap in [1i32, 2, 4, 8, 32, 33, 100, 4096] {
        let s = rng.ascii(cap as usize);
        append_once_diff(cap, &s, "row19-one-over");
    }

    // row 20: grow from capacity == 0
    for len in [1usize, 2, 7, 64, 1000] {
        append_once_diff(0, &rng.ascii(len), "row20-grow-from-zero");
    }

    // row 21: empty string, no grow
    for cap in [1i32, 2, 32, 4096] {
        append_once_diff(cap, &CString::new("").unwrap(), "row21-empty");
    }

    // row 22: empty string on capacity 0 -> required 1 > 0 -> grows
    append_once_diff(0, &CString::new("").unwrap(), "row22-empty-on-zero-cap");

    // row 23: string much longer than capacity
    for cap in [0i32, 1, 4, 32] {
        for len in [128usize, 1000, 10_000] {
            append_once_diff(cap, &rng.ascii(len), "row23-long");
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 24-25: repeated appends (the composed low-level pipeline)
// ---------------------------------------------------------------------------
fn append_sequence_diff(initial_cap: c_int, strings: &[CString], label: &str) {
    unsafe {
        let cb = (c().create_buffer)(initial_cap);
        let rb = (rust().create_buffer)(initial_cap);
        assert!(!cb.is_null() && !rb.is_null(), "{label}: create failed");
        assert_eq!(snapshot(rb), snapshot(cb), "{label}: post-create divergence");

        for (i, s) in strings.iter().enumerate() {
            let crc = (c().append_to_buffer)(cb, s.as_ptr());
            let rrc = (rust().append_to_buffer)(rb, s.as_ptr());
            assert_eq!(rrc, crc, "{label}: step {i} rc rust={rrc} c={crc}");
            let cs = snapshot(cb);
            let rs = snapshot(rb);
            assert_eq!(rs, cs, "{label}: step {i} state rust={rs:?} c={cs:?}");
        }

        (c().destroy_buffer)(cb);
        (rust().destroy_buffer)(rb);
    }
}

#[test]
fn cfg_24_repeated_appends_randomized() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 0x24);
    for _ in 0..200 {
        let n = rng.range(1, 64);
        let strings: Vec<CString> = (0..n).map(|_| rng.ascii_range(0, 48)).collect();
        let cap = rng.range(0, 128) as c_int;
        append_sequence_diff(cap, &strings, "row24");
    }
}

#[test]
fn cfg_25_repeated_appends_tiny_initial_capacity() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 0x25);
    for _ in 0..200 {
        let n = rng.range(1, 64);
        let strings: Vec<CString> = (0..n).map(|_| rng.ascii_range(1, 12)).collect();
        let cap = rng.range(0, 8) as c_int;
        append_sequence_diff(cap, &strings, "row25");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 26: cross-library buffer ownership
// ---------------------------------------------------------------------------
#[test]
fn cfg_26_cross_library_buffer_ownership() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 0x26);
    for _ in 0..300 {
        let cap = rng.range(0, 64) as c_int;
        let n = rng.range(1, 20);
        let strings: Vec<CString> = (0..n).map(|_| rng.ascii_range(0, 24)).collect();

        unsafe {
            // C creates, Rust appends, C destroys.
            let b1 = (c().create_buffer)(cap);
            // Rust creates, C appends, Rust destroys.
            let b2 = (rust().create_buffer)(cap);
            assert!(!b1.is_null() && !b2.is_null());

            for (i, s) in strings.iter().enumerate() {
                let rc1 = (rust().append_to_buffer)(b1, s.as_ptr());
                let rc2 = (c().append_to_buffer)(b2, s.as_ptr());
                assert_eq!(rc1, rc2, "cross step {i}: rust-on-C-buf={rc1} c-on-rust-buf={rc2}");
                let s1 = snapshot(b1);
                let s2 = snapshot(b2);
                assert_eq!(s1, s2, "cross step {i}: {s1:?} vs {s2:?}");
            }

            (c().destroy_buffer)(b1);
            (rust().destroy_buffer)(b2);
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 34: interleaved repeated calls — no hidden global state
// ---------------------------------------------------------------------------
#[test]
fn cfg_34_no_hidden_global_state() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 0x34);
    let ops: Vec<CString> = ["add", "subtract", "multiply", "divide", "nonsense", ""]
        .iter()
        .map(|s| CString::new(*s).unwrap())
        .collect();

    // Repeat an identical script many times, interleaving both libraries, and
    // require the results to be identical on every repetition.
    let mut first: Option<Vec<i32>> = None;
    for _rep in 0..50 {
        let mut trace = Vec::new();
        let mut r = Rng::new(0xDEAD_BEEF);
        for _ in 0..200 {
            let a = r.next_small_i32();
            let mut b = r.next_small_i32();
            let op = &ops[r.range(0, ops.len() - 1)];
            if op.as_bytes() == b"divide" && (b == 0 || (a == i32::MIN && b == -1)) {
                b = 5;
            }
            unsafe {
                let cv = (c().perform_operation)(a, b, op.as_ptr());
                let rv = (rust().perform_operation)(a, b, op.as_ptr());
                assert_eq!(rv, cv, "interleaved perform_operation({a},{b},{op:?})");
                trace.push(cv);

                let code = r.next_i32();
                let cn = cstr_bytes((c().get_operation_name)(code));
                let rn = cstr_bytes((rust().get_operation_name)(code));
                assert_eq!(rn, cn, "interleaved get_operation_name({code})");

                let cap = r.range(0, 64) as c_int;
                let cb = (c().create_buffer)(cap);
                let rb = (rust().create_buffer)(cap);
                let s = rng.ascii(r.range(0, 40));
                (c().append_to_buffer)(cb, s.as_ptr());
                (rust().append_to_buffer)(rb, s.as_ptr());
                assert_eq!(snapshot(rb), snapshot(cb), "interleaved append cap={cap}");
                (c().destroy_buffer)(cb);
                (rust().destroy_buffer)(rb);
            }
        }
        match &first {
            None => first = Some(trace),
            Some(f) => assert_eq!(&trace, f, "results changed between repetitions"),
        }
    }
}

// ---------------------------------------------------------------------------
// Guard: the struct layout the tests assume matches what both libraries use.
// ---------------------------------------------------------------------------
#[test]
fn struct_layout_is_abi_identical() {
    assert_eq!(std::mem::size_of::<StringBuffer>(), 16);
    assert_eq!(std::mem::align_of::<StringBuffer>(), 8);
    unsafe {
        // Fill a buffer via C, read the fields via the Rust library's append
        // and confirm the Rust library agrees about where `length` lives.
        let b = (c().create_buffer)(4);
        let s = CString::new("abcdefgh").unwrap();
        let rc = (rust().append_to_buffer)(b, s.as_ptr());
        assert_eq!(rc, 0);
        assert_eq!((*b).length, 8);
        assert_eq!((*b).capacity, 18); // (0 + 8 + 1) * 2
        assert_eq!(cstr_bytes((*b).data), b"abcdefgh".to_vec());
        (c().destroy_buffer)(b);
        let _ = (std::ptr::null_mut::<c_void>(), 0 as *mut c_char);
    }
}
