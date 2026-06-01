#![allow(non_camel_case_types)]

use std::ffi::c_int;

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

/// # Safety
/// `d` must point to a buffer of at least 8 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut tflac_u8, n: tflac_u64) {
    *d.add(0) = n as tflac_u8;
    *d.add(1) = (n >> 8) as tflac_u8;
    *d.add(2) = (n >> 16) as tflac_u8;
    *d.add(3) = (n >> 24) as tflac_u8;
    *d.add(4) = (n >> 32) as tflac_u8;
    *d.add(5) = (n >> 40) as tflac_u8;
    *d.add(6) = (n >> 48) as tflac_u8;
    *d.add(7) = (n >> 56) as tflac_u8;
}

/// # Safety
/// `m` must be a valid pointer to a `tflac_md5` struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(m: *mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    let m_ref = &mut *m;
    let mut bytes: tflac_u32;
    m_ref.total = m_ref.total.wrapping_add(bits as tflac_u64);
    bytes = bits / 8;
    let pos2 = (m_ref.pos % 64) as usize;
    tflac_pack_u64le(m_ref.buffer.as_mut_ptr().add(pos2), val);
    m_ref.pos = m_ref.pos.wrapping_add(bytes);
    if m_ref.pos >= 64 {
        m_ref.pos %= 64;
        bytes = m_ref.pos;
        // C semantics: while (bytes--) — loops while pre-decrement value is non-zero,
        // copies indices (bytes-1)..0 from buffer[64+i] to buffer[i].
        loop {
            let cur = bytes;
            if cur == 0 {
                break;
            }
            bytes = cur.wrapping_sub(1);
            let idx = bytes as usize;
            m_ref.buffer[idx] = m_ref.buffer[64 + idx];
        }
    }
}

/// # Safety
/// `t` must be a valid pointer to a `tflac` struct.
/// `samples` must point to a sufficiently large buffer of `tflac_s32` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    let t_ref = &mut *t;
    let mut b: tflac_u32 = t_ref.cur_blocksize.wrapping_mul(t_ref.channels);
    let step: tflac_u32 = std::mem::size_of::<tflac_uint>() as tflac_u32; // 8
    let mut v: tflac_uint;
    let mut samples_ptr = samples;
    let mut _i: c_int = 0;
    while _i <= 4 {
        v = (*samples_ptr.add(0) as tflac_uint) & 0xFF;
        v |= ((*samples_ptr.add(1) as tflac_uint) & 0xFF) << 8;
        v |= ((*samples_ptr.add(2) as tflac_uint) & 0xFF) << 16;
        v |= ((*samples_ptr.add(3) as tflac_uint) & 0xFF) << 24;
        v |= ((*samples_ptr.add(4) as tflac_uint) & 0xFF) << 32;
        v |= ((*samples_ptr.add(5) as tflac_uint) & 0xFF) << 40;
        v |= ((*samples_ptr.add(6) as tflac_uint) & 0xFF) << 48;
        v |= ((*samples_ptr.add(7) as tflac_uint) & 0xFF) << 56;
        tflac_md5_addsample(
            &mut t_ref.md5_ctx,
            8 * std::mem::size_of::<tflac_uint>() as tflac_u32,
            v,
        );
        b = b.wrapping_sub(step);
        // C: samples += (8 * sizeof(tflac_s32));
        // sizeof(tflac_s32) is 4, so this advances pointer by 32 elements.
        samples_ptr = samples_ptr.add(8 * std::mem::size_of::<tflac_s32>());
        _i += 1;
    }
    b
}
