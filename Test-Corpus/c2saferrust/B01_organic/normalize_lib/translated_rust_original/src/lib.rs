
extern "C" {
    fn sqrtf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
#[no_mangle]
pub fn normalize(dest: &mut [f32], src: &[f32], size: i32) {
    let size = size.max(0) as usize;
    let len = size.min(src.len()).min(dest.len());

    let mut sum = 0.0f32;
    for &x in &src[..len] {
        sum += x * x;
    }

    if sum > 0.0 {
        let scale = 1.0f32 / sum.sqrt();
        for (d, &s) in dest[..len].iter_mut().zip(&src[..len]) {
            *d = s * scale;
        }
    } else if !core::ptr::eq(dest.as_ptr(), src.as_ptr()) {
        dest[..len].fill(0.0);
    }
}

