use std::os::raw::{c_int, c_uint};

pub type TflacU8 = u8;
pub type TflacS32 = i32;
pub type TflacU32 = u32;
pub type TflacU64 = u64;

#[repr(C)]
pub struct TflacMd5 {
    pub pos: TflacU32,
    pub total: TflacU64,
    pub buffer: [TflacU8; 72],
}

#[repr(C)]
pub struct Tflac {
    pub md5_ctx: TflacMd5,
    pub cur_blocksize: TflacU32,
    pub channels: TflacU32,
}

type TflacUint = TflacU64;

fn tflac_pack_u64le(d: &mut [TflacU8], n: TflacU64) {
    d[0] = n as TflacU8;
    d[1] = (n >> 8) as TflacU8;
    d[2] = (n >> 16) as TflacU8;
    d[3] = (n >> 24) as TflacU8;
    d[4] = (n >> 32) as TflacU8;
    d[5] = (n >> 40) as TflacU8;
    d[6] = (n >> 48) as TflacU8;
    d[7] = (n >> 56) as TflacU8;
}

fn tflac_md5_addsample(m: &mut TflacMd5, bits: TflacU32, val: TflacUint) {
    let bytes: TflacU32;
    m.total += bits as TflacU64;
    bytes = bits / 8;
    let pos2 = (m.pos % 64) as usize;
    tflac_pack_u64le(&mut m.buffer[pos2..pos2 + 8], val);
    m.pos += bytes;
    if m.pos >= 64 {
        m.pos %= 64;
        let mut bytes = m.pos;
        while bytes > 0 {
            bytes -= 1;
            m.buffer[bytes as usize] = m.buffer[64 + bytes as usize];
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn update_md5(t: *mut Tflac, samples: *const TflacS32) -> TflacU32 {
    unsafe {
        let t = &mut *t;
        let mut b = t.cur_blocksize * t.channels;
        let step = std::mem::size_of::<TflacUint>() as TflacU32;
        let mut v: TflacUint;
        let mut samples = samples;
        for _ in 0..=4 {
            v = ((*samples as TflacUint) & 0xFF) << 0;
            v |= (((*samples.add(1)) as TflacUint) & 0xFF) << 8;
            v |= (((*samples.add(2)) as TflacUint) & 0xFF) << 16;
            v |= (((*samples.add(3)) as TflacUint) & 0xFF) << 24;
            v |= (((*samples.add(4)) as TflacUint) & 0xFF) << 32;
            v |= (((*samples.add(5)) as TflacUint) & 0xFF) << 40;
            v |= (((*samples.add(6)) as TflacUint) & 0xFF) << 48;
            v |= (((*samples.add(7)) as TflacUint) & 0xFF) << 56;
            tflac_md5_addsample(&mut t.md5_ctx, 8 * std::mem::size_of::<TflacUint>() as TflacU32, v);
            b -= step;
            samples = samples.add(8 * std::mem::size_of::<TflacS32>());
        }
        b
    }
}
