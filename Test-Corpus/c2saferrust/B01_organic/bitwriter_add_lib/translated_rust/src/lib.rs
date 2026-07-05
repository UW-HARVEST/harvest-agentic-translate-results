
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type tflac_u8 = uint8_t;
pub type tflac_u32 = uint32_t;
pub type tflac_u64 = uint64_t;
pub type tflac_uint = tflac_u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac_bitwriter {
    pub val: tflac_uint,
    pub bits: tflac_u32,
    pub pos: tflac_u32,
    pub len: tflac_u32,
    pub tot: tflac_u32,
    pub buffer: *mut tflac_u8,
}
#[no_mangle]
pub fn bitwriter_add(
    bw: &mut tflac_bitwriter,
    mut bits: tflac_u32,
    mut val: tflac_uint,
) -> i32 {
    let word_bits = (8 * core::mem::size_of::<tflac_uint>()) as tflac_u32;
    let mask: tflac_uint = (!0 as tflac_uint) << 1;

    let shift = word_bits.wrapping_sub(bits) as usize;
    val <<= shift;

    bw.tot = bw.tot.wrapping_add(bits);

    let mut i = 0;
    while bw.bits.wrapping_add(bits) >= word_bits && i < 100 {
        let mut b = word_bits.wrapping_sub(bw.bits).wrapping_sub(1);
        if b > bits {
            b = bits;
        }

        bw.val |= val >> bw.bits;
        bw.bits = bw.bits.wrapping_add(b);
        bw.val &= mask;

        val <<= b as usize;
        bits = bits.wrapping_sub(b);
        i += 1;
    }

    bw.val |= val >> bw.bits;
    bw.bits = bw.bits.wrapping_add(bits);

    0
}

