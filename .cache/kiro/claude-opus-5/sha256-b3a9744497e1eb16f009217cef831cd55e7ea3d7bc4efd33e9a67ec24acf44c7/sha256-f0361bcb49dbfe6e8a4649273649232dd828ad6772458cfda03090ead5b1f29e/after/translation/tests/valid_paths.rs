//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH shared objects
//! through their exported `parse_number` symbol with many randomized inputs
//! (fixed seed) and compares the full post-state byte images.

mod harness;

use harness::*;

/* ===================== row 1 — digits reaching end of buffer ============== */

#[test]
fn cfg_row01_digits_to_end() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        let n = rng.range(1, 9);
        let s = rng.digits(n);
        diff("row01", &s);
    }
    // deterministic edges
    for s in ["0", "1", "9", "00", "0000000000", "123456789"] {
        diff("row01/fixed", s.as_bytes());
    }
}

/* ===================== row 2 — digits then a terminator =================== */

#[test]
fn cfg_row02_digits_then_terminator() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 12);
        s.push(non_accepted_byte(&mut rng));
        let tail = rng.below(4);
        for _ in 0..tail {
            s.push(rng.byte());
        }
        diff("row02", &s);
    }
    for s in ["12}", "7,", "0]", "42 ", "5\0", "9\n", "3\"", "8x"] {
        diff("row02/fixed", s.as_bytes());
    }
}

/* ===================== row 3 — mid-buffer offset ========================== */

#[test]
fn cfg_row03_mid_buffer_offset() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let lead = rng.range(1, 8);
        let mut s: Vec<u8> = (0..lead).map(|_| non_accepted_byte(&mut rng)).collect();
        let offset = s.len();
        s.extend_from_slice(&rng.digits_range(1, 10));
        s.push(non_accepted_byte(&mut rng));
        for _ in 0..rng.below(5) {
            s.push(rng.byte());
        }
        diff_at("row03", &s, offset);
    }
    let b = b"[ 42, 7]";
    for off in 0..b.len() {
        diff_at("row03/scan", b, off);
    }
}

/* ===================== row 4 — offset == length =========================== */

#[test]
fn cfg_row04_offset_at_length() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let n = rng.range(1, 16);
        let s: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        diff_full("row04", &s, s.len(), s.len(), rng.next_u64() as usize, SENTINEL_ITEM);
    }
    for s in ["1", "123", "abc"] {
        diff_at("row04/fixed", s.as_bytes(), s.len());
    }
}

/* ===================== row 5 — offset > length ============================ */

#[test]
fn cfg_row05_offset_past_length() {
    let mut rng = Rng::new(SEED ^ 5);
    let backing = b"0123456789ABCDEF".to_vec();
    for _ in 0..N {
        // Keep `length` inside the real allocation, push `offset` beyond it.
        let length = rng.range(0, backing.len());
        let offset = length + rng.range(1, 4096);
        diff_full("row05", &backing, length, offset, rng.next_u64() as usize, SENTINEL_ITEM);
    }
    for (length, offset) in [(4usize, 5usize), (0, 1), (16, 17), (1, 1 << 40)] {
        diff_full("row05/fixed", &backing, length, offset, 0, SENTINEL_ITEM);
    }
}

/* ===================== row 6 — offset near/at SIZE_MAX (wraparound) ======= */

#[test]
fn cfg_row06_offset_size_max_wrap() {
    let backing = b"12345".to_vec();
    // `can_access_at_index` computes `offset + index` in wrapping `size_t`
    // arithmetic. Note that index 0 is always tested first, so `offset >= length`
    // exits the loop before any wrap can be observed; a pair with
    // `offset < length` and `offset` near SIZE_MAX would make the *C* read far
    // outside the caller's allocation (a genuine segfault in the C, hence not a
    // legal input). We therefore sweep the whole `offset >= length` family,
    // including both operands at SIZE_MAX.
    let offsets = [
        usize::MAX,
        usize::MAX - 1,
        usize::MAX - 5,
        usize::MAX / 2,
        1usize << 63,
        (1usize << 63) + 1,
    ];
    let lengths = [0usize, 1, 5, usize::MAX, usize::MAX - 1];
    for &offset in &offsets {
        for &length in &lengths {
            if offset < length {
                continue; // would read outside the real allocation in BOTH impls
            }
            diff_full("row06", &backing, length, offset, 0, SENTINEL_ITEM);
        }
    }
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..N {
        let offset = usize::MAX - rng.below(8);
        let length = if rng.bool() { usize::MAX } else { rng.next_u64() as usize };
        if offset < length {
            continue;
        }
        diff_full("row06/rand", &backing, length, offset, 0, SENTINEL_ITEM);
    }
    // The complementary shape that IS legal: offset just below a huge length,
    // with the scan stopped by a real non-accepted byte inside the allocation.
    let stopper = b"9\x00".to_vec();
    diff_full("row06/huge-len", &stopper, usize::MAX, 0, 0, SENTINEL_ITEM);
}

