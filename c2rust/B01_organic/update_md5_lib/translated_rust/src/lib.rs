pub type __uint8_t = u8;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type int32_t = __int32_t;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type tflac_u8 = uint8_t;
pub type tflac_s32 = int32_t;
pub type tflac_u32 = uint32_t;
pub type tflac_u64 = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac_md5 {
    pub pos: tflac_u32,
    pub total: tflac_u64,
    pub buffer: [tflac_u8; 72],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: tflac_u32,
    pub channels: tflac_u32,
}
pub type tflac_uint = tflac_u64;
#[no_mangle]
pub unsafe extern "C" fn tflac_pack_u64le(mut d: *mut tflac_u8, mut n: tflac_u64) {
    *d.offset(0 as ::core::ffi::c_int as isize) = n as tflac_u8;
    *d.offset(1 as ::core::ffi::c_int as isize) = (n >> 8 as ::core::ffi::c_int) as tflac_u8;
    *d.offset(2 as ::core::ffi::c_int as isize) = (n >> 16 as ::core::ffi::c_int) as tflac_u8;
    *d.offset(3 as ::core::ffi::c_int as isize) = (n >> 24 as ::core::ffi::c_int) as tflac_u8;
    *d.offset(4 as ::core::ffi::c_int as isize) = (n >> 32 as ::core::ffi::c_int) as tflac_u8;
    *d.offset(5 as ::core::ffi::c_int as isize) = (n >> 40 as ::core::ffi::c_int) as tflac_u8;
    *d.offset(6 as ::core::ffi::c_int as isize) = (n >> 48 as ::core::ffi::c_int) as tflac_u8;
    *d.offset(7 as ::core::ffi::c_int as isize) = (n >> 56 as ::core::ffi::c_int) as tflac_u8;
}
#[no_mangle]
pub unsafe extern "C" fn tflac_md5_addsample(
    mut m: *mut tflac_md5,
    mut bits: tflac_u32,
    mut val: tflac_uint,
) {
    let mut bytes: tflac_u32 = 0;
    (*m).total = ((*m).total as ::core::ffi::c_ulong)
        .wrapping_add(bits as tflac_u64 as ::core::ffi::c_ulong) as tflac_u64
        as tflac_u64;
    bytes = bits.wrapping_div(8 as tflac_u32);
    let mut pos2: tflac_u32 = (*m).pos.wrapping_rem(64 as tflac_u32);
    tflac_pack_u64le(
        (&raw mut (*m).buffer as *mut tflac_u8).offset(pos2 as isize) as *mut tflac_u8,
        val as tflac_u64,
    );
    (*m).pos = ((*m).pos as ::core::ffi::c_uint).wrapping_add(bytes as ::core::ffi::c_uint)
        as tflac_u32 as tflac_u32;
    if (*m).pos >= 64 as tflac_u32 {
        (*m).pos = ((*m).pos as ::core::ffi::c_uint).wrapping_rem(64 as ::core::ffi::c_uint)
            as tflac_u32 as tflac_u32;
        bytes = (*m).pos;
        loop {
            let fresh0 = bytes;
            bytes = bytes.wrapping_sub(1);
            if !(fresh0 != 0) {
                break;
            }
            (*m).buffer[bytes as usize] =
                (*m).buffer[(64 as tflac_u32).wrapping_add(bytes) as usize];
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn update_md5(mut t: *mut tflac, mut samples: *const tflac_s32) -> tflac_u32 {
    let mut b: tflac_u32 = (*t).cur_blocksize.wrapping_mul((*t).channels);
    let step: tflac_u32 = ::core::mem::size_of::<tflac_uint>() as tflac_u32;
    let mut v: tflac_uint = 0;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i <= 4 as ::core::ffi::c_int {
        v = (*samples.offset(0 as ::core::ffi::c_int as isize) as tflac_uint & 0xff as tflac_uint)
            << 0 as ::core::ffi::c_int;
        v = (v as ::core::ffi::c_ulong
            | ((*samples.offset(1 as ::core::ffi::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 8 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as tflac_uint;
        v = (v as ::core::ffi::c_ulong
            | ((*samples.offset(2 as ::core::ffi::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 16 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as tflac_uint;
        v = (v as ::core::ffi::c_ulong
            | ((*samples.offset(3 as ::core::ffi::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as tflac_uint;
        v = (v as ::core::ffi::c_ulong
            | ((*samples.offset(4 as ::core::ffi::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 32 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as tflac_uint;
        v = (v as ::core::ffi::c_ulong
            | ((*samples.offset(5 as ::core::ffi::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 40 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as tflac_uint;
        v = (v as ::core::ffi::c_ulong
            | ((*samples.offset(6 as ::core::ffi::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 48 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as tflac_uint;
        v = (v as ::core::ffi::c_ulong
            | ((*samples.offset(7 as ::core::ffi::c_int as isize) as tflac_uint
                & 0xff as tflac_uint)
                << 56 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as tflac_uint;
        tflac_md5_addsample(
            &raw mut (*t).md5_ctx,
            (8 as usize).wrapping_mul(::core::mem::size_of::<tflac_uint>() as usize) as tflac_u32,
            v,
        );
        b = (b as ::core::ffi::c_uint).wrapping_sub(step as ::core::ffi::c_uint) as tflac_u32
            as tflac_u32;
        samples = samples.offset(
            (8 as usize).wrapping_mul(::core::mem::size_of::<tflac_s32>() as usize) as isize,
        );
        i += 1;
    }
    return b;
}
