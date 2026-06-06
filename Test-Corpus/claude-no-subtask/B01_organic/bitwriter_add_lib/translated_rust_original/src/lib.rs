use std::ffi::c_int;

pub type TflacU8 = u8;
pub type TflacU32 = u32;
pub type TflacU64 = u64;
pub type TflacUint = TflacU64;

#[repr(C)]
pub struct TflacBitwriter {
    pub val: TflacUint,
    pub bits: TflacU32,
    pub pos: TflacU32,
    pub len: TflacU32,
    pub tot: TflacU32,
    pub buffer: *mut TflacU8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut TflacBitwriter,
    mut bits: TflacU32,
    mut val: TflacUint,
) -> c_int {
    let mask: TflacUint = 18446744073709551615u64.wrapping_shl(1);
    let bw = unsafe { &mut *bw };

    let uint_bits: TflacU32 = (8 * std::mem::size_of::<TflacUint>()) as TflacU32;

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    val = val.wrapping_shl(uint_bits.wrapping_sub(bits));
    bw.tot = bw.tot.wrapping_add(bits);

    let mut i: c_int = 0;
    while (bw.bits.wrapping_add(bits) >= uint_bits) && i < 100 {
        let mut b: TflacU32 = uint_bits.wrapping_sub(bw.bits).wrapping_sub(1);
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
