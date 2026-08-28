// Phase B -- valid-path differential tests for the LOW-LEVEL entry points.
// CONFIGS.md rows 1..24.
//
// Everything is invoked through `dlsym` on both `.so`s; no Rust function is
// called directly.

mod common;

use common::*;
use std::ffi::c_int;

// ===========================================================================
// CONFIGS.md rows 1-6 -- get_operation_name (the `switch` arms, A1)
// ===========================================================================

fn check_op_name(op_code: c_int, expect: &[u8]) {
    let (c, r) = pair();
    unsafe {
        let cn = read_cstr((c.get_operation_name)(op_code));
        let rn = read_cstr((r.get_operation_name)(op_code));
        assert_eq!(
            cn, rn,
            "get_operation_name({op_code}): C {:?} != Rust {:?}",
            String::from_utf8_lossy(&cn),
            String::from_utf8_lossy(&rn)
        );
        assert_eq!(cn, expect, "get_operation_name({op_code}) unexpected");
    }
}

#[test]
fn row01_op_name_add() {
    check_op_name(0, b"add");
}

#[test]
fn row02_op_name_subtract() {
    check_op_name(1, b"subtract");
}

#[test]
fn row03_op_name_multiply() {
    check_op_name(2, b"multiply");
}

#[test]
fn row04_op_name_divide() {
    check_op_name(3, b"divide");
}

#[test]
fn row05_op_name_default_arm_randomized() {
    // Out-of-range "enum" values: a C enum accepts any int.
    for v in [
        4,
        5,
        6,
        100,
        1 << 20,
        -1,
        -2,
        -3,
        -4,
        -100,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
    ] {
        check_op_name(v, b"unknown");
    }
    let mut rng = Rng::new();
    for _ in 0..4096 {
        let v = rng.spicy_i32();
        let expect: &[u8] = match v {
            0 => b"add",
            1 => b"subtract",
            2 => b"multiply",
            3 => b"divide",
            _ => b"unknown",
        };
        check_op_name(v, expect);
    }
}

#[test]
fn row06_op_name_pointer_stability() {
    let (c, r) = pair();
    unsafe {
        for api in [c, r] {
            // Static storage: repeated calls hand back the identical pointer.
            for k in [0, 1, 2, 3, 4, -1, 999] {
                let a = (api.get_operation_name)(k);
                let b = (api.get_operation_name)(k);
                assert_eq!(a, b, "{}: get_operation_name({k}) not stable", api.tag);
            }
            // Distinct arms are distinct objects.
            let ps: Vec<_> = (0..5).map(|k| (api.get_operation_name)(k)).collect();
            for i in 0..5 {
                for j in (i + 1)..5 {
                    assert_ne!(ps[i], ps[j], "{}: arms {i}/{j} alias", api.tag);
                }
            }
            // And the 5th (out-of-range) arm is "unknown", same object as any
            // other out-of-range code.
            assert_eq!(
                (api.get_operation_name)(4),
                (api.get_operation_name)(i32::MIN),
                "{}: default arm not a single object",
                api.tag
            );
        }
    }
}

// ===========================================================================
// CONFIGS.md rows 7-13 -- perform_operation (string dispatch A2 x overflow)
// ===========================================================================

fn check_perform(a: c_int, b: c_int, op: &[u8]) {
    let (cl, rl) = pair();
    let s = cstring(op);
    unsafe {
        let cv = (cl.perform_operation)(a, b, s.as_ptr());
        let rv = (rl.perform_operation)(a, b, s.as_ptr());
        assert_eq!(
            cv,
            rv,
            "perform_operation({a}, {b}, {:?}): C {cv} != Rust {rv}",
            String::from_utf8_lossy(op)
        );
    }
}

/// Operand pairs shared by rows 7-9: boundaries plus randomised draws.
fn arith_pairs(n: usize) -> Vec<(i32, i32)> {
    let mut v = vec![
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MAX, -1),
        (i32::MIN, -1),
        (i32::MIN, 1),
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MIN + 1, -1),
        (2, i32::MAX),
        (-2, i32::MIN),
        (65536, 65536),
        (46341, 46341),
    ];
    let mut rng = Rng::new();
    for _ in 0..n {
        v.push((rng.spicy_i32(), rng.spicy_i32()));
    }
    v
}

