// Core haraka functions: tweak_constants, haraka512_perm, haraka512, haraka256, sponge

pub fn tweak_constants(ctx: &mut SpxCtx) {
    // Copy standard constants
    for i in 0..10 {
        ctx.tweaked512_rc64[i] = HARAKA512_RC64[i];
    }
    // Generate tweaked constants from pub_seed
    let mut buf = [0u8; 40 * 16];
    haraka_s(&mut buf, 40 * 16, &ctx.pub_seed[..SPX_N], SPX_N as u64, ctx);
    for i in 0..10 {
        interleave_constant32(&mut ctx.tweaked256_rc32[i], &buf[32 * i..]);
        interleave_constant(&mut ctx.tweaked512_rc64[i], &buf[64 * i..]);
    }
}

pub fn haraka512_perm(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut w = [0u32; 16];
    let mut q = [0u64; 8];
    br_range_dec32le(&mut w, &inp[..64]);
    for i in 0..4 {
        let mut a = 0u64;
        let mut b = 0u64;
        br_aes_ct64_interleave_in(&mut a, &mut b, &w[i * 4..]);
        q[i] = a;
        q[i + 4] = b;
    }
    br_aes_ct64_ortho(&mut q);

    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct64_bitslice_sbox(&mut q);
            shift_rows(&mut q);
            mix_columns(&mut q);
            add_round_key(&mut q, &ctx.tweaked512_rc64[2 * i + j]);
        }
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
    br_range_enc32le(&mut out[..64], &w);
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
            add_round_key32(&mut q, &ctx.tweaked256_rc32[2 * i + j]);
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

fn haraka_s_absorb(s: &mut [u8; 64], m: &[u8], mlen: usize, p: u8, ctx: &SpxCtx) {
    let r = HARAKAS_RATE;
    let mut off = 0usize;
    let mut remaining = mlen;
    while remaining >= r {
        for i in 0..r { s[i] ^= m[off + i]; }
        let tmp = *s;
        haraka512_perm(s, &tmp, ctx);
        remaining -= r;
        off += r;
    }
    let mut t = vec![0u8; r];
    for i in 0..remaining { t[i] = m[off + i]; }
    t[remaining] = p;
    t[r - 1] |= 128;
    for i in 0..r { s[i] ^= t[i]; }
}

fn haraka_s_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u8; 64], ctx: &SpxCtx) {
    for i in 0..nblocks {
        let tmp = *s;
        haraka512_perm(s, &tmp, ctx);
        h[i * HARAKAS_RATE..(i + 1) * HARAKAS_RATE].copy_from_slice(&s[..HARAKAS_RATE]);
    }
}

pub fn haraka_s(out: &mut [u8], outlen: usize, inp: &[u8], inlen: u64, ctx: &SpxCtx) {
    let mut s = [0u8; 64];
    haraka_s_absorb(&mut s, inp, inlen as usize, 0x1F, ctx);
    let full_blocks = outlen / 32;
    if full_blocks > 0 {
        haraka_s_squeezeblocks(&mut out[..full_blocks * 32], full_blocks, &mut s, ctx);
    }
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

pub fn haraka_s_inc_absorb(s_inc: &mut [u8; 65], m: &[u8], mut mlen: usize, ctx: &SpxCtx) {
    let mut m_off = 0usize;
    while mlen + s_inc[64] as usize >= HARAKAS_RATE {
        let avail = HARAKAS_RATE - s_inc[64] as usize;
        for i in 0..avail {
            s_inc[s_inc[64] as usize + i] ^= m[m_off + i];
        }
        mlen -= avail;
        m_off += avail;
        s_inc[64] = 0;
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        haraka512_perm(&mut s_inc[..64], &tmp, ctx);
    }
    for i in 0..mlen {
        s_inc[s_inc[64] as usize + i] ^= m[m_off + i];
    }
    s_inc[64] += mlen as u8;
}

pub fn haraka_s_inc_finalize(s_inc: &mut [u8; 65]) {
    s_inc[s_inc[64] as usize] ^= 0x1F;
    s_inc[HARAKAS_RATE - 1] ^= 128;
    s_inc[64] = 0;
}

pub fn haraka_s_inc_squeeze(out: &mut [u8], mut outlen: usize, s_inc: &mut [u8; 65], ctx: &SpxCtx) {
    let mut out_off = 0usize;
    // First consume leftover bytes
    let mut i = 0usize;
    while i < outlen && i < s_inc[64] as usize {
        out[out_off] = s_inc[HARAKAS_RATE - s_inc[64] as usize + i];
        out_off += 1;
        i += 1;
    }
    outlen -= i;
    s_inc[64] -= i as u8;

    while outlen > 0 {
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        haraka512_perm(&mut s_inc[..64], &tmp, ctx);
        let take = if outlen < HARAKAS_RATE { outlen } else { HARAKAS_RATE };
        out[out_off..out_off + take].copy_from_slice(&s_inc[..take]);
        out_off += take;
        outlen -= take;
        s_inc[64] = (HARAKAS_RATE - take) as u8;
    }
}
