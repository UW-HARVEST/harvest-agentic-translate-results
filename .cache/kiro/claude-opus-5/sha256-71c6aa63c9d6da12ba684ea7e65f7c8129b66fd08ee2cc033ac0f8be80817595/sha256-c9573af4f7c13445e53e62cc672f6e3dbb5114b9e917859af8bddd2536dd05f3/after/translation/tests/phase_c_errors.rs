// Phase C — one differential test per row of ERRORS.md, plus the generic FFI
// boundary cases (null pointers, zero/oversized lengths, one-step-past-range and
// out-of-range "enum" ints crossing the FFI boundary).
//
// Each case asserts the SAME rejection on both sides: the same sentinel /
// returned NULL / unmodified state / emitted diagnostic — not merely "both
// failed somehow".
//
// `harness = false`: these cases compare the exact bytes written to fd 1.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

fn main() {
    let mut t = Runner::new();
    // rows 1-4: get_operation
    t.case("err01_get_operation_negative", err01_get_operation_negative);
    t.case("err02_get_operation_too_large", err02_get_operation_too_large);
    t.case("err03_get_operation_op_macros", err03_get_operation_op_macros);
    t.case("err04_get_operation_boundary", err04_get_operation_boundary);
    t.case("err04b_get_operation_exhaustive_fuzz", err04b_get_operation_exhaustive_fuzz);
    // rows 5-7: execute_operation
    t.case("err05_execute_operation_null_func", err05_execute_operation_null_func);
    t.case(
        "err06_execute_operation_null_func_null_name",
        err06_execute_operation_null_func_null_name,
    );
    t.case(
        "err07_execute_operation_null_name_success",
        err07_execute_operation_null_name_success,
    );
    // rows 8-13: compute_checksum
    t.case("err08_compute_checksum_null_values", err08_compute_checksum_null_values);
    t.case("err09_compute_checksum_zero_count", err09_compute_checksum_zero_count);
    t.case("err10_compute_checksum_negative_count", err10_compute_checksum_negative_count);
    t.case("err11_compute_checksum_null_and_bad_count", err11_compute_checksum_null_and_bad_count);
    t.case("err12_compute_checksum_oversized_count", err12_compute_checksum_oversized_count);
    t.case("err13_compute_checksum_mask_bound", err13_compute_checksum_mask_bound);
    // row 14: init_state
    t.case("err14_init_state_null", err14_init_state_null);
    // rows 15-17: apply_operation
    t.case("err15_apply_operation_null_state", err15_apply_operation_null_state);
    t.case("err16_apply_operation_null_func", err16_apply_operation_null_func);
    t.case("err17_apply_operation_both_null", err17_apply_operation_both_null);
    t.case("err_misaligned_values_pointer", err_misaligned_values_pointer);
    // generic FFI boundary sweep
    t.case("errX_null_and_range_sweep", err_x_null_and_range_sweep);
    t.finish();
}

const NULL_FN: *const c_void = std::ptr::null();

/// Compare a `get_operation` result on both sides.
fn diff_get_operation(opcode: c_int, expect_null: bool) {
    let (c, r) = both();
    let (cp, rp) = serial(|| {
        // get_operation prints nothing; capture anyway so a regression that
        // *starts* printing is caught.
        let (cp, co) = capture_stdout(|| unsafe { (c.get_operation)(opcode) });
        let (rp, ro) = capture_stdout(|| unsafe { (r.get_operation)(opcode) });
        assert_same_output(&co, &ro, &format!("get_operation({opcode})"));
        assert!(co.is_empty(), "get_operation({opcode}) printed {:?}", show(&co));
        (cp, rp)
    });
    assert_eq!(
        cp.is_null(),
        rp.is_null(),
        "get_operation({opcode}): C returned {}, Rust returned {}",
        if cp.is_null() { "NULL" } else { "non-NULL" },
        if rp.is_null() { "NULL" } else { "non-NULL" }
    );
    assert_eq!(
        cp.is_null(),
        expect_null,
        "get_operation({opcode}): C returned {}, ERRORS.md expects {}",
        if cp.is_null() { "NULL" } else { "non-NULL" },
        if expect_null { "NULL" } else { "non-NULL" }
    );
}

// --- row 1 --------------------------------------------------------------------
fn err01_get_operation_negative() {
    for op in [-1, -2, -3, -4, -100, -0xABCD, i32::MIN + 1, i32::MIN] {
        diff_get_operation(op, true);
    }
}