/* ===================== row 7 — length == 0 ================================ */

#[test]
fn cfg_row07_length_zero() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..N {
        let n = rng.range(1, 16);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        diff_full("row07", &s, 0, 0, rng.next_u64() as usize, SENTINEL_ITEM);
    }
    diff_full("row07/empty-alloc", &[0u8], 0, 0, 0, SENTINEL_ITEM);
}

/* ===================== row 8 — length == 1, each accepted byte ============ */

#[test]
fn cfg_row08_length_one_each_accepted_byte() {
    for &b in ACCEPTED {
        let backing = [b, b'9'];
        diff_full("row08/accepted", &backing, 1, 0, 0, SENTINEL_ITEM);
    }
    for b in 0u8..=255 {
        let backing = [b, b'9'];
        diff_full("row08/allbytes", &backing, 1, 0, 0, SENTINEL_ITEM);
    }
}

/* ===================== row 9 — fractional, has_decimal_point ============== */

#[test]
fn cfg_row09_decimal_fraction() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 10);
        s.push(b'.');
        s.extend_from_slice(&rng.digits_range(1, 12));
        if rng.bool() {
            s.insert(0, if rng.bool() { b'-' } else { b'+' });
        }
        if rng.bool() {
            s.push(non_accepted_byte(&mut rng));
        }
        diff("row09", &s);
    }
    for s in ["3.14159", "-2.5", "+0.5", "0.0", "1.000000000000000001"] {
        diff("row09/fixed", s.as_bytes());
    }
}

/* ===================== row 10 — leading '.' =============================== */

#[test]
fn cfg_row10_leading_decimal_point() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N {
        let mut s = vec![b'.'];
        s.extend_from_slice(&rng.digits_range(1, 14));
        if rng.bool() {
            s.insert(0, if rng.bool() { b'-' } else { b'+' });
        }
        diff("row10", &s);
    }
    for s in [".5", ".0", "-.25", "+.75", ".000000000000000000001"] {
        diff("row10/fixed", s.as_bytes());
    }
}

/* ===================== row 11 — trailing '.' ============================== */

#[test]
fn cfg_row11_trailing_decimal_point() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 12);
        s.push(b'.');
        if rng.bool() {
            s.push(non_accepted_byte(&mut rng));
        }
        diff("row11", &s);
    }
    for s in ["5.", "0.", "-7.", "123.", "9."] {
        diff("row11/fixed", s.as_bytes());
    }
}

/* ===================== row 12 — two decimal points (partial consume) ====== */

#[test]
fn cfg_row12_two_decimal_points_partial() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 6);
        s.push(b'.');
        s.extend_from_slice(&rng.digits_range(1, 6));
        s.push(b'.');
        s.extend_from_slice(&rng.digits_range(1, 6));
        diff("row12", &s);
    }
    for s in ["1.2.3", "0..1", "..5", "1...2", "3.4.5.6"] {
        diff("row12/fixed", s.as_bytes());
    }
}

/* ===================== rows 13/14/15 — exponents ========================== */

#[test]
fn cfg_row13_exponent_lower_e() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 8);
        s.push(b'e');
        s.extend_from_slice(&rng.digits_range(1, 3));
        diff("row13", &s);
    }
    for s in ["1e0", "2e5", "9e30", "1e308", "0e0"] {
        diff("row13/fixed", s.as_bytes());
    }
}

