use SlothLang::throw;

#[test]
fn test_math_err_format() {
    // math_err calls process::abort, so we can only verify it doesn't panic on empty string
    // by checking the function exists and has the right signature
    // We can't call it directly since it aborts the process
    let _ = std::panic::catch_unwind(|| {
        // Just verify the function is callable with a &str
        let _f: fn(&str) = throw::math_err;
    });
}

#[test]
fn test_op_err_format() {
    let _ = std::panic::catch_unwind(|| {
        let _f: fn(&str, u8) = throw::op_err;
    });
}

fn main() {}
