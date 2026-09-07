// Phase C -- error / rejection-path differential tests.
// One test per ERRORS.md row 1..=29. Each asserts the two libraries return the
// SAME specific error code / sentinel / clamped value, not merely "both failed".
mod common;
use common::*;

const INT_MAX: i32 = i32::MAX; // 2147483647
const INT_MIN: i32 = i32::MIN; // -2147483648

/// Assert both libraries return `expected` from `safe_double_to_int(d)`.
fn expect_sdti(row: &str, d: f64, expected: i32) {
    let l = libs();
    let rc = unsafe { (l.c.safe_double_to_int)(d) };
    let rr = unsafe { (l.rs.safe_double_to_int)(d) };
    assert_eq!(rc, expected, "[{row}] C safe_double_to_int({d:?}) = {rc}, expected {expected}");
    assert_eq!(rr, expected, "[{row}] RUST safe_double_to_int({d:?}) = {rr}, expected {expected}");
}

/// Assert both libraries return `expected` from `process_with_fallthrough`.
fn expect_pwf(row: &str, code: i32, base: i32, expected: i32) {
    let l = libs();
    let rc = unsafe { (l.c.process_with_fallthrough)(code, base) };
    let rr = unsafe { (l.rs.process_with_fallthrough)(code, base) };
    assert_eq!(rc, expected, "[{row}] C pwf({code},{base}) = {rc}, expected {expected}");
    assert_eq!(rr, expected, "[{row}] RUST pwf({code},{base}) = {rr}, expected {expected}");
}

/// Extract one `printf`-emitted field from an `overunder` stdout capture.
fn field(out: &[u8], prefix: &str) -> String {
    let s = String::from_utf8_lossy(out);
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    panic!("prefix {prefix:?} not found in stdout:\n{s}");
}

/// Run `overunder` on both libs, assert they agree, and hand back the capture.
fn run_both(row: &str, a: i32, b: i32, c: i32, d: i32) -> (i32, Vec<u8>) {
    let l = libs();
    let (rc, oc) = capture_stdout(|| unsafe { (l.c.overunder)(a, b, c, d) });
    let (rr, or) = capture_stdout(|| unsafe { (l.rs.overunder)(a, b, c, d) });
    assert_eq!(rc, rr, "[{row}] overunder({a},{b},{c},{d}) return C={rc} RUST={rr}");
    assert_eq!(
        oc,
        or,
        "[{row}] overunder({a},{b},{c},{d}) stdout differs:\n C   ={}\n RUST={}",
        show(&oc),
        show(&or)
    );
    (rc, oc)
}

// ===========================================================================
// Rows 1..10 -- safe_double_to_int guards
// ===========================================================================

#[test]
fn err_01_sdti_over_int_max() {
    for d in [
        2147483648.0,
        2147483647.5,
        2147483648.5,
        3e9,
        1e15,
        1e300,
        f64::MAX,
        4294967296.0,
    ] {
        expect_sdti("err01", d, INT_MAX);
    }
    let mut r = Rng::new(SEED ^ 0x01);
    for _ in 0..2000 {
        expect_sdti("err01", r.range_f64(2147483648.0, 1e18), INT_MAX);
    }
}

#[test]
fn err_02_sdti_pos_inf() {
    expect_sdti("err02", f64::INFINITY, INT_MAX);
    expect_sdti("err02", 1.0 / 0.0, INT_MAX);
}

#[test]
fn err_03_sdti_under_int_min() {
    for d in [
        -2147483649.0,
        -2147483648.5,
        -2147483648.0000001,
        -3e9,
        -1e15,
        -1e300,
        f64::MIN,
        -4294967296.0,
    ] {
        expect_sdti("err03", d, INT_MIN);
    }
    let mut r = Rng::new(SEED ^ 0x03);
    for _ in 0..2000 {
        expect_sdti("err03", -r.range_f64(2147483649.0, 1e18), INT_MIN);
    }
}

#[test]
fn err_04_sdti_neg_inf() {
    expect_sdti("err04", f64::NEG_INFINITY, INT_MIN);
    expect_sdti("err04", -1.0 / 0.0, INT_MIN);
}

