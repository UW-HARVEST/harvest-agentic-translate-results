// Phase B addendum -- the INITIAL contents of the exported `matrix` data symbol.
//
// !!! This file must contain EXACTLY ONE test. !!!
//
// `matrix` is a writable `.data` symbol shared by every test in a test binary,
// and the other suites deliberately overwrite it (via `with_matrix_lock`) to get
// deterministic inputs. That masks the symbol's *initializer*: a Rust `.data`
// initializer that disagrees with the C one would be invisible there, because
// both sides get overwritten with the same harness constant before comparison.
//
// Keeping this check alone in its own integration-test binary guarantees it
// observes both libraries in a pristine, never-written state.

mod common;
use common::*;
use std::ffi::c_int;

#[test]
fn initial_matrix_data_symbol_and_derived_results_match() {
    let p = load_pair();

    // 1. Raw initializer bytes, read before ANY test writes to the symbol.
    let c_init = p.c.get_matrix();
    let r_init = p.r.get_matrix();
    assert_eq!(
        c_init, r_init,
        "initial contents of the exported `matrix` .data symbol differ:\n  C    = {c_init:?}\n  RUST = {r_init:?}"
    );

    // 2. Cross-check against the literal transcription of the C definition, so a
    //    matching-but-wrong pair cannot slip through.
    assert_eq!(
        c_init, MATRIX_DEFAULT,
        "C `matrix` initializer no longer matches c_src/src/lib.c"
    );

    // 3. Everything derived from the pristine global.
    let cs = unsafe { (p.c.calculate_matrix_checksum)() };
    let rs = unsafe { (p.r.calculate_matrix_checksum)() };
    assert_eq!(cs, rs, "checksum over the pristine matrix");
    assert_eq!(
        cs,
        MATRIX_DEFAULT.iter().fold(0 as c_int, |a, &b| a.wrapping_add(b)),
        "checksum must equal the sum of the C initializer's 12 values"
    );

    for (a, b, c, d) in [
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, 0, 1, 0),
        (c_int::MAX, c_int::MIN, 1, -1),
    ] {
        assert_eq!(
            unsafe { (p.c.matrixsum)(a, b, c, d) },
            unsafe { (p.r.matrixsum)(a, b, c, d) },
            "matrixsum({a},{b},{c},{d}) over the pristine matrix"
        );
    }

    // 4. The symbol must still be pristine (nothing above wrote to it).
    assert_eq!(p.c.get_matrix(), MATRIX_DEFAULT);
    assert_eq!(p.r.get_matrix(), MATRIX_DEFAULT);
}
