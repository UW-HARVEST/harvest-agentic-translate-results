#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let ch_ne_2 = (channels != 2) as u32;
    let ch_eq_2 = (channels == 2) as u32;
    let bd_ne_32 = (bitdepth != 32) as u32;

    18u32.wrapping_add(channels).wrapping_add(
        (blocksize
            .wrapping_mul(bitdepth)
            .wrapping_mul(channels.wrapping_mul(ch_ne_2))
            .wrapping_add(blocksize.wrapping_mul(bitdepth).wrapping_mul(ch_eq_2))
            .wrapping_add(
                blocksize
                    .wrapping_mul(bitdepth.wrapping_add(bd_ne_32))
                    .wrapping_mul(ch_eq_2),
            )
            .wrapping_add(7))
            / 8,
    )
}
