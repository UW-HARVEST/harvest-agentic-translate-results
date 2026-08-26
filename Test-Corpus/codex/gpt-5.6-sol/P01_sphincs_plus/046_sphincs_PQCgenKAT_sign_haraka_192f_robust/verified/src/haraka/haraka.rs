//! Translation of `lib/haraka/src/haraka.c` — the constant table
//! (`haraka512_rc64`), the constant tweaking helpers, the Haraka sponge
//! (`haraka_S*`) and the Haraka-512 / Haraka-256 compression functions.
//!
//! The bit-sliced AES helpers of the same C file (lines 54-678) live in
//! [`crate::haraka::aes_ct`] and are only called from here.
//!
//! Every expression, index and constant is transcribed verbatim from the C
//! reference implementation so that the behaviour is byte-identical.

use crate::context::SpxCtx;
use crate::haraka::aes_ct::{
    add_round_key, add_round_key32, br_aes_ct64_bitslice_Sbox, br_aes_ct64_interleave_in,
    br_aes_ct64_interleave_out, br_aes_ct64_ortho, br_aes_ct_bitslice_Sbox, br_aes_ct_ortho,
    br_dec32le, br_enc32le, br_range_dec32le, br_range_enc32le, mix_columns, mix_columns32,
    shift_rows, shift_rows32,
};
use crate::params::SPX_N;

/// `#define HARAKAS_RATE 32`
const HARAKAS_RATE: usize = 32;

/// `static const uint64_t haraka512_rc64[10][8]`
static haraka512_rc64: [[u64; 8]; 10] = [
    [
        0x24cf0ab9086f628b,
        0xbdd6eeecc83b8382,
        0xd96fb0306cdad0a7,
        0xaace082ac8f95f89,
        0x449d8e8870d7041f,
        0x49bb2f80b2b3e2f8,
        0x0569ae98d93bb258,
        0x23dc9691e7d6a4b1,
    ],
    [
        0xd8ba10ede0fe5b6e,
        0x7ecf7dbe424c7b8e,
        0x6ea9949c6df62a31,
        0xbf3f3c97ec9c313e,
        0x241d03a196a1861e,
        0xead3a51116e5a2ea,
        0x77d479fcad9574e3,
        0x18657a1af894b7a0,
    ],
    [
        0x10671e1a7f595522,
        0xd9a00ff675d28c7b,
        0x2f1edf0d2b9ba661,
        0xb8ff58b8e3de45f9,
        0xee29261da9865c02,
        0xd1532aa4b50bdf43,
        0x8bf858159b231bb1,
        0xdf17439d22d4f599,
    ],
    [
        0xdd4b2f0870b918c0,
        0x757a81f3b39b1bb6,
        0x7a5c556898952e3f,
        0x7dd70a16d915d87a,
        0x3ae61971982b8301,
        0xc3ab319e030412be,
        0x17c0033ac094a8cb,
        0x5a0630fc1a8dc4ef,
    ],
    [
        0x17708988c1632f73,
        0xf92ddae090b44f4f,
        0x11ac0285c43aa314,
        0x509059941936b8ba,
        0xd03e152fa2ce9b69,
        0x3fbcbcb63a32998b,
        0x6204696d692254f7,
        0x915542ed93ec59b4,
    ],
    [
        0xf4ed94aa8879236e,
        0xff6cb41cd38e03c0,
        0x069b38602368aeab,
        0x669495b820f0ddba,
        0xf42013b1b8bf9e3d,
        0xcf935efe6439734d,
        0xbc1dcf42ca29e3f8,
        0x7e6d3ed29f78ad67,
    ],
    [
        0xf3b0f6837ffcddaa,
        0x3a76faef934ddf41,
        0xcec7ae583a9c8e35,
        0xe4dd18c68f0260af,
        0x2c0e5df1ad398eaa,
        0x478df5236ae22e8c,
        0xfb944c46fe865f39,
        0xaa48f82f028132ba,
    ],
    [
        0x231b9ae2b76aca77,
        0x292a76a712db0b40,
        0x5850625dc8134491,
        0x73137dd469810fb5,
        0x8a12a6a202a474fd,
        0xd36fd9daa78bdb80,
        0xb34c5e733505706f,
        0xbaf1cdca818d9d96,
    ],
    [
        0x2e99781335e8c641,
        0xbddfe5cce47d560e,
        0xf74e9bf32e5e040c,
        0x1d7a709d65996be9,
        0x670df36a9cf66cdd,
        0xd05ef84a176a2875,
        0x0f888e828cb1c44e,
        0x1a79e9c9727b052c,
    ],
    [
        0x83497348628d84de,
        0x2e9387d51f22a754,
        0xb000068da2f852d6,
        0x378c9e1190fd6fe5,
        0x870027c316de7293,
        0xe51a9d4462e047bb,
        0x90ecf7f8c6251195,
        0x655953bfbed90a9c,
    ],
];

