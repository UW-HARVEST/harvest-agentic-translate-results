//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error return, no
//! sentinel, no `assert` and no range check, so its whole rejection surface is
//! the *unchecked* `g_pow43[...]` subscript plus the signed-overflow paths.
//!
//! For inputs where the subscript stays in bounds the two objects are compared
//! bit-for-bit. For inputs where the C reads out of bounds (genuine UB, and the
//! loaded bytes are whatever the linker put next to `.rodata` in *that* object)
//! the tests instead assert that both objects compute the **same subscript,
//! same `sign`, same `frac`, same `poly` and same `mult`** — each object is
//! compared against an oracle built from its *own* table base, located at run
//! time via `/proc/self/maps`. See the rationale section of `ERRORS.md`.

mod support;

use support::{DOMAIN_HI, DOMAIN_LO, Rng, TABLE_LEN, assert_same, assert_same_all, decode, libs,
              in_defined_domain, oracle};

/// Assert that both objects agree with the oracle computed from their *own*
/// table base. Returns `true` if the input was actually callable in both.
fn check_table_relative(x: i32) -> bool {
    let l = libs();
    let (cv, rv) = match (l.c_table.as_ref(), l.rust_table.as_ref()) {
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };
    let (Some(c_exp), Some(r_exp)) = (oracle(x, cv), oracle(x, rv)) else {
        // The C's load address is not inside a readable mapping of one of the
        // objects: a real call would trap, exactly as the C program would.
        return false;
    };
    let c_got = unsafe { (l.c)(x) }.to_bits();
    let r_got = unsafe { (l.rust)(x) }.to_bits();
    assert_eq!(
        c_got, c_exp,
        "C .so disagrees with the C algorithm at x={x} (decoded {:?})",
        decode(x)
    );
    assert_eq!(
        r_got, r_exp,
        "Rust .so computes a different index/sign/frac/poly/mult than the C \
         algorithm at x={x} (decoded {:?})",
        decode(x)
    );
    true
}

