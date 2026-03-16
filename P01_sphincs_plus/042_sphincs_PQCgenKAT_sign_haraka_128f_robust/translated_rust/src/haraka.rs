use crate::aes_bitslice::*;
use crate::context::SpxCtx;
use crate::params::*;

const HARAKAS_RATE: usize = 32;

pub static HARAKA512_RC64: [[u64; 8]; 10] = [
    [0x24cf0ab9086f628b, 0xbdd6eeecc83b8382, 0xd96fb0306cdad0a7, 0xaace082ac8f95f89, 0x449d8e8870d7041f, 0x49bb2f80b2b3e2f8, 0x0569ae98d93bb258, 0x23dc9691e7d6a4b1],
    [0xd8ba10ede0fe5b6e, 0x7ecf7dbe424c7b8e, 0x6ea9949c6df62a31, 0xbf3f3c97ec9c313e, 0x241d03a196a1861e, 0xead3a51116e5a2ea, 0x77d479fcad9574e3, 0x18657a1af894b7a0],
    [0x10671e1a7f595522, 0xd9a00ff675d28c7b, 0x2f1edf0d2b9ba661, 0xb8ff58b8e3de45f9, 0xee29261da9865c02, 0xd1532aa4b50bdf43, 0x8bf858159b231bb1, 0xdf17439d22d4f599],
    [0xdd4b2f0870b918c0, 0x757a81f3b39b1bb6, 0x7a5c556898952e3f, 0x7dd70a16d915d87a, 0x3ae61971982b8301, 0xc3ab319e030412be, 0x17c0033ac094a8cb, 0x5a0630fc1a8dc4ef],
    [0x17708988c1632f73, 0xf92ddae090b44f4f, 0x11ac0285c43aa314, 0x509059941936b8ba, 0xd03e152fa2ce9b69, 0x3fbcbcb63a32998b, 0x6204696d692254f7, 0x915542ed93ec59b4],
    [0xf4ed94aa8879236e, 0xff6cb41cd38e03c0, 0x069b38602368aeab, 0x669495b820f0ddba, 0xf42013b1b8bf9e3d, 0xcf935efe6439734d, 0xbc1dcf42ca29e3f8, 0x7e6d3ed29f78ad67],
    [0xf3b0f6837ffcddaa, 0x3a76faef934ddf41, 0xcec7ae583a9c8e35, 0xe4dd18c68f0260af, 0x2c0e5df1ad398eaa, 0x478df5236ae22e8c, 0xfb944c46fe865f39, 0xaa48f82f028132ba],
    [0x231b9ae2b76aca77, 0x292a76a712db0b40, 0x5850625dc8134491, 0x73137dd469810fb5, 0x8a12a6a202a474fd, 0xd36fd9daa78bdb80, 0xb34c5e733505706f, 0xbaf1cdca818d9d96],
    [0x2e99781335e8c641, 0xbddfe5cce47d560e, 0xf74e9bf32e5e040c, 0x1d7a709d65996be9, 0x670df36a9cf66cdd, 0xd05ef84a176a2875, 0x0f888e828cb1c44e, 0x1a79e9c9727b052c],
    [0x83497348628d84de, 0x2e9387d51f22a754, 0xb000068da2f852d6, 0x378c9e1190fd6fe5, 0x870027c316de7293, 0xe51a9d4462e047bb, 0x90ecf7f8c6251195, 0x655953bfbed90a9c],
];

fn interleave_constant(out: &mut [u64; 8], inp: &[u8]) {
    let mut tmp = [0u32; 16];
    br_range_dec32le(&mut tmp, inp);
    for i in 0..4 {
        let mut q0 = 0u64;
        let mut q1 = 0u64;
        br_aes_ct64_interleave_in(&mut q0, &mut q1, &tmp[i * 4..]);
        out[i] = q0;
        out[i + 4] = q1;
    }
    br_aes_ct64_ortho(out);
}