// --- row 2 --------------------------------------------------------------------
fn err02_get_operation_too_large() {
    for op in [4, 5, 6, 7, 8, 100, 0xFFFF, 0x1_0000, i32::MAX - 1, i32::MAX] {
        diff_get_operation(op, true);
    }
}

// --- row 3: the OP_* macros run 1..4, but the table is indexed 0..3, so the
// highest documented opcode (OP_SHIFT == 4) is itself out of range.
fn err03_get_operation_op_macros() {
    const OP_ADD: c_int = 0x01;
    const OP_MULTIPLY: c_int = 0x02;
    const OP_XOR: c_int = 0x03;
    const OP_SHIFT: c_int = 0x04;
    diff_get_operation(OP_ADD, false);
    diff_get_operation(OP_MULTIPLY, false);
    diff_get_operation(OP_XOR, false);
    diff_get_operation(OP_SHIFT, true); // == 4: rejected
}

// --- row 4: one step past each end -------------------------------------------
fn err04_get_operation_boundary() {
    diff_get_operation(-1, true);
    diff_get_operation(0, false);
    diff_get_operation(3, false);
    diff_get_operation(4, true);
}

/// An out-of-range int arriving where a 0..3 opcode is expected is a real input
/// (C has no enum checking at the ABI boundary); sweep a wide random set.
fn err04b_get_operation_exhaustive_fuzz() {
    let (c, r) = both();
    let mut rng = Rng::new(0x04B0_0004);
    for _ in 0..200_000 {
        let op = rng.next_i32();
        let (cp, rp) = unsafe { ((c.get_operation)(op), (r.get_operation)(op)) };
        assert_eq!(cp.is_null(), rp.is_null(), "get_operation({op}) nullness diverged");
        assert_eq!(
            cp.is_null(),
            !(0..4).contains(&op),
            "get_operation({op}): unexpected nullness in C"
        );
    }
    for op in -16..=16 {
        let (cp, rp) = unsafe { ((c.get_operation)(op), (r.get_operation)(op)) };
        assert_eq!(cp.is_null(), rp.is_null(), "get_operation({op}) nullness diverged");
    }
}

// --- rows 5-7: execute_operation ---------------------------------------------
fn err05_execute_operation_null_func() {
    let (c, r) = both();
    for name_s in ["XOR", "SHIFT", "", "a very long operation name ".repeat(20).as_str()] {
        let name = cstring(name_s);
        let (cv, co, rv, ro) = serial(|| {
            let (cv, co) =
                capture_stdout(|| unsafe { (c.execute_operation)(NULL_FN, 7, 9, name.as_ptr()) });
            let (rv, ro) =
                capture_stdout(|| unsafe { (r.execute_operation)(NULL_FN, 7, 9, name.as_ptr()) });
            (cv, co, rv, ro)
        });
        assert_eq!(cv, 0, "C execute_operation(NULL, ...) must return the 0 sentinel");
        assert_eq!(rv, cv, "execute_operation(NULL, ..., {name_s:?}): C={cv} Rust={rv}");
        assert_same_output(&co, &ro, &format!("execute_operation(NULL, ..., {name_s:?})"));
        let expected = format!("Error: Operation function pointer is NULL for {name_s}\n");
        assert_eq!(
            String::from_utf8_lossy(&co),
            expected,
            "unexpected C diagnostic for op_name {name_s:?}"
        );
        // The "Variable a/b" log lines must NOT appear: `func` is never called.
        assert!(!co.windows(10).any(|w| w == b"Variable a"), "C logged before the NULL check");
        assert!(!ro.windows(10).any(|w| w == b"Variable a"), "Rust logged before the NULL check");
    }
    // Randomized (a, b) must not change the sentinel.
    let name = cstring("XOR");
    let mut rng = Rng::new(0x0500_0005);
    serial(|| {
        for _ in 0..2_000 {
            let (a, b) = (rng.next_i32(), rng.next_i32());
            let (v, _) = capture_stdout(|| unsafe {
                (
                    (c.execute_operation)(NULL_FN, a, b, name.as_ptr()),
                    (r.execute_operation)(NULL_FN, a, b, name.as_ptr()),
                )
            });
            assert_eq!(v.0, 0);
            assert_eq!(v.1, 0, "Rust execute_operation(NULL, {a}, {b}) returned {}", v.1);
        }
    });
}

