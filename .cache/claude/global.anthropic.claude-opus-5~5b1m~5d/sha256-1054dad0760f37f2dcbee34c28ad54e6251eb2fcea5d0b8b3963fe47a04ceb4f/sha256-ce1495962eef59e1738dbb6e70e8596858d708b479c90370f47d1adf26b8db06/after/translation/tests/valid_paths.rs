// Phase B -- valid-path differential tests, one test per CONFIGS.md row.
// Both implementations are reached only through their `.so` exports.

mod common;
use common::*;

// ---- C1..C4: distinguished scalar values ----------------------------------

fn c1_zero() {
    assert_same(0, "C1");
}

fn c2_one() {
    assert_same(1, "C2");
}

fn c3_collides_with_bedrooms_constant() {
    // floors == the hard-coded bedrooms value; bedrooms must remain 3.
    assert_same(3, "C3");
}

fn c4_collides_with_bathrooms_constant() {
    assert_same(2, "C4");
}

// ---- C5..C7: byte-width ranges --------------------------------------------

fn c5_exhaustive_single_byte_range() {
    // Exhaustive 1..=255: every low-byte value, hence every %02x nibble pair.
    assert_same_all(1..=255, "C5");
}

fn c6_two_byte_range_randomized() {
    let mut rng = Rng::new(0xC6_5EED);
    let mut xs: Vec<i32> = (0..512).map(|_| rng.in_range(256, 65535)).collect();
    xs.extend([256, 257, 65534, 65535]); // range endpoints
    assert_same_all(xs, "C6");
}

fn c7_three_byte_range_randomized() {
    let mut rng = Rng::new(0xC7_5EED);
    let mut xs: Vec<i32> = (0..512).map(|_| rng.in_range(0x10000, 0xFF_FFFF)).collect();
    xs.extend([0x10000, 0x10001, 0xFF_FFFE, 0xFF_FFFF]);
    assert_same_all(xs, "C7");
}

// ---- C8..C10: extremes ----------------------------------------------------

fn c8_int_max() {
    assert_same(i32::MAX, "C8");
}

fn c9_int_min() {
    assert_same(i32::MIN, "C9");
}

fn c10_minus_one_all_bits_set() {
    assert_same(-1, "C10");
}

// ---- C11..C12: walking ones / walking zeros ------------------------------

fn c11_walking_ones() {
    let xs: Vec<i32> = (0..32).map(|k| 1i32.wrapping_shl(k)).collect();
    assert_same_all(xs, "C11");
}

fn c12_walking_zeros() {
    let xs: Vec<i32> = (0..32).map(|k| !(1i32.wrapping_shl(k))).collect();
    assert_same_all(xs, "C12");
}

// ---- C13: unsigned-char promotion axis ----------------------------------

fn c13_high_bit_byte_patterns() {
    // If the Rust translation had used `i8`/`c_char` instead of `u8`, printf
    // would sign-extend and print `ffffff80` where C prints `80`.
    let xs: Vec<i32> = vec![
        0x80,
        0x8000,
        0x0080_0000,
        0x8000_0000u32 as i32,
        0x8080_8080u32 as i32,
        0xFF00_FF00u32 as i32,
        0x00FF_00FF,
        0x8000_0080u32 as i32,
        0x0080_8000,
        0xF0F0_F0F0u32 as i32,
    ];
    assert_same_all(xs, "C13");
}

// ---- C14: %02x zero-padding axis ----------------------------------------

fn c14_low_nibble_only_bytes() {
    let xs: Vec<i32> = vec![
        0x0102_0304,
        0x0F0F_0F0F,
        0x0001_0203,
        0x0A0B_0C0D,
        0x0000_0001,
        0x0100_0000,
        0x0000_0100,
        0x0001_0000,
    ];
    assert_same_all(xs, "C14");
}

// ---- C15: interior zero bytes -------------------------------------------

fn c15_interior_zero_patterns() {
    let xs: Vec<i32> = vec![
        0xFF00_00FFu32 as i32,
        0x00FF_FF00,
        0xFF00_FF00u32 as i32,
        0x0000_00FF,
        0xFF00_0000u32 as i32,
        0x00FF_0000,
        0x0000_FF00,
    ];
    assert_same_all(xs, "C15");
}

