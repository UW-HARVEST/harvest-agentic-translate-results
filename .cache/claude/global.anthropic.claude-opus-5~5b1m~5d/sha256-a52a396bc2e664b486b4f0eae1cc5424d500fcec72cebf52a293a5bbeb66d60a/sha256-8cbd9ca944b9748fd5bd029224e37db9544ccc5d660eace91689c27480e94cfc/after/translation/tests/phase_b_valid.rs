//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//! Both implementations are reached only through their `.so` exports.

mod common;

use common::{DataRecord, Pair, Rng, RECORD_SIZE};
use std::os::raw::c_int;

const N: usize = 400; // randomized cases per row

// ---------------------------------------------------------------- rows 1 & 2

#[test]
fn row01_add_three() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xA11CE01);
    for _ in 0..N {
        let (a, b, c) = (rng.edgy(), rng.edgy(), rng.edgy());
        assert_eq!(p.c.add_three(a, b, c), p.r.add_three(a, b, c), "add_three({a},{b},{c})");
    }
}

#[test]
fn row02_multiply_add() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xA11CE02);
    for _ in 0..N {
        let (a, b, c) = (rng.edgy(), rng.edgy(), rng.edgy());
        assert_eq!(p.c.multiply_add(a, b, c), p.r.multiply_add(a, b, c), "multiply_add({a},{b},{c})");
    }
}

// ------------------------------------------------------------- rows 3,4,5

#[test]
fn row03_complex_calc_fresh_counter() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xA11CE03);
    for _ in 0..N {
        let (a, b, c) = (rng.edgy(), rng.edgy(), rng.edgy());
        assert_eq!(
            p.c.complex_calc(a, b, c),
            p.r.complex_calc(a, b, c),
            "complex_calc({a},{b},{c}) @counter=0"
        );
    }
}

#[test]
fn row04_row05_complex_calc_with_accumulated_counter() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xA11CE04);
    for step in 0..N {
        // row 5: drive global_counter with randomized increments (wraps)
        let inc = rng.edgy();
        p.c.increment_counter(inc, 999);
        p.r.increment_counter(inc, 999);

        // row 4: observe it through complex_calc
        let (a, b, c) = (rng.edgy(), rng.edgy(), rng.edgy());
        assert_eq!(
            p.c.complex_calc(a, b, c),
            p.r.complex_calc(a, b, c),
            "step {step}: complex_calc({a},{b},{c}) after increments"
        );
    }
}

// ------------------------------------------------------------- rows 6,11,12

#[test]
fn row06_row11_row12_accumulator_and_process_pointer_data() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xA11CE06);

    // row 11: global_accumulator still 0
    for _ in 0..N {
        let mut v = rng.edgy();
        let m = rng.edgy();
        let cr = p.c.process_pointer_data(&mut v as *mut c_int, m);
        let rr = p.r.process_pointer_data(&mut v as *mut c_int, m);
        assert_eq!(cr, rr, "process_pointer_data({v},{m}) @acc=0");
    }

    // rows 6 + 12: accumulate (acc = acc*2 + v, wraps) then observe
    for step in 0..N {
        let av = rng.edgy();
        p.c.update_accumulator(av, 888);
        p.r.update_accumulator(av, 888);

        let mut v = rng.edgy();
        let m = rng.edgy();
        let cr = p.c.process_pointer_data(&mut v as *mut c_int, m);
        let rr = p.r.process_pointer_data(&mut v as *mut c_int, m);
        assert_eq!(cr, rr, "step {step}: process_pointer_data({v},{m}) after acc updates");
    }
}

// ----------------------------------------------------------- rows 7,8,9,10

#[test]
fn row07_row08_apply_operation_pure_ops() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xA11CE07);
    for name in ["add_three", "multiply_add"] {
        let ca = p.c.op3_addr(name);
        let ra = p.r.op3_addr(name);
        for _ in 0..N {
            let (a, b, c) = (rng.edgy(), rng.edgy(), rng.edgy());
            assert_eq!(
                p.c.apply_operation(ca, a, b, c),
                p.r.apply_operation(ra, a, b, c),
                "apply_operation({name},{a},{b},{c})"
            );
        }
    }
}