/// `static void interleave_constant(uint64_t *out, const unsigned char *in)`
unsafe fn interleave_constant(out: *mut u64, in_: *const u8) {
    let mut tmp_32_constant = [0u32; 16];

    br_range_dec32le(tmp_32_constant.as_mut_ptr(), 16, in_);
    for i in 0..4usize {
        br_aes_ct64_interleave_in(
            out.add(i),
            out.add(i + 4),
            tmp_32_constant.as_ptr().add(i << 2),
        );
    }
    br_aes_ct64_ortho(out);
}

/// `static void interleave_constant32(uint32_t *out, const unsigned char *in)`
unsafe fn interleave_constant32(out: *mut u32, in_: *const u8) {
    for i in 0..4usize {
        *out.add(2 * i) = br_dec32le(in_.add(4 * i));
        *out.add(2 * i + 1) = br_dec32le(in_.add(4 * i + 16));
    }
    br_aes_ct_ortho(out);
}

/// `void tweak_constants(spx_ctx *ctx)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_tweak_constants(ctx: *mut SpxCtx) {
    let mut buf = [0u8; 40 * 16];

    /* Use the standard constants to generate tweaked ones. */
    core::ptr::copy_nonoverlapping(
        haraka512_rc64.as_ptr() as *const u8,
        (*ctx).tweaked512_rc64.as_mut_ptr() as *mut u8,
        40 * 16,
    );

    /* Constants for pk.seed */
    SPX_haraka_S(
        buf.as_mut_ptr(),
        (40 * 16) as u64,
        (*ctx).pub_seed.as_ptr(),
        SPX_N as u64,
        ctx as *const SpxCtx,
    );
    for i in 0..10usize {
        interleave_constant32(
            (*ctx).tweaked256_rc32[i].as_mut_ptr(),
            buf.as_ptr().add(32 * i),
        );
        interleave_constant(
            (*ctx).tweaked512_rc64[i].as_mut_ptr(),
            buf.as_ptr().add(64 * i),
        );
    }
}

/// ```c
/// static void haraka_S_absorb(unsigned char *s, unsigned int r,
///                             const unsigned char *m, unsigned long long mlen,
///                             unsigned char p, const spx_ctx *ctx)
/// ```
///
/// The C code uses `SPX_VLA(uint8_t, t, r)`; every call site passes
/// `r == HARAKAS_RATE`, so a fixed-size buffer of that length is used here.
unsafe fn haraka_S_absorb(s: *mut u8, r: u32, m: *const u8, mlen: u64, p: u8, ctx: *const SpxCtx) {
    let mut i: u64;
    let mut t_buf = [0u8; HARAKAS_RATE];
    let t = t_buf.as_mut_ptr();

    let r64 = r as u64;
    let mut m = m;
    let mut mlen = mlen;

    while mlen >= r64 {
        /* XOR block to state */
        i = 0;
        while i < r64 {
            *s.add(i as usize) ^= *m.add(i as usize);
            i += 1;
        }
        SPX_haraka512_perm(s, s, ctx);
        mlen -= r64;
        m = m.add(r as usize);
    }

    i = 0;
    while i < r64 {
        *t.add(i as usize) = 0;
        i += 1;
    }
    i = 0;
    while i < mlen {
        *t.add(i as usize) = *m.add(i as usize);
        i += 1;
    }
    *t.add(i as usize) = p;
    *t.add((r64 - 1) as usize) |= 128;
    i = 0;
    while i < r64 {
        *s.add(i as usize) ^= *t.add(i as usize);
        i += 1;
    }
}