// ---- C16: randomized negatives ------------------------------------------

fn c16_negative_randomized() {
    let mut rng = Rng::new(0xC16_5EED);
    let mut xs: Vec<i32> = (0..1024)
        .map(|_| rng.in_range(i32::MIN as i64, -1))
        .collect();
    xs.extend([-2, -3, -128, -129, -32768, -32769, i32::MIN + 1]);
    assert_same_all(xs, "C16");
}

// ---- C17: full-range randomized property test ---------------------------

fn c17_full_range_randomized() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let xs: Vec<i32> = (0..4096).map(|_| rng.next_i32()).collect();
    assert_same_all(xs, "C17");
}

// ---- C18: exhaustive per-byte-position sweep ----------------------------

fn c18_exhaustive_byte_position_sweep() {
    let mut xs: Vec<i32> = Vec::with_capacity(4 * 256);
    for shift in [0u32, 8, 16, 24] {
        for b in 0u32..=255 {
            xs.push((b << shift) as i32);
        }
    }
    assert_same_all(xs, "C18");
}

// ---- C19..C21: call-count / call-order axes -----------------------------

fn c19_single_call_in_isolation() {
    // Baseline already covered by C1..C18, restated as its own row: a lone
    // call in a fresh capture window.
    let c = c_out(42);
    let r = rust_out(42);
    assert_eq!(c, r, "[C19] single isolated call diverged");
    assert_eq!(c.len(), 33, "[C19] expected exactly one 33-byte line");
}

fn c20_repeated_same_value_batch() {
    // 64 calls to each library inside ONE capture window: proves no hidden
    // static state accumulates between calls.
    let cf = c_driver();
    let rf = rust_driver();
    let c = capture_stdout(|| {
        for _ in 0..64 {
            unsafe { cf(7) }
        }
    });
    let r = capture_stdout(|| {
        for _ in 0..64 {
            unsafe { rf(7) }
        }
    });
    assert_eq!(c, r, "[C20] repeated-call batch diverged");
    assert_eq!(c.len(), 64 * 33, "[C20] expected 64 lines of 33 bytes");
    // Every line must be identical to the first (statelessness).
    let first = &c[..33];
    for (i, line) in c.chunks(33).enumerate() {
        assert_eq!(line, first, "[C20] line {i} differs => hidden state");
    }
}

fn c21_sequential_distinct_values_batch() {
    let mut rng = Rng::new(0xC21_5EED);
    let xs: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    let cf = c_driver();
    let rf = rust_driver();
    let c = capture_stdout(|| {
        for &x in &xs {
            unsafe { cf(x) }
        }
    });
    let r = capture_stdout(|| {
        for &x in &xs {
            unsafe { rf(x) }
        }
    });
    assert_eq!(c, r, "[C21] sequential distinct-value batch diverged");
    assert_eq!(c.len(), xs.len() * 33, "[C21] wrong total byte count");
    // Ordering: line i must encode xs[i].
    for (i, (line, &x)) in c.chunks(33).zip(xs.iter()).enumerate() {
        let expect: String = x.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            &line[..8],
            expect.as_bytes(),
            "[C21] line {i} out of order for x={x}"
        );
    }
}

// ---- C22: interleaving on the shared libc stdout ------------------------

fn c22_interleaved_c_and_rust_same_stream() {
    // Alternate C and Rust calls inside a SINGLE capture window. Both `.so`s
    // must write through the same process-wide libc `stdout` with the same
    // buffering, so paired lines must be equal and ordering preserved.
    let mut rng = Rng::new(0xC22_5EED);
    let xs: Vec<i32> = (0..128).map(|_| rng.next_i32()).collect();
    let cf = c_driver();
    let rf = rust_driver();

    let out = capture_stdout(|| {
        for &x in &xs {
            unsafe { cf(x) };
            unsafe { rf(x) };
        }
    });

    assert_eq!(out.len(), xs.len() * 2 * 33, "[C22] wrong total byte count");
    for (i, pair) in out.chunks(66).enumerate() {
        let (c_line, r_line) = pair.split_at(33);
        assert_eq!(
            c_line, r_line,
            "[C22] interleaved pair {i} diverged for x={}",
            xs[i]
        );
    }
}

