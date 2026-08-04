// Translation of c_src/src/lib.c to Rust.
//
// The original C code is a shared library that exposes a single function
// `tfm`. There is no `main` in the C sources, so this executable's `main`
// performs no I/O — matching the behavior of running the original C
// translation unit (which produces no output for any input).

/// Equivalent of the C function:
///
/// ```c
/// void tfm(float *dest, const float *src, int count);
/// ```
///
/// `src` is read in groups of 3 floats and `dest` is written in groups of
/// 2 floats. The caller must ensure that `src` has at least `3 * count`
/// elements and `dest` has at least `2 * count` elements.
pub fn tfm(dest: &mut [f32], src: &[f32], count: i32) {
    let count = count as usize;
    for i in 0..count {
        let s = &src[i * 3..i * 3 + 3];
        let d = &mut dest[i * 2..i * 2 + 2];
        if s[0] < s[1] {
            let dx2 = s[0];
            let dy2 = s[1];
            let dxy = s[2];
            let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2)
                + (4.0f32 * dxy * dxy);
            // Mirror C's `((0) > (sqd)) ? (0) : (sqd)`: if 0 > sqd return 0, else sqd.
            let clamped = if 0.0f32 > sqd { 0.0f32 } else { sqd };
            let lambda = 0.5f32 * (dy2 + dx2 + clamped.sqrt());
            d[0] = dx2 - lambda;
            d[1] = dxy;
        } else {
            let dy2 = s[0];
            let dx2 = s[1];
            let dxy = s[2];
            let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2)
                + (4.0f32 * dxy * dxy);
            let clamped = if 0.0f32 > sqd { 0.0f32 } else { sqd };
            let lambda = 0.5f32 * (dy2 + dx2 + clamped.sqrt());
            d[0] = dxy;
            d[1] = dx2 - lambda;
        }
    }
}

fn main() {
    // The C source defines no `main`, so this binary intentionally
    // performs no I/O — preserving byte-identical (empty) output for any
    // input that would be supplied to a hypothetical executable built
    // from the original library.
}
