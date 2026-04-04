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
pub unsafe extern "C" fn bitwriter_add(
    mut bw: *mut tflac_bitwriter,
    mut bits: tflac_u32,
    mut val: tflac_uint,
) -> ::core::ffi::c_int {
    let mask: tflac_uint = (18446744073709551615 as tflac_uint) << 1 as ::core::ffi::c_int;
    let mut b: tflac_u32 = 0;
    let mut r: ::core::ffi::c_int = 0;
    val <<= (8 as usize)
        .wrapping_mul(::core::mem::size_of::<tflac_uint>() as usize)
        .wrapping_sub(bits as usize);
    (*bw).tot = ((*bw).tot as ::core::ffi::c_uint).wrapping_add(bits as ::core::ffi::c_uint)
        as tflac_u32 as tflac_u32;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while (*bw).bits.wrapping_add(bits) as usize
        >= (8 as usize).wrapping_mul(::core::mem::size_of::<tflac_uint>() as usize)
        && i < 100 as ::core::ffi::c_int
    {
        b = (8 as usize)
            .wrapping_mul(::core::mem::size_of::<tflac_uint>() as usize)
            .wrapping_sub((*bw).bits as usize)
            .wrapping_sub(1 as usize) as tflac_u32;
        b = if b > bits { bits } else { b };
        (*bw).val = ((*bw).val as ::core::ffi::c_ulong
            | (val >> (*bw).bits) as ::core::ffi::c_ulong) as tflac_uint;
        (*bw).bits = ((*bw).bits as ::core::ffi::c_uint).wrapping_add(b as ::core::ffi::c_uint)
            as tflac_u32 as tflac_u32;
        (*bw).val =
            ((*bw).val as ::core::ffi::c_ulong & mask as ::core::ffi::c_ulong) as tflac_uint;
        val <<= b;
        bits = (bits as ::core::ffi::c_uint).wrapping_sub(b as ::core::ffi::c_uint) as tflac_u32
            as tflac_u32;
        i += 1;
    }
    (*bw).val = ((*bw).val as ::core::ffi::c_ulong | (val >> (*bw).bits) as ::core::ffi::c_ulong)
        as tflac_uint;
    (*bw).bits = ((*bw).bits as ::core::ffi::c_uint).wrapping_add(bits as ::core::ffi::c_uint)
        as tflac_u32 as tflac_u32;
    return 0 as ::core::ffi::c_int;
}
