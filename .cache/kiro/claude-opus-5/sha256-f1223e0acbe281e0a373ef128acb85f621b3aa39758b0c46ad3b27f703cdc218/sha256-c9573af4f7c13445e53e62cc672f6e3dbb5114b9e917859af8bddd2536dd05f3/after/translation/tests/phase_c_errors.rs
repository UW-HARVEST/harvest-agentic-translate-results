//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (rows 1..39). Each constructs the exact
//! invalid input/condition, calls BOTH `.so`s, and asserts they return the SAME
//! sentinel — not merely "both failed".

mod common;
use common::*;

use std::os::unix::process::ExitStatusExt;

fn signal_of(out: &std::process::Output) -> Option<i32> {
    out.status.signal()
}

// ---------------------------------------------------------------------------
// add_node
// ---------------------------------------------------------------------------

/// Row 1: `node_count >= MAX_NODES` → `-1`.
#[test]
fn err_01_add_node_capacity_exhausted() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 101);
    for k in 0..MAX_NODES {
        assert_eq!(p.add_node(k as i32 + 1, -1, b"fill", 1.0), k as i32);
    }
    // every further insertion is rejected with exactly -1, forever
    for _ in 0..200 {
        let nlen = rng.below(60) as usize;
        let name = rng.bytes(nlen, 0x01, 0xFF);
        assert_eq!(
            p.add_node(rng.next_i32(), rng.next_i32(), &name, rng.next_f64_bits()),
            -1,
            "add_node past MAX_NODES must return -1"
        );
    }
    // and the store is unchanged: node 100 still last visible, no node 101
    p.find_node_by_id(100);
    assert!(p.find_node_by_id(101).is_none());
    assert_eq!(p.get_children_count(-1), MAX_NODES as i32);
}

/// Row 2: boundary — the 100th insertion (index 99) must still succeed.
#[test]
fn err_02_add_node_last_slot_succeeds() {
    let p = Pair::fresh();
    for k in 0..(MAX_NODES - 1) {
        assert_eq!(p.add_node(k as i32 + 1, -1, b"f", 1.0), k as i32);
    }
    assert_eq!(p.add_node(100, -1, b"last", 9.5), 99, "index 99 must succeed");
    assert_eq!(p.add_node(101, -1, b"over", 9.5), -1, "index 100 must fail");
    p.find_node_by_id(100);
    assert!(p.find_node_by_id(101).is_none());
}

/// Row 3: name longer than `MAX_NAME_LEN - 1` → truncated, `name[49] == 0`.
#[test]
fn err_03_add_node_name_overlong_truncates() {
    let mut rng = Rng::new(SEED ^ 103);
    for len in [49usize, 50, 51, 64, 100, 1000] {
        for &(lo, hi) in &[(b'a', b'z'), (0x01u8, 0xFFu8), (0x80, 0xFF)] {
            let p = Pair::fresh();
            let name = rng.bytes(len, lo, hi);
            assert_eq!(p.add_node(1, -1, &name, 1.0), 0);
            let (cp, rp) = p.find_node_by_id(1).expect("node 1");
            for ptr in [cp, rp] {
                let n = unsafe { &*ptr };
                assert_eq!(n.name[MAX_NAME_LEN - 1], 0, "name[49] must be NUL");
                let stored: Vec<u8> = n.name[..MAX_NAME_LEN - 1].iter().map(|&b| b as u8).collect();
                assert_eq!(stored, &name[..MAX_NAME_LEN - 1], "first 49 bytes must be copied");
            }
            p.process_node_name(1);
        }
    }
}

/// Row 4: `name = NULL` → `strncpy` NULL deref. Both must die the same way.
#[test]
fn err_04_null_name_crashes_both() {
    if let Some(mode) = crash_mode() {
        let p = Pair::fresh();
        let lib = match mode.as_str() {
            "c" => &p.c,
            "r" => &p.r,
            _ => return,
        };
        let v = unsafe { (lib.add_node)(1, -1, std::ptr::null(), 1.0) };
        eprintln!("NO CRASH: add_node returned {v}");
        std::process::exit(77);
    }
    let c = run_isolated("err_04_null_name_crashes_both", "c");
    let r = run_isolated("err_04_null_name_crashes_both", "r");
    assert_deadly_signals_match("add_node(NULL name)", &c, &r);
}

/// SIGSEGV.
const SIGSEGV: i32 = 11;
/// SIGABRT — what a Rust panic in an `extern "C"` fn turns into.
const SIGABRT: i32 = 6;

