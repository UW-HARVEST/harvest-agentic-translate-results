#![allow(non_camel_case_types)]

pub type tflac_u8 = u8;
pub type tflac_s32 = i32;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;
type tflac_uint = tflac_u64;

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

#[unsafe(no_mangle)]
pub extern "C" fn tflac_pack_u64le(d: *mut tflac_u8, n: tflac_u64) {
    unsafe {
        // Match the C byte stores exactly.
        *d.add(0) = n as tflac_u8;
        *d.add(1) = (n >> 8) as tflac_u8;
        *d.add(2) = (n >> 16) as tflac_u8;
        *d.add(3) = (n >> 24) as tflac_u8;
        *d.add(4) = (n >> 32) as tflac_u8;
        *d.add(5) = (n >> 40) as tflac_u8;
        *d.add(6) = (n >> 48) as tflac_u8;
        *d.add(7) = (n >> 56) as tflac_u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tflac_md5_addsample(m: *mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    unsafe {
        let m = &mut *m;
        m.total = m.total.wrapping_add(bits as tflac_u64);

        let mut bytes = bits / 8;
        let pos2 = m.pos % 64;
        tflac_pack_u64le(m.buffer.as_mut_ptr().add(pos2 as usize), val);
        m.pos = m.pos.wrapping_add(bytes);

        if m.pos >= 64 {
            m.pos %= 64;
            bytes = m.pos;
            while bytes != 0 {
                bytes -= 1;
                m.buffer[bytes as usize] = m.buffer[(64 + bytes) as usize];
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn update_md5(t: *mut tflac, mut samples: *const tflac_s32) -> tflac_u32 {
    unsafe {
        let t = &mut *t;
        let mut b = t.cur_blocksize.wrapping_mul(t.channels);
        let step = core::mem::size_of::<tflac_uint>() as tflac_u32;

        for _ in 0..=4 {
            let mut v = ((*samples.add(0) as tflac_uint) & 0xFF) << 0;
            v |= ((*samples.add(1) as tflac_uint) & 0xFF) << 8;
            v |= ((*samples.add(2) as tflac_uint) & 0xFF) << 16;
            v |= ((*samples.add(3) as tflac_uint) & 0xFF) << 24;
            v |= ((*samples.add(4) as tflac_uint) & 0xFF) << 32;
            v |= ((*samples.add(5) as tflac_uint) & 0xFF) << 40;
            v |= ((*samples.add(6) as tflac_uint) & 0xFF) << 48;
            v |= ((*samples.add(7) as tflac_uint) & 0xFF) << 56;

            tflac_md5_addsample(&mut t.md5_ctx, (8 * core::mem::size_of::<tflac_uint>()) as tflac_u32, v);
            b = b.wrapping_sub(step);

            // Preserve the original C bug: pointer arithmetic advances by
            // 8 * sizeof(tflac_s32) elements, not 8 elements.
            samples = samples.add(8 * core::mem::size_of::<tflac_s32>());
        }

        b
    }
}
