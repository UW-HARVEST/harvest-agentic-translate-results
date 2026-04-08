use c_aces::error::AcesError;

#[test]
fn test_aces_error_generic() {
    let err = AcesError::GenericError("test error".to_string());
    let msg = format!("{:?}", err);
    assert!(msg.contains("test error"));
}

#[test]
fn test_result_ok() {
    let r: c_aces::error::Result<u64> = Ok(42);
    assert_eq!(r.unwrap(), 42);
}

#[test]
fn test_result_err() {
    let r: c_aces::error::Result<u64> = Err(AcesError::GenericError("fail".into()));
    assert!(r.is_err());
}

fn main() {}