/// Both libraries must die on a NULL dereference, and with the *same* signal.
///
/// The C load faults with `SIGSEGV`. So does the release Rust `.so` — the
/// artifact that corresponds to the C shared library. A **debug** Rust build
/// additionally carries rustc's `-C debug-assertions` UB checks, which detect
/// the null dereference *before* it faults and panic ("null pointer
/// dereference occurred"); a panic escaping an `extern "C"` fn aborts, so the
/// debug `.so` dies with `SIGABRT` instead. That is the compiler's deliberate
/// UB tripwire on input that has no defined behaviour in C either, not a
/// behavioural divergence in the translation, so the exact-signal assertion is
/// only enforced when debug assertions are off. Both builds are still required
/// to die from a deadly signal rather than return.
#[track_caller]
fn assert_deadly_signals_match(
    what: &str,
    c: &std::process::Output,
    r: &std::process::Output,
) {
    let cs = signal_of(c);
    let rs = signal_of(r);
    let ctx = || {
        format!(
            "{what}: C signal={cs:?} status={:?}, Rust signal={rs:?} status={:?}\n\
             C stderr: {}\nRust stderr: {}",
            c.status.code(),
            r.status.code(),
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
        )
    };
    assert_eq!(cs, Some(SIGSEGV), "C must fault with SIGSEGV. {}", ctx());
    if cfg!(debug_assertions) {
        assert_eq!(
            rs,
            Some(SIGABRT),
            "debug Rust must trip the UB check and abort. {}",
            ctx()
        );
    } else {
        assert_eq!(cs, rs, "release Rust must fault identically to C. {}", ctx());
    }
}

// ---------------------------------------------------------------------------
// find_node_by_id
// ---------------------------------------------------------------------------

/// Row 5: no node has that id → `NULL`.
#[test]
fn err_05_find_absent_id_returns_null() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 105);
    for k in 0..20 {
        p.add_node(k + 1, -1, b"n", 1.0);
    }
    for probe in [21i32, 0, -1, -2, 100, 1000, i32::MIN, i32::MAX] {
        assert!(p.find_node_by_id(probe).is_none(), "id {probe} must be absent");
    }
    for _ in 0..5_000 {
        let id = rng.next_i32();
        let expect_present = (1..=20).contains(&id);
        assert_eq!(
            p.find_node_by_id(id).is_some(),
            expect_present,
            "presence of id {id}"
        );
    }
}

/// Row 6: empty store → `NULL`.
#[test]
fn err_06_find_on_empty_store_returns_null() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 106);
    for probe in [0i32, 1, -1, i32::MIN, i32::MAX] {
        assert!(p.find_node_by_id(probe).is_none());
    }
    for _ in 0..2_000 {
        assert!(p.find_node_by_id(rng.next_i32()).is_none());
    }
}

/// Row 7: node exists but `active == 0` → `NULL`.
#[test]
fn err_07_find_inactive_returns_null() {
    let p = Pair::fresh();
    for k in 0..10 {
        p.add_node(k + 1, -1, b"n", (k as f64) + 1.0);
    }
    for k in 0..10 {
        p.set_active(k + 1, 0);
        assert!(
            p.find_node_by_id(k + 1).is_none(),
            "deactivated node {} must be invisible",
            k + 1
        );
    }
    assert_eq!(p.get_children_count(-1), 0, "all inactive → 0 children");
}

/// Row 8: extremal ids → `NULL`.
#[test]
fn err_08_find_extremal_ids_return_null() {
    let p = Pair::fresh();
    p.add_node(5, -1, b"only", 1.0);
    for probe in [i32::MIN, i32::MIN + 1, -1, 0, 4, 6, i32::MAX - 1, i32::MAX] {
        assert!(p.find_node_by_id(probe).is_none(), "id {probe}");
    }
    assert!(p.find_node_by_id(5).is_some());
    // and with extremal ids actually stored, the neighbours must still miss
    let q = Pair::fresh();
    q.add_node(i32::MIN, i32::MAX, b"a", 1.0);
    q.add_node(i32::MAX, i32::MIN, b"b", 2.0);
    assert!(q.find_node_by_id(i32::MIN).is_some());
    assert!(q.find_node_by_id(i32::MAX).is_some());
    assert!(q.find_node_by_id(i32::MIN + 1).is_none());
    assert!(q.find_node_by_id(i32::MAX - 1).is_none());
    q.get_children_count(i32::MIN);
    q.get_children_count(i32::MAX);
}

// ---------------------------------------------------------------------------
// get_children_count
// ---------------------------------------------------------------------------

/// Row 9: no node has that `parent_id` → `0`.
#[test]
fn err_09_children_count_no_match_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 109);
    for k in 0..15 {
        p.add_node(k + 1, 7, b"n", 1.0);
    }
    for probe in [6i32, 8, 0, -1, i32::MIN, i32::MAX] {
        assert_eq!(p.get_children_count(probe), 0, "parent {probe}");
    }
    assert_eq!(p.get_children_count(7), 15);
    for _ in 0..3_000 {
        let q = rng.next_i32();
        assert_eq!(p.get_children_count(q), if q == 7 { 15 } else { 0 });
    }
}

