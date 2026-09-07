// Phase B — valid-path differential tests. One test per CONFIGS.md row.
// Every call goes through a .so loaded with libloading (C and Rust alike).

mod common;

use common::*;
use std::ffi::{c_char, c_int, CStr, CString};

fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Byte string (possibly non-UTF-8) as a NUL-terminated C buffer.
fn cbytes(b: &[u8]) -> Vec<c_char> {
    assert!(!b.contains(&0));
    let mut v: Vec<c_char> = b.iter().map(|&x| x as c_char).collect();
    v.push(0);
    v
}

fn name_of(p: *const c_char) -> Vec<u8> {
    assert!(!p.is_null());
    unsafe { CStr::from_ptr(p).to_bytes().to_vec() }
}

// --- C1 -------------------------------------------------------------------
#[test]
fn c1_create_destroy_fixed_capacities() {
    let p = pair();
    for cap in [0i32, 1, 7, 32, 1 << 20] {
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            let sc = p.c.snapshot(bc);
            let sr = p.rs.snapshot(br);
            assert_eq!(sc, sr, "create_buffer({cap})");
            assert!(!sc.null, "create_buffer({cap}) unexpectedly NULL");
            assert_eq!(sc.capacity, cap);
            assert_eq!(sc.length, 0);
            assert_eq!(sc.bytes, Vec::<u8>::new(), "data[0] must be NUL");
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C2 -------------------------------------------------------------------
#[test]
fn c2_create_destroy_randomized_capacities() {
    let p = pair();
    let mut rng = Rng::new(0xC2_5EED);
    for _ in 0..2000 {
        let cap = rng.range(1, 4096) as c_int;
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "create_buffer({cap})");
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C3 -------------------------------------------------------------------
#[test]
fn c3_append_no_growth() {
    let p = pair();
    let s = cs("hello world");
    unsafe {
        let bc = (p.c.create_buffer)(256);
        let br = (p.rs.create_buffer)(256);
        let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
        let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
        assert_eq!(rc, 0);
        assert_eq!(rc, rr);
        let sc = p.c.snapshot(bc);
        assert_eq!(sc, p.rs.snapshot(br));
        assert_eq!(sc.capacity, 256, "no realloc expected");
        assert_eq!(sc.length, 11);
        assert_eq!(sc.bytes, b"hello world".to_vec());
        (p.c.destroy_buffer)(bc);
        (p.rs.destroy_buffer)(br);
    }
}

// --- C4 -------------------------------------------------------------------
#[test]
fn c4_append_forces_growth() {
    let p = pair();
    let s = cs("a string much longer than one byte");
    unsafe {
        let bc = (p.c.create_buffer)(1);
        let br = (p.rs.create_buffer)(1);
        let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
        let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
        assert_eq!((rc, rr), (0, 0));
        let sc = p.c.snapshot(bc);
        assert_eq!(sc, p.rs.snapshot(br));
        assert_eq!(sc.capacity, (s.as_bytes().len() as c_int + 1) * 2);
        assert_eq!(sc.length, s.as_bytes().len() as c_int);
        assert_eq!(sc.bytes, s.as_bytes().to_vec());
        (p.c.destroy_buffer)(bc);
        (p.rs.destroy_buffer)(br);
    }
}

// --- C5 -------------------------------------------------------------------
#[test]
fn c5_boundary_required_equals_capacity() {
    let p = pair();
    for cap in 1..=64i32 {
        let s = cs(&"x".repeat(cap as usize - 1)); // required == cap
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
            let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
            assert_eq!((rc, rr), (0, 0), "cap={cap}");
            let sc = p.c.snapshot(bc);
            assert_eq!(sc, p.rs.snapshot(br), "cap={cap}");
            assert_eq!(sc.capacity, cap, "cap={cap}: growth must NOT happen");
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C6 -------------------------------------------------------------------
#[test]
fn c6_boundary_required_equals_capacity_plus_one() {
    let p = pair();
    for cap in 1..=64i32 {
        let s = cs(&"y".repeat(cap as usize)); // required == cap + 1
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
            let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
            assert_eq!((rc, rr), (0, 0), "cap={cap}");
            let sc = p.c.snapshot(bc);
            assert_eq!(sc, p.rs.snapshot(br), "cap={cap}");
            assert_eq!(sc.capacity, (cap + 1) * 2, "cap={cap}: must grow to required*2");
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C7 -------------------------------------------------------------------
#[test]
fn c7_repeated_empty_appends() {
    let p = pair();
    let s = cs("");
    for cap in [0i32, 1, 8, 32] {
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            for i in 0..8 {
                let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
                let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
                assert_eq!((rc, rr), (0, 0), "cap={cap} i={i}");
                assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "cap={cap} i={i}");
            }
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C8 -------------------------------------------------------------------
#[test]
fn c8_many_one_byte_appends_from_capacity_one() {
    let p = pair();
    let s = cs("z");
    unsafe {
        let bc = (p.c.create_buffer)(1);
        let br = (p.rs.create_buffer)(1);
        for i in 0..200 {
            let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
            let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
            assert_eq!((rc, rr), (0, 0), "i={i}");
            let sc = p.c.snapshot(bc);
            assert_eq!(sc, p.rs.snapshot(br), "i={i}");
            assert_eq!(sc.length, i + 1);
        }
        (p.c.destroy_buffer)(bc);
        (p.rs.destroy_buffer)(br);
    }
}

// --- C9 -------------------------------------------------------------------
#[test]
fn c9_randomized_many_appends() {
    let p = pair();
    let mut rng = Rng::new(0xC9_5EED);
    for round in 0..300 {
        let cap = rng.range(0, 64) as c_int;
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            let nappends = rng.range(1, 12);
            for i in 0..nappends {
                let len = rng.range(0, 40) as usize;
                let bytes: Vec<u8> = (0..len).map(|_| rng.range(1, 127) as u8).collect();
                let s = cbytes(&bytes);
                let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
                let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
                assert_eq!((rc, rr), (0, 0), "round={round} i={i}");
                assert_eq!(
                    p.c.snapshot(bc),
                    p.rs.snapshot(br),
                    "round={round} i={i} cap={cap} len={len}"
                );
            }
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C10 ------------------------------------------------------------------
#[test]
fn c10_high_bit_bytes() {
    let p = pair();
    let mut rng = Rng::new(0xC10_5EED);
    for round in 0..300 {
        let cap = rng.range(0, 32) as c_int;
        unsafe {
            let bc = (p.c.create_buffer)(cap);
            let br = (p.rs.create_buffer)(cap);
            for i in 0..rng.range(1, 8) {
                let len = rng.range(0, 32) as usize;
                let bytes: Vec<u8> = (0..len).map(|_| rng.range(0x80, 0xFF) as u8).collect();
                let s = cbytes(&bytes);
                let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
                let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
                assert_eq!((rc, rr), (0, 0), "round={round} i={i}");
                assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "round={round} i={i}");
            }
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C11 ------------------------------------------------------------------
#[test]
fn c11_append_from_zero_capacity() {
    let p = pair();
    let mut rng = Rng::new(0xC11_5EED);
    for _ in 0..200 {
        let len = rng.range(0, 24) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.range(b'a' as u32, b'z' as u32) as u8).collect();
        let s = cbytes(&bytes);
        unsafe {
            let bc = (p.c.create_buffer)(0);
            let br = (p.rs.create_buffer)(0);
            let rc = (p.c.append_to_buffer)(bc, s.as_ptr());
            let rr = (p.rs.append_to_buffer)(br, s.as_ptr());
            assert_eq!((rc, rr), (0, 0));
            let sc = p.c.snapshot(bc);
            assert_eq!(sc, p.rs.snapshot(br), "len={len}");
            assert_eq!(sc.capacity, (len as c_int + 1) * 2);
            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C12 ------------------------------------------------------------------
#[test]
fn c12_get_operation_name_valid_arms() {
    let p = pair();
    let expect: [&[u8]; 4] = [b"add", b"subtract", b"multiply", b"divide"];
    for op in 0..4i32 {
        unsafe {
            let nc = name_of((p.c.get_operation_name)(op));
            let nr = name_of((p.rs.get_operation_name)(op));
            assert_eq!(nc, nr, "get_operation_name({op})");
            assert_eq!(nc, expect[op as usize].to_vec());
        }
    }
}

// --- C13 ------------------------------------------------------------------
#[test]
fn c13_get_operation_name_default_arm() {
    let p = pair();
    let mut rng = Rng::new(0xC13_5EED);
    let mut cases: Vec<i32> = vec![4, 5, -1, -2, -3, 42, i32::MAX, i32::MIN, i32::MIN + 1, i32::MAX - 1];
    for _ in 0..2000 {
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

// --- C14..C17 -------------------------------------------------------------
fn diff_perform(op: &str, skip_div_overflow: bool, seed: u64) {
    let p = pair();
    let opc = cs(op);
    let mut rng = Rng::new(seed);
    let mut pairs: Vec<(i32, i32)> = Vec::new();
    for a in [0i32, 1, -1, 2, -2, 7, -7, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        for b in [0i32, 1, -1, 2, -2, 3, -3, i32::MAX, i32::MIN] {
            pairs.push((a, b));
        }
    }
    for _ in 0..3000 {
        pairs.push((rng.interesting_i32(), rng.interesting_i32()));
    }
    for (a, b) in pairs {
        // INT_MIN / -1 traps (see ERRORS.md E20); covered separately in Phase C.
        if skip_div_overflow && b == -1 && a == i32::MIN {
            continue;
        }
        unsafe {
            let rc = (p.c.perform_operation)(a, b, opc.as_ptr());
            let rr = (p.rs.perform_operation)(a, b, opc.as_ptr());
            assert_eq!(rc, rr, "perform_operation({a}, {b}, {op:?}): C={rc} RUST={rr}");
        }
    }
}

#[test]
fn c14_perform_add() {
    diff_perform("add", false, 0xC14_5EED);
}

#[test]
fn c15_perform_subtract() {
    diff_perform("subtract", false, 0xC15_5EED);
}

#[test]
fn c16_perform_multiply() {
    diff_perform("multiply", false, 0xC16_5EED);
}

#[test]
fn c17_perform_divide_nonzero() {
    diff_perform("divide", true, 0xC17_5EED);
}

// --- C18 ------------------------------------------------------------------
#[test]
fn c18_perform_divide_by_zero() {
    let p = pair();
    let opc = cs("divide");
    let mut rng = Rng::new(0xC18_5EED);
    let mut vals: Vec<i32> = vec![0, 1, -1, i32::MAX, i32::MIN];
    for _ in 0..1000 {
        vals.push(rng.interesting_i32());
    }
    for a in vals {
        unsafe {
            let rc = (p.c.perform_operation)(a, 0, opc.as_ptr());
            let rr = (p.rs.perform_operation)(a, 0, opc.as_ptr());
            assert_eq!(rc, rr, "divide({a}, 0)");
            assert_eq!(rc, 0, "divide({a}, 0) must be 0");
        }
    }
}

// --- C19 ------------------------------------------------------------------
#[test]
fn c19_cross_library_operation_pointer() {
    let p = pair();
    let mut rng = Rng::new(0xC19_5EED);
    for op_code in [-3i32, -2, -1, 0, 1, 2, 3, 4, 99] {
        unsafe {
            let from_c = (p.c.get_operation_name)(op_code);
            let from_r = (p.rs.get_operation_name)(op_code);
            assert_eq!(name_of(from_c), name_of(from_r), "op_code={op_code}");
            for _ in 0..300 {
                let a = rng.interesting_i32();
                let mut b = rng.interesting_i32();
                if op_code == 3 && b == -1 && a == i32::MIN {
                    b = 1; // trapping case, see E20
                }
                // C name -> Rust impl, and Rust name -> C impl.
                let r1 = (p.rs.perform_operation)(a, b, from_c);
                let r2 = (p.c.perform_operation)(a, b, from_r);
                let r3 = (p.c.perform_operation)(a, b, from_c);
                let r4 = (p.rs.perform_operation)(a, b, from_r);
                assert_eq!(
                    (r1, r2, r3), (r4, r4, r4),
                    "op_code={op_code} a={a} b={b}"
                );
            }
        }
    }
}

// --- C20 ------------------------------------------------------------------
/// Hand-composed reproduction of the `buffapp` pipeline built only from the
/// low-level entry points, run in lock-step against both libraries.
#[test]
fn c20_composed_pipeline_low_level() {
    let p = pair();
    let mut rng = Rng::new(0xC20_5EED);
    for round in 0..400 {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        let p3 = rng.interesting_i32();
        let p4 = rng.interesting_i32();
        // Avoid the trapping divisions; they are Phase C material.
        let traps = |a: i32, b: i32, code: i32| code.wrapping_rem(4) == 3 && b == -1 && a == i32::MIN;
        if traps(p1, p2, p1) || traps(p3, p4, p3) {
            continue;
        }
        unsafe {
            let bc = (p.c.create_buffer)(32);
            let br = (p.rs.create_buffer)(32);
            assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "round={round} create");

            let start = cs("Starting computation with 4 parameters\n");
            assert_eq!(
                (p.c.append_to_buffer)(bc, start.as_ptr()),
                (p.rs.append_to_buffer)(br, start.as_ptr())
            );
            assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "round={round} start line");

            let op1c = (p.c.get_operation_name)(p1.wrapping_rem(4));
            let op1r = (p.rs.get_operation_name)(p1.wrapping_rem(4));
            assert_eq!(name_of(op1c), name_of(op1r), "round={round} op1");

            let line1 = cs(&format!(
                "Operation 1: {}({}, {})\n",
                String::from_utf8(name_of(op1c)).unwrap(),
                p1,
                p2
            ));
            assert_eq!(
                (p.c.append_to_buffer)(bc, line1.as_ptr()),
                (p.rs.append_to_buffer)(br, line1.as_ptr())
            );
            assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "round={round} line1");

            let i1c = (p.c.perform_operation)(p1, p2, op1c);
            let i1r = (p.rs.perform_operation)(p1, p2, op1r);
            assert_eq!(i1c, i1r, "round={round} intermediate1");

            let op2c = (p.c.get_operation_name)(p3.wrapping_rem(4));
            let op2r = (p.rs.get_operation_name)(p3.wrapping_rem(4));
            assert_eq!(name_of(op2c), name_of(op2r), "round={round} op2");

            let line2 = cs(&format!(
                "Operation 2: {}({}, {})\n",
                String::from_utf8(name_of(op2c)).unwrap(),
                p3,
                p4
            ));
            assert_eq!(
                (p.c.append_to_buffer)(bc, line2.as_ptr()),
                (p.rs.append_to_buffer)(br, line2.as_ptr())
            );

            let i2c = (p.c.perform_operation)(p3, p4, op2c);
            let i2r = (p.rs.perform_operation)(p3, p4, op2r);
            assert_eq!(i2c, i2r, "round={round} intermediate2");

            let mult = cs("multiply");
            let i3c = (p.c.perform_operation)(i1c, i2c, mult.as_ptr());
            let i3r = (p.rs.perform_operation)(i1r, i2r, mult.as_ptr());
            assert_eq!(i3c, i3r, "round={round} intermediate3");

            let line3 = cs(&format!("Operation 3: multiply({}, {})\n", i1c, i2c));
            assert_eq!(
                (p.c.append_to_buffer)(bc, line3.as_ptr()),
                (p.rs.append_to_buffer)(br, line3.as_ptr())
            );
            assert_eq!(p.c.snapshot(bc), p.rs.snapshot(br), "round={round} line3");

            (p.c.destroy_buffer)(bc);
            (p.rs.destroy_buffer)(br);
        }
    }
}

// --- C21 ------------------------------------------------------------------
#[test]
fn c21_buffapp_all_positive_op_pairs() {
    for m1 in 0..4i32 {
        for m3 in 0..4i32 {
            for (p2, p4) in [(3i32, 5i32), (0, 7), (9, 0), (12, 4)] {
                diff_buffapp(m1 + 8, p2, m3 + 8, p4);
            }
        }
    }
}

// --- C22 ------------------------------------------------------------------
#[test]
fn c22_buffapp_negative_remainders() {
    for m1 in [-1i32, -2, -3, 0] {
        for m3 in [-1i32, -2, -3, 0] {
            for (p2, p4) in [(3i32, 5i32), (-4, 6), (0, -9)] {
                diff_buffapp(m1 - 8, p2, m3 - 8, p4);
            }
        }
    }
}

// --- C23 ------------------------------------------------------------------
#[test]
fn c23_buffapp_zero_intermediate3_fallback() {
    // op1 = add (p1 % 4 == 0) with p1 + p2 == 0 -> intermediate1 == 0
    // -> intermediate3 == 0 -> fallback `result = p1+p2+p3+p4`.
    diff_buffapp(4, -4, 5, 3);
    diff_buffapp(8, -8, 9, 1);
    // op1 = "unknown" via negative remainder -> intermediate1 == 0.
    diff_buffapp(-5, 3, 6, 2);
    // op2 = divide by zero -> intermediate2 == 0.
    diff_buffapp(4, 6, 7, 0);
    // both zero
    diff_buffapp(0, 0, 0, 0);
}

// --- C24 ------------------------------------------------------------------
#[test]
fn c24_buffapp_division_path() {
    for (a, b, c, d) in [
        (4i32, 6i32, 5i32, 3i32),
        (8, 2, 6, 4),
        (12, 7, 10, 3),
        (100, 3, 101, 7),
        (13, 5, 14, 6),
    ] {
        diff_buffapp(a, b, c, d);
    }
}

// --- C25 ------------------------------------------------------------------
#[test]
fn c25_buffapp_extremes() {
    let vals = [0i32, 1, -1, 2, -2, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    // Skip only the trapping combinations (Phase C covers them).
                    let i1 = eval(a, b, a.wrapping_rem(4));
                    let (i1, trap1) = match i1 {
                        Some(v) => (v, false),
                        None => (0, true),
                    };
                    let i2 = eval(c, d, c.wrapping_rem(4));
                    let (i2, trap2) = match i2 {
                        Some(v) => (v, false),
                        None => (0, true),
                    };
                    if trap1 || trap2 {
                        continue;
                    }
                    let i3 = i1.wrapping_mul(i2);
                    let res = i1.wrapping_add(i2);
                    if i3 != 0 && res == i32::MIN && i3 == -1 {
                        continue; // trapping final division
                    }
                    diff_buffapp(a, b, c, d);
                }
            }
        }
    }
}

/// Mirror of `perform_operation` used only to skip trapping inputs.
/// Returns `None` when the C code would raise SIGFPE.
fn eval(a: i32, b: i32, code: i32) -> Option<i32> {
    match code {
        0 => Some(a.wrapping_add(b)),
        1 => Some(a.wrapping_sub(b)),
        2 => Some(a.wrapping_mul(b)),
        3 => {
            if b == 0 {
                Some(0)
            } else if a == i32::MIN && b == -1 {
                None
            } else {
                Some(a / b)
            }
        }
        _ => Some(0),
    }
}

// --- C26 ------------------------------------------------------------------
#[test]
fn c26_buffapp_randomized() {
    let mut rng = Rng::new(0xC26_5EED);
    let mut done = 0;
    let mut tries = 0;
    while done < 1500 && tries < 20000 {
        tries += 1;
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let c = rng.interesting_i32();
        let d = rng.interesting_i32();
        let i1 = match eval(a, b, a.wrapping_rem(4)) {
            Some(v) => v,
            None => continue,
        };
        let i2 = match eval(c, d, c.wrapping_rem(4)) {
            Some(v) => v,
            None => continue,
        };
        let i3 = i1.wrapping_mul(i2);
        let res = i1.wrapping_add(i2);
        if i3 == -1 && res == i32::MIN {
            continue;
        }
        diff_buffapp(a, b, c, d);
        done += 1;
    }
    assert!(done >= 1500, "only generated {done} cases");
}

// --- C27 ------------------------------------------------------------------
#[test]
fn c27_interleaved_invocations() {
    let p = pair();
    let mut rng = Rng::new(0xC27_5EED);
    // Alternate C and Rust repeatedly in one process; state-free behaviour means
    // the Nth call's output must not depend on who ran before it.
    for _ in 0..300 {
        let a = (rng.range(0, 40) as i32) - 20;
        let b = (rng.range(0, 40) as i32) - 20;
        let c = (rng.range(0, 40) as i32) - 20;
        let d = (rng.range(0, 40) as i32) - 20;
        // Four invocations in one process, alternating implementations, all
        // inside a single isolated child so ordering effects would be visible.
        let oc = run_isolated(|| unsafe {
            let r1 = (p.c.buffapp)(a, b, c, d);
            let r2 = (p.rs.buffapp)(a, b, c, d);
            let r3 = (p.rs.buffapp)(a, b, c, d);
            let r4 = (p.c.buffapp)(a, b, c, d);
            assert_eq!((r1, r2, r3), (r4, r4, r4), "buffapp({a},{b},{c},{d})");
            Some(r1)
        });
        let orv = run_isolated(|| unsafe {
            let r1 = (p.rs.buffapp)(a, b, c, d);
            let r2 = (p.c.buffapp)(a, b, c, d);
            let r3 = (p.c.buffapp)(a, b, c, d);
            let r4 = (p.rs.buffapp)(a, b, c, d);
            assert_eq!((r1, r2, r3), (r4, r4, r4), "buffapp({a},{b},{c},{d})");
            Some(r1)
        });
        assert_eq!(oc.ret, orv.ret, "buffapp({a},{b},{c},{d}) ret");
        assert_eq!(oc.signal, orv.signal, "buffapp({a},{b},{c},{d}) signal");
        assert_eq!(oc.exit_code, Some(0), "C-first child died: {oc:?}");
        assert_eq!(orv.exit_code, Some(0), "RUST-first child died: {orv:?}");
        // Four identical log blocks either way round, in the same order.
        assert_eq!(
            oc.stdout,
            orv.stdout,
            "interleaved stdout ({a},{b},{c},{d}):\nC-first   = {:?}\nRUST-first= {:?}",
            String::from_utf8_lossy(&oc.stdout),
            String::from_utf8_lossy(&orv.stdout)
        );
    }
}
