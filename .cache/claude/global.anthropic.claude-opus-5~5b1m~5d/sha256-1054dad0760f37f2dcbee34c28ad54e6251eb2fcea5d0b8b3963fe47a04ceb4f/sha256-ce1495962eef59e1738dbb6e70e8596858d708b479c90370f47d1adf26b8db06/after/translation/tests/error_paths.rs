// Phase C -- error-path differential tests, one test per ERRORS.md row.
//
// The library has ZERO rejection branches (no `return -1`, no NULL check, no
// assert, no range check -- see ERRORS.md). The "error surface" is therefore
// the set of generic boundary inputs, and the property to verify is that BOTH
// implementations agree they are NOT errors, producing byte-identical output
// and returning normally. Structurally-impossible rows are asserted
// structurally.

mod common;
use common::*;

// ---- E1: the library has no error returns at all ------------------------

fn e1_library_has_no_rejection_branch() {
    // Mechanically re-verify the premise against the C source itself, so this
    // row cannot silently rot if c_src ever changes.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/driver.c"),
    )
    .expect("read c_src/src/driver.c");

    let code: Vec<&str> = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect();
    let code = code.join("\n");

    for forbidden in ["return", "assert", "NULL", "errno", "exit(", "abort("] {
        assert!(
            !code.contains(forbidden),
            "[E1] c_src/src/driver.c unexpectedly contains `{forbidden}` -- \
             the error surface changed and ERRORS.md must be regenerated"
        );
    }
    // Exactly one conditional exists, and it is the print_hex loop control.
    assert_eq!(
        code.matches("if").count(),
        0,
        "[E1] no `if` statement should exist in driver.c"
    );
    assert_eq!(
        code.matches("for").count(),
        1,
        "[E1] exactly one `for` (the print_hex loop) should exist"
    );
}

// ---- E2..E5: value boundaries, all of which are NON-errors ---------------

fn e2_zero_is_not_an_error() {
    assert_same(0, "E2");
    assert_eq!(
        c_out(0),
        b"00000000030000000000000000000040\n".to_vec(),
        "[E2] exact expected C output"
    );
}

fn e3_int_max_is_not_an_error() {
    assert_same(i32::MAX, "E3");
    assert_eq!(
        c_out(i32::MAX),
        b"ffffff7f030000000000000000000040\n".to_vec(),
        "[E3] exact expected C output"
    );
}

fn e4_int_min_is_not_an_error() {
    assert_same(i32::MIN, "E4");
    assert_eq!(
        c_out(i32::MIN),
        b"00000080030000000000000000000040\n".to_vec(),
        "[E4] exact expected C output"
    );
}

fn e5_minus_one_sentinel_is_not_an_error() {
    assert_same(-1, "E5");
    assert_eq!(
        c_out(-1),
        b"ffffffff030000000000000000000040\n".to_vec(),
        "[E5] exact expected C output"
    );
}

// ---- E6..E7: unsigned bit patterns one step past the signed range -------

/// Call `driver` through a signature that declares the parameter `unsigned int`,
/// so we genuinely push out-of-signed-range bit patterns across the ABI rather
/// than converting them in Rust first.
type DriverU = unsafe extern "C" fn(u32);

fn c_out_u(x: u32) -> Vec<u8> {
    let f: DriverU = unsafe { std::mem::transmute(c_driver()) };
    capture_stdout(|| unsafe { f(x) })
}

fn rust_out_u(x: u32) -> Vec<u8> {
    let f: DriverU = unsafe { std::mem::transmute(rust_driver()) };
    capture_stdout(|| unsafe { f(x) })
}

fn e6_unsigned_0x80000000_past_int_max() {
    let c = c_out_u(0x8000_0000);
    let r = rust_out_u(0x8000_0000);
    assert_eq!(c, r, "[E6] divergence for unsigned 0x80000000");
    // Must be reinterpreted, not rejected -- identical to E4 (INT_MIN).
    assert_eq!(c, c_out(i32::MIN), "[E6] must match INT_MIN reinterpretation");
    assert_eq!(c, b"00000080030000000000000000000040\n".to_vec());
}

fn e7_unsigned_0xffffffff_all_bits() {
    let c = c_out_u(0xFFFF_FFFF);
    let r = rust_out_u(0xFFFF_FFFF);
    assert_eq!(c, r, "[E7] divergence for unsigned 0xffffffff");
    assert_eq!(c, c_out(-1), "[E7] must match -1 reinterpretation");
    assert_eq!(c, b"ffffffff030000000000000000000040\n".to_vec());
}

// ---- E8: out-of-range "enum-like" ints with no valid variant ------------