/// ```c
/// static void haraka_S_squeezeblocks(unsigned char *h, unsigned long long nblocks,
///                                    unsigned char *s, unsigned int r,
///                                    const spx_ctx *ctx)
/// ```
unsafe fn haraka_S_squeezeblocks(h: *mut u8, nblocks: u64, s: *mut u8, r: u32, ctx: *const SpxCtx) {
    let mut h = h;
    let mut nblocks = nblocks;

    while nblocks > 0 {
        SPX_haraka512_perm(s, s, ctx);
        core::ptr::copy_nonoverlapping(s as *const u8, h, HARAKAS_RATE);
        h = h.add(r as usize);
        nblocks -= 1;
    }
}

/// `void haraka_S_inc_init(uint8_t *s_inc)`
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

/// ```c
/// void haraka_S_inc_absorb(uint8_t *s_inc, const uint8_t *m, size_t mlen,
///         const spx_ctx *ctx)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_absorb(
    s_inc: *mut u8,
    m: *const u8,
    mlen: usize,
    ctx: *const SpxCtx,
) {
    let mut i: usize;

    let mut m = m;
    let mut mlen = mlen;

    /* Recall that s_inc[64] is the non-absorbed bytes xored into the state */
    while mlen.wrapping_add(*s_inc.add(64) as usize) >= HARAKAS_RATE {
        i = 0;
        while i < HARAKAS_RATE.wrapping_sub(*s_inc.add(64) as usize) {
            /* Take the i'th byte from message
            xor with the s_inc[64] + i'th byte of the state */
            let idx = (*s_inc.add(64) as usize).wrapping_add(i);
            *s_inc.add(idx) ^= *m.add(i);
            i += 1;
        }
        mlen = mlen.wrapping_sub(HARAKAS_RATE.wrapping_sub(*s_inc.add(64) as usize));
        m = m.add(HARAKAS_RATE.wrapping_sub(*s_inc.add(64) as usize));
        *s_inc.add(64) = 0;

        SPX_haraka512_perm(s_inc, s_inc, ctx);
    }

    i = 0;
    while i < mlen {
        let idx = (*s_inc.add(64) as usize).wrapping_add(i);
        *s_inc.add(idx) ^= *m.add(i);
        i += 1;
    }
    *s_inc.add(64) = (*s_inc.add(64)).wrapping_add(mlen as u8);
}

/// `void haraka_S_inc_finalize(uint8_t *s_inc)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_finalize(s_inc: *mut u8) {
    /* After haraka_S_inc_absorb, we are guaranteed that s_inc[64] < HARAKAS_RATE,
    so we can always use one more byte for p in the current state. */
    let idx = *s_inc.add(64) as usize;
    *s_inc.add(idx) ^= 0x1F;
    *s_inc.add(HARAKAS_RATE - 1) ^= 128;
    *s_inc.add(64) = 0;
}

/// ```c
/// void haraka_S_inc_squeeze(uint8_t *out, size_t outlen, uint8_t *s_inc,
///         const spx_ctx *ctx)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_squeeze(
    out: *mut u8,
    outlen: usize,
    s_inc: *mut u8,
    ctx: *const SpxCtx,
) {
    let mut i: usize;

    let mut out = out;
    let mut outlen = outlen;

    /* First consume any bytes we still have sitting around */
    i = 0;
    while i < outlen && i < *s_inc.add(64) as usize {
        /* There are s_inc[64] bytes left, so r - s_inc[64] is the first
        available byte. We consume from there, i.e., up to r. */
        let idx = HARAKAS_RATE
            .wrapping_sub(*s_inc.add(64) as usize)
            .wrapping_add(i);
        *out.add(i) = *s_inc.add(idx);
        i += 1;
    }
    out = out.add(i);
    outlen = outlen.wrapping_sub(i);
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
        outlen = outlen.wrapping_sub(i);
        *s_inc.add(64) = HARAKAS_RATE.wrapping_sub(i) as u8;
    }
}