fn err06_execute_operation_null_func_null_name() {
    let (c, r) = both();
    let (cv, co, rv, ro) = serial(|| {
        let (cv, co) = capture_stdout(|| unsafe {
            (c.execute_operation)(NULL_FN, -1, -2, std::ptr::null())
        });
        let (rv, ro) = capture_stdout(|| unsafe {
            (r.execute_operation)(NULL_FN, -1, -2, std::ptr::null())
        });
        (cv, co, rv, ro)
    });
    assert_eq!(cv, 0);
    assert_eq!(rv, cv, "execute_operation(NULL, .., NULL name): C={cv} Rust={rv}");
    assert_same_output(&co, &ro, "execute_operation(NULL func, NULL op_name)");
    // glibc renders a null `%s` as "(null)"; both libraries go through the same
    // printf, so the rendering must be identical rather than merely present.
    assert_eq!(
        String::from_utf8_lossy(&co),
        "Error: Operation function pointer is NULL for (null)\n"
    );
}

fn err07_execute_operation_null_name_success() {
    let (c, r) = both();
    for op in 0..4 {
        let (cv, co, rv, ro) = serial(|| {
            let (cv, co) = capture_stdout(|| unsafe {
                let f = (c.get_operation)(op);
                (c.execute_operation)(f, 1234, -5678, std::ptr::null())
            });
            let (rv, ro) = capture_stdout(|| unsafe {
                let f = (r.get_operation)(op);
                (r.execute_operation)(f, 1234, -5678, std::ptr::null())
            });
            (cv, co, rv, ro)
        });
        assert_eq!(rv, cv, "execute_operation(op {op}, NULL name): C={cv} Rust={rv}");
        assert_same_output(&co, &ro, &format!("execute_operation(op {op}, NULL op_name)"));
        let s = String::from_utf8_lossy(&co);
        assert!(s.contains("Result of (null): "), "unexpected C output: {s:?}");
    }
}

// --- rows 8-13: compute_checksum ---------------------------------------------
fn diff_checksum_raw(values: *mut c_int, count: c_int, ctx: &str) -> u32 {
    let (c, r) = both();
    let (cs, rs) = unsafe { ((c.compute_checksum)(values, count), (r.compute_checksum)(values, count)) };
    assert_eq!(cs, rs, "compute_checksum({ctx}, {count}): C=0x{cs:08X} Rust=0x{rs:08X}");
    cs
}

fn err08_compute_checksum_null_values() {
    for &count in &[1i32, 2, 3, 4, 5, 16, 1_000, i32::MAX] {
        let v = diff_checksum_raw(std::ptr::null_mut(), count, "NULL");
        assert_eq!(v, 0, "compute_checksum(NULL, {count}) must return 0, got 0x{v:08X}");
    }
}

fn err09_compute_checksum_zero_count() {
    let mut buf: Vec<c_int> = vec![0x0102_0304, 0x0506_0708, -1, i32::MIN];
    let v = diff_checksum_raw(buf.as_mut_ptr(), 0, "valid buffer");
    assert_eq!(v, 0, "compute_checksum(buf, 0) must return 0, got 0x{v:08X}");
}

fn err10_compute_checksum_negative_count() {
    let mut buf: Vec<c_int> = vec![0x0102_0304, 0x0506_0708, -1, i32::MIN];
    for &count in &[-1i32, -2, -4, -5, -100, i32::MIN + 1, i32::MIN] {
        let v = diff_checksum_raw(buf.as_mut_ptr(), count, "valid buffer");
        assert_eq!(v, 0, "compute_checksum(buf, {count}) must return 0, got 0x{v:08X}");
    }
}

fn err11_compute_checksum_null_and_bad_count() {
    for &count in &[0i32, -1, -4, i32::MIN] {
        let v = diff_checksum_raw(std::ptr::null_mut(), count, "NULL");
        assert_eq!(v, 0, "compute_checksum(NULL, {count}) must return 0, got 0x{v:08X}");
    }
}

