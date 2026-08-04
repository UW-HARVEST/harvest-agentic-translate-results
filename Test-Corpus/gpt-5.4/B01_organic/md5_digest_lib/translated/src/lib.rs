pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

#[repr(C)]
pub struct tflac_md5 {
    pub a: tflac_u32,
    pub b: tflac_u32,
    pub c: tflac_u32,
    pub d: tflac_u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    let m = unsafe { &*m };
    let out = unsafe { core::slice::from_raw_parts_mut(out, 16) };

    out[0] = m.a as tflac_u8;
    out[1] = (m.a >> 8) as tflac_u8;
    out[2] = (m.a >> 16) as tflac_u8;
    out[3] = (m.a >> 24) as tflac_u8;
    out[4] = m.b as tflac_u8;
    out[5] = (m.b >> 8) as tflac_u8;
    out[6] = (m.b >> 16) as tflac_u8;
    out[7] = (m.b >> 24) as tflac_u8;
    out[8] = m.c as tflac_u8;
    out[9] = (m.c >> 8) as tflac_u8;
    out[10] = (m.c >> 16) as tflac_u8;
    out[11] = (m.c >> 24) as tflac_u8;
    out[12] = m.d as tflac_u8;
    out[13] = (m.d >> 8) as tflac_u8;
    out[14] = (m.d >> 16) as tflac_u8;
    out[15] = (m.d >> 24) as tflac_u8;
}