// Phase C — error-path differential tests.
// One test per row of ERRORS.md (E1 .. E21) plus the generic FFI boundaries
// (G1 .. G6). Each constructs the exact rejecting input and asserts that BOTH
// libraries return the same sentinel / fallback, not merely that both "failed".

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// E1 / E2 / E3 — is_valid_operation rejections.
// ---------------------------------------------------------------------------

fn diff_is_valid(v: i8, expect_false: bool) {
    let (c, r) = both();
    let cv = unsafe { (c.is_valid_operation)(v as c_char) };
    let rv = unsafe { (r.is_valid_operation)(v as c_char) };
    assert_eq!(cv, rv, "is_valid_operation({v}): C={cv} Rust={rv}");
    if expect_false {
        assert_eq!(cv, 0, "is_valid_operation({v}) must reject");
    } else {
        assert_eq!(cv, 1, "is_valid_operation({v}) must accept");
    }
}

#[test]
fn e1_is_valid_zero() {
    diff_is_valid(0, true);
}

#[test]
fn e2_is_valid_below_range() {
    for v in 1i8..=47 {
        diff_is_valid(v, true);
    }
    for v in i8::MIN..=-1 {
        diff_is_valid(v, true);
    }
    // The accepted window, for contrast.
    for v in 49i8..=53 {
        diff_is_valid(v, false);
    }
    diff_is_valid(48, true); // '0', one step below '1'
}

#[test]
fn e3_is_valid_above_range() {
    diff_is_valid(54, true); // '6', one step above '5'
    for v in 54i8..=127 {
        diff_is_valid(v, true);
    }
}

// ---------------------------------------------------------------------------
// E4 / E5 — divide- and modulo-by-zero guards.
// ---------------------------------------------------------------------------

#[test]
fn e4_divide_by_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE4);
    let mut vals: Vec<i32> = BOUNDARY.to_vec();
    for _ in 0..512 {
        vals.push(rng.spicy_i32());
    }
    for &a in &vals {
        for &u in &[0i32, 1, -1, i32::MIN, i32::MAX] {
            let cv = unsafe { (c.divide_operation)(a, 0, u) };
            let rv = unsafe { (r.divide_operation)(a, 0, u) };
            assert_eq!(cv, rv, "divide_operation({a}, 0, {u})");
            assert_eq!(cv, 0, "the C guard returns 0, not a trap");
        }
    }
}

#[test]
fn e5_modulo_by_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE5);
    let mut vals: Vec<i32> = BOUNDARY.to_vec();
    for _ in 0..512 {
        vals.push(rng.spicy_i32());
    }
    for &a in &vals {
        for &u in &[0i32, 1, -1, i32::MIN, i32::MAX] {
            let cv = unsafe { (c.modulo_operation)(a, 0, u) };
            let rv = unsafe { (r.modulo_operation)(a, 0, u) };
            assert_eq!(cv, rv, "modulo_operation({a}, 0, {u})");
            assert_eq!(cv, 0, "the C guard returns 0, not a trap");
        }
    }
}

// ---------------------------------------------------------------------------
// E6 / G1 / G4 — out-of-range `Operation` enum values over the FFI boundary.
// ---------------------------------------------------------------------------

#[test]
fn e6_select_operation_out_of_range_enum() {
    let (c, r) = both();
    let mut invalid: Vec<i32> = vec![0, 6, 7, -1, -2, -5, 100, -100, i32::MIN, i32::MAX];
    let mut rng = Rng::new(0xE6);
    for _ in 0..1024 {
        let v = rng.spicy_i32();
        if !(1..=5).contains(&v) {
            invalid.push(v);
        }
    }
    for &op in &invalid {
        let ca = unsafe { (c.select_operation)(op) };
        let ra = unsafe { (r.select_operation)(op) };
        let ci = c
            .identify_op(ca)
            .unwrap_or_else(|| panic!("C select_operation({op}) -> unknown {ca:#x}"));
        let ri = r
            .identify_op(ra)
            .unwrap_or_else(|| panic!("Rust select_operation({op}) -> unknown {ra:#x}"));
        assert_eq!(ci, ri, "select_operation({op}): C picked #{ci}, Rust #{ri}");
        assert_eq!(ci, 0, "the `default:` arm must yield add_operation");
        // …and the fallback really behaves like addition in both.
        let cf = c.op_by_index(ci);
        let rf = r.op_by_index(ri);
        let cv = unsafe { cf(7, 11, 0) };
        let rv = unsafe { rf(7, 11, 0) };
        assert_eq!((cv, rv), (18, 18), "default arm must add");
    }
}

