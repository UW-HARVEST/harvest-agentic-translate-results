#![allow(non_camel_case_types)]

use std::ffi::c_int;

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
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut tflac_bitwriter,
    mut bits: tflac_u32,
    mut val: tflac_uint,
) -> c_int {
    const TFLAC_UINT_BITS: tflac_u32 = (8 * std::mem::size_of::<tflac_uint>()) as tflac_u32;

    let mask = tflac_uint::MAX.wrapping_shl(1);
    let bw = unsafe { &mut *bw };
    let mut i: c_int = 0;

    val = val.wrapping_shl(TFLAC_UINT_BITS.wrapping_sub(bits));
    bw.tot = bw.tot.wrapping_add(bits);

    while bw.bits.wrapping_add(bits) >= TFLAC_UINT_BITS && i < 100 {
        let mut b = TFLAC_UINT_BITS.wrapping_sub(bw.bits).wrapping_sub(1);
        b = if b > bits { bits } else { b };
        bw.val |= val.wrapping_shr(bw.bits);
        bw.bits = bw.bits.wrapping_add(b);
        bw.val &= mask;
        val = val.wrapping_shl(b);
        bits = bits.wrapping_sub(b);
        i += 1;
    }

    bw.val |= val.wrapping_shr(bw.bits);
    bw.bits = bw.bits.wrapping_add(bits);
    0
}
