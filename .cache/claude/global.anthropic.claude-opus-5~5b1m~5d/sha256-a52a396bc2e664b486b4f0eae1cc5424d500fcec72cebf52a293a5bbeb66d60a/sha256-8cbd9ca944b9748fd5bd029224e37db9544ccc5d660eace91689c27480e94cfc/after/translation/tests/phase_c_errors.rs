//! Phase C — error/rejection-path differential tests, one test per ERRORS.md row.
//!
//! The C library has no error codes or sentinels, so "same error" means:
//!   * for guard branches: the same no-op / same returned value, byte-identical buffer;
//!   * for the unchecked-pointer paths: death by the *same signal*, verified by
//!     re-exec'ing this binary as a child (`crash_child`).

mod common;

use common::{DataRecord, Pair, Rng};
use std::os::raw::{c_int, c_void};

// =================================================================== helpers

fn diff_shift_noop(p: &Pair, data: &[c_int], size: c_int, shift_by: c_int) {
    let mut cbuf = data.to_vec();
    let mut rbuf = data.to_vec();
    p.c.shift_array_data(cbuf.as_mut_ptr(), size, shift_by);
    p.r.shift_array_data(rbuf.as_mut_ptr(), size, shift_by);
    assert_eq!(cbuf, rbuf, "C/Rust differ: shift_array_data(size={size}, shift_by={shift_by})");
    assert_eq!(cbuf, data, "expected no-op: shift_array_data(size={size}, shift_by={shift_by})");
}

fn diff_shift(p: &Pair, data: &[c_int], size: c_int, shift_by: c_int) {
    let mut cbuf = data.to_vec();
    let mut rbuf = data.to_vec();
    p.c.shift_array_data(cbuf.as_mut_ptr(), size, shift_by);
    p.r.shift_array_data(rbuf.as_mut_ptr(), size, shift_by);
    assert_eq!(cbuf, rbuf, "shift_array_data(size={size}, shift_by={shift_by})");
}

/// `slack` identical trailing records make the C's out-of-bounds reads (negative
/// `shift`) land on deterministic, identical memory in both buffers.
fn diff_records_slack(
    p: &Pair,
    values: &[c_int],
    num_records: c_int,
    shift: c_int,
    slack: usize,
    rng: &mut Rng,
) -> c_int {
    let mut base: Vec<DataRecord> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let mut r = DataRecord::zeroed();
            r.id = i as c_int;
            r.value = v;
            r
        })
        .collect();
    for _ in 0..slack {
        let mut r = DataRecord::zeroed();
        r.id = -1;
        r.value = rng.edgy();
        base.push(r);
    }
    let mut cbuf = base.clone();
    let mut rbuf = base.clone();
    let cr = p.c.manipulate_records(cbuf.as_mut_ptr(), num_records, shift);
    let rr = p.r.manipulate_records(rbuf.as_mut_ptr(), num_records, shift);
    assert_eq!(cr, rr, "manipulate_records(num={num_records}, shift={shift}) return");
    assert_eq!(
        common::as_bytes(&cbuf),
        common::as_bytes(&rbuf),
        "manipulate_records(num={num_records}, shift={shift}) buffer image"
    );
    cr
}

// ============================================ rows 1-7: shift_array_data guard

#[test]
fn err01_shift_by_zero_is_noop() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC01);
    for _ in 0..200 {
        let size = rng.range(1, 33);
        let data: Vec<c_int> = (0..size).map(|_| rng.edgy()).collect();
        diff_shift_noop(&p, &data, size, 0);
    }
}

#[test]
fn err02_shift_by_negative_is_noop() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC02);
    for _ in 0..200 {
        let size = rng.range(1, 33);
        let data: Vec<c_int> = (0..size).map(|_| rng.edgy()).collect();
        for sb in [-1, -size, i32::MIN, i32::MIN + 1, -(rng.range(1, 1000))] {
            diff_shift_noop(&p, &data, size, sb);
        }
    }
}

#[test]
fn err03_shift_by_equals_size_is_noop() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC03);
    for _ in 0..200 {
        let size = rng.range(1, 33);
        let data: Vec<c_int> = (0..size).map(|_| rng.edgy()).collect();
        diff_shift_noop(&p, &data, size, size);
    }
}

#[test]
fn err04_shift_by_greater_than_size_is_noop() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC04);
    for _ in 0..200 {
        let size = rng.range(1, 33);
        let data: Vec<c_int> = (0..size).map(|_| rng.edgy()).collect();
        for sb in [size + 1, size + 1000, i32::MAX, i32::MAX - 1] {
            diff_shift_noop(&p, &data, size, sb);
        }
    }
}