#[test]
fn row07_perform_add() {
    for (a, b) in arith_pairs(4096) {
        check_perform(a, b, b"add");
    }
}

#[test]
fn row08_perform_subtract() {
    for (a, b) in arith_pairs(4096) {
        check_perform(a, b, b"subtract");
    }
}

#[test]
fn row09_perform_multiply() {
    for (a, b) in arith_pairs(4096) {
        check_perform(a, b, b"multiply");
    }
}

#[test]
fn row10_perform_divide_nonzero_all_sign_combos() {
    // Truncation toward zero in all four sign quadrants.
    for (a, b) in arith_pairs(4096) {
        // (INT_MIN, -1) traps in C; that is ERRORS.md #16, not a valid-path row.
        if b == 0 || (a == i32::MIN && b == -1) {
            continue;
        }
        check_perform(a, b, b"divide");
    }
    let mut rng = Rng::new();
    for _ in 0..4096 {
        let a = rng.range_i32(-100_000, 100_000);
        let mut b = rng.range_i32(-1000, 1000);
        if b == 0 {
            b = 7;
        }
        check_perform(a, b, b"divide");
    }
}

#[test]
fn row11_perform_divide_boundaries() {
    let edges = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    for &a in &edges {
        for &b in &edges {
            if b == 0 || (a == i32::MIN && b == -1) {
                continue;
            }
            check_perform(a, b, b"divide");
        }
    }
}

#[test]
fn row12_perform_divide_by_zero_returns_zero() {
    let mut rng = Rng::new();
    for a in [0, 1, -1, i32::MIN, i32::MAX, 12345, -99999] {
        check_perform(a, 0, b"divide");
    }
    for _ in 0..2048 {
        check_perform(rng.spicy_i32(), 0, b"divide");
    }
}

#[test]
fn row13_perform_with_name_from_get_operation_name() {
    // Composed pipeline: the operation selector is the pointer the library's own
    // get_operation_name returned, not a literal from the test.
    let (cl, rl) = pair();
    let mut rng = Rng::new();
    unsafe {
        for code in [0, 1, 2, 3, 4, -1, -2, -3, 7, i32::MIN, i32::MAX] {
            for _ in 0..256 {
                let a = rng.spicy_i32();
                let mut b = rng.spicy_i32();
                if a == i32::MIN && b == -1 {
                    b = -2;
                }
                let cop = (cl.get_operation_name)(code);
                let rop = (rl.get_operation_name)(code);
                let cv = (cl.perform_operation)(a, b, cop);
                let rv = (rl.perform_operation)(a, b, rop);
                assert_eq!(cv, rv, "composed code={code} a={a} b={b}: {cv} != {rv}");

                // Cross-wire the pointers too: the ABI contract is just
                // `const char*`, so each library must accept the other's.
                let cx = (cl.perform_operation)(a, b, rop);
                let rx = (rl.perform_operation)(a, b, cop);
                assert_eq!(cx, cv, "cross-wired C differs, code={code}");
                assert_eq!(rx, rv, "cross-wired Rust differs, code={code}");
            }
        }
    }
}

#[test]
fn row13b_perform_unmatched_operation_strings() {
    // A2 no-match fall-through with valid (non-NULL) strings.
    let mut rng = Rng::new();
    let fixed: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"ADD".to_vec(),
        b"Add".to_vec(),
        b"add ".to_vec(),
        b" add".to_vec(),
        b"addd".to_vec(),
        b"ad".to_vec(),
        b"a".to_vec(),
        b"subtrac".to_vec(),
        b"subtracte".to_vec(),
        b"multiply2".to_vec(),
        b"div".to_vec(),
        b"divide\t".to_vec(),
        b"unknown".to_vec(),
        b"DIVIDE".to_vec(),
        b"\x7f".to_vec(),
        b"\x01\x02\x03".to_vec(),
    ];
    for op in &fixed {
        for (a, b) in arith_pairs(64) {
            check_perform(a, b, op);
        }
    }
    for _ in 0..2048 {
        let len = rng.below(12) as usize;
        let s = rng.ascii_bytes(len);
        check_perform(rng.spicy_i32(), rng.spicy_i32(), &s);
    }
}