fn interleave_constant32(out: &mut [u32; 8], inp: &[u8]) {
    for i in 0..4 {
        out[2 * i] = br_dec32le(&inp[4 * i..]);
        out[2 * i + 1] = br_dec32le(&inp[4 * i + 16..]);
    }
    br_aes_ct_ortho(out);
}

pub fn tweak_constants(ctx: &mut SpxCtx) {
    // Copy standard constants first
    for i in 0..10 {
        ctx.tweaked512_rc64[i] = HARAKA512_RC64[i];
    }
    let mut buf = [0u8; 40 * 16];
    haraka_s(&mut buf, 40 * 16, &ctx.pub_seed[..SPX_N], SPX_N, ctx);
    // Now tweak with the generated buf - need to re-borrow
    // We need a copy of buf since we can't borrow ctx mutably while using it
    let buf_copy = buf;
    for i in 0..10 {
        interleave_constant32(&mut ctx.tweaked256_rc32[i], &buf_copy[32 * i..]);
        interleave_constant(&mut ctx.tweaked512_rc64[i], &buf_copy[64 * i..]);
    }
}

pub fn haraka512_perm(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut w = [0u32; 16];
    let mut q = [0u64; 8];
    br_range_dec32le(&mut w, inp);
    for i in 0..4 {
        let mut q0 = 0u64;
        let mut q1 = 0u64;
        br_aes_ct64_interleave_in(&mut q0, &mut q1, &w[i * 4..]);
        q[i] = q0;
        q[i + 4] = q1;
    }
    br_aes_ct64_ortho(&mut q);
    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct64_bitslice_sbox(&mut q);
            shift_rows(&mut q);
            mix_columns(&mut q);
            // add_round_key
            for k in 0..8 { q[k] ^= ctx.tweaked512_rc64[2 * i + j][k]; }
        }
        // Mix states
        for j in 0..8 {
            let tmp_q = q[j];
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
    br_aes_ct64_ortho(&mut q);
    for i in 0..4 {
        br_aes_ct64_interleave_out(&mut w[i * 4..], q[i], q[i + 4]);
    }
    br_range_enc32le(out, &w);
}

pub fn haraka512(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut buf = [0u8; 64];
    haraka512_perm(&mut buf, inp, ctx);
    for i in 0..64 { buf[i] ^= inp[i]; }
    out[..8].copy_from_slice(&buf[8..16]);
    out[8..16].copy_from_slice(&buf[24..32]);
    out[16..24].copy_from_slice(&buf[32..40]);
    out[24..32].copy_from_slice(&buf[48..56]);
}

pub fn haraka256(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut q = [0u32; 8];
    for i in 0..4 {
        q[2 * i] = br_dec32le(&inp[4 * i..]);
        q[2 * i + 1] = br_dec32le(&inp[4 * i + 16..]);
    }
    br_aes_ct_ortho(&mut q);
    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct_bitslice_sbox(&mut q);
            shift_rows32(&mut q);
            mix_columns32(&mut q);
            for k in 0..8 { q[k] ^= ctx.tweaked256_rc32[2 * i + j][k]; }
        }
        for j in 0..8 {
            let tmp_q = q[j];
            q[j] = (tmp_q & 0x81818181)
                | (tmp_q & 0x02020202) << 1
                | (tmp_q & 0x04040404) << 2
                | (tmp_q & 0x08080808) << 3
                | (tmp_q & 0x10101010) >> 3
                | (tmp_q & 0x20202020) >> 2
                | (tmp_q & 0x40404040) >> 1;
        }
    }
    br_aes_ct_ortho(&mut q);
    for i in 0..4 {
        br_enc32le(&mut out[4 * i..], q[2 * i]);
        br_enc32le(&mut out[4 * i + 16..], q[2 * i + 1]);
    }
    for i in 0..32 { out[i] ^= inp[i]; }
}

fn haraka512_perm_inplace(s: &mut [u8], ctx: &SpxCtx) {
    let tmp: Vec<u8> = s[..64].to_vec();
    haraka512_perm(s, &tmp, ctx);
}