#[test]
fn err05_shift_size_nonpositive_is_noop() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC05);
    for _ in 0..200 {
        let data: Vec<c_int> = (0..8).map(|_| rng.edgy()).collect();
        for size in [0, -1, -8, i32::MIN, i32::MIN + 1] {
            for sb in [i32::MIN, -1, 0, 1, 8, i32::MAX] {
                diff_shift_noop(&p, &data, size, sb);
            }
        }
    }
}

#[test]
fn err06_shift_by_size_minus_one_last_accepted() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC06);
    for _ in 0..200 {
        let size = rng.range(2, 33);
        let data: Vec<c_int> = (0..size).map(|_| rng.edgy()).collect();
        let mut cbuf = data.clone();
        p.c.shift_array_data(cbuf.as_mut_ptr(), size, size - 1);
        // documented C behaviour: 1 element moved down, the rest zeroed
        let mut expect = vec![0; size as usize];
        expect[0] = data[(size - 1) as usize];
        assert_eq!(cbuf, expect, "C behaviour changed for shift_by == size-1");
        diff_shift(&p, &data, size, size - 1);
    }
}

#[test]
fn err07_shift_by_one_first_accepted() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC07);
    for _ in 0..200 {
        let size = rng.range(2, 33);
        let data: Vec<c_int> = (0..size).map(|_| rng.edgy()).collect();
        diff_shift(&p, &data, size, 1);
    }
}

// ======================================= rows 8-14: manipulate_records guard

#[test]
fn err08_records_shift_zero_no_memmove() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC08);
    for _ in 0..200 {
        let n = rng.range(1, 9);
        let vals: Vec<c_int> = (0..n).map(|_| rng.small()).collect();
        let got = diff_records_slack(&p, &vals, n, 0, 4, &mut rng);
        let expect: i32 = vals.iter().fold(0i32, |a, &b| a.wrapping_add(b));
        assert_eq!(got, expect, "shift==0 must be a plain sum");
    }
}

#[test]
fn err09_records_shift_negative_reads_past_end() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC09);
    for _ in 0..300 {
        let n = rng.range(1, 6);
        let vals: Vec<c_int> = (0..n).map(|_| rng.small()).collect();
        // slack >= |shift| so the OOB reads stay inside our own buffer
        for shift in [-1, -2, -3, -4] {
            diff_records_slack(&p, &vals, n, shift, 8, &mut rng);
        }
    }
}

#[test]
fn err10_records_shift_equals_num_returns_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC10);
    for _ in 0..200 {
        let n = rng.range(1, 9);
        let vals: Vec<c_int> = (0..n).map(|_| rng.edgy()).collect();
        let got = diff_records_slack(&p, &vals, n, n, 4, &mut rng);
        assert_eq!(got, 0, "shift == num_records must return 0");
    }
}

#[test]
fn err11_records_shift_greater_than_num_returns_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC11);
    for _ in 0..200 {
        let n = rng.range(1, 9);
        let vals: Vec<c_int> = (0..n).map(|_| rng.edgy()).collect();
        for shift in [n + 1, n + 100, i32::MAX, i32::MAX - 1] {
            let got = diff_records_slack(&p, &vals, n, shift, 4, &mut rng);
            assert_eq!(got, 0, "shift > num_records must return 0 (shift={shift})");
        }
    }
}

#[test]
fn err12_records_zero_length() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC12);
    for _ in 0..50 {
        let got = diff_records_slack(&p, &[], 0, 0, 4, &mut rng);
        assert_eq!(got, 0);
    }
}

#[test]
fn err13_err14_records_negative_num() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC13);
    for _ in 0..200 {
        let vals: Vec<c_int> = (0..4).map(|_| rng.edgy()).collect();
        for num in [-1, -4, i32::MIN, i32::MIN + 1] {
            // any shift >= num; also shift values that would need the (unreachable)
            // 0 < shift < num_records guard
            for shift in [num, num + 1, 0, 1, i32::MAX, -1] {
                // The C loop bound is `num_records - shift` evaluated in `int`,
                // i.e. it WRAPS. Skip combinations whose wrapped bound would run
                // past the slack we allocated (those are pure OOB reads of
                // unrelated heap memory, covered by the crash-parity tests).
                let bound = num.wrapping_sub(shift);
                if bound > 4 {
                    continue;
                }
                let got = diff_records_slack(&p, &vals, num, shift, 4, &mut rng);
                if bound <= 0 {
                    assert_eq!(got, 0, "num={num} shift={shift} must return 0");
                }
            }
        }
    }
}

// ================================= rows 15-18: compute_with_dynamic_memory

#[test]
fn err15_compute_count_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC15);
    for _ in 0..200 {
        let base = rng.edgy();
        let cr = p.c.compute_with_dynamic_memory(base, 0);
        let rr = p.r.compute_with_dynamic_memory(base, 0);
        assert_eq!(cr, rr, "count=0 base={base}");
        assert_eq!(cr, 0, "count=0 must return 0");
    }
}

