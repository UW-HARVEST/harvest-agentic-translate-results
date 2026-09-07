//! Phase C — error-path differential tests. One test per ERRORS.md row.
//! Each test constructs the exact rejecting condition, calls BOTH `.so`s, and
//! asserts the SAME error code / sentinel (not merely "both failed").
//!
//! Ground-truth constants (from c_src/src/lib.c):
//!   STATUS_OK 0000=0  STATUS_WARNING 0001=1
//!   STATUS_ERROR 0002=2  STATUS_CRITICAL 0377=255

mod common;

use common::{Pair, Rng, VALID_MODES};

const STATUS_ERROR: i32 = 0o2;
const ERR_MODE1: i32 = STATUS_ERROR | 0o20; // 18
const ERR_MODE2: i32 = STATUS_ERROR | 0o40; // 34
const ERR_MODE4: i32 = STATUS_ERROR | 0o100; // 66
const ERR_DEFAULT: i32 = STATUS_ERROR | 0o200; // 130

const SEED: u64 = 0xBADC0DE_9999;

// ------------------------------------------------------------------ row 1
// case 0001: find_node_by_id() == NULL  ->  return STATUS_ERROR | 0020
#[test]
fn err01_mode1_node_not_found_returns_18() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 1);
    // Named/likely ids first, then a broad randomized sweep: node_count is 0,
    // so this branch must fire for EVERY node_id.
    for node_id in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 99, 100, i32::MIN, i32::MAX] {
        let (c, r) = p.call(0o1, node_id, 0, 0);
        assert_eq!(c, r, "mode1 node_id={node_id}: C={c} Rust={r}");
        assert_eq!(
            c, ERR_MODE1,
            "C ground truth for mode1/node_id={node_id} must be STATUS_ERROR|0020=18, got {c}"
        );
    }
    for _ in 0..common::iters(30_000) {
        let (n, d, f) = (rng.next_i32_biased(), rng.next_i32_biased(), rng.next_i32_biased());
        let (c, r) = p.call(0o1, n, d, f);
        assert_eq!(c, r, "mode1({n},{d},{f}): C={c} Rust={r}");
        assert_eq!(c, ERR_MODE1, "mode1({n},{d},{f}) C returned {c}, expected 18");
    }
}

// ------------------------------------------------------------------ row 2
// case 0002: find_node_by_id() == NULL  ->  return STATUS_ERROR | 0040
#[test]
fn err02_mode2_node_not_found_returns_34() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 2);
    for node_id in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 99, 100, i32::MIN, i32::MAX] {
        let (c, r) = p.call(0o2, node_id, 0, 0);
        assert_eq!(c, r, "mode2 node_id={node_id}: C={c} Rust={r}");
        assert_eq!(c, ERR_MODE2, "C ground truth mode2/node_id={node_id} must be 34, got {c}");
    }
    for _ in 0..common::iters(30_000) {
        let (n, d, f) = (rng.next_i32_biased(), rng.next_i32_biased(), rng.next_i32_biased());
        let (c, r) = p.call(0o2, n, d, f);
        assert_eq!(c, r, "mode2({n},{d},{f}): C={c} Rust={r}");
        assert_eq!(c, ERR_MODE2, "mode2({n},{d},{f}) C returned {c}, expected 34");
    }
}

// ------------------------------------------------------------------ row 3
// case 0004: find_node_by_id() == NULL  ->  return STATUS_ERROR | 0100
#[test]
fn err03_mode4_node_not_found_returns_66() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 3);
    for node_id in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 99, 100, i32::MIN, i32::MAX] {
        let (c, r) = p.call(0o4, node_id, 0, 0);
        assert_eq!(c, r, "mode4 node_id={node_id}: C={c} Rust={r}");
        assert_eq!(c, ERR_MODE4, "C ground truth mode4/node_id={node_id} must be 66, got {c}");
    }
    for _ in 0..common::iters(30_000) {
        let (n, d, f) = (rng.next_i32_biased(), rng.next_i32_biased(), rng.next_i32_biased());
        let (c, r) = p.call(0o4, n, d, f);
        assert_eq!(c, r, "mode4({n},{d},{f}): C={c} Rust={r}");
        assert_eq!(c, ERR_MODE4, "mode4({n},{d},{f}) C returned {c}, expected 66");
    }
}