#[test]
fn cfg_row14_exponent_upper_e() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 8);
        s.push(b'E');
        s.extend_from_slice(&rng.digits_range(1, 3));
        diff("row14", &s);
    }
    for s in ["1E0", "2E5", "7E-3", "1E308", "5E1"] {
        diff("row14/fixed", s.as_bytes());
    }
}

#[test]
fn cfg_row15_signed_exponent() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 8);
        if rng.bool() {
            s.push(b'.');
            s.extend_from_slice(&rng.digits_range(1, 6));
        }
        s.push(if rng.bool() { b'e' } else { b'E' });
        s.push(if rng.bool() { b'+' } else { b'-' });
        s.extend_from_slice(&rng.digits_range(1, 3));
        diff("row15", &s);
    }
    for s in ["1e+5", "1e-5", "2.5E+10", "2.5E-10", "1e+308", "1e-308"] {
        diff("row15/fixed", s.as_bytes());
    }
}

/* ===================== row 16 — dangling exponent (partial) =============== */

#[test]
fn cfg_row16_dangling_exponent_partial() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 8);
        s.push(if rng.bool() { b'e' } else { b'E' });
        match rng.below(3) {
            0 => {}
            1 => s.push(b'+'),
            _ => s.push(b'-'),
        }
        if rng.bool() {
            s.push(non_accepted_byte(&mut rng));
        }
        diff("row16", &s);
    }
    for s in ["1e", "1E", "1e+", "1e-", "42E-", "0e", "1.5e", "1.5e+"] {
        diff("row16/fixed", s.as_bytes());
    }
}

/* ===================== row 17 — repeated exponent (partial) =============== */

#[test]
fn cfg_row17_repeated_exponent_partial() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 5);
        for _ in 0..rng.range(2, 4) {
            s.push(if rng.bool() { b'e' } else { b'E' });
            if rng.bool() {
                s.push(if rng.bool() { b'+' } else { b'-' });
            }
            s.extend_from_slice(&rng.digits_range(1, 3));
        }
        diff("row17", &s);
    }
    for s in ["1e2e3", "1E2E3", "1e2E3", "5e1e1e1", "1e+2e-3"] {
        diff("row17/fixed", s.as_bytes());
    }
}

/* ===================== rows 18/19 — leading sign ========================== */

#[test]
fn cfg_row18_leading_plus() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..N {
        let mut s = vec![b'+'];
        s.extend_from_slice(&rng.digits_range(1, 12));
        if rng.bool() {
            s.push(b'.');
            s.extend_from_slice(&rng.digits_range(1, 6));
        }
        diff("row18", &s);
    }
    for s in ["+0", "+1", "+42", "+2147483647", "+0.0"] {
        diff("row18/fixed", s.as_bytes());
    }
}

#[test]
fn cfg_row19_leading_minus() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..N {
        let mut s = vec![b'-'];
        s.extend_from_slice(&rng.digits_range(1, 12));
        if rng.bool() {
            s.push(b'.');
            s.extend_from_slice(&rng.digits_range(1, 6));
        }
        diff("row19", &s);
    }
    for s in ["-0", "-1", "-42", "-2147483648", "-0.0", "-999999999999"] {
        diff("row19/fixed", s.as_bytes());
    }
}

/* ===================== row 20 — negative zero ============================= */

#[test]
fn cfg_row20_negative_zero() {
    for s in [
        "-0", "-0.0", "-0e5", "-0E5", "-0.000", "-0e-5", "-.0", "-0e999", "-0.0e+10",
    ] {
        diff("row20/fixed", s.as_bytes());
    }
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..N {
        let mut s = vec![b'-'];
        for _ in 0..rng.range(1, 5) {
            s.push(b'0');
        }
        if rng.bool() {
            s.push(b'.');
            for _ in 0..rng.range(1, 5) {
                s.push(b'0');
            }
        }
        if rng.bool() {
            s.push(b'e');
            if rng.bool() {
                s.push(if rng.bool() { b'+' } else { b'-' });
            }
            s.extend_from_slice(&rng.digits_range(1, 3));
        }
        diff("row20", &s);
    }
}

/* ===================== row 21 — embedded sign (partial consume) =========== */

