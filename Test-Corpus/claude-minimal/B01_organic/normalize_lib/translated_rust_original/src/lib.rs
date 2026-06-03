use std::os::raw::c_int;

/// Normalize a vector of `size` floats from `src` into `dest`.
///
/// # Safety
///
/// `src` and `dest` must each point to at least `size` valid `f32` elements.
/// They may alias / be the same pointer (matching the C semantics).
#[no_mangle]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    if size <= 0 {
        return;
    }
    let n = size as usize;

    let mut sum: f32 = 0.0;
    for i in 0..n {
        let v = *src.add(i);
        sum += v * v;
    }

    if sum > 0.0 {
        let inv = 1.0f32 / sum.sqrt();
        for i in 0..n {
            *dest.add(i) = *src.add(i) * inv;
        }
    } else if dest as *const f32 != src {
        std::ptr::write_bytes(dest, 0u8, n);
    }
}
