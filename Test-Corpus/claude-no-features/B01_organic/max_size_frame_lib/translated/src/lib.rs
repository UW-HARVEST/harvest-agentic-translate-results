use std::ffi::c_uint;

pub type TflacU32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: TflacU32, channels: TflacU32, bitdepth: TflacU32) -> TflacU32 {
    // Reproduce the C expression exactly using wrapping arithmetic to match
    // C's unsigned overflow semantics.
    let channels_ne_2: TflacU32 = (channels != 2) as TflacU32;
    let channels_eq_2: TflacU32 = (channels == 2) as TflacU32;
    let bitdepth_ne_32: TflacU32 = (bitdepth != 32) as TflacU32;

    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(channels_ne_2));
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(channels_eq_2);
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bitdepth_ne_32))
        .wrapping_mul(channels_eq_2);

    let sum = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    18u32
        .wrapping_add(channels)
        .wrapping_add(sum / 8)
}

// Reference c_uint to avoid unused import warning if needed; keep types matching
// the C API exactly.
#[allow(dead_code)]
const _: fn() = || {
    let _: c_uint = 0;
};
