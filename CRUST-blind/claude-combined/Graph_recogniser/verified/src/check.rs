#[cfg(debug_assertions)]
macro_rules! check {
    ($x:expr) => {
        assert!($x);
    };
}
#[cfg(not(debug_assertions))]
macro_rules! check {
    ($x:expr) => {
        (); // No-op in release mode
    };
}
#[cfg(debug_assertions)]
macro_rules! debugin {
    ($stm:stmt) => {
        $stm
    };
}
#[cfg(not(debug_assertions))]
macro_rules! debugin {
    ($stm:stmt) => {
        (); // No-op in release mode
    };
}

/// Function form of `check` macro for use in functions where macro use is awkward.
#[cfg(debug_assertions)]
pub fn check(cond: bool) {
    assert!(cond);
}

#[cfg(not(debug_assertions))]
pub fn check(_cond: bool) {
    // No-op in release mode
}
