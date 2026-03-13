#[repr(C)]
pub struct tflac_md5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut u8) {
    let m = unsafe { &*m };
    let out = unsafe { core::slice::from_raw_parts_mut(out, 16) };
    out[0..4].copy_from_slice(&m.a.to_le_bytes());
    out[4..8].copy_from_slice(&m.b.to_le_bytes());
    out[8..12].copy_from_slice(&m.c.to_le_bytes());
    out[12..16].copy_from_slice(&m.d.to_le_bytes());
}