/// Row 10: matching children all `active == 0` → `0`.
#[test]
fn err_10_children_count_all_inactive_zero() {
    let p = Pair::fresh();
    for k in 0..12 {
        p.add_node(k + 1, 3, b"n", 1.0);
    }
    assert_eq!(p.get_children_count(3), 12);
    for k in 0..12 {
        p.set_active(k + 1, 0);
    }
    assert_eq!(p.get_children_count(3), 0, "all inactive → 0");
    // reactivating one brings the count back to exactly 1
    let (cp, rp) = {
        // node 1 is invisible now, so poke the storage via a re-add is not
        // possible; instead re-activate through a fresh lookup of a still-live
        // node. Build a second pair to keep the assertion precise.
        let q = Pair::fresh();
        q.add_node(1, 3, b"n", 1.0);
        q.add_node(2, 3, b"n", 1.0);
        q.set_active(2, 0);
        assert_eq!(q.get_children_count(3), 1);
        q.find_node_by_id(1).unwrap()
    };
    let _ = (cp, rp);
}

/// Row 11: empty store → `0`.
#[test]
fn err_11_children_count_empty_store_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 111);
    for probe in [0i32, -1, 1, i32::MIN, i32::MAX] {
        assert_eq!(p.get_children_count(probe), 0);
    }
    for _ in 0..2_000 {
        assert_eq!(p.get_children_count(rng.next_i32()), 0);
    }
}

// ---------------------------------------------------------------------------
// calculate_subtree_sum
// ---------------------------------------------------------------------------

/// Row 12: node absent → `0.0` (positive zero).
#[test]
fn err_12_subtree_sum_absent_node_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 112);
    // empty store
    for probe in [0i32, 1, -1, i32::MIN, i32::MAX] {
        let v = p.calculate_subtree_sum(probe);
        assert_eq!(v.to_bits(), 0.0f64.to_bits(), "must be +0.0 for absent {probe}");
    }
    for k in 0..10 {
        p.add_node(k + 1, -1, b"n", -5.0);
    }
    for _ in 0..3_000 {
        let id = rng.next_i32();
        let v = p.calculate_subtree_sum(id);
        if !(1..=10).contains(&id) {
            assert_eq!(v.to_bits(), 0.0f64.to_bits(), "absent id {id} → +0.0");
        }
    }
}

/// Row 13: root present, every child inactive → only the root's own value.
#[test]
fn err_13_subtree_sum_inactive_children() {
    let mut rng = Rng::new(SEED ^ 113);
    for _ in 0..100 {
        let p = Pair::fresh();
        let root_val = rng.next_finite_f64();
        p.add_node(1, -1, b"root", root_val);
        let kids = rng.below(8) as usize + 1;
        for k in 0..kids {
            p.add_node(k as i32 + 2, 1, b"kid", rng.next_finite_f64());
        }
        for k in 0..kids {
            p.set_active(k as i32 + 2, 0);
        }
        let v = p.calculate_subtree_sum(1);
        assert_eq!(v.to_bits(), root_val.to_bits(), "only the root value must remain");
        // and once the root itself is inactive: +0.0
        p.set_active(1, 0);
        let v = p.calculate_subtree_sum(1);
        assert_eq!(v.to_bits(), 0.0f64.to_bits());
    }
}

/// Row 14: non-finite `value` propagates verbatim.
#[test]
fn err_14_subtree_sum_nonfinite_value() {
    let specials = [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::from_bits(0x7FF8_0000_0000_0001), // NaN with payload
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(1),                     // smallest subnormal
    ];
    for a in specials {
        for b in specials {
            let p = Pair::fresh();
            p.add_node(1, -1, b"r", a);
            p.add_node(2, 1, b"k", b);
            p.calculate_subtree_sum(1);
            p.calculate_subtree_sum(2);
            let s = p.calculate_subtree_sum(1);
            p.safe_double_to_int(s);
        }
    }
    // -0.0 root with no children must stay -0.0 (bitwise)
    let p = Pair::fresh();
    p.add_node(1, -1, b"r", -0.0);
    assert_eq!(p.calculate_subtree_sum(1).to_bits(), (-0.0f64).to_bits());
}

// ---------------------------------------------------------------------------
// process_string
// ---------------------------------------------------------------------------

