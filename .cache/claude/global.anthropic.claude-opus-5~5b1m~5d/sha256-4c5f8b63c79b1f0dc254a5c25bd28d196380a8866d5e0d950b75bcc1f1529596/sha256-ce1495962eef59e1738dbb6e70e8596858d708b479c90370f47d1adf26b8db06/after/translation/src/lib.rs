//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared object):
//!   * `tflac_pack_u64le`
//!   * `tflac_md5_addsample`
//!   * `update_md5`
//!
//! Behaviour — including the quirks/bugs of the original C — is reproduced
//! exactly: the sample-pointer stride in `update_md5` really does advance by
//! `8 * sizeof(tflac_s32)` *elements* (32), the loop really does run five
//! times (`i <= 4`), and the byte counter `b` is decremented by
//! `sizeof(tflac_uint)` (8) each iteration with unsigned wraparound.

#![allow(non_camel_case_types)]

/* ------------------------------------------------------------------ */
/* Typedefs mirroring lib.h                                            */
/* ------------------------------------------------------------------ */

pub type tflac_u8 = u8;
pub type tflac_s32 = i32;
pub type tflac_u32 = u32;
pub type tflac_u64 = u64;

/// `typedef tflac_u64 tflac_uint;` from lib.c
pub type tflac_uint = tflac_u64;

/// Size of the MD5 scratch buffer: `tflac_u8 buffer[64 + 8]`.
const TFLAC_MD5_BUFFER_LEN: usize = 64 + 8;

/// `struct tflac_md5` — verified layout: size 88, pos@0, total@8, buffer@16.
#[repr(C)]
pub struct tflac_md5 {
    pub pos: tflac_u32,
    pub total: tflac_u64,
    pub buffer: [tflac_u8; TFLAC_MD5_BUFFER_LEN],
}

/// `struct tflac` — verified layout: size 96, md5_ctx@0, cur_blocksize@88,
/// channels@92.
#[repr(C)]
pub struct tflac {
    pub md5_ctx: tflac_md5,
    pub cur_blocksize: tflac_u32,
    pub channels: tflac_u32,
}

/* ------------------------------------------------------------------ */
/* Raw memory access primitives                                        */
/* ------------------------------------------------------------------ */
//
// Every load/store below is performed as a *byte-granular volatile* access,
// which is what the C compiler emits for these functions and, crucially, what
// gives the translation the same observable behaviour as the C for the two
// pointer situations the C does not guard against:
//
//   * a NULL (or otherwise unmapped) pointer must fault with SIGSEGV exactly
//     like the C does — a plain `*p = v` would instead trip Rust's debug-only
//     "null pointer dereference" check and abort with SIGABRT;
//   * a *misaligned* `tflac`/`tflac_md5` pointer must still work, as it does
//     in C on this target — a plain `*(p as *mut u64)` would trip Rust's
//     debug-only "misaligned pointer dereference" check and abort.
//
// Byte-granular access is also endian-neutral: `from_ne_bytes`/`to_ne_bytes`
// reproduce the native multi-byte load/store the C performs on any target.
// All arithmetic uses `wrapping_add` so no pointer-offset precondition check
// can fire where the C simply computes an address.

#[inline(always)]
unsafe fn ld_u8(p: *const tflac_u8) -> tflac_u8 {
    p.read_volatile()
}

#[inline(always)]
unsafe fn st_u8(p: *mut tflac_u8, v: tflac_u8) {
    p.write_volatile(v)
}

#[inline(always)]
unsafe fn ld_u32(p: *const tflac_u8) -> tflac_u32 {
    let mut b = [0u8; 4];
    let mut i = 0usize;
    while i < 4 {
        b[i] = ld_u8(p.wrapping_add(i));
        i += 1;
    }
    tflac_u32::from_ne_bytes(b)
}

#[inline(always)]
unsafe fn st_u32(p: *mut tflac_u8, v: tflac_u32) {
    let b = v.to_ne_bytes();
    let mut i = 0usize;
    while i < 4 {
        st_u8(p.wrapping_add(i), b[i]);
        i += 1;
    }
}

