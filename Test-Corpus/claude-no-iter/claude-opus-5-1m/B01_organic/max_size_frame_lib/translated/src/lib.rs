use std::ffi::c_uint;

pub type TflacU32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(
    blocksize: TflacU32,
    channels: TflacU32,
    bitdepth: TflacU32,
) -> TflacU32 {
    let ch_ne_2: TflacU32 = (channels != 2) as c_uint as TflacU32;
    let ch_eq_2: TflacU32 = (channels == 2) as c_uint as TflacU32;
    let bd_ne_32: TflacU32 = (bitdepth != 32) as c_uint as TflacU32;

    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(ch_ne_2));
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(ch_eq_2);
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bd_ne_32))
        .wrapping_mul(ch_eq_2);

    let inner = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    18u32
        .wrapping_add(channels)
        .wrapping_add(inner / 8)
}
