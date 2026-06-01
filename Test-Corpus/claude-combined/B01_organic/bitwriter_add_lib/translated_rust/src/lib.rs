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

/// # Safety
/// `bw` must be a valid pointer to a `TflacBitwriter` struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bitwriter_add(
    bw: *mut TflacBitwriter,
    mut bits: TflacU32,
    mut val: TflacUint,
) -> c_int {
    // const tflac_uint mask = (18446744073709551615UL) << 1;
    // 0xFFFFFFFFFFFFFFFF << 1 == 0xFFFFFFFFFFFFFFFE
    let mask: TflacUint = 0xFFFFFFFFFFFFFFFFu64.wrapping_shl(1);
    let bw = unsafe { &mut *bw };

    // val <<= ((8 * sizeof(tflac_uint)) - bits);
    // sizeof(tflac_uint) == 8, so 8 * 8 == 64. If bits == 0, shift by 64.
    // In C this is undefined, but on x86-64 the shift count is masked to 6 bits.
    // Use wrapping_shl which does the same masking on Rust.
    let shift_amount: u32 = (8u32 * core::mem::size_of::<TflacUint>() as u32).wrapping_sub(bits);
    val = val.wrapping_shl(shift_amount);
    bw.tot = bw.tot.wrapping_add(bits);

    let mut i: c_int = 0;
    while bw.bits.wrapping_add(bits) as usize >= 8 * core::mem::size_of::<TflacUint>()
        && i < 100
    {
        // b = (8 * sizeof(tflac_uint)) - bw->bits - 1;
        let mut b: TflacU32 = (8u32 * core::mem::size_of::<TflacUint>() as u32)
            .wrapping_sub(bw.bits)
            .wrapping_sub(1);
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
