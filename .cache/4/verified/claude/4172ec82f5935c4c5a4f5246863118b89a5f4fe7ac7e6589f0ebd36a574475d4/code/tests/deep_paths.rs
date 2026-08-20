//! Deep differential tests for the code the public API cannot reach.
//!
//! `initialize_test_data()` is `static` and never called in `lib.c`, so
//! `node_count` is permanently 0 and `jumpnode`'s modes 1/2/4 always take their
//! "node not found" error return. That makes most of the algorithm
//! (`find_node_by_id`, `add_node`, `process_backward`, `safe_double_to_int`,
//! `compute_size_metric`, the parent walk, the sqrt accumulation and the
//! backward scan) unreachable — and therefore unverifiable — through `jumpnode`
//! alone.
//!
//! These tests pair the Rust cdylib built with `--features shadow_probe`
//! against `shadow_c/build/libshadow_c.so`, which `#include`s the untouched
//! `c_src/src/lib.c`. Both sides expose the same `probe_*` wrappers, so the
//! low-level functions are compared directly and `jumpnode` can be driven with
//! populated node storage.
//!
//! Everything here is compiled out unless the `shadow_probe` feature is on.

mod common;

#[cfg(feature = "shadow_probe")]
mod deep {
    use crate::common::shadow::{lock, shadow};
    use crate::common::{Rng, ARG_BOUNDARIES, F64_BOUNDARIES};
    use crate::both_syms;
    use std::os::raw::{c_char, c_double, c_int};

    type FnI = unsafe extern "C" fn() -> c_int;
    type FnV = unsafe extern "C" fn();
    type FnD2I = unsafe extern "C" fn(c_double) -> c_int;
    type FnI2I = unsafe extern "C" fn(c_int) -> c_int;
    type FnI2D = unsafe extern "C" fn(c_int) -> c_double;
    type FnII2I = unsafe extern "C" fn(c_int, c_int) -> c_int;
    type FnAdd = unsafe extern "C" fn(c_int, c_int, c_double) -> c_int;
    type FnPb = unsafe extern "C" fn(*mut c_int, usize, c_int) -> c_int;
    type FnStr = unsafe extern "C" fn(*const c_char) -> c_int;
    type FnSz = unsafe extern "C" fn() -> usize;
    type FnJump = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

    // ------------------------------------------------------------------
    // Struct layout & constants
    // ------------------------------------------------------------------
    #[test]
    fn deep_struct_layout_and_constants_match() {
        let _g = lock();
        let p = shadow();
        let (c, r) = both_syms!(p, "probe_sizeof_node", FnSz);
        assert_eq!(
            unsafe { c() },
            unsafe { r() },
            "sizeof(Node) differs: the #[repr(C)] layout does not match the C struct"
        );

        let (c, r) = both_syms!(p, "probe_status", FnI2I);
        for which in -3..=8 {
            assert_eq!(
                unsafe { c(which) },
                unsafe { r(which) },
                "STATUS/MAX_NODES constant #{which} differs"
            );
        }
    }

    // ------------------------------------------------------------------
    // safe_double_to_int  (ERRORS.md rows 7, 8, 9)
    // ------------------------------------------------------------------
    #[test]
    fn deep_safe_double_to_int_all_shapes() {
        let _g = lock();
        let p = shadow();
        let (c, r) = both_syms!(p, "probe_safe_double_to_int", FnD2I);

        for &v in &F64_BOUNDARIES {
            let (cv, rv) = (unsafe { c(v) }, unsafe { r(v) });
            assert_eq!(cv, rv, "safe_double_to_int({v:?} bits={:#x}) C={cv} Rust={rv}", v.to_bits());
        }

        let mut rng = Rng::new(0x3001);
        for _ in 0..200_000 {
            let v = rng.shaped_f64();
            let (cv, rv) = (unsafe { c(v) }, unsafe { r(v) });
            assert_eq!(
                cv, rv,
                "safe_double_to_int({v:?} bits={:#x}) C={cv} Rust={rv}",
                v.to_bits()
            );
        }

        // Dense sweep across the clamp boundaries, ulp by ulp.
        for anchor in [2147483647.0f64, -2147483648.0f64, 0.0f64] {
            let mut v = anchor;
            for _ in 0..2000 {
                v = f64::from_bits(v.to_bits().wrapping_sub(1));
            }
            for _ in 0..4000 {
                let (cv, rv) = (unsafe { c(v) }, unsafe { r(v) });
                assert_eq!(cv, rv, "ulp sweep at {v:?}: C={cv} Rust={rv}");
                v = f64::from_bits(v.to_bits().wrapping_add(1));
            }
        }
    }

    // ------------------------------------------------------------------
    // compute_size_metric
    // ------------------------------------------------------------------
    #[test]
    fn deep_compute_size_metric_all_lengths() {
        let _g = lock();
        let p = shadow();
        let (c, r) = both_syms!(p, "probe_compute_size_metric", FnStr);

        let mut rng = Rng::new(0x3002);
        // Every length 0..=512, then large lengths.
        let mut lens: Vec<usize> = (0..=512).collect();
        lens.extend([1000, 4095, 4096, 65535, 65536, 100_000]);
        for &len in &lens {
            let mut buf: Vec<u8> = (0..len).map(|_| 1 + (rng.below(255) as u8)).collect();
            buf.push(0);
            let (cv, rv) = (
                unsafe { c(buf.as_ptr() as *const c_char) },
                unsafe { r(buf.as_ptr() as *const c_char) },
            );
            assert_eq!(cv, rv, "compute_size_metric(len={len}) C={cv} Rust={rv}");
        }

        // Interior NUL: strlen must stop at the first NUL, not the buffer end.
        for &(pre, post) in &[(0usize, 5usize), (1, 1), (7, 100), (33, 2), (128, 128)] {
            let mut buf: Vec<u8> = vec![b'x'; pre];
            buf.push(0);
            buf.extend(std::iter::repeat(b'y').take(post));
            buf.push(0);
            let (cv, rv) = (
                unsafe { c(buf.as_ptr() as *const c_char) },
                unsafe { r(buf.as_ptr() as *const c_char) },
            );
            assert_eq!(cv, rv, "interior NUL at {pre}: C={cv} Rust={rv}");
        }
    }

    // ------------------------------------------------------------------
    // process_backward  (ERRORS.md row 14)
    // ------------------------------------------------------------------
    #[test]
    fn deep_process_backward_offsets_and_sizes() {
        let _g = lock();
        let p = shadow();
        let (c, r) = both_syms!(p, "probe_process_backward", FnPb);

        let mut rng = Rng::new(0x3003);
        for size in 0usize..=40 {
            for _ in 0..40 {
                let mut a: Vec<c_int> = (0..size.max(1)).map(|_| rng.i32()).collect();
                // start_offset within [0, size] is the in-bounds domain; a few
                // values past `size` are also exercised (the `while (ptr > start)`
                // guard is immediately false, so the result is 0). Negative
                // offsets are skipped: the C reads before the array, which is
                // genuine undefined behaviour with no defined result to match.
                for off in 0..=(size as c_int + 6) {
                    let cv = unsafe { c(a.as_mut_ptr(), size, off) };
                    let rv = unsafe { r(a.as_mut_ptr(), size, off) };
                    assert_eq!(
                        cv, rv,
                        "process_backward(size={size}, off={off}) C={cv} Rust={rv}"
                    );
                }
            }
        }

        // Overflow of the `sum +=` accumulation must wrap identically.
        for _ in 0..2000 {
            let size = 1 + (rng.below(32) as usize);
            let mut a: Vec<c_int> = (0..size)
                .map(|_| if rng.below(2) == 0 { c_int::MAX } else { c_int::MIN })
                .collect();
            let off = rng.below(size as u64 + 1) as c_int;
            let cv = unsafe { c(a.as_mut_ptr(), size, off) };
            let rv = unsafe { r(a.as_mut_ptr(), size, off) };
            assert_eq!(cv, rv, "wrapping sum (size={size}, off={off}) C={cv} Rust={rv}");
        }

        // The exact shape jumpnode mode 0002 uses: size 16, offset = depth.
        for depth in 0..=22 {
            let mut a: Vec<c_int> = vec![0; 20];
            for (i, slot) in a.iter_mut().enumerate() {
                *slot = if i < 4 {
                    [0o100, 0o200, 0o300, 0o400][i]
                } else {
                    (i as c_int) * 0o7
                };
            }
            let cv = unsafe { c(a.as_mut_ptr(), 16, depth) };
            let rv = unsafe { r(a.as_mut_ptr(), 16, depth) };
            assert_eq!(cv, rv, "mode-2 shape depth={depth}: C={cv} Rust={rv}");
        }
    }

    // ------------------------------------------------------------------
    // add_node / find_node_by_id  (ERRORS.md rows 5, 6)
    // ------------------------------------------------------------------
    /// Reset both libraries and replay the same `add_node` sequence on each.
    fn populate(nodes: &[(c_int, c_int, c_double)]) -> Vec<(c_int, c_int)> {
        let p = shadow();
        let (creset, rreset) = both_syms!(p, "probe_reset", FnV);
        let (cadd, radd) = both_syms!(p, "probe_add_node", FnAdd);
        unsafe {
            creset();
            rreset();
        }
        let mut rcs = Vec::with_capacity(nodes.len());
        for &(id, parent, value) in nodes {
            let cv = unsafe { cadd(id, parent, value) };
            let rv = unsafe { radd(id, parent, value) };
            assert_eq!(
                cv, rv,
                "add_node({id},{parent},{value:?}) C={cv} Rust={rv}"
            );
            rcs.push((cv, rv));
        }
        let (ccnt, rcnt) = both_syms!(p, "probe_node_count", FnI);
        assert_eq!(unsafe { ccnt() }, unsafe { rcnt() }, "node_count diverged");
        rcs
    }

    fn assert_storage_matches(count: c_int) {
        let p = shadow();
        let (cid, rid) = both_syms!(p, "probe_node_id", FnI2I);
        let (cpid, rpid) = both_syms!(p, "probe_node_parent_id", FnI2I);
        let (cval, rval) = both_syms!(p, "probe_node_value", FnI2D);
        let (cdat, rdat) = both_syms!(p, "probe_node_data", FnII2I);
        for i in 0..count {
            assert_eq!(unsafe { cid(i) }, unsafe { rid(i) }, "node[{i}].id");
            assert_eq!(
                unsafe { cpid(i) },
                unsafe { rpid(i) },
                "node[{i}].parent_id"
            );
            let (cv, rv) = (unsafe { cval(i) }, unsafe { rval(i) });
            assert_eq!(
                cv.to_bits(),
                rv.to_bits(),
                "node[{i}].value bits: C={:#x} Rust={:#x}",
                cv.to_bits(),
                rv.to_bits()
            );
            for k in 0..4 {
                assert_eq!(
                    unsafe { cdat(i, k) },
                    unsafe { rdat(i, k) },
                    "node[{i}].data[{k}]"
                );
            }
        }
    }

    #[test]
    fn deep_add_node_and_find_node_by_id() {
        let _g = lock();
        let p = shadow();
        let mut rng = Rng::new(0x3004);

        for _round in 0..300 {
            let n = rng.below(30) as usize;
            let nodes: Vec<(c_int, c_int, c_double)> = (0..n)
                .map(|_| {
                    (
                        // Small id space on purpose, so duplicates occur and
                        // find_node_by_id's "first match wins" is exercised.
                        (rng.below(12) as c_int) - 2,
                        (rng.below(12) as c_int) - 2,
                        rng.shaped_f64(),
                    )
                })
                .collect();
            populate(&nodes);
            assert_storage_matches(n as c_int);

            let (cfind, rfind) = both_syms!(p, "probe_find", FnI2I);
            for id in -6..=16 {
                assert_eq!(
                    unsafe { cfind(id) },
                    unsafe { rfind(id) },
                    "find_node_by_id({id}) index mismatch"
                );
            }
            for &id in &ARG_BOUNDARIES {
                assert_eq!(unsafe { cfind(id) }, unsafe { rfind(id) }, "find({id})");
            }
        }
    }