/// Row 15: empty string → `0`.
#[test]
fn err_15_process_string_empty_returns_zero() {
    let p = Pair::fresh();
    assert_eq!(p.process_string(b""), 0);
    // via a stored node with an empty name too
    p.add_node(1, -1, b"", 1.0);
    assert_eq!(p.process_node_name(1), 0);
    // and a name that is only an interior NUL
    p.add_node(2, -1, b"\0tail", 1.0);
    assert_eq!(p.process_node_name(2), 0);
}

/// Row 16: `str = NULL` → NULL deref. Both must die the same way.
#[test]
fn err_16_null_string_crashes_both() {
    if let Some(mode) = crash_mode() {
        let p = Pair::fresh();
        let lib = match mode.as_str() {
            "c" => &p.c,
            "r" => &p.r,
            _ => return,
        };
        let v = unsafe { (lib.process_string)(std::ptr::null_mut()) };
        eprintln!("NO CRASH: process_string returned {v}");
        std::process::exit(77);
    }
    let c = run_isolated("err_16_null_string_crashes_both", "c");
    let r = run_isolated("err_16_null_string_crashes_both", "r");
    assert_deadly_signals_match("process_string(NULL)", &c, &r);
}

/// Row 17: bytes ≥ 0x80 sign-extend (signed `char` ABI) → total can go negative.
#[test]
fn err_17_process_string_high_bit_bytes_signed() {
    let p = Pair::fresh();
    assert_eq!(p.process_string(&[0x80]), -128, "0x80 must sign-extend to -128");
    assert_eq!(p.process_string(&[0xFF]), -1, "0xFF must sign-extend to -1");
    assert_eq!(p.process_string(&[0x7F]), 127);
    assert_eq!(p.process_string(&[0x80, 0x80]), -256);
    assert_eq!(p.process_string(&vec![0x80u8; 100]), -12_800);
    let mut rng = Rng::new(SEED ^ 117);
    for _ in 0..5_000 {
        let nlen = rng.below(64) as usize + 1;
        let s = rng.bytes(nlen, 0x80, 0xFF);
        let v = p.process_string(&s);
        assert!(v < 0, "all-high-bit string must sum negative, got {v}");
    }
}

/// Row 18: the `int` accumulator overflows and wraps.
#[test]
fn err_18_process_string_accumulator_overflow() {
    let p = Pair::fresh();
    // 20e6 * 127 = 2.54e9 > INT_MAX → wraps negative
    let n = 20_000_000usize;
    let v = p.process_string(&vec![0x7Fu8; n]);
    let expected = (n as i64 * 127) as u32 as i32;
    assert_eq!(v, expected, "wrapped accumulator");
    assert!(v < 0, "expected the wrap to land negative, got {v}");
    // and the negative direction
    let v2 = p.process_string(&vec![0x80u8; n]);
    let expected2 = ((n as i64) * -128) as i64 as u32 as i32;
    assert_eq!(v2, expected2);
}

// ---------------------------------------------------------------------------
// safe_double_to_int
// ---------------------------------------------------------------------------

/// Row 19: `d > (double)INT_MAX` → `INT_MAX`.
#[test]
fn err_19_sdti_above_int_max() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 119);
    for d in [
        2_147_483_648.0f64,
        2_147_483_647.5,
        2_147_483_647.000_001,
        1e18,
        1e300,
        f64::MAX,
    ] {
        assert_eq!(p.safe_double_to_int(d), i32::MAX, "{d} → INT_MAX");
    }
    for _ in 0..5_000 {
        let d = i32::MAX as f64 + 1.0 + (rng.next_u32() as f64);
        assert_eq!(p.safe_double_to_int(d), i32::MAX);
    }
}

/// Row 20: `d < (double)INT_MIN` → `INT_MIN`.
#[test]
fn err_20_sdti_below_int_min() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 120);
    for d in [
        -2_147_483_649.0f64,
        -2_147_483_648.5,
        -1e18,
        -1e300,
        f64::MIN,
    ] {
        assert_eq!(p.safe_double_to_int(d), i32::MIN, "{d} → INT_MIN");
    }
    for _ in 0..5_000 {
        let d = i32::MIN as f64 - 1.0 - (rng.next_u32() as f64);
        assert_eq!(p.safe_double_to_int(d), i32::MIN);
    }
}

/// Row 21: NaN reaches the `d != d` test only after both range tests → `0`.
#[test]
fn err_21_sdti_nan_returns_zero() {
    let p = Pair::fresh();
    assert_eq!(p.safe_double_to_int(f64::NAN), 0);
    assert_eq!(p.safe_double_to_int(-f64::NAN), 0);
    assert_eq!(p.safe_double_to_int(0.0 / 0.0), 0);
    assert_eq!(p.safe_double_to_int(f64::INFINITY - f64::INFINITY), 0);
    assert_eq!(p.safe_double_to_int(f64::INFINITY * 0.0), 0);
}

