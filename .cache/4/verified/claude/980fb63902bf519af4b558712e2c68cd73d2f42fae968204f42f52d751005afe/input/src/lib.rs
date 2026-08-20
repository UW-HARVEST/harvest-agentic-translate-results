//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (exactly matching the C shared object's dynamic symbols):
//!   * `tflac_pack_u64le`
//!   * `tflac_md5_addsample`
//!   * `update_md5`
//!
//! The C semantics (including wrapping arithmetic and the unchecked buffer
//! indexing performed by `tflac_md5_addsample`) are reproduced exactly; no
//! bugs of the original code are "fixed".

#![allow(non_camel_case_types)]

use core::ffi::c_int;

// ---------------------------------------------------------------------------
// Typedefs from lib.h
// ---------------------------------------------------------------------------

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;
/// `typedef int32_t tflac_s32;`
pub type tflac_s32 = i32;
/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;
/// `typedef uint64_t tflac_u64;`
pub type tflac_u64 = u64;

/// `typedef tflac_u64 tflac_uint;` (from lib.c)
pub type tflac_uint = tflac_u64;

// ---------------------------------------------------------------------------
// Structs from lib.h
// ---------------------------------------------------------------------------

/// ```c
/// struct tflac_md5 {
///     tflac_u32 pos;
///     tflac_u64 total;
///     tflac_u8 buffer[64 + 8];
/// };
/// ```
/// Layout (x86-64 SysV): size 88, pos @0, total @8, buffer @16.
#[repr(C)]
pub struct tflac_md5 {
    pub pos: tflac_u32,
    pub total: tflac_u64,
    pub buffer: [tflac_u8; 64 + 8],
}

/// ```c
/// struct tflac {
///     tflac_md5 md5_ctx;
///     tflac_u32 cur_blocksize;
///     tflac_u32 channels;
/// };
/// ```
/// Layout (x86-64 SysV): size 96, md5_ctx @0, cur_blocksize @88, channels @92.
#[repr(C)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: tflac_u32,
    pub channels: tflac_u32,
}

// ---------------------------------------------------------------------------
// void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n)
// ---------------------------------------------------------------------------

/// ```c
/// void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n) {
///     d[0] = (tflac_u8)(n);
///     d[1] = (tflac_u8)(n >> 8);
///     ...
///     d[7] = (tflac_u8)(n >> 56);
/// }
/// ```
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

// ---------------------------------------------------------------------------
// void tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_uint val)
// ---------------------------------------------------------------------------