// ---------------------------------------------------------------------------
// E7 / E8 / E9 / G3 — allocate_results failures.
// ---------------------------------------------------------------------------

#[test]
fn e7_allocate_negative_count() {
    let (c, r) = both();
    for &count in &[-1i32, -2, -10, -1000, i32::MIN, i32::MIN + 1, -715827883] {
        let cp = unsafe { (c.allocate_results)(count) };
        let rp = unsafe { (r.allocate_results)(count) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "allocate_results({count}): C null={} Rust null={}",
            cp.is_null(),
            rp.is_null()
        );
        assert!(
            cp.is_null(),
            "a negative count sign-extends to a huge size_t, so calloc must fail"
        );
        unsafe {
            libc_free(cp);
            libc_free(rp);
        }
    }
}

#[test]
fn e8_allocate_huge_count() {
    let (c, r) = both();
    for &count in &[i32::MAX, i32::MAX - 1, 1_000_000_000, 178_956_971] {
        let cp = unsafe { (c.allocate_results)(count) };
        unsafe { libc_free(cp) };
        let rp = unsafe { (r.allocate_results)(count) };
        unsafe { libc_free(rp) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "allocate_results({count}): C null={} Rust null={} \
             (both must go through the same libc calloc)",
            cp.is_null(),
            rp.is_null()
        );
    }
}

#[test]
fn e9_allocate_zero_count() {
    let (c, r) = both();
    let cp = unsafe { (c.allocate_results)(0) };
    let rp = unsafe { (r.allocate_results)(0) };
    assert_eq!(
        cp.is_null(),
        rp.is_null(),
        "allocate_results(0): C null={} Rust null={}",
        cp.is_null(),
        rp.is_null()
    );
    assert!(!cp.is_null(), "glibc calloc(0, 24) returns a unique block");
    unsafe {
        libc_free(cp);
        libc_free(rp);
    }
}

// ---------------------------------------------------------------------------
// E10 — *history == NULL discards the caller's history_count.
// ---------------------------------------------------------------------------

#[test]
fn e10_history_null_resets_count() {
    let (c, r) = both();
    for &count in &[1i32, 5, 9, 10, 11, 12345, -1, -10, i32::MIN, i32::MAX] {
        let mut ch: *mut ComputationResult = std::ptr::null_mut();
        let mut rh: *mut ComputationResult = std::ptr::null_mut();
        let mut cn = count;
        let mut rn = count;
        let cv = unsafe { (c.perform_computation_with_history)(3, 4, OP_ADD, &mut ch, &mut cn) };
        let rv = unsafe { (r.perform_computation_with_history)(3, 4, OP_ADD, &mut rh, &mut rn) };
        assert_eq!(cv, rv, "count={count}: return");
        assert_eq!(cn, rn, "count={count}: history_count");
        assert_eq!(cn, 1, "count={count}: must be reset to 0 then incremented");
        assert!(!ch.is_null() && !rh.is_null());
        unsafe {
            assert_eq!((*ch).value, (*rh).value);
            assert_eq!((*ch).value, 7);
            assert_eq!((*ch).status, (*rh).status);
            assert_eq!((*ch).timestamp, (*rh).timestamp);
            libc_free(ch);
            libc_free(rh);
        }
    }
}

// ---------------------------------------------------------------------------
// E11 / G5 — history full: silent drop, exactly at and past the limit.
// ---------------------------------------------------------------------------

