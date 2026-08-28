use std::ffi::c_int;

#[repr(C)]
pub struct TflacBitwriter {
    pub val: u64,
    pub bits: u32,
    pub pos: u32,
    pub len: u32,
    pub tot: u32,
    pub buffer: *mut u8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut TflacBitwriter,
    mut bits: u32,
    mut val: u64,
) -> c_int {
    let bw = unsafe { &mut *bw };
    let mask = u64::MAX << 1;

    val = val.wrapping_shl(64_u32.wrapping_sub(bits));
    bw.tot = bw.tot.wrapping_add(bits);

    let mut i = 0;
    while bw.bits.wrapping_add(bits) >= 64 && i < 100 {
        let available = 63_u32.wrapping_sub(bw.bits);
        let b = available.min(bits);

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
