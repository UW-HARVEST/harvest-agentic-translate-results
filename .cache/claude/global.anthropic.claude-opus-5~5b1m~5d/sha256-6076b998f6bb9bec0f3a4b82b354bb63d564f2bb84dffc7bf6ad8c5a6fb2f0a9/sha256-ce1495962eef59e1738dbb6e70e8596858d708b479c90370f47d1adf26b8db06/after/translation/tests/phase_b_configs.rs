// PHASE B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH shared objects through
// their exported C symbols and compares results byte-for-byte, using many
// seeded-random inputs per row.
//
// Rows 17-24 drive `mathop`, whose observable output includes its printf lines.
// They live in the `phase_stdout` binary (`harness = false`) so that no libtest
// progress output can interleave with a captured stdout region.

mod common;

use common::*;
use std::ptr;

const ITERS: usize = 3000;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Allocate a `count`-slot buffer using the implementation's own
/// `allocate_results`, so that entry point is exercised everywhere too.
unsafe fn alloc_buf(im: &Impl, count: i32) -> *mut ComputationResult {
    let p = (im.allocate_results)(count);
    assert!(!p.is_null(), "{}: allocate_results({count}) returned NULL", im.name);
    p
}

// ---------------------------------------------------------------------------
// Row 1 -- is_valid_operation, exhaustive over the whole `char` domain
// ---------------------------------------------------------------------------
#[test]
fn cfg_01_is_valid_operation_exhaustive() {
    let l = libs();
    for v in i8::MIN..=i8::MAX {
        let c = unsafe { (l.c.is_valid_operation)(v) };
        let r = unsafe { (l.rust.is_valid_operation)(v) };
        assert_same(c, r, &format!("is_valid_operation({v})"));
        // Cross-check against the C semantics spelled out by hand.
        let expected = u8::from(v != 0 && v >= b'1' as i8 && v <= b'5' as i8);
        assert_eq!(c, expected, "C itself disagrees with the source for {v}");
    }
}

// ---------------------------------------------------------------------------
// Row 2 -- get_operation_priority
// ---------------------------------------------------------------------------
#[test]
fn cfg_02_get_operation_priority_random() {
    let l = libs();
    let mut fixed: Vec<i32> = vec![
        OP_ADD,
        OP_MULTIPLY,
        OP_SUBTRACT,
        OP_DIVIDE,
        OP_MODULO,
        0,
        6,
        -1,
        -5,
        i32::MAX,
        i32::MIN,
        i32::MAX / 10,
        i32::MAX / 10 + 1,
        i32::MIN / 10,
        i32::MIN / 10 - 1,
        0x1999_999A,
    ];
    let mut rng = Rng::new();
    for _ in 0..ITERS {
        fixed.push(rng.interesting_i32());
    }
    for op in fixed {
        let c = unsafe { (l.c.get_operation_priority)(op) };
        let r = unsafe { (l.rust.get_operation_priority)(op) };
        assert_same(c, r, &format!("get_operation_priority({op})"));
    }
}

// ---------------------------------------------------------------------------
// Rows 3-5 -- add / multiply / subtract
// ---------------------------------------------------------------------------
fn binop_random(name: &str, cf: FnMath, rf: FnMath) {
    let mut rng = Rng::new();
    for i in 0..ITERS {
        let (a, b, unused) = (rng.interesting_i32(), rng.interesting_i32(), rng.next_i32());
        let c = unsafe { cf(a, b, unused) };
        let r = unsafe { rf(a, b, unused) };
        assert_same(c, r, &format!("{name}({a}, {b}, {unused}) [iter {i}]"));
    }
}

#[test]
fn cfg_03_add_operation_random() {
    let l = libs();
    binop_random("add_operation", l.c.add_operation, l.rust.add_operation);
}

#[test]
fn cfg_04_multiply_operation_random() {
    let l = libs();
    binop_random(
        "multiply_operation",
        l.c.multiply_operation,
        l.rust.multiply_operation,
    );
}

#[test]
fn cfg_05_subtract_operation_random() {
    let l = libs();
    binop_random(
        "subtract_operation",
        l.c.subtract_operation,
        l.rust.subtract_operation,
    );
}

