//! Lowest level of the call graph: the leaf functions that depend on nothing
//! but their arguments and the exported `matrix` global.

mod common;

use common::{INT_PROBES, MatrixT, both, matrix_guard};
use std::ffi::c_int;

#[test]
fn process_flags_matches_on_probe_values() {
    let (c, rust) = both();
    for &v in INT_PROBES {
        let got_c = unsafe { (c.process_flags)(v) };
        let got_rust = unsafe { (rust.process_flags)(v) };
        assert_eq!(got_c, got_rust, "process_flags({v})");
    }
}

/// All 2^8 low-byte patterns cover every combination of the four flag bits plus
/// the four bits the function must ignore.
#[test]
fn process_flags_matches_on_all_low_bytes() {
    let (c, rust) = both();
    for v in 0..=0xFFi32 {
        assert_eq!(
            unsafe { (c.process_flags)(v) },
            unsafe { (rust.process_flags)(v) },
            "process_flags({v:#04x})"
        );
    }
}

/// Sweeps the whole 16-bit space and the same patterns shifted into the high
/// bits / negated, to be sure no upper bit leaks into the result.
#[test]
fn process_flags_matches_on_wide_sweep() {
    let (c, rust) = both();
    for base in 0..=0xFFFFi32 {
        for v in [base, -base, base << 16, base | c_int::MIN] {
            assert_eq!(
                unsafe { (c.process_flags)(v) },
                unsafe { (rust.process_flags)(v) },
                "process_flags({v})"
            );
        }
    }
}

#[test]
fn calculate_matrix_checksum_matches_with_initial_matrix() {
    let _guard = matrix_guard();
    let (c, rust) = both();

    // The statically initialised matrix must be identical to begin with.
    assert_eq!(
        c.read_matrix(),
        rust.read_matrix(),
        "initial `matrix` global differs"
    );

    assert_eq!(unsafe { (c.calculate_matrix_checksum)() }, unsafe {
        (rust.calculate_matrix_checksum)()
    });
}

/// `matrix` is a mutable global that both libraries export, so writing through
/// the exported symbol must change the checksum identically on both sides.
#[test]
fn calculate_matrix_checksum_matches_after_mutating_the_global() {
    let _guard = matrix_guard();
    let (c, rust) = both();
    let saved_c = c.read_matrix();
    let saved_rust = rust.read_matrix();

    let cases: &[MatrixT] = &[
        [[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 1, 1], [1, 1, 1, 1], [1, 1, 1, 1]],
        [[-1, -1, -1, -1], [-1, -1, -1, -1], [-1, -1, -1, -1]],
        [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]],
        // Positive overflow of the running `int` sum.
        [
            [c_int::MAX, c_int::MAX, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, c_int::MAX],
        ],
        // Negative overflow.
        [
            [c_int::MIN, c_int::MIN, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, c_int::MIN],
        ],
        // Mixed extremes in every slot.
        [
            [c_int::MAX, c_int::MIN, c_int::MAX, c_int::MIN],
            [c_int::MIN, c_int::MAX, c_int::MIN, c_int::MAX],
            [1, -1, c_int::MAX, c_int::MIN],
        ],
        // Distinct powers of two, so a wrong traversal order or a dropped cell
        // would show up in the sum.
        [
            [1, 2, 4, 8],
            [16, 32, 64, 128],
            [256, 512, 1024, 1 << 30],
        ],
    ];

    for m in cases {
        c.write_matrix(m);
        rust.write_matrix(m);
        assert_eq!(c.read_matrix(), rust.read_matrix(), "matrix write-back");
        assert_eq!(
            unsafe { (c.calculate_matrix_checksum)() },
            unsafe { (rust.calculate_matrix_checksum)() },
            "calculate_matrix_checksum with matrix {m:?}"
        );
    }

    c.write_matrix(&saved_c);
    rust.write_matrix(&saved_rust);
}

/// The checksum only reads the 3x4 window; poke one cell at a time to confirm
/// both implementations visit exactly the same twelve elements.
#[test]
fn calculate_matrix_checksum_visits_every_cell() {
    let _guard = matrix_guard();
    let (c, rust) = both();
    let saved = c.read_matrix();
    assert_eq!(saved, rust.read_matrix());

    for i in 0..3 {
        for j in 0..4 {
            let mut m = [[0i32; 4]; 3];
            m[i][j] = 0x1234;
            c.write_matrix(&m);
            rust.write_matrix(&m);
            let got_c = unsafe { (c.calculate_matrix_checksum)() };
            let got_rust = unsafe { (rust.calculate_matrix_checksum)() };
            assert_eq!(got_c, got_rust, "checksum with only [{i}][{j}] set");
            assert_eq!(got_c, 0x1234, "checksum should read matrix[{i}][{j}]");
        }
    }

    c.write_matrix(&saved);
    rust.write_matrix(&saved);
}