// ---- C23: output-shape invariants ---------------------------------------

fn c23_output_shape_invariants() {
    // sizeof(house_t) == 16 with no padding, bedrooms == 3, bathrooms == 2.0.
    // Asserted for a spread of inputs; `assert_same` re-checks these on every
    // other row too.
    let mut rng = Rng::new(0xC23_5EED);
    let mut xs: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX, 3, 2];
    xs.extend((0..64).map(|_| rng.next_i32()));
    for x in xs {
        let c = c_out(x);
        let r = rust_out(x);
        assert_eq!(c, r, "[C23] divergence for {x}");
        assert_eq!(c.len(), 33, "[C23] sizeof(house_t) must be 16 => 33 bytes");
        assert_eq!(&c[16..32], b"0000000000000040", "[C23] 2.0 as LE double");
        assert_eq!(&c[8..16], b"03000000", "[C23] bedrooms == 3");
    }
}

// ---- C24: `print_hex` must stay private --------------------------------

fn c24_print_hex_is_not_exported() {
    assert!(
        !c_has_symbol(b"print_hex"),
        "[C24] print_hex is `static` in C and must not be an exported symbol"
    );
    assert!(
        !rust_has_symbol(b"print_hex"),
        "[C24] Rust must not export print_hex either (C keeps it file-local)"
    );
    // And the one symbol that IS public must resolve in both.
    assert!(c_has_symbol(b"driver"), "[C24] C .so must export driver");
    assert!(rust_has_symbol(b"driver"), "[C24] Rust .so must export driver");
}

// --- sequential runner entry point (harness = false) ---------------------

fn main() {
    common::run_suite(
        "valid_paths",
        &[
        ("c1_zero", c1_zero as fn()),
        ("c2_one", c2_one as fn()),
        ("c3_collides_with_bedrooms_constant", c3_collides_with_bedrooms_constant as fn()),
        ("c4_collides_with_bathrooms_constant", c4_collides_with_bathrooms_constant as fn()),
        ("c5_exhaustive_single_byte_range", c5_exhaustive_single_byte_range as fn()),
        ("c6_two_byte_range_randomized", c6_two_byte_range_randomized as fn()),
        ("c7_three_byte_range_randomized", c7_three_byte_range_randomized as fn()),
        ("c8_int_max", c8_int_max as fn()),
        ("c9_int_min", c9_int_min as fn()),
        ("c10_minus_one_all_bits_set", c10_minus_one_all_bits_set as fn()),
        ("c11_walking_ones", c11_walking_ones as fn()),
        ("c12_walking_zeros", c12_walking_zeros as fn()),
        ("c13_high_bit_byte_patterns", c13_high_bit_byte_patterns as fn()),
        ("c14_low_nibble_only_bytes", c14_low_nibble_only_bytes as fn()),
        ("c15_interior_zero_patterns", c15_interior_zero_patterns as fn()),
        ("c16_negative_randomized", c16_negative_randomized as fn()),
        ("c17_full_range_randomized", c17_full_range_randomized as fn()),
        ("c18_exhaustive_byte_position_sweep", c18_exhaustive_byte_position_sweep as fn()),
        ("c19_single_call_in_isolation", c19_single_call_in_isolation as fn()),
        ("c20_repeated_same_value_batch", c20_repeated_same_value_batch as fn()),
        ("c21_sequential_distinct_values_batch", c21_sequential_distinct_values_batch as fn()),
        ("c22_interleaved_c_and_rust_same_stream", c22_interleaved_c_and_rust_same_stream as fn()),
        ("c23_output_shape_invariants", c23_output_shape_invariants as fn()),
        ("c24_print_hex_is_not_exported", c24_print_hex_is_not_exported as fn()),
        ],
    );
}
