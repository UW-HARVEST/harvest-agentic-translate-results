use std::os::raw::c_uint;

pub type tflac_u32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    18u32
        .wrapping_add(channels)
        .wrapping_add(
            ((blocksize
                .wrapping_mul(bitdepth)
                .wrapping_mul(channels.wrapping_mul((channels != 2) as c_uint)))
                .wrapping_add(blocksize.wrapping_mul(bitdepth).wrapping_mul((channels == 2) as c_uint))
                .wrapping_add(
                    blocksize
                        .wrapping_mul(bitdepth.wrapping_add((bitdepth != 32) as c_uint))
                        .wrapping_mul((channels == 2) as c_uint),
                )
                .wrapping_add(7))
                / 8,
        )
}
