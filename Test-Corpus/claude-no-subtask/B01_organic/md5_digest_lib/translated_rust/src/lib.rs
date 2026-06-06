use std::ffi::c_void;

pub type TflacU8 = u8;
pub type TflacU32 = u32;

#[repr(C)]
pub struct TflacMd5 {
    pub a: TflacU32,
    pub b: TflacU32,
    pub c: TflacU32,
    pub d: TflacU32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const TflacMd5, out: *mut TflacU8) {
    let _ = m as *const c_void;
    let m = &*m;
    let out = std::slice::from_raw_parts_mut(out, 16);
    out[0] = m.a as TflacU8;
    out[1] = (m.a >> 8) as TflacU8;
    out[2] = (m.a >> 16) as TflacU8;
    out[3] = (m.a >> 24) as TflacU8;
    out[4] = m.b as TflacU8;
    out[5] = (m.b >> 8) as TflacU8;
    out[6] = (m.b >> 16) as TflacU8;
    out[7] = (m.b >> 24) as TflacU8;
    out[8] = m.c as TflacU8;
    out[9] = (m.c >> 8) as TflacU8;
    out[10] = (m.c >> 16) as TflacU8;
    out[11] = (m.c >> 24) as TflacU8;
    out[12] = m.d as TflacU8;
    out[13] = (m.d >> 8) as TflacU8;
    out[14] = (m.d >> 16) as TflacU8;
    out[15] = (m.d >> 24) as TflacU8;
}
