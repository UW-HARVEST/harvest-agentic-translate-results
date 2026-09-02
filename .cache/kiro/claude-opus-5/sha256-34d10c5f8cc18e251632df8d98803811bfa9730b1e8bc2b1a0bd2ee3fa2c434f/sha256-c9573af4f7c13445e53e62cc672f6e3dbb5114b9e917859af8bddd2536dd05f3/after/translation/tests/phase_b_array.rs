//! Phase B — valid-path differential tests, rows C16..C32 of `CONFIGS.md`.
//! These drive the LOW-LEVEL `ResultArray` entry points directly and then the
//! composed pipeline, not just the `arrayfunc` convenience wrapper.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Helpers: build the same array in both libraries
// ---------------------------------------------------------------------------

/// Fills a fresh (poisoned) ResultArray in each library via that library's own
/// `init_result_array`, then asserts they already agree.
fn paired_arrays(values: &[c_int], count: c_int, ctx: &str) -> (ResultArray, ResultArray) {
    let (c, r) = both();
    let mut ca = ResultArray::poisoned();
    let mut ra = ResultArray::poisoned();
    let mut cv = values.to_vec();
    let mut rv = values.to_vec();
    (c.init_result_array)(&mut ca, cv.as_mut_ptr(), count);
    (r.init_result_array)(&mut ra, rv.as_mut_ptr(), count);
    eq_struct(&format!("{ctx} (after init)"), &ca, &ra);
    (ca, ra)
}

fn rnd_values(rng: &mut Rng) -> Vec<c_int> {
    (0..16).map(|_| rng.next_i32_spicy()).collect()
}

// ---------------------------------------------------------------------------
// C16 / C17 — compare_results_in_array
// ---------------------------------------------------------------------------

#[test]
fn c16_compare_in_range_random_pairs() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..3000 {
        let count = 1 + rng.below(10) as c_int;
        let values = rnd_values(&mut rng);
        let (mut ca, mut ra) = paired_arrays(&values, count, &format!("C16 #{i}"));
        let i1 = rng.below(count as u64) as c_int;
        let i2 = rng.below(count as u64) as c_int;
        eq_int(
            &format!("C16 #{i} count={count} ({i1},{i2})"),
            (c.compare_results_in_array)(&mut ca, i1, i2),
            (r.compare_results_in_array)(&mut ra, i1, i2),
        );
        // compare must not mutate
        eq_struct(&format!("C16 #{i} (unchanged)"), &ca, &ra);
    }
}

