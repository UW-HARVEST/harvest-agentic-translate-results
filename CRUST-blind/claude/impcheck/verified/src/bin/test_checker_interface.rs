use impcheck::checker_interface;

#[test]
fn test_checker_interface_constants() {
    // Verify exact char values match the C #defines.
    assert_eq!(checker_interface::TRUSTED_CHK_INIT, 'B');
    assert_eq!(checker_interface::TRUSTED_CHK_LOAD, 'L');
    assert_eq!(checker_interface::TRUSTED_CHK_END_LOAD, 'E');
    assert_eq!(checker_interface::TRUSTED_CHK_CLS_PRODUCE, 'a');
    assert_eq!(checker_interface::TRUSTED_CHK_CLS_IMPORT, 'i');
    assert_eq!(checker_interface::TRUSTED_CHK_CLS_DELETE, 'd');
    assert_eq!(checker_interface::TRUSTED_CHK_VALIDATE_UNSAT, 'V');
    assert_eq!(checker_interface::TRUSTED_CHK_VALIDATE_SAT, 'M');
    assert_eq!(checker_interface::TRUSTED_CHK_TERMINATE, 'T');
    assert_eq!(checker_interface::TRUSTED_CHK_RES_ACCEPT, 'A');
    assert_eq!(checker_interface::TRUSTED_CHK_RES_ERROR, 'E');
}

#[test]
fn test_checker_interface_byte_values() {
    // The C code uses these as 'char' values in fputc/fgetc, so byte values matter.
    assert_eq!(checker_interface::TRUSTED_CHK_INIT as u8, b'B');
    assert_eq!(checker_interface::TRUSTED_CHK_LOAD as u8, b'L');
    assert_eq!(checker_interface::TRUSTED_CHK_END_LOAD as u8, b'E');
    assert_eq!(checker_interface::TRUSTED_CHK_CLS_PRODUCE as u8, b'a');
    assert_eq!(checker_interface::TRUSTED_CHK_CLS_IMPORT as u8, b'i');
    assert_eq!(checker_interface::TRUSTED_CHK_CLS_DELETE as u8, b'd');
    assert_eq!(checker_interface::TRUSTED_CHK_VALIDATE_UNSAT as u8, b'V');
    assert_eq!(checker_interface::TRUSTED_CHK_VALIDATE_SAT as u8, b'M');
    assert_eq!(checker_interface::TRUSTED_CHK_TERMINATE as u8, b'T');
    assert_eq!(checker_interface::TRUSTED_CHK_RES_ACCEPT as u8, b'A');
    assert_eq!(checker_interface::TRUSTED_CHK_RES_ERROR as u8, b'E');
}

fn main() {}
