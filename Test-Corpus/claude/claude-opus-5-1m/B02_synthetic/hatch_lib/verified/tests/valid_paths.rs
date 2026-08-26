//! Phase B — valid-path differential tests, one block per `CONFIGS.md` row.
//!
//! Everything runs inside a single `#[test]` so the two libraries' hidden
//! `static` state advances in a deterministic order. Each block records its row
//! id; the final assertion fails if any documented row was not exercised.

mod common;
use common::*;
use std::ffi::c_void;

/// A caller-supplied `operation_func`, defined in the *test* binary (row C20).
extern "C" fn external_callback(a: i32, b: i32, c: i32) -> i32 {
    a.wrapping_mul(7).wrapping_sub(b).wrapping_add(c)
}

fn ext_cb_addr() -> *const c_void {
    external_callback as *const () as *const c_void
}

#[test]
fn phase_b_all_config_rows() {
    let mut h = harness();
    let mut rng = Rng::new(SEED);
    let mut cov = Coverage::new();

    // =====================================================================
    // add_three / multiply_add — pure leaves
    // =====================================================================
    // C1: 4096 random (a,b,c) over the full i32 range.
    cov.hit("C1");
    for _ in 0..4096 {
        let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        h.add_three(a, b, c);
    }

    // C2: full cross product of the 11 edge scalars (1331 combinations).
    cov.hit("C2");
    for &a in EDGE.iter() {
        for &b in EDGE.iter() {
            for &c in EDGE.iter() {
                h.add_three(a, b, c);
            }
        }
    }

    // C3: 4096 random (a,b,c) over the full i32 range.
    cov.hit("C3");
    for _ in 0..4096 {
        let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        h.multiply_add(a, b, c);
    }

    // C4: degenerate multipliers.
    cov.hit("C4");
    for &b in &[0i32, 1, -1, i32::MIN, i32::MAX, 2, -2] {
        for &a in EDGE.iter() {
            for &c in &[0i32, 1, -1, i32::MIN, i32::MAX] {
                h.multiply_add(a, b, c);
            }
        }
    }

    // =====================================================================
    // complex_calc — reads global_counter (state axis B)
    // =====================================================================
    // C5: pristine counter (0).
    cov.hit("C5");
    h.set_counter(0);
    for _ in 0..1024 {
        let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        h.complex_calc(a, b, c);
    }
    for &a in EDGE.iter() {
        for &b in EDGE.iter() {
            h.complex_calc(a, b, rng.spicy_i32());
        }
    }

    // C6: counter > 0.
    cov.hit("C6");
    for &target in &[1i32, 7, 1000, 123_456_789] {
        h.set_counter(target);
        for _ in 0..256 {
            let (a, b, c) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            h.complex_calc(a, b, c);
        }
    }

    // C7: counter < 0.
    cov.hit("C7");
    for &target in &[-1i32, -9, -1000, -987_654_321] {
        h.set_counter(target);
        for _ in 0..256 {
            let (a, b, c) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            h.complex_calc(a, b, c);
        }
    }

    // C8: counter near INT_MAX / INT_MIN so `(a-b)*c + counter` wraps.
    cov.hit("C8");
    for &target in &[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1] {
        h.set_counter(target);
        for _ in 0..256 {
            let (a, b, c) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            h.complex_calc(a, b, c);
        }
        for &a in EDGE.iter() {
            h.complex_calc(a, 1, 1);
            h.complex_calc(a, -1, -1);
        }
    }

    // =====================================================================
    // increment_counter / update_accumulator — the mutators (axes B, C)
    // =====================================================================
    // C9: 256 random positive deltas from pristine; observed via complex_calc.
    cov.hit("C9");
    h.set_counter(0);
    for _ in 0..256 {
        let d = rng.range(1, 1_000_000);
        h.increment_counter(d, 999);
        // (a-b)*c == 0 when c == 0, so complex_calc(_,_,0) reads out the counter.
        assert_eq!(h.complex_calc(rng.next_i32(), rng.next_i32(), 0), h.state().counter);
    }

    // C10: 256 mixed-sign deltas.
    cov.hit("C10");
    for _ in 0..256 {
        let d = rng.next_i32();
        h.increment_counter(d, 0);
        assert_eq!(h.complex_calc(0, 0, 0), h.state().counter);
    }

    // C11: drive the counter deliberately past INT_MAX.
    cov.hit("C11");
    h.set_counter(i32::MAX - 5);
    for _ in 0..12 {
        h.increment_counter(1, 0);
        assert_eq!(h.complex_calc(0, 0, 0), h.state().counter);
    }
    h.set_counter(i32::MIN + 5);
    for _ in 0..12 {
        h.increment_counter(-1, 0);
        assert_eq!(h.complex_calc(0, 0, 0), h.state().counter);
    }
    h.increment_counter(i32::MAX, 0);
    h.increment_counter(i32::MAX, 0);
    assert_eq!(h.complex_calc(0, 0, 0), h.state().counter);

    // C12: a single update_accumulator from pristine; observed via
    //      process_pointer_data(&0, 0) == 0*0 + accumulator.
    cov.hit("C12");
    h.set_accum(0);
    let probe = [0i32];
    for &v in EDGE.iter() {
        h.set_accum(0);
        h.update_accumulator(v, 888);
        assert_eq!(h.state().accum, v);
        assert_eq!(h.process_pointer_data(&probe, 0, 0), v);
    }

    // C13: 256-step random accumulator sequence (non-commutative, wraps).
    cov.hit("C13");
    h.set_accum(0);
    for _ in 0..256 {
        let v = rng.spicy_i32();
        h.update_accumulator(v, rng.next_i32());
        assert_eq!(h.process_pointer_data(&probe, 0, 0), h.state().accum);
    }

    // C14: 512-step interleaving of both mutators.
    cov.hit("C14");
    for _ in 0..512 {
        if rng.next_u64() & 1 == 0 {
            h.increment_counter(rng.spicy_i32(), rng.next_i32());
        } else {
            h.update_accumulator(rng.spicy_i32(), rng.next_i32());
        }
        let st = h.state();
        assert_eq!(h.complex_calc(0, 0, 0), st.counter);
        assert_eq!(h.process_pointer_data(&probe, 0, 0), st.accum);
    }

    // C15: `unused_param` must be ignored by both mutators.
    cov.hit("C15");
    for &u in &[0i32, 999, 888, -1, i32::MIN, i32::MAX] {
        h.set_counter(1234);
        h.increment_counter(5, u);
        assert_eq!(h.state().counter, 1239);
        assert_eq!(h.complex_calc(0, 0, 0), 1239);

        h.set_accum(77);
        h.update_accumulator(3, u);
        assert_eq!(h.state().accum, 157);
        assert_eq!(h.process_pointer_data(&probe, 0, 0), 157);
    }

    // =====================================================================
    // apply_operation — higher-order dispatch (axis D)
    // =====================================================================
    // C16 / C17: op = add_three / multiply_add.
    cov.hit("C16");
    for _ in 0..1024 {
        let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        h.apply_operation_own(Which::AddThree, a, b, c);
    }
    cov.hit("C17");
    for _ in 0..1024 {
        let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        h.apply_operation_own(Which::MultiplyAdd, a, b, c);
    }

    // C18: op = complex_calc with a pristine counter.
    cov.hit("C18");
    h.set_counter(0);
    for _ in 0..512 {
        let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        h.apply_operation_own(Which::ComplexCalc, a, b, c);
    }

    // C19: op = complex_calc with state != 0 — must flow through the call.
    cov.hit("C19");
    for &target in &[1i32, -1, 424_242, i32::MAX, i32::MIN] {
        h.set_counter(target);
        for _ in 0..256 {
            let (a, b, c) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            h.apply_operation_own(Which::ComplexCalc, a, b, c);
        }
    }

    // C20: caller-supplied external callback (raw fn-pointer ABI).
    cov.hit("C20");
    for _ in 0..1024 {
        let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        let model = a.wrapping_mul(7).wrapping_sub(b).wrapping_add(c);
        h.apply_operation_with(ext_cb_addr(), a, b, c, model);
    }
    for &a in EDGE.iter() {
        let model = a.wrapping_mul(7).wrapping_sub(a).wrapping_add(a);
        h.apply_operation_with(ext_cb_addr(), a, a, a, model);
    }

    // C21: cross-library callbacks — C's dispatcher calling Rust's callback and
    //      vice versa (proves the exported wrappers are ABI-interchangeable).
    cov.hit("C21");
    for _ in 0..512 {
        let (a, b, c) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        let m_add = a.wrapping_add(b).wrapping_add(c);
        let m_mul = a.wrapping_mul(b).wrapping_add(c);
        let m_cpx = a.wrapping_sub(b).wrapping_mul(c).wrapping_add(h.state().counter);

        // C dispatcher + Rust callbacks
        assert_eq!(unsafe { (h.c.apply_operation)(h.r.addr_add_three, a, b, c) }, m_add);
        assert_eq!(unsafe { (h.c.apply_operation)(h.r.addr_multiply_add, a, b, c) }, m_mul);
        assert_eq!(unsafe { (h.c.apply_operation)(h.r.addr_complex_calc, a, b, c) }, m_cpx);
        // Rust dispatcher + C callbacks
        assert_eq!(unsafe { (h.r.apply_operation)(h.c.addr_add_three, a, b, c) }, m_add);
        assert_eq!(unsafe { (h.r.apply_operation)(h.c.addr_multiply_add, a, b, c) }, m_mul);
        assert_eq!(unsafe { (h.r.apply_operation)(h.c.addr_complex_calc, a, b, c) }, m_cpx);
    }

    // C34 (also exercised standalone below): apply_operation adds no checks and
    // the callback overflow simply wraps — see ERRORS.md E34.

    // =====================================================================
    // shift_array_data — buffer shapes (axes E, F, I)
    // =====================================================================
    let shift_case = |h: &H, rng: &mut Rng, size: i32, shift_by: i32, pad: usize| {
        let data: Vec<i32> = (0..size.max(0) as usize).map(|_| rng.spicy_i32()).collect();
        let got = h.shift_array_data(&data, size, shift_by, pad);
        let mut expect = model_shift(&data, size, shift_by);
        expect.extend(std::iter::repeat(0x5A5A_5A5Ai32).take(pad));
        assert_eq!(
            got, expect,
            "shift_array_data(size={size}, shift_by={shift_by}) vs model"
        );
    };

    // C22: size = 2, shift_by = 1 (smallest valid shift).
    cov.hit("C22");
    for _ in 0..256 {
        shift_case(&h, &mut rng, 2, 1, 4);
    }

    // C23: size = 3, shift_by in {1, 2 = size-1}.
    cov.hit("C23");
    for _ in 0..256 {
        shift_case(&h, &mut rng, 3, 1, 4);
        shift_case(&h, &mut rng, 3, 2, 4);
    }

    // C24: size = 10, shift_by in {1, 5, 9}.
    cov.hit("C24");
    for _ in 0..256 {
        for sb in [1, 5, 9] {
            shift_case(&h, &mut rng, 10, sb, 4);
        }
    }

    // C25: size = 1000, 64 random valid shifts.
    cov.hit("C25");
    for _ in 0..64 {
        let sb = rng.range(1, 999);
        shift_case(&h, &mut rng, 1000, sb, 8);
    }

    // C26: extreme element payloads.
    cov.hit("C26");
    for pattern in 0..5 {
        let data: Vec<i32> = (0..16)
            .map(|i| match pattern {
                0 => i32::MIN,
                1 => i32::MAX,
                2 => 0,
                3 => {
                    if i % 2 == 0 {
                        i32::MIN
                    } else {
                        i32::MAX
                    }
                }
                _ => -1,
            })
            .collect();
        for sb in [1, 8, 15] {
            let got = h.shift_array_data(&data, 16, sb, 4);
            let mut expect = model_shift(&data, 16, sb);
            expect.extend(std::iter::repeat(0x5A5A_5A5Ai32).take(4));
            assert_eq!(got, expect, "shift_array_data extreme payload {pattern} sb={sb}");
        }
    }

    // C27: guard elements past `size` must never be written.
    cov.hit("C27");
    for _ in 0..128 {
        let data: Vec<i32> = (0..16).map(|_| rng.next_i32()).collect();
        let sb = rng.range(1, 15);
        let got = h.shift_array_data(&data, 16, sb, 48);
        assert!(
            got[16..].iter().all(|&x| x == 0x5A5A_5A5Ai32),
            "shift_array_data(size=16, shift_by={sb}) wrote past the end: {:?}",
            &got[16..]
        );
    }

    // =====================================================================
    // process_pointer_data — reads one int, adds global_accumulator
    // =====================================================================
    // C28: pristine accumulator.
    cov.hit("C28");
    h.set_accum(0);
    for _ in 0..1024 {
        let v = rng.next_i32();
        let m = rng.next_i32();
        h.process_pointer_data(&[v], 0, m);
    }

    // C29: accumulator != 0 (positive, negative, near-overflow).
    cov.hit("C29");
    for &target in &[1i32, -1, 999_983, -999_983, i32::MAX, i32::MIN] {
        h.set_accum(target);
        for _ in 0..256 {
            let v = rng.spicy_i32();
            let m = rng.spicy_i32();
            h.process_pointer_data(&[v], 0, m);
        }
    }

    // C30: degenerate multipliers x edge values.
    cov.hit("C30");
    for &target in &[0i32, 12345, -12345, i32::MAX] {
        h.set_accum(target);
        for &m in &[0i32, 1, -1, i32::MIN, i32::MAX, 2, -2] {
            for &v in EDGE.iter() {
                h.process_pointer_data(&[v], 0, m);
            }
        }
    }

    // C31: interior pointer into a larger array.
    cov.hit("C31");
    h.set_accum(4242);
    for _ in 0..256 {
        let len = rng.range(1, 32) as usize;
        let arr: Vec<i32> = (0..len).map(|_| rng.spicy_i32()).collect();
        let idx = rng.range(0, len as i32 - 1) as usize;
        h.process_pointer_data(&arr, idx, rng.spicy_i32());
    }

    // =====================================================================
    // compute_with_dynamic_memory — allocation sizes (axis H)
    // =====================================================================
    // C32 / C33 / C34: count = 1, 2, 8.
    for (row, count) in [("C32", 1i32), ("C33", 2), ("C34", 8)] {
        cov.hit(row);
        for &base in EDGE.iter() {
            h.compute_with_dynamic_memory(base, count);
        }
        for _ in 0..256 {
            h.compute_with_dynamic_memory(rng.next_i32(), count);
        }
    }

    // C35: count = 1000 with 256 random bases (sum wraps).
    cov.hit("C35");
    for &base in EDGE.iter() {
        h.compute_with_dynamic_memory(base, 1000);
    }
    for _ in 0..256 {
        h.compute_with_dynamic_memory(rng.next_i32(), 1000);
    }

    // C36: 1<<22 elements = 16 MiB allocation (large but valid).
    cov.hit("C36");
    for &base in &[0i32, 1, -1, 12345, i32::MAX, i32::MIN] {
        h.compute_with_dynamic_memory(base, 1 << 22);
    }

    // C37: 128 random (base, count) pairs with count in 1..4096.
    cov.hit("C37");
    for _ in 0..128 {
        let count = rng.range(1, 4096);
        h.compute_with_dynamic_memory(rng.next_i32(), count);
    }

    // =====================================================================
    // get_time_based_value — pure function of `seed` (axis G)
    // =====================================================================
    // C38: seed = 0, +-1.
    cov.hit("C38");
    for &s in &[0i32, 1, -1] {
        h.get_time_based_value(s);
    }

    // C39: |seed| < 596524 -> seed*3600 does not overflow.
    cov.hit("C39");
    for _ in 0..1024 {
        let s = rng.range(-596_523, 596_523);
        h.get_time_based_value(s);
    }
    for &s in &[596_523i32, -596_523, 596_522, -596_522, 100, -100, 99, -99] {
        h.get_time_based_value(s);
    }

    // C40: |seed| >= 596524 -> seed*3600 overflows int and wraps; both signs.
    //      Also pins the truncation direction for a negative quotient.
    cov.hit("C40");
    for &s in &[596_524i32, -596_524, 1_000_000, -1_000_000, 2_000_000, -2_000_000] {
        h.get_time_based_value(s);
    }
    // seed = 1_000_000 -> wrap(3_600_000_000) = -694_967_296 -> /100 = -6_949_672.96
    // -> truncation toward zero -> -6_949_672 -> + 1_000_000 = -5_949_672
    assert_eq!(h.get_time_based_value(1_000_000), -5_949_672);
    for _ in 0..1024 {
        let mag = rng.range(596_524, i32::MAX);
        let s = if rng.next_u64() & 1 == 0 { mag } else { mag.wrapping_neg() };
        h.get_time_based_value(s);
    }

    // C41: 4096 random seeds over the full i32 range.
    cov.hit("C41");
    for _ in 0..4096 {
        h.get_time_based_value(rng.next_i32());
    }
    for &s in EDGE.iter() {
        h.get_time_based_value(s);
    }

    // =====================================================================
    // manipulate_records — struct ABI + memmove (axes E, F, I)
    // =====================================================================
    let rec_case = |h: &H, rng: &mut Rng, n: usize, num_records: i32, shift: i32, pad: usize| {
        let recs = random_records(rng, n);
        let values: Vec<i32> = recs.iter().map(|r| r.value).collect();
        let (got, after) = h.manipulate_records(&recs, num_records, shift, pad);
        let expect = model_manipulate(&values, num_records, shift);
        assert_eq!(
            got, expect,
            "manipulate_records(n={num_records}, shift={shift}) vs model"
        );
        after
    };

    // C42: num_records = 1, shift = 0.
    cov.hit("C42");
    for _ in 0..256 {
        rec_case(&h, &mut rng, 1, 1, 0, 2);
    }

    // C43: num_records = 2, shift = 1 (smallest valid memmove).
    cov.hit("C43");
    for _ in 0..256 {
        rec_case(&h, &mut rng, 2, 2, 1, 2);
    }

    // C44: num_records = 5, shift = 2 — exactly what `hatch` does internally.
    cov.hit("C44");
    for _ in 0..256 {
        rec_case(&h, &mut rng, 5, 5, 2, 2);
    }

    // C45: num_records = 10, shift in {1, 5, 9}.
    cov.hit("C45");
    for _ in 0..128 {
        for shift in [1, 5, 9] {
            rec_case(&h, &mut rng, 10, 10, shift, 2);
        }
    }

    // C46: num_records = 64, 64 random valid shifts, all four fields random.
    cov.hit("C46");
    for _ in 0..64 {
        let shift = rng.range(1, 63);
        rec_case(&h, &mut rng, 64, 64, shift, 3);
    }

    // C47: full post-call byte image + guard records untouched.
    cov.hit("C47");
    for _ in 0..128 {
        let n = rng.range(2, 24) as usize;
        let shift = rng.range(1, n as i32 - 1);
        let recs = random_records(&mut rng, n);
        let values: Vec<i32> = recs.iter().map(|r| r.value).collect();
        let (got, after) = h.manipulate_records(&recs, n as i32, shift, 5);
        assert_eq!(got, model_manipulate(&values, n as i32, shift));
        // memmove side effect: records[0..n-shift] == old records[shift..n]
        let moved = n - shift as usize;
        for i in 0..moved {
            assert_eq!(
                after[i], recs[i + shift as usize],
                "manipulate_records(n={n}, shift={shift}) memmove wrong at {i}"
            );
        }
        // the tail records[n-shift..n] are left as-is by C (memmove only)
        for i in moved..n {
            assert_eq!(
                after[i], recs[i],
                "manipulate_records(n={n}, shift={shift}) tail must be untouched at {i}"
            );
        }
        // guard records past num_records must be pristine
        for i in n..n + 5 {
            assert_eq!(after[i].id, 0x5A5A_5A5A, "guard record {i} clobbered");
            assert_eq!(after[i].value, 0x3C3C_3C3C, "guard record {i} clobbered");
            assert_eq!(after[i].timestamp, 0x1234_5678_9ABC_DEF0, "guard record {i} clobbered");
            assert!(after[i].name.iter().all(|&b| b == 0x41), "guard record {i} clobbered");
        }
    }

    // C48: `.value` fields near INT_MAX/INT_MIN so `total` wraps.
    cov.hit("C48");
    for pattern in 0..4 {
        let mut recs = random_records(&mut rng, 32);
        for (i, r) in recs.iter_mut().enumerate() {
            r.value = match pattern {
                0 => i32::MAX,
                1 => i32::MIN,
                2 => {
                    if i % 2 == 0 {
                        i32::MAX
                    } else {
                        i32::MIN
                    }
                }
                _ => i32::MAX - i as i32,
            };
        }
        let values: Vec<i32> = recs.iter().map(|r| r.value).collect();
        let (got, _) = h.manipulate_records(&recs, 32, 3, 2);
        assert_eq!(got, model_manipulate(&values, 32, 3), "overflow pattern {pattern}");
    }

    // =====================================================================
    // hatch — the top-level composition (axes B, C, G)
    // =====================================================================
    // C49: first calls from a pristine state.
    cov.hit("C49");
    h.set_counter(0);
    h.set_accum(0);
    for &(a, b, c, d) in &[(1i32, 2i32, 3i32, 4i32), (0, 0, 0, 0), (5, 6, 7, 8), (-1, -2, -3, -4)] {
        h.hatch(a, b, c, d);
    }

    // C50: 512 random parameter vectors over the full i32 range.
    cov.hit("C50");
    for _ in 0..512 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        h.hatch(a, b, c, d);
    }

    // C51: all zero, then all +-1.
    cov.hit("C51");
    h.set_counter(0);
    h.set_accum(0);
    h.hatch(0, 0, 0, 0);
    h.hatch(1, 1, 1, 1);
    h.hatch(-1, -1, -1, -1);
    h.hatch(1, -1, 1, -1);
    h.hatch(-1, 1, -1, 1);

    // C52: all 16 combinations of INT_MIN / INT_MAX.
    cov.hit("C52");
    for i in 0..16u32 {
        let pick = |bit: u32| if (i >> bit) & 1 == 0 { i32::MIN } else { i32::MAX };
        h.hatch(pick(0), pick(1), pick(2), pick(3));
    }

    // C53: 20 repeated calls with identical params — the accumulator doubles
    //      each time, so the results must differ while still matching.
    cov.hit("C53");
    h.set_counter(0);
    h.set_accum(0);
    let mut seen = Vec::new();
    for _ in 0..20 {
        seen.push(h.hatch(3, 5, 7, 11));
    }
    assert!(
        seen.windows(2).any(|w| w[0] != w[1]),
        "hatch must be state-dependent across repeated calls, got {seen:?}"
    );

    // C54: hatch interleaved with direct mutator / reader calls.
    cov.hit("C54");
    for _ in 0..256 {
        match rng.next_u64() % 4 {
            0 => {
                h.increment_counter(rng.spicy_i32(), rng.next_i32());
            }
            1 => {
                h.update_accumulator(rng.spicy_i32(), rng.next_i32());
            }
            2 => {
                let st = h.state();
                assert_eq!(h.complex_calc(0, 0, 0), st.counter);
                assert_eq!(h.process_pointer_data(&probe, 0, 0), st.accum);
            }
            _ => {
                h.hatch(rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            }
        }
    }

    // C55: hatch after the state has been driven to overflow.
    cov.hit("C55");
    for &(ct, ac) in &[
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
    ] {
        h.set_counter(ct);
        h.set_accum(ac);
        h.hatch(1, 1, 1, 1);
        h.set_counter(ct);
        h.set_accum(ac);
        h.hatch(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
        h.set_counter(ct);
        h.set_accum(ac);
        h.hatch(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    }

    // =====================================================================
    // C56: full-pipeline replay across every entry point
    // =====================================================================
    cov.hit("C56");
    for step in 0..2000u32 {
        match rng.next_u64() % 12 {
            0 => {
                h.add_three(rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            }
            1 => {
                h.multiply_add(rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            }
            2 => {
                h.complex_calc(rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            }
            3 => {
                h.increment_counter(rng.spicy_i32(), rng.next_i32());
            }
            4 => {
                h.update_accumulator(rng.spicy_i32(), rng.next_i32());
            }
            5 => {
                let which = match rng.next_u64() % 3 {
                    0 => Which::AddThree,
                    1 => Which::MultiplyAdd,
                    _ => Which::ComplexCalc,
                };
                h.apply_operation_own(which, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            }
            6 => {
                let size = rng.range(0, 24);
                let sb = rng.range(-4, 28);
                let data: Vec<i32> =
                    (0..size.max(0) as usize).map(|_| rng.spicy_i32()).collect();
                let got = h.shift_array_data(&data, size, sb, 6);
                let mut expect = model_shift(&data, size, sb);
                expect.extend(std::iter::repeat(0x5A5A_5A5Ai32).take(6));
                assert_eq!(got, expect, "step {step}: shift_array_data({size},{sb})");
            }
            7 => {
                let len = rng.range(1, 16) as usize;
                let arr: Vec<i32> = (0..len).map(|_| rng.spicy_i32()).collect();
                let idx = rng.range(0, len as i32 - 1) as usize;
                h.process_pointer_data(&arr, idx, rng.spicy_i32());
            }
            8 => {
                h.compute_with_dynamic_memory(rng.spicy_i32(), rng.range(0, 512));
            }
            9 => {
                h.get_time_based_value(rng.spicy_i32());
            }
            10 => {
                let n = rng.range(1, 20) as usize;
                let shift = rng.range(0, n as i32);
                rec_case(&h, &mut rng, n, n as i32, shift, 4);
            }
            _ => {
                h.hatch(rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            }
        }
    }

    cov.assert_complete(CONFIG_ROWS, "CONFIGS.md");
}
