use std::os::raw::c_int;

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;
pub type tflac_uint = tflac_u64;

#[repr(C)]
pub struct tflac_bitwriter {
    pub val: tflac_uint,
    pub bits: tflac_u32,
    pub pos: tflac_u32,
    pub len: tflac_u32,
    pub tot: tflac_u32,
    pub buffer: *mut tflac_u8,
}

#[unsafe(no_mangle)]
pub extern "C" fn bitwriter_add(bw: *mut tflac_bitwriter, mut bits: tflac_u32, mut val: tflac_uint) -> c_int {
    let mask: tflac_uint = 18446744073709551615u64 << 1;
    let mut b: tflac_u32;
    
    let uint_bits = (8 * std::mem::size_of::<tflac_uint>()) as u32;
    let shift = uint_bits.wrapping_sub(bits);
    val = val.checked_shl(shift).unwrap_or(0);
    
    unsafe {
        (*bw).tot = (*bw).tot.wrapping_add(bits);
        let mut i = 0;
        while ((*bw).bits.wrapping_add(bits) >= uint_bits) && i < 100 {
            b = uint_bits.wrapping_sub((*bw).bits).wrapping_sub(1);
            b = if b > bits { bits } else { b };
            
            (*bw).val |= val.checked_shr((*bw).bits).unwrap_or(0);
            (*bw).bits = (*bw).bits.wrapping_add(b);
            (*bw).val &= mask;
            val = val.checked_shl(b).unwrap_or(0);
            bits = bits.wrapping_sub(b);
            i += 1;
        }
        (*bw).val |= val.checked_shr((*bw).bits).unwrap_or(0);
        (*bw).bits = (*bw).bits.wrapping_add(bits);
    }
    0
}
