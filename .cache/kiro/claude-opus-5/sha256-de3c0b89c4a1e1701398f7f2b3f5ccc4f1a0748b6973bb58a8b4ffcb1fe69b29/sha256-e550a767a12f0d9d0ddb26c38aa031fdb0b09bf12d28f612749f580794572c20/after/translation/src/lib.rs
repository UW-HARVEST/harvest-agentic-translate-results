//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI reproduced (from `nm -D` on the C shared object):
//!   * `tflac_pack_u64le`
//!   * `tflac_md5_addsample`
//!   * `update_md5`
//!
//! The C code contains several genuine defects (an out-of-bounds carry-down
//! copy in `tflac_md5_addsample`, and a `samples` pointer that advances by
//! `8 * sizeof(tflac_s32)` *elements* instead of 8 in `update_md5`). Per the
//! translation contract these are reproduced verbatim rather than fixed, so
//! the observable byte-level behaviour matches the C build exactly.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// include/lib.h
// ---------------------------------------------------------------------------

pub type tflac_u8 = u8;
pub type tflac_s32 = i32;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;

/// `typedef tflac_u64 tflac_uint;` from src/lib.c
type tflac_uint = tflac_u64;

/// Byte offset of `tflac_md5::buffer` within `struct tflac_md5`.
///
/// Verified against the C ABI: `sizeof(tflac_md5) == 88`,
/// `offsetof(pos) == 0`, `offsetof(total) == 8`, `offsetof(buffer) == 16`.
const MD5_BUFFER_OFFSET: usize = 16;

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

// ---------------------------------------------------------------------------
// src/lib.c
// ---------------------------------------------------------------------------

/// ```c
/// void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n);
/// ```
///
/// Stores `n` as eight little-endian bytes at `d[0..8]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut tflac_u8, n: tflac_u64) {
    unsafe {
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

/// ```c
/// void tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_uint val);
/// ```
///
/// The trailing carry-down loop is a transcription of `while (bytes--)`, whose
/// condition tests the pre-decrement value while the body uses the
/// post-decrement one; indices therefore run `pos-1 .. 0`. Because `pos` can be
/// as large as 63 while `buffer` is only 72 bytes long, `buffer[64 + bytes]`
/// reads past the end of the array. The C compiler emits a plain unchecked
/// `movzbl 0x10(%rax,%rdx,1)`, so the read is reproduced here with raw pointer
/// arithmetic rooted at `m` rather than at the `buffer` field.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(m: *mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    unsafe {
        let mut bytes: tflac_u32;

        // ((m->total) += (tflac_u64)(bits));
        let total = core::ptr::addr_of_mut!((*m).total);
        core::ptr::write_unaligned(
            total,
            core::ptr::read_unaligned(total).wrapping_add(bits as tflac_u64),
        );

        // bytes = bits / 8;
        bytes = bits / 8;

        let pos = core::ptr::addr_of_mut!((*m).pos);

        // tflac_u32 pos2 = m->pos % 64;
        let pos2: tflac_u32 = core::ptr::read_unaligned(pos) % 64;

        // &m->buffer[0], reached through the whole-struct pointer so that the
        // deliberately out-of-bounds accesses below keep `m`'s provenance.
        let buffer: *mut tflac_u8 = (m as *mut tflac_u8).add(MD5_BUFFER_OFFSET);

        // tflac_pack_u64le(&m->buffer[pos2], val);
        tflac_pack_u64le(buffer.add(pos2 as usize), val);

        // m->pos += bytes;
        core::ptr::write_unaligned(pos, core::ptr::read_unaligned(pos).wrapping_add(bytes));

        // if (m->pos >= 64) {
        if core::ptr::read_unaligned(pos) >= 64 {
            // m->pos %= 64;
            core::ptr::write_unaligned(pos, core::ptr::read_unaligned(pos) % 64);
            // bytes = m->pos;
            bytes = core::ptr::read_unaligned(pos);
            // while (bytes--) { m->buffer[bytes] = m->buffer[64 + bytes]; }
            while bytes != 0 {
                bytes = bytes.wrapping_sub(1);
                *buffer.add(bytes as usize) = *buffer.add(64usize + bytes as usize);
            }
        }
    }
}

/// ```c
/// tflac_u32 update_md5(tflac *t, const tflac_s32 *samples);
/// ```
///
/// Five fixed iterations. Each reads eight consecutive samples, keeps the low
/// byte of each (the `(tflac_uint)` cast sign-extends before the `& 0xFF`) and
/// packs them little-endian into a 64-bit word. `samples` then advances by
/// `8 * sizeof(tflac_s32) == 32` *elements*, not 8 — reproduced as written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    unsafe {
        let mut samples = samples;

        // tflac_u32 b = t->cur_blocksize * t->channels;
        let cur_blocksize = core::ptr::read_unaligned(core::ptr::addr_of!((*t).cur_blocksize));
        let channels = core::ptr::read_unaligned(core::ptr::addr_of!((*t).channels));
        let mut b: tflac_u32 = cur_blocksize.wrapping_mul(channels);

        // const tflac_u32 step = sizeof(tflac_uint);
        const STEP: tflac_u32 = core::mem::size_of::<tflac_uint>() as tflac_u32;

        let md5_ctx: *mut tflac_md5 = core::ptr::addr_of_mut!((*t).md5_ctx);

        let mut i: core::ffi::c_int = 0;
        while i <= 4 {
            let mut v: tflac_uint = ((*samples.add(0) as tflac_uint) & 0xFF) << 0;
            v |= ((*samples.add(1) as tflac_uint) & 0xFF) << 8;
            v |= ((*samples.add(2) as tflac_uint) & 0xFF) << 16;
            v |= ((*samples.add(3) as tflac_uint) & 0xFF) << 24;
            v |= ((*samples.add(4) as tflac_uint) & 0xFF) << 32;
            v |= ((*samples.add(5) as tflac_uint) & 0xFF) << 40;
            v |= ((*samples.add(6) as tflac_uint) & 0xFF) << 48;
            v |= ((*samples.add(7) as tflac_uint) & 0xFF) << 56;

            // tflac_md5_addsample(&t->md5_ctx, (8 * sizeof(tflac_uint)), v);
            tflac_md5_addsample(
                md5_ctx,
                (8 * core::mem::size_of::<tflac_uint>()) as tflac_u32,
                v,
            );

            // b -= step;
            b = b.wrapping_sub(STEP);

            // samples += (8 * sizeof(tflac_s32));
            samples = samples.add(8 * core::mem::size_of::<tflac_s32>());

            i += 1;
        }

        b
    }
}
