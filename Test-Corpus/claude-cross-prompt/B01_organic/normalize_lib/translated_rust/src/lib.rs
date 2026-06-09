// Translation of c_src/src/lib.c

/// Normalize a vector by its L2 norm.
///
/// Reproduces the exact behavior of the C function:
/// - Computes sum of squares of `src`.
/// - If sum > 0: writes `src[i] * (1 / sqrt(sum))` into `dest`.
/// - Else if `dest` and `src` point to different memory: zeroes `dest`.
/// - Otherwise (sum == 0 and dest aliases src): leaves `dest` unchanged.
pub fn normalize(dest: &mut [f32], src: &[f32], size: i32) {
    let n = size as usize;
    let mut sum: f32 = 0.0;
    for i in 0..n {
        sum += src[i] * src[i];
    }
    if sum > 0.0 {
        sum = 1.0 / sum.sqrt();
        for i in 0..n {
            dest[i] = src[i] * sum;
        }
    } else {
        // The C code only zeroes dest when dest != src (pointer comparison).
        // In safe Rust we cannot have dest and src be the same slice, so
        // this branch always zeroes dest here. However, to match the C
        // behavior more closely when the caller might pass aliased data,
        // we provide a separate `normalize_in_place` for the aliased case.
        for i in 0..n {
            dest[i] = 0.0;
        }
    }
}

/// In-place version: equivalent to calling C's `normalize(buf, buf, size)`.
/// Matches the C behavior where, when sum == 0 and dest == src, the buffer
/// is left untouched.
pub fn normalize_in_place(buf: &mut [f32], size: i32) {
    let n = size as usize;
    let mut sum: f32 = 0.0;
    for i in 0..n {
        sum += buf[i] * buf[i];
    }
    if sum > 0.0 {
        sum = 1.0 / sum.sqrt();
        for i in 0..n {
            buf[i] = buf[i] * sum;
        }
    }
    // else: dest == src, so memset is skipped (matches C).
}
