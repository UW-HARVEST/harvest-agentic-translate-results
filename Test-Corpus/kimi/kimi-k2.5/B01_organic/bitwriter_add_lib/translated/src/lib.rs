use std::os::raw::{c_int, c_uint, c_ulonglong};

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
pub extern "C" fn bitwriter_add(bw: *mut tflac_bitwriter, bits: tflac_u32, val: tflac_uint) -> c_int {
    const MASK: tflac_uint = tflac_uint::MAX << 1;
    let mut bits = bits;
    let mut val = val;
    let mut i = 0;
    
    unsafe {
        val <<= ((8 * std::mem::size_of::<tflac_uint>()) as tflac_u32 - bits) as u32;
        (*bw).tot += bits;
        
        while ((*bw).bits + bits >= (8 * std::mem::size_of::<tflac_uint>()) as tflac_u32) && i < 100 {
            let mut b = (8 * std::mem::size_of::<tflac_uint>()) as tflac_u32 - (*bw).bits - 1;
            if b > bits {
                b = bits;
            }
            (*bw).val |= val >> (*bw).bits;
            (*bw).bits += b;
            (*bw).val &= MASK;
            val <<= b;
            bits -= b;
            i += 1;
        }
        
        (*bw).val |= val >> (*bw).bits;
        (*bw).bits += bits;
    }
    
    0
}