#[test]
fn cfg_row21_embedded_sign_partial() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 6);
        for _ in 0..rng.range(1, 3) {
            s.push(if rng.bool() { b'+' } else { b'-' });
        }
        s.extend_from_slice(&rng.digits_range(1, 6));
        diff("row21", &s);
    }
    for s in ["1-2", "1+2", "1--2", "1++2", "12-34", "5+-6", "1-2-3"] {
        diff("row21/fixed", s.as_bytes());
    }
}

/* ===================== rows 22/23 — overflow to +/- inf =================== */

#[test]
fn cfg_row22_overflow_to_pos_inf() {
    for s in [
        "1e999",
        "1e400",
        "1E1000",
        "2e308",
        "1.7976931348623159e308",
        "9e99999999",
    ] {
        diff("row22/fixed", s.as_bytes());
    }
    let nines = "9".repeat(400);
    diff("row22/400nines", nines.as_bytes());
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..N {
        let mut s = rng.digits_range(1, 6);
        s.push(if rng.bool() { b'e' } else { b'E' });
        if rng.bool() {
            s.push(b'+');
        }
        s.extend_from_slice(&rng.digits_range(4, 8));
        diff("row22", &s);
    }
}

#[test]
fn cfg_row23_overflow_to_neg_inf() {
    for s in ["-1e999", "-1e400", "-1E1000", "-2e308", "-9e99999999"] {
        diff("row23/fixed", s.as_bytes());
    }
    let s = format!("-{}", "9".repeat(400));
    diff("row23/400nines", s.as_bytes());
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..N {
        let mut s = vec![b'-'];
        s.extend_from_slice(&rng.digits_range(1, 6));
        s.push(if rng.bool() { b'e' } else { b'E' });
        s.extend_from_slice(&rng.digits_range(4, 8));
        diff("row23", &s);
    }
}

/* ===================== row 24 — underflow / subnormal ===================== */

#[test]
fn cfg_row24_underflow_and_subnormal() {
    for s in [
        "1e-999",
        "-1e-999",
        "1e-400",
        "1e-320",
        "4.9e-324",
        "5e-324",
        "2.4703282292062327e-324",
        "1e-310",
        "-1e-320",
        "1e-308",
        "2.2250738585072011e-308",
    ] {
        diff("row24/fixed", s.as_bytes());
    }
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..N {
        let mut s: Vec<u8> = Vec::new();
        if rng.bool() {
            s.push(b'-');
        }
        s.extend_from_slice(&rng.digits_range(1, 6));
        if rng.bool() {
            s.push(b'.');
            s.extend_from_slice(&rng.digits_range(1, 10));
        }
        s.push(b'e');
        s.push(b'-');
        s.extend_from_slice(&rng.digits_range(3, 4));
        diff("row24", &s);
    }
}

/* ===================== rows 25/26 — INT_MAX / INT_MIN boundaries ========== */

#[test]
fn cfg_row25_int_max_boundary() {
    for s in [
        "2147483645",
        "2147483646",
        "2147483646.9999",
        "2147483647",
        "2147483646.9999999999999995",
        "2147483647.0000000000001",
        "2147483647.5",
        "2147483648",
        "2147483649",
        "2.147483647e9",
        "2.1474836470000001e9",
        "21474836470e-1",
    ] {
        diff("row25/fixed", s.as_bytes());
    }
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..N {
        // random values densely around 2^31
        let delta = rng.range(0, 40) as i64 - 20;
        let base = 2_147_483_647i64 + delta;
        let mut s = base.to_string();
        if rng.bool() {
            s.push('.');
            s.push_str(&String::from_utf8(rng.digits_range(1, 6)).unwrap());
        }
        diff("row25", s.as_bytes());
    }
}

#[test]
fn cfg_row26_int_min_boundary() {
    for s in [
        "-2147483646",
        "-2147483647",
        "-2147483647.5",
        "-2147483648",
        "-2147483648.0000000001",
        "-2147483649",
        "-2147483650",
        "-2.147483648e9",
        "-21474836480e-1",
    ] {
        diff("row26/fixed", s.as_bytes());
    }
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..N {
        let delta = rng.range(0, 40) as i64 - 20;
        let base = -2_147_483_648i64 + delta;
        let mut s = base.to_string();
        if rng.bool() {
            s.push('.');
            s.push_str(&String::from_utf8(rng.digits_range(1, 6)).unwrap());
        }
        diff("row26", s.as_bytes());
    }
}

