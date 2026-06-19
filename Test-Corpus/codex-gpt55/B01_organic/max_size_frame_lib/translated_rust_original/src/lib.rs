#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let channels_not_two = u32::from(channels != 2);
    let channels_is_two = u32::from(channels == 2);
    let bitdepth_not_32 = u32::from(bitdepth != 32);

    let numerator = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(channels_not_two))
        .wrapping_add(
            blocksize
                .wrapping_mul(bitdepth)
                .wrapping_mul(channels_is_two),
        )
        .wrapping_add(
            blocksize
                .wrapping_mul(bitdepth.wrapping_add(bitdepth_not_32))
                .wrapping_mul(channels_is_two),
        )
        .wrapping_add(7);

    18u32
        .wrapping_add(channels)
        .wrapping_add(numerator / 8)
}