/// Oversized length is silently clamped rather than rejected; the Rust must clamp
/// identically instead of reading out of bounds or returning an error.
fn err12_compute_checksum_oversized_count() {
    let mut rng = Rng::new(0x1200_0012);
    for _ in 0..500 {
        let mut buf: Vec<c_int> = (0..4).map(|_| rng.next_i32_biased()).collect();
        let base = diff_checksum_raw(buf.as_mut_ptr(), 4, "4-int buffer");
        for &count in &[5i32, 6, 7, 8, 16, 64, 1_000, 0x7FFF_FFFE, i32::MAX] {
            let v = diff_checksum_raw(buf.as_mut_ptr(), count, "4-int buffer");
            assert_eq!(v, base, "count={count} was not clamped to 4 (got 0x{v:04X}, want 0x{base:04X})");
        }
    }
}

fn err13_compute_checksum_mask_bound() {
    let mut rng = Rng::new(0x1300_0013);
    for _ in 0..20_000 {
        let mut buf: Vec<c_int> = (0..4).map(|_| rng.next_i32()).collect();
        let count = (rng.next_u32() % 11) as c_int - 3; // -3 ..= 7
        let v = diff_checksum_raw(buf.as_mut_ptr(), count, "random buffer");
        assert!(v <= 0xFFFF, "result 0x{v:08X} exceeds MASK_LOWER");
    }
}

// --- row 14: init_state -------------------------------------------------------

/// The C `compute_checksum` reaches its `values` argument only through `memcpy`,
/// which imposes no alignment requirement, so a misaligned `int*` arriving across
/// the FFI boundary is handled rather than rejected. The Rust must read the bytes
/// the same way (and must not assume 4-byte alignment).
fn err_misaligned_values_pointer() {
    let (c, r) = both();
    // 16 payload bytes at every possible offset within an over-aligned buffer.
    let mut raw = [0u8; 32];
    let mut rng = Rng::new(0xA11C_0000);
    for _ in 0..2_000 {
        for b in raw.iter_mut() {
            *b = (rng.next_u32() & 0xFF) as u8;
        }
        for offset in 0..4usize {
            let p = unsafe { raw.as_mut_ptr().add(offset) } as *mut c_int;
            for count in 1..=4i32 {
                let cs = unsafe { (c.compute_checksum)(p, count) };
                let rs = unsafe { (r.compute_checksum)(p, count) };
                assert_eq!(
                    cs, rs,
                    "compute_checksum(misaligned+{offset}, {count}): C=0x{cs:04X} Rust=0x{rs:04X}"
                );
            }
        }
    }
}

fn err14_init_state_null() {
    let (c, r) = both();
    for &v in &[0i32, 1, -1, i32::MAX, i32::MIN, 0xABCD] {
        let (co, ro) = serial(|| {
            let (_, co) = capture_stdout(|| unsafe { (c.init_state)(std::ptr::null_mut(), v) });
            let (_, ro) = capture_stdout(|| unsafe { (r.init_state)(std::ptr::null_mut(), v) });
            (co, ro)
        });
        assert_same_output(&co, &ro, &format!("init_state(NULL, {v})"));
        assert_eq!(
            String::from_utf8_lossy(&co),
            "Error: state pointer is NULL in init_state\n",
            "unexpected diagnostic for init_state(NULL, {v})"
        );
    }
}

// --- rows 15-17: apply_operation ---------------------------------------------
fn err15_apply_operation_null_state() {
    let (c, r) = both();
    for op in 0..4 {
        let (co, ro) = serial(|| {
            let (_, co) = capture_stdout(|| unsafe {
                let f = (c.get_operation)(op);
                (c.apply_operation)(std::ptr::null_mut(), 42, f);
            });
            let (_, ro) = capture_stdout(|| unsafe {
                let f = (r.get_operation)(op);
                (r.apply_operation)(std::ptr::null_mut(), 42, f);
            });
            (co, ro)
        });
        assert_same_output(&co, &ro, &format!("apply_operation(NULL, 42, op {op})"));
        assert_eq!(
            String::from_utf8_lossy(&co),
            "Error: state pointer is NULL in apply_operation\n"
        );
    }
}