/// Row 22: `+inf` → `INT_MAX`.
#[test]
fn err_22_sdti_pos_inf() {
    let p = Pair::fresh();
    assert_eq!(p.safe_double_to_int(f64::INFINITY), i32::MAX);
    assert_eq!(p.safe_double_to_int(1.0 / 0.0), i32::MAX);
    assert_eq!(p.safe_double_to_int(f64::MAX * 2.0), i32::MAX);
}

/// Row 23: `-inf` → `INT_MIN`.
#[test]
fn err_23_sdti_neg_inf() {
    let p = Pair::fresh();
    assert_eq!(p.safe_double_to_int(f64::NEG_INFINITY), i32::MIN);
    assert_eq!(p.safe_double_to_int(-1.0 / 0.0), i32::MIN);
    assert_eq!(p.safe_double_to_int(f64::MIN * 2.0), i32::MIN);
}

/// Row 24: one representable step past each end of the valid range.
#[test]
fn err_24_sdti_one_step_past_range() {
    let p = Pair::fresh();
    // next double above (double)INT_MAX
    let up = f64::from_bits((i32::MAX as f64).to_bits() + 1);
    assert!(up > i32::MAX as f64);
    assert_eq!(p.safe_double_to_int(up), i32::MAX, "one ulp above INT_MAX");
    // next double below (double)INT_MIN (negative → increasing bits = more negative)
    let down = f64::from_bits((i32::MIN as f64).to_bits() + 1);
    assert!(down < i32::MIN as f64);
    assert_eq!(p.safe_double_to_int(down), i32::MIN, "one ulp below INT_MIN");
    for k in 1..=64u64 {
        assert_eq!(
            p.safe_double_to_int(f64::from_bits((i32::MAX as f64).to_bits() + k)),
            i32::MAX
        );
        assert_eq!(
            p.safe_double_to_int(f64::from_bits((i32::MIN as f64).to_bits() + k)),
            i32::MIN
        );
    }
}

/// Row 25: exactly on the boundary — NOT rejected (strict `>` / `<`).
#[test]
fn err_25_sdti_exact_boundaries() {
    let p = Pair::fresh();
    assert_eq!(p.safe_double_to_int(i32::MAX as f64), i32::MAX);
    assert_eq!(p.safe_double_to_int(i32::MIN as f64), i32::MIN);
    // one ulp *inside* the range
    let in_hi = f64::from_bits((i32::MAX as f64).to_bits() - 1);
    let in_lo = f64::from_bits((i32::MIN as f64).to_bits() - 1);
    p.safe_double_to_int(in_hi);
    p.safe_double_to_int(in_lo);
    for k in 1..=64u64 {
        p.safe_double_to_int(f64::from_bits((i32::MAX as f64).to_bits() - k));
        p.safe_double_to_int(f64::from_bits((i32::MIN as f64).to_bits() - k));
    }
}

/// Row 26: every NaN bit pattern → `0`.
#[test]
fn err_26_sdti_nan_payloads() {
    let p = Pair::fresh();
    let nans: [u64; 10] = [
        0x7FF8_0000_0000_0000, // canonical qNaN
        0xFFF8_0000_0000_0000, // -qNaN
        0x7FF8_0000_0000_0001,
        0xFFF8_0000_0000_0001,
        0x7FF0_0000_0000_0001, // sNaN
        0xFFF0_0000_0000_0001,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x7FF7_FFFF_FFFF_FFFF,
        0xFFF7_FFFF_FFFF_FFFF,
    ];
    for bits in nans {
        let d = f64::from_bits(bits);
        assert!(d.is_nan(), "0x{bits:016x} should be NaN");
        assert_eq!(p.safe_double_to_int(d), 0, "NaN 0x{bits:016x} → 0");
    }
    // random NaN payloads
    let mut rng = Rng::new(SEED ^ 126);
    for _ in 0..5_000 {
        let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        if payload == 0 {
            continue;
        }
        let sign = (rng.next_u64() & 1) << 63;
        let d = f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload);
        assert_eq!(p.safe_double_to_int(d), 0);
    }
}

/// Row 27: `-0.0` passes all guards → `0`.
#[test]
fn err_27_sdti_negative_zero() {
    let p = Pair::fresh();
    assert_eq!(p.safe_double_to_int(-0.0), 0);
    assert_eq!(p.safe_double_to_int(0.0), 0);
    assert_eq!(p.safe_double_to_int(-f64::MIN_POSITIVE), 0);
    assert_eq!(p.safe_double_to_int(f64::from_bits(0x8000_0000_0000_0001)), 0);
    assert_eq!(p.safe_double_to_int(-0.999_999_999_999), 0);
}

// ---------------------------------------------------------------------------
// maxnmin
// ---------------------------------------------------------------------------