// ---------------------------------------------------------------------------
// Rows 6-7 -- divide / modulo (all sign combinations, b == 1, b == -1)
// ---------------------------------------------------------------------------
fn divlike_random(name: &str, cf: FnMath, rf: FnMath) {
    let mut cases: Vec<(i32, i32)> = Vec::new();
    for &a in &[0, 1, -1, 7, -7, 6, -6, i32::MAX, i32::MIN, i32::MIN + 1] {
        for &b in &[1, -1, 2, -2, 3, -3, 7, -7, i32::MAX, i32::MIN] {
            cases.push((a, b));
        }
    }
    let mut rng = Rng::new();
    for _ in 0..ITERS {
        let a = rng.interesting_i32();
        let mut b = rng.interesting_i32();
        if b == 0 {
            b = 1; // zero divisor is an ERRORS.md row, covered in phase C
        }
        cases.push((a, b));
    }
    for (a, b) in cases {
        if b == 0 || is_c_div_trap(a, b) {
            continue; // ERRORS.md rows 5/6 and 25
        }
        let c = unsafe { cf(a, b, 0) };
        let r = unsafe { rf(a, b, 0) };
        assert_same(c, r, &format!("{name}({a}, {b})"));
    }
}

#[test]
fn cfg_06_divide_operation_random() {
    let l = libs();
    divlike_random("divide_operation", l.c.divide_operation, l.rust.divide_operation);
}

#[test]
fn cfg_07_modulo_operation_random() {
    let l = libs();
    divlike_random("modulo_operation", l.c.modulo_operation, l.rust.modulo_operation);
}

