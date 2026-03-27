use std::os::raw::c_int;

#[repr(C)]
pub struct tflac_bitwriter {
    pub val: u64,
    pub bits: u32,
    pub pos: u32,
    pub len: u32,
    pub tot: u32,
    pub buffer: *mut u8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut tflac_bitwriter,
    mut bits: u32,
    mut val: u64,
) -> c_int {
    let bw = unsafe { &mut *bw };
    let mask: u64 = 18446744073709551615u64.wrapping_shl(1);
    val = val.wrapping_shl(64u32.wrapping_sub(bits));
    bw.tot = bw.tot.wrapping_add(bits);
    let mut i: c_int = 0;
    while bw.bits.wrapping_add(bits) >= 64 && i < 100 {
        let mut b: u32 = 64u32.wrapping_sub(bw.bits).wrapping_sub(1);
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
