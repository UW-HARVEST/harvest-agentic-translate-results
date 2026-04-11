pub type __uint32_t = u32;
pub type uint32_t = u32;
pub type tflac_u32 = u32;
#[no_mangle]
pub extern "C" fn max_size_frame(
    mut blocksize: tflac_u32,
    mut channels: tflac_u32,
    mut bitdepth: tflac_u32,
) -> tflac_u32 {
    return (18 as tflac_u32).wrapping_add(channels).wrapping_add(
        blocksize
            .wrapping_mul(bitdepth)
            .wrapping_mul(
                channels
                    .wrapping_mul((channels != 2 as tflac_u32) as libc::c_int as tflac_u32),
            )
            .wrapping_add(
                blocksize
                    .wrapping_mul(bitdepth)
                    .wrapping_mul((channels == 2 as tflac_u32) as libc::c_int as tflac_u32),
            )
            .wrapping_add(
                blocksize
                    .wrapping_mul(bitdepth.wrapping_add(
                        (bitdepth != 32 as tflac_u32) as libc::c_int as tflac_u32,
                    ))
                    .wrapping_mul((channels == 2 as tflac_u32) as libc::c_int as tflac_u32),
            )
            .wrapping_add(7 as libc::c_int as tflac_u32)
            .wrapping_div(8 as tflac_u32),
    );
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