#[inline(always)]
unsafe fn ld_u64(p: *const tflac_u8) -> tflac_u64 {
    let mut b = [0u8; 8];
    let mut i = 0usize;
    while i < 8 {
        b[i] = ld_u8(p.wrapping_add(i));
        i += 1;
    }
    tflac_u64::from_ne_bytes(b)
}

#[inline(always)]
unsafe fn st_u64(p: *mut tflac_u8, v: tflac_u64) {
    let b = v.to_ne_bytes();
    let mut i = 0usize;
    while i < 8 {
        st_u8(p.wrapping_add(i), b[i]);
        i += 1;
    }
}

/// Load one `tflac_s32` (`int32_t`) the way the C `samples[k]` load does.
#[inline(always)]
unsafe fn ld_s32(p: *const tflac_s32) -> tflac_s32 {
    ld_u32(p as *const tflac_u8) as tflac_s32
}

/* ------------------------------------------------------------------ */
/* tflac_pack_u64le                                                    */
/* ------------------------------------------------------------------ */

/// ```c
/// void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n) {
///     d[0] = (tflac_u8)(n);
///     d[1] = (tflac_u8)(n >> 8);
///     ...
///     d[7] = (tflac_u8)(n >> 56);
/// }
/// ```
///
/// Stores `n` into `d[0..8]` in little-endian byte order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_pack_u64le(d: *mut tflac_u8, n: tflac_u64) {
    st_u8(d.wrapping_add(0), n as tflac_u8);
    st_u8(d.wrapping_add(1), (n >> 8) as tflac_u8);
    st_u8(d.wrapping_add(2), (n >> 16) as tflac_u8);
    st_u8(d.wrapping_add(3), (n >> 24) as tflac_u8);
    st_u8(d.wrapping_add(4), (n >> 32) as tflac_u8);
    st_u8(d.wrapping_add(5), (n >> 40) as tflac_u8);
    st_u8(d.wrapping_add(6), (n >> 48) as tflac_u8);
    st_u8(d.wrapping_add(7), (n >> 56) as tflac_u8);
}