#[test]
fn row09_apply_operation_state_dependent_callee() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xA11CE09);
    let ca = p.c.op3_addr("complex_calc");
    let ra = p.r.op3_addr("complex_calc");
    for step in 0..N {
        let inc = rng.edgy();
        p.c.increment_counter(inc, 0);
        p.r.increment_counter(inc, 0);
        let (a, b, c) = (rng.edgy(), rng.edgy(), rng.edgy());
        assert_eq!(
            p.c.apply_operation(ca, a, b, c),
            p.r.apply_operation(ra, a, b, c),
            "step {step}: apply_operation(complex_calc,{a},{b},{c})"
        );
    }
}

#[test]
fn row10_apply_operation_through_abi_mismatched_pointer() {
    // Passing a `void(int,int)` through `operation_func` is what the C would do
    // if a caller mismatched the types; the SysV ABI makes the call itself safe
    // (extra register arg ignored, return value = whatever is left in eax).
    // What must match is the *side effect* on global_counter, observed via
    // complex_calc. The raw return value is deliberately not compared.
    let p = Pair::fresh();
    let mut rng = Rng::new(0xA11CE10);
    let ca = p.c.mod2_addr("increment_counter");
    let ra = p.r.mod2_addr("increment_counter");
    for step in 0..N {
        let v = rng.small();
        p.c.apply_operation(ca, v, 0, 0);
        p.r.apply_operation(ra, v, 0, 0);
        assert_eq!(
            p.c.complex_calc(0, 0, 0),
            p.r.complex_calc(0, 0, 0),
            "step {step}: global_counter diverged after transmuted call with v={v}"
        );
    }
}

// -------------------------------------------------------- rows 13..18 shift

fn diff_shift(p: &Pair, data: &[c_int], size: c_int, shift_by: c_int) {
    let mut cbuf = data.to_vec();
    let mut rbuf = data.to_vec();
    p.c.shift_array_data(cbuf.as_mut_ptr(), size, shift_by);
    p.r.shift_array_data(rbuf.as_mut_ptr(), size, shift_by);
    assert_eq!(
        cbuf, rbuf,
        "shift_array_data(size={size}, shift_by={shift_by}) on {:?}",
        data
    );
}

#[test]
fn row13_shift_guard_taken_random() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB13);
    for _ in 0..N {
        let size = rng.range(16, 65);
        let shift_by = rng.range(1, size);
        let data: Vec<c_int> = (0..size).map(|_| rng.edgy()).collect();
        diff_shift(&p, &data, size, shift_by);
    }
}

#[test]
fn row14_row15_row17_shift_boundary_shifts() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB14);
    for _ in 0..N {
        let size = rng.range(2, 65);
        let data: Vec<c_int> = (0..size).map(|_| rng.edgy()).collect();
        diff_shift(&p, &data, size, 1); // row 14 (also row 17 when size==2)
        diff_shift(&p, &data, size, size - 1); // row 15
    }
    // row 17 explicitly
    diff_shift(&p, &[7, -9], 2, 1);
}

#[test]
fn row16_row18_shift_guard_not_taken() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB18);
    // row 16
    diff_shift(&p, &[42], 1, 0);
    for _ in 0..N {
        let size = rng.range(1, 33);
        let data: Vec<c_int> = (0..size.max(1)).map(|_| rng.edgy()).collect();
        for shift_by in [
            0,
            -1,
            -size,
            i32::MIN,
            size,
            size + 1,
            i32::MAX,
        ] {
            diff_shift(&p, &data, size, shift_by);
        }
        // size <= 0 (row 18 tail)
        for bad_size in [0, -1, i32::MIN] {
            for shift_by in [-1, 0, 1, i32::MAX] {
                diff_shift(&p, &data, bad_size, shift_by);
            }
        }
    }
}

// ------------------------------------------------------ rows 19..22 dyn mem

#[test]
fn row19_compute_count8() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB19);
    for _ in 0..N {
        let base = rng.edgy();
        assert_eq!(
            p.c.compute_with_dynamic_memory(base, 8),
            p.r.compute_with_dynamic_memory(base, 8),
            "compute_with_dynamic_memory({base}, 8)"
        );
    }
}

#[test]
fn row20_compute_count1() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB20);
    for _ in 0..N {
        let base = rng.edgy();
        assert_eq!(
            p.c.compute_with_dynamic_memory(base, 1),
            p.r.compute_with_dynamic_memory(base, 1),
            "compute_with_dynamic_memory({base}, 1)"
        );
    }
}