    #[test]
    fn deep_add_node_capacity_limit() {
        let _g = lock();
        let p = shadow();
        let (cstatus, _) = both_syms!(p, "probe_status", FnI2I);
        let max_nodes = unsafe { cstatus(4) };
        let status_error = unsafe { cstatus(2) };
        assert_eq!(max_nodes, 100);
        assert_eq!(status_error, 2);

        // Fill to exactly MAX_NODES, then overflow by a wide margin.
        let nodes: Vec<(c_int, c_int, c_double)> = (0..(max_nodes + 25))
            .map(|i| (i, i - 1, i as c_double + 0.25))
            .collect();
        let rcs = populate(&nodes);

        for (i, &(cv, rv)) in rcs.iter().enumerate() {
            let expected = if (i as c_int) < max_nodes { 0 } else { status_error };
            assert_eq!(cv, rv, "add_node #{i}");
            assert_eq!(
                cv, expected,
                "add_node #{i} should return {expected} (STATUS_OK below MAX_NODES, \
                 STATUS_ERROR at/after it)"
            );
        }

        let (ccnt, rcnt) = both_syms!(p, "probe_node_count", FnI);
        assert_eq!(unsafe { ccnt() }, max_nodes);
        assert_eq!(unsafe { rcnt() }, max_nodes);
        assert_storage_matches(max_nodes);

        // jumpnode over a full table, incl. the mode-4 backward scan.
        let (cj, rj) = both_syms!(p, "jumpnode", FnJump);
        for &m in &[1, 2, 3, 4, 0] {
            for id in [-1, 0, 1, 50, 99, 100, 101] {
                for depth in [0, 1, 3, 16, 99, 200] {
                    let cv = unsafe { cj(m, id, depth, 7) };
                    let rv = unsafe { rj(m, id, depth, 7) };
                    assert_eq!(cv, rv, "full-table jumpnode({m},{id},{depth},7)");
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // jumpnode mode 0001 with populated storage (rows 10, 11, 12)
    // ------------------------------------------------------------------
    #[test]
    fn deep_mode1_parent_walk_proper_trees() {
        let _g = lock();
        let p = shadow();
        let (cj, rj) = both_syms!(p, "jumpnode", FnJump);
        let mut rng = Rng::new(0x3005);

        for _round in 0..250 {
            let n = 1 + (rng.below(20) as usize);
            // A proper tree: node 0 is the root (parent_id == -1), every other
            // node's parent is an earlier node, so the walk always terminates
            // on the parent_id == -1 sentinel (ERRORS.md row 10).
            let nodes: Vec<(c_int, c_int, c_double)> = (0..n)
                .map(|i| {
                    let id = i as c_int + 1;
                    let parent = if i == 0 {
                        -1
                    } else {
                        (rng.below(i as u64) as c_int) + 1
                    };
                    (id, parent, rng.shaped_f64())
                })
                .collect();
            populate(&nodes);

            // Depth is unbounded here: the sentinel terminates the walk.
            for id in 0..=(n as c_int + 2) {
                for &depth in &[
                    i32::MIN,
                    -1,
                    0,
                    1,
                    2,
                    3,
                    n as i32,
                    n as i32 + 1,
                    1000,
                    i32::MAX,
                ] {
                    let f = rng.i32();
                    let cv = unsafe { cj(1, id, depth, f) };
                    let rv = unsafe { rj(1, id, depth, f) };
                    assert_eq!(cv, rv, "mode1 tree jumpnode(1,{id},{depth},{f})");
                }
            }
        }
    }

    #[test]
    fn deep_mode1_dangling_parents_and_cycles() {
        let _g = lock();
        let p = shadow();
        let (cj, rj) = both_syms!(p, "jumpnode", FnJump);
        let mut rng = Rng::new(0x3006);

        for _round in 0..400 {
            let n = 1 + (rng.below(12) as usize);
            // Arbitrary parent links: dangling ids (row 11), self-loops and
            // cycles all occur. Depth is bounded because a cycle makes the walk
            // run exactly `depth` times.
            let nodes: Vec<(c_int, c_int, c_double)> = (0..n)
                .map(|_| {
                    (
                        (rng.below(8) as c_int) - 1,
                        (rng.below(10) as c_int) - 2,
                        rng.shaped_f64(),
                    )
                })
                .collect();
            populate(&nodes);

            for id in -2..=8 {
                for depth in [-1, 0, 1, 2, 3, 5, 17, 64, 255] {
                    let f = rng.i32();
                    let cv = unsafe { cj(1, id, depth, f) };
                    let rv = unsafe { rj(1, id, depth, f) };
                    assert_eq!(cv, rv, "mode1 graph jumpnode(1,{id},{depth},{f})");
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // jumpnode mode 0002 with populated storage
    // ------------------------------------------------------------------
    #[test]
    fn deep_mode2_array_backward_sum() {
        let _g = lock();
        let p = shadow();
        let (cj, rj) = both_syms!(p, "jumpnode", FnJump);
        let mut rng = Rng::new(0x3007);

        for _round in 0..300 {
            let n = 1 + (rng.below(10) as usize);
            let nodes: Vec<(c_int, c_int, c_double)> = (0..n)
                .map(|i| (i as c_int + 1, i as c_int, rng.shaped_f64()))
                .collect();
            populate(&nodes);

            // depth is process_backward's start_offset. Only [0, 22] is used:
            // negative offsets make the C read before `temp_array`, which is
            // undefined behaviour reading stack garbage — there is no defined
            // C result to match against.
            for id in 0..=(n as c_int + 1) {
                for depth in 0..=22 {
                    for &f in &[0, 1, -1, 7, 127, i32::MAX, i32::MIN, 134_217_728] {
                        let cv = unsafe { cj(2, id, depth, f) };
                        let rv = unsafe { rj(2, id, depth, f) };
                        assert_eq!(cv, rv, "mode2 jumpnode(2,{id},{depth},{f})");
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // jumpnode mode 0004 with populated storage (row 13)
    // ------------------------------------------------------------------
    #[test]
    fn deep_mode4_sqrt_and_backward_scan() {
        let _g = lock();
        let p = shadow();
        let (cj, rj) = both_syms!(p, "jumpnode", FnJump);
        let mut rng = Rng::new(0x3008);

        // node_count 0..6 straddles the `node_count > 2` guard and the
        // `iter > node_storage` guard of the 3-step backward scan.
        for n in 0usize..=6 {
            for _round in 0..120 {
                let nodes: Vec<(c_int, c_int, c_double)> = (0..n)
                    .map(|i| (i as c_int + 1, i as c_int, rng.shaped_f64()))
                    .collect();
                populate(&nodes);

                for id in 0..=(n as c_int + 1) {
                    for &depth in &[
                        i32::MIN,
                        -1000,
                        -100,
                        -11,
                        -10,
                        -9,
                        -1,
                        0,
                        1,
                        10,
                        1000,
                        i32::MAX,
                    ] {
                        let f = rng.i32();
                        let cv = unsafe { cj(4, id, depth, f) };
                        let rv = unsafe { rj(4, id, depth, f) };
                        assert_eq!(cv, rv, "mode4 jumpnode(4,{id},{depth},{f}) n={n}");
                    }
                }
            }
        }
    }

    /// High-resolution `depth` sweep for mode 0004.
    ///
    /// `data[]` is always `{0100,0200,0300,0400}`, so the pre-scale accumulation
    /// is the fixed constant `sum(sqrt(d)) * 2.718281828 = 133.658229975...`.
    /// A tiny error in that constant (or in the sqrt) is invisible at small
    /// `depth` because `safe_double_to_int` truncates. The `1.0 + depth*0.1`
    /// scale amplifies it by ~`depth/10`, so this sweep drives `depth` up to the
    /// point just before the clamp (`~1.607e8`) where the amplification is ~1e7
    /// and a sub-ulp constant error moves the truncated result.
    #[test]
    fn deep_mode4_depth_high_resolution_sweep() {
        let _g = lock();
        let p = shadow();
        let (cj, rj) = both_syms!(p, "jumpnode", FnJump);

        // Node values are 0.0 so the backward scan contributes nothing and the
        // sqrt/scale arithmetic is isolated.
        let nodes: Vec<(c_int, c_double)> = (1..=4).map(|i| (i, 0.0)).collect();
        let nodes: Vec<(c_int, c_int, c_double)> =
            nodes.into_iter().map(|(i, v)| (i, 0, v)).collect();
        populate(&nodes);

        let mut rng = Rng::new(0x300b);
        // The clamp kicks in around depth 1.607e8; sweep the whole sensitive
        // range plus well past the clamp on both sides.
        for _ in 0..40_000 {
            let depth = rng.below(170_000_000) as c_int;
            for d in [depth, -depth] {
                let cv = unsafe { cj(4, 1, d, 0) };
                let rv = unsafe { rj(4, 1, d, 0) };
                assert_eq!(cv, rv, "mode4 hi-res depth={d}: C={cv} Rust={rv}");
            }
        }

        // Exhaustive walk of the immediate clamp neighbourhood.
        for d in 160_669_700..160_669_830 {
            let cv = unsafe { cj(4, 1, d, 0) };
            let rv = unsafe { rj(4, 1, d, 0) };
            assert_eq!(cv, rv, "mode4 clamp edge depth={d}: C={cv} Rust={rv}");
            let cv = unsafe { cj(4, 1, -d, 0) };
            let rv = unsafe { rj(4, 1, -d, 0) };
            assert_eq!(cv, rv, "mode4 clamp edge depth={}: C={cv} Rust={rv}", -d);
        }

        // Dense low range, where the result changes by small steps.
        for d in -20_000..20_000 {
            let cv = unsafe { cj(4, 1, d, 0) };
            let rv = unsafe { rj(4, 1, d, 0) };
            assert_eq!(cv, rv, "mode4 dense depth={d}: C={cv} Rust={rv}");
        }
    }

    // ------------------------------------------------------------------
    // initialize_test_data itself
    // ------------------------------------------------------------------
    #[test]
    fn deep_initialize_test_data_and_full_jumpnode_sweep() {
        let _g = lock();
        let p = shadow();
        let (creset, rreset) = both_syms!(p, "probe_reset", FnV);
        let (cinit, rinit) = both_syms!(p, "probe_init", FnI);
        let (cj, rj) = both_syms!(p, "jumpnode", FnJump);

        unsafe {
            creset();
            rreset();
        }
        let (cn, rn) = (unsafe { cinit() }, unsafe { rinit() });
        assert_eq!(cn, rn, "initialize_test_data left different node_count");
        assert_eq!(cn, 7, "the C adds 7 nodes");
        assert_storage_matches(cn);

        let mut rng = Rng::new(0x3009);
        // The full 4-axis sweep, now over the state initialize_test_data builds.
        for &m in &[1, 2, 3, 4, 0, 5, i32::MIN, i32::MAX] {
            for id in -2..=10 {
                // mode 2 uses depth as an array offset: keep it in-bounds.
                let depths: Vec<i32> = if m == 2 {
                    (0..=20).collect()
                } else {
                    vec![i32::MIN, -1000, -1, 0, 1, 2, 3, 7, 8, 1000, i32::MAX]
                };
                for depth in depths {
                    for _ in 0..3 {
                        let f = rng.shaped_i32();
                        let cv = unsafe { cj(m, id, depth, f) };
                        let rv = unsafe { rj(m, id, depth, f) };
                        assert_eq!(cv, rv, "init-state jumpnode({m},{id},{depth},{f})");
                    }
                }
            }
        }

        // Idempotence: calling init repeatedly must keep both in lockstep.
        for _ in 0..5 {
            let (cn2, rn2) = (unsafe { cinit() }, unsafe { rinit() });
            assert_eq!(cn2, rn2);
            assert_eq!(cn2, 7);
            assert_storage_matches(cn2);
        }

        // Reset must return both to the pristine .bss state, where the public
        // API's sentinels reappear.
        unsafe {
            creset();
            rreset();
        }
        for (m, expect) in [(1, 18), (2, 34), (4, 66), (0, 130)] {
            let cv = unsafe { cj(m, 1, 3, 0) };
            let rv = unsafe { rj(m, 1, 3, 0) };
            assert_eq!(cv, rv);
            assert_eq!(cv, expect, "after reset, mode {m} must return {expect}");
        }
    }

    // ------------------------------------------------------------------
    // Randomized end-to-end fuzz over arbitrary state + arbitrary args
    // ------------------------------------------------------------------
    #[test]
    fn deep_full_pipeline_fuzz() {
        let _g = lock();
        let p = shadow();
        let (cj, rj) = both_syms!(p, "jumpnode", FnJump);
        let mut rng = Rng::new(0x300a);

        for _round in 0..600 {
            let n = rng.below(15) as usize;
            let nodes: Vec<(c_int, c_int, c_double)> = (0..n)
                .map(|_| {
                    (
                        (rng.below(10) as c_int) - 2,
                        if rng.below(4) == 0 {
                            -1
                        } else {
                            (rng.below(10) as c_int) - 2
                        },
                        rng.shaped_f64(),
                    )
                })
                .collect();
            populate(&nodes);
            assert_storage_matches(n as c_int);

            for _ in 0..60 {
                let m = rng.pick(&[1, 2, 3, 4, 0, 9, -1]);
                let id = (rng.below(14) as c_int) - 3;
                // Bound depth: cycles make mode 1 iterate `depth` times, and
                // mode 2 uses depth as an in-array offset.
                let depth = if m == 2 {
                    rng.below(23) as c_int
                } else {
                    (rng.below(300) as c_int) - 40
                };
                let f = rng.shaped_i32();
                let cv = unsafe { cj(m, id, depth, f) };
                let rv = unsafe { rj(m, id, depth, f) };
                assert_eq!(cv, rv, "fuzz jumpnode({m},{id},{depth},{f}) n={n}");
            }
        }
    }
}