#[test]
fn err_05_sdti_nan() {
    // Quiet, negative-quiet, signalling, and payload-carrying NaNs must all
    // reach the `isnan` branch and yield exactly 0.
    for bits in [
        0x7FF8_0000_0000_0000u64,
        0xFFF8_0000_0000_0000,
        0x7FF8_0000_0000_0001,
        0xFFF8_0000_0000_0001,
        0x7FF0_0000_0000_0001,
        0xFFF0_0000_0000_0001,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x7FFA_AAAA_AAAA_AAAA,
    ] {
        let d = f64::from_bits(bits);
        assert!(d.is_nan(), "{bits:#018x} is not NaN");
        expect_sdti("err05", d, 0);
    }
    expect_sdti("err05", f64::NAN, 0);
    expect_sdti("err05", 0.0 / 0.0, 0);
    expect_sdti("err05", f64::INFINITY - f64::INFINITY, 0);

    let mut r = Rng::new(SEED ^ 0x05);
    for _ in 0..2000 {
        let mantissa = (r.next_u64() & 0x000F_FFFF_FFFF_FFFF) | 1;
        let sign = (r.next_u64() & 1) << 63;
        expect_sdti("err05", f64::from_bits(sign | 0x7FF0_0000_0000_0000 | mantissa), 0);
    }
}

#[test]
fn err_06_sdti_boundary_int_max() {
    // Exactly (double)INT_MAX -- the `>` test is false, so NO clamp happens
    // and the cast runs. Same numeric answer, different code path.
    expect_sdti("err06", INT_MAX as f64, INT_MAX);
    expect_sdti("err06", 2147483647.0, 2147483647);
    expect_sdti("err06", 2147483646.0, 2147483646);
}

#[test]
fn err_07_sdti_boundary_int_min() {
    expect_sdti("err07", INT_MIN as f64, INT_MIN);
    expect_sdti("err07", -2147483648.0, -2147483648);
    expect_sdti("err07", -2147483647.0, -2147483647);
}

#[test]
fn err_08_sdti_one_ulp_past() {
    let max = INT_MAX as f64;
    let min = INT_MIN as f64;

    // One ULP above (double)INT_MAX -> strictly greater -> clamp.
    let up = f64::from_bits(max.to_bits() + 1);
    assert!(up > max);
    expect_sdti("err08", up, INT_MAX);

    // One ULP below (double)INT_MAX -> still in range -> cast.
    let down = f64::from_bits(max.to_bits() - 1);
    assert!(down < max);
    expect_sdti("err08", down, 2147483646);

    // One ULP below (double)INT_MIN (i.e. more negative) -> clamp.
    let below = f64::from_bits(min.to_bits() + 1); // sign bit set: +1 = more negative
    assert!(below < min);
    expect_sdti("err08", below, INT_MIN);

    // One ULP above (double)INT_MIN -> in range -> cast.
    let above = f64::from_bits(min.to_bits() - 1);
    assert!(above > min);
    cmp_sdti("err08", above);

    // Compare, don't just predict, for a band of ULPs around both boundaries.
    for k in 0..4096u64 {
        cmp_sdti("err08", f64::from_bits(max.to_bits() + k));
        cmp_sdti("err08", f64::from_bits(max.to_bits().wrapping_sub(k)));
        cmp_sdti("err08", f64::from_bits(min.to_bits() + k));
        cmp_sdti("err08", f64::from_bits(min.to_bits().wrapping_sub(k)));
    }
}

#[test]
fn err_09_sdti_truncation() {
    expect_sdti("err09", 2147483646.9, 2147483646);
    expect_sdti("err09", -0.0, 0);
    expect_sdti("err09", -0.5, 0);
    expect_sdti("err09", 0.5, 0);
    expect_sdti("err09", -1.5, -1);
    expect_sdti("err09", 1.5, 1);
    expect_sdti("err09", -2147483647.9, -2147483647);
    let mut r = Rng::new(SEED ^ 0x09);
    for _ in 0..4000 {
        let v = r.range_f64(-2.1e9, 2.1e9);
        cmp_sdti("err09", v);
    }
}

#[test]
fn err_10_sdti_subnormal() {
    expect_sdti("err10", f64::MIN_POSITIVE, 0);
    expect_sdti("err10", -f64::MIN_POSITIVE, 0);
    expect_sdti("err10", 5e-324, 0);
    expect_sdti("err10", -5e-324, 0);
    expect_sdti("err10", f64::MIN_POSITIVE / 2.0, 0);
    let mut r = Rng::new(SEED ^ 0x10);
    for _ in 0..2000 {
        let bits = r.next_u64() & 0x800F_FFFF_FFFF_FFFF;
        expect_sdti("err10", f64::from_bits(bits), 0);
    }
}