// ------------------------------------------------------------------ row 4
// default: -> STATUS_ERROR | 0200. Out-of-range "enum" values across FFI.
#[test]
fn err04_default_arm_out_of_range_enum_returns_130() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 4);

    // Values immediately past the documented valid range on both sides.
    for m in [-2i32, -1, 0, 5, 6, 7, 8, 0o10, 0o377, 255, 256] {
        let (c, r) = p.call(m, 0, 0, 0);
        assert_eq!(c, r, "mode={m}: C={c} Rust={r}");
        assert_eq!(c, ERR_DEFAULT, "C ground truth for mode={m} must be 130, got {c}");
    }

    // Extremes and every power of two / its negation.
    let mut modes: Vec<i32> = vec![i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1];
    for k in 0..32 {
        modes.push(1i32.wrapping_shl(k));
        modes.push(1i32.wrapping_shl(k).wrapping_neg());
    }
    for &m in &modes {
        if VALID_MODES.contains(&m) {
            continue;
        }
        let (c, r) = p.call(m, 0, 0, 0);
        assert_eq!(c, r, "mode={m}: C={c} Rust={r}");
        assert_eq!(c, ERR_DEFAULT, "C for mode={m} returned {c}, expected 130");
    }

    // Exhaustive small range + randomized wide sweep.
    for m in -4096i32..=4096 {
        if VALID_MODES.contains(&m) {
            continue;
        }
        let (c, r) = p.call(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(c, r, "mode={m}: C={c} Rust={r}");
        assert_eq!(c, ERR_DEFAULT, "C for mode={m} returned {c}, expected 130");
    }
    for _ in 0..common::iters(100_000) {
        let m = rng.next_i32();
        if VALID_MODES.contains(&m) {
            continue;
        }
        let (c, r) = p.call(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(c, r, "mode={m}: C={c} Rust={r}");
        assert_eq!(c, ERR_DEFAULT, "C for mode={m} returned {c}, expected 130");
    }
}

// ------------------------------------------------------------------ row 5
// find_node_by_id returns NULL for every id (node_count == 0). The NULL
// sentinel is only observable through rows 1/2/4, so assert all three agree
// for the SAME id — i.e. the lookup failed identically in all three arms.
#[test]
fn err05_find_node_by_id_null_sentinel_consistent_across_arms() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..common::iters(10_000) {
        let id = rng.next_i32_biased();
        let (c1, r1) = p.call(0o1, id, 0, 0);
        let (c2, r2) = p.call(0o2, id, 0, 0);
        let (c4, r4) = p.call(0o4, id, 0, 0);
        assert_eq!((c1, c2, c4), (r1, r2, r4), "id={id}: C={:?} Rust={:?}", (c1, c2, c4), (r1, r2, r4));
        assert_eq!(
            (c1, c2, c4),
            (ERR_MODE1, ERR_MODE2, ERR_MODE4),
            "id={id}: lookup did not fail in all three arms (C gave {:?})",
            (c1, c2, c4)
        );
    }
}

