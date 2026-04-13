use std::os::raw::{c_uint, c_uchar};

#[repr(C)]
pub struct TflacMd5 {
    pub a: c_uint,
    pub b: c_uint,
    pub c: c_uint,
    pub d: c_uint,
}

#[unsafe(no_mangle)]
pub extern "C" fn md5_digest(m: *const TflacMd5, out: *mut c_uchar) {
    let m = unsafe { &*m };
    let out = unsafe { std::slice::from_raw_parts_mut(out, 16) };
    out[0] = m.a as c_uchar;
    out[1] = (m.a >> 8) as c_uchar;
    out[2] = (m.a >> 16) as c_uchar;
    out[3] = (m.a >> 24) as c_uchar;
    out[4] = m.b as c_uchar;
    out[5] = (m.b >> 8) as c_uchar;
    out[6] = (m.b >> 16) as c_uchar;
    out[7] = (m.b >> 24) as c_uchar;
    out[8] = m.c as c_uchar;
    out[9] = (m.c >> 8) as c_uchar;
    out[10] = (m.c >> 16) as c_uchar;
    out[11] = (m.c >> 24) as c_uchar;
    out[12] = m.d as c_uchar;
    out[13] = (m.d >> 8) as c_uchar;
    out[14] = (m.d >> 16) as c_uchar;
    out[15] = (m.d >> 24) as c_uchar;
}
