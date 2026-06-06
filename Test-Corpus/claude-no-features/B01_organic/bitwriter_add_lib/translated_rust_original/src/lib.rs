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
    // (18446744073709551615UL) << 1 == 0xFFFFFFFFFFFFFFFE
    let mask: tflac_uint = 18446744073709551615u64.wrapping_shl(1);
    let bw = &mut *bw;
    let mut b: tflac_u32;
    let uint_bits: tflac_u32 = (8 * core::mem::size_of::<tflac_uint>()) as tflac_u32;

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    val = val.wrapping_shl(uint_bits.wrapping_sub(bits));
    // bw->tot += bits;
    bw.tot = bw.tot.wrapping_add(bits);
    let mut i: c_int = 0;
    // while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100)
    while bw.bits.wrapping_add(bits) >= uint_bits && i < 100 {
        // b = (8 * sizeof(tflac_uint)) - bw->bits - 1;
        b = uint_bits.wrapping_sub(bw.bits).wrapping_sub(1);
        // b = b > bits ? bits : b;
        b = if b > bits { bits } else { b };
        // bw->val |= (val >> bw->bits);
        bw.val |= val.wrapping_shr(bw.bits);
        // bw->bits += b;
        bw.bits = bw.bits.wrapping_add(b);
        // bw->val &= mask;
        bw.val &= mask;
        // val <<= b;
        val = val.wrapping_shl(b);
        // bits -= b;
        bits = bits.wrapping_sub(b);
        i += 1;
    }
    // bw->val |= (val >> bw->bits);
    bw.val |= val.wrapping_shr(bw.bits);
    // bw->bits += bits;
    bw.bits = bw.bits.wrapping_add(bits);
    0
}