/* ------------------------------------------------------------------ */
/* tflac_md5_addsample                                                 */
/* ------------------------------------------------------------------ */

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
/// Every operation is performed in the C's order, including the facts that
/// `pos2` is derived from the *old* `pos`, that `m->pos` is written before the
/// `>= 64` test, and that `bytes` is reused as the spill counter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tflac_md5_addsample(
    m: *mut tflac_md5,
    bits: tflac_u32,
    val: tflac_uint,
) {
    let mut bytes: tflac_u32;

    let p_pos = core::ptr::addr_of_mut!((*m).pos) as *mut tflac_u8;
    let p_total = core::ptr::addr_of_mut!((*m).total) as *mut tflac_u8;
    let buffer = core::ptr::addr_of_mut!((*m).buffer) as *mut tflac_u8;

    // ((m->total) += (tflac_u64)(bits));
    st_u64(p_total, ld_u64(p_total).wrapping_add(bits as tflac_u64));

    // bytes = bits / 8;
    bytes = bits / 8;

    // tflac_u32 pos2 = m->pos % 64;
    let pos2 = ld_u32(p_pos) % 64;

    // tflac_pack_u64le(&m->buffer[pos2], val);
    tflac_pack_u64le(buffer.wrapping_add(pos2 as usize), val);

    // m->pos += bytes;   (unsigned 32-bit wraparound)
    let mut pos = ld_u32(p_pos).wrapping_add(bytes);
    st_u32(p_pos, pos);

    // if (m->pos >= 64) { ... }
    if pos >= 64 {
        // m->pos %= 64;
        pos %= 64;
        st_u32(p_pos, pos);

        // bytes = m->pos;
        bytes = pos;

        // while (bytes--) m->buffer[bytes] = m->buffer[64 + bytes];
        //
        // `bytes` is unsigned, so a zero counter skips the loop body entirely
        // (the post-decrement wraparound is never observed).
        while bytes != 0 {
            bytes -= 1;
            st_u8(
                buffer.wrapping_add(bytes as usize),
                ld_u8(buffer.wrapping_add(64usize.wrapping_add(bytes as usize))),
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/* update_md5                                                          */
/* ------------------------------------------------------------------ */

/// ```c
/// tflac_u32 update_md5(tflac *t, const tflac_s32 *samples);
/// ```
///
/// Packs eight sample low-bytes into one 64-bit word, feeds it to
/// `tflac_md5_addsample`, and repeats five times (`for (int i = 0; i <= 4;
/// i++)`), returning the remaining byte count.
///
/// Faithful details carried over from the C:
///   * `b` starts as `cur_blocksize * channels` (unsigned 32-bit wraparound).
///   * Each iteration subtracts `step == sizeof(tflac_uint) == 8` from `b`,
///     wrapping on underflow.
///   * `samples` advances by `8 * sizeof(tflac_s32) == 32` *elements* per
///     iteration (pointer arithmetic already scales by the element size), so
///     only the first 8 of every 32 samples are consumed; the reads therefore
///     land on `samples[0..8]`, `[32..40]`, `[64..72]`, `[96..104]`,
///     `[128..136]`.
///   * Each sample is sign-extended to 64 bits and then masked with `0xFF`,
///     i.e. only its lowest byte survives.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_md5(t: *mut tflac, samples: *const tflac_s32) -> tflac_u32 {
    // tflac_u32 b = t->cur_blocksize * t->channels;
    let cur_blocksize = ld_u32(core::ptr::addr_of!((*t).cur_blocksize) as *const tflac_u8);
    let channels = ld_u32(core::ptr::addr_of!((*t).channels) as *const tflac_u8);
    let mut b: tflac_u32 = cur_blocksize.wrapping_mul(channels);

    // const tflac_u32 step = sizeof(tflac_uint);
    let step: tflac_u32 = core::mem::size_of::<tflac_uint>() as tflac_u32;

    let mut samples = samples;
    let md5_ctx = core::ptr::addr_of_mut!((*t).md5_ctx);

    let mut i: i32 = 0;
    while i <= 4 {
        // Sign-extend to tflac_uint (u64) then keep the low byte, exactly as
        // `(((tflac_uint)samples[k]) & 0xFF) << (8 * k)` does in C.
        let s = |k: usize| -> tflac_uint {
            (ld_s32(samples.wrapping_add(k)) as i64 as tflac_uint) & 0xFF
        };

        let mut v: tflac_uint = s(0) << 0;
        v |= s(1) << 8;
        v |= s(2) << 16;
        v |= s(3) << 24;
        v |= s(4) << 32;
        v |= s(5) << 40;
        v |= s(6) << 48;
        v |= s(7) << 56;

        // tflac_md5_addsample(&t->md5_ctx, (8 * sizeof(tflac_uint)), v);
        tflac_md5_addsample(
            md5_ctx,
            (8 * core::mem::size_of::<tflac_uint>()) as tflac_u32,
            v,
        );

        // b -= step;
        b = b.wrapping_sub(step);

        // samples += (8 * sizeof(tflac_s32));  /* == 32 elements */
        samples = samples.wrapping_add(8 * core::mem::size_of::<tflac_s32>());

        i += 1;
    }

    b
}

/* ------------------------------------------------------------------ */
/* Compile-time ABI assertions                                         */
/* ------------------------------------------------------------------ */

const _: () = {
    // Layouts verified against the C compiler on this platform
    // (`sizeof`/`_Alignof`/`offsetof`).
    assert!(core::mem::size_of::<tflac_md5>() == 88);
    assert!(core::mem::align_of::<tflac_md5>() == 8);
    assert!(core::mem::size_of::<tflac>() == 96);
    assert!(core::mem::align_of::<tflac>() == 8);
    assert!(core::mem::size_of::<tflac_uint>() == 8);
    assert!(core::mem::size_of::<tflac_s32>() == 4);
};
