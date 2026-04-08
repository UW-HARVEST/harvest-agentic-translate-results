use recordManager::dberror::{RC, PAGE_SIZE, throw, error_message, print_error};

#[test]
fn test_page_size() {
    assert_eq!(PAGE_SIZE, 4096);
}

#[test]
fn test_rc_values() {
    assert_eq!(RC::Ok as i32, 0);
    assert_eq!(RC::FileNotFound as i32, 1);
    assert_eq!(RC::FileHandleNotInit as i32, 2);
    assert_eq!(RC::WriteFailed as i32, 3);
    assert_eq!(RC::ReadNonExistingPage as i32, 4);
    assert_eq!(RC::RmCompareValueOfDifferentDatatype as i32, 200);
    assert_eq!(RC::RmExprResultIsNotBoolean as i32, 201);
    assert_eq!(RC::RmBooleanExprArgIsNotBoolean as i32, 202);
    assert_eq!(RC::RmNoMoreTuples as i32, 203);
    assert_eq!(RC::RmNoPrintForDatatype as i32, 204);
    assert_eq!(RC::RmUnknownDatatype as i32, 205);
    assert_eq!(RC::ImKeyNotFound as i32, 300);
    assert_eq!(RC::ImKeyAlreadyExists as i32, 301);
    assert_eq!(RC::ImNToLarge as i32, 302);
    assert_eq!(RC::ImNoMoreEntries as i32, 303);
    assert_eq!(RC::MemoryAllocationFail as i32, 401);
    assert_eq!(RC::BufferpoolInUse as i32, 402);
    assert_eq!(RC::CloseFailed as i32, 403);
    assert_eq!(RC::Error as i32, 404);
    assert_eq!(RC::BufferpoolFull as i32, 405);
    assert_eq!(RC::ReadFailed as i32, 406);
    assert_eq!(RC::InvalidHeader as i32, 407);
    assert_eq!(RC::SeekFailed as i32, 408);
    assert_eq!(RC::DestroyFailed as i32, 409);
    assert_eq!(RC::RecordNotFound as i32, 410);
    assert_eq!(RC::GeneralError as i32, 411);
    assert_eq!(RC::ShutdownWithoutInit as i32, 420);
    assert_eq!(RC::LoggingSetupFailure as i32, 430);
}

#[test]
fn test_throw() {
    let code = throw(RC::Error, "test error");
    assert_eq!(code, 404);
}

#[test]
fn test_error_message_with_msg() {
    throw(RC::RmCompareValueOfDifferentDatatype, "test error");
    let msg = error_message(RC::RmCompareValueOfDifferentDatatype);
    assert_eq!(msg, "EC (200), \"test error\"\n");
}

fn main() {}
