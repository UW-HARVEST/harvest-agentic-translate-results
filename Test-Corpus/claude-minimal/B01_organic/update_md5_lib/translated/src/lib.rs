#![allow(non_camel_case_types)]
#![allow(dead_code)]

pub type tflac_u8 = u8;
pub type tflac_s32 = i32;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;

pub type tflac_uint = tflac_u64;

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

pub fn tflac_pack_u64le(d: &mut [tflac_u8], n: tflac_u64) {
    d[0] = n as tflac_u8;
    d[1] = (n >> 8) as tflac_u8;
    d[2] = (n >> 16) as tflac_u8;
    d[3] = (n >> 24) as tflac_u8;
    d[4] = (n >> 32) as tflac_u8;
    d[5] = (n >> 40) as tflac_u8;
    d[6] = (n >> 48) as tflac_u8;
    d[7] = (n >> 56) as tflac_u8;
}

pub fn tflac_md5_addsample(m: &mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    let mut bytes: tflac_u32;
    m.total = m.total.wrapping_add(bits as tflac_u64);
    bytes = bits / 8;
    let pos2: tflac_u32 = m.pos % 64;
    tflac_pack_u64le(&mut m.buffer[pos2 as usize..], val);
    m.pos = m.pos.wrapping_add(bytes);
    if m.pos >= 64 {
        m.pos %= 64;
        bytes = m.pos;
        while bytes > 0 {
            bytes -= 1;
            m.buffer[bytes as usize] = m.buffer[64 + bytes as usize];
        }
    }
}

/// # Safety
/// `samples` must point to a valid array containing enough elements for the
/// reads performed by this function. The original C function uses pointer
/// arithmetic of `8 * sizeof(tflac_s32)` (i.e. 32 `i32` elements) per
/// iteration and reads indices 0..=7 of each stride.
pub unsafe fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    let t_ref = &mut *t;
    let mut b: tflac_u32 = t_ref.cur_blocksize.wrapping_mul(t_ref.channels);
    let step: tflac_u32 = core::mem::size_of::<tflac_uint>() as tflac_u32;
    let mut v: tflac_uint;
    let mut samples = samples;
    for _i in 0..=4 {
        v = ((*samples.add(0) as tflac_uint) & 0xFF) << 0;
        v |= ((*samples.add(1) as tflac_uint) & 0xFF) << 8;
        v |= ((*samples.add(2) as tflac_uint) & 0xFF) << 16;
        v |= ((*samples.add(3) as tflac_uint) & 0xFF) << 24;
        v |= ((*samples.add(4) as tflac_uint) & 0xFF) << 32;
        v |= ((*samples.add(5) as tflac_uint) & 0xFF) << 40;
        v |= ((*samples.add(6) as tflac_uint) & 0xFF) << 48;
        v |= ((*samples.add(7) as tflac_uint) & 0xFF) << 56;
        tflac_md5_addsample(
            &mut t_ref.md5_ctx,
            (8 * core::mem::size_of::<tflac_uint>()) as tflac_u32,
            v,
        );
        b = b.wrapping_sub(step);
        samples = samples.add(8 * core::mem::size_of::<tflac_s32>());
    }
    b
}
