// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md (E1, plus the generic FFI boundary rows B1..B8).
// Each constructs the exact rejecting condition, calls BOTH `.so`s, and asserts
// they return the SAME sentinel — not merely "both failed somehow".

mod common;

use common::{assert_same, reference_min, Both, List, Rng};

// ------------------------------------------------------------------ E1 / B1 / B2

/// E1: `head == NULL` takes the `else return -1` branch in the C.
#[test]
fn err_e1_null_head() {
    let both = Both::load();
    let got_c = unsafe { (both.c)(std::ptr::null_mut()) };
    let got_rust = unsafe { (both.rust)(std::ptr::null_mut()) };

    assert_eq!(got_c, -1, "E1: C must return the -1 sentinel for NULL");
    assert_eq!(
        got_c.to_ne_bytes(),
        got_rust.to_ne_bytes(),
        "E1: C returned {got_c}, Rust returned {got_rust}"
    );
}

/// B1: the NULL rejection is idempotent — no hidden state makes call N differ.
#[test]
fn err_b1_null_repeated() {
    let both = Both::load();
    for call in 0..1000 {
        let got_c = unsafe { (both.c)(std::ptr::null_mut()) };
        let got_rust = unsafe { (both.rust)(std::ptr::null_mut()) };
        assert_eq!(got_c, -1, "B1 call {call}");
        assert_eq!(got_rust, -1, "B1 call {call}");
        assert_eq!(got_c, got_rust, "B1 call {call}");
    }
}

/// B2: "zero length" is only representable as NULL (there is no count param),
/// so an empty `List` must hit the very same rejection.
#[test]
fn err_b2_zero_length_is_null() {
    let both = Both::load();
    let mut empty = List::new(&[]);
    assert!(empty.head().is_null(), "B2: empty list head must be NULL");
    let got = assert_same(&both, &[], "B2 zero length");
    assert_eq!(got, -1);

    // Interleave NULL and non-NULL calls: the rejection must not leak state into
    // the success path or vice versa.
    let mut rng = Rng::new(0xBEEF_0002);
    for i in 0..500 {
        let v = rng.i32_any();
        let got_null_c = unsafe { (both.c)(std::ptr::null_mut()) };
        let got_null_rust = unsafe { (both.rust)(std::ptr::null_mut()) };
        assert_eq!(got_null_c, got_null_rust, "B2 interleave {i} null");
        assert_eq!(got_null_c, -1);
        let got_ok = assert_same(&both, &[v], &format!("B2 interleave {i} single"));
        assert_eq!(got_ok, v);
    }
}

// ----------------------------------------------------------------------- B3/B4

/// B3: minimum non-empty input — 1 node, `next == NULL`, loop body never runs.
#[test]
fn err_b3_single_node() {
    let both = Both::load();
    let mut rng = Rng::new(0xBEEF_0003);
    for i in 0..1000 {
        let v = rng.i32_any();
        let got = assert_same(&both, &[v], &format!("B3 iter {i}"));
        assert_eq!(got, v, "B3: single node returns its own value");
    }
}

/// B4: a VALID list whose true minimum is `-1` returns the same value as the
/// NULL error. The C cannot distinguish the two; the Rust must not "fix" this.
#[test]
fn err_b4_minus_one_ambiguity() {
    let both = Both::load();

    // Single node holding -1.
    assert_eq!(assert_same(&both, &[-1], "B4 [-1]"), -1);

    // Longer valid lists whose minimum is exactly -1.
    let mut rng = Rng::new(0xBEEF_0004);
    for i in 0..500 {
        let n = rng.usize_in(2, 32);
        let mut vals: Vec<i32> = (0..n).map(|_| rng.i32_in(0, i32::MAX)).collect();
        vals[rng.usize_in(0, n - 1)] = -1;
        let got = assert_same(&both, &vals, &format!("B4 iter {i}"));
        assert_eq!(got, -1, "B4: valid-list minimum -1 aliases the error code");
    }

    // And the aliasing is exact: NULL and a [-1] list are indistinguishable in
    // BOTH implementations.
    let null_c = unsafe { (both.c)(std::ptr::null_mut()) };
    let mut one = List::new(&[-1]);
    let list_c = unsafe { (both.c)(one.head()) };
    assert_eq!(null_c, list_c, "B4: C aliases NULL and [-1]");
    let null_rust = unsafe { (both.rust)(std::ptr::null_mut()) };
    let mut one_r = List::new(&[-1]);
    let list_rust = unsafe { (both.rust)(one_r.head()) };
    assert_eq!(null_rust, list_rust, "B4: Rust aliases NULL and [-1] too");
}

// ----------------------------------------------------------------- B5/B6/B7/B8