#[test]
fn e11_history_full_silent_drop() {
    let (c, r) = both();
    // 32 slots so an (incorrect) write anywhere in range would be visible.
    const N: usize = 32;
    let sentinel = ComputationResult {
        value: 0x5A5A_5A5A,
        timestamp: 0x1234_5678_9ABC_DEF0,
        status: STATUS_ERROR,
    };
    for &count in &[9i32, 10, 11, 12, 31, 1000, i32::MAX] {
        let mut cbuf = [sentinel; N];
        let mut rbuf = [sentinel; N];
        let mut ch = cbuf.as_mut_ptr();
        let mut rh = rbuf.as_mut_ptr();
        let mut cn = count;
        let mut rn = count;
        let cv = unsafe { (c.perform_computation_with_history)(100, 7, OP_SUBTRACT, &mut ch, &mut cn) };
        let rv = unsafe { (r.perform_computation_with_history)(100, 7, OP_SUBTRACT, &mut rh, &mut rn) };
        assert_eq!(cv, rv, "count={count}: return value still computed");
        assert_eq!(cv, 93, "count={count}: the computation itself is unaffected");
        assert_eq!(cn, rn, "count={count}: history_count");
        if count < 10 {
            assert_eq!(cn, count + 1, "count={count}: below the limit -> recorded");
            assert_eq!(cbuf[count as usize].value, 93);
            assert_eq!(rbuf[count as usize].value, 93);
        } else {
            assert_eq!(cn, count, "count={count}: at/over the limit -> not recorded");
            assert!(
                cbuf.iter().all(|&x| x == sentinel),
                "count={count}: C must not write when full"
            );
            assert!(
                rbuf.iter().all(|&x| x == sentinel),
                "count={count}: Rust must not write when full"
            );
        }
        assert_eq!(cbuf, rbuf, "count={count}: buffers must be identical");
    }
}

// ---------------------------------------------------------------------------
// E12 — negative history_count writes out of range (reproduced, not fixed).
// ---------------------------------------------------------------------------

#[test]
fn e12_history_negative_count_writes_oob() {
    let (c, r) = both();
    const PAD: usize = 8;
    const N: usize = 24;
    for &count in &[-1i32, -2, -3, -8] {
        let mut cbuf = [ComputationResult::default(); N];
        let mut rbuf = [ComputationResult::default(); N];
        for i in 0..N {
            cbuf[i] = ComputationResult {
                value: i as c_int,
                timestamp: 77,
                status: STATUS_WARNING,
            };
            rbuf[i] = cbuf[i];
        }
        let mut ch = unsafe { cbuf.as_mut_ptr().add(PAD) };
        let mut rh = unsafe { rbuf.as_mut_ptr().add(PAD) };
        let mut cn = count;
        let mut rn = count;
        let cv = unsafe { (c.perform_computation_with_history)(9, 9, OP_MULTIPLY, &mut ch, &mut cn) };
        let rv = unsafe { (r.perform_computation_with_history)(9, 9, OP_MULTIPLY, &mut rh, &mut rn) };
        assert_eq!(cv, rv);
        assert_eq!(cv, 81);
        assert_eq!(cn, rn, "count={count}");
        assert_eq!(cn, count + 1, "count={count}: increments toward 0");
        let idx = (PAD as i32 + count) as usize;
        assert_eq!(cbuf[idx].value, 81, "C wrote at index {count}");
        assert_eq!(rbuf[idx].value, 81, "Rust wrote at index {count}");
        assert_eq!(cbuf, rbuf, "count={count}: identical memory effects");
    }
}

// ---------------------------------------------------------------------------
// E13 — out-of-range op inside the recorder.
// ---------------------------------------------------------------------------