// ------------------------------------------------------------------ row 6
// add_node: node_count >= MAX_NODES -> STATUS_ERROR. Internal linkage, and its
// only caller `initialize_test_data` is never called, so it is unreachable
// through the ABI. Prove that unreachability: if add_node had ever run,
// node_count would be 7 and modes 1/2/4 would NOT return their NULL errors.
#[test]
fn err06_add_node_and_storage_never_populated() {
    let p = Pair::load();
    for id in 1..=7i32 {
        for depth in [0, 1, 5] {
            let (c1, r1) = p.call(0o1, id, depth, 0);
            let (c2, r2) = p.call(0o2, id, depth, 0);
            let (c4, r4) = p.call(0o4, id, depth, 0);
            assert_eq!((c1, c2, c4), (r1, r2, r4));
            // The ids initialize_test_data() *would* have created still miss.
            assert_eq!(
                (c1, c2, c4),
                (ERR_MODE1, ERR_MODE2, ERR_MODE4),
                "node storage is unexpectedly populated for id={id} (C gave {:?}) — \
                 initialize_test_data must remain uncalled",
                (c1, c2, c4)
            );
        }
    }
    // Also confirm no node id anywhere in 0..MAX_NODES resolves.
    for id in 0..100i32 {
        let (c, r) = p.call(0o1, id, 0, 0);
        assert_eq!(c, r);
        assert_eq!(c, ERR_MODE1, "id={id} resolved unexpectedly");
    }
}

// ------------------------------------------------------------------ rows 7/8/9
// safe_double_to_int clamps (> INT_MAX, < INT_MIN) and NaN. Reached only from
// modes 1 and 4, both of which return their NULL error first, so the clamps are
// ABI-unreachable. Assert the arms that would use them still short-circuit for
// the inputs that would have driven the accumulator out of range.
#[test]
fn err07_08_09_safe_double_to_int_clamps_unreachable_but_consistent() {
    let p = Pair::load();
    // depth values that would blow up `1.0 + depth*0.1` (mode 4) or the
    // accumulation loop (mode 1) if a node existed.
    let depths = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 1 << 30, -(1 << 30), 0, 1, -1];
    for &d in &depths {
        for node_id in [0, 1, 7, -1, i32::MIN, i32::MAX] {
            for f in [0, -1, i32::MAX, i32::MIN] {
                let (c1, r1) = p.call(0o1, node_id, d, f);
                assert_eq!(c1, r1, "mode1({node_id},{d},{f}): C={c1} Rust={r1}");
                assert_eq!(c1, ERR_MODE1);
                let (c4, r4) = p.call(0o4, node_id, d, f);
                assert_eq!(c4, r4, "mode4({node_id},{d},{f}): C={c4} Rust={r4}");
                assert_eq!(c4, ERR_MODE4);
            }
        }
    }
}

// ------------------------------------------------------------------ rows 10/11
// process_backward: start_offset >= size (depth >= 16) -> returns 0;
// start_offset < 0 -> out-of-bounds backwards read. Reached only from mode 2,
// which returns ERR_MODE2 first. Sweep the exact boundary values.
#[test]
fn err10_11_process_backward_offsets_unreachable_but_consistent() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 10);
    let mut depths: Vec<i32> = vec![i32::MIN, i32::MIN + 1, -1024, -20, -19, -17, -16, -4, -1];
    depths.extend(0..=24);
    depths.extend([1024, i32::MAX - 1, i32::MAX]);
    for &d in &depths {
        for f in [0, 1, -1, i32::MAX, i32::MIN, 134_217_728, 268_435_456] {
            let (c, r) = p.call(0o2, rng.next_i32_biased(), d, f);
            assert_eq!(c, r, "mode2(depth={d}, flags={f}): C={c} Rust={r}");
            assert_eq!(c, ERR_MODE2, "mode2(depth={d}, flags={f}) C returned {c}, expected 34");
        }
    }
}