// ---------------------------------------------------------------------------
// Row 8 -- select_operation: every switch arm, and the returned function
//          pointer is actually invoked (behavioural identity, not address)
// ---------------------------------------------------------------------------
#[test]
fn cfg_08_select_operation_all_arms_invoked() {
    let l = libs();
    let mut ops: Vec<i32> = vec![1, 2, 3, 4, 5, 0, 6, 7, -1, -2, i32::MIN, i32::MAX];
    let mut rng = Rng::new();
    for _ in 0..200 {
        ops.push(rng.interesting_op());
        ops.push(rng.next_i32());
    }

    for op in ops {
        let cf = unsafe { (l.c.select_operation)(op) }
            .unwrap_or_else(|| panic!("C select_operation({op}) returned NULL"));
        let rf = unsafe { (l.rust.select_operation)(op) }
            .unwrap_or_else(|| panic!("Rust select_operation({op}) returned NULL"));

        for _ in 0..40 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let uses_idiv = op == OP_DIVIDE || op == OP_MODULO;
            if uses_idiv && is_c_div_trap(a, b) {
                continue;
            }
            let c = unsafe { cf(a, b, 0) };
            let r = unsafe { rf(a, b, 0) };
            assert_same(c, r, &format!("select_operation({op})({a}, {b})"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 -- get_computation_timestamp
// ---------------------------------------------------------------------------
#[test]
fn cfg_09_get_computation_timestamp() {
    let l = libs();
    for i in 0..500 {
        let c = unsafe { (l.c.get_computation_timestamp)() };
        let r = unsafe { (l.rust.get_computation_timestamp)() };
        assert_same(c, r, &format!("get_computation_timestamp() [iter {i}]"));
        // `time() >> 29`: a 64-bit time_t shifted right 29 bits.
        assert!(c > 0, "unexpected timestamp {c}");
    }
}

// ---------------------------------------------------------------------------
// Row 10 -- allocate_results with valid counts: non-NULL and fully zeroed
// ---------------------------------------------------------------------------
#[test]
fn cfg_10_allocate_results_valid_counts_zeroed() {
    let l = libs();
    for &count in &[0i32, 1, 2, 3, 10, 11, 64, 1024, 65536] {
        let cp = unsafe { (l.c.allocate_results)(count) };
        let rp = unsafe { (l.rust.allocate_results)(count) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "allocate_results({count}) null-ness differs (C {:?} / Rust {:?})",
            cp,
            rp
        );
        assert!(!cp.is_null(), "allocate_results({count}) should succeed");

        if count > 0 {
            let cb = unsafe { raw_bytes(cp, count as usize) };
            let rb = unsafe { raw_bytes(rp, count as usize) };
            assert_eq!(cb, rb, "allocate_results({count}) contents differ");
            assert!(
                cb.iter().all(|&b| b == 0),
                "allocate_results({count}) must be calloc-zeroed"
            );
            assert_eq!(cb.len(), count as usize * 24, "record stride must be 24");
        }
        unsafe {
            free(cp as *mut _);
            free(rp as *mut _);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 -- perform_computation_with_history: lazy allocation from *history == NULL
// ---------------------------------------------------------------------------
#[test]
fn cfg_11_pcwh_lazy_alloc_all_ops() {
    let l = libs();
    let mut rng = Rng::new();

    for op in [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO] {
        for _ in 0..200 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_c_div_trap(a, b) {
                continue;
            }
            // A deliberately stale count: the NULL branch must reset it to 0.
            let stale = rng.next_i32();

            let mut ch: *mut ComputationResult = ptr::null_mut();
            let mut cc: i32 = stale;
            let cres = unsafe { (l.c.perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };

            let mut rh: *mut ComputationResult = ptr::null_mut();
            let mut rc: i32 = stale;
            let rres =
                unsafe { (l.rust.perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

            let ctx = format!("pcwh(a={a}, b={b}, op={op}, *history=NULL, count={stale})");
            assert_same(cres, rres, &format!("{ctx} return"));
            assert_same(cc, rc, &format!("{ctx} count"));
            assert_eq!(cc, 1, "the NULL branch must reset the count then record once");
            assert!(!ch.is_null() && !rh.is_null(), "{ctx}: buffer not allocated");

            let cs = unsafe { slots(ch, 10) };
            let rs = unsafe { slots(rh, 10) };
            assert_eq!(cs, rs, "{ctx}: 10-slot buffer differs");
            assert_eq!(cs[0].value, cres, "{ctx}: recorded value");
            assert_eq!(cs[0].status, 0, "{ctx}: status must be STATUS_SUCCESS");
            // Slots 1..10 must still be calloc-zeroed.
            assert!(
                cs[1..].iter().all(|s| *s == ComputationResult::default()),
                "{ctx}: untouched slots must stay zeroed"
            );
            unsafe {
                free(ch as *mut _);
                free(rh as *mut _);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 -- caller-provided buffer, every op
// ---------------------------------------------------------------------------
#[test]
fn cfg_12_pcwh_caller_buffer_all_ops() {
    let l = libs();
    let mut rng = Rng::new();

    for op in [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO] {
        for _ in 0..200 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_c_div_trap(a, b) {
                continue;
            }
            let start = (rng.below(10)) as i32; // 0..9 -- inside capacity

            let mut ch = unsafe { alloc_buf(&l.c, 10) };
            let mut cc = start;
            let cres = unsafe { (l.c.perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };

            let mut rh = unsafe { alloc_buf(&l.rust, 10) };
            let mut rc = start;
            let rres =
                unsafe { (l.rust.perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

            let ctx = format!("pcwh(a={a}, b={b}, op={op}, caller buffer, count={start})");
            assert_same(cres, rres, &format!("{ctx} return"));
            assert_same(cc, rc, &format!("{ctx} count"));
            assert_eq!(cc, start + 1, "{ctx}: count must advance by one");
            assert_eq!(
                unsafe { raw_bytes(ch, 10) },
                unsafe { raw_bytes(rh, 10) },
                "{ctx}: buffer bytes differ"
            );
            unsafe {
                free(ch as *mut _);
                free(rh as *mut _);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 -- fill to capacity and past it, comparing the whole array each step
// ---------------------------------------------------------------------------
#[test]
fn cfg_13_pcwh_fill_to_capacity_sequence() {
    let l = libs();

    for trial in 0..40u64 {
        let mut rng = Rng::with_seed(Rng::SEED ^ (trial + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));

        let mut ch: *mut ComputationResult = ptr::null_mut();
        let mut cc: i32 = 0;
        let mut rh: *mut ComputationResult = ptr::null_mut();
        let mut rc: i32 = 0;

        for step in 0..25 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let op = rng.interesting_op();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_c_div_trap(a, b) {
                continue;
            }
            let cres = unsafe { (l.c.perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
            let rres =
                unsafe { (l.rust.perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

            let ctx = format!("trial {trial} step {step}: pcwh(a={a}, b={b}, op={op})");
            assert_same(cres, rres, &format!("{ctx} return"));
            assert_same(cc, rc, &format!("{ctx} count"));
            assert_eq!(
                unsafe { raw_bytes(ch, 10) },
                unsafe { raw_bytes(rh, 10) },
                "{ctx}: 240-byte history buffer differs"
            );
            let expected = std::cmp::min(step as i32 + 1, HISTORY_CAPACITY);
            assert_eq!(cc, expected, "{ctx}: count should saturate at 10");
        }
        assert_eq!(cc, HISTORY_CAPACITY, "trial {trial}: expected saturation");
        unsafe {
            free(ch as *mut _);
            free(rh as *mut _);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 -- *history == NULL while *history_count is stale/non-zero
// ---------------------------------------------------------------------------
#[test]
fn cfg_14_pcwh_null_history_with_stale_count() {
    let l = libs();
    let mut rng = Rng::new();

    for &stale in &[0i32, 1, 5, 9, 10, 11, 99, -1, -7, i32::MAX, i32::MIN] {
        for _ in 0..25 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let op = rng.interesting_op();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_c_div_trap(a, b) {
                continue;
            }
            let mut ch: *mut ComputationResult = ptr::null_mut();
            let mut cc = stale;
            let cres = unsafe { (l.c.perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };

            let mut rh: *mut ComputationResult = ptr::null_mut();
            let mut rc = stale;
            let rres =
                unsafe { (l.rust.perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

            let ctx = format!("pcwh(*history=NULL, stale count={stale}, a={a}, b={b}, op={op})");
            assert_same(cres, rres, &format!("{ctx} return"));
            assert_same(cc, rc, &format!("{ctx} count"));
            assert_eq!(cc, 1, "{ctx}: NULL branch resets the count to 0, then records");
            assert_eq!(
                unsafe { raw_bytes(ch, 10) },
                unsafe { raw_bytes(rh, 10) },
                "{ctx}: buffer differs"
            );
            unsafe {
                free(ch as *mut _);
                free(rh as *mut _);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 -- out-of-range `op` still records (falls back to ADD)
// ---------------------------------------------------------------------------
#[test]
fn cfg_15_pcwh_out_of_range_op_records() {
    let l = libs();
    let mut rng = Rng::new();

    for &op in &[0i32, 6, 7, -1, -100, i32::MIN, i32::MAX] {
        for _ in 0..100 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();

            let mut ch = unsafe { alloc_buf(&l.c, 10) };
            let mut cc = 0;
            let cres = unsafe { (l.c.perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };

            let mut rh = unsafe { alloc_buf(&l.rust, 10) };
            let mut rc = 0;
            let rres =
                unsafe { (l.rust.perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

            let ctx = format!("pcwh(out-of-range op={op}, a={a}, b={b})");
            assert_same(cres, rres, &format!("{ctx} return"));
            assert_same(cc, rc, &format!("{ctx} count"));
            // The default arm is add_operation.
            assert_eq!(cres, a.wrapping_add(b), "{ctx}: default arm must be ADD");
            assert_eq!(
                unsafe { raw_bytes(ch, 10) },
                unsafe { raw_bytes(rh, 10) },
                "{ctx}: buffer differs"
            );
            unsafe {
                free(ch as *mut _);
                free(rh as *mut _);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 -- two independent caller histories interleaved (no cross-talk)
// ---------------------------------------------------------------------------
#[test]
fn cfg_16_pcwh_two_independent_histories() {
    let l = libs();
    let mut rng = Rng::new();

    let mut ch1: *mut ComputationResult = ptr::null_mut();
    let mut cc1 = 0;
    let mut ch2: *mut ComputationResult = ptr::null_mut();
    let mut cc2 = 0;
    let mut rh1: *mut ComputationResult = ptr::null_mut();
    let mut rc1 = 0;
    let mut rh2: *mut ComputationResult = ptr::null_mut();
    let mut rc2 = 0;

    for step in 0..60 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let op = rng.interesting_op();
        if (op == OP_DIVIDE || op == OP_MODULO) && is_c_div_trap(a, b) {
            continue;
        }
        let second = step % 3 == 0;
        let (cres, rres) = if second {
            unsafe {
                (
                    (l.c.perform_computation_with_history)(a, b, op, &mut ch2, &mut cc2),
                    (l.rust.perform_computation_with_history)(a, b, op, &mut rh2, &mut rc2),
                )
            }
        } else {
            unsafe {
                (
                    (l.c.perform_computation_with_history)(a, b, op, &mut ch1, &mut cc1),
                    (l.rust.perform_computation_with_history)(a, b, op, &mut rh1, &mut rc1),
                )
            }
        };
        let ctx = format!("step {step} (history {}): pcwh(a={a}, b={b}, op={op})", second as u8 + 1);
        assert_same(cres, rres, &format!("{ctx} return"));
        assert_same((cc1, cc2), (rc1, rc2), &format!("{ctx} counts"));
        // Either history may still be unallocated early on; the helper compares
        // allocation state first and never dereferences NULL.
        unsafe { assert_buffers_match(ch1, rh1, 10, &format!("{ctx} history 1")) };
        unsafe { assert_buffers_match(ch2, rh2, 10, &format!("{ctx} history 2")) };
    }
    assert!(cc1 > 0 && cc2 > 0, "both histories should have been used");
    unsafe {
        free(ch1 as *mut _);
        free(ch2 as *mut _);
        free(rh1 as *mut _);
        free(rh2 as *mut _);
    }
}

// ---------------------------------------------------------------------------
// Row 25 -- the composed low-level pipeline, driven directly
//           select_operation -> fn ptr -> perform_computation_with_history
// ---------------------------------------------------------------------------
#[test]
fn cfg_25_composed_lowlevel_pipeline() {
    let l = libs();

    for trial in 0..25u64 {
        let mut rng = Rng::with_seed(Rng::SEED ^ (trial + 7).wrapping_mul(0xD6E8_FEB8_6659_FD93));

        // Caller-owned state, exactly as `mathop` keeps it internally.
        let mut ch: *mut ComputationResult = ptr::null_mut();
        let mut cc: i32 = 0;
        let mut rh: *mut ComputationResult = ptr::null_mut();
        let mut rc: i32 = 0;

        for step in 0..30 {
            let op = rng.interesting_op();
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_c_div_trap(a, b) {
                continue;
            }
            let ctx = format!("trial {trial} step {step}: op={op} a={a} b={b}");

            // 1. lowest level: the dispatcher and the raw function pointer.
            let cf = unsafe { (l.c.select_operation)(op) }.expect("C fn ptr");
            let rf = unsafe { (l.rust.select_operation)(op) }.expect("Rust fn ptr");
            let cdirect = unsafe { cf(a, b, rng.next_i32()) };
            let rdirect = unsafe { rf(a, b, rng.next_i32()) };
            assert_same(cdirect, rdirect, &format!("{ctx} direct fn ptr"));

            // 2. the priority helper on the same op.
            assert_same(
                unsafe { (l.c.get_operation_priority)(op) },
                unsafe { (l.rust.get_operation_priority)(op) },
                &format!("{ctx} priority"),
            );

            // 3. the recording layer, chained: feed the previous result forward.
            let cres = unsafe { (l.c.perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
            let rres =
                unsafe { (l.rust.perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };
            assert_same(cres, rres, &format!("{ctx} pcwh return"));
            assert_same(cres, cdirect, &format!("{ctx} pcwh vs direct call"));
            assert_same(cc, rc, &format!("{ctx} count"));
            assert_eq!(
                unsafe { raw_bytes(ch, 10) },
                unsafe { raw_bytes(rh, 10) },
                "{ctx}: history buffer differs"
            );

            // 4. chain the result back in, like mathop's second stage.
            let op2 = rng.interesting_op();
            if !((op2 == OP_DIVIDE || op2 == OP_MODULO) && is_c_div_trap(cres, b)) {
                let c2 =
                    unsafe { (l.c.perform_computation_with_history)(cres, b, op2, &mut ch, &mut cc) };
                let r2 = unsafe {
                    (l.rust.perform_computation_with_history)(rres, b, op2, &mut rh, &mut rc)
                };
                assert_same(c2, r2, &format!("{ctx} chained return (op2={op2})"));
                assert_same(cc, rc, &format!("{ctx} chained count"));
                assert_eq!(
                    unsafe { raw_bytes(ch, 10) },
                    unsafe { raw_bytes(rh, 10) },
                    "{ctx}: chained history buffer differs"
                );
            }
        }
        unsafe {
            free(ch as *mut _);
            free(rh as *mut _);
        }
    }
}
