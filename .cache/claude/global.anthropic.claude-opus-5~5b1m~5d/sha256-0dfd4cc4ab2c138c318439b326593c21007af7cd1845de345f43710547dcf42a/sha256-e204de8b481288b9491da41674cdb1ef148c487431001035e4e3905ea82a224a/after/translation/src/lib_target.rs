// Translation of c_src/src/lib.c (the externally visible `target`, declared in
// include/api.h).  Note that a.c and b.c each define their *own* `static int
// target`, which shadow this one only inside those translation units; engine.c
// sees this global one.
//
// Copyright 2025 MIT Lincoln Laboratory  (see c_src for the full license text)

/// `int target(int code)` from lib.c
pub fn target(code: i32) -> i32 {
    if code < 0 {
        return 7;
    }
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