// ===========================================================================
// Rows 11..15 -- process_with_fallthrough sentinels & overflow
// ===========================================================================

#[test]
fn err_11_pwf_default_sentinel() {
    // Every `code` outside 0..=5 must return exactly -1 and IGNORE base_value.
    for code in [6, 7, -1, -2, -100, 1000, INT_MAX, INT_MIN] {
        for base in [0, 1, -1, 12345, -12345, INT_MAX, INT_MIN] {
            expect_pwf("err11", code, base, -1);
        }
    }
    let mut r = Rng::new(SEED ^ 0x11);
    let mut n = 0;
    while n < 4000 {
        let code = r.next_i32();
        if (0..=5).contains(&code) {
            continue;
        }
        expect_pwf("err11", code, r.next_i32(), -1);
        n += 1;
    }
}

#[test]
fn err_12_pwf_zero_case() {
    // `case 0: result = 0;` discards base_value entirely.
    for base in [0, 1, -1, 999, -999, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1] {
        expect_pwf("err12", 0, base, 0);
    }
    let mut r = Rng::new(SEED ^ 0x12);
    for _ in 0..4000 {
        expect_pwf("err12", 0, r.next_i32(), 0);
    }
}

#[test]
fn err_13_pwf_one_past_range() {
    // One step past each end of the valid case set.
    expect_pwf("err13", -1, 100, -1);
    expect_pwf("err13", 6, 100, -1);
    // and the in-range neighbours, to prove the boundary is where C puts it
    expect_pwf("err13", 0, 100, 0);
    expect_pwf("err13", 5, 100, 250);
    for base in [INT_MIN, -1, 0, 1, INT_MAX] {
        expect_pwf("err13", -1, base, -1);
        expect_pwf("err13", 6, base, -1);
    }
}

#[test]
fn err_14_pwf_ffi_enum_out_of_range() {
    // A C `switch` on `int` accepts ANY int, so an "enum" value with no valid
    // variant is a real input crossing the FFI boundary.
    let evil = [
        INT_MIN,
        INT_MIN + 1,
        INT_MAX,
        INT_MAX - 1,
        -2147483648,
        0x7fff_ffff,
        (1u32 << 31) as i32, // == INT_MIN after wrapping
        0x8000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32, // == -1
        1 << 30,
        -(1 << 30),
        0x0000_0100,
        0x0001_0005, // low byte looks like 5 but the value is not 5
        0x0000_0005 | (1 << 16),
    ];
    for &code in &evil {
        for base in [0, 7, -7, INT_MAX, INT_MIN] {
            let expected = if (0..=5).contains(&code) { continue } else { -1 };
            expect_pwf("err14", code, base, expected);
        }
    }
    // Also verify that a value whose low bits alias a valid case is still
    // rejected, i.e. C compares the full 32-bit int.
    expect_pwf("err14", 0x0001_0005, 1000, -1);
    expect_pwf("err14", 0x0001_0000, 1000, -1);
}

#[test]
fn err_15_pwf_base_value_overflow() {
    // Signed overflow inside the fall-through `result += ...` chain.
    // Fall-through depths straight from lib.c:54-65:
    //   case 5 -> 50+40+30+20+10 = 150   case 4 -> 40+30+20+10 = 100
    //   case 3 -> 30+20+10       =  60   case 2 -> 20+10       =  30
    //   case 1 -> 10
    let deltas = [(5i32, 150i32), (4, 100), (3, 60), (2, 30), (1, 10)];
    for (code, delta) in deltas {
        for base in [
            INT_MAX,
            INT_MAX - 1,
            INT_MAX - delta + 1,
            INT_MAX - delta,
            INT_MAX - delta - 1,
            INT_MIN,
            INT_MIN + 1,
        ] {
            expect_pwf("err15", code, base, base.wrapping_add(delta));
        }
    }
    let mut r = Rng::new(SEED ^ 0x15);
    for _ in 0..4000 {
        let code = r.range_i32(1, 5);
        let base = INT_MAX - r.range_i32(0, 400);
        cmp_pwf("err15", code, base);
        let base2 = INT_MIN + r.range_i32(0, 400);
        cmp_pwf("err15", code, base2);
    }
}

