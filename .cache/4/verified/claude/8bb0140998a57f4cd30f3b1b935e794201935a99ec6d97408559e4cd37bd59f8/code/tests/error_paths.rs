//! Phase C — error/rejection-path differential tests.
//!
//! One block per non-fatal `ERRORS.md` row. The five rows whose C behaviour is
//! a fatal signal (E11, E12, E31, E32, E33) live in `tests/crash_parity.rs`,
//! which compares the terminating signal in a forked child.

mod common;
use common::*;

const GUARD_I32: i32 = 0x5A5A_5A5A;
const GUARD_REC_VALUE: i32 = 0x3C3C_3C3C;

#[test]
fn phase_c_nonfatal_error_rows() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 0xC0FF_EE);
    let mut cov = Coverage::new();

    // =====================================================================
    // shift_array_data — every way the L67 guard rejects (must be a no-op)
    // =====================================================================
    // Asserts the buffer is byte-identical afterwards in BOTH libraries.
    let expect_noop = |h: &H, rng: &mut Rng, size: i32, shift_by: i32, label: &str| {
        let len = 24usize;
        let data: Vec<i32> = (0..len).map(|_| rng.spicy_i32()).collect();
        let got = h.shift_array_data(&data, size, shift_by, 8);
        let mut expect = data.clone();
        expect.extend(std::iter::repeat(GUARD_I32).take(8));
        assert_eq!(
            got, expect,
            "{label}: shift_array_data(size={size}, shift_by={shift_by}) must be a no-op"
        );
    };

    // E1: shift_by == 0
    cov.hit("E1");
    for _ in 0..64 {
        expect_noop(&h, &mut rng, 24, 0, "E1");
        expect_noop(&h, &mut rng, 1, 0, "E1");
        expect_noop(&h, &mut rng, 0, 0, "E1");
    }

    // E2: shift_by < 0
    cov.hit("E2");
    for _ in 0..64 {
        for sb in [-1, -2, -24, -1000, -100_000] {
            expect_noop(&h, &mut rng, 24, sb, "E2");
        }
    }

    // E3: shift_by == INT_MIN
    cov.hit("E3");
    for _ in 0..16 {
        expect_noop(&h, &mut rng, 24, i32::MIN, "E3");
        expect_noop(&h, &mut rng, 24, i32::MIN + 1, "E3");
    }

    // E4: shift_by == size
    cov.hit("E4");
    for size in [0i32, 1, 2, 3, 24] {
        for _ in 0..16 {
            expect_noop(&h, &mut rng, size, size, "E4");
        }
    }

    // E5: shift_by == size + 1 (one step past the valid range) and beyond
    cov.hit("E5");
    for size in [0i32, 1, 2, 24] {
        for _ in 0..16 {
            expect_noop(&h, &mut rng, size, size + 1, "E5");
            expect_noop(&h, &mut rng, size, size + 1000, "E5");
        }
    }

    // E6: shift_by == INT_MAX with size < INT_MAX
    cov.hit("E6");
    for _ in 0..16 {
        expect_noop(&h, &mut rng, 24, i32::MAX, "E6");
        expect_noop(&h, &mut rng, 24, i32::MAX - 1, "E6");
    }

    // E7: size == 0 (zero length) for every shift_by
    cov.hit("E7");
    for &sb in EDGE.iter() {
        expect_noop(&h, &mut rng, 0, sb, "E7");
    }

    // E8: size < 0 (negative length)
    cov.hit("E8");
    for &size in &[-1i32, -4, -24, -1000, i32::MIN, i32::MIN + 1] {
        for &sb in EDGE.iter() {
            expect_noop(&h, &mut rng, size, sb, "E8");
        }
    }

    // E9: size == 1 — no shift_by can satisfy 0 < shift_by < 1
    cov.hit("E9");
    for &sb in EDGE.iter() {
        expect_noop(&h, &mut rng, 1, sb, "E9");
    }
    for _ in 0..64 {
        let sb = rng.next_i32();
        expect_noop(&h, &mut rng, 1, sb, "E9");
    }

    // E10: arr == NULL while the guard is false -> NULL is never dereferenced,
    //      both libraries must return normally.
    cov.hit("E10");
    for &(size, sb) in &[
        (10i32, 0i32),
        (10, -1),
        (10, 10),
        (10, 11),
        (10, i32::MIN),
        (10, i32::MAX),
        (0, 1),
        (0, 0),
        (-5, 1),
        (1, 1),
        (i32::MIN, 1),
    ] {
        unsafe { (h.c.shift_array_data)(std::ptr::null_mut(), size, sb) };
        unsafe { (h.r.shift_array_data)(std::ptr::null_mut(), size, sb) };
    }

    // =====================================================================
    // process_pointer_data
    // =====================================================================
    // E13: value * multiplier overflows int -> wraps
    cov.hit("E13");
    for &acc in &[0i32, 1, -1, i32::MAX, i32::MIN] {
        h.set_accum(acc);
        for &v in &[i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 65537, -65537] {
            for &m in &[2i32, -2, i32::MAX, i32::MIN, 65537, -1] {
                h.process_pointer_data(&[v], 0, m);
            }
        }
    }

    // =====================================================================
    // compute_with_dynamic_memory
    // =====================================================================
    // E14: count == 0 -> both loops skipped -> 0
    cov.hit("E14");
    for &base in EDGE.iter() {
        assert_eq!(h.compute_with_dynamic_memory(base, 0), 0, "E14 base={base}");
    }
    for _ in 0..128 {
        assert_eq!(h.compute_with_dynamic_memory(rng.next_i32(), 0), 0);
    }

    // E15: count < 0 -> malloc((size_t)huge) returns NULL, but the loop guards
    //      are false so NULL is never dereferenced -> 0, no crash.
    cov.hit("E15");
    for &count in &[-1i32, -2, -8, -1000, -1_000_000, i32::MIN + 1] {
        for &base in EDGE.iter() {
            assert_eq!(
                h.compute_with_dynamic_memory(base, count),
                0,
                "E15 base={base} count={count}"
            );
        }
    }
    for _ in 0..128 {
        let count = rng.range(i32::MIN + 1, -1);
        assert_eq!(h.compute_with_dynamic_memory(rng.next_i32(), count), 0);
    }

    // E16: count == INT_MIN
    cov.hit("E16");
    for &base in EDGE.iter() {
        assert_eq!(
            h.compute_with_dynamic_memory(base, i32::MIN),
            0,
            "E16 base={base}"
        );
    }

    // E17: base + i*3 / sum += overflow int -> wraps
    cov.hit("E17");
    for &base in &[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 2_000_000_000] {
        for &count in &[1i32, 2, 3, 8, 100, 1000, 4096] {
            h.compute_with_dynamic_memory(base, count);
        }
    }
    // A count large enough that `i*3` itself gets big, plus a wrapping sum.
    for &count in &[100_000i32, 1_000_000] {
        for &base in &[0i32, i32::MAX, i32::MIN] {
            h.compute_with_dynamic_memory(base, count);
        }
    }

    // =====================================================================
    // get_time_based_value
    // =====================================================================
    // E18: seed * 3600 overflows int -> wraps
    cov.hit("E18");
    for &s in &[596_524i32, 600_000, 1_000_000, 2_000_000, 100_000_000] {
        h.get_time_based_value(s);
        h.get_time_based_value(-s);
    }

    // E19: seed == INT_MAX
    cov.hit("E19");
    h.get_time_based_value(i32::MAX);
    h.get_time_based_value(i32::MAX - 1);

    // E20: seed == INT_MIN -> seed*3600 wraps to exactly 0 -> result == INT_MIN
    cov.hit("E20");
    assert_eq!(
        h.get_time_based_value(i32::MIN),
        i32::MIN,
        "E20: INT_MIN*3600 wraps to 0, so the result is INT_MIN"
    );
    h.get_time_based_value(i32::MIN + 1);

    // E21: negative diff/100 must truncate TOWARD ZERO, not floor
    cov.hit("E21");
    assert_eq!(h.get_time_based_value(1_000_000), -5_949_672, "E21 truncation");
    // Sweep seeds whose wrapped product has a non-zero remainder mod 100 in
    // both directions, so floor vs truncate would disagree.
    let mut checked_neg = 0usize;
    let mut checked_pos = 0usize;
    for _ in 0..4096 {
        let s = rng.next_i32();
        let prod = s.wrapping_mul(3600);
        if prod % 100 != 0 {
            if prod < 0 {
                checked_neg += 1;
            } else {
                checked_pos += 1;
            }
            h.get_time_based_value(s);
        }
    }
    assert!(
        checked_neg > 0 && checked_pos > 0,
        "E21 must exercise both signs of a non-exact quotient (neg={checked_neg}, pos={checked_pos})"
    );

    // =====================================================================
    // manipulate_records — every way the L111 guard / L116 bound rejects
    // =====================================================================
    // Helper: build `n` records + `pad` guard records, run both libraries, and
    // compare against a model over the FULL padded value list (so the
    // deliberately out-of-bounds reads of E23 are modelled too).
    let rec_expect = |h: &H, rng: &mut Rng, n: usize, num_records: i32, shift: i32, pad: usize| {
        let recs = random_records(rng, n);
        let mut padded: Vec<i32> = recs.iter().map(|r| r.value).collect();
        padded.extend(std::iter::repeat(GUARD_REC_VALUE).take(pad));
        let (got, after) = h.manipulate_records(&recs, num_records, shift, pad);
        let expect = model_manipulate(&padded, num_records, shift);
        assert_eq!(
            got, expect,
            "manipulate_records(n={num_records}, shift={shift}) vs model"
        );
        (got, after, recs)
    };

    // E22: shift == 0 -> no memmove, loop runs num_records times
    cov.hit("E22");
    for n in [1usize, 2, 5, 17] {
        for _ in 0..64 {
            let (got, after, recs) = rec_expect(&h, &mut rng, n, n as i32, 0, 4);
            assert_eq!(
                got,
                recs.iter().map(|r| r.value).fold(0i32, |a, b| a.wrapping_add(b)),
                "E22 must sum every element"
            );
            assert_eq!(&after[..n], &recs[..], "E22 must not move anything");
        }
    }

    // E23: shift < 0 -> no memmove AND the loop bound num_records-shift exceeds
    //      num_records, so C reads PAST the end of the caller array. Reproduced
    //      verbatim; a padded buffer makes the read deterministic.
    cov.hit("E23");
    for n in [1usize, 2, 5, 12] {
        for k in 1i32..=6 {
            for _ in 0..16 {
                let (got, after, recs) = rec_expect(&h, &mut rng, n, n as i32, -k, 8);
                // the read extends k records past num_records into the guards
                let in_bounds: i32 =
                    recs.iter().map(|r| r.value).fold(0i32, |a, b| a.wrapping_add(b));
                let expect = in_bounds.wrapping_add(GUARD_REC_VALUE.wrapping_mul(k));
                assert_eq!(got, expect, "E23 n={n} shift=-{k} out-of-bounds sum");
                assert_eq!(&after[..n], &recs[..], "E23 must not move anything");
            }
        }
    }
    // shift == INT_MIN + n keeps the bound bounded; also exercise -1 on n == 0
    for _ in 0..16 {
        rec_expect(&h, &mut rng, 0, 0, -3, 8);
    }

    // E24: shift == num_records -> guard false, bound 0 -> 0
    cov.hit("E24");
    for n in [0usize, 1, 2, 5, 17] {
        for _ in 0..32 {
            let (got, after, recs) = rec_expect(&h, &mut rng, n, n as i32, n as i32, 4);
            assert_eq!(got, 0, "E24 n={n} must return 0");
            assert_eq!(&after[..n], &recs[..], "E24 must not move anything");
        }
    }

    // E25: shift > num_records -> bound negative -> 0
    cov.hit("E25");
    for n in [0usize, 1, 2, 5, 17] {
        for extra in [1i32, 2, 100, 100_000] {
            for _ in 0..8 {
                let (got, after, recs) =
                    rec_expect(&h, &mut rng, n, n as i32, n as i32 + extra, 4);
                assert_eq!(got, 0, "E25 n={n} shift={} must return 0", n as i32 + extra);
                assert_eq!(&after[..n], &recs[..], "E25 must not move anything");
            }
        }
    }
    // shift == INT_MAX
    for n in [1usize, 5] {
        let (got, _, _) = rec_expect(&h, &mut rng, n, n as i32, i32::MAX, 4);
        assert_eq!(got, 0, "E25 shift=INT_MAX must return 0");
    }

    // E26: num_records == 0, shift == 0 (zero length)
    cov.hit("E26");
    for _ in 0..32 {
        let (got, _, _) = rec_expect(&h, &mut rng, 0, 0, 0, 4);
        assert_eq!(got, 0, "E26 must return 0");
    }
    // and with a non-empty buffer but num_records == 0
    for _ in 0..32 {
        let recs = random_records(&mut rng, 8);
        let (got, after) = h.manipulate_records(&recs, 0, 0, 0);
        assert_eq!(got, 0, "E26 num_records=0 on a non-empty buffer must return 0");
        assert_eq!(after, recs, "E26 must not touch the buffer");
    }

    // E27: num_records < 0 (negative length) with shift == 0 -> bound negative
    cov.hit("E27");
    for &n in &[-1i32, -2, -8, -1000, i32::MIN + 1, i32::MIN] {
        for _ in 0..8 {
            let recs = random_records(&mut rng, 8);
            let (got, after) = h.manipulate_records(&recs, n, 0, 0);
            assert_eq!(got, 0, "E27 num_records={n} must return 0");
            assert_eq!(after, recs, "E27 must not touch the buffer");
        }
    }

    // E28: num_records - shift overflows int: INT_MIN - INT_MIN == 0
    cov.hit("E28");
    for _ in 0..16 {
        let recs = random_records(&mut rng, 4);
        let (got, after) = h.manipulate_records(&recs, i32::MIN, i32::MIN, 0);
        assert_eq!(got, 0, "E28 INT_MIN/INT_MIN must return 0");
        assert_eq!(after, recs, "E28 must not touch the buffer");
        // shift > 0 is false for INT_MIN, and INT_MAX - INT_MAX == 0 too
        let (got2, _) = h.manipulate_records(&recs, i32::MAX, i32::MAX, 0);
        assert_eq!(got2, 0, "E28 INT_MAX/INT_MAX must return 0");
    }

    // E29: total += overflows int -> wraps
    cov.hit("E29");
    for pattern in 0..4 {
        for &(n, shift) in &[(8usize, 0i32), (8, 1), (32, 3), (64, 7)] {
            let mut recs = random_records(&mut rng, n);
            for (i, r) in recs.iter_mut().enumerate() {
                r.value = match pattern {
                    0 => i32::MAX,
                    1 => i32::MIN,
                    2 => {
                        if i % 2 == 0 {
                            i32::MAX
                        } else {
                            i32::MIN + 1
                        }
                    }
                    _ => i32::MAX - (i as i32) * 7,
                };
            }
            let values: Vec<i32> = recs.iter().map(|r| r.value).collect();
            let (got, _) = h.manipulate_records(&recs, n as i32, shift, 2);
            assert_eq!(
                got,
                model_manipulate(&values, n as i32, shift),
                "E29 pattern={pattern} n={n} shift={shift}"
            );
        }
    }

    // E30: records == NULL while the loop bound is <= 0 -> NULL is never
    //      dereferenced, both libraries must return 0.
    cov.hit("E30");
    for &(n, shift) in &[
        (0i32, 0i32),
        (5, 5),
        (5, 6),
        (5, 100),
        (5, i32::MAX),
        (0, 1),
        (-1, 0),
        (-10, 0),
        (i32::MIN, 0),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
    ] {
        let vc = unsafe { (h.c.manipulate_records)(std::ptr::null_mut(), n, shift) };
        let vr = unsafe { (h.r.manipulate_records)(std::ptr::null_mut(), n, shift) };
        assert_eq!(vc, vr, "E30 manipulate_records(NULL,{n},{shift}) C={vc} Rust={vr}");
        assert_eq!(vc, 0, "E30 manipulate_records(NULL,{n},{shift}) must be 0");
    }

    // =====================================================================
    // apply_operation / arithmetic leaves — overflow wrapping
    // =====================================================================
    // E34: apply_operation adds no checks; the callback's own overflow wraps
    cov.hit("E34");
    h.set_counter(i32::MAX);
    for &(a, b, c) in &[
        (i32::MAX, 1i32, 0i32),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, -1, 0),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, 0, 1),
    ] {
        h.apply_operation_own(Which::AddThree, a, b, c);
        h.apply_operation_own(Which::MultiplyAdd, a, b, c);
        h.apply_operation_own(Which::ComplexCalc, a, b, c);
    }
    h.set_counter(0);

    // E35: add_three overflow
    cov.hit("E35");
    for &(a, b, c) in &[
        (i32::MAX, 1i32, 0i32),
        (i32::MAX, 0, 1),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, -1, 0),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, 1, -1),
        (1, 1, i32::MAX),
    ] {
        h.add_three(a, b, c);
    }

    // E36: multiply_add overflow (incl. INT_MIN * -1)
    cov.hit("E36");
    for &(a, b, c) in &[
        (i32::MIN, -1i32, 0i32),
        (-1, i32::MIN, 0),
        (i32::MAX, 2, 0),
        (i32::MIN, 2, 0),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (65536, 65536, 0),
        (i32::MAX, 1, 1),
    ] {
        h.multiply_add(a, b, c);
    }

    // E37: complex_calc `a - b` overflow
    cov.hit("E37");
    h.set_counter(0);
    for &(a, b, c) in &[
        (i32::MIN, 1i32, 1i32),
        (i32::MAX, -1, 1),
        (i32::MIN, i32::MAX, 1),
        (i32::MAX, i32::MIN, 1),
        (0, i32::MIN, 1),
    ] {
        h.complex_calc(a, b, c);
    }

    // E38: complex_calc `(a-b)*c + counter` overflow
    cov.hit("E38");
    for &ct in &[i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        h.set_counter(ct);
        for &(a, b, c) in &[
            (1i32, 0i32, 1i32),
            (-1, 0, 1),
            (i32::MAX, 0, i32::MAX),
            (i32::MIN, 0, i32::MIN),
            (i32::MIN, i32::MAX, i32::MAX),
            (65536, 0, 65536),
        ] {
            h.complex_calc(a, b, c);
        }
    }

    // E39: global_counter += value overflow, and it persists in the `.so`
    cov.hit("E39");
    h.set_counter(0);
    h.increment_counter(i32::MAX, 0);
    assert_eq!(h.complex_calc(0, 0, 0), i32::MAX, "E39 setup");
    h.increment_counter(i32::MAX, 0);
    assert_eq!(h.state().counter, -2);
    assert_eq!(h.complex_calc(0, 0, 0), -2, "E39 wrap must persist");
    h.increment_counter(i32::MIN, 0);
    assert_eq!(h.complex_calc(0, 0, 0), h.state().counter, "E39 wrap must persist");
    for _ in 0..256 {
        h.increment_counter(rng.spicy_i32(), rng.next_i32());
        assert_eq!(h.complex_calc(0, 0, 0), h.state().counter);
    }

    // E40: global_accumulator = accumulator*2 + value overflow, persisting
    cov.hit("E40");
    h.set_accum(0);
    h.update_accumulator(i32::MAX, 0);
    assert_eq!(h.process_pointer_data(&[0], 0, 0), i32::MAX, "E40 setup");
    h.update_accumulator(0, 0); // INT_MAX*2 == -2
    assert_eq!(h.state().accum, -2);
    assert_eq!(h.process_pointer_data(&[0], 0, 0), -2, "E40 wrap must persist");
    h.set_accum(i32::MIN);
    h.update_accumulator(0, 0); // INT_MIN*2 == 0
    assert_eq!(h.state().accum, 0);
    assert_eq!(h.process_pointer_data(&[0], 0, 0), 0, "E40 wrap must persist");
    for _ in 0..256 {
        h.update_accumulator(rng.spicy_i32(), rng.next_i32());
        assert_eq!(h.process_pointer_data(&[0], 0, 0), h.state().accum);
    }

    // E41: every accumulation inside hatch overflows
    cov.hit("E41");
    for &ct in &[0i32, i32::MAX, i32::MIN] {
        for &ac in &[0i32, i32::MAX, i32::MIN] {
            h.set_counter(ct);
            h.set_accum(ac);
            h.hatch(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
            h.set_counter(ct);
            h.set_accum(ac);
            h.hatch(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
            h.set_counter(ct);
            h.set_accum(ac);
            h.hatch(i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        }
    }

    // E42: param3 == INT_MIN drives get_time_based_value(INT_MIN) inside hatch
    cov.hit("E42");
    h.set_counter(0);
    h.set_accum(0);
    for &p in &[i32::MIN, i32::MIN + 1, i32::MAX, 1_000_000, -1_000_000, 596_524] {
        h.hatch(1, 2, p, 4);
        h.hatch(p, p, p, p);
    }

    cov.assert_complete(ERROR_ROWS_NONFATAL, "ERRORS.md (non-fatal)");
}
