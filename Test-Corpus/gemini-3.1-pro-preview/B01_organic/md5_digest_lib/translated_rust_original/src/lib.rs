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
    let (m_ref, out_slice) = unsafe {
        (&*m, std::slice::from_raw_parts_mut(out, 16))
    };
    
    out_slice[0..4].copy_from_slice(&m_ref.a.to_le_bytes());
    out_slice[4..8].copy_from_slice(&m_ref.b.to_le_bytes());
    out_slice[8..12].copy_from_slice(&m_ref.c.to_le_bytes());
    out_slice[12..16].copy_from_slice(&m_ref.d.to_le_bytes());
}
