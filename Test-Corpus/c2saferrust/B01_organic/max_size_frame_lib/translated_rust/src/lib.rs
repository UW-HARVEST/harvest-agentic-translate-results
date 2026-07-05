
pub type __uint32_t = u32;
pub type uint32_t = __uint32_t;
pub type tflac_u32 = uint32_t;
#[no_mangle]
pub fn max_size_frame(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    let bits = if channels == 2 {
        let left_right = blocksize.wrapping_mul(bitdepth);
        let side = blocksize.wrapping_mul(bitdepth.wrapping_add((bitdepth != 32) as tflac_u32));
        left_right.wrapping_add(side)
    } else {
        blocksize
            .wrapping_mul(bitdepth)
            .wrapping_mul(channels)
    };

    18u32
        .wrapping_add(channels)
        .wrapping_add(bits.wrapping_add(7).wrapping_div(8))
}

