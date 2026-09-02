//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI reproduced (from `nm -D` on the C shared object):
//!   * `tflac_pack_u64le`
//!   * `tflac_md5_addsample`
//!   * `update_md5`
//!
//! The C code contains several genuine defects — an out-of-bounds carry-down
//! copy in `tflac_md5_addsample`, and a `samples` pointer that advances by
//! `8 * sizeof(tflac_s32)` *elements* instead of 8 in `update_md5`. Per the
//! translation contract these are reproduced verbatim rather than fixed, so the
//! observable byte-level behaviour matches the C build exactly.
//!
//! # Why every access goes through `memcpy`
//!
//! This crate must reproduce C's *raw, unchecked* memory semantics, including
//! accesses the C makes that are out of bounds or through a null pointer. A
//! plain Rust raw-pointer dereference is not equivalent: with
//! `-C debug-assertions=on` (the default for `cargo build` / `cargo test`)
//! rustc inserts a MIR-level null check on every raw-pointer dereference, which
//! turns `update_md5(t, NULL)` into `abort()` (SIGABRT) where the C raises
//! SIGSEGV. Likewise the `core::ptr` helpers carry `assert_unsafe_precondition!`
//! checks that fire on null.
//!
//! Every access to caller-supplied memory is therefore performed with a call to
//! libc `memcpy`, which is opaque to rustc's instrumentation and faults exactly
//! where the C's load or store would. This keeps the crate's behaviour
//! identical in debug and release builds. Pointer arithmetic uses
//! `wrapping_add`, which likewise carries no preconditions.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::mem::{offset_of, size_of};

// ---------------------------------------------------------------------------
// include/lib.h
// ---------------------------------------------------------------------------

pub type tflac_u8 = u8;
pub type tflac_s32 = i32;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;

/// `typedef tflac_u64 tflac_uint;` from src/lib.c
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

// Field offsets, derived from the `#[repr(C)]` layout rather than hard-coded.
// Verified against the C ABI (probe compiled against `c_src/include/lib.h`):
//   sizeof(tflac_md5) == 88, pos@0, total@8, buffer@16
//   sizeof(tflac)     == 96, md5_ctx@0, cur_blocksize@88, channels@92
const OFF_MD5_POS: usize = offset_of!(tflac_md5, pos);
const OFF_MD5_TOTAL: usize = offset_of!(tflac_md5, total);
const OFF_MD5_BUFFER: usize = offset_of!(tflac_md5, buffer);
const OFF_T_MD5: usize = offset_of!(tflac, md5_ctx);
const OFF_T_CBS: usize = offset_of!(tflac, cur_blocksize);
const OFF_T_CH: usize = offset_of!(tflac, channels);

const _: () = {
    assert!(size_of::<tflac_md5>() == 88);
    assert!(size_of::<tflac>() == 96);
    assert!(OFF_MD5_POS == 0);
    assert!(OFF_MD5_TOTAL == 8);
    assert!(OFF_MD5_BUFFER == 16);
    assert!(OFF_T_MD5 == 0);
    assert!(OFF_T_CBS == 88);
    assert!(OFF_T_CH == 92);
};

// ---------------------------------------------------------------------------
// raw, unchecked accessors (see the module docs for why these use memcpy)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;
}

#[inline(always)]
unsafe fn ld_u8(p: *const u8) -> u8 {
    let mut v: u8 = 0;
    unsafe { memcpy(&mut v as *mut u8, p, 1) };
    v
}

#[inline(always)]
unsafe fn st_u8(p: *mut u8, v: u8) {
    unsafe { memcpy(p, &v as *const u8, 1) };
}

#[inline(always)]
unsafe fn ld_u32(p: *const u8) -> u32 {
    let mut b = [0u8; 4];
    unsafe { memcpy(b.as_mut_ptr(), p, 4) };
    u32::from_ne_bytes(b)
}

#[inline(always)]
unsafe fn st_u32(p: *mut u8, v: u32) {
    let b = v.to_ne_bytes();
    unsafe { memcpy(p, b.as_ptr(), 4) };
}

