#[repr(C)]
pub struct tflac_md5 {
    pub pos: u32,
    pub total: u64,
    pub buffer: [u8; 64 + 8],
}

#[repr(C)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: u32,
    pub channels: u32,
}

fn tflac_pack_u64le(d: &mut [u8], n: u64) {
    d[0] = n as u8;
    d[1] = (n >> 8) as u8;
    d[2] = (n >> 16) as u8;
    d[3] = (n >> 24) as u8;
    d[4] = (n >> 32) as u8;
    d[5] = (n >> 40) as u8;
    d[6] = (n >> 48) as u8;
    d[7] = (n >> 56) as u8;
}

fn tflac_md5_addsample(m: &mut tflac_md5, bits: u32, val: u64) {
    m.total = m.total.wrapping_add(bits as u64);
    let bytes = bits / 8;
    let pos2 = (m.pos % 64) as usize;
    tflac_pack_u64le(&mut m.buffer[pos2..], val);
    m.pos = m.pos.wrapping_add(bytes);
    if m.pos >= 64 {
        m.pos %= 64;
        let mut b = m.pos as usize;
        while b > 0 {
            b -= 1;
            m.buffer[b] = m.buffer[64 + b];
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, samples: *const i32) -> u32 {
    let t = unsafe { &mut *t };
    let mut samples = samples;
    let mut b = t.cur_blocksize.wrapping_mul(t.channels);
    const STEP: u32 = 8; // sizeof(u64)
    const BITS: u32 = 8 * 8; // 8 * sizeof(u64)

    for _i in 0..5 {
        let s = unsafe { core::slice::from_raw_parts(samples, 8) };
        let mut v: u64 = 0;
        v |= ((s[0] as u64) & 0xFF) << 0;
        v |= ((s[1] as u64) & 0xFF) << 8;
        v |= ((s[2] as u64) & 0xFF) << 16;
        v |= ((s[3] as u64) & 0xFF) << 24;
        v |= ((s[4] as u64) & 0xFF) << 32;
        v |= ((s[5] as u64) & 0xFF) << 40;
        v |= ((s[6] as u64) & 0xFF) << 48;
        v |= ((s[7] as u64) & 0xFF) << 56;
        tflac_md5_addsample(&mut t.md5_ctx, BITS, v);
        b = b.wrapping_sub(STEP);
        samples = unsafe { samples.add(8 * 4) }; // 8 * sizeof(i32) = 32 elements
    }
    b
}
