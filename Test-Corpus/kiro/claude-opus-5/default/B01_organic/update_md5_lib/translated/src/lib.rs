//! Rust translation of `c_src/src/lib.c` (tflac md5 sample-accumulation helpers).
//!
//! The translation is intentionally literal: integer widths, wrapping behaviour,
//! evaluation order and the (buggy) pointer stride in `update_md5` are all
//! reproduced exactly as written in the C source.

use std::ffi::c_int;
use std::mem::offset_of;

pub type TflacU8 = u8;
pub type TflacS32 = i32;
pub type TflacU32 = u32;
pub type TflacU64 = u64;

/// `typedef tflac_u64 tflac_uint;`
pub type TflacUint = TflacU64;

/// `sizeof(tflac_uint)` == 8
const SIZEOF_TFLAC_UINT: TflacU32 = 8;
/// `sizeof(tflac_s32)` == 4
const SIZEOF_TFLAC_S32: usize = 4;

#[repr(C)]
pub struct TflacMd5 {
    pub pos: TflacU32,
    pub total: TflacU64,
    pub buffer: [TflacU8; 64 + 8],
}

#[repr(C)]
pub struct Tflac {
    pub md5_ctx: TflacMd5,
    pub cur_blocksize: TflacU32,
    pub channels: TflacU32,
}

// Layout must match the C structs exactly (verified against gcc x86-64):
// tflac_md5 = 88 bytes {pos@0, total@8, buffer@16}, tflac = 96 bytes.
const _: () = {
    assert!(size_of::<TflacMd5>() == 88);
    assert!(align_of::<TflacMd5>() == 8);
    assert!(size_of::<Tflac>() == 96);
    assert!(offset_of!(TflacMd5, total) == 8);
    assert!(offset_of!(TflacMd5, buffer) == 16);
    assert!(offset_of!(Tflac, cur_blocksize) == 88);
    assert!(offset_of!(Tflac, channels) == 92);
};

/// ```c
/// void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut TflacU8, n: TflacU64) {
    unsafe {
        *d.add(0) = n as TflacU8;
        *d.add(1) = (n >> 8) as TflacU8;
        *d.add(2) = (n >> 16) as TflacU8;
        *d.add(3) = (n >> 24) as TflacU8;
        *d.add(4) = (n >> 32) as TflacU8;
        *d.add(5) = (n >> 40) as TflacU8;
        *d.add(6) = (n >> 48) as TflacU8;
        *d.add(7) = (n >> 56) as TflacU8;
    }
}

/// ```c
/// void tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_uint val);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(m: *mut TflacMd5, bits: TflacU32, val: TflacUint) {
    unsafe {
        let total = &raw mut (*m).total;
        let pos = &raw mut (*m).pos;
        let buf: *mut TflacU8 = (&raw mut (*m).buffer).cast();

        *total = (*total).wrapping_add(bits as TflacU64);

        let mut bytes: TflacU32 = bits / 8;

        let pos2: TflacU32 = *pos % 64;
        tflac_pack_u64le(buf.add(pos2 as usize), val);

        *pos = (*pos).wrapping_add(bytes);
        if *pos >= 64 {
            *pos %= 64;
            bytes = *pos;
            // `while (bytes--)` runs for bytes-1 ..= 0.
            // Raw pointer accesses mirror the C byte-for-byte, including the
            // out-of-bounds accesses that are possible when the incoming
            // `pos` was greater than 63.
            while bytes != 0 {
                bytes -= 1;
                *buf.add(bytes as usize) = *buf.add(64 + bytes as usize);
            }
        }
    }
}

/// ```c
/// tflac_u32 update_md5(tflac *t, const tflac_s32 *samples);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut Tflac, samples: *const TflacS32) -> TflacU32 {
    unsafe {
        let mut b: TflacU32 = (*t).cur_blocksize.wrapping_mul((*t).channels);
        let step: TflacU32 = SIZEOF_TFLAC_UINT;
        let mut v: TflacUint;

        let mut samples = samples;

        let mut i: c_int = 0;
        while i <= 4 {
            v = ((*samples.add(0) as TflacUint) & 0xFF) << 0;
            v |= ((*samples.add(1) as TflacUint) & 0xFF) << 8;
            v |= ((*samples.add(2) as TflacUint) & 0xFF) << 16;
            v |= ((*samples.add(3) as TflacUint) & 0xFF) << 24;
            v |= ((*samples.add(4) as TflacUint) & 0xFF) << 32;
            v |= ((*samples.add(5) as TflacUint) & 0xFF) << 40;
            v |= ((*samples.add(6) as TflacUint) & 0xFF) << 48;
            v |= ((*samples.add(7) as TflacUint) & 0xFF) << 56;

            tflac_md5_addsample(&raw mut (*t).md5_ctx, 8 * SIZEOF_TFLAC_UINT, v);

            b = b.wrapping_sub(step);

            // C: `samples += (8 * sizeof(tflac_s32));` -> advances 32 elements.
            samples = samples.add(8 * SIZEOF_TFLAC_S32);

            i += 1;
        }

        b
    }
}