// ===========================================================================
// CONFIGS.md rows 14-24 -- create_buffer / append_to_buffer / destroy_buffer
// ===========================================================================

/// Create a buffer in both libraries and diff the initial state.
unsafe fn create_both(cap: c_int) -> (*mut StringBuffer, *mut StringBuffer) {
    let (cl, rl) = pair();
    let cb = (cl.create_buffer)(cap);
    let rb = (rl.create_buffer)(cap);
    assert_eq!(
        cb.is_null(),
        rb.is_null(),
        "create_buffer({cap}): nullness differs (C null={}, Rust null={})",
        cb.is_null(),
        rb.is_null()
    );
    if !cb.is_null() {
        assert_eq!(
            snapshot(cb),
            snapshot(rb),
            "create_buffer({cap}): initial state differs"
        );
    }
    (cb, rb)
}

#[test]
fn row14_create_capacity_one() {
    unsafe {
        let (cl, rl) = pair();
        let (cb, rb) = create_both(1);
        assert!(!cb.is_null());
        assert_eq!((*cb).capacity, 1);
        assert_eq!((*cb).length, 0);
        assert_eq!(read_n((*cb).data, 1), vec![0u8]);
        (cl.destroy_buffer)(cb);
        (rl.destroy_buffer)(rb);
    }
}

#[test]
fn row15_create_capacity_32_the_buffapp_value() {
    unsafe {
        let (cl, rl) = pair();
        let (cb, rb) = create_both(32);
        assert_eq!((*cb).capacity, 32);
        assert_eq!((*cb).length, 0);
        (cl.destroy_buffer)(cb);
        (rl.destroy_buffer)(rb);
    }
}