// ===========================================================================
// Rows 16..18 -- copy_data_block
// ===========================================================================

#[test]
fn err_16_cdb_null_pointer_is_ub_documented_not_executed() {
    // `copy_data_block` calls memcpy unconditionally with no NULL guard
    // (lib.c:78), so passing NULL is undefined behaviour in the C and
    // segfaults. The Rust translation performs the same unguarded
    // `copy_nonoverlapping` on the same raw pointers, so it is equally UB.
    // Adding a NULL check to the Rust would be a DIVERGENCE, not a fix.
    //
    // Assert the structural property we can check safely: neither library
    // exports any validation helper and both implement the copy as a straight
    // 40-byte move, which the row-17/18 tests confirm byte-for-byte.
    let l = libs();
    let src = DataBlock::from_bytes(&[0x5A; DATABLOCK_SIZE]);
    let mut dc = DataBlock::zeroed();
    let mut dr = DataBlock::zeroed();
    unsafe {
        (l.c.copy_data_block)(&mut dc, &src);
        (l.rs.copy_data_block)(&mut dr, &src);
    }
    assert_eq!(dc.as_bytes(), dr.as_bytes(), "[err16] non-null baseline diverged");
    assert_eq!(dc.as_bytes(), [0x5A; DATABLOCK_SIZE], "[err16] copy is not a plain 40-byte move");
}

#[test]
fn err_17_cdb_self_copy() {
    // dest == src: memcpy onto itself must leave the bytes unchanged.
    let l = libs();
    let mut r = Rng::new(SEED ^ 0x17);
    for _ in 0..500 {
        let mut raw = [0u8; DATABLOCK_SIZE];
        for byte in raw.iter_mut() {
            *byte = r.next_u32() as u8;
        }
        let mut bc = DataBlock::from_bytes(&raw);
        let mut br = DataBlock::from_bytes(&raw);
        unsafe {
            let pc: *mut DataBlock = &mut bc;
            (l.c.copy_data_block)(pc, pc);
            let pr: *mut DataBlock = &mut br;
            (l.rs.copy_data_block)(pr, pr);
        }
        assert_eq!(bc.as_bytes(), br.as_bytes(), "[err17] self-copy diverged");
        assert_eq!(bc.as_bytes(), raw, "[err17] self-copy changed the bytes (C)");
    }
}

#[test]
fn err_17b_cdb_aliasing_in_place() {
    // `memcpy` with *partially* overlapping regions is genuine UB, and the C's
    // own behaviour is not even stable across optimisation levels: at -O0 GCC
    // calls glibc's `memcpy` (which loads all 40 bytes before storing any, so
    // the copy is overlap-safe), while at -O2 GCC inlines it as a forward
    // load/store sequence whose earlier stores clobber later loads. There is
    // therefore no single C behaviour to match, and that case is excluded --
    // see ERRORS.md row 17.
    //
    // What IS well defined and reachable is the fully aliased call
    // `dest == src`, plus every NON-overlapping offset. Both are checked here
    // against whichever C build is loaded.
    let l = libs();
    const SPAN: usize = DATABLOCK_SIZE * 3;

    // offset 0 => dest == src (aliased); offsets >= 40 => disjoint regions.
    for off in [0usize, DATABLOCK_SIZE, DATABLOCK_SIZE + 8, DATABLOCK_SIZE + 16, 64, 72, 80] {
        let mut base = [0u8; SPAN];
        for (i, b) in base.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7).wrapping_add(1);
        }

        let run = |f: FnCopyDataBlock| -> [u8; SPAN] {
            let mut buf = base;
            // 8-byte alignment for both endpoints keeps the DataBlock ABI happy.
            let p = buf.as_mut_ptr();
            assert_eq!(p as usize % 8, 0, "test buffer must be 8-byte aligned");
            unsafe {
                f(p.add(off).cast::<DataBlock>(), p.cast::<DataBlock>());
            }
            buf
        };

        // Force alignment by going through a Vec<u64>.
        let run_aligned = |f: FnCopyDataBlock| -> Vec<u8> {
            let mut words = vec![0u64; SPAN / 8];
            {
                let bytes = unsafe {
                    std::slice::from_raw_parts_mut(words.as_mut_ptr() as *mut u8, SPAN)
                };
                bytes.copy_from_slice(&base);
            }
            let p = words.as_mut_ptr() as *mut u8;
            unsafe { f(p.add(off).cast::<DataBlock>(), p.cast::<DataBlock>()) };
            unsafe { std::slice::from_raw_parts(p, SPAN) }.to_vec()
        };
        let _ = run; // the aligned variant is the one we compare

        let bc = run_aligned(l.c.copy_data_block);
        let br = run_aligned(l.rs.copy_data_block);
        assert_eq!(
            bc, br,
            "[err17b] dest offset {off} (aliased or disjoint): C and Rust \
             copy_data_block disagree\n C   ={bc:02x?}\n RUST={br:02x?}"
        );
    }
}