#[test]
fn err16_compute_count_negative() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC16);
    let mut counts = vec![-1, -2, -8, i32::MIN, i32::MIN + 1, -1_000_000];
    for _ in 0..100 {
        counts.push(-(rng.range(1, i32::MAX)));
    }
    for count in counts {
        let base = rng.edgy();
        let cr = p.c.compute_with_dynamic_memory(base, count);
        let rr = p.r.compute_with_dynamic_memory(base, count);
        assert_eq!(cr, rr, "count={count} base={base}");
        assert_eq!(cr, 0, "negative count must return 0 (count={count})");
    }
}

#[test]
fn err17_compute_count_one() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC17);
    for _ in 0..200 {
        let base = rng.edgy();
        let cr = p.c.compute_with_dynamic_memory(base, 1);
        let rr = p.r.compute_with_dynamic_memory(base, 1);
        assert_eq!(cr, rr, "count=1 base={base}");
        assert_eq!(cr, base, "count=1 must return base");
    }
}

#[test]
fn err18_compute_overflowing_base() {
    let p = Pair::fresh();
    for base in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, i32::MAX - 20, i32::MIN + 20] {
        for count in [1, 2, 8, 100, 1000] {
            assert_eq!(
                p.c.compute_with_dynamic_memory(base, count),
                p.r.compute_with_dynamic_memory(base, count),
                "compute_with_dynamic_memory({base}, {count})"
            );
        }
    }
}

// ==================================== rows 19-20: get_time_based_value overflow

#[test]
fn err19_err20_time_seed_overflow() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC19);
    let mut seeds = vec![
        596523,
        596524,
        596525,
        -596523,
        -596524,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
    ];
    for _ in 0..300 {
        seeds.push(rng.i32());
    }
    for seed in seeds {
        assert_eq!(
            p.c.get_time_based_value(seed),
            p.r.get_time_based_value(seed),
            "get_time_based_value({seed})"
        );
    }
}

// ==================================== rows 21-23: arithmetic overflow surface

#[test]
fn err21_pure_op_overflow() {
    let p = Pair::fresh();
    let vals = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                assert_eq!(p.c.add_three(a, b, c), p.r.add_three(a, b, c), "add_three");
                assert_eq!(p.c.multiply_add(a, b, c), p.r.multiply_add(a, b, c), "multiply_add");
                assert_eq!(p.c.complex_calc(a, b, c), p.r.complex_calc(a, b, c), "complex_calc");
            }
        }
    }
}

#[test]
fn err22_global_overflow() {
    let p = Pair::fresh();
    // push global_counter past INT_MAX, and global_accumulator through its
    // doubling wrap, checking after every single step.
    for &v in &[i32::MAX, i32::MAX, 1, i32::MIN, -1, i32::MAX, 7] {
        p.c.increment_counter(v, 999);
        p.r.increment_counter(v, 999);
        assert_eq!(
            p.c.complex_calc(0, 0, 0),
            p.r.complex_calc(0, 0, 0),
            "global_counter diverged after += {v}"
        );

        p.c.update_accumulator(v, 888);
        p.r.update_accumulator(v, 888);
        let mut one: c_int = 1;
        assert_eq!(
            p.c.process_pointer_data(&mut one, 0),
            p.r.process_pointer_data(&mut one, 0),
            "global_accumulator diverged after update {v}"
        );
    }
}

#[test]
fn err23_hatch_extremes() {
    let vals = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &vals {
        let p = Pair::fresh();
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    assert_eq!(
                        p.c.hatch(a, b, c, d),
                        p.r.hatch(a, b, c, d),
                        "hatch({a},{b},{c},{d})"
                    );
                }
            }
        }
    }
}

// ============================ rows 27-28: null pointer with guard NOT taken

#[test]
fn err27_shift_null_pointer_guard_not_taken() {
    let p = Pair::fresh();
    // The guard short-circuits before any dereference, so passing NULL is safe
    // in the C — and must be equally safe (no panic, no UB trap) in Rust.
    for (size, shift_by) in [
        (10, 0),
        (10, -1),
        (10, 10),
        (10, 11),
        (0, 0),
        (0, 5),
        (-1, 5),
        (i32::MIN, i32::MAX),
        (0, i32::MAX),
    ] {
        p.c.shift_array_data(std::ptr::null_mut(), size, shift_by);
        p.r.shift_array_data(std::ptr::null_mut(), size, shift_by);
    }
}

