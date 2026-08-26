// Translation of C library code from c_src/src/lib.c and c_src/include/lib.h

#![allow(dead_code)]
#![allow(non_camel_case_types)]

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;
pub type tflac_uint = tflac_u64;

pub struct TflacBitwriter<'a> {
    pub val: tflac_uint,
    pub bits: tflac_u32,
    pub pos: tflac_u32,
    pub len: tflac_u32,
    pub tot: tflac_u32,
    pub buffer: &'a mut [tflac_u8],
}

/// Translation of `bitwriter_add` from c_src/src/lib.c.
///
/// Preserves the exact behavior of the original C code, including
/// the C semantics of integer shifts and the `i < 100` loop limit.
pub fn bitwriter_add(bw: &mut TflacBitwriter, mut bits: tflac_u32, mut val: tflac_uint) -> i32 {
    let mask: tflac_uint = 18446744073709551615u64.wrapping_shl(1);
    let uint_bits: tflac_u32 = (8 * core::mem::size_of::<tflac_uint>()) as tflac_u32;

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    // In C, shifting by the full width is undefined behavior, but we use
    // wrapping_shl to mirror the typical x86_64 behavior (mod width).
    val = val.wrapping_shl(uint_bits.wrapping_sub(bits));
    bw.tot = bw.tot.wrapping_add(bits);

    let mut i: i32 = 0;
    while (bw.bits + bits >= uint_bits) && i < 100 {
        let mut b: tflac_u32 = uint_bits - bw.bits - 1;
        b = if b > bits { bits } else { b };
        bw.val |= val.wrapping_shr(bw.bits);
        bw.bits += b;
        bw.val &= mask;
        val = val.wrapping_shl(b);
        bits -= b;
        i += 1;
    }
    bw.val |= val.wrapping_shr(bw.bits);
    bw.bits += bits;
    0
}