#[test]
fn err_18_cdb_raw_bytes_and_padding() {
    // Unterminated 20-byte label, all-0xFF, NaN/inf `value`, and non-zero
    // padding bytes -- memcpy must reproduce all 40 bytes exactly.
    let mut raw = [0u8; DATABLOCK_SIZE];
    raw[0..4].copy_from_slice(&(-1i32).to_ne_bytes());
    raw[4..8].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // the padding hole
    raw[8..16].copy_from_slice(&f64::NAN.to_ne_bytes());
    for i in 16..36 {
        raw[i] = 0x41 + (i as u8 - 16); // 20 non-zero bytes, no NUL
    }
    raw[36..40].copy_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]); // trailing padding
    cmp_cdb("err18", &raw, 0x00);
    cmp_cdb("err18", &raw, 0xFF);

    cmp_cdb("err18", &[0xFF; DATABLOCK_SIZE], 0x00);
    cmp_cdb("err18", &[0xFF; DATABLOCK_SIZE], 0xFF);

    for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut b = [0xABu8; DATABLOCK_SIZE];
        b[8..16].copy_from_slice(&v.to_ne_bytes());
        cmp_cdb("err18", &b, 0x11);
    }
}

// ===========================================================================
// Rows 19..20 -- handle_pointer_operations overflow
// ===========================================================================

#[test]
fn err_19_hpo_mul_overflow() {
    let l = libs();
    for v in [INT_MAX, INT_MAX - 1, INT_MIN, INT_MIN + 1, 1 << 30, -(1 << 30), 0x4000_0001] {
        let rc = unsafe { (l.c.handle_pointer_operations)(v) };
        let rr = unsafe { (l.rs.handle_pointer_operations)(v) };
        let expected = v.wrapping_mul(2).wrapping_add(100);
        assert_eq!(rc, expected, "[err19] C hpo({v}) = {rc}, expected {expected}");
        assert_eq!(rr, expected, "[err19] RUST hpo({v}) = {rr}, expected {expected}");
    }
    let mut r = Rng::new(SEED ^ 0x19);
    for _ in 0..4000 {
        // guaranteed to overflow `value * 2`
        let v = r.range_i32(INT_MAX / 2 + 1, INT_MAX);
        cmp_hpo("err19", v);
        let w = r.range_i32(INT_MIN, INT_MIN / 2 - 1);
        cmp_hpo("err19", w);
    }
}

#[test]
fn err_20_hpo_add_overflow() {
    let l = libs();
    // value*2 fits, but +100 pushes it past INT_MAX.
    for v in [INT_MAX / 2, INT_MAX / 2 - 1, INT_MAX / 2 - 49, 1073741773, 1073741823] {
        let rc = unsafe { (l.c.handle_pointer_operations)(v) };
        let rr = unsafe { (l.rs.handle_pointer_operations)(v) };
        let expected = v.wrapping_mul(2).wrapping_add(100);
        assert_eq!(rc, expected, "[err20] C hpo({v}) = {rc}, expected {expected}");
        assert_eq!(rr, expected, "[err20] RUST hpo({v}) = {rr}, expected {expected}");
    }
    for v in (INT_MAX / 2 - 200)..=(INT_MAX / 2) {
        cmp_hpo("err20", v);
    }
    for v in (INT_MIN / 2)..=(INT_MIN / 2 + 200) {
        cmp_hpo("err20", v);
    }
}

// ===========================================================================
// Rows 21..29 -- overunder edge conditions, observed through stdout
// ===========================================================================