#[test]
fn e13_history_out_of_range_op() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE13);
    for &op in &[0i32, 6, 7, -1, -3, 42, i32::MIN, i32::MAX] {
        for _ in 0..32 {
            let a = rng.spicy_i32();
            let b = rng.spicy_i32();
            let mut cbuf = [ComputationResult::default(); 10];
            let mut rbuf = [ComputationResult::default(); 10];
            let mut ch = cbuf.as_mut_ptr();
            let mut rh = rbuf.as_mut_ptr();
            let mut cn = 0;
            let mut rn = 0;
            let cv = unsafe { (c.perform_computation_with_history)(a, b, op, &mut ch, &mut cn) };
            let rv = unsafe { (r.perform_computation_with_history)(a, b, op, &mut rh, &mut rn) };
            assert_eq!(cv, rv, "op={op} ({a},{b})");
            assert_eq!(
                cv,
                a.wrapping_add(b),
                "op={op}: the default arm must add ({a},{b})"
            );
            assert_eq!(cn, rn);
            assert_eq!(cbuf, rbuf, "op={op} ({a},{b}): records");
        }
    }
}

// ---------------------------------------------------------------------------
// E14 — div/mod by zero recorded through the recorder.
// ---------------------------------------------------------------------------

#[test]
fn e14_history_div_mod_by_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE14);
    for op in [OP_DIVIDE, OP_MODULO] {
        for _ in 0..64 {
            let a = rng.spicy_i32();
            let mut cbuf = [ComputationResult::default(); 10];
            let mut rbuf = [ComputationResult::default(); 10];
            let mut ch = cbuf.as_mut_ptr();
            let mut rh = rbuf.as_mut_ptr();
            let mut cn = 0;
            let mut rn = 0;
            let cv = unsafe { (c.perform_computation_with_history)(a, 0, op, &mut ch, &mut cn) };
            let rv = unsafe { (r.perform_computation_with_history)(a, 0, op, &mut rh, &mut rn) };
            assert_eq!(cv, rv, "op={op} a={a}");
            assert_eq!(cv, 0, "the guard's 0 is what gets recorded");
            assert_eq!(cn, 1);
            assert_eq!(rn, 1);
            assert_eq!(cbuf[0].value, 0);
            assert_eq!(rbuf[0].value, 0);
            assert_eq!(cbuf[0].status, STATUS_SUCCESS);
            assert_eq!(rbuf[0].status, STATUS_SUCCESS);
            assert_eq!(cbuf, rbuf, "op={op} a={a}");
        }
    }
}

// ---------------------------------------------------------------------------
// E20 — `history == NULL` (null out-parameter). `*history` is dereferenced with
// no null check, so both libraries must fault the *same* way. Compared in
// forked children so the test process survives.
// ---------------------------------------------------------------------------

#[test]
fn e20_null_history_pointer_faults_identically() {
    let (c, r) = both();
    let cf = c.perform_computation_with_history;
    let rf = r.perform_computation_with_history;

    let c_out = run_in_child(|| {
        let mut n: c_int = 0;
        let v = unsafe { cf(1, 2, OP_ADD, std::ptr::null_mut(), &mut n) };
        (v & 0x7f) as c_int // only reached if no fault
    });
    let r_out = run_in_child(|| {
        let mut n: c_int = 0;
        let v = unsafe { rf(1, 2, OP_ADD, std::ptr::null_mut(), &mut n) };
        (v & 0x7f) as c_int
    });
    assert_eq!(
        c_out, r_out,
        "history == NULL: C {c_out:?} vs Rust {r_out:?}"
    );
    assert!(
        matches!(c_out, Outcome::Signaled(SIGSEGV | SIGBUS)),
        "the C reference is expected to fault on the null deref, got {c_out:?}"
    );

    // Same for a null `history_count` with a valid, non-NULL `*history`:
    // `*history_count` is read by the `< 10` test.
    let block = unsafe { (c.allocate_results)(10) };
    assert!(!block.is_null());
    let c_out = run_in_child(|| {
        let mut h = block;
        let v = unsafe { cf(1, 2, OP_ADD, &mut h, std::ptr::null_mut()) };
        (v & 0x7f) as c_int
    });
    let r_out = run_in_child(|| {
        let mut h = block;
        let v = unsafe { rf(1, 2, OP_ADD, &mut h, std::ptr::null_mut()) };
        (v & 0x7f) as c_int
    });
    assert_eq!(
        c_out, r_out,
        "history_count == NULL: C {c_out:?} vs Rust {r_out:?}"
    );
    unsafe { libc_free(block) };
}

