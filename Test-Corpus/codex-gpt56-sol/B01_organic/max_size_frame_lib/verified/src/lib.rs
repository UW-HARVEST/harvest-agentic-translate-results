use std::ffi::c_uint;

#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: c_uint, channels: c_uint, bitdepth: c_uint) -> c_uint {
    let channels_eq_2 = c_uint::from(channels == 2);
    let channels_ne_2 = c_uint::from(channels != 2);
    let bitdepth_ne_32 = c_uint::from(bitdepth != 32);

    let first = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(channels_ne_2));
    let second = blocksize.wrapping_mul(bitdepth).wrapping_mul(channels_eq_2);
    let third = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bitdepth_ne_32))
        .wrapping_mul(channels_eq_2);

    let bytes = first
        .wrapping_add(second)
        .wrapping_add(third)
        .wrapping_add(7)
        / 8;

    18_u32.wrapping_add(channels).wrapping_add(bytes)
}
