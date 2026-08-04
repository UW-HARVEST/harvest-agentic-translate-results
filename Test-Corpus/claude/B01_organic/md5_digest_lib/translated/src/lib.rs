#[allow(non_camel_case_types)]
pub type tflac_u8 = u8;
#[allow(non_camel_case_types)]
pub type tflac_u32 = u32;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct tflac_md5 {
    pub a: tflac_u32,
    pub b: tflac_u32,
    pub c: tflac_u32,
    pub d: tflac_u32,
}

/// # Safety
/// `m` must be a valid pointer to a `tflac_md5` and `out` must point to at
/// least 16 writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    let m = &*m;
    let out_slice = std::slice::from_raw_parts_mut(out, 16);

    out_slice[0] = m.a as tflac_u8;
    out_slice[1] = (m.a >> 8) as tflac_u8;
    out_slice[2] = (m.a >> 16) as tflac_u8;
    out_slice[3] = (m.a >> 24) as tflac_u8;
    out_slice[4] = m.b as tflac_u8;
    out_slice[5] = (m.b >> 8) as tflac_u8;
    out_slice[6] = (m.b >> 16) as tflac_u8;
    out_slice[7] = (m.b >> 24) as tflac_u8;
    out_slice[8] = m.c as tflac_u8;
    out_slice[9] = (m.c >> 8) as tflac_u8;
    out_slice[10] = (m.c >> 16) as tflac_u8;
    out_slice[11] = (m.c >> 24) as tflac_u8;
    out_slice[12] = m.d as tflac_u8;
    out_slice[13] = (m.d >> 8) as tflac_u8;
    out_slice[14] = (m.d >> 16) as tflac_u8;
    out_slice[15] = (m.d >> 24) as tflac_u8;
}