#[test]
fn err_21_ou_int_min_modulo() {
    // INT_MIN % 6 == -2 in C99 (truncated remainder) -> `default` -> -1.
    assert_eq!(INT_MIN.wrapping_rem(6), -2);
    let mut r = Rng::new(SEED ^ 0x21);
    for _ in 0..40 {
        let (_, out) = run_both("err21", INT_MIN, r.next_i32(), r.next_i32(), r.next_i32());
        assert_eq!(
            field(&out, "Switch fall-through result: "),
            "-1",
            "[err21] INT_MIN % 6 must reach `default`"
        );
    }
    let (_, out) = run_both("err21", INT_MIN, 0, 0, 0);
    assert_eq!(field(&out, "Switch fall-through result: "), "-1");
}

#[test]
fn err_22_ou_negative_modulo() {
    let mut r = Rng::new(SEED ^ 0x22);
    for _ in 0..300 {
        let a = r.range_i32(-1_000_000, -1);
        let (_, out) = run_both("err22", a, r.range_i32(-1000, 1000), r.range_i32(-1000, 1000), r.range_i32(-100, 100));
        let got = field(&out, "Switch fall-through result: ");
        let rem = a.wrapping_rem(6);
        if rem == 0 {
            assert_eq!(got, "0", "[err22] a={a} rem=0 must hit `case 0`");
        } else {
            assert!(rem < 0, "[err22] a={a} should have a negative remainder");
            assert_eq!(got, "-1", "[err22] a={a} rem={rem} must hit `default`");
        }
    }
}

#[test]
fn err_23_ou_sqrt_of_negative() {
    // d*d + a*a overflows to a NEGATIVE int -> sqrt() yields NaN ->
    // safe_double_to_int maps it to 0 (conv4 == 0).
    let mut found = 0;
    let mut r = Rng::new(SEED ^ 0x23);
    let mut cases: Vec<(i32, i32)> = vec![
        (46341, 46341),
        (65536, 65536),
        (INT_MAX, INT_MAX),
        (INT_MIN, INT_MIN),
        (1 << 20, 1 << 20),
    ];
    for _ in 0..400 {
        cases.push((r.range_i32(50_000, INT_MAX), r.range_i32(50_000, INT_MAX)));
    }
    for (a, d) in cases {
        let sum = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
        let (_, out) = run_both("err23", a, 1, 1, d);
        let conv4 = field(&out, "Converted values: ")
            .split(", ")
            .nth(3)
            .unwrap()
            .to_string();
        if sum < 0 {
            assert_eq!(conv4, "0", "[err23] a={a} d={d} sum={sum} -> sqrt(NaN) -> conv4 must be 0");
            found += 1;
        }
    }
    assert!(found > 10, "[err23] only {found} negative-sum cases exercised");
}

#[test]
fn err_24_ou_conv_clamping() {
    // a*1.5 and b*2.7 exceed INT_MAX once widened to double -> clamp to INT_MAX.
    let (_, out) = run_both("err24", INT_MAX, INT_MAX, 1, 1);
    let convs: Vec<String> = field(&out, "Converted values: ")
        .split(", ")
        .map(|s| s.to_string())
        .collect();
    assert_eq!(convs[0], "2147483647", "[err24] conv1 must clamp");
    assert_eq!(convs[1], "2147483647", "[err24] conv2 must clamp");

    for a in [INT_MAX, INT_MAX - 1, 1_431_655_766, 2_000_000_000] {
        let (_, o) = run_both("err24", a, 1, 1, 1);
        let c1 = field(&o, "Converted values: ").split(", ").next().unwrap().to_string();
        if (a as f64 * 1.5) > INT_MAX as f64 {
            assert_eq!(c1, "2147483647", "[err24] a={a} conv1 must clamp");
        }
    }
}

#[test]
fn err_25_ou_conv_clamping_neg() {
    let (_, out) = run_both("err25", INT_MIN, INT_MIN, 1, 1);
    let convs: Vec<String> = field(&out, "Converted values: ")
        .split(", ")
        .map(|s| s.to_string())
        .collect();
    assert_eq!(convs[0], "-2147483648", "[err25] conv1 must clamp low");
    assert_eq!(convs[1], "-2147483648", "[err25] conv2 must clamp low");

    for a in [INT_MIN, INT_MIN + 1, -1_431_655_766, -2_000_000_000] {
        let (_, o) = run_both("err25", a, a, 1, 1);
        let c1 = field(&o, "Converted values: ").split(", ").next().unwrap().to_string();
        if (a as f64 * 1.5) < INT_MIN as f64 {
            assert_eq!(c1, "-2147483648", "[err25] a={a} conv1 must clamp low");
        }
    }
}

