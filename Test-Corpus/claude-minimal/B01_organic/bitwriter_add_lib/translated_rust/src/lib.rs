// Translation of c_src/src/lib.c

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

/// Translation of `bitwriter_add` from `c_src/src/lib.c`.
///
/// # Safety
///
/// `bw` must be a valid, non-null pointer to a `TflacBitwriter`.
#[no_mangle]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut TflacBitwriter,
    mut bits: TflacU32,
    mut val: TflacUint,
) -> i32 {
    let mask: TflacUint = (18446744073709551615u64).wrapping_shl(1);
    let bw = &mut *bw;
    let uint_bits: TflacU32 = (8 * core::mem::size_of::<TflacUint>()) as TflacU32;

    val = val.wrapping_shl(uint_bits - bits);
    bw.tot = bw.tot.wrapping_add(bits);
    let mut i: i32 = 0;
    while (bw.bits + bits >= uint_bits) && i < 100 {
        let mut b: TflacU32 = uint_bits - bw.bits - 1;
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
