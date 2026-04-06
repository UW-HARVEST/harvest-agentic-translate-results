pub type __uint32_t = u32;
pub type uint32_t = __uint32_t;
pub type tflac_u32 = uint32_t;
#[no_mangle]
pub unsafe extern "C" fn max_size_frame(
    mut blocksize: tflac_u32,
    mut channels: tflac_u32,
    mut bitdepth: tflac_u32,
) -> tflac_u32 {
    return (18 as tflac_u32).wrapping_add(channels).wrapping_add(
        blocksize
            .wrapping_mul(bitdepth)
            .wrapping_mul(
                channels
                    .wrapping_mul((channels != 2 as tflac_u32) as ::core::ffi::c_int as tflac_u32),
            )
            .wrapping_add(
                blocksize
                    .wrapping_mul(bitdepth)
                    .wrapping_mul((channels == 2 as tflac_u32) as ::core::ffi::c_int as tflac_u32),
            )
            .wrapping_add(
                blocksize
                    .wrapping_mul(bitdepth.wrapping_add(
                        (bitdepth != 32 as tflac_u32) as ::core::ffi::c_int as tflac_u32,
                    ))
                    .wrapping_mul((channels == 2 as tflac_u32) as ::core::ffi::c_int as tflac_u32),
            )
            .wrapping_add(7 as ::core::ffi::c_int as tflac_u32)
            .wrapping_div(8 as tflac_u32),
    );
}
