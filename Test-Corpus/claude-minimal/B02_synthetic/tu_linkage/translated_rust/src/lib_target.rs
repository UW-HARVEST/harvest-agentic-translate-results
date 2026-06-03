// Translated from c_src/src/lib.c
// Provides the public `target` function used as the extension/default implementation.

pub fn target(code: i32) -> i32 {
    if code < 0 {
        return 7;
    }
    let m = code.rem_euclid(10);
    if m == 0 {
        return 0;
    }
    if m <= 3 {
        return 1;
    }
    if m <= 6 {
        return 2;
    }
    if m == 7 {
        return 3;
    }
    4
}