#[test]
fn row21_compute_random_counts() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB21);
    for _ in 0..N {
        let base = rng.edgy();
        let count = rng.range(2, 257);
        assert_eq!(
            p.c.compute_with_dynamic_memory(base, count),
            p.r.compute_with_dynamic_memory(base, count),
            "compute_with_dynamic_memory({base}, {count})"
        );
    }
}

#[test]
fn row22_compute_count0() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB22);
    for _ in 0..N {
        let base = rng.edgy();
        assert_eq!(
            p.c.compute_with_dynamic_memory(base, 0),
            p.r.compute_with_dynamic_memory(base, 0),
            "compute_with_dynamic_memory({base}, 0)"
        );
    }
}

// ----------------------------------------------------- rows 23..25 time val

#[test]
fn row23_time_based_small_seed() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB23);
    for _ in 0..N {
        let seed = rng.range(-596523, 596523);
        assert_eq!(
            p.c.get_time_based_value(seed),
            p.r.get_time_based_value(seed),
            "get_time_based_value({seed})"
        );
    }
}

#[test]
fn row24_time_based_overflow_region() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB24);
    let mut seeds: Vec<i32> = vec![
        596523,
        596524,
        -596524,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1_000_000,
        -1_000_000,
        2_000_000_000,
        -2_000_000_000,
    ];
    for _ in 0..N {
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

#[test]
fn row25_time_based_zero_and_negative() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB25);
    assert_eq!(p.c.get_time_based_value(0), p.r.get_time_based_value(0));
    for _ in 0..N {
        let seed = -(rng.range(1, 100_000));
        assert_eq!(
            p.c.get_time_based_value(seed),
            p.r.get_time_based_value(seed),
            "get_time_based_value({seed})"
        );
    }
}

// -------------------------------------------------- rows 26..31, 37 records

fn rand_records(rng: &mut Rng, n: usize, garbage: bool) -> Vec<DataRecord> {
    (0..n)
        .map(|i| {
            let mut r = DataRecord::zeroed();
            r.value = rng.edgy();
            if garbage {
                r.id = rng.i32();
                r.timestamp = rng.next_u64() as i64;
                for b in r.name.iter_mut() {
                    *b = (rng.next_u64() & 0xff) as u8;
                }
            } else {
                r.id = i as c_int;
                r.timestamp = 0;
                let s = format!("Record_{i}");
                r.name[..s.len()].copy_from_slice(s.as_bytes());
            }
            r
        })
        .collect()
}

/// Compares both the return value and the full byte image of the record buffer.
/// A `slack` of extra records is appended (identical in both buffers) so that the
/// out-of-bounds reads the C performs for negative `shift` land on identical,
/// deterministic memory in both processes rather than on unrelated heap data.
fn diff_records(
    p: &Pair,
    recs: &[DataRecord],
    num_records: c_int,
    shift: c_int,
    slack: usize,
    sentinel: &mut Rng,
) {
    let mut base = recs.to_vec();
    for _ in 0..slack {
        let mut r = DataRecord::zeroed();
        r.value = sentinel.edgy();
        r.id = -1;
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
}

#[test]
fn row26_records_guard_taken_random() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB26);
    for _ in 0..N {
        let n = rng.range(2, 9);
        let shift = rng.range(1, n);
        let recs = rand_records(&mut rng, n as usize, false);
        diff_records(&p, &recs, n, shift, 4, &mut rng);
    }
}

#[test]
fn row27_row28_records_boundary_shifts() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB27);
    for _ in 0..N {
        let n = rng.range(2, 13);
        let recs = rand_records(&mut rng, n as usize, false);
        diff_records(&p, &recs, n, 1, 4, &mut rng); // row 27
        diff_records(&p, &recs, n, n - 1, 4, &mut rng); // row 28
    }
}

#[test]
fn row29_records_shift_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB29);
    for _ in 0..N {
        let n = rng.range(1, 13);
        let recs = rand_records(&mut rng, n as usize, false);
        diff_records(&p, &recs, n, 0, 4, &mut rng);
    }
}

#[test]
fn row30_records_hatch_shape() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB30);
    for _ in 0..N {
        let recs = rand_records(&mut rng, 5, false);
        diff_records(&p, &recs, 5, 2, 4, &mut rng);
    }
}