/* ===================== row 27 — long mantissa rounding ==================== */

#[test]
fn cfg_row27_long_mantissa_rounding() {
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..N {
        let mut s: Vec<u8> = Vec::new();
        if rng.bool() {
            s.push(b'-');
        }
        s.extend_from_slice(&rng.digits_range(18, 40));
        if rng.bool() {
            s.push(b'.');
            s.extend_from_slice(&rng.digits_range(18, 40));
        }
        if rng.bool() {
            s.push(if rng.bool() { b'e' } else { b'E' });
            if rng.bool() {
                s.push(if rng.bool() { b'+' } else { b'-' });
            }
            s.extend_from_slice(&rng.digits_range(1, 3));
        }
        diff("row27", &s);
    }
    // classic hard-rounding cases for strtod
    for s in [
        "8.98846567431158e307",
        "1.7976931348623157e308",
        "0.500000000000000166533453693773481063544750213623046875",
        "9007199254740993",
        "9007199254740992.5",
        "1.0000000000000002220446049250313080847263336181640625",
        "123456789012345678901234567890",
    ] {
        diff("row27/fixed", s.as_bytes());
    }
}

/* ===================== row 28 — very long accepted run =================== */

#[test]
fn cfg_row28_very_long_accepted_run() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..120 {
        let n = rng.range(1, 4096);
        let mut s: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        s.push(non_accepted_byte(&mut rng));
        diff("row28", &s);
    }
    for n in [1usize, 2, 15, 16, 17, 31, 32, 33, 255, 256, 257, 1023, 1024, 4095, 4096] {
        let s = "9".repeat(n);
        diff("row28/nines", s.as_bytes());
        let s = format!("0.{}", "1".repeat(n));
        diff("row28/frac", s.as_bytes());
    }
}

/* ===================== row 29 — fuzz over the accepted charset ============ */

#[test]
fn cfg_row29_fuzz_accepted_charset() {
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..(N * 8) {
        let n = rng.range(1, 24);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        let offset = rng.below(s.len());
        diff_full("row29", &s, s.len(), offset, rng.next_u64() as usize, SENTINEL_ITEM);
    }
}

/* ===================== row 30 — fuzz over arbitrary bytes ================= */

#[test]
fn cfg_row30_fuzz_arbitrary_bytes() {
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..(N * 8) {
        let n = rng.range(1, 40);
        // Bias towards the accepted charset half the time so runs are non-trivial.
        let s: Vec<u8> = (0..n)
            .map(|_| if rng.bool() { *rng.pick(ACCEPTED) } else { rng.byte() })
            .collect();
        let offset = rng.below(s.len() + 2);
        let length = if rng.below(8) == 0 { rng.below(s.len() + 1) } else { s.len() };
        diff_full("row30", &s, length, offset, rng.next_u64() as usize, SENTINEL_ITEM);
    }
}

/* ===================== row 31 — content with no NUL anywhere ============== */

#[test]
fn cfg_row31_unterminated_content() {
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..N {
        // Allocation contains ONLY accepted bytes and ends exactly at `length`,
        // so there is no '\0' for strtod to find in the caller's buffer.
        let n = rng.range(1, 20);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        assert!(!s.contains(&0));
        diff("row31", &s);
    }
    for s in ["1", "12", "1.5", "1e5", "-3", "+7", "2147483647", "1e999"] {
        // exact-fit allocation, no terminator
        let v: Vec<u8> = s.bytes().collect();
        diff("row31/fixed", &v);
    }
}

/* ===================== row 32 — bogus huge `length` ======================= */

#[test]
fn cfg_row32_bogus_huge_length() {
    let mut rng = Rng::new(SEED ^ 32);
    // The scan is stopped by a real non-accepted byte inside the allocation,
    // so `length` may lie arbitrarily without provoking an OOB read.
    for _ in 0..N {
        let mut s = rng.digits_range(1, 12);
        s.push(0); // NUL is not in the accepted set -> stops the scan
        for &length in &[usize::MAX, usize::MAX / 2, 1 << 40, s.len() + 1_000_000] {
            diff_full("row32", &s, length, 0, 0, SENTINEL_ITEM);
        }
    }
    for s in ["1}", "42\0", "-7]", "1e5,"] {
        diff_full("row32/fixed", s.as_bytes(), usize::MAX, 0, 0, SENTINEL_ITEM);
    }
}

