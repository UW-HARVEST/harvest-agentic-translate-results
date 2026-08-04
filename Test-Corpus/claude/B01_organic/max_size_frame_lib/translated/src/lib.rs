use std::ffi::c_uint;

type TflacU32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(
    blocksize: TflacU32,
    channels: TflacU32,
    bitdepth: TflacU32,
) -> TflacU32 {
    let ne2: TflacU32 = if channels != 2 { 1 } else { 0 };
    let eq2: TflacU32 = if channels == 2 { 1 } else { 0 };
    let bd_ne32: TflacU32 = if bitdepth != 32 { 1 } else { 0 };

    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(ne2));
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(eq2);
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bd_ne32))
        .wrapping_mul(eq2);

    let sum = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    18u32
        .wrapping_add(channels)
        .wrapping_add(sum / 8)
}

// Silence unused import warning if any
#[allow(dead_code)]
fn _unused(_x: c_uint) {}