/// Row 28: `(param1 % 6) + 1` names no node → first block skipped.
#[test]
fn err_28_maxnmin_selected_node_null() {
    let p = Pair::fresh();
    // C's % truncates toward zero, so p1 < 0 with p1 % 6 != 0 gives id <= 0.
    for p1 in [-1i32, -2, -3, -4, -5, -7, -11, -6001, i32::MIN, i32::MIN + 1] {
        let node_id = p1 % 6 + 1;
        assert!(
            !(1..=6).contains(&node_id),
            "p1={p1} should select a missing id, got {node_id}"
        );
        for p2 in [0i32, 1, 5, -1] {
            for p3 in [1i32, 0, -1] {
                for p4 in [0i32, 1, 2, -1] {
                    p.maxnmin(p1, p2, p3, p4);
                }
            }
        }
    }
    // and the difference is observable: id 0 vs id 1 must give different results
    // only by the amount the skipped block contributes
    let with = p.maxnmin(0, 0, 1, 0); // node_id 1 → present
    let without = p.maxnmin(-6, 0, 1, 0); // node_id 1 as well (−6%6==0)
    assert_eq!(with, without);
    let skipped = p.maxnmin(-1, 0, 1, 0); // node_id 0 → absent
    assert_ne!(with, skipped, "the NULL branch must change the result");
}

/// Row 29: `(param2 % 6) + 1` names no node → second block skipped.
#[test]
fn err_29_maxnmin_second_node_null() {
    let p = Pair::fresh();
    for p2 in [-1i32, -2, -3, -4, -5, -7, -11, i32::MIN, i32::MIN + 1] {
        let node_id = p2 % 6 + 1;
        assert!(!(1..=6).contains(&node_id), "p2={p2} → id {node_id}");
        for p1 in [0i32, 1, 5] {
            for p3 in [1i32, 0, -1, 100, i32::MAX] {
                for p4 in [0i32, 1, 2, -1] {
                    p.maxnmin(p1, p2, p3, p4);
                }
            }
        }
    }
    // second-block skip is observable through param3, which only feeds that block
    let a = p.maxnmin(0, -1, 1, 0);
    let b = p.maxnmin(0, -1, 1_000, 0);
    assert_eq!(a, b, "with second_node NULL, param3 must not matter");
    let c = p.maxnmin(0, 0, 1, 0);
    let d = p.maxnmin(0, 0, 1_000, 0);
    assert_ne!(c, d, "with second_node present, param3 must matter");
}

/// Row 30: the `if (*name_ptr)` guard.
#[test]
fn err_30_maxnmin_empty_name_branch() {
    let p = Pair::fresh();
    p.maxnmin(0, 0, 1, 0);
    // The six nodes maxnmin builds all have non-empty names in BOTH libraries,
    // so the guard is provably taken (never skipped) identically on both sides.
    for id in 1..=6 {
        let (cp, rp) = p.find_node_by_id(id).expect("node");
        for ptr in [cp, rp] {
            assert_ne!(unsafe { (*ptr).name[0] }, 0, "node {id} name must be non-empty");
        }
        assert_ne!(p.process_node_name(id), 0);
    }
    // The guard's else-branch is equivalent to adding 0, which is exactly what
    // process_string returns for an empty string — asserted differentially here.
    let q = Pair::fresh();
    q.add_node(1, -1, b"", 1.0);
    let (cp, rp) = q.find_node_by_id(1).unwrap();
    assert_eq!(unsafe { (*cp).name[0] }, 0);
    assert_eq!(unsafe { (*rp).name[0] }, 0);
    assert_eq!(q.process_node_name(1), 0, "empty name contributes 0 on both sides");
}

/// Row 31: `param3 == -1` → division by zero in the final term.
#[test]
fn err_31_maxnmin_div_by_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 131);
    for p1 in [0i32, 1, -1, 6, -6, 5, i32::MAX, i32::MIN] {
        for p2 in [0i32, 1, -1, 6, -6, 5, i32::MAX, i32::MIN] {
            for p4 in [0i32, 1, -1, 2, -2, 3, i32::MAX, i32::MIN] {
                p.maxnmin(p1, p2, -1, p4);
            }
        }
    }
    for _ in 0..20_000 {
        p.maxnmin(rng.next_i32(), rng.next_i32(), -1, rng.next_i32());
    }
    // +inf * positive → INT_MAX contribution; sanity-check the direction
    // (1 + 0) / 0 = +inf, * 1 = +inf → final term INT_MAX
    assert_eq!(p.safe_double_to_int(f64::INFINITY), i32::MAX);
}