/* ===================== row 33 — sequential multi-call pipeline ============ */

#[test]
fn cfg_row33_sequential_multi_call_pipeline() {
    let mut rng = Rng::new(SEED ^ 33);
    for _ in 0..200 {
        // Build a buffer of several numbers separated by delimiters.
        let count = rng.range(2, 8);
        let mut content: Vec<u8> = Vec::new();
        for k in 0..count {
            if k > 0 {
                content.push(*rng.pick(b",; \t[]{}\"".as_slice()));
            }
            if rng.bool() {
                content.push(b'-');
            }
            content.extend_from_slice(&rng.digits_range(1, 8));
            if rng.bool() {
                content.push(b'.');
                content.extend_from_slice(&rng.digits_range(1, 6));
            }
            if rng.bool() {
                content.push(if rng.bool() { b'e' } else { b'E' });
                if rng.bool() {
                    content.push(if rng.bool() { b'+' } else { b'-' });
                }
                content.extend_from_slice(&rng.digits_range(1, 3));
            }
        }
        content.push(b']');

        // Drive both libraries through the WHOLE buffer, step by step, letting
        // each call resume from the offset the previous one left behind.
        let mut buf_c = ParseBuffer {
            content: content.as_ptr(),
            length: content.len(),
            offset: 0,
            depth: 3,
        };
        let mut buf_r = buf_c;
        let mut item_c = SENTINEL_ITEM;
        let mut item_r = SENTINEL_ITEM;

        for step in 0..(content.len() + 4) {
            // Safety: valid pointers, C-ABI symbols.
            let rc = unsafe { (c_parse_number())(&mut item_c, &mut buf_c) };
            let rr = unsafe { (rust_parse_number())(&mut item_r, &mut buf_r) };
            assert_eq!(rc, rr, "row33 step {step}: return differs ({rc} vs {rr})");
            assert_eq!(
                (item_c.type_, item_c.valueint, item_c.valuedouble.to_bits()),
                (item_r.type_, item_r.valueint, item_r.valuedouble.to_bits()),
                "row33 step {step}: item differs"
            );
            assert_eq!(
                (buf_c.length, buf_c.offset, buf_c.depth),
                (buf_r.length, buf_r.offset, buf_r.depth),
                "row33 step {step}: buffer differs"
            );
            if rc == 0 {
                // parse failed: skip one byte (as a real tokenizer would) and continue
                if buf_c.offset >= buf_c.length {
                    break;
                }
                buf_c.offset += 1;
                buf_r.offset += 1;
            }
        }
    }
}

/* ===================== row 34 — item pre-state sentinels ================= */

#[test]
fn cfg_row34_item_prestate_sentinels() {
    let pres = [
        SENTINEL_ITEM,
        CJson { type_: 0, valueint: 0, valuedouble: 0.0 },
        CJson { type_: 8, valueint: 1, valuedouble: 1.0 },
        CJson { type_: i32::MIN, valueint: i32::MAX, valuedouble: f64::NAN },
        CJson { type_: i32::MAX, valueint: i32::MIN, valuedouble: f64::INFINITY },
        CJson { type_: -1, valueint: -1, valuedouble: f64::NEG_INFINITY },
        CJson {
            type_: 0x0BAD_F00Du32 as i32,
            valueint: 0x0BAD_BEEFu32 as i32,
            valuedouble: f64::from_bits(0x7ff8_0000_dead_beef),
        },
        CJson { type_: 1 << 3, valueint: 42, valuedouble: -0.0 },
    ];
    let inputs: &[&str] = &[
        "42",      // success, in-range
        "1e999",   // success, saturate high
        "-1e999",  // success, saturate low
        "",        // failure, empty
        "x",       // failure, non-numeric lead
        "+",       // failure, unparsable
        "-0",      // success, negative zero
        "1e",      // success, partial consume
    ];
    for pre in pres {
        for s in inputs {
            diff_full("row34", s.as_bytes(), s.len(), 0, 0xDEAD_BEEF, pre);
        }
        diff_null_buffer("row34/null-buffer", pre);
        diff_null_content("row34/null-content", 10, 0, 7, pre);
    }
}

