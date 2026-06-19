use std::ffi::{c_uchar, c_uint};

#[repr(C)]
pub struct tflac_md5 {
    pub a: c_uint,
    pub b: c_uint,
    pub c: c_uint,
    pub d: c_uint,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut c_uchar) {
    unsafe {
        let m = &*m;

        *out.add(0) = m.a as c_uchar;
        *out.add(1) = (m.a >> 8) as c_uchar;
        *out.add(2) = (m.a >> 16) as c_uchar;
        *out.add(3) = (m.a >> 24) as c_uchar;
        *out.add(4) = m.b as c_uchar;
        *out.add(5) = (m.b >> 8) as c_uchar;
        *out.add(6) = (m.b >> 16) as c_uchar;
        *out.add(7) = (m.b >> 24) as c_uchar;
        *out.add(8) = m.c as c_uchar;
        *out.add(9) = (m.c >> 8) as c_uchar;
        *out.add(10) = (m.c >> 16) as c_uchar;
        *out.add(11) = (m.c >> 24) as c_uchar;
        *out.add(12) = m.d as c_uchar;
        *out.add(13) = (m.d >> 8) as c_uchar;
        *out.add(14) = (m.d >> 16) as c_uchar;
        *out.add(15) = (m.d >> 24) as c_uchar;
    }
}
