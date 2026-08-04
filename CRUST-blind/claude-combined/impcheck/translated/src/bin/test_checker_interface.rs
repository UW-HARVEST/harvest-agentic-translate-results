use impcheck::checker_interface::*;

#[test]
fn test_init_constant() {
    assert_eq!(TRUSTED_CHK_INIT, 'B');
}

#[test]
fn test_load_constant() {
    assert_eq!(TRUSTED_CHK_LOAD, 'L');
}

#[test]
fn test_end_load_constant() {
    assert_eq!(TRUSTED_CHK_END_LOAD, 'E');
}

#[test]
fn test_cls_produce_constant() {
    assert_eq!(TRUSTED_CHK_CLS_PRODUCE, 'a');
}

#[test]
fn test_cls_import_constant() {
    assert_eq!(TRUSTED_CHK_CLS_IMPORT, 'i');
}

#[test]
fn test_cls_delete_constant() {
    assert_eq!(TRUSTED_CHK_CLS_DELETE, 'd');
}

#[test]
fn test_validate_unsat_constant() {
    assert_eq!(TRUSTED_CHK_VALIDATE_UNSAT, 'V');
}

#[test]
fn test_validate_sat_constant() {
    assert_eq!(TRUSTED_CHK_VALIDATE_SAT, 'M');
}

#[test]
fn test_terminate_constant() {
    assert_eq!(TRUSTED_CHK_TERMINATE, 'T');
}

#[test]
fn test_res_accept_constant() {
    assert_eq!(TRUSTED_CHK_RES_ACCEPT, 'A');
}

#[test]
fn test_res_error_constant() {
    assert_eq!(TRUSTED_CHK_RES_ERROR, 'E');
}

#[test]
fn test_constants_are_distinct_directives() {
    // Top directives should be unique chars
    let directives = [
        TRUSTED_CHK_INIT,
        TRUSTED_CHK_LOAD,
        // Note: TRUSTED_CHK_END_LOAD == 'E' which is same as TRUSTED_CHK_RES_ERROR
        TRUSTED_CHK_CLS_PRODUCE,
        TRUSTED_CHK_CLS_IMPORT,
        TRUSTED_CHK_CLS_DELETE,
        TRUSTED_CHK_VALIDATE_UNSAT,
        TRUSTED_CHK_VALIDATE_SAT,
        TRUSTED_CHK_TERMINATE,
    ];
    for i in 0..directives.len() {
        for j in (i + 1)..directives.len() {
            assert_ne!(directives[i], directives[j]);
        }
    }
}

fn main() {}
