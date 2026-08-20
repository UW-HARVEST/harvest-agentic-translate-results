// Phase C — error-path differential tests, one test per row of ERRORS.md.
//
// Each test constructs the EXACT invalid input/condition the C source checks
// for, calls BOTH shared objects, and asserts they return the SAME
// error/sentinel value (not merely "both failed somehow").

mod common;

use common::{both, eq_i32, Buf, Rng};

const INT_MAX: i32 = i32::MAX; // 2147483647
const INT_MIN: i32 = i32::MIN; // -2147483648

fn sdti(row: &str, d: f64, expected_c: i32) {
    let (c, r) = both();
    let cv = unsafe { (c.safe_double_to_int)(d) };
    let rv = unsafe { (r.safe_double_to_int)(d) };
    eq_i32(row, (d, d.to_bits()), cv, rv);
    assert_eq!(
        cv, expected_c,
        "[{row}] C safe_double_to_int({d}) returned {cv}, ERRORS.md says {expected_c}"
    );
}

// ---------------------------------------------------------------------------
// Row 1 — isnan(d) -> 0
// ---------------------------------------------------------------------------

#[test]
fn err_row01_nan() {
    let nans: [u64; 8] = [
        0x7ff8_0000_0000_0000, // canonical quiet NaN
        0xfff8_0000_0000_0000, // negative quiet NaN
        0x7ff8_0000_0000_0001, // quiet NaN, payload 1
        0xfff8_0000_0000_0001,
        0x7ff0_0000_0000_0001, // signalling NaN
        0xfff0_0000_0000_0001,
        0x7fff_ffff_ffff_ffff, // max payload
        0xffff_ffff_ffff_ffff,
    ];
    for bits in nans {
        sdti("row01", f64::from_bits(bits), 0);
    }
    sdti("row01", f64::NAN, 0);
    sdti("row01", -f64::NAN, 0);
    sdti("row01", 0.0 / 0.0, 0);
    sdti("row01", f64::INFINITY - f64::INFINITY, 0);
    // random NaN payloads
    let mut rng = Rng::fixed();
    for _ in 0..500 {
        let bits = 0x7ff8_0000_0000_0000u64 | (rng.next_u64() & 0x8007_ffff_ffff_ffff);
        let d = f64::from_bits(bits);
        if d.is_nan() {
            sdti("row01", d, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 2 & 3 — isinf(d) -> INT_MAX / INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn err_row02_pos_inf() {
    sdti("row02", f64::INFINITY, INT_MAX);
    sdti("row02", 1.0 / 0.0, INT_MAX);
    sdti("row02", f64::MAX * 2.0, INT_MAX);
    sdti("row02", f64::from_bits(0x7ff0_0000_0000_0000), INT_MAX);
}

#[test]
fn err_row03_neg_inf() {
    sdti("row03", f64::NEG_INFINITY, INT_MIN);
    sdti("row03", -1.0 / 0.0, INT_MIN);
    sdti("row03", f64::MIN * 2.0, INT_MIN);
    sdti("row03", f64::from_bits(0xfff0_0000_0000_0000), INT_MIN);
}

// ---------------------------------------------------------------------------
// Rows 4 & 5 — finite out-of-int-range clamps
// ---------------------------------------------------------------------------

#[test]
fn err_row04_ge_int_max() {
    for d in [
        2_147_483_647.0,      // exactly (double)INT_MAX  -> clamp (>= is inclusive)
        2_147_483_647.000_001,
        2_147_483_647.5,
        2_147_483_648.0,
        2_147_483_649.0,
        4_294_967_296.0,
        1e10,
        1e300,
        f64::MAX,
    ] {
        sdti("row04", d, INT_MAX);
    }
    // one step BELOW the boundary must NOT clamp
    let just_below = f64::from_bits(2_147_483_647.0f64.to_bits() - 1);
    sdti("row04", just_below, 2_147_483_646);
}

#[test]
fn err_row05_le_int_min() {
    for d in [
        -2_147_483_648.0,      // exactly (double)INT_MIN -> clamp (<= is inclusive)
        -2_147_483_648.000_001,
        -2_147_483_648.5,
        -2_147_483_649.0,
        -4_294_967_296.0,
        -1e10,
        -1e300,
        f64::MIN,
    ] {
        sdti("row05", d, INT_MIN);
    }
    // one step ABOVE the boundary must NOT clamp
    let just_above = f64::from_bits((-2_147_483_648.0f64).to_bits() - 1);
    sdti("row05", just_above, -2_147_483_647);
}

// ---------------------------------------------------------------------------
// Rows 6-9 — process_array_reverse with non-positive counts (incl. NULL)
// ---------------------------------------------------------------------------

fn reverse(row: &str, ptr: *mut i32, count: i32, expected_c: i32) {
    let (c, r) = both();
    let cv = unsafe { (c.process_array_reverse)(ptr, count) };
    let rv = unsafe { (r.process_array_reverse)(ptr, count) };
    eq_i32(row, (ptr, count), cv, rv);
    assert_eq!(cv, expected_c, "[{row}] C returned {cv}, expected {expected_c}");
}

#[test]
fn err_row06_par_count_zero() {
    let mut rng = Rng::fixed();
    let mut b = Buf::random(&mut rng, 8);
    for k in 0..8 {
        reverse("row06", b.ptr_at(k), 0, 0);
    }
}

#[test]
fn err_row07_par_count_negative() {
    let mut rng = Rng::fixed();
    let mut b = Buf::random(&mut rng, 8);
    for count in [-1i32, -2, -100, INT_MIN, INT_MIN + 1, -0o777] {
        reverse("row07", b.ptr_at(4), count, 0);
    }
    for _ in 0..500 {
        let count = rng.range_i32(INT_MIN, -1);
        reverse("row07", b.ptr_at(4), count, 0);
    }
}

#[test]
fn err_row08_par_null_count_zero() {
    reverse("row08", std::ptr::null_mut(), 0, 0);
}

#[test]
fn err_row09_par_null_count_negative() {
    for count in [-1i32, -7, INT_MIN, INT_MIN + 1] {
        reverse("row09", std::ptr::null_mut(), count, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 10 & 23 — switch default: (operation outside {0..4}) -> 0
// ---------------------------------------------------------------------------

fn sw(row: &str, value: i32, operation: i32, expected_c: i32) {
    let (c, r) = both();
    let cv = unsafe { (c.switch_fallthrough_calculator)(value, operation) };
    let rv = unsafe { (r.switch_fallthrough_calculator)(value, operation) };
    eq_i32(row, (value, operation), cv, rv);
    assert_eq!(
        cv, expected_c,
        "[{row}] C switch_fallthrough_calculator({value},{operation}) = {cv}, expected {expected_c}"
    );
}

#[test]
fn err_row10_switch_default() {
    let bad_ops: [i32; 14] = [
        5, 6, 7, 100, 0o777, -1, -2, -5, -100, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1, 1 << 30,
    ];
    let values: [i32; 9] = [0, 1, -1, 42, -42, 0o777, INT_MAX, INT_MIN, 0o200];
    for op in bad_ops {
        for v in values {
            sw("row10", v, op, 0);
        }
    }
    // randomised: any operation outside 0..=4 must yield exactly 0
    let mut rng = Rng::fixed();
    let mut checked = 0;
    for _ in 0..5000 {
        let op = rng.next_i32();
        if !(0..=4).contains(&op) {
            sw("row10", rng.next_i32(), op, 0);
            checked += 1;
        }
    }
    assert!(checked > 4000, "expected many random default-path hits, got {checked}");
}

#[test]
fn err_row23_out_of_range_enum_ints() {
    // A C enum accepts any int. Feed values that correspond to no `case` label,
    // including the extremes and one-step-past-valid-range on both ends.
    for op in [
        -1i32,
        5,
        i32::MIN,
        i32::MAX,
        -2_147_483_647,
        2_147_483_646,
        0x8000_0000u32 as i32,
        0x7fff_ffff,
        -0x8000_0000i64 as i32,
    ] {
        sw("row23", 12345, op, 0);
        sw("row23", 0, op, 0);
        sw("row23", i32::MIN, op, 0);
    }
    // and confirm the *valid* neighbours are NOT the default path
    let (c, _) = both();
    for op in 0..=4 {
        let cv = unsafe { (c.switch_fallthrough_calculator)(1, op) };
        assert_ne!(cv, 0, "operation {op} must not take the default branch");
    }
}

// ---------------------------------------------------------------------------
// Rows 11, 12, 24 — allocate_and_compute malloc failure -> -1
// ---------------------------------------------------------------------------

fn alloc(row: &str, size: i32, mult: f64, expected_c: i32) {
    let (c, r) = both();
    let cv = unsafe { (c.allocate_and_compute)(size, mult) };
    let rv = unsafe { (r.allocate_and_compute)(size, mult) };
    eq_i32(row, (size, mult), cv, rv);
    assert_eq!(
        cv, expected_c,
        "[{row}] C allocate_and_compute({size},{mult}) = {cv}, expected {expected_c}"
    );
}

#[test]
fn err_row11_alloc_negative_size() {
    // (size_t)negative * 16 wraps to ~2^64 -> malloc fails -> -1
    for size in [-1i32, -2, -3, -8, -9, -100, -0o777, INT_MIN, INT_MIN + 1] {
        alloc("row11", size, 1.5, -1);
        alloc("row11", size, 0.0, -1);
        alloc("row11", size, f64::NAN, -1);
        alloc("row11", size, f64::INFINITY, -1);
    }
    let mut rng = Rng::fixed();
    for _ in 0..400 {
        alloc("row11", rng.range_i32(INT_MIN, -1), rng.f64_in(-100.0, 100.0), -1);
    }
}

#[test]
fn err_row12_alloc_huge_size() {
    // size * sizeof(DataPoint) bytes cannot be satisfied: 8 GiB and up.
    for size in [INT_MAX, INT_MAX - 1, 0x4000_0000, 0x2000_0000] {
        alloc("row12", size, 1.5, -1);
    }
    // A "large but satisfiable" size (16 Mi elements = 256 MiB) is a VALID path;
    // only require that the two libraries agree on it (they must not disagree
    // about whether the allocation succeeded).
    let (c, r) = both();
    let cv = unsafe { (c.allocate_and_compute)(0x0100_0000, 1.5) };
    let rv = unsafe { (r.allocate_and_compute)(0x0100_0000, 1.5) };
    eq_i32("row12/large-but-ok", 0x0100_0000, cv, rv);
}

#[test]
fn err_row24_zero_and_oversized_lengths() {
    // Same boundary set applied to every length/size parameter in the API.
    let counts: [i32; 5] = [0, -1, -2, INT_MIN, INT_MIN + 1];
    let mut rng = Rng::fixed();
    let mut b = Buf::random(&mut rng, 4);

    for count in counts {
        // both pointer APIs, with a real pointer and with NULL
        reverse("row24/reverse", b.ptr_at(3), count, 0);
        reverse("row24/reverse-null", std::ptr::null_mut(), count, 0);
        foreach("row24/foreach", b.ptr(), count, 0);
        foreach("row24/foreach-null", std::ptr::null_mut(), count, 0);
    }

    // allocate_and_compute: 0 succeeds (malloc(0) != NULL), negatives and
    // oversized fail with -1.
    alloc("row24/alloc-zero", 0, 1.5, 0);
    alloc("row24/alloc-neg1", -1, 1.5, -1);
    alloc("row24/alloc-intmin", INT_MIN, 1.5, -1);
    alloc("row24/alloc-intmax", INT_MAX, 1.5, -1);
}

// ---------------------------------------------------------------------------
// Rows 13-16 — foreach_sum with non-positive counts (incl. NULL)
// ---------------------------------------------------------------------------

fn foreach(row: &str, ptr: *mut i32, count: i32, expected_c: i32) {
    let (c, r) = both();
    let cv = unsafe { (c.foreach_sum)(ptr, count) };
    let rv = unsafe { (r.foreach_sum)(ptr, count) };
    eq_i32(row, (ptr, count), cv, rv);
    assert_eq!(cv, expected_c, "[{row}] C returned {cv}, expected {expected_c}");
}

#[test]
fn err_row13_foreach_count_zero() {
    let mut rng = Rng::fixed();
    let mut b = Buf::random(&mut rng, 8);
    foreach("row13", b.ptr(), 0, 0);
    for k in 0..8 {
        foreach("row13", b.ptr_at(k), 0, 0);
    }
}

#[test]
fn err_row14_foreach_count_negative() {
    let mut rng = Rng::fixed();
    let mut b = Buf::random(&mut rng, 8);
    for count in [-1i32, -2, -100, INT_MIN, INT_MIN + 1, -0o777] {
        foreach("row14", b.ptr(), count, 0);
    }
    for _ in 0..500 {
        foreach("row14", b.ptr(), rng.range_i32(INT_MIN, -1), 0);
    }
}

#[test]
fn err_row15_foreach_null_count_zero() {
    foreach("row15", std::ptr::null_mut(), 0, 0);
}

#[test]
fn err_row16_foreach_null_count_negative() {
    for count in [-1i32, -7, INT_MIN, INT_MIN + 1] {
        foreach("row16", std::ptr::null_mut(), count, 0);
    }
}

// ---------------------------------------------------------------------------
// Rows 17-22 — fallcalc-level error/edge conditions
// ---------------------------------------------------------------------------

fn fc(row: &str, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let (c, r) = both();
    let cv = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
    let rv = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
    eq_i32(row, (p1, p2, p3, p4), cv, rv);
    cv
}

#[test]
fn err_row17_fallcalc_malloc_never_fails() {
    // The `data_array == NULL -> return -1` branch cannot fire: the request is a
    // fixed 5 * sizeof(int) = 20 bytes. Assert that neither library ever
    // returns the -1 sentinel and that both stay inside 0..=0777.
    let mut rng = Rng::fixed();
    for _ in 0..5000 {
        let v = fc(
            "row17",
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
        assert_ne!(v, -1, "row17: fallcalc unexpectedly hit the malloc-failure path");
        assert!((0..=0o777).contains(&v), "row17: {v} outside 0..=0777");
    }
}

#[test]
fn err_row18_fallcalc_alloc_size_zero() {
    // param4 % 10 == -1  =>  allocate_and_compute(0, 1.5) -> malloc(0) != NULL -> 0
    let (c, _) = both();
    for p4 in [-1i32, -11, -21, -31, -101, -1_000_001] {
        assert_eq!(p4 % 10 + 1, 0, "precondition for row18");
        // the contribution of allocate_and_compute is 0 for this sub-mode
        assert_eq!(unsafe { (c.allocate_and_compute)(p4 % 10 + 1, 1.5) }, 0);
        for p1 in -2..=2 {
            for p2 in -2..=2 {
                for p3 in [-3i32, 0, 2, 4, 129, 1000] {
                    let v = fc("row18", p1, p2, p3, p4);
                    assert!((0..=0o777).contains(&v));
                }
            }
        }
    }
}

#[test]
fn err_row19_fallcalc_alloc_negative() {
    // param4 % 10 <= -2  =>  allocate_and_compute(negative, 1.5) == -1
    let (c, _) = both();
    for p4 in [-2i32, -3, -4, -5, -6, -7, -8, -9, -12, -19, -99, INT_MIN] {
        let size = p4 % 10 + 1;
        assert!(size < 0, "precondition for row19: p4={p4} size={size}");
        assert_eq!(
            unsafe { (c.allocate_and_compute)(size, 1.5) },
            -1,
            "row19: allocate_and_compute({size},1.5) should be -1"
        );
        for p1 in -2..=2 {
            for p2 in -2..=2 {
                for p3 in [-4i32, -1, 0, 1, 3, 128, 129, 999] {
                    let v = fc("row19", p1, p2, p3, p4);
                    assert!((0..=0o777).contains(&v));
                }
            }
        }
    }
}

#[test]
fn err_row20_fallcalc_switch_default() {
    // param3 < 0 with param3 % 5 != 0  =>  switch default: -> switch_result == 0
    let (c, _) = both();
    for p3 in [-1i32, -2, -3, -4, -6, -7, -8, -9, -11, -99, INT_MIN] {
        let op = p3 % 5;
        assert!(op < 0, "precondition for row20: p3={p3} op={op}");
        assert_eq!(
            unsafe { (c.switch_fallthrough_calculator)(12345, op) },
            0,
            "row20: op {op} should take the default branch"
        );
        for p1 in -2..=2 {
            for p2 in [-0o200i32, -1, 0, 1, 0o200, INT_MAX, INT_MIN] {
                for p4 in [-9i32, -1, 0, 3, 9] {
                    let v = fc("row20", p1, p2, p3, p4);
                    assert!((0..=0o777).contains(&v));
                }
            }
        }
    }
}

#[test]
fn err_row21_fallcalc_float_saturation() {
    // floating_calc saturates safe_double_to_int at both ends.
    let (c, _) = both();
    // p1 = INT_MAX -> 3.7 * 2^31 ~= 7.9e9 > INT_MAX
    assert_eq!(
        unsafe { (c.safe_double_to_int)(INT_MAX as f64 * 3.7) },
        INT_MAX
    );
    assert_eq!(
        unsafe { (c.safe_double_to_int)(INT_MIN as f64 * 3.7) },
        INT_MIN
    );
    for p1 in [INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1, 1_000_000_000, -1_000_000_000] {
        for p2 in [INT_MAX, INT_MIN, 0, 1_000_000_000, -1_000_000_000] {
            for p3 in [INT_MAX, INT_MIN, 0, 129, -129] {
                for p4 in [-9i32, -1, 0, 4, 9] {
                    let v = fc("row21", p1, p2, p3, p4);
                    assert!((0..=0o777).contains(&v));
                }
            }
        }
    }
}

#[test]
fn err_row22_masks_and_limits() {
    // The min/max and mask constants themselves, as observable outputs.
    let (c, r) = both();

    // INT_MAX / INT_MIN are returned verbatim by safe_double_to_int.
    assert_eq!(unsafe { (c.safe_double_to_int)(f64::INFINITY) }, INT_MAX);
    assert_eq!(unsafe { (r.safe_double_to_int)(f64::INFINITY) }, INT_MAX);
    assert_eq!(unsafe { (c.safe_double_to_int)(f64::NEG_INFINITY) }, INT_MIN);
    assert_eq!(unsafe { (r.safe_double_to_int)(f64::NEG_INFINITY) }, INT_MIN);

    // OCTAL_MASK_1 == 0777 bounds every masked result.
    let mut rng = Rng::fixed();
    for _ in 0..3000 {
        let v = fc("row22", rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(v & !0o777, 0, "row22: fallcalc result {v} has bits above 0777");
    }
    // operations 0,1,2 also mask with 0777
    for op in 0..=2 {
        for _ in 0..2000 {
            let v0 = rng.next_i32();
            let cv = unsafe { (c.switch_fallthrough_calculator)(v0, op) };
            let rv = unsafe { (r.switch_fallthrough_calculator)(v0, op) };
            eq_i32("row22/switch-mask", (v0, op), cv, rv);
            assert_eq!(cv & !0o777, 0, "row22: op {op} result {cv} exceeds 0777");
        }
    }
    // OCTAL_MASK_2 (0100) / OCTAL_FLAG (0200) / OCTAL_BASE (010) shift results
    // in ops 3 and 4, which are NOT masked -- confirm both libs agree there too.
    for op in 3..=4 {
        for _ in 0..2000 {
            let v0 = rng.next_i32();
            let cv = unsafe { (c.switch_fallthrough_calculator)(v0, op) };
            let rv = unsafe { (r.switch_fallthrough_calculator)(v0, op) };
            eq_i32("row22/switch-unmasked", (v0, op), cv, rv);
        }
    }
}