#[test]
fn row16_create_various_capacities() {
    unsafe {
        let (cl, rl) = pair();
        let mut caps: Vec<c_int> = vec![1, 2, 3, 4, 7, 8, 16, 31, 32, 33, 64, 1024, 1 << 20];
        let mut rng = Rng::new();
        for _ in 0..512 {
            caps.push(rng.range_i32(1, 65536));
        }
        for cap in caps {
            let (cb, rb) = create_both(cap);
            assert_eq!((*cb).capacity, cap);
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

/// Append the same string to both buffers and diff return value + full state.
unsafe fn append_both(
    cb: *mut StringBuffer,
    rb: *mut StringBuffer,
    s: &[u8],
    ctx: &str,
) -> c_int {
    let (cl, rl) = pair();
    let cs = cstring(s);
    let cr = (cl.append_to_buffer)(cb, cs.as_ptr());
    let rr = (rl.append_to_buffer)(rb, cs.as_ptr());
    assert_eq!(cr, rr, "{ctx}: append return C {cr} != Rust {rr}");
    assert_eq!(snapshot(cb), snapshot(rb), "{ctx}: buffer state differs");
    cr
}

#[test]
fn row17_append_no_grow_branch() {
    unsafe {
        let (cl, rl) = pair();
        let mut rng = Rng::new();
        for _ in 0..512 {
            let cap = rng.range_i32(16, 4096);
            let (cb, rb) = create_both(cap);
            // required = 0 + len + 1 <= cap  =>  len <= cap - 1
            let len = rng.below(cap as u64) as usize; // 0 ..= cap-1
            let s = rng.ascii_bytes(len);
            assert_eq!(append_both(cb, rb, &s, "row17"), 0);
            assert_eq!((*cb).capacity, cap, "row17: capacity must not grow");
            assert_eq!((*cb).length, len as c_int);
            assert_eq!(read_n((*cb).data, len), s);
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row18_append_grow_branch_exact_new_capacity() {
    unsafe {
        let (cl, rl) = pair();
        let mut rng = Rng::new();
        for _ in 0..512 {
            let cap = rng.range_i32(1, 64);
            let (cb, rb) = create_both(cap);
            // Force required > cap.
            let len = (cap as usize) + rng.below(200) as usize;
            let s = rng.ascii_bytes(len);
            assert_eq!(append_both(cb, rb, &s, "row18"), 0);
            let required = len as c_int + 1;
            assert!(required > cap);
            assert_eq!(
                (*cb).capacity,
                required * 2,
                "row18: new_capacity must be required*2"
            );
            assert_eq!((*cb).length, len as c_int);
            assert_eq!(read_n((*cb).data, len + 1), [s.as_slice(), &[0]].concat());
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row19_append_exact_capacity_boundary_sweep() {
    unsafe {
        let (cl, rl) = pair();
        for cap in 1..=64i32 {
            // len == cap-1  -> required == cap  -> NOT > cap  -> no grow
            // len == cap    -> required == cap+1 -> > cap      -> grow
            for delta in [-1i64, 0, 1] {
                let len = (cap as i64 - 1 + delta).max(0) as usize;
                let mut rng = Rng::with_seed(SEED ^ (cap as u64) << 8 ^ (delta as u64));
                let s = rng.ascii_bytes(len);
                let (cb, rb) = create_both(cap);
                assert_eq!(append_both(cb, rb, &s, "row19"), 0);
                let required = len as c_int + 1;
                let expect_cap = if required > cap { required * 2 } else { cap };
                assert_eq!(
                    (*cb).capacity,
                    expect_cap,
                    "row19: cap={cap} len={len} grow decision"
                );
                assert_eq!((*cb).length, len as c_int);
                (cl.destroy_buffer)(cb);
                (rl.destroy_buffer)(rb);
            }
        }
    }
}

#[test]
fn row20_append_empty_string() {
    unsafe {
        let (cl, rl) = pair();
        // From length == 0 with capacity 0: required = 1 > 0 -> grows to 2.
        for cap in [0i32, 1, 2, 8, 32] {
            let (cb, rb) = create_both(cap);
            if cb.is_null() {
                continue;
            }
            assert_eq!(append_both(cb, rb, b"", "row20-empty-fresh"), 0);
            assert_eq!((*cb).length, 0);
            let expect_cap = if 1 > cap { 2 } else { cap };
            assert_eq!((*cb).capacity, expect_cap);
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
        // From length > 0.
        for cap in [4i32, 16, 64] {
            let (cb, rb) = create_both(cap);
            append_both(cb, rb, b"hello", "row20-seed");
            let before_len = (*cb).length;
            let before_cap = (*cb).capacity;
            assert_eq!(append_both(cb, rb, b"", "row20-empty-after"), 0);
            assert_eq!((*cb).length, before_len, "empty append must not move length");
            let required = before_len + 1;
            let expect_cap = if required > before_cap { required * 2 } else { before_cap };
            assert_eq!((*cb).capacity, expect_cap);
            assert_eq!(read_n((*cb).data, before_len as usize), b"hello".to_vec());
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row21_many_random_appends_trajectory() {
    unsafe {
        let (cl, rl) = pair();
        let mut rng = Rng::new();
        for trial in 0..64 {
            let cap = rng.range_i32(1, 128);
            let (cb, rb) = create_both(cap);
            let mut hwm = 1usize; // data[0] was written by create_buffer
            for step in 0..120 {
                let len = match rng.below(5) {
                    0 => 0,
                    1 => 1,
                    2 => rng.below(8) as usize,
                    3 => rng.below(64) as usize,
                    _ => rng.below(300) as usize,
                };
                let s = rng.ascii_bytes(len);
                let ctx = format!("row21 trial={trial} step={step} cap={cap} len={len}");
                assert_eq!(append_both(cb, rb, &s, &ctx), 0);
                hwm = hwm.max((*cb).length as usize + 1);
                // Everything up to the high-water mark was written by an append
                // in both libraries, so it is comparable byte-for-byte.
                assert_eq!(
                    snapshot_hwm(cb, hwm),
                    snapshot_hwm(rb, hwm),
                    "{ctx}: high-water bytes differ"
                );
                assert!(
                    (*cb).length < (*cb).capacity,
                    "{ctx}: invariant length < capacity"
                );
            }
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row22_append_after_length_rewound_to_zero() {
    // This is exactly what buffapp line 116 does: `log_buffer->length = 0;`
    unsafe {
        let (cl, rl) = pair();
        let mut rng = Rng::new();
        for trial in 0..128 {
            let cap = rng.range_i32(1, 64);
            let (cb, rb) = create_both(cap);
            let first = rng.ascii_below(80);
            append_both(cb, rb, &first, "row22-first");
            let hwm = (*cb).length as usize + 1;

            // External rewind, then overwrite.
            (*cb).length = 0;
            (*rb).length = 0;
            let second = rng.ascii_below(80);
            let ctx = format!("row22 trial={trial}");
            assert_eq!(append_both(cb, rb, &second, &ctx), 0);
            assert_eq!((*cb).length, second.len() as c_int);
            // Stale bytes past the new NUL came from `first`, so they are
            // deterministic up to the high-water mark.
            let hwm = hwm.max(second.len() + 1);
            assert_eq!(
                snapshot_hwm(cb, hwm),
                snapshot_hwm(rb, hwm),
                "{ctx}: stale-byte tail differs"
            );
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row23_append_at_externally_set_midbuffer_offset() {
    unsafe {
        let (cl, rl) = pair();
        let mut rng = Rng::new();
        for trial in 0..256 {
            let cap = rng.range_i32(8, 256);
            let (cb, rb) = create_both(cap);
            // Set length into the middle, leaving an uninitialised gap that we
            // deliberately do NOT compare.
            let off = rng.range_i32(1, cap - 1);
            (*cb).length = off;
            (*rb).length = off;
            let len = rng.below(400) as usize;
            let s = rng.ascii_bytes(len);
            let cs = cstring(&s);
            let cr = (cl.append_to_buffer)(cb, cs.as_ptr());
            let rr = (rl.append_to_buffer)(rb, cs.as_ptr());
            let ctx = format!("row23 trial={trial} cap={cap} off={off} len={len}");
            assert_eq!(cr, rr, "{ctx}: return differs");
            assert_eq!((*cb).capacity, (*rb).capacity, "{ctx}: capacity differs");
            assert_eq!((*cb).length, (*rb).length, "{ctx}: length differs");
            let required = off + len as c_int + 1;
            let expect_cap = if required > cap { required * 2 } else { cap };
            assert_eq!((*cb).capacity, expect_cap, "{ctx}: grow decision");
            assert_eq!((*cb).length, off + len as c_int, "{ctx}: length arithmetic");
            // Compare only the region strcpy actually wrote: [off, off+len].
            let cbytes = read_n((*cb).data.add(off as usize), len + 1);
            let rbytes = read_n((*rb).data.add(off as usize), len + 1);
            assert_eq!(cbytes, rbytes, "{ctx}: written region differs");
            assert_eq!(cbytes, [s.as_slice(), &[0]].concat(), "{ctx}: wrong bytes");
            (*cb).length = 0;
            (*rb).length = 0;
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

#[test]
fn row24_full_lowlevel_lifecycle_randomized() {
    unsafe {
        let (cl, rl) = pair();
        let mut rng = Rng::new();
        for trial in 0..256 {
            let cap = rng.range_i32(1, 512);
            let (cb, rb) = create_both(cap);
            let mut hwm = 1usize;
            let steps = 1 + rng.below(30) as usize;
            for step in 0..steps {
                match rng.below(6) {
                    0 => {
                        // rewind
                        (*cb).length = 0;
                        (*rb).length = 0;
                    }
                    _ => {
                        let s = rng.ascii_below(120);
                        let ctx = format!("row24 trial={trial} step={step}");
                        assert_eq!(append_both(cb, rb, &s, &ctx), 0);
                        hwm = hwm.max((*cb).length as usize + 1);
                        assert_eq!(snapshot_hwm(cb, hwm), snapshot_hwm(rb, hwm), "{ctx}");
                    }
                }
            }
            // Each heap block goes back to the allocator through the same
            // library that produced it.
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}
