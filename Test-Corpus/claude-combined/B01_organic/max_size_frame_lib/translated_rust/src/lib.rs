use std::ffi::c_uint;

#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: c_uint, channels: c_uint, bitdepth: c_uint) -> c_uint {
    // Replicate the exact C arithmetic, including unsigned wraparound semantics.
    let ne2: c_uint = if channels != 2 { 1 } else { 0 };
    let eq2: c_uint = if channels == 2 { 1 } else { 0 };
    let bd_ne_32: c_uint = if bitdepth != 32 { 1 } else { 0 };

    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(ne2));
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(eq2);
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bd_ne_32))
        .wrapping_mul(eq2);

    let sum = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    18u32
        .wrapping_add(channels)
        .wrapping_add(sum / 8)
}
