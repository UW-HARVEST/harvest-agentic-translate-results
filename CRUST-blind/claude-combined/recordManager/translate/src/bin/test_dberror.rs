use recordManager::dberror::{self, RC, PAGE_SIZE};

#[test]
fn test_page_size_constant() {
    assert_eq!(PAGE_SIZE, 4096);
}

#[test]
fn test_throw_returns_int_code() {
    let code = dberror::throw(RC::FileNotFound, "test message");
    assert_eq!(code, 1);
    let code = dberror::throw(RC::WriteFailed, "test");
    assert_eq!(code, 3);
    let code = dberror::throw(RC::Ok, "ok");
    assert_eq!(code, 0);
}

#[test]
fn test_error_message_with_message() {
    dberror::throw(RC::FileNotFound, "file missing");
    let msg = dberror::error_message(RC::FileNotFound);
    assert_eq!(msg, "EC (1), \"file missing\"\n");
}

#[test]
fn test_rc_codes() {
    assert_eq!(RC::Ok as i32, 0);
    assert_eq!(RC::FileNotFound as i32, 1);
    assert_eq!(RC::FileHandleNotInit as i32, 2);
    assert_eq!(RC::WriteFailed as i32, 3);
    assert_eq!(RC::ReadNonExistingPage as i32, 4);
    assert_eq!(RC::RmNoMoreTuples as i32, 203);
    assert_eq!(RC::MemoryAllocationFail as i32, 401);
    assert_eq!(RC::RecordNotFound as i32, 410);
}

fn main() {}
