use impcheck::checker_interface::*;

#[test]
fn test_checker_interface_constants() {
    assert_eq!(TRUSTED_CHK_INIT, 'B');
    assert_eq!(TRUSTED_CHK_LOAD, 'L');
    assert_eq!(TRUSTED_CHK_END_LOAD, 'E');
    assert_eq!(TRUSTED_CHK_CLS_PRODUCE, 'a');
    assert_eq!(TRUSTED_CHK_CLS_IMPORT, 'i');
    assert_eq!(TRUSTED_CHK_CLS_DELETE, 'd');
    assert_eq!(TRUSTED_CHK_VALIDATE_UNSAT, 'V');
    assert_eq!(TRUSTED_CHK_VALIDATE_SAT, 'M');
    assert_eq!(TRUSTED_CHK_TERMINATE, 'T');
    assert_eq!(TRUSTED_CHK_RES_ACCEPT, 'A');
    assert_eq!(TRUSTED_CHK_RES_ERROR, 'E');
}

#[test]
fn test_end_load_equals_res_error() {
    // Both are 'E' in the C code
    assert_eq!(TRUSTED_CHK_END_LOAD, TRUSTED_CHK_RES_ERROR);
}

fn main() {}