/* ===================== row 35 — `depth` is preserved ===================== */

#[test]
fn cfg_row35_depth_preserved() {
    let mut rng = Rng::new(SEED ^ 35);
    for &depth in &[0usize, 1, 2, 1000, usize::MAX, usize::MAX - 1, 1 << 63] {
        for s in ["42", "", "x", "1.5e3", "-0"] {
            diff_full("row35", s.as_bytes(), s.len(), 0, depth, SENTINEL_ITEM);
        }
    }
    for _ in 0..N {
        let depth = rng.next_u64() as usize;
        let s = rng.digits_range(1, 10);
        diff_full("row35/rand", &s, s.len(), 0, depth, SENTINEL_ITEM);
    }
}

/* ===================== row 36 — hex / keyword lookalikes ================= */

#[test]
fn cfg_row36_hex_and_keyword_lookalikes() {
    for s in [
        "0x1A", "0X1f", "0x", "0X", "-0x10", "0xdeadbeef", "nan", "NaN", "NAN", "nan(1)", "inf",
        "Inf", "INF", "infinity", "Infinity", "null", "true", "false", "0b101", "0o17", "1_000",
        "1'000", "e10", "E10", "0xp1", "1p3", "0x1p3",
    ] {
        diff("row36/fixed", s.as_bytes());
    }
    // 'e'/'E' ARE accepted by the scan, so hex-ish strings starting with a digit
    // and containing only accepted bytes must be handled by strtod as decimal.
    for s in ["1e", "1E", "0e0", "9E9", "0e", "0E"] {
        diff("row36/e", s.as_bytes());
    }
}

/* ===================== row 37 — leading whitespace / NUL ================= */

#[test]
fn cfg_row37_leading_whitespace_or_nul() {
    for s in [
        " 12", "\t12", "\n12", "\r12", "\u{b}12", "\u{c}12", "  -3", "\012", "\0", " ", "\t",
        "\n", "  ", " +1", " .5",
    ] {
        diff("row37/fixed", s.as_bytes());
    }
    let nul_lead: &[u8] = &[0, b'1', b'2'];
    diff("row37/nul", nul_lead);
}

/* ===================== row 38 — ABI layout ================================ */

#[test]
fn abi_struct_layout_matches_c() {
    use std::mem::{align_of, size_of};

    // Rust-side mirror
    assert_eq!(size_of::<CJson>(), 16, "sizeof(cJSON)");
    assert_eq!(align_of::<CJson>(), 8, "alignof(cJSON)");
    assert_eq!(size_of::<ParseBuffer>(), 32, "sizeof(parse_buffer)");
    assert_eq!(align_of::<ParseBuffer>(), 8, "alignof(parse_buffer)");

    // And the same for the crate's own public types, as seen through the ABI.
    // (Compile a tiny probe against the REAL c_src header when a C compiler is
    // available; skip gracefully otherwise.)
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let inc = root.join("c_src/include");
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/abi_probe");
    let src = out.with_extension("c");
    std::fs::create_dir_all(out.parent().unwrap()).ok();
    let probe = r#"
#include <stddef.h>
#include <stdio.h>
#include "lib.h"
int main(void) {
    printf("%zu %zu %zu %zu %zu %zu %zu %zu %zu %zu %zu\n",
        sizeof(cJSON), _Alignof(cJSON),
        offsetof(cJSON, type), offsetof(cJSON, valueint), offsetof(cJSON, valuedouble),
        sizeof(parse_buffer), _Alignof(parse_buffer),
        offsetof(parse_buffer, content), offsetof(parse_buffer, length),
        offsetof(parse_buffer, offset), offsetof(parse_buffer, depth));
    return 0;
}
"#;
    if std::fs::write(&src, probe).is_err() {
        eprintln!("abi probe: cannot write source, skipping C-side check");
        return;
    }
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let st = std::process::Command::new(&cc)
        .arg("-I")
        .arg(&inc)
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .status();
    match st {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!("abi probe: `{cc}` unavailable, skipping C-side check");
            return;
        }
    }
    let o = std::process::Command::new(&out).output().expect("run probe");
    let txt = String::from_utf8_lossy(&o.stdout);
    let v: Vec<usize> = txt
        .split_whitespace()
        .map(|x| x.parse().expect("probe number"))
        .collect();
    assert_eq!(v.len(), 11, "probe output: {txt:?}");
    assert_eq!(v[0], size_of::<CJson>(), "sizeof(cJSON)");
    assert_eq!(v[1], align_of::<CJson>(), "alignof(cJSON)");
    assert_eq!(v[2], 0, "offsetof(cJSON,type)");
    assert_eq!(v[3], 4, "offsetof(cJSON,valueint)");
    assert_eq!(v[4], 8, "offsetof(cJSON,valuedouble)");
    assert_eq!(v[5], size_of::<ParseBuffer>(), "sizeof(parse_buffer)");
    assert_eq!(v[6], align_of::<ParseBuffer>(), "alignof(parse_buffer)");
    assert_eq!(v[7], 0, "offsetof(parse_buffer,content)");
    assert_eq!(v[8], 8, "offsetof(parse_buffer,length)");
    assert_eq!(v[9], 16, "offsetof(parse_buffer,offset)");
    assert_eq!(v[10], 24, "offsetof(parse_buffer,depth)");
}