fn haraka_s_absorb(s: &mut [u8; 64], m: &[u8], mlen: usize, p: u8, ctx: &SpxCtx) {
    let mut offset = 0usize;
    let mut remaining = mlen;
    while remaining >= HARAKAS_RATE {
        for i in 0..HARAKAS_RATE { s[i] ^= m[offset + i]; }
        haraka512_perm_inplace(s, ctx);
        remaining -= HARAKAS_RATE;
        offset += HARAKAS_RATE;
    }
    let mut t = [0u8; HARAKAS_RATE];
    for i in 0..remaining { t[i] = m[offset + i]; }
    t[remaining] = p;
    t[HARAKAS_RATE - 1] |= 128;
    for i in 0..HARAKAS_RATE { s[i] ^= t[i]; }
}

fn haraka_s_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u8; 64], ctx: &SpxCtx) {
    for i in 0..nblocks {
        haraka512_perm_inplace(s, ctx);
        h[i * HARAKAS_RATE..(i + 1) * HARAKAS_RATE].copy_from_slice(&s[..HARAKAS_RATE]);
    }
}

pub fn haraka_s(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize, ctx: &SpxCtx) {
    let mut s = [0u8; 64];
    haraka_s_absorb(&mut s, inp, inlen, 0x1F, ctx);
    let full_blocks = outlen / 32;
    haraka_s_squeezeblocks(out, full_blocks, &mut s, ctx);
    let done = full_blocks * 32;
    if outlen % 32 != 0 {
        let mut d = [0u8; 32];
        haraka_s_squeezeblocks(&mut d, 1, &mut s, ctx);
        out[done..done + (outlen % 32)].copy_from_slice(&d[..outlen % 32]);
    }
}

pub fn haraka_s_inc_init(s_inc: &mut [u8; 65]) {
    for i in 0..65 { s_inc[i] = 0; }
}

pub fn haraka_s_inc_absorb(s_inc: &mut [u8; 65], m: &[u8], mlen: usize, ctx: &SpxCtx) {
    let mut offset = 0usize;
    let mut remaining = mlen;
    while remaining + (s_inc[64] as usize) >= HARAKAS_RATE {
        let avail = HARAKAS_RATE - s_inc[64] as usize;
        for i in 0..avail {
            s_inc[s_inc[64] as usize + i] ^= m[offset + i];
        }
        remaining -= avail;
        offset += avail;
        s_inc[64] = 0;
        let mut tmp: [u8; 64] = [0; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        let inp = tmp;
        haraka512_perm(&mut tmp, &inp, ctx);
        s_inc[..64].copy_from_slice(&tmp);
    }
    for i in 0..remaining {
        s_inc[s_inc[64] as usize + i] ^= m[offset + i];
    }
    s_inc[64] += remaining as u8;
}

pub fn haraka_s_inc_finalize(s_inc: &mut [u8; 65]) {
    s_inc[s_inc[64] as usize] ^= 0x1F;
    s_inc[HARAKAS_RATE - 1] ^= 128;
    s_inc[64] = 0;
}

pub fn haraka_s_inc_squeeze(out: &mut [u8], outlen: usize, s_inc: &mut [u8; 65], ctx: &SpxCtx) {
    let mut out_offset = 0usize;
    let mut remaining = outlen;
    // First consume leftover bytes
    let mut i = 0usize;
    while i < remaining && i < s_inc[64] as usize {
        out[out_offset] = s_inc[HARAKAS_RATE - s_inc[64] as usize + i];
        out_offset += 1;
        i += 1;
    }
    remaining -= i;
    s_inc[64] -= i as u8;

    while remaining > 0 {
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        let inp = tmp;
        haraka512_perm(&mut tmp, &inp, ctx);
        s_inc[..64].copy_from_slice(&tmp);

        let take = remaining.min(HARAKAS_RATE);
        out[out_offset..out_offset + take].copy_from_slice(&s_inc[..take]);
        out_offset += take;
        remaining -= take;
        s_inc[64] = (HARAKAS_RATE - take) as u8;
    }
}