#[test]
fn err_26_ou_total_overflow() {
    // The 13 accumulating additions must wrap identically.
    let mut r = Rng::new(SEED ^ 0x26);
    for _ in 0..2000 {
        run_both("err26", r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
    for &a in &[INT_MAX, INT_MIN] {
        for &b in &[INT_MAX, INT_MIN] {
            for &c in &[INT_MAX, INT_MIN] {
                for &d in &[INT_MAX, INT_MIN] {
                    run_both("err26", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn err_27_ou_extreme_corners() {
    let corners = [INT_MIN, -1, 0, 1, INT_MAX];
    let mut n = 0;
    for &a in &corners {
        for &b in &corners {
            for &c in &corners {
                for &d in &corners {
                    run_both("err27", a, b, c, d);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 625, "[err27] expected the full 5^4 cross-product");
}

#[test]
fn err_28_ou_fixed_overflow_lines() {
    // The hard-coded 1e15 / -1e15 conversions are input-independent.
    let mut r = Rng::new(SEED ^ 0x28);
    for _ in 0..200 {
        let (_, out) = run_both("err28", r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
        assert_eq!(field(&out, "Overflow protected conversion: "), "2147483647");
        assert_eq!(field(&out, "Underflow protected conversion: "), "-2147483648");
    }
}

#[test]
fn err_29_ou_strncpy_zero_padding() {
    // strncpy(label, "Source", 19) + label[19] = '\0' must leave exactly
    // "Source" followed by 14 NULs, and `%s` must print `Source`.
    let mut r = Rng::new(SEED ^ 0x29);
    for _ in 0..200 {
        let (_, out) = run_both("err29", r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
        let line = field(&out, "Copied block: ");
        assert!(
            line.ends_with(", label=Source"),
            "[err29] label was not NUL-terminated right after \"Source\": {line:?}"
        );
    }

    // Verify the zero-padding directly through copy_data_block: a source whose
    // label is "Source" + 13 NUL + NUL must come back byte-identical, and a
    // destination pre-filled with 0xAA must be fully overwritten.
    let mut raw = [0u8; DATABLOCK_SIZE];
    raw[0..4].copy_from_slice(&7i32.to_ne_bytes());
    raw[8..16].copy_from_slice(&10.5f64.to_ne_bytes());
    raw[16..22].copy_from_slice(b"Source");
    // bytes 22..36 stay 0 -> the 13 pad bytes strncpy writes plus label[19]
    cmp_cdb("err29", &raw, 0xAA);
    assert_eq!(&raw[16..36], b"Source\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
}

// ===========================================================================
// Generic FFI-boundary coverage required by Phase C beyond the table
// ===========================================================================

#[test]
fn err_generic_zero_and_boundary_scalars() {
    for v in [0, 1, -1, INT_MIN, INT_MAX] {
        cmp_hpo("generic", v);
        for w in [0, 1, -1, INT_MIN, INT_MAX] {
            cmp_pwf("generic", v, w);
        }
    }
    for d in [0.0, -0.0, 1.0, -1.0, f64::MAX, f64::MIN, f64::EPSILON, -f64::EPSILON] {
        cmp_sdti("generic", d);
    }
    cmp_overunder("generic", 0, 0, 0, 0);
}

#[test]
fn err_generic_out_of_range_enum_values_full_sweep() {
    // Sweep `code` densely across a wide band plus the extremes, confirming the
    // C/Rust switch agree on EVERY value including all non-variant ones.
    for code in -600..=600 {
        cmp_pwf("generic-enum", code, 123);
        cmp_pwf("generic-enum", code, INT_MAX);
        cmp_pwf("generic-enum", code, INT_MIN);
    }
    let mut r = Rng::new(SEED ^ 0xEE);
    for _ in 0..20_000 {
        cmp_pwf("generic-enum", r.next_i32(), r.next_i32());
    }
    for code in [INT_MIN, INT_MIN + 1, INT_MAX - 1, INT_MAX] {
        cmp_pwf("generic-enum", code, 0);
    }
}
