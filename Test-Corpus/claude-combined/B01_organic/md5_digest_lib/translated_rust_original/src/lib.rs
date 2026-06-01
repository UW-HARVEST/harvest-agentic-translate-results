#[repr(C)]
pub struct TflacMd5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

/// # Safety
/// `m` must point to a valid `TflacMd5` and `out` must point to at least 16 writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const TflacMd5, out: *mut u8) {
    let m = &*m;
    let bytes = [
        m.a as u8,
        (m.a >> 8) as u8,
        (m.a >> 16) as u8,
        (m.a >> 24) as u8,
        m.b as u8,
        (m.b >> 8) as u8,
        (m.b >> 16) as u8,
        (m.b >> 24) as u8,
        m.c as u8,
        (m.c >> 8) as u8,
        (m.c >> 16) as u8,
        (m.c >> 24) as u8,
        m.d as u8,
        (m.d >> 8) as u8,
        (m.d >> 16) as u8,
        (m.d >> 24) as u8,
    ];
    let out_slice = std::slice::from_raw_parts_mut(out, 16);
    out_slice.copy_from_slice(&bytes);
}