#[test]
fn c17_compare_exhaustive_count_x_index_cross_product() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for count in 0..=10i32 {
        let values = rnd_values(&mut rng);
        let (mut ca, mut ra) = paired_arrays(&values, count, &format!("C17 count={count}"));
        for i1 in 0..(count + 3) {
            for i2 in 0..(count + 3) {
                eq_int(
                    &format!("C17 count={count} ({i1},{i2})"),
                    (c.compare_results_in_array)(&mut ca, i1, i2),
                    (r.compare_results_in_array)(&mut ra, i1, i2),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C18..C21 — process_with_foreach, one pass, each of the 4 operations
// ---------------------------------------------------------------------------

fn foreach_row(row: &str, pick: fn(&Lib) -> OperationFunc) {
    let (c, r) = both();
    let mut rng = Rng::seeded();

    // Every count, dense.
    for count in 0..=10i32 {
        for i in 0..300 {
            let values = rnd_values(&mut rng);
            let ctx = format!("{row} count={count} #{i}");
            let (mut ca, mut ra) = paired_arrays(&values, count, &ctx);
            let cv = (c.process_with_foreach)(&mut ca, pick(c));
            let rv = (r.process_with_foreach)(&mut ra, pick(r));
            eq_int(&ctx, cv, rv);
            eq_struct(&format!("{ctx} (after pass)"), &ca, &ra);
        }
    }
    // Saturating shapes.
    for values in [
        vec![i32::MAX; 16],
        vec![i32::MIN; 16],
        vec![0; 16],
        (0..16)
            .map(|i| if i % 2 == 0 { i32::MAX } else { i32::MIN })
            .collect::<Vec<_>>(),
    ] {
        let ctx = format!("{row} extreme");
        let (mut ca, mut ra) = paired_arrays(&values, 10, &ctx);
        let cv = (c.process_with_foreach)(&mut ca, pick(c));
        let rv = (r.process_with_foreach)(&mut ra, pick(r));
        eq_int(&ctx, cv, rv);
        eq_struct(&format!("{ctx} (after pass)"), &ca, &ra);
    }
}

#[test]
fn c18_foreach_add() {
    foreach_row("C18 add", |l| l.add_operation);
}

#[test]
fn c19_foreach_multiply() {
    foreach_row("C19 mul", |l| l.multiply_operation);
}

#[test]
fn c20_foreach_subtract() {
    foreach_row("C20 sub", |l| l.subtract_operation);
}

/// `b` is `item->rank`, so slot 0 takes `modulo_operation`'s `b == 0` early
/// return and slots 1.. take the `idiv` path. Both branches in one pass.
#[test]
fn c21_foreach_modulo() {
    foreach_row("C21 mod", |l| l.modulo_operation);
}

// ---------------------------------------------------------------------------
// C22 — the 4 chained passes, state carried between them
// ---------------------------------------------------------------------------

#[test]
fn c22_foreach_four_chained_passes() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for count in 0..=10i32 {
        for i in 0..200 {
            let values = rnd_values(&mut rng);
            let ctx = format!("C22 count={count} #{i}");
            let (mut ca, mut ra) = paired_arrays(&values, count, &ctx);
            let cops = c.operations();
            let rops = r.operations();
            let mut ctotal: c_int = 0;
            let mut rtotal: c_int = 0;
            for k in 0..4 {
                ctotal = ctotal.wrapping_add((c.process_with_foreach)(&mut ca, cops[k]));
                rtotal = rtotal.wrapping_add((r.process_with_foreach)(&mut ra, rops[k]));
                eq_int(&format!("{ctx} pass{k} running total"), ctotal, rtotal);
                eq_struct(&format!("{ctx} pass{k} struct"), &ca, &ra);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C23 — arbitrary caller-supplied callbacks (the `op` axis is a raw fn ptr)
// ---------------------------------------------------------------------------

extern "C" fn cb_const_max(_a: c_int, _b: c_int, _c: c_int, _d: c_int) -> c_int {
    i32::MAX
}
extern "C" fn cb_const_min(_a: c_int, _b: c_int, _c: c_int, _d: c_int) -> c_int {
    i32::MIN
}
extern "C" fn cb_rank(_a: c_int, b: c_int, _c: c_int, _d: c_int) -> c_int {
    b
}
extern "C" fn cb_asserts_unused(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    // `process_with_foreach` must always pass 0 for both unused args.
    assert_eq!(c, 0, "unused1 must be 0");
    assert_eq!(d, 0, "unused2 must be 0");
    a ^ b
}
extern "C" fn cb_zero(_a: c_int, _b: c_int, _c: c_int, _d: c_int) -> c_int {
    0
}

#[test]
fn c23_foreach_arbitrary_callbacks() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    let cbs: [(&str, OperationFunc); 5] = [
        ("const_max", cb_const_max),
        ("const_min", cb_const_min),
        ("rank", cb_rank),
        ("xor+asserts_unused_are_zero", cb_asserts_unused),
        ("zero", cb_zero),
    ];
    for (name, cb) in cbs {
        for count in 0..=10i32 {
            for i in 0..100 {
                let values = rnd_values(&mut rng);
                let ctx = format!("C23 {name} count={count} #{i}");
                let (mut ca, mut ra) = paired_arrays(&values, count, &ctx);
                // The SAME callback pointer is handed to both libraries.
                let cv = (c.process_with_foreach)(&mut ca, cb);
                let rv = (r.process_with_foreach)(&mut ra, cb);
                eq_int(&ctx, cv, rv);
                eq_struct(&format!("{ctx} (struct)"), &ca, &ra);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C24..C26 — compute_weighted_sum
// ---------------------------------------------------------------------------

#[test]
fn c24_weighted_sum_random() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for count in 0..=10i32 {
        for i in 0..500 {
            let values = rnd_values(&mut rng);
            let ctx = format!("C24 count={count} #{i}");
            let (mut ca, mut ra) = paired_arrays(&values, count, &ctx);
            eq_int(
                &ctx,
                (c.compute_weighted_sum)(&mut ca),
                (r.compute_weighted_sum)(&mut ra),
            );
            eq_struct(&format!("{ctx} (unchanged)"), &ca, &ra);
        }
    }
}

#[test]
fn c25_weighted_sum_saturating_shapes() {
    let (c, r) = both();
    let shapes: Vec<(&str, Vec<c_int>)> = vec![
        ("all_max", vec![i32::MAX; 16]),
        ("all_min", vec![i32::MIN; 16]),
        (
            "alternating",
            (0..16)
                .map(|i| if i % 2 == 0 { i32::MAX } else { i32::MIN })
                .collect(),
        ),
        ("all_zero", vec![0; 16]),
        ("all_neg_one", vec![-1; 16]),
        (
            "ramp",
            (0..16).map(|i| i32::MAX - i * 100_000_000).collect(),
        ),
    ];
    for (name, values) in shapes {
        for count in 0..=10i32 {
            let ctx = format!("C25 {name} count={count}");
            let (mut ca, mut ra) = paired_arrays(&values, count, &ctx);
            eq_int(
                &ctx,
                (c.compute_weighted_sum)(&mut ca),
                (r.compute_weighted_sum)(&mut ra),
            );
        }
    }
}

#[test]
fn c26_weighted_sum_after_foreach_passes() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for count in 0..=10i32 {
        for i in 0..200 {
            let values = rnd_values(&mut rng);
            let ctx = format!("C26 count={count} #{i}");
            let (mut ca, mut ra) = paired_arrays(&values, count, &ctx);
            let cops = c.operations();
            let rops = r.operations();
            for k in 0..4 {
                (c.process_with_foreach)(&mut ca, cops[k]);
                (r.process_with_foreach)(&mut ra, rops[k]);
            }
            eq_struct(&format!("{ctx} (pre-weighted)"), &ca, &ra);
            eq_int(
                &ctx,
                (c.compute_weighted_sum)(&mut ca),
                (r.compute_weighted_sum)(&mut ra),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C27..C30 — arrayfunc
// ---------------------------------------------------------------------------

fn af(ctx: &str, p: [c_int; 4]) {
    let (c, r) = both();
    eq_int(
        &format!("{ctx} params={p:?}"),
        (c.arrayfunc)(p[0], p[1], p[2], p[3]),
        (r.arrayfunc)(p[0], p[1], p[2], p[3]),
    );
}

#[test]
fn c27_arrayfunc_random_quadruples() {
    let mut rng = Rng::seeded();
    for i in 0..20000 {
        let p = [
            rng.next_i32_spicy(),
            rng.next_i32_spicy(),
            rng.next_i32_spicy(),
            rng.next_i32_spicy(),
        ];
        af(&format!("C27 #{i}"), p);
    }
}

#[test]
fn c28_arrayfunc_small_magnitude_sweep() {
    for a in -4..=4i32 {
        for b in -4..=4i32 {
            for c in -4..=4i32 {
                for d in -4..=4i32 {
                    af("C28", [a, b, c, d]);
                }
            }
        }
    }
}

#[test]
fn c29_arrayfunc_boundary_cross_product() {
    let bs = boundary_i32();
    for &a in bs.iter() {
        for &b in bs.iter() {
            for &c in bs.iter() {
                for &d in bs.iter() {
                    af("C29", [a, b, c, d]);
                }
            }
        }
    }
    // Each slot at a boundary while the others are random.
    let mut rng = Rng::seeded();
    for &v in bs.iter() {
        for slot in 0..4 {
            for _ in 0..500 {
                let mut p = [
                    rng.next_i32(),
                    rng.next_i32(),
                    rng.next_i32(),
                    rng.next_i32(),
                ];
                p[slot] = v;
                af("C29 slot", p);
            }
        }
    }
}

#[test]
fn c30_arrayfunc_overflowing_value_initialisers() {
    // param1+param2 overflow, param2-param3 overflow, param3*2 overflow,
    // param4/2+1 at the extremes.
    let cases: Vec<[c_int; 4]> = vec![
        [i32::MAX, 1, 0, 0],
        [i32::MAX, i32::MAX, 0, 0],
        [i32::MIN, -1, 0, 0],
        [i32::MIN, i32::MIN, 0, 0],
        [0, i32::MIN, 1, 0],
        [0, i32::MAX, -1, 0],
        [0, i32::MIN, i32::MAX, 0],
        [0, 0, i32::MAX, 0],
        [0, 0, i32::MIN, 0],
        [0, 0, i32::MAX / 2 + 1, 0],
        [0, 0, 0, i32::MIN],
        [0, 0, 0, i32::MAX],
        [0, 0, 0, -1],
        [0, 0, 0, 1],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN],
        [i32::MIN, i32::MAX, i32::MIN, i32::MAX],
    ];
    for (i, p) in cases.iter().enumerate() {
        af(&format!("C30 #{i}"), *p);
    }
    // Randomized near-overflow pairs.
    let mut rng = Rng::seeded();
    for i in 0..2000 {
        let k = rng.below(64) as i32;
        let p = [
            i32::MAX - k,
            i32::MAX - (rng.below(64) as i32),
            i32::MIN + (rng.below(64) as i32),
            i32::MIN + (rng.below(64) as i32),
        ];
        af(&format!("C30 rnd #{i}"), p);
    }
}

// ---------------------------------------------------------------------------
// C31 — the composed pipeline driven by hand through the low-level API
// ---------------------------------------------------------------------------

/// Reproduces `arrayfunc`'s body using only the low-level exports of `lib`,
/// and returns the result plus the final array state.
fn manual_arrayfunc(lib: &Lib, p: [c_int; 4]) -> (c_int, ResultArray) {
    let mut values: [c_int; 8] = [
        p[0],
        p[1],
        p[2],
        p[3],
        p[0].wrapping_add(p[1]),
        p[1].wrapping_sub(p[2]),
        p[2].wrapping_mul(2),
        (p[3] / 2).wrapping_add(1),
    ];
    let mut arr = ResultArray::zeroed();
    (lib.init_result_array)(&mut arr, values.as_mut_ptr(), 8);

    let mut result: c_int = 0;
    let ops = lib.operations();
    for k in 0..4 {
        result = result.wrapping_add((lib.process_with_foreach)(&mut arr, ops[k]));
    }
    result = result.wrapping_add((lib.compute_weighted_sum)(&mut arr));

    let mut i: c_int = 0;
    while i < arr.count - 1 {
        result = result.wrapping_add((lib.compare_results_in_array)(&mut arr, i, i + 1));
        i += 1;
    }
    let final_scale = result as f64 * 0.333;
    result = (lib.safe_double_to_int)(final_scale);
    (result, arr)
}

#[test]
fn c31_manual_pipeline_matches_arrayfunc_in_both_libs() {
    let (c, r) = both();
    let mut rng = Rng::seeded();

    let mut cases: Vec<[c_int; 4]> = vec![
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [-1, -1, -1, -1],
        [i32::MAX, i32::MIN, 0, -1],
        [7, -3, 11, -25],
    ];
    for _ in 0..4000 {
        cases.push([
            rng.next_i32_spicy(),
            rng.next_i32_spicy(),
            rng.next_i32_spicy(),
            rng.next_i32_spicy(),
        ]);
    }

    for (i, &p) in cases.iter().enumerate() {
        let ctx = format!("C31 #{i} params={p:?}");
        let (cman, carr) = manual_arrayfunc(c, p);
        let (rman, rarr) = manual_arrayfunc(r, p);
        eq_int(&format!("{ctx} manual"), cman, rman);
        eq_struct(&format!("{ctx} final struct"), &carr, &rarr);
        // The hand-composed pipeline must equal the one-shot wrapper in BOTH.
        let cone = (c.arrayfunc)(p[0], p[1], p[2], p[3]);
        let rone = (r.arrayfunc)(p[0], p[1], p[2], p[3]);
        eq_int(&format!("{ctx} wrapper"), cone, rone);
        assert_eq!(cman, cone, "C: manual pipeline != C arrayfunc [{ctx}]");
        assert_eq!(rman, rone, "RUST: manual pipeline != Rust arrayfunc [{ctx}]");
    }
}

/// Cross-library composition: a struct produced by one library's
/// `init_result_array` is consumed by the *other* library's functions. This is
/// the strongest ABI check — it only works if the layouts are truly identical.
#[test]
fn c31b_cross_library_struct_handoff() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..2000 {
        let mut values = rnd_values(&mut rng);
        let count = rng.below(11) as c_int;

        // C fills, Rust consumes.
        let mut a1 = ResultArray::poisoned();
        (c.init_result_array)(&mut a1, values.as_mut_ptr(), count);
        let mut a2 = a1;
        let v1 = (r.compute_weighted_sum)(&mut a1);
        let v2 = (c.compute_weighted_sum)(&mut a2);
        eq_int(&format!("C31b #{i} C-init weighted"), v2, v1);

        // Rust fills, C consumes.
        let mut b1 = ResultArray::poisoned();
        (r.init_result_array)(&mut b1, values.as_mut_ptr(), count);
        let mut b2 = b1;
        let w1 = (c.process_with_foreach)(&mut b1, c.add_operation);
        let w2 = (r.process_with_foreach)(&mut b2, r.add_operation);
        eq_int(&format!("C31b #{i} Rust-init foreach"), w1, w2);
        eq_struct(&format!("C31b #{i} Rust-init foreach struct"), &b1, &b2);
    }
}

// ---------------------------------------------------------------------------
// C32 — struct ABI
// ---------------------------------------------------------------------------

#[test]
fn c32_struct_layout_matches_c() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<Result>(), 24, "sizeof(Result)");
    assert_eq!(align_of::<Result>(), 8, "alignof(Result)");
    assert_eq!(size_of::<ResultArray>(), 248, "sizeof(ResultArray)");
    assert_eq!(align_of::<ResultArray>(), 8, "alignof(ResultArray)");

    let dummy = ResultArray::zeroed();
    let base = &dummy as *const ResultArray as usize;
    assert_eq!(&dummy.data as *const _ as usize - base, 0, "offsetof(data)");
    assert_eq!(&dummy.count as *const _ as usize - base, 240, "offsetof(count)");
    let e = &dummy.data[0];
    let eb = e as *const Result as usize;
    assert_eq!(&e.value as *const _ as usize - eb, 0, "offsetof(value)");
    assert_eq!(&e.scaled as *const _ as usize - eb, 8, "offsetof(scaled)");
    assert_eq!(&e.rank as *const _ as usize - eb, 16, "offsetof(rank)");

    // Confirm the C library agrees on element stride: with count=10 and
    // distinct values, compare_results_in_array must order every pair by index,
    // which is only true if the stride matches.
    let (c, r) = both();
    let mut values: Vec<c_int> = (0..10).collect();
    let mut ca = ResultArray::poisoned();
    let mut ra = ResultArray::poisoned();
    (c.init_result_array)(&mut ca, values.as_mut_ptr(), 10);
    (r.init_result_array)(&mut ra, values.as_mut_ptr(), 10);
    for i in 0..10i32 {
        assert_eq!(ca.data[i as usize].value, i, "C wrote the wrong stride");
        assert_eq!(ra.data[i as usize].value, i, "Rust wrote the wrong stride");
        for j in 0..10i32 {
            let want = if i < j {
                -1
            } else if i > j {
                1
            } else {
                0
            };
            assert_eq!((c.compare_results_in_array)(&mut ca, i, j), want);
            assert_eq!((r.compare_results_in_array)(&mut ra, i, j), want);
        }
    }
}

/// Raw 248-byte image, padding included.
///
/// Both buffers are memset to the same pattern *before* the call, so any
/// difference is attributable to the libraries rather than to stack noise in
/// this test binary.
#[test]
fn c32b_raw_byte_image_including_padding() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    const SZ: usize = std::mem::size_of::<ResultArray>();
    for i in 0..1000 {
        let mut values = rnd_values(&mut rng);
        let count = rng.below(11) as c_int;

        let mut cbuf = [0xAAu8; SZ];
        let mut rbuf = [0xAAu8; SZ];
        let cp = cbuf.as_mut_ptr() as *mut ResultArray;
        let rp = rbuf.as_mut_ptr() as *mut ResultArray;
        (c.init_result_array)(cp, values.as_mut_ptr(), count);
        (r.init_result_array)(rp, values.as_mut_ptr(), count);

        if cbuf != rbuf {
            // Report which byte offsets differ and whether any of them is an
            // observable (non-padding) byte.
            let pad: Vec<usize> = (0..10)
                .flat_map(|e| {
                    let b = e * 24;
                    (b + 4..b + 8).chain(b + 20..b + 24)
                })
                .chain(244..248)
                .collect();
            let diffs: Vec<usize> = (0..SZ).filter(|&k| cbuf[k] != rbuf[k]).collect();
            let observable: Vec<usize> =
                diffs.iter().copied().filter(|k| !pad.contains(k)).collect();
            assert!(
                observable.is_empty(),
                "raw image diverged at OBSERVABLE byte offsets {observable:?} \
                 [C32b #{i} count={count}]"
            );
            // Padding-only difference: the C standard leaves padding bytes
            // unspecified, so this is not observable by any conforming caller.
            // Recorded rather than failed.
            PAD_ONLY_DIFFS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    let n = PAD_ONLY_DIFFS.load(std::sync::atomic::Ordering::Relaxed);
    println!("C32b: {n}/1000 cases differed in padding bytes only (unobservable)");
}

static PAD_ONLY_DIFFS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
