use std::os::raw::c_int;

type TflacU8 = u8;
type TflacU32 = u32;
type TflacU64 = u64;
type TflacUint = TflacU64;

#[repr(C)]
pub struct tflac_bitwriter {
    pub val: TflacUint,
    pub bits: TflacU32,
    pub pos: TflacU32,
    pub len: TflacU32,
    pub tot: TflacU32,
    pub buffer: *mut TflacU8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut tflac_bitwriter,
    mut bits: TflacU32,
    mut val: TflacUint,
) -> c_int {
    let bw = unsafe { &mut *bw };
    let mask: TflacUint = 18446744073709551615u64.wrapping_shl(1);

    val = val.wrapping_shl(64u32.wrapping_sub(bits));
    bw.tot = bw.tot.wrapping_add(bits);

    let mut i: c_int = 0;
    while bw.bits.wrapping_add(bits) >= 64 && i < 100 {
        let mut b: TflacU32 = 64u32.wrapping_sub(bw.bits).wrapping_sub(1);
        if b > bits {
            b = bits;
        }
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