// ---------------------------------------------------------------------------
// G7 — a misaligned `*history`. The C code does an unaligned store (which x86
// performs happily); the Rust must produce byte-identical memory rather than
// tripping an alignment check.
// ---------------------------------------------------------------------------

#[test]
fn g7_misaligned_history_buffer() {
    let (c, r) = both();
    for skew in 1usize..8 {
        let mut cbuf = vec![0xEEu8; 24 * 12 + 8];
        let mut rbuf = vec![0xEEu8; 24 * 12 + 8];
        let mut ch = unsafe { cbuf.as_mut_ptr().add(skew) } as *mut ComputationResult;
        let mut rh = unsafe { rbuf.as_mut_ptr().add(skew) } as *mut ComputationResult;
        let mut cn: c_int = 0;
        let mut rn: c_int = 0;
        for step in 0..3 {
            let cv = unsafe { (c.perform_computation_with_history)(step, 5, OP_ADD, &mut ch, &mut cn) };
            let rv = unsafe { (r.perform_computation_with_history)(step, 5, OP_ADD, &mut rh, &mut rn) };
            assert_eq!(cv, rv, "skew={skew} step={step}");
        }
        assert_eq!(cn, rn, "skew={skew}: history_count");
        assert_eq!(cn, 3);
        assert_eq!(cbuf, rbuf, "skew={skew}: byte-identical memory effects");
        assert!(
            cbuf.iter().any(|&b| b != 0xEE),
            "skew={skew}: something must have been written"
        );
    }
}

// ---------------------------------------------------------------------------
// E21 — INT_MIN / -1 and INT_MIN % -1 are signed-overflow *undefined
// behaviour* in C. This test pins down what each library actually does, so the
// one and only divergence is measured rather than assumed. The C reference has
// no defined result here (gcc emits `idiv`, the CPU raises SIGFPE), so there is
// nothing for the Rust to be byte-identical to; every input with defined C
// behaviour is covered by phase_b_pure.rs.
// ---------------------------------------------------------------------------

#[test]
fn e21_int_min_div_minus_one_documented() {
    let (c, r) = both();
    for (name, cf, rf) in [
        ("divide_operation", c.divide_operation, r.divide_operation),
        ("modulo_operation", c.modulo_operation, r.modulo_operation),
    ] {
        let c_out = run_in_child(|| {
            let v = unsafe { cf(i32::MIN, -1, 0) };
            if v == 0 {
                1
            } else {
                2
            }
        });
        let r_out = run_in_child(|| {
            let v = unsafe { rf(i32::MIN, -1, 0) };
            if v == 0 {
                1
            } else {
                2
            }
        });
        // Documented, deliberate divergence on UB input.
        assert!(
            matches!(c_out, Outcome::Signaled(SIGFPE)),
            "{name}(INT_MIN, -1): the C reference build traps; got {c_out:?}"
        );
        assert!(
            matches!(r_out, Outcome::Exited(_)),
            "{name}(INT_MIN, -1): Rust returns a wrapped value; got {r_out:?}"
        );
    }

    // The neighbours of the UB point *are* defined and must match exactly.
    for &(a, b) in &[
        (i32::MIN, 1),
        (i32::MIN + 1, -1),
        (i32::MIN, -2),
        (i32::MAX, -1),
        (-1, -1),
        (0, -1),
    ] {
        let cd = unsafe { (c.divide_operation)(a, b, 0) };
        let rd = unsafe { (r.divide_operation)(a, b, 0) };
        assert_eq!(cd, rd, "divide_operation({a},{b})");
        let cm = unsafe { (c.modulo_operation)(a, b, 0) };
        let rm = unsafe { (r.modulo_operation)(a, b, 0) };
        assert_eq!(cm, rm, "modulo_operation({a},{b})");
    }
}