/* ===================== row 39 — exhaustive short-input sweeps ============= */

/// Alphabet = the 15 accepted bytes plus one run-terminating byte.
const ALPHA: &[u8] = b"0123456789+-eE.}";

fn sweep(len: usize, label: &str) {
    let base = ALPHA.len();
    let total = base.pow(len as u32);
    let mut s = vec![0u8; len];
    for n in 0..total {
        let mut k = n;
        for slot in s.iter_mut() {
            *slot = ALPHA[k % base];
            k /= base;
        }
        diff(label, &s);
    }
}

/// EXHAUSTIVE over every string of length 1..=4 from the accepted charset plus a
/// terminator (16^1 + 16^2 + 16^3 + 16^4 = 69 904 inputs). This covers every
/// combination of the scan loop's byte classes, `has_decimal_point`, and all
/// three `strtod` consumption outcomes without relying on the RNG.
#[test]
fn cfg_row39a_exhaustive_len1_to_4() {
    for len in 1..=4 {
        sweep(len, "row39a");
    }
}

/// EXHAUSTIVE over every string of length 5 from the same 16-byte alphabet
/// (1 048 576 inputs).
#[test]
fn cfg_row39b_exhaustive_len5() {
    sweep(5, "row39b");
}

/// EXHAUSTIVE over every 2-byte input over the FULL 0..=255 byte range
/// (65 536 inputs) — no charset restriction at all.
#[test]
fn cfg_row39c_exhaustive_all_byte_pairs() {
    for a in 0u8..=255 {
        for b in 0u8..=255 {
            diff("row39c", &[a, b]);
        }
    }
}

/// EXHAUSTIVE over every 3-byte input whose bytes come from a mixed alphabet of
/// accepted bytes, whitespace, NUL and JSON delimiters, at EVERY start offset.
#[test]
fn cfg_row39d_exhaustive_len3_every_offset() {
    const MIX: &[u8] = b"0123456789+-eE. \t\0,]}\"xX";
    let base = MIX.len();
    let mut s = [0u8; 3];
    for n in 0..base.pow(3) {
        let mut k = n;
        for slot in s.iter_mut() {
            *slot = MIX[k % base];
            k /= base;
        }
        for off in 0..=3usize {
            diff_full("row39d", &s, 3, off, 0, SENTINEL_ITEM);
        }
    }
}

/// EXHAUSTIVE over every string of length 6 from the 16-byte alphabet
/// (16 777 216 inputs). This is the deepest brute-force layer; it reaches
/// mantissa/exponent shapes such as `1.2e-9`, `+.5e-3`, `1e2e3}` etc.
#[test]
fn cfg_row39e_exhaustive_len6() {
    sweep(6, "row39e");
}

/// EXHAUSTIVE over every string of length 7 from the 16-byte alphabet
/// (268 435 456 inputs). Kept `#[ignore]`d so the default run stays quick; run
/// with `cargo test --release -- --ignored`. It has been executed and passes.
#[test]
#[ignore = "268M inputs; run explicitly with --ignored"]
fn cfg_row39f_exhaustive_len7() {
    sweep(7, "row39f");
}
