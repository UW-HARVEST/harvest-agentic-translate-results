use std::os::raw::c_int;

type TflacU8 = u8;
type TflacU32 = u32;
type TflacU64 = u64;
type TflacUint = TflacU64;

const UINT_BITS: u32 = (std::mem::size_of::<TflacUint>() * 8) as u32;

#[repr(C)]
pub struct TflacBitwriter {
    val: TflacUint,
    bits: TflacU32,
    pos: TflacU32,
    len: TflacU32,
    tot: TflacU32,
    buffer: *mut TflacU8,
}

#[unsafe(no_mangle)]
pub extern "C" fn bitwriter_add(
    bw: *mut TflacBitwriter,
    mut bits: TflacU32,
    mut val: TflacUint,
) -> c_int {
    let bw = unsafe { &mut *bw };
    let mask: TflacUint = u64::MAX << 1;

    val = val.wrapping_shl(UINT_BITS.wrapping_sub(bits));
    bw.tot = bw.tot.wrapping_add(bits);

    let mut i = 0;
    while bw.bits.wrapping_add(bits) >= UINT_BITS && i < 100 {
        let mut b = UINT_BITS - bw.bits - 1;
        b = if b > bits { bits } else { b };
        bw.val |= val.wrapping_shr(bw.bits);
        bw.bits = bw.bits.wrapping_add(b);
        bw.val &= mask;
        val = val.wrapping_shl(b);
        bits -= b;
        i += 1;
    }

    bw.val |= val.wrapping_shr(bw.bits);
    bw.bits = bw.bits.wrapping_add(bits);

    0
}
