mod tables;

use tables::{EXPONENT, MANTISSA, OFFSET};

#[unsafe(no_mangle)]
pub extern "C" fn half2float(h: u16) -> f32 {
    let n: i32 = (h >> 10) as i32;
    let idx = ((h & 0x3ff) as u32).wrapping_add(OFFSET[n as usize] as u32) as usize;
    let num: u32 = MANTISSA[idx].wrapping_add(EXPONENT[n as usize]);
    f32::from_bits(num)
}