/// Sanity: the run-time table location must have succeeded, otherwise the
/// out-of-bounds rows would silently degrade into no-ops.
#[test]
fn tables_are_locatable_in_both_objects() {
    let l = libs();
    assert!(
        l.c_table.is_some(),
        "could not locate g_pow43 inside the C .so"
    );
    assert!(
        l.rust_table.is_some(),
        "could not locate g_pow43 inside the Rust .so"
    );
    // And the located tables really are the table: index 0..144 must read back
    // the transcribed values in both objects.
    for (name, v) in [("C", l.c_table.unwrap()), ("Rust", l.rust_table.unwrap())] {
        for i in 0..TABLE_LEN as i32 {
            let p = v.readable_at(i).expect("in-bounds entry must be readable");
            let got = unsafe { std::ptr::read_unaligned(p) };
            assert_eq!(
                got.to_bits(),
                support::G_POW43[i as usize].to_bits(),
                "{name} table entry {i} mismatch"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 1 — x == -16: the exact lower edge, still in bounds
// ---------------------------------------------------------------------------

#[test]
fn edge_lower_minus16_exact_match() {
    assert_eq!(decode(-16).idx, 0);
    assert!(in_defined_domain(-16));
    assert_same(-16);
    let l = libs();
    assert_eq!(unsafe { (l.c)(-16) }.to_bits(), 0x0000_0000);
    assert_eq!(unsafe { (l.rust)(-16) }.to_bits(), 0x0000_0000);
}

// ---------------------------------------------------------------------------
// Row 2 — x == -17: one step past the lower edge, idx == -1
// ---------------------------------------------------------------------------

#[test]
fn one_past_lower_edge_minus17_same_index() {
    assert_eq!(decode(-17).idx, -1);
    assert!(!in_defined_domain(-17));
    assert!(
        check_table_relative(-17),
        "x=-17 should be callable: idx -1 is inside the object's .rodata mapping"
    );
}

// ---------------------------------------------------------------------------
// Row 3 — x <= -18 down to INT_MIN: arbitrarily far before the table
// ---------------------------------------------------------------------------

#[test]
fn far_below_table_same_index() {
    // Contiguous walk away from the edge for as long as the address stays in a
    // readable mapping, then a randomized sweep of the whole negative range.
    let mut called = 0usize;
    for x in (-4096..=-17).rev() {
        assert_eq!(decode(x).idx, 16 + x);
        if check_table_relative(x) {
            called += 1;
        }
    }
    assert!(called > 0, "no negative out-of-bounds input was callable");

    let mut rng = Rng::new(0xE770_0003);
    for _ in 0..20_000 {
        let x = rng.range(i32::MIN, -17);
        // Index must match the C formula for every such input, wrapping
        // included, whether or not the call itself is safe to perform.
        assert_eq!(decode(x).idx, 16i32.wrapping_add(x));
        check_table_relative(x);
    }
    // Extremes.
    assert_eq!(decode(i32::MIN).idx, 16i32.wrapping_add(i32::MIN));
    assert_eq!(decode(i32::MIN + 1).idx, 16i32.wrapping_add(i32::MIN + 1));
}

// ---------------------------------------------------------------------------
// Row 4 — x == 8223: the exact upper edge, idx == 144, still in bounds
// ---------------------------------------------------------------------------

#[test]
fn edge_upper_8223_exact_match() {
    assert_eq!(decode(8223).idx, (TABLE_LEN - 1) as i32);
    assert!(in_defined_domain(8223));
    assert_same(8223);
    // The whole last block is in bounds and must match bit-for-bit.
    assert_same_all(8192..=8223);
}

// ---------------------------------------------------------------------------
// Row 5 — x == 8224: one step past the end, idx == 145 == TABLE_LEN
// ---------------------------------------------------------------------------

#[test]
fn one_past_upper_edge_8224_same_index() {
    let d = decode(8224);
    assert_eq!(d.idx, TABLE_LEN as i32, "idx should be exactly one past end");
    assert_eq!(d.sign, 64, "x=8224 has bit 5 set, so sign flips to 64");
    assert_eq!(d.mult, 256);
    assert!(!in_defined_domain(8224));
    assert!(
        check_table_relative(8224),
        "x=8224 should be callable: idx 145 is inside the object's .rodata mapping"
    );
    // 8224..=8255 all land on idx 145.
    for x in 8224..=8255 {
        assert_eq!(decode(x).idx, 145, "x={x}");
        check_table_relative(x);
    }
}

// ---------------------------------------------------------------------------
// Row 6 — x >= 8225 up to INT_MAX: arbitrarily far past the table
// ---------------------------------------------------------------------------

#[test]
fn far_above_table_same_index() {
    let mut called = 0usize;
    for x in 8225..=20_000 {
        let d = decode(x);
        assert!(d.idx >= TABLE_LEN as i32);
        if check_table_relative(x) {
            called += 1;
        }
    }
    assert!(called > 0, "no positive out-of-bounds input was callable");

    let mut rng = Rng::new(0xE770_0006);
    for _ in 0..20_000 {
        let x = rng.range(8225, i32::MAX);
        let d = decode(x);
        let sign = if x & 32 != 0 { 64 } else { 0 };
        assert_eq!(d.sign, sign, "sign formula mismatch at x={x}");
        assert_eq!(d.idx, 16i32.wrapping_add(x.wrapping_add(sign) >> 6));
        check_table_relative(x);
    }
    // i32::MAX = 0x7fffffff has bit 5 set, so `2*x & 64` is 64 and `x + sign`
    // overflows (see row 9).
    assert_eq!(decode(i32::MAX).sign, 64);
}

// ---------------------------------------------------------------------------
// Row 7 — signed overflow of `2 * x`
// ---------------------------------------------------------------------------

#[test]
fn overflow_of_2x_wraps_identically() {
    // `2 * x` overflows for every x > INT_MAX/2. Only bit 6 of the product is
    // used, and two's-complement wrap preserves the low bits, so the C's `sign`
    // is fully determined by bit 5 of x. Verify the Rust translation agrees for
    // the whole overflowing range.
    let mut rng = Rng::new(0xE770_0007);
    let mut saw_zero = 0usize;
    let mut saw_64 = 0usize;
    for _ in 0..50_000 {
        let x = rng.range(1_073_741_824, i32::MAX);
        assert!(x.checked_mul(2).is_none(), "x={x} should overflow 2*x");
        let d = decode(x);
        let expect = if x & 32 != 0 { 64 } else { 0 };
        assert_eq!(d.sign, expect, "sign mismatch under 2*x overflow at x={x}");
        if expect == 0 { saw_zero += 1 } else { saw_64 += 1 }
        check_table_relative(x);
    }
    assert!(saw_zero > 0 && saw_64 > 0, "both sign states must be covered");
    // Exact overflow threshold and the extremes.
    for x in [1_073_741_823i32, 1_073_741_824, i32::MAX - 1, i32::MAX] {
        let expect = if x & 32 != 0 { 64 } else { 0 };
        assert_eq!(decode(x).sign, expect, "x={x}");
        check_table_relative(x);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — `x << 3` cannot overflow (guarded by 129 <= x < 1024)
// ---------------------------------------------------------------------------

#[test]
fn shift_by_3_cannot_overflow() {
    for x in 129i32..1024 {
        let shifted = x << 3;
        assert!(
            (1032..=8184).contains(&shifted),
            "x={x} shifted out of the expected range: {shifted}"
        );
        assert_eq!(x.checked_shl(3), Some(shifted));
        assert_eq!(decode(x).mult, 16);
    }
    // The guard really is exclusive at 1024, so no larger x reaches the shift.
    assert_eq!(decode(1024).mult, 256);
    assert_same_all(129..1024);
}

// ---------------------------------------------------------------------------
// Row 9 — signed overflow of `x + sign`
// ---------------------------------------------------------------------------

#[test]
fn overflow_of_x_plus_sign_wraps_identically() {
    // `x + sign` overflows only when sign == 64 and x > INT_MAX - 64.
    let mut overflowing = 0usize;
    for x in (i32::MAX - 200)..=i32::MAX {
        let sign = if x & 32 != 0 { 64 } else { 0 };
        let d = decode(x);
        assert_eq!(d.sign, sign, "x={x}");
        assert_eq!(
            d.idx,
            16i32.wrapping_add(x.wrapping_add(sign) >> 6),
            "wrapped index mismatch at x={x}"
        );
        if sign == 64 && x.checked_add(sign).is_none() {
            overflowing += 1;
            // Wrapping makes `x + sign` negative, so the arithmetic shift must
            // produce a large *negative* subscript.
            assert!(
                x.wrapping_add(sign) < 0,
                "x={x}: x+sign should have wrapped negative"
            );
            assert!(d.idx < 0, "x={x}: subscript should be negative, got {}", d.idx);
        }
        check_table_relative(x);
    }
    assert!(
        overflowing > 0,
        "the x+sign overflow window was not reached"
    );
}

// ---------------------------------------------------------------------------
// Row 10 — division by zero is unreachable
// ---------------------------------------------------------------------------

#[test]
fn denominator_never_zero_no_inf_or_nan() {
    // Every input that reaches the division has a denominator >= 64.
    let check = |x_in: i32| {
        let mut x = x_in;
        if x < 129 {
            return;
        }
        if x < 1024 {
            x = x.wrapping_shl(3);
        }
        let sign = x.wrapping_mul(2) & 64;
        let den = (x & !63).wrapping_add(sign);
        assert_ne!(den, 0, "denominator became zero at x={x_in}");
    };
    for x in 129..=200_000 {
        check(x);
    }
    let mut rng = Rng::new(0xE770_0010);
    for _ in 0..200_000 {
        check(rng.range(129, i32::MAX));
    }
    check(i32::MAX);

    // Consequently no input in the defined domain yields inf/NaN in either
    // object.
    let l = libs();
    for x in DOMAIN_LO..=DOMAIN_HI {
        let c = unsafe { (l.c)(x) };
        let r = unsafe { (l.rust)(x) };
        assert!(c.is_finite() && r.is_finite(), "non-finite at x={x}");
        assert_eq!(c.to_bits(), r.to_bits(), "x={x}");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — every `int` is accepted; there is no rejection path
// ---------------------------------------------------------------------------

/// Row 11 + `CONFIGS.md` row 26: full `i32` randomized sweep.
#[test]
fn every_int_is_accepted_full_range_sweep() {
    let mut rng = Rng::new(0xE770_0011);
    let mut in_domain = 0usize;
    let mut oob_called = 0usize;
    let mut oob_skipped = 0usize;

    // Hand-picked structural values first.
    let mut xs: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1_073_741_825,
        -1_000_000,
        -65,
        -64,
        -63,
        -33,
        -32,
        -18,
        -17,
        -16,
        -15,
        -1,
        0,
        1,
        31,
        32,
        63,
        64,
        127,
        128,
        129,
        130,
        1022,
        1023,
        1024,
        1025,
        8191,
        8192,
        8223,
        8224,
        8225,
        8255,
        8256,
        1_073_741_823,
        1_073_741_824,
        2_147_483_583,
        i32::MAX - 1,
        i32::MAX,
    ];
    xs.extend((0..200_000).map(|_| rng.next_i32()));

    for x in xs {
        if in_defined_domain(x) {
            assert_same(x);
            in_domain += 1;
        } else if check_table_relative(x) {
            oob_called += 1;
        } else {
            oob_skipped += 1;
        }
    }
    assert!(in_domain > 0);
    assert!(
        oob_called > 0,
        "no out-of-bounds input was exercised through both objects"
    );
    eprintln!(
        "full-range sweep: {in_domain} in-domain, {oob_called} out-of-bounds \
         called, {oob_skipped} skipped (unmapped target address)"
    );
}

/// `CONFIGS.md` row 26: full-`i32` table-relative sweep.
///
/// A uniform `i32` draw almost never lands in the 8240-wide defined domain, so
/// the population is stratified: uniform over all of `i32`, uniform over the
/// defined domain, and uniform over the out-of-bounds bands that are still
/// close enough to the table to be dereferenceable (which is where the index
/// arithmetic can actually be observed).
#[test]
fn row26_full_i32_table_relative_sweep() {
    let mut rng = Rng::new(0xC0F1_6026);
    let mut in_domain = 0usize;
    let mut oob = 0usize;

    let mut xs: Vec<i32> = Vec::new();
    xs.extend((0..100_000).map(|_| rng.next_i32()));
    xs.extend((0..50_000).map(|_| rng.range(DOMAIN_LO, DOMAIN_HI)));
    xs.extend((0..50_000).map(|_| rng.range(DOMAIN_HI + 1, DOMAIN_HI + 100_000)));
    xs.extend((0..50_000).map(|_| rng.range(DOMAIN_LO - 100_000, DOMAIN_LO - 1)));

    for x in xs {
        if in_defined_domain(x) {
            assert_same(x);
            in_domain += 1;
        } else if check_table_relative(x) {
            oob += 1;
        }
    }
    assert!(in_domain > 10_000, "in-domain coverage too thin: {in_domain}");
    assert!(oob > 1_000, "out-of-bounds coverage too thin: {oob}");
    eprintln!("row26: {in_domain} in-domain, {oob} out-of-bounds verified");
}

// ---------------------------------------------------------------------------
// Row 12 — no pointer / length parameters
// ---------------------------------------------------------------------------

#[test]
fn no_pointer_or_length_parameters() {
    // `float pow43(int x)` is the entire API surface (c_src/include/lib.h is one
    // line), so there is no null-pointer, zero-length or oversized-length path
    // to test. This test documents that and asserts the symbol really has that
    // signature in both objects by calling it through the exact prototype.
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("c_src/include/lib.h must be readable");
    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        decls,
        vec!["float pow43(int x);"],
        "the public header gained a declaration; the error surface must be re-derived"
    );
    assert!(!header.contains('*'), "header declares no pointer types");

    let l = libs();
    let c: unsafe extern "C" fn(std::ffi::c_int) -> f32 = l.c;
    let r: unsafe extern "C" fn(std::ffi::c_int) -> f32 = l.rust;
    assert_eq!(unsafe { c(42) }.to_bits(), unsafe { r(42) }.to_bits());
}
