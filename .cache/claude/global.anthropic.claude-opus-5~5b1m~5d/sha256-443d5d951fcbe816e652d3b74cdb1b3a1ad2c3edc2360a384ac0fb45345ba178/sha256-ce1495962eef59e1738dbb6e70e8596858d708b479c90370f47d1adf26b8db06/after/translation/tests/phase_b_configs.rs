// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH the C `.so` and the
// Rust `.so` through their exported `driver` symbol (via libloading) and
// compares captured stdout byte-for-byte. Rows that describe a value class use
// many randomized inputs from a fixed seed rather than one hand-picked value.

mod common;
use common::*;

// ---------------------------------------------------------------- C1
fn c1_zero_all_zero_byte_pattern() {
    let out = assert_same(0);
    assert_eq!(
        out,
        b"00000000030000000000000000000040\n".to_vec(),
        "ground-truth check for driver(0)"
    );
}

// ---------------------------------------------------------------- C2
fn c2_one_smallest_positive_zero_padding() {
    let out = assert_same(1);
    assert_eq!(
        out,
        b"01000000030000000000000000000040\n".to_vec(),
        "ground-truth check for driver(1)"
    );
}

// ---------------------------------------------------------------- C3
fn c3_single_byte_values_exhaustive() {
    // All 256 one-byte patterns, exhaustively rather than sampled.
    let xs: Vec<i32> = (0..=0xFF).collect();
    assert_same_all("C3 single-byte exhaustive", &xs);
}

// ---------------------------------------------------------------- C4
fn c4_two_byte_values_random() {
    let mut rng = Rng::new(SEED ^ 4);
    let xs: Vec<i32> = (0..512).map(|_| rng.in_range(0x0100, 0xFFFF)).collect();
    assert_same_all("C4 two-byte", &xs);
}

// ---------------------------------------------------------------- C5
fn c5_three_byte_values_random() {
    let mut rng = Rng::new(SEED ^ 5);
    let xs: Vec<i32> = (0..512).map(|_| rng.in_range(0x01_0000, 0xFF_FFFF)).collect();
    assert_same_all("C5 three-byte", &xs);
}

// ---------------------------------------------------------------- C6
fn c6_full_width_positive_random() {
    let mut rng = Rng::new(SEED ^ 6);
    let xs: Vec<i32> = (0..512)
        .map(|_| rng.in_range(0x0100_0000, i32::MAX))
        .collect();
    assert_same_all("C6 full-width positive", &xs);
}

// ---------------------------------------------------------------- C7
fn c7_small_negative_values() {
    // -1 ..= -255 exhaustively: high bytes are 0xff, low byte varies.
    let xs: Vec<i32> = (1..=255).map(|v: i32| -v).collect();
    assert_same_all("C7 small negative", &xs);
}

// ---------------------------------------------------------------- C8
fn c8_full_range_negative_random() {
    let mut rng = Rng::new(SEED ^ 8);
    let xs: Vec<i32> = (0..512).map(|_| rng.in_range(i32::MIN, -1)).collect();
    assert_same_all("C8 full-range negative", &xs);
}

