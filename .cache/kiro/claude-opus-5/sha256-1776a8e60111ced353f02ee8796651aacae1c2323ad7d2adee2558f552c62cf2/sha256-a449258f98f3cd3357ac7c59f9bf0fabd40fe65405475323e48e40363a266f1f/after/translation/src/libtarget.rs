// Translation of c_src/src/lib.c
//
// This is the single *global* `target` symbol (declared in include/api.h).
// It is stateless. Note that `a.c` and `b.c` each declare their own
// `static int target(int)` which shadow this one *within those files only*;
// engine.c, which includes api.h, binds to this global one.

pub fn target(code: i32) -> i32 {
    if code < 0 {
        return 7;
    }
    // code >= 0, so C's `%` yields a value in 0..=9
    let m = code % 10;
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