#[test]
fn row31_row37_records_all_fields_and_abi() {
    assert_eq!(std::mem::size_of::<DataRecord>(), RECORD_SIZE);
    assert_eq!(std::mem::align_of::<DataRecord>(), 8);
    // offsets 0 / 4 / 8 / 16
    let r = DataRecord::zeroed();
    let b = &r as *const DataRecord as usize;
    assert_eq!(&r.id as *const _ as usize - b, 0);
    assert_eq!(&r.value as *const _ as usize - b, 4);
    assert_eq!(&r.timestamp as *const _ as usize - b, 8);
    assert_eq!(&r.name as *const _ as usize - b, 16);

    // row 31: fully randomized garbage in every field; the 48-byte memmove must
    // relocate id/timestamp/name too, so the buffer image comparison is the check.
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB31);
    for _ in 0..N {
        let n = rng.range(2, 9);
        let shift = rng.range(1, n);
        let recs = rand_records(&mut rng, n as usize, true);
        diff_records(&p, &recs, n, shift, 4, &mut rng);
    }
}

// ---------------------------------------------------------- rows 32..36 hatch

#[test]
fn row32_hatch_fresh_state_each_case() {
    let mut rng = Rng::new(0xB32);
    // 60 fresh library pairs (dlopen is the expensive part); each sees exactly
    // one hatch call with pristine globals.
    for _ in 0..60 {
        let p = Pair::fresh();
        let (a, b, c, d) = (rng.edgy(), rng.edgy(), rng.edgy(), rng.edgy());
        assert_eq!(p.c.hatch(a, b, c, d), p.r.hatch(a, b, c, d), "hatch({a},{b},{c},{d}) fresh");
    }
}

#[test]
fn row33_hatch_repeated_sequence() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB33);
    for step in 0..N {
        let (a, b, c, d) = (rng.small(), rng.small(), rng.small(), rng.small());
        assert_eq!(
            p.c.hatch(a, b, c, d),
            p.r.hatch(a, b, c, d),
            "step {step}: hatch({a},{b},{c},{d}) with carried-over globals"
        );
    }
}

#[test]
fn row34_hatch_boundary_params() {
    let vals = [0i32, 1, -1, 2, -2, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    // fresh pair per case so a divergence is attributable to the params alone
    for &a in &vals {
        for &b in &vals {
            let p = Pair::fresh();
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

#[test]
fn row35_hatch_long_wrapping_sequence() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB35);
    for step in 0..250 {
        let (a, b, c, d) = (rng.edgy(), rng.edgy(), rng.edgy(), rng.edgy());
        assert_eq!(
            p.c.hatch(a, b, c, d),
            p.r.hatch(a, b, c, d),
            "step {step}: hatch({a},{b},{c},{d}) long wrapping sequence"
        );
    }
}

#[test]
fn row36_mixed_pipeline() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xB36);
    let cc = p.c.op3_addr("complex_calc");
    let rc = p.r.op3_addr("complex_calc");
    for step in 0..600 {
        match rng.next_u64() % 7 {
            0 => {
                let v = rng.edgy();
                p.c.increment_counter(v, 999);
                p.r.increment_counter(v, 999);
            }
            1 => {
                let v = rng.edgy();
                p.c.update_accumulator(v, 888);
                p.r.update_accumulator(v, 888);
            }
            2 => {
                let (a, b, c, d) = (rng.edgy(), rng.edgy(), rng.edgy(), rng.edgy());
                assert_eq!(p.c.hatch(a, b, c, d), p.r.hatch(a, b, c, d), "step {step}: hatch");
            }
            3 => {
                let (a, b, c) = (rng.edgy(), rng.edgy(), rng.edgy());
                assert_eq!(
                    p.c.complex_calc(a, b, c),
                    p.r.complex_calc(a, b, c),
                    "step {step}: complex_calc"
                );
            }
            4 => {
                let mut v = rng.edgy();
                let m = rng.edgy();
                assert_eq!(
                    p.c.process_pointer_data(&mut v, m),
                    p.r.process_pointer_data(&mut v, m),
                    "step {step}: process_pointer_data"
                );
            }
            5 => {
                let (a, b, c) = (rng.edgy(), rng.edgy(), rng.edgy());
                assert_eq!(
                    p.c.apply_operation(cc, a, b, c),
                    p.r.apply_operation(rc, a, b, c),
                    "step {step}: apply_operation"
                );
            }
            _ => {
                let base = rng.edgy();
                let count = rng.range(0, 64);
                assert_eq!(
                    p.c.compute_with_dynamic_memory(base, count),
                    p.r.compute_with_dynamic_memory(base, count),
                    "step {step}: compute_with_dynamic_memory"
                );
            }
        }
    }
}
