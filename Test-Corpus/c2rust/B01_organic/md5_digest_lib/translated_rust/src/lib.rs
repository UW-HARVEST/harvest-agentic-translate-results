pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type tflac_u8 = uint8_t;
pub type tflac_u32 = uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac_md5 {
    pub a: tflac_u32,
    pub b: tflac_u32,
    pub c: tflac_u32,
    pub d: tflac_u32,
}
#[no_mangle]
pub unsafe extern "C" fn md5_digest(mut m: *const tflac_md5, mut out: *mut tflac_u8) {
    *out.offset(0 as ::core::ffi::c_int as isize) = (*m).a as tflac_u8;
    *out.offset(1 as ::core::ffi::c_int as isize) = ((*m).a >> 8 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(2 as ::core::ffi::c_int as isize) =
        ((*m).a >> 16 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(3 as ::core::ffi::c_int as isize) =
        ((*m).a >> 24 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(4 as ::core::ffi::c_int as isize) = (*m).b as tflac_u8;
    *out.offset(5 as ::core::ffi::c_int as isize) = ((*m).b >> 8 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(6 as ::core::ffi::c_int as isize) =
        ((*m).b >> 16 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(7 as ::core::ffi::c_int as isize) =
        ((*m).b >> 24 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(8 as ::core::ffi::c_int as isize) = (*m).c as tflac_u8;
    *out.offset(9 as ::core::ffi::c_int as isize) = ((*m).c >> 8 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(10 as ::core::ffi::c_int as isize) =
        ((*m).c >> 16 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(11 as ::core::ffi::c_int as isize) =
        ((*m).c >> 24 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(12 as ::core::ffi::c_int as isize) = (*m).d as tflac_u8;
    *out.offset(13 as ::core::ffi::c_int as isize) =
        ((*m).d >> 8 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(14 as ::core::ffi::c_int as isize) =
        ((*m).d >> 16 as ::core::ffi::c_int) as tflac_u8;
    *out.offset(15 as ::core::ffi::c_int as isize) =
        ((*m).d >> 24 as ::core::ffi::c_int) as tflac_u8;
}
