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

fn main() {}