/// Row 32: `0.0 / 0.0` → NaN → final term `0`.
#[test]
fn err_32_maxnmin_zero_over_zero_nan() {
    let p = Pair::fresh();
    // p1 + p2 == 0 and p3 == -1
    let pairs: &[(i32, i32)] = &[
        (0, 0),
        (1, -1),
        (-1, 1),
        (6, -6),
        (-6, 6),
        (i32::MAX, -i32::MAX),
        (-i32::MAX, i32::MAX),
        (12345, -12345),
    ];
    for &(a, b) in pairs {
        assert_eq!(a.wrapping_add(b), 0, "setup: {a}+{b} must be 0");
        for p4 in [0i32, 1, -1, 7, i32::MAX, i32::MIN] {
            p.maxnmin(a, b, -1, p4);
        }
    }
    // The final term is 0, so p4 must not change the result — but p4 also feeds
    // `parent_id = (p4 % 3) + 1`, so only p4 values with the same residue mod 3
    // are comparable.
    let base = p.maxnmin(6, -6, -1, 0);
    for p4 in [3i32, -3, 300, -300, 999] {
        assert_eq!(p4 % 3, 0, "setup: same parent_id residue");
        assert_eq!(
            p.maxnmin(6, -6, -1, p4),
            base,
            "NaN final term must be 0 regardless of p4 (residue held fixed)"
        );
    }
}

/// Row 33: `±inf * 0.0` → NaN → final term `0`.
#[test]
fn err_33_maxnmin_inf_times_zero_nan() {
    let p = Pair::fresh();
    // p3 == -1 makes the denominator 0.0; a non-zero numerator then gives ±inf,
    // and p4 == 0 turns it into inf * 0.0 == NaN.
    //
    // param3 also feeds `second_node->value * param3`, so to isolate the final
    // term, param2 is chosen so that `(param2 % 6) + 1` names no node and the
    // whole second block is skipped (see row 29). param4 is held at 0, so the
    // `parent_id` residue is fixed too.
    for p1 in [1i32, -1, 5, -5, 100, 12345, i32::MAX, i32::MIN, i32::MIN + 1] {
        let p2 = -1; // (−1 % 6) + 1 == 0 → second_node == NULL
        assert!(!(1..=6).contains(&(p2 % 6 + 1)));
        if p1.wrapping_add(p2) == 0 {
            continue; // that would be 0/0, which row 32 covers
        }
        let inf_case = p.maxnmin(p1, p2, -1, 0); // ±inf * 0.0 → NaN → 0
        let finite_case = p.maxnmin(p1, p2, 0, 0); // finite * 0.0 → ±0.0 → 0
        assert_eq!(
            inf_case, finite_case,
            "p4==0 ⇒ final term 0 whether or not p3==-1 (p1={p1})"
        );
    }
    // broader sweep: just assert C and Rust agree
    for p1 in [1i32, -1, 5, -5, 100, i32::MAX, i32::MIN] {
        for p2 in [0i32, 1, 2, 6, -1, -7] {
            p.maxnmin(p1, p2, -1, 0);
            p.maxnmin(p1, p2, -1, 3);
            p.maxnmin(p1, p2, -1, -3);
        }
    }
}

/// Row 34: `param3 == INT_MAX` → `param3 + 1` wraps to `INT_MIN`.
#[test]
fn err_34_maxnmin_param3_overflow() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 134);
    for p1 in [0i32, 1, -1, 6, i32::MAX, i32::MIN] {
        for p2 in [0i32, 1, -1, 6, i32::MAX, i32::MIN] {
            for p4 in [0i32, 1, -1, 2, i32::MAX, i32::MIN] {
                p.maxnmin(p1, p2, i32::MAX, p4);
                p.maxnmin(p1, p2, i32::MIN, p4);
            }
        }
    }
    for _ in 0..10_000 {
        p.maxnmin(rng.next_i32(), rng.next_i32(), i32::MAX, rng.next_i32());
    }
}

/// Row 35: `param1 + param2` signed overflow wraps before the cast.
#[test]
fn err_35_maxnmin_sum_overflow() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 135);
    let extremes = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1];
    for a in extremes {
        for b in extremes {
            for p3 in [0i32, 1, -1, 2, i32::MAX] {
                for p4 in [0i32, 1, -1, 3] {
                    p.maxnmin(a, b, p3, p4);
                }
            }
        }
    }
    // INT_MAX + INT_MAX wraps to -2 → quotient -2/(p3+1)
    p.maxnmin(i32::MAX, i32::MAX, 0, 1);
    p.maxnmin(i32::MIN, i32::MIN, 0, 1); // wraps to 0
    for _ in 0..10_000 {
        let a = i32::MAX - (rng.next_u32() % 1000) as i32;
        let b = i32::MAX - (rng.next_u32() % 1000) as i32;
        p.maxnmin(a, b, rng.next_i32(), rng.next_i32());
    }
}