#[test]
fn err28_records_null_pointer_no_iterations() {
    let p = Pair::fresh();
    for (num, shift) in [(0, 0), (0, 1), (-1, 0), (-1, 5), (i32::MIN, 0), (3, 3), (3, 9)] {
        let cr = p.c.manipulate_records(std::ptr::null_mut(), num, shift);
        let rr = p.r.manipulate_records(std::ptr::null_mut(), num, shift);
        assert_eq!(cr, rr, "manipulate_records(NULL, {num}, {shift})");
        assert_eq!(cr, 0, "manipulate_records(NULL, {num}, {shift}) must return 0");
    }
}

// =========== row 29: "out-of-range enum" degenerates to arbitrary int (no enums)

#[test]
fn err29_no_enums_arbitrary_ints_accepted_identically() {
    // c_src/include/lib.h declares only `int hatch(int,int,int,int)`; there is no
    // enum anywhere in the C API, so every int bit pattern is in-range input.
    // This sweeps values with no "valid variant" meaning across every entry point.
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC29);
    for _ in 0..300 {
        let v = rng.i32();
        let mut cell: c_int = v;
        assert_eq!(p.c.add_three(v, v, v), p.r.add_three(v, v, v));
        assert_eq!(p.c.multiply_add(v, v, v), p.r.multiply_add(v, v, v));
        assert_eq!(p.c.complex_calc(v, v, v), p.r.complex_calc(v, v, v));
        assert_eq!(
            p.c.process_pointer_data(&mut cell, v),
            p.r.process_pointer_data(&mut cell, v)
        );
        assert_eq!(p.c.get_time_based_value(v), p.r.get_time_based_value(v));
        let count = (v.rem_euclid(64)) as c_int;
        assert_eq!(
            p.c.compute_with_dynamic_memory(v, count),
            p.r.compute_with_dynamic_memory(v, count)
        );
    }
}

// ===================== rows 24, 25, 26, 30: matching fatal signals (subprocess)

/// Bodies that must crash. Run in a child process; the parent compares signals.
fn crash_body(case: &str, which: &str) {
    let p = Pair::shared();
    let imp = if which == "c" { &p.c } else { &p.r };
    match case {
        // row 24: call through a NULL operation_func
        "24" => {
            let r = imp.apply_operation(std::ptr::null::<c_void>(), 1, 2, 3);
            println!("no crash: {r}");
        }
        // row 25: dereference a NULL int*
        "25" => {
            let r = imp.process_pointer_data(std::ptr::null_mut(), 3);
            println!("no crash: {r}");
        }
        // row 26: memmove from NULL with the guard satisfied
        "26" => {
            imp.shift_array_data(std::ptr::null_mut(), 10, 3);
            println!("no crash");
        }
        // row 30: oversized length — guard passes, memmove/memset run off the buffer
        "30" => {
            let mut buf = vec![0 as c_int; 8];
            imp.shift_array_data(buf.as_mut_ptr(), i32::MAX, i32::MAX - 1);
            println!("no crash: {}", buf[0]);
        }
        other => panic!("unknown crash case {other}"),
    }
}

#[test]
#[ignore = "child-process helper for the crash-parity tests"]
fn crash_child() {
    let case = std::env::var("HATCH_CRASH_CASE").expect("HATCH_CRASH_CASE unset");
    let which = std::env::var("HATCH_CRASH_IMPL").expect("HATCH_CRASH_IMPL unset");
    crash_body(&case, &which);
}

fn run_crash_child(case: &str, which: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "crash_child", "--ignored", "--nocapture", "--test-threads=1"])
        .env("HATCH_CRASH_CASE", case)
        .env("HATCH_CRASH_IMPL", which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to spawn crash child");
    (out.status.code(), out.status.signal())
}

fn assert_same_crash(case: &str) {
    let (c_code, c_sig) = run_crash_child(case, "c");
    let (r_code, r_sig) = run_crash_child(case, "rust");
    assert_eq!(
        c_sig, r_sig,
        "row {case}: C died with signal {c_sig:?} (exit {c_code:?}) but Rust with {r_sig:?} (exit {r_code:?})"
    );
    if c_sig.is_none() {
        // Neither crashed: they must at least agree on the exit status.
        assert_eq!(c_code, r_code, "row {case}: exit code mismatch");
    } else {
        assert_eq!(c_sig, Some(11), "row {case}: expected SIGSEGV, got {c_sig:?}");
    }
}

#[test]
fn err24_apply_operation_null_fn_pointer_same_signal() {
    assert_same_crash("24");
}

#[test]
fn err25_process_pointer_data_null_same_signal() {
    assert_same_crash("25");
}

#[test]
fn err26_shift_array_data_null_guard_taken_same_signal() {
    assert_same_crash("26");
}

#[test]
fn err30_shift_array_data_oversized_length_same_signal() {
    assert_same_crash("30");
}