/// ```c
/// void tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_uint val) {
///     tflac_u32 bytes;
///     ((m->total) += (tflac_u64)(bits));
///     bytes = bits / 8;
///     tflac_u32 pos2 = m->pos % 64;
///     tflac_pack_u64le(&m->buffer[pos2], val);
///     m->pos += bytes;
///     if (m->pos >= 64) {
///         m->pos %= 64;
///         bytes = m->pos;
///         while (bytes--) {
///             m->buffer[bytes] = m->buffer[64 + bytes];
///         }
///     }
/// }
/// ```
///
/// Note: the trailing copy loop indexes `buffer[64 + bytes]` with
/// `bytes` up to 62, which can read past the 72-byte buffer.  That behaviour is
/// part of the original code and is reproduced here verbatim using raw
/// pointer arithmetic (byte-wise, exactly as the C compiler emits it).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(m: *mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    unsafe {
        let mut bytes: tflac_u32;

        // m->total += (tflac_u64)bits;
        let total = core::ptr::addr_of_mut!((*m).total);
        core::ptr::write_unaligned(total, core::ptr::read_unaligned(total).wrapping_add(bits as tflac_u64));

        bytes = bits / 8;

        let pos_ptr = core::ptr::addr_of_mut!((*m).pos);
        let buffer: *mut tflac_u8 = core::ptr::addr_of_mut!((*m).buffer) as *mut tflac_u8;

        // tflac_u32 pos2 = m->pos % 64;
        let pos2 = core::ptr::read_unaligned(pos_ptr) % 64;

        // tflac_pack_u64le(&m->buffer[pos2], val);
        tflac_pack_u64le(buffer.add(pos2 as usize), val);

        // m->pos += bytes;
        core::ptr::write_unaligned(
            pos_ptr,
            core::ptr::read_unaligned(pos_ptr).wrapping_add(bytes),
        );

        if core::ptr::read_unaligned(pos_ptr) >= 64 {
            // m->pos %= 64;
            let p = core::ptr::read_unaligned(pos_ptr) % 64;
            core::ptr::write_unaligned(pos_ptr, p);

            // bytes = m->pos;
            bytes = p;

            // while (bytes--) { m->buffer[bytes] = m->buffer[64 + bytes]; }
            while bytes != 0 {
                bytes -= 1;
                let v = core::ptr::read_unaligned(buffer.add(64usize + bytes as usize));
                core::ptr::write_unaligned(buffer.add(bytes as usize), v);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// tflac_u32 update_md5(tflac *t, const tflac_s32 *samples)
// ---------------------------------------------------------------------------

/// ```c
/// tflac_u32 update_md5(tflac *t, const tflac_s32 *samples) {
///     tflac_u32 b = t->cur_blocksize * t->channels;
///     const tflac_u32 step = sizeof(tflac_uint);
///     tflac_uint v;
///     for (int i = 0; i <= 4; i++) {
///         v  = (((tflac_uint)samples[0]) & 0xFF) << 0;
///         v |= (((tflac_uint)samples[1]) & 0xFF) << 8;
///         ...
///         v |= (((tflac_uint)samples[7]) & 0xFF) << 56;
///         tflac_md5_addsample(&t->md5_ctx, (8 * sizeof(tflac_uint)), v);
///         b -= step;
///         samples += (8 * sizeof(tflac_s32));
///     }
///     return b;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    unsafe {
        // tflac_u32 b = t->cur_blocksize * t->channels;  (unsigned wraparound)
        let cur_blocksize = core::ptr::read_unaligned(core::ptr::addr_of!((*t).cur_blocksize));
        let channels = core::ptr::read_unaligned(core::ptr::addr_of!((*t).channels));
        let mut b: tflac_u32 = cur_blocksize.wrapping_mul(channels);

        // const tflac_u32 step = sizeof(tflac_uint);  /* == 8 */
        let step: tflac_u32 = core::mem::size_of::<tflac_uint>() as tflac_u32;

        let md5_ctx: *mut tflac_md5 = core::ptr::addr_of_mut!((*t).md5_ctx);

        let mut samples = samples;
        let mut i: c_int = 0;
        while i <= 4 {
            let mut v: tflac_uint = ((core::ptr::read_unaligned(samples.add(0)) as tflac_uint) & 0xFF) << 0;
            v |= ((core::ptr::read_unaligned(samples.add(1)) as tflac_uint) & 0xFF) << 8;
            v |= ((core::ptr::read_unaligned(samples.add(2)) as tflac_uint) & 0xFF) << 16;
            v |= ((core::ptr::read_unaligned(samples.add(3)) as tflac_uint) & 0xFF) << 24;
            v |= ((core::ptr::read_unaligned(samples.add(4)) as tflac_uint) & 0xFF) << 32;
            v |= ((core::ptr::read_unaligned(samples.add(5)) as tflac_uint) & 0xFF) << 40;
            v |= ((core::ptr::read_unaligned(samples.add(6)) as tflac_uint) & 0xFF) << 48;
            v |= ((core::ptr::read_unaligned(samples.add(7)) as tflac_uint) & 0xFF) << 56;

            // 8 * sizeof(tflac_uint) == 64
            tflac_md5_addsample(md5_ctx, 8u32.wrapping_mul(core::mem::size_of::<tflac_uint>() as tflac_u32), v);

            b = b.wrapping_sub(step);

            // samples += 8 * sizeof(tflac_s32);  /* 32 elements */
            samples = samples.add(8usize * core::mem::size_of::<tflac_s32>());

            i += 1;
        }

        b
    }
}
