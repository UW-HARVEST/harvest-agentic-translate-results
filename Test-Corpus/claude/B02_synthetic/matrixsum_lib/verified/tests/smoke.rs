mod common;
use common::*;

#[test]
fn smoke_harness_loads_both_and_matrix_symbol_is_writable() {
    let p = load_pair();
    with_matrix_lock(&p, || {
        // The data symbol must point at the documented default contents.
        assert_eq!(p.c.get_matrix(), MATRIX_DEFAULT, "C matrix defaults");
        assert_eq!(p.r.get_matrix(), MATRIX_DEFAULT, "Rust matrix defaults");

        let cs = unsafe { (p.c.calculate_matrix_checksum)() };
        let rs = unsafe { (p.r.calculate_matrix_checksum)() };
        assert_eq!(cs, rs, "default checksum");

        // Writing through the exported symbol must change the checksum.
        let mutated = [1000; MATRIX_LEN];
        p.set_matrices(&mutated);
        assert_eq!(p.c.get_matrix(), mutated);
        assert_eq!(p.r.get_matrix(), mutated);
        let cs2 = unsafe { (p.c.calculate_matrix_checksum)() };
        let rs2 = unsafe { (p.r.calculate_matrix_checksum)() };
        assert_eq!(cs2, 12_000, "checksum must read the live symbol");
        assert_eq!(cs2, rs2);

        let cm = unsafe { (p.c.matrixsum)(1, 2, 3, 4) };
        let rm = unsafe { (p.r.matrixsum)(1, 2, 3, 4) };
        assert_eq!(cm, rm, "matrixsum(1,2,3,4)");
    });
}
