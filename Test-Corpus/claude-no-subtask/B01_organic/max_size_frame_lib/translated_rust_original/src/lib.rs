use std::ffi::c_uint;

pub type TflacU32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(blocksize: TflacU32, channels: TflacU32, bitdepth: TflacU32) -> TflacU32 {
    let neq2: u32 = if channels != 2 { 1 } else { 0 };
    let eq2: u32 = if channels == 2 { 1 } else { 0 };
    let bd_neq32: u32 = if bitdepth != 32 { 1 } else { 0 };

    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(neq2));
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(eq2);
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bd_neq32))
        .wrapping_mul(eq2);

    let sum = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    18u32
        .wrapping_add(channels)
        .wrapping_add(sum / 8)
}

// Suppress unused import warning if any
#[allow(dead_code)]
fn _unused(_: c_uint) {}