/// B5: `INT_MIN` (one step past the negative range of most callers' assumptions)
/// in first, middle and last position.
#[test]
fn err_b5_int_min() {
    let both = Both::load();

    assert_eq!(assert_same(&both, &[i32::MIN], "B5 single"), i32::MIN);
    assert_eq!(
        assert_same(&both, &[i32::MIN, 0, 7], "B5 first"),
        i32::MIN
    );
    assert_eq!(
        assert_same(&both, &[0, i32::MIN, 7], "B5 middle"),
        i32::MIN
    );
    assert_eq!(assert_same(&both, &[0, 7, i32::MIN], "B5 last"), i32::MIN);

    // INT_MIN together with INT_MAX, and INT_MIN duplicated (tie at the bottom).
    assert_eq!(
        assert_same(&both, &[i32::MAX, i32::MIN, i32::MAX], "B5 with MAX"),
        i32::MIN
    );
    assert_eq!(
        assert_same(&both, &[i32::MIN, i32::MIN], "B5 duplicated"),
        i32::MIN
    );

    let mut rng = Rng::new(0xBEEF_0005);
    for i in 0..500 {
        let n = rng.usize_in(1, 24);
        let mut vals: Vec<i32> = (0..n).map(|_| rng.i32_any()).collect();
        vals[rng.usize_in(0, n - 1)] = i32::MIN;
        let got = assert_same(&both, &vals, &format!("B5 iter {i}"));
        assert_eq!(got, i32::MIN);
    }
}

/// B6: `INT_MAX` at the top of the range, including an all-INT_MAX list where the
/// strict `<` never fires.
#[test]
fn err_b6_int_max() {
    let both = Both::load();

    assert_eq!(assert_same(&both, &[i32::MAX], "B6 single"), i32::MAX);
    for n in [2usize, 3, 8, 33, 257] {
        let vals = vec![i32::MAX; n];
        let got = assert_same(&both, &vals, &format!("B6 all-MAX n={n}"));
        assert_eq!(got, i32::MAX, "B6: no `<` fires, seed survives");
    }
    // INT_MAX as the minimum only when nothing smaller exists.
    assert_eq!(
        assert_same(&both, &[i32::MAX, i32::MAX, i32::MAX - 1], "B6 near-MAX"),
        i32::MAX - 1
    );
}

/// B7: values that are huge as `unsigned` but negative as `int`. An unsigned
/// comparison would return a different winner, so this pins the signedness.
#[test]
fn err_b7_signed_compare() {
    let both = Both::load();

    // 0x80000000 == INT_MIN, 0xFFFFFFFF == -1 when read as int.
    let vals = [
        1i32,
        0x8000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x7FFF_FFFFu32 as i32,
    ];
    let got = assert_same(&both, &vals, "B7 mixed bit patterns");
    assert_eq!(got, i32::MIN, "B7: signed `<` picks 0x80000000 == INT_MIN");

    // Unsigned would say the min is 1; signed says -1.
    let got2 = assert_same(&both, &[1, 0xFFFF_FFFFu32 as i32], "B7 1 vs 0xFFFFFFFF");
    assert_eq!(got2, -1, "B7: signed compare, not unsigned");

    let mut rng = Rng::new(0xBEEF_0007);
    let domain = [
        0u32 as i32,
        1,
        0x7FFF_FFFEu32 as i32,
        0x7FFF_FFFFu32 as i32,
        0x8000_0000u32 as i32,
        0x8000_0001u32 as i32,
        0xFFFF_FFFEu32 as i32,
        0xFFFF_FFFFu32 as i32,
    ];
    for i in 0..1000 {
        let n = rng.usize_in(1, 24);
        let vals: Vec<i32> = (0..n).map(|_| rng.pick(&domain)).collect();
        let got = assert_same(&both, &vals, &format!("B7 iter {i}"));
        assert_eq!(got, reference_min(&vals), "B7 iter {i}");
    }
}

/// B8: oversized input — a very long chain, checking neither side hits a
/// recursion/stack limit the other does not.
#[test]
fn err_b8_oversized_length() {
    let both = Both::load();
    let mut rng = Rng::new(0xBEEF_0008);

    let vals: Vec<i32> = (0..100_000).map(|_| rng.i32_any()).collect();
    let got = assert_same(&both, &vals, "B8 100k random");
    assert_eq!(got, reference_min(&vals));

    // Minimum planted at the very last node of a long chain: the deepest
    // possible `<` fire.
    let mut vals2: Vec<i32> = (0..100_000).map(|_| rng.i32_in(0, i32::MAX)).collect();
    *vals2.last_mut().unwrap() = i32::MIN;
    let got2 = assert_same(&both, &vals2, "B8 100k min-at-end");
    assert_eq!(got2, i32::MIN);

    // All-equal long chain: `<` never fires across 100k nodes.
    let vals3 = vec![42i32; 100_000];
    assert_eq!(assert_same(&both, &vals3, "B8 100k all-equal"), 42);
}

// ------------------------------------------------------------------------- B9

/// B9: out-of-range enum across the FFI boundary.
///
/// ERRORS.md row B9 is N/A: the public API declares no enum, flag or mode. This
/// test documents and mechanically re-verifies that claim so the row is
/// discharged by evidence rather than assumption — if a future revision of the
/// header adds an enum parameter, this test's premise breaks and must be
/// revisited.
#[test]
fn err_b9_no_enum_parameter_exists() {
    let header = include_str!("../../c_src/include/simplestruct.h");
    assert!(
        !header.contains("enum"),
        "B9 premise broken: the C header now declares an enum; add real \
         out-of-range-enum differential coverage to ERRORS.md"
    );
    // The sole entry point takes one pointer and returns a bare int.
    assert!(
        header.contains("int smallestValue (struct ListNode *date);"),
        "B9 premise broken: the public signature changed"
    );

    // The only *defined* invalid pointer value is NULL, which is E1.
    let both = Both::load();
    assert_eq!(unsafe { (both.c)(std::ptr::null_mut()) }, unsafe {
        (both.rust)(std::ptr::null_mut())
    });
}