#[inline(always)]
unsafe fn ld_u64(p: *const u8) -> u64 {
    let mut b = [0u8; 8];
    unsafe { memcpy(b.as_mut_ptr(), p, 8) };
    u64::from_ne_bytes(b)
}

#[inline(always)]
unsafe fn st_u64(p: *mut u8, v: u64) {
    let b = v.to_ne_bytes();
    unsafe { memcpy(p, b.as_ptr(), 8) };
}

#[inline(always)]
unsafe fn ld_i32(p: *const u8) -> i32 {
    let mut b = [0u8; 4];
    unsafe { memcpy(b.as_mut_ptr(), p, 4) };
    i32::from_ne_bytes(b)
}

// ---------------------------------------------------------------------------
// src/lib.c
// ---------------------------------------------------------------------------

/// ```c
/// void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n) {
///     d[0] = (tflac_u8)(n);
///     d[1] = (tflac_u8)(n >> 8);
///     ...
///     d[7] = (tflac_u8)(n >> 56);
/// }
/// ```
///
/// Stores `n` as eight little-endian bytes at `d[0..8]`. The stores are emitted
/// one byte at a time, in the same order as the C, so that a partially mapped
/// destination faults after exactly the same prefix has been written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut tflac_u8, n: tflac_u64) {
    unsafe {
        st_u8(d.wrapping_add(0), n as tflac_u8);
        st_u8(d.wrapping_add(1), (n >> 8) as tflac_u8);
        st_u8(d.wrapping_add(2), (n >> 16) as tflac_u8);
        st_u8(d.wrapping_add(3), (n >> 24) as tflac_u8);
        st_u8(d.wrapping_add(4), (n >> 32) as tflac_u8);
        st_u8(d.wrapping_add(5), (n >> 40) as tflac_u8);
        st_u8(d.wrapping_add(6), (n >> 48) as tflac_u8);
        st_u8(d.wrapping_add(7), (n >> 56) as tflac_u8);
    }
}

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
/// The trailing carry-down loop is a transcription of `while (bytes--)`, whose
/// condition tests the pre-decrement value while the body uses the
/// post-decrement one; indices therefore run `pos-1 .. 0`. Because `pos` can be
/// as large as 63 while `buffer` is only 72 bytes long, `buffer[64 + bytes]`
/// reads past the end of the array. The C compiler emits a plain unchecked
/// `movzbl 0x10(%rax,%rdx,1)`, so the read is reproduced here with pointer
/// arithmetic rooted at `m` rather than at the `buffer` field.
///
/// Note that `m->total` is updated *before* anything else, so a null `m` faults
/// on the load at `m + 8`, exactly as in the C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(m: *mut tflac_md5, bits: tflac_u32, val: tflac_uint) {
    unsafe {
        let base = m as *mut u8;
        let mut bytes: tflac_u32;

        // ((m->total) += (tflac_u64)(bits));
        let total = base.wrapping_add(OFF_MD5_TOTAL);
        st_u64(total, ld_u64(total).wrapping_add(bits as tflac_u64));

        // bytes = bits / 8;
        bytes = bits / 8;

        let pos = base.wrapping_add(OFF_MD5_POS);

        // tflac_u32 pos2 = m->pos % 64;
        let pos2: tflac_u32 = ld_u32(pos) % 64;

        // &m->buffer[0], reached through the whole-struct pointer so that the
        // deliberately out-of-bounds accesses below keep `m`'s provenance.
        let buffer: *mut tflac_u8 = base.wrapping_add(OFF_MD5_BUFFER);

        // tflac_pack_u64le(&m->buffer[pos2], val);
        tflac_pack_u64le(buffer.wrapping_add(pos2 as usize), val);

        // m->pos += bytes;
        st_u32(pos, ld_u32(pos).wrapping_add(bytes));

        // if (m->pos >= 64) {
        if ld_u32(pos) >= 64 {
            // m->pos %= 64;
            st_u32(pos, ld_u32(pos) % 64);
            // bytes = m->pos;
            bytes = ld_u32(pos);
            // while (bytes--) { m->buffer[bytes] = m->buffer[64 + bytes]; }
            while bytes != 0 {
                bytes = bytes.wrapping_sub(1);
                st_u8(
                    buffer.wrapping_add(bytes as usize),
                    ld_u8(buffer.wrapping_add(64usize + bytes as usize)),
                );
            }
        }
    }
}

