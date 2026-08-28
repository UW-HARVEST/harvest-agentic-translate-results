#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let is_stereo = u32::from(channels == 2);
    let is_not_stereo = u32::from(channels != 2);
    let bitdepth_is_not_32 = u32::from(bitdepth != 32);

    let non_stereo_size = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(is_not_stereo));
    let stereo_channel_size = blocksize.wrapping_mul(bitdepth).wrapping_mul(is_stereo);
    let stereo_side_size = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bitdepth_is_not_32))
        .wrapping_mul(is_stereo);

    let bytes = non_stereo_size
        .wrapping_add(stereo_channel_size)
        .wrapping_add(stereo_side_size)
        .wrapping_add(7)
        / 8;

    18u32.wrapping_add(channels).wrapping_add(bytes)
}
