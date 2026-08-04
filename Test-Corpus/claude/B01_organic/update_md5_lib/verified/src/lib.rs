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

fn tflac_pack_u64le_slice(d: &mut [tflac_u8], n: tflac_u64) {
    d[0] = n as tflac_u8;
    d[1] = (n >> 8) as tflac_u8;
    d[2] = (n >> 16) as tflac_u8;
    d[3] = (n >> 24) as tflac_u8;
    d[4] = (n >> 32) as tflac_u8;
    d[5] = (n >> 40) as tflac_u8;
    d[6] = (n >> 48) as tflac_u8;
    d[7] = (n >> 56) as tflac_u8;
}

/// # Safety
/// `d` must point to at least 8 writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut tflac_u8, n: tflac_u64) {
    let slice = unsafe { std::slice::from_raw_parts_mut(d, 8) };
    tflac_pack_u64le_slice(slice, n);
}

fn tflac_md5_addsample_impl(m: &mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    let mut bytes: tflac_u32;
    m.total = m.total.wrapping_add(bits as tflac_u64);
    bytes = bits / 8;
    let pos2: tflac_u32 = m.pos % 64;
    tflac_pack_u64le_slice(&mut m.buffer[pos2 as usize..], val);
    m.pos = m.pos.wrapping_add(bytes);
    if m.pos >= 64 {
        m.pos %= 64;
        bytes = m.pos;
        // Mirror C semantics: `while (bytes--)`: test, then decrement.
        // Body uses the post-decrement value of `bytes`.
        loop {
            let cond = bytes;
            let cur = bytes.wrapping_sub(1);
            bytes = cur;
            if cond == 0 {
                break;
            }
            // In C, the index used inside the body is the *post-decrement* value
            // (which is the value already assigned back to `bytes` above).
            m.buffer[cur as usize] = m.buffer[64 + cur as usize];
        }
    }
}

/// # Safety
/// `m` must be a valid pointer to a `tflac_md5` struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(
    m: *mut tflac_md5,
    bits: tflac_u32,
    val: tflac_uint,
) {
    let m = unsafe { &mut *m };
    tflac_md5_addsample_impl(m, bits, val);
}

/// # Safety
/// `t` must be a valid pointer to a `tflac` struct, and `samples` must point
/// to a sufficiently large array of `tflac_s32` values (matching the access
/// pattern of the original C code).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    let t = unsafe { &mut *t };
    let mut b: tflac_u32 = t.cur_blocksize.wrapping_mul(t.channels);
    let step: tflac_u32 = std::mem::size_of::<tflac_uint>() as tflac_u32;
    let mut v: tflac_uint;
    let mut samples = samples;
    let mut _i: c_int = 0;
    while _i <= 4 {
        unsafe {
            v = ((*samples.offset(0) as tflac_uint) & 0xFF) << 0;
            v |= ((*samples.offset(1) as tflac_uint) & 0xFF) << 8;
            v |= ((*samples.offset(2) as tflac_uint) & 0xFF) << 16;
            v |= ((*samples.offset(3) as tflac_uint) & 0xFF) << 24;
            v |= ((*samples.offset(4) as tflac_uint) & 0xFF) << 32;
            v |= ((*samples.offset(5) as tflac_uint) & 0xFF) << 40;
            v |= ((*samples.offset(6) as tflac_uint) & 0xFF) << 48;
            v |= ((*samples.offset(7) as tflac_uint) & 0xFF) << 56;
        }
        tflac_md5_addsample_impl(
            &mut t.md5_ctx,
            8u32.wrapping_mul(std::mem::size_of::<tflac_uint>() as u32),
            v,
        );
        b = b.wrapping_sub(step);
        // C: `samples += (8 * sizeof(tflac_s32));` — pointer arithmetic
        // advances by `8 * sizeof(tflac_s32) = 32` *elements* (not bytes),
        // i.e. 32 * 4 = 128 bytes. Preserve that behavior exactly.
        unsafe {
            samples = samples.offset((8 * std::mem::size_of::<tflac_s32>()) as isize);
        }
        _i += 1;
    }
    b
}
