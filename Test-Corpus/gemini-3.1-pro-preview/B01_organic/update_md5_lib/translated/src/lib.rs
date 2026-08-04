pub type tflac_u8 = u8;
pub type tflac_s32 = i32;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;

#[repr(C)]
pub struct tflac_md5 {
    pub pos: tflac_u32,
    pub total: tflac_u64,
    pub buffer: [tflac_u8; 64 + 8],
}

#[repr(C)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: tflac_u32,
    pub channels: tflac_u32,
}

type tflac_uint = tflac_u64;

fn tflac_pack_u64le(d: &mut [tflac_u8], n: tflac_u64) {
    d[0] = n as tflac_u8;
    d[1] = (n >> 8) as tflac_u8;
    d[2] = (n >> 16) as tflac_u8;
    d[3] = (n >> 24) as tflac_u8;
    d[4] = (n >> 32) as tflac_u8;
    d[5] = (n >> 40) as tflac_u8;
    d[6] = (n >> 48) as tflac_u8;
    d[7] = (n >> 56) as tflac_u8;
}

fn tflac_md5_addsample(m: &mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    m.total += bits as tflac_u64;
    let mut bytes = bits / 8;
    let pos2 = (m.pos % 64) as usize;
    tflac_pack_u64le(&mut m.buffer[pos2..], val);
    m.pos += bytes;
    if m.pos >= 64 {
        m.pos %= 64;
        bytes = m.pos;
        while bytes > 0 {
            bytes -= 1;
            m.buffer[bytes as usize] = m.buffer[64 + bytes as usize];
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    let t = unsafe { &mut *t };
    let mut b = t.cur_blocksize * t.channels;
    let step = std::mem::size_of::<tflac_uint>() as tflac_u32;
    let mut samples_ptr = samples;

    for _ in 0..=4 {
        let s = unsafe { std::slice::from_raw_parts(samples_ptr, 8) };
        let mut v: tflac_uint = ((s[0] as tflac_uint) & 0xFF) << 0;
        v |= ((s[1] as tflac_uint) & 0xFF) << 8;
        v |= ((s[2] as tflac_uint) & 0xFF) << 16;
        v |= ((s[3] as tflac_uint) & 0xFF) << 24;
        v |= ((s[4] as tflac_uint) & 0xFF) << 32;
        v |= ((s[5] as tflac_uint) & 0xFF) << 40;
        v |= ((s[6] as tflac_uint) & 0xFF) << 48;
        v |= ((s[7] as tflac_uint) & 0xFF) << 56;

        tflac_md5_addsample(
            &mut t.md5_ctx,
            (8 * std::mem::size_of::<tflac_uint>()) as tflac_u32,
            v,
        );
        b -= step;
        samples_ptr = unsafe { samples_ptr.add(8 * std::mem::size_of::<tflac_s32>()) };
    }
    b
}
