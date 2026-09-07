//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads both `.so`s through `libloading` and compares the bytes
//! each writes to `stdout`. The Rust functions are only ever reached through
//! their exported `#[no_mangle]` symbols.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// Runs both entry points for a set of `char` values and compares outputs.
fn compare_print_hex(row: &str, values: impl IntoIterator<Item = u8>) {
    for v in values {
        let iv = v as c_char;
        let c_out = cap_print_hex(c(), iv);
        let r_out = cap_print_hex(rs(), iv);
        assert_same(
            &format!("{row}: printHexCharLine(0x{v:02x} = {iv})"),
            &c_out,
            &r_out,
        );
        // Sanity: the C must actually have produced something.
        assert!(
            !c_out.is_empty(),
            "{row}: C produced no output for printHexCharLine(0x{v:02x})"
        );
    }
}

fn compare_driver(row: &str, values: impl IntoIterator<Item = u8>) {
    for v in values {
        let iv = v as c_char;
        let c_out = cap_driver(c(), iv);
        let r_out = cap_driver(rs(), iv);
        assert_same(&format!("{row}: driver(0x{v:02x} = {iv})"), &c_out, &r_out);
        assert!(
            !c_out.is_empty(),
            "{row}: C produced no output for driver(0x{v:02x})"
        );
    }
}

// ---------------------------------------------------------------------------
// C1 / C2 — exhaustive over the entire input domain (256 values each)
// ---------------------------------------------------------------------------

fn c1_print_hex_char_line_exhaustive_all_256() {
    compare_print_hex("C1", 0u8..=0xFF);
}

fn c2_driver_exhaustive_all_256() {
    compare_driver("C2", 0u8..=0xFF);
}

// ---------------------------------------------------------------------------
// C3..C6 — printHexCharLine value regimes, randomized
// ---------------------------------------------------------------------------

fn c3_print_hex_zero() {
    compare_print_hex("C3", [0u8]);
}

fn c4_print_hex_single_digit_randomized() {
    let mut rng = Rng::new(SEED ^ 4);
    let vals: Vec<u8> = (0..SAMPLES).map(|_| rng.in_range_u8(0x01, 0x0F)).collect();
    compare_print_hex("C4", vals);
}

fn c5_print_hex_two_digit_positive_randomized() {
    let mut rng = Rng::new(SEED ^ 5);
    let vals: Vec<u8> = (0..SAMPLES).map(|_| rng.in_range_u8(0x10, 0x7F)).collect();
    compare_print_hex("C5", vals);
}

fn c6_print_hex_negative_randomized() {
    let mut rng = Rng::new(SEED ^ 6);
    let vals: Vec<u8> = (0..SAMPLES).map(|_| rng.in_range_u8(0x80, 0xFF)).collect();
    compare_print_hex("C6", vals);
}

// ---------------------------------------------------------------------------
// C7..C11 — driver value regimes, randomized + named boundaries
// ---------------------------------------------------------------------------

fn c7_driver_result_single_digit_randomized() {
    let mut rng = Rng::new(SEED ^ 7);
    let vals: Vec<u8> = (0..SAMPLES).map(|_| rng.in_range_u8(0x00, 0x0E)).collect();
    compare_driver("C7", vals);
}

fn c8_driver_result_two_digit_positive_randomized() {
    let mut rng = Rng::new(SEED ^ 8);
    let vals: Vec<u8> = (0..SAMPLES).map(|_| rng.in_range_u8(0x0F, 0x7D)).collect();
    compare_driver("C8", vals);
}

fn c9_driver_overflow_boundary_7e_7f() {
    compare_driver("C9", [0x7Eu8, 0x7F]);
}

fn c10_driver_negative_randomized() {
    let mut rng = Rng::new(SEED ^ 10);
    let vals: Vec<u8> = (0..SAMPLES).map(|_| rng.in_range_u8(0x80, 0xFD)).collect();
    compare_driver("C10", vals);
}

fn c11_driver_boundary_fe_ff() {
    compare_driver("C11", [0xFEu8, 0xFF]);
}

// ---------------------------------------------------------------------------
// C12 / C13 — FFI argument-width axis: full-width int with no char value
// ---------------------------------------------------------------------------

fn wide_arg_values() -> Vec<c_int> {
    let mut vals: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MAX,
        c_int::MIN + 1,
        c_int::MAX - 1,
        0x1FF,
        0x100,
        0x101,
        0x17F,
        0x180,
        -0x1FF,
        0xFFFF,
        0x7FFF_FF00u32 as c_int,
        0xFFFF_FF00u32 as c_int,
        0xFFFF_FF7Fu32 as c_int,
        0xFFFF_FF80u32 as c_int,
        0xDEAD_BEEFu32 as c_int,
        0x0000_FF00,
        0x1234_5678,
    ];
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..SAMPLES {
        vals.push(rng.next_u32() as c_int);
    }
    vals
}

fn c12_print_hex_wide_int_argument() {
    for v in wide_arg_values() {
        let c_out = cap_print_hex_int(c(), v);
        let r_out = cap_print_hex_int(rs(), v);
        assert_same(
            &format!("C12: printHexCharLine as void(*)(int) with {v} (0x{:08x})", v as u32),
            &c_out,
            &r_out,
        );
    }
}

fn c13_driver_wide_int_argument() {
    for v in wide_arg_values() {
        let c_out = cap_driver_int(c(), v);
        let r_out = cap_driver_int(rs(), v);
        assert_same(
            &format!("C13: driver as void(*)(int) with {v} (0x{:08x})", v as u32),
            &c_out,
            &r_out,
        );
    }
}

// ---------------------------------------------------------------------------
// C14 — composed pipeline: driver(x) must equal printHexCharLine(x + 1)
// ---------------------------------------------------------------------------

