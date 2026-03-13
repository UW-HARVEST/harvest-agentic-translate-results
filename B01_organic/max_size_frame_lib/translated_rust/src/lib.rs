type TflacU32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: TflacU32, channels: TflacU32, bitdepth: TflacU32) -> TflacU32 {
    let ch_ne2 = (channels != 2) as u32;
    let ch_eq2 = (channels == 2) as u32;
    let bd_ne32 = (bitdepth != 32) as u32;

    18u32
        .wrapping_add(channels)
        .wrapping_add(
            blocksize
                .wrapping_mul(bitdepth)
                .wrapping_mul(channels.wrapping_mul(ch_ne2))
                .wrapping_add(blocksize.wrapping_mul(bitdepth).wrapping_mul(ch_eq2))
                .wrapping_add(
                    blocksize
                        .wrapping_mul(bitdepth.wrapping_add(bd_ne32))
                        .wrapping_mul(ch_eq2),
                )
                .wrapping_add(7)
                / 8,
        )
}