/// Row 36: all-extremal parameter corners.
#[test]
fn err_36_maxnmin_extremal_corners() {
    let p = Pair::fresh();
    let ex = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for a in ex {
        for b in ex {
            for c in ex {
                for d in ex {
                    p.maxnmin(a, b, c, d);
                }
            }
        }
    }
}

/// Row 37: `param4 % 3` negative → `parent_id <= 0` → 0 children.
#[test]
fn err_37_maxnmin_parent_id_nonpositive() {
    let p = Pair::fresh();
    for p4 in [-1i32, -2, -4, -5, -3001, i32::MIN, i32::MIN + 1] {
        let parent = p4 % 3 + 1;
        assert!(parent <= 1, "p4={p4} → parent {parent}");
        for p1 in [0i32, 1, 5] {
            for p2 in [0i32, 1, 5] {
                for p3 in [1i32, 0, -1] {
                    p.maxnmin(p1, p2, p3, p4);
                }
            }
        }
    }
    // parent_id 0 has no children at all → the `children*10` term is 0
    assert_eq!(-1i32 % 3 + 1, 0);
    let zero_children = p.maxnmin(0, 0, 1, -1);
    let one_child_group = p.maxnmin(0, 0, 1, 0); // parent 1 → 2 children → +20
    assert_eq!(one_child_group - zero_children, 20);
}

/// Row 38: `maxnmin` resets a full store before rebuilding.
#[test]
fn err_38_maxnmin_resets_full_store() {
    let mut rng = Rng::new(SEED ^ 138);
    for _ in 0..30 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let filled = Pair::fresh();
        for k in 0..MAX_NODES {
            filled.add_node(k as i32 + 1, k as i32, b"junk", rng.next_finite_f64());
        }
        assert_eq!(filled.add_node(1, 1, b"x", 1.0), -1);
        let after = filled.maxnmin(a, b, c, d);
        let clean = Pair::fresh();
        assert_eq!(
            clean.maxnmin(a, b, c, d),
            after,
            "maxnmin({a},{b},{c},{d}) must be independent of prior store state"
        );
        // the store is now exactly the 6-node tree in both libraries
        assert_eq!(filled.get_children_count(1), 2);
        assert_eq!(filled.get_children_count(2), 2);
        assert_eq!(filled.get_children_count(3), 1);
        assert!(filled.find_node_by_id(7).is_none());
        assert_eq!(filled.add_node(7, 1, b"next", 1.0), 6);
    }
}

/// Row 39: arbitrary `int` (no valid "variant") on every `int` parameter.
#[test]
fn err_39_arbitrary_int_domain_all_entry_points() {
    let mut rng = Rng::new(SEED ^ 139);

    // find_node_by_id / get_children_count on a fresh and on a populated store
    for populated in [false, true] {
        let p = Pair::fresh();
        if populated {
            for k in 0..30 {
                let nlen = rng.below(60) as usize;
                let name = rng.bytes(nlen, 0x01, 0xFF);
                p.add_node(rng.next_i32(), rng.next_i32(), &name, rng.next_f64_bits());
            }
        }
        for _ in 0..10_000 {
            p.find_node_by_id(rng.next_i32());
            p.get_children_count(rng.next_i32());
        }
        for probe in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            p.find_node_by_id(probe);
            p.get_children_count(probe);
        }
    }

    // calculate_subtree_sum with an acyclic id-graph (so both terminate) but
    // fully arbitrary query ids
    {
        let p = Pair::fresh();
        for k in 0..50 {
            let parent = if k == 0 { -1 } else { rng.range_i32(0, k) };
            p.add_node(k + 1, parent, b"n", rng.next_finite_f64());
        }
        for _ in 0..10_000 {
            p.calculate_subtree_sum(rng.next_i32());
        }
        for probe in [i32::MIN, i32::MIN + 1, -1, 0, 51, i32::MAX - 1, i32::MAX] {
            p.calculate_subtree_sum(probe);
        }
    }

    // add_node with arbitrary id/parent_id, and safe_double_to_int / maxnmin
    // over arbitrary bit patterns
    {
        let p = Pair::fresh();
        for _ in 0..MAX_NODES {
            let nlen = rng.below(60) as usize;
            let name = rng.bytes(nlen, 0x00, 0xFF);
            p.add_node(rng.next_i32(), rng.next_i32(), &name, rng.next_f64_bits());
        }
        for _ in 0..2_000 {
            p.add_node(rng.next_i32(), rng.next_i32(), b"rejected", rng.next_f64_bits());
        }
    }
    {
        let p = Pair::fresh();
        for _ in 0..20_000 {
            p.safe_double_to_int(rng.next_f64_bits());
            p.maxnmin(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        }
    }
}