fn c14_composed_pipeline_driver_equals_print_hex_of_incremented() {
    for v in 0u8..=0xFF {
        let arg = v as c_char;
        let incremented = v.wrapping_add(1) as c_char;

        let c_driver = cap_driver(c(), arg);
        let c_primitive = cap_print_hex(c(), incremented);
        let r_driver = cap_driver(rs(), arg);
        let r_primitive = cap_print_hex(rs(), incremented);

        // Within C: the composition identity the source implies.
        assert_same(
            &format!("C14 (C internal): driver(0x{v:02x}) vs printHexCharLine(0x{:02x})", v.wrapping_add(1)),
            &c_driver,
            &c_primitive,
        );
        // Within Rust: same identity must hold.
        assert_same(
            &format!("C14 (Rust internal): driver(0x{v:02x}) vs printHexCharLine(0x{:02x})", v.wrapping_add(1)),
            &r_driver,
            &r_primitive,
        );
        // Across the boundary, both ways.
        assert_same(&format!("C14 (cross, driver 0x{v:02x})"), &c_driver, &r_driver);
        assert_same(
            &format!("C14 (cross, primitive 0x{:02x})", v.wrapping_add(1)),
            &c_primitive,
            &r_primitive,
        );
    }
}

// ---------------------------------------------------------------------------
// C15 — long randomized interleaved sequence, whole transcript compared
// ---------------------------------------------------------------------------

/// A scripted sequence of calls, so the exact same script can be replayed
/// against either implementation.
#[derive(Clone, Copy)]
enum Op {
    PrintHex(u8),
    Driver(u8),
}

fn build_script(n: usize, seed: u64) -> Vec<Op> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let r = rng.next_u64();
            let v = (r >> 8) as u8;
            if r & 1 == 0 { Op::PrintHex(v) } else { Op::Driver(v) }
        })
        .collect()
}

fn run_script(api: &Api, script: &[Op]) {
    for op in script {
        unsafe {
            match *op {
                Op::PrintHex(v) => (api.print_hex_char_line)(v as c_char),
                Op::Driver(v) => (api.driver)(v as c_char),
            }
        }
    }
}

fn c15_long_interleaved_sequence_transcript() {
    let script = build_script(4096, SEED ^ 15);

    let c_out = capture(|| run_script(c(), &script));
    let r_out = capture(|| run_script(rs(), &script));

    assert_eq!(
        c_out.iter().filter(|b| **b == b'\n').count(),
        script.len(),
        "C15: C emitted one line per call"
    );
    assert_same("C15: 4096-call mixed transcript", &c_out, &r_out);
}

// ---------------------------------------------------------------------------
// C16 — shared stdout: alternate C and Rust calls with no intermediate flush
// ---------------------------------------------------------------------------

fn c16_shared_stdout_interleaved_ordering() {
    let script = build_script(1024, SEED ^ 16);

    // Alternate the two implementations call-by-call into one buffered stream.
    // If the Rust translation wrote via `std::io::stdout` instead of libc
    // `printf`, the two buffers would flush independently and the interleaving
    // would come out reordered.
    let interleaved = capture(|| {
        for (i, op) in script.iter().enumerate() {
            let api = if i % 2 == 0 { c() } else { rs() };
            run_script(api, std::slice::from_ref(op));
        }
    });

    // Reference: the same script run entirely through C.
    let all_c = capture(|| run_script(c(), &script));

    assert_same(
        "C16: C/Rust alternating on one shared, fully-buffered stdout",
        &all_c,
        &interleaved,
    );

    // And entirely through Rust, to pin the ordering from the other side too.
    let all_rust = capture(|| run_script(rs(), &script));
    assert_same("C16: all-Rust vs all-C on the same script", &all_c, &all_rust);
}


// -------------------------------------------------------------------------
// Sequential entry point (`harness = false`; see tests/common/mod.rs for why).
// -------------------------------------------------------------------------

fn main() {
    common::install_quiet_panic_hook();
    let mut r = Runner::new();
    r.row("c1_print_hex_char_line_exhaustive_all_256", c1_print_hex_char_line_exhaustive_all_256);
    r.row("c2_driver_exhaustive_all_256", c2_driver_exhaustive_all_256);
    r.row("c3_print_hex_zero", c3_print_hex_zero);
    r.row("c4_print_hex_single_digit_randomized", c4_print_hex_single_digit_randomized);
    r.row("c5_print_hex_two_digit_positive_randomized", c5_print_hex_two_digit_positive_randomized);
    r.row("c6_print_hex_negative_randomized", c6_print_hex_negative_randomized);
    r.row("c7_driver_result_single_digit_randomized", c7_driver_result_single_digit_randomized);
    r.row("c8_driver_result_two_digit_positive_randomized", c8_driver_result_two_digit_positive_randomized);
    r.row("c9_driver_overflow_boundary_7e_7f", c9_driver_overflow_boundary_7e_7f);
    r.row("c10_driver_negative_randomized", c10_driver_negative_randomized);
    r.row("c11_driver_boundary_fe_ff", c11_driver_boundary_fe_ff);
    r.row("c12_print_hex_wide_int_argument", c12_print_hex_wide_int_argument);
    r.row("c13_driver_wide_int_argument", c13_driver_wide_int_argument);
    r.row("c14_composed_pipeline_driver_equals_print_hex_of_incremented", c14_composed_pipeline_driver_equals_print_hex_of_incremented);
    r.row("c15_long_interleaved_sequence_transcript", c15_long_interleaved_sequence_transcript);
    r.row("c16_shared_stdout_interleaved_ordering", c16_shared_stdout_interleaved_ordering);
    r.finish("Phase B");
}