fn e8_out_of_range_enum_like_values() {
    // A C `int` parameter accepts any int, so a value with no meaningful
    // "variant" is a real input. Neither side may validate or diverge.
    let xs: Vec<i32> = vec![
        -2,
        4,
        5,
        999_999,
        -999_999,
        i32::MAX - 1,
        i32::MIN + 1,
        0xDEAD_BEEFu32 as i32,
        0xCAFE_BABEu32 as i32,
        0x7FFF_FFFE,
        i32::MAX,
        i32::MIN,
    ];
    for x in xs {
        assert_same(x, "E8");
    }
    // Also sweep the immediate neighbourhood of every "valid-looking" small
    // value, one step past in both directions.
    for x in -8..=8 {
        assert_same(x, "E8");
    }
}

// ---- E9: aliasing with the hard-coded constant --------------------------

fn e9_value_equal_to_bedrooms_constant() {
    assert_same(3, "E9");
    assert_eq!(
        c_out(3),
        b"03000000030000000000000000000040\n".to_vec(),
        "[E9] bedrooms must stay 3 when floors == 3"
    );
}

// ---- E10..E11: print_hex edge conditions are unreachable via the ABI ----

fn e10_print_hex_len_le_zero_is_unreachable() {
    // `driver` always passes sizeof(house_t); print_hex is `static` and not
    // exported, so no external caller can supply len <= 0.
    assert!(
        !c_has_symbol(b"print_hex"),
        "[E10] print_hex must not be reachable through the C ABI"
    );
    assert!(
        !rust_has_symbol(b"print_hex"),
        "[E10] print_hex must not be reachable through the Rust ABI"
    );
    // Proof that the length is a fixed 16: 32 hex digits + newline, always.
    for x in [0, 1, -1, i32::MIN, i32::MAX] {
        assert_eq!(c_out(x).len(), 33, "[E10] len is always sizeof(house_t)=16");
        assert_eq!(rust_out(x).len(), 33, "[E10] Rust must match");
    }
}

fn e11_print_hex_null_pointer_is_unreachable() {
    assert!(
        !c_has_symbol(b"print_hex"),
        "[E11] no FFI path exists to pass NULL to print_hex"
    );
    assert!(!rust_has_symbol(b"print_hex"), "[E11] same for Rust");
}

// ---- E12..E13: no pointer / length parameters exist --------------------

fn e12_no_pointer_parameter_to_nullify() {
    // The header declares `void driver(int x)`. Assert that mechanically so
    // the row is grounded in the C source, not in an assumption.
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("read c_src/include/driver.h");
    assert!(
        hdr.contains("void driver(int x);"),
        "[E12] public API changed; ERRORS.md must be regenerated"
    );
    assert!(
        !hdr.contains('*'),
        "[E12] no pointer parameter exists in the public API"
    );
}

fn e13_no_length_parameter_to_oversize() {
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("read c_src/include/driver.h");
    // Exactly one declaration, taking exactly one `int`.
    let decls: Vec<&str> = hdr
        .lines()
        .filter(|l| l.contains("driver(") && l.trim_end().ends_with(';'))
        .collect();
    assert_eq!(decls.len(), 1, "[E13] expected exactly one public function");
    assert_eq!(
        decls[0].matches(',').count(),
        0,
        "[E13] driver takes a single parameter; there is no length argument"
    );
}

// --- sequential runner entry point (harness = false) ---------------------

fn main() {
    common::run_suite(
        "error_paths",
        &[
        ("e1_library_has_no_rejection_branch", e1_library_has_no_rejection_branch as fn()),
        ("e2_zero_is_not_an_error", e2_zero_is_not_an_error as fn()),
        ("e3_int_max_is_not_an_error", e3_int_max_is_not_an_error as fn()),
        ("e4_int_min_is_not_an_error", e4_int_min_is_not_an_error as fn()),
        ("e5_minus_one_sentinel_is_not_an_error", e5_minus_one_sentinel_is_not_an_error as fn()),
        ("e6_unsigned_0x80000000_past_int_max", e6_unsigned_0x80000000_past_int_max as fn()),
        ("e7_unsigned_0xffffffff_all_bits", e7_unsigned_0xffffffff_all_bits as fn()),
        ("e8_out_of_range_enum_like_values", e8_out_of_range_enum_like_values as fn()),
        ("e9_value_equal_to_bedrooms_constant", e9_value_equal_to_bedrooms_constant as fn()),
        ("e10_print_hex_len_le_zero_is_unreachable", e10_print_hex_len_le_zero_is_unreachable as fn()),
        ("e11_print_hex_null_pointer_is_unreachable", e11_print_hex_null_pointer_is_unreachable as fn()),
        ("e12_no_pointer_parameter_to_nullify", e12_no_pointer_parameter_to_nullify as fn()),
        ("e13_no_length_parameter_to_oversize", e13_no_length_parameter_to_oversize as fn()),
        ],
    );
}