/// The state must be left completely untouched — in particular `operation_count`
/// is NOT incremented when the function pointer is NULL.
fn err16_apply_operation_null_func() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1600_0016);
    serial(|| {
        for _ in 0..500 {
            let initial = rng.next_i32_biased();
            let value = rng.next_i32_biased();
            let run = |api: &Api| {
                let mut s = StateBuf::poisoned();
                unsafe {
                    (api.init_state)(s.as_mut_ptr(), initial);
                    (api.apply_operation)(s.as_mut_ptr(), value, NULL_FN);
                }
                s
            };
            let (cs, co) = capture_stdout(|| run(c));
            let (rs, ro) = capture_stdout(|| run(r));
            assert_eq!(cs, rs, "apply_operation(state, {value}, NULL):\n C: {cs:?}\n R: {rs:?}");
            assert_eq!(cs.accumulator(), initial, "accumulator changed despite NULL func");
            assert_eq!(cs.operation_count(), 0, "operation_count incremented despite NULL func");
            assert_eq!(cs.checksum(), 0);
            assert_same_output(&co, &ro, "init_state + apply_operation(NULL func)");
            assert!(
                String::from_utf8_lossy(&co)
                    .ends_with("Error: operation function pointer is NULL in apply_operation\n"),
                "unexpected C output: {:?}",
                show(&co)
            );
        }
    });
}

/// Check order: `state` is tested before `func`, so with both NULL only the
/// *state* diagnostic is emitted.
fn err17_apply_operation_both_null() {
    let (c, r) = both();
    let (co, ro) = serial(|| {
        let (_, co) =
            capture_stdout(|| unsafe { (c.apply_operation)(std::ptr::null_mut(), 1, NULL_FN) });
        let (_, ro) =
            capture_stdout(|| unsafe { (r.apply_operation)(std::ptr::null_mut(), 1, NULL_FN) });
        (co, ro)
    });
    assert_same_output(&co, &ro, "apply_operation(NULL, 1, NULL)");
    assert_eq!(
        String::from_utf8_lossy(&co),
        "Error: state pointer is NULL in apply_operation\n",
        "the state check must win over the func check"
    );
}

// --- generic boundary sweep ---------------------------------------------------
/// Every pointer parameter × NULL, every length × {negative, 0, oversized}, and
/// every opcode-shaped int × out-of-range, in one randomized sweep, so that a
/// combination not spelled out above still gets exercised.
fn err_x_null_and_range_sweep() {
    let (c, r) = both();
    let mut rng = Rng::new(0xF000_000F);
    let names = [Some(cstring("op")), Some(cstring("")), None];
    serial(|| {
        for _ in 0..4_000 {
            let op = match rng.next_u32() % 3 {
                0 => (rng.next_u32() % 4) as c_int, // valid
                1 => (rng.next_u32() % 9) as c_int - 4, // straddles both ends
                _ => rng.next_i32(),                // anything
            };
            let a = rng.next_i32_biased();
            let b = rng.next_i32_biased();
            let count = match rng.next_u32() % 3 {
                0 => (rng.next_u32() % 5) as c_int,
                1 => (rng.next_u32() % 17) as c_int - 8,
                _ => rng.next_i32(),
            };
            let null_state = rng.next_u32() % 3 == 0;
            let null_values = rng.next_u32() % 3 == 0;
            let name = &names[(rng.next_u32() % 3) as usize];
            let name_ptr =
                name.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null());

            let run = |api: &Api| {
                let mut s = StateBuf::poisoned();
                let mut vals: [c_int; 4] = [a, b, a ^ b, a.wrapping_add(b)];
                unsafe {
                    let f = (api.get_operation)(op);
                    let sp = if null_state { std::ptr::null_mut() } else { s.as_mut_ptr() };
                    (api.init_state)(sp, a);
                    (api.apply_operation)(sp, b, f);
                    let ev = (api.execute_operation)(f, a, b, name_ptr);
                    let vp =
                        if null_values { std::ptr::null_mut() } else { vals.as_mut_ptr() };
                    let cs = (api.compute_checksum)(vp, count);
                    (s, ev, cs, f.is_null())
                }
            };
            let ((cs, cev, ccs, cnull), co) = capture_stdout(|| run(c));
            let ((rs, rev, rcs, rnull), ro) = capture_stdout(|| run(r));

            let ctx = format!(
                "op={op} a={a} b={b} count={count} null_state={null_state} \
                 null_values={null_values} name={:?}",
                name.as_ref().map(|_| "str")
            );
            assert_eq!(cnull, rnull, "get_operation nullness diverged [{ctx}]");
            assert_eq!(cev, rev, "execute_operation diverged [{ctx}]: C={cev} Rust={rev}");
            assert_eq!(ccs, rcs, "compute_checksum diverged [{ctx}]: C={ccs} Rust={rcs}");
            assert_eq!(cs, rs, "state diverged [{ctx}]:\n C: {cs:?}\n R: {rs:?}");
            assert_same_output(&co, &ro, &ctx);
        }
    });
}
