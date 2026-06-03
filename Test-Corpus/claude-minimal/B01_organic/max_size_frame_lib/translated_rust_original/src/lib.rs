#![allow(non_camel_case_types)]

pub type tflac_u32 = u32;

#[no_mangle]
pub extern "C" fn max_size_frame(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    let channels_neq_2: tflac_u32 = if channels != 2 { 1 } else { 0 };
    let channels_eq_2: tflac_u32 = if channels == 2 { 1 } else { 0 };
    let bitdepth_neq_32: tflac_u32 = if bitdepth != 32 { 1 } else { 0 };

    18u32
        + channels
        + (((blocksize * bitdepth * (channels * channels_neq_2))
            + (blocksize * bitdepth * channels_eq_2)
            + (blocksize * (bitdepth + bitdepth_neq_32) * channels_eq_2)
            + 7)
            / 8)
}