// ------------------------------------------------------------------ row 16
// compute_size_metric with strlen == 0 -> 8. Not reachable through jumpnode:
// sprintf always emits at least "Node_0_Depth_0" (14 chars) so mode 3's metric
// is always >= 14*2+8 = 36 before the flag mask. Assert that lower bound holds
// on both sides for a wide randomized sweep.
#[test]
fn err16_compute_size_metric_never_sees_empty_string() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..common::iters(50_000) {
        let (n, d) = (rng.next_i32_biased(), rng.next_i32_biased());
        let (c, r) = p.call(0o3, n, d, 0);
        assert_eq!(c, r, "mode3({n},{d},0): C={c} Rust={r}");
        assert!(
            c >= 36,
            "mode3({n},{d},0) C returned {c}; buffer can never be shorter than \
             \"Node_0_Depth_0\" so metric must be >= 36"
        );
        assert!(c <= 76, "mode3({n},{d},0) C returned {c}; max buffer is 34 chars -> 76");
    }
}

// ------------------------------------------------------------------ row 17
// case 0003: flags masked with 0177 — high bits and the sign bit must be
// discarded identically.
#[test]
fn err17_mode3_flag_mask_discards_high_bits() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..common::iters(50_000) {
        let (n, d, f) = (rng.next_i32_biased(), rng.next_i32_biased(), rng.next_i32());
        let (c, r) = p.call(0o3, n, d, f);
        assert_eq!(c, r, "mode3({n},{d},{f}): C={c} Rust={r}");
        // Changing only the bits above 0177 must not change the result.
        let f2 = f ^ !0o177i32;
        let (c2, r2) = p.call(0o3, n, d, f2);
        assert_eq!(c2, r2, "mode3({n},{d},{f2}): C={c2} Rust={r2}");
        assert_eq!(
            c, c2,
            "C: masking bug — flags {f} vs {f2} differ only above 0177 but gave {c} vs {c2}"
        );
    }
    // flags = -1 must contribute exactly 127.
    let (c0, r0) = p.call(0o3, 0, 0, 0);
    let (cm, rm) = p.call(0o3, 0, 0, -1);
    assert_eq!(c0, r0);
    assert_eq!(cm, rm);
    assert_eq!(cm - c0, 0o177, "flags=-1 must add 0177=127, added {}", cm - c0);
}

// ------------------------------------------------------------------ row 18
// case 0003 with INT_MIN: %d widest output, buffer[50] must not overflow.
#[test]
fn err18_mode3_int_min_widest_format_no_overflow() {
    let p = Pair::load();
    for &n in &[i32::MIN, i32::MIN + 1, i32::MAX] {
        for &d in &[i32::MIN, i32::MIN + 1, i32::MAX] {
            for f in [0, 0o177, -1, i32::MIN, i32::MAX] {
                let (c, r) = p.call(0o3, n, d, f);
                assert_eq!(c, r, "mode3({n},{d},{f}): C={c} Rust={r}");
            }
        }
    }
    // Exact widest case: strlen("Node_-2147483648_Depth_-2147483648") == 34.
    let (c, r) = p.call(0o3, i32::MIN, i32::MIN, 0);
    assert_eq!(c, r);
    assert_eq!(c, 34 * 2 + 0o10, "widest buffer metric must be 76, C gave {c}");
}

// ------------------------------------------------------------------ generic
// Generic FFI boundaries: all-zero, all-INT_MIN, all-INT_MAX argument vectors.
#[test]
fn err_generic_extreme_argument_vectors() {
    let p = Pair::load();
    let extremes = [0i32, 1, -1, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1];
    for &a in &extremes {
        for &b in &extremes {
            for &cc in &extremes {
                for &d in &extremes {
                    p.assert_same(a, b, cc, d);
                }
            }
        }
    }
}

// The ABI takes four `int`s and no pointers, so null-pointer and buffer-length
// abuse are not expressible. Confirm the exported symbol has the expected
// arity/type by calling it through a correctly-typed FFI signature above; a
// mismatch would fault rather than return.
#[test]
fn err_generic_no_pointer_parameters_in_abi() {
    let p = Pair::load();
    // Sanity: repeated calls with the extreme vector do not corrupt state.
    for _ in 0..1000 {
        p.assert_same(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
        p.assert_same(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    }
}
