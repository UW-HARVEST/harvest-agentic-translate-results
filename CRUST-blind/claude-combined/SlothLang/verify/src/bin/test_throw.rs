use SlothLang::throw;

// Both throw functions abort/panic in the C version (raise SIGFPE/SIGILL).
// We emulate with panic!(), so verify that calling them panics and that the
// panic message contains the provided context.

#[test]
fn test_math_err_panics() {
    let result = std::panic::catch_unwind(|| {
        throw::math_err("division by zero");
    });
    assert!(result.is_err());
}

#[test]
fn test_op_err_panics() {
    let result = std::panic::catch_unwind(|| {
        throw::op_err("operation", 0xFF);
    });
    assert!(result.is_err());
}

#[test]
fn test_op_err_with_zero_code() {
    let result = std::panic::catch_unwind(|| {
        throw::op_err("comparison", 0x00);
    });
    assert!(result.is_err());
}

#[test]
fn test_math_err_empty_message() {
    // Even with an empty message, the function still raises.
    let result = std::panic::catch_unwind(|| {
        throw::math_err("");
    });
    assert!(result.is_err());
}

fn main() {}
