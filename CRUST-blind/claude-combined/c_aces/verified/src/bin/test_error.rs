use c_aces::error::{AcesError, Result};

#[test]
fn test_aces_error_generic() {
    let err = AcesError::GenericError("hello".to_string());
    match err {
        AcesError::GenericError(s) => assert_eq!(s, "hello"),
    }
}

#[test]
fn test_result_ok() {
    let r: Result<i32> = Ok(42);
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 42);
}

#[test]
fn test_result_err() {
    let r: Result<i32> = Err(AcesError::GenericError("oops".to_string()));
    assert!(r.is_err());
    match r {
        Err(AcesError::GenericError(s)) => assert_eq!(s, "oops"),
        _ => panic!("expected error"),
    }
}

fn main() {}
