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
    const MASK: TflacUint = TflacUint::MAX << 1;

    val = val.wrapping_shl(64_u32.wrapping_sub(bits));

    unsafe {
        (*bw).tot = (*bw).tot.wrapping_add(bits);
    }

    let mut i = 0;
    while unsafe { (*bw).bits.wrapping_add(bits) >= 64 } && i < 100 {
        let mut b = unsafe { 64_u32.wrapping_sub((*bw).bits).wrapping_sub(1) };
        b = b.min(bits);

        unsafe {
            (*bw).val |= val.wrapping_shr((*bw).bits);
            (*bw).bits = (*bw).bits.wrapping_add(b);
            (*bw).val &= MASK;
        }

        val = val.wrapping_shl(b);
        bits = bits.wrapping_sub(b);
        i += 1;
    }

    unsafe {
        (*bw).val |= val.wrapping_shr((*bw).bits);
        (*bw).bits = (*bw).bits.wrapping_add(bits);
    }

    0
}

#[cfg(test)]
mod tests {
    use super::TflacBitwriter;
    use std::mem::{align_of, offset_of, size_of};

    #[test]
    fn bitwriter_layout_matches_c() {
        assert_eq!(size_of::<TflacBitwriter>(), 32);
        assert_eq!(align_of::<TflacBitwriter>(), 8);
        assert_eq!(offset_of!(TflacBitwriter, val), 0);
        assert_eq!(offset_of!(TflacBitwriter, bits), 8);
        assert_eq!(offset_of!(TflacBitwriter, pos), 12);
        assert_eq!(offset_of!(TflacBitwriter, len), 16);
        assert_eq!(offset_of!(TflacBitwriter, tot), 20);
        assert_eq!(offset_of!(TflacBitwriter, buffer), 24);
    }
}