/// ```c
/// void haraka_S(unsigned char *out, unsigned long long outlen,
///               const unsigned char *in, unsigned long long inlen,
///               const spx_ctx *ctx)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S(
    out: *mut u8,
    outlen: u64,
    in_: *const u8,
    inlen: u64,
    ctx: *const SpxCtx,
) {
    let mut i: u64;
    let mut s = [0u8; 64];
    let mut d = [0u8; 32];

    let mut out = out;

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

/// ```c
/// void haraka512_perm(unsigned char *out, const unsigned char *in,
///         const spx_ctx *ctx)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka512_perm(out: *mut u8, in_: *const u8, ctx: *const SpxCtx) {
    let mut w = [0u32; 16];
    let mut q = [0u64; 8];
    let mut tmp_q: u64;

    br_range_dec32le(w.as_mut_ptr(), 16, in_);
    for i in 0..4usize {
        let qp = q.as_mut_ptr();
        br_aes_ct64_interleave_in(qp.add(i), qp.add(i + 4), w.as_ptr().add(i << 2));
    }
    br_aes_ct64_ortho(q.as_mut_ptr());

    /* AES rounds */
    for i in 0..5usize {
        for j in 0..2usize {
            br_aes_ct64_bitslice_Sbox(q.as_mut_ptr());
            shift_rows(q.as_mut_ptr());
            mix_columns(q.as_mut_ptr());
            add_round_key(q.as_mut_ptr(), (*ctx).tweaked512_rc64[2 * i + j].as_ptr());
        }
        /* Mix states */
        for j in 0..8usize {
            tmp_q = q[j];
            q[j] = (tmp_q & 0x0001000100010001) << 5
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
        }
    }

    br_aes_ct64_ortho(q.as_mut_ptr());
    for i in 0..4usize {
        br_aes_ct64_interleave_out(w.as_mut_ptr().add(i << 2), q[i], q[i + 4]);
    }
    br_range_enc32le(out, w.as_ptr(), 16);
}

/// `void haraka512(unsigned char *out, const unsigned char *in, const spx_ctx *ctx)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka512(out: *mut u8, in_: *const u8, ctx: *const SpxCtx) {
    let mut buf = [0u8; 64];

    SPX_haraka512_perm(buf.as_mut_ptr(), in_, ctx);
    /* Feed-forward */
    for i in 0..64usize {
        buf[i] = buf[i] ^ *in_.add(i);
    }

    /* Truncated */
    core::ptr::copy_nonoverlapping(buf.as_ptr().add(8), out, 8);
    core::ptr::copy_nonoverlapping(buf.as_ptr().add(24), out.add(8), 8);
    core::ptr::copy_nonoverlapping(buf.as_ptr().add(32), out.add(16), 8);
    core::ptr::copy_nonoverlapping(buf.as_ptr().add(48), out.add(24), 8);
}

/// ```c
/// void haraka256(unsigned char *out, const unsigned char *in,
///         const spx_ctx *ctx)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka256(out: *mut u8, in_: *const u8, ctx: *const SpxCtx) {
    let mut q = [0u32; 8];
    let mut tmp_q: u32;

    for i in 0..4usize {
        q[2 * i] = br_dec32le(in_.add(4 * i));
        q[2 * i + 1] = br_dec32le(in_.add(4 * i + 16));
    }
    br_aes_ct_ortho(q.as_mut_ptr());

    /* AES rounds */
    for i in 0..5usize {
        for j in 0..2usize {
            br_aes_ct_bitslice_Sbox(q.as_mut_ptr());
            shift_rows32(q.as_mut_ptr());
            mix_columns32(q.as_mut_ptr());
            add_round_key32(q.as_mut_ptr(), (*ctx).tweaked256_rc32[2 * i + j].as_ptr());
        }

        /* Mix states */
        for j in 0..8usize {
            tmp_q = q[j];
            q[j] = (tmp_q & 0x81818181)
                | (tmp_q & 0x02020202) << 1
                | (tmp_q & 0x04040404) << 2
                | (tmp_q & 0x08080808) << 3
                | (tmp_q & 0x10101010) >> 3
                | (tmp_q & 0x20202020) >> 2
                | (tmp_q & 0x40404040) >> 1;
        }
    }

    br_aes_ct_ortho(q.as_mut_ptr());
    for i in 0..4usize {
        br_enc32le(out.add(4 * i), q[2 * i]);
        br_enc32le(out.add(4 * i + 16), q[2 * i + 1]);
    }

    for i in 0..32usize {
        *out.add(i) ^= *in_.add(i);
    }
}
