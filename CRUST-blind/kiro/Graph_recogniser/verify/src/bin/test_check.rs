use Graph_recogniser::check;
use Graph_recogniser::debugin;

#[test]
fn test_check_true() {
    check!(true);
    check!(1 == 1);
    check!(2 > 1);
}

#[test]
fn test_debugin_executes() {
    let mut x = 0;
    debugin!(x = 42);
    // In debug mode, debugin executes the statement
    #[cfg(debug_assertions)]
    assert_eq!(x, 42);
    // In release mode, debugin is a no-op
    #[cfg(not(debug_assertions))]
    assert_eq!(x, 0);
}

fn main() {}
