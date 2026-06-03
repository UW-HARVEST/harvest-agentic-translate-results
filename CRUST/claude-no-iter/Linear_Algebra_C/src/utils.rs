/// Returns the exclusive-or of two booleans.
pub fn exclusive_or(a: bool, b: bool) -> bool {
    (a || b) && !(a && b)
}

/// Rounds the given value to n digits.
pub fn roundn(val: f64, n: u32) -> f64 {
    assert!(n > 0);
    let x = 10f64.powi(n as i32);
    (val * x).round() / x
}

/// A basic custom assert function.
/// (In Rust one would normally use the built-in assert! macro.)
pub fn custom_assert(condition: i32) {
    if condition == 0 {
        println!("Assertion failed.");
        print_call_stack();
        std::process::exit(1);
    }
}

/// Prints the call stack up until an assertion failure.
pub fn print_call_stack() {
    println!("----------------------------- STACK TRACE ------------------------------");
    println!("(call stack printing not implemented in pure Rust)");
    println!("------------------------------------------------------------------------");
}