/// ```c
/// tflac_u32 update_md5(tflac *t, const tflac_s32 *samples) {
///     tflac_u32 b = t->cur_blocksize * t->channels;
///     const tflac_u32 step = sizeof(tflac_uint);
///     tflac_uint v;
///     for (int i = 0; i <= 4; i++) {
///         v  = (((tflac_uint)samples[0]) & 0xFF) << 0;
///         ...
///         v |= (((tflac_uint)samples[7]) & 0xFF) << 56;
///         tflac_md5_addsample(&t->md5_ctx, (8 * sizeof(tflac_uint)), v);
///         b -= step;
///         samples += (8 * sizeof(tflac_s32));
///     }
///     return b;
/// }
/// ```
///
/// Five fixed iterations. Each reads eight consecutive samples, keeps the low
/// byte of each (the `(tflac_uint)` cast sign-extends before the `& 0xFF`) and
/// packs them little-endian into a 64-bit word. `samples` then advances by
/// `8 * sizeof(tflac_s32) == 32` *elements* (128 bytes), not 8 — reproduced as
/// written, so the elements actually read are indices
/// `0..7, 32..39, 64..71, 96..103, 128..135`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    unsafe {
        let tb = t as *mut u8;

        // tflac_u32 b = t->cur_blocksize * t->channels;
        let cur_blocksize = ld_u32(tb.wrapping_add(OFF_T_CBS));
        let channels = ld_u32(tb.wrapping_add(OFF_T_CH));
        let mut b: tflac_u32 = cur_blocksize.wrapping_mul(channels);

        // const tflac_u32 step = sizeof(tflac_uint);
        const STEP: tflac_u32 = size_of::<tflac_uint>() as tflac_u32;

        // &t->md5_ctx
        let md5_ctx: *mut tflac_md5 = tb.wrapping_add(OFF_T_MD5) as *mut tflac_md5;

        // `samples += (8 * sizeof(tflac_s32))` advances by this many *elements*.
        const ELEM_STRIDE: usize = 8 * size_of::<tflac_s32>();
        const BYTE_STRIDE: usize = ELEM_STRIDE * size_of::<tflac_s32>();

        let mut sp = samples as *const u8;

        let mut i: core::ffi::c_int = 0;
        while i <= 4 {
            const E: usize = size_of::<tflac_s32>();
            let mut v: tflac_uint = ((ld_i32(sp.wrapping_add(0 * E)) as tflac_uint) & 0xFF) << 0;
            v |= ((ld_i32(sp.wrapping_add(1 * E)) as tflac_uint) & 0xFF) << 8;
            v |= ((ld_i32(sp.wrapping_add(2 * E)) as tflac_uint) & 0xFF) << 16;
            v |= ((ld_i32(sp.wrapping_add(3 * E)) as tflac_uint) & 0xFF) << 24;
            v |= ((ld_i32(sp.wrapping_add(4 * E)) as tflac_uint) & 0xFF) << 32;
            v |= ((ld_i32(sp.wrapping_add(5 * E)) as tflac_uint) & 0xFF) << 40;
            v |= ((ld_i32(sp.wrapping_add(6 * E)) as tflac_uint) & 0xFF) << 48;
            v |= ((ld_i32(sp.wrapping_add(7 * E)) as tflac_uint) & 0xFF) << 56;

            // tflac_md5_addsample(&t->md5_ctx, (8 * sizeof(tflac_uint)), v);
            tflac_md5_addsample(md5_ctx, (8 * size_of::<tflac_uint>()) as tflac_u32, v);

            // b -= step;
            b = b.wrapping_sub(STEP);

            // samples += (8 * sizeof(tflac_s32));
            sp = sp.wrapping_add(BYTE_STRIDE);

            i += 1;
        }

        b
    }
}
