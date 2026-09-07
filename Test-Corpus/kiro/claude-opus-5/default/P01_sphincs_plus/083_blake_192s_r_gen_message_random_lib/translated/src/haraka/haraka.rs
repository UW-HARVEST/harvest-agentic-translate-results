//! Second half (lines 679-965) of `lib/haraka/src/haraka.c`.
use core::ffi::{c_uint, c_ulonglong};
use std::vec::Vec;
use crate::context::SpxCtx;
use crate::params::*;
use super::aes_ct::*;

unsafe fn interleave_constant(out: *mut u64, in_: *const u8) {
    let mut tmp_32_constant: [u32; 16] = [0; 16];
    let mut i: i32;

    br_range_dec32le(tmp_32_constant.as_mut_ptr(), 16, in_);
    i = 0;
    while i < 4 {
        br_aes_ct64_interleave_in(
            out.add(i as usize),
            out.add((i + 4) as usize),
            tmp_32_constant.as_ptr().add((i << 2) as usize),
        );
        i += 1;
    }
    br_aes_ct64_ortho(out);
}

unsafe fn interleave_constant32(out: *mut u32, in_: *const u8) {
    let mut i: i32;
    i = 0;
    while i < 4 {
        *out.add((2 * i) as usize) = br_dec32le(in_.add((4 * i) as usize));
        *out.add((2 * i + 1) as usize) = br_dec32le(in_.add((4 * i + 16) as usize));
        i += 1;
    }
    br_aes_ct_ortho(out);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_tweak_constants(ctx: *mut SpxCtx) {
    let mut buf: [u8; 40 * 16] = [0; 40 * 16];
    let mut i: i32;

    /* Use the standard constants to generate tweaked ones. */
    core::ptr::copy_nonoverlapping(
        HARAKA512_RC64.as_ptr() as *const u8,
        (*ctx).tweaked512_rc64.as_mut_ptr() as *mut u8,
        40 * 16,
    );

    /* Constants for pk.seed */
    SPX_haraka_S(
        buf.as_mut_ptr(),
        (40 * 16) as c_ulonglong,
        (*ctx).pub_seed.as_ptr(),
        SPX_N as c_ulonglong,
        ctx,
    );
    i = 0;
    while i < 10 {
        interleave_constant32(
            (*ctx).tweaked256_rc32[i as usize].as_mut_ptr(),
            buf.as_ptr().add((32 * i) as usize),
        );
        interleave_constant(
            (*ctx).tweaked512_rc64[i as usize].as_mut_ptr(),
            buf.as_ptr().add((64 * i) as usize),
        );
        i += 1;
    }
}

unsafe fn haraka_S_absorb(
    s: *mut u8,
    r: c_uint,
    mut m: *const u8,
    mut mlen: c_ulonglong,
    p: u8,
    ctx: *const SpxCtx,
) {
    let mut i: c_ulonglong;
    let mut t: Vec<u8> = vec![0u8; r as usize];

    while mlen >= r as c_ulonglong {
        /* XOR block to state */
        i = 0;
        while i < r as c_ulonglong {
            *s.add(i as usize) ^= *m.add(i as usize);
            i += 1;
        }
        SPX_haraka512_perm(s, s, ctx);
        mlen -= r as c_ulonglong;
        m = m.add(r as usize);
    }

    i = 0;
    while i < r as c_ulonglong {
        t[i as usize] = 0;
        i += 1;
    }
    i = 0;
    while i < mlen {
        t[i as usize] = *m.add(i as usize);
        i += 1;
    }
    t[i as usize] = p;
    t[(r - 1) as usize] |= 128;
    i = 0;
    while i < r as c_ulonglong {
        *s.add(i as usize) ^= t[i as usize];
        i += 1;
    }
}

unsafe fn haraka_S_squeezeblocks(
    mut h: *mut u8,
    mut nblocks: c_ulonglong,
    s: *mut u8,
    r: c_uint,
    ctx: *const SpxCtx,
) {
    while nblocks > 0 {
        SPX_haraka512_perm(s, s, ctx);
        core::ptr::copy_nonoverlapping(s, h, HARAKAS_RATE);
        h = h.add(r as usize);
        nblocks -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_init(s_inc: *mut u8) {
    let mut i: usize;

    i = 0;
    while i < 64 {
        *s_inc.add(i) = 0;
        i += 1;
    }
    *s_inc.add(64) = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_absorb(
    s_inc: *mut u8,
    mut m: *const u8,
    mut mlen: usize,
    ctx: *const SpxCtx,
) {
    let mut i: usize;

    /* Recall that s_inc[64] is the non-absorbed bytes xored into the state */
    while mlen + (*s_inc.add(64)) as usize >= HARAKAS_RATE {
        i = 0;
        while i < (HARAKAS_RATE - (*s_inc.add(64)) as usize) {
            /* Take the i'th byte from message
               xor with the s_inc[64] + i'th byte of the state */
            *s_inc.add((*s_inc.add(64)) as usize + i) ^= *m.add(i);
            i += 1;
        }
        mlen -= HARAKAS_RATE - (*s_inc.add(64)) as usize;
        m = m.add(HARAKAS_RATE - (*s_inc.add(64)) as usize);
        *s_inc.add(64) = 0;

        SPX_haraka512_perm(s_inc, s_inc, ctx);
    }

    i = 0;
    while i < mlen {
        *s_inc.add((*s_inc.add(64)) as usize + i) ^= *m.add(i);
        i += 1;
    }
    *s_inc.add(64) = (*s_inc.add(64)).wrapping_add(mlen as u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_finalize(s_inc: *mut u8) {
    /* After haraka_S_inc_absorb, we are guaranteed that s_inc[64] < HARAKAS_RATE,
       so we can always use one more byte for p in the current state. */
    *s_inc.add((*s_inc.add(64)) as usize) ^= 0x1F;
    *s_inc.add(HARAKAS_RATE - 1) ^= 128;
    *s_inc.add(64) = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_squeeze(
    mut out: *mut u8,
    mut outlen: usize,
    s_inc: *mut u8,
    ctx: *const SpxCtx,
) {
    let mut i: usize;

    /* First consume any bytes we still have sitting around */
    i = 0;
    while i < outlen && i < (*s_inc.add(64)) as usize {
        /* There are s_inc[64] bytes left, so r - s_inc[64] is the first
           available byte. We consume from there, i.e., up to r. */
        *out.add(i) = *s_inc.add(HARAKAS_RATE - (*s_inc.add(64)) as usize + i);
        i += 1;
    }
    out = out.add(i);
    outlen -= i;
    *s_inc.add(64) = (*s_inc.add(64)).wrapping_sub(i as u8);

    /* Then squeeze the remaining necessary blocks */
    while outlen > 0 {
        SPX_haraka512_perm(s_inc, s_inc, ctx);

        i = 0;
        while i < outlen && i < HARAKAS_RATE {
            *out.add(i) = *s_inc.add(i);
            i += 1;
        }
        out = out.add(i);
        outlen -= i;
        *s_inc.add(64) = (HARAKAS_RATE - i) as u8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S(
    mut out: *mut u8,
    outlen: c_ulonglong,
    in_: *const u8,
    inlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    let mut i: c_ulonglong;
    let mut s: [u8; 64] = [0; 64];
    let mut d: [u8; 32] = [0; 32];

    i = 0;
    while i < 64 {
        s[i as usize] = 0;
        i += 1;
    }
    haraka_S_absorb(s.as_mut_ptr(), 32, in_, inlen, 0x1F, ctx);

    haraka_S_squeezeblocks(out, outlen / 32, s.as_mut_ptr(), 32, ctx);
    out = out.add(((outlen / 32) * 32) as usize);

    if outlen % 32 != 0 {
        haraka_S_squeezeblocks(d.as_mut_ptr(), 1, s.as_mut_ptr(), 32, ctx);
        i = 0;
        while i < outlen % 32 {
            *out.add(i as usize) = d[i as usize];
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka512_perm(
    out: *mut u8,
    in_: *const u8,
    ctx: *const SpxCtx,
) {
    let mut w: [u32; 16] = [0; 16];
    let mut q: [u64; 8] = [0; 8];
    let mut tmp_q: u64;
    let mut i: c_uint;
    let mut j: c_uint;

    br_range_dec32le(w.as_mut_ptr(), 16, in_);
    i = 0;
    while i < 4 {
        br_aes_ct64_interleave_in(
            q.as_mut_ptr().add(i as usize),
            q.as_mut_ptr().add((i + 4) as usize),
            w.as_ptr().add((i << 2) as usize),
        );
        i += 1;
    }
    br_aes_ct64_ortho(q.as_mut_ptr());

    /* AES rounds */
    i = 0;
    while i < 5 {
        j = 0;
        while j < 2 {
            br_aes_ct64_bitslice_Sbox(q.as_mut_ptr());
            shift_rows(q.as_mut_ptr());
            mix_columns(q.as_mut_ptr());
            add_round_key(
                q.as_mut_ptr(),
                (*ctx).tweaked512_rc64[(2 * i + j) as usize].as_ptr(),
            );
            j += 1;
        }
        /* Mix states */
        j = 0;
        while j < 8 {
            tmp_q = q[j as usize];
            q[j as usize] = (tmp_q & 0x0001000100010001) << 5
                | (tmp_q & 0x0002000200020002) << 12
                | (tmp_q & 0x0004000400040004) >> 1
                | (tmp_q & 0x0008000800080008) << 6
                | (tmp_q & 0x0020002000200020) << 9
                | (tmp_q & 0x0040004000400040) >> 4
                | (tmp_q & 0x0080008000800080) << 3
                | (tmp_q & 0x2100210021002100) >> 5
                | (tmp_q & 0x0210021002100210) << 2
                | (tmp_q & 0x0800080008000800) << 4
                | (tmp_q & 0x1000100010001000) >> 12
                | (tmp_q & 0x4000400040004000) >> 10
                | (tmp_q & 0x8400840084008400) >> 3;
            j += 1;
        }
        i += 1;
    }

    br_aes_ct64_ortho(q.as_mut_ptr());
    i = 0;
    while i < 4 {
        br_aes_ct64_interleave_out(
            w.as_mut_ptr().add((i << 2) as usize),
            q[i as usize],
            q[(i + 4) as usize],
        );
        i += 1;
    }
    br_range_enc32le(out, w.as_ptr(), 16);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka512(out: *mut u8, in_: *const u8, ctx: *const SpxCtx) {
    let mut i: i32;

    let mut buf: [u8; 64] = [0; 64];

    SPX_haraka512_perm(buf.as_mut_ptr(), in_, ctx);
    /* Feed-forward */
    i = 0;
    while i < 64 {
        buf[i as usize] = buf[i as usize] ^ *in_.add(i as usize);
        i += 1;
    }

    /* Truncated */
    core::ptr::copy_nonoverlapping(buf.as_ptr().add(8), out, 8);
    core::ptr::copy_nonoverlapping(buf.as_ptr().add(24), out.add(8), 8);
    core::ptr::copy_nonoverlapping(buf.as_ptr().add(32), out.add(16), 8);
    core::ptr::copy_nonoverlapping(buf.as_ptr().add(48), out.add(24), 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka256(out: *mut u8, in_: *const u8, ctx: *const SpxCtx) {
    let mut q: [u32; 8] = [0; 8];
    let mut tmp_q: u32;
    let mut i: i32;
    let mut j: i32;

    i = 0;
    while i < 4 {
        q[(2 * i) as usize] = br_dec32le(in_.add((4 * i) as usize));
        q[(2 * i + 1) as usize] = br_dec32le(in_.add((4 * i + 16) as usize));
        i += 1;
    }
    br_aes_ct_ortho(q.as_mut_ptr());

    /* AES rounds */
    i = 0;
    while i < 5 {
        j = 0;
        while j < 2 {
            br_aes_ct_bitslice_Sbox(q.as_mut_ptr());
            shift_rows32(q.as_mut_ptr());
            mix_columns32(q.as_mut_ptr());
            add_round_key32(
                q.as_mut_ptr(),
                (*ctx).tweaked256_rc32[(2 * i + j) as usize].as_ptr(),
            );
            j += 1;
        }

        /* Mix states */
        j = 0;
        while j < 8 {
            tmp_q = q[j as usize];
            q[j as usize] = (tmp_q & 0x81818181)
                | (tmp_q & 0x02020202) << 1
                | (tmp_q & 0x04040404) << 2
                | (tmp_q & 0x08080808) << 3
                | (tmp_q & 0x10101010) >> 3
                | (tmp_q & 0x20202020) >> 2
                | (tmp_q & 0x40404040) >> 1;
            j += 1;
        }
        i += 1;
    }

    br_aes_ct_ortho(q.as_mut_ptr());
    i = 0;
    while i < 4 {
        br_enc32le(out.add((4 * i) as usize), q[(2 * i) as usize]);
        br_enc32le(out.add((4 * i + 16) as usize), q[(2 * i + 1) as usize]);
        i += 1;
    }

    i = 0;
    while i < 32 {
        *out.add(i as usize) ^= *in_.add(i as usize);
        i += 1;
    }
}