// ---------------------------------------------------------------- C9
fn c9_embedded_zero_bytes() {
    let xs: Vec<i32> = [
        0x00FF_00FFu32,
        0xFF00_FF00,
        0x0001_0000,
        0x0000_FF00,
        0x00FF_FF00,
        0xFF00_00FF,
        0x0000_00FF,
        0xFF00_0000,
        0x0000_0100,
        0x0100_0001,
        0x00AA_0000,
        0x0000_0000,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    assert_same_all("C9 embedded zero bytes", &xs);
}

// ---------------------------------------------------------------- C10
fn c10_all_bytes_high_bit_set() {
    // Probes char vs unsigned char sign-extension in the hex printer.
    let mut xs: Vec<i32> = [
        0x8080_8080u32,
        0xFFFF_FFFF,
        0x8090_A0B0,
        0xFEDC_BA98,
        0x8000_0000,
        0xC0C0_C0C0,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    // Every single byte value >= 0x80 in the low lane, too.
    xs.extend((0x80..=0xFF).map(|b: u32| (0x8080_8000u32 | b) as i32));
    assert_same_all("C10 high-bit bytes", &xs);
}

// ---------------------------------------------------------------- C11
fn c11_all_bytes_below_0x10() {
    // Every byte needs %02x zero-padding.
    let mut xs: Vec<i32> = vec![0x0102_0304, 0x0F0E_0D0C, 0x0000_0001, 0x0101_0101];
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..128 {
        let b = |r: &mut Rng| (r.next_u32() & 0x0F) as i32;
        let v = (b(&mut rng) << 24) | (b(&mut rng) << 16) | (b(&mut rng) << 8) | b(&mut rng);
        xs.push(v);
    }
    assert_same_all("C11 low-nibble bytes", &xs);
}

// ---------------------------------------------------------------- C12
fn c12_printable_ascii_byte_patterns() {
    let xs: Vec<i32> = [
        0x4142_4344u32, // "ABCD"
        0x2F2E_2D2C,
        0x7A79_7877,
        0x3031_3233,
        0x2020_2020,
        0x7F7F_7F7F,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    assert_same_all("C12 printable ASCII", &xs);
}

// ---------------------------------------------------------------- C13
fn c13_boundary_values() {
    let xs: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
        0x7FFF_FFFF,
        0x8000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
    ];
    assert_same_all("C13 boundaries", &xs);
}

// ---------------------------------------------------------------- C14
fn c14_powers_of_two_and_masks() {
    let mut xs: Vec<i32> = Vec::new();
    for k in 0..32u32 {
        xs.push((1u32 << k) as i32); // single 1-bit walking all 32 lanes
        xs.push(((1u32 << k).wrapping_sub(1)) as i32); // low-mask
        xs.push(!(1u32 << k) as i32); // single 0-bit
    }
    assert_same_all("C14 powers of two", &xs);
}

// ---------------------------------------------------------------- C15
fn c15_bulk_uniform_random_i32() {
    let mut rng = Rng::new(SEED);
    let xs: Vec<i32> = (0..4096).map(|_| rng.next_i32()).collect();
    assert_same_all("C15 bulk random", &xs);
}

// ---------------------------------------------------------------- C16
fn c16_struct_shape_invariants_hold_for_both_libraries() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..256 {
        let x = rng.next_i32();
        let c = run_c(x);
        let r = run_rust(x);
        // Invariants asserted independently against BOTH implementations.
        check_shape(x, &c);
        check_shape(x, &r);
        assert_eq!(c, r, "divergence at driver({x})");
    }
}

// ---------------------------------------------------------------- C17
fn c17_call_multiplicity_and_interleaving() {
    let mut rng = Rng::new(SEED ^ 17);
    let xs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();

    // (a) many sequential calls inside ONE capture: output must concatenate
    // in call order identically for both libraries.
    let c_seq = run_c_seq(&xs);
    let r_seq = run_rust_seq(&xs);
    assert_eq!(
        c_seq, r_seq,
        "sequential multi-call output diverged for {} calls",
        xs.len()
    );
    let model: Vec<u8> = xs.iter().flat_map(|&x| expected_output(x)).collect();
    assert_eq!(c_seq, model, "C sequential output disagrees with the model");
    assert_eq!(c_seq.len(), xs.len() * 33);

    // (b) interleaved C / Rust calls in alternating order: each library must
    // still be stateless, so per-call outputs stay pairwise equal.
    for &x in &xs {
        let c = run_c(x);
        let r = run_rust(x);
        assert_eq!(c, r, "interleaved divergence at driver({x})");
        let r2 = run_rust(x);
        let c2 = run_c(x);
        assert_eq!(c2, r2, "reverse-order interleaved divergence at driver({x})");
        assert_eq!(c, c2, "C not idempotent for driver({x})");
        assert_eq!(r, r2, "Rust not idempotent for driver({x})");
    }
}

// ---------------------------------------------------------------- C18
fn c18_repeated_identical_input() {
    for &x in &[0, 1, -1, i32::MIN, i32::MAX, 0x1234_5678] {
        let xs = vec![x; 64];
        let c = run_c_seq(&xs);
        let r = run_rust_seq(&xs);
        assert_eq!(c, r, "repeated-input divergence for driver({x})");
        // No accumulation: 64 identical lines.
        let one = expected_output(x);
        let expect: Vec<u8> = one.iter().cycle().take(one.len() * 64).copied().collect();
        assert_eq!(c, expect, "C accumulated state across repeats of {x}");
    }
}

// ---------------------------------------------------------------- C19
fn c19_symbol_parity_c_vs_rust() {
    let c_syms = dynamic_symbols(&c_so_path());
    let r_syms = dynamic_symbols(&rust_so_path());

    assert!(
        c_syms.iter().any(|s| s == "driver"),
        "the C .so must export `driver`; got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c_syms:?}\nRust: {r_syms:?}"
    );

    // `print_hex` is `static` in C, so neither object may export it.
    assert!(
        !c_syms.iter().any(|s| s == "print_hex"),
        "unexpected: C exports the static print_hex"
    );
    assert!(
        !r_syms.iter().any(|s| s == "print_hex"),
        "Rust must not export print_hex (it is `static` in the C)"
    );
}

// ---------------------------------------------------------------- runner
fn main() {
    let cases: &[(&str, fn())] = &[
        ("c1_zero_all_zero_byte_pattern", c1_zero_all_zero_byte_pattern as fn()),
        ("c2_one_smallest_positive_zero_padding", c2_one_smallest_positive_zero_padding as fn()),
        ("c3_single_byte_values_exhaustive", c3_single_byte_values_exhaustive as fn()),
        ("c4_two_byte_values_random", c4_two_byte_values_random as fn()),
        ("c5_three_byte_values_random", c5_three_byte_values_random as fn()),
        ("c6_full_width_positive_random", c6_full_width_positive_random as fn()),
        ("c7_small_negative_values", c7_small_negative_values as fn()),
        ("c8_full_range_negative_random", c8_full_range_negative_random as fn()),
        ("c9_embedded_zero_bytes", c9_embedded_zero_bytes as fn()),
        ("c10_all_bytes_high_bit_set", c10_all_bytes_high_bit_set as fn()),
        ("c11_all_bytes_below_0x10", c11_all_bytes_below_0x10 as fn()),
        ("c12_printable_ascii_byte_patterns", c12_printable_ascii_byte_patterns as fn()),
        ("c13_boundary_values", c13_boundary_values as fn()),
        ("c14_powers_of_two_and_masks", c14_powers_of_two_and_masks as fn()),
        ("c15_bulk_uniform_random_i32", c15_bulk_uniform_random_i32 as fn()),
        ("c16_struct_shape_invariants_hold_for_both_libraries", c16_struct_shape_invariants_hold_for_both_libraries as fn()),
        ("c17_call_multiplicity_and_interleaving", c17_call_multiplicity_and_interleaving as fn()),
        ("c18_repeated_identical_input", c18_repeated_identical_input as fn()),
        ("c19_symbol_parity_c_vs_rust", c19_symbol_parity_c_vs_rust as fn()),
    ];
    run_suite("phase_b_configs", cases);
}
