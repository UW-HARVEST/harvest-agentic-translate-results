#![allow(unused_imports)]
use c_aces::error::{AcesError, Result};

#[test]
fn test_aces_error_generic() {
    let e = AcesError::GenericError("oops".to_string());
    let msg = format!("{:?}", e);
    assert!(msg.contains("GenericError"));
    assert!(msg.contains("oops"));
}

#[test]
fn test_result_alias_ok() {
    fn good() -> Result<i32> {
        Ok(42)
    }
    let r = good();
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 42);
}

#[test]
fn test_result_alias_err() {
    fn bad() -> Result<i32> {
        Err(AcesError::GenericError("nope".to_string()))
    }
    let r = bad();
    assert!(r.is_err());
    match r {
        Err(AcesError::GenericError(s)) => assert_eq!(s, "nope"),
        _ => panic!("expected GenericError"),
    }
}

fn main() {}
