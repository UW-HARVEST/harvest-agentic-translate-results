pub fn haraka512_perm(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut w = [0u32; 16];
    let mut q = [0u64; 8];
    br_range_dec32le(&mut w, inp);
    for i in 0..4 {
        let mut a = 0u64; let mut b = 0u64;
        br_aes_ct64_interleave_in(&mut a, &mut b, &w[i*4..]);
        q[i] = a; q[i+4] = b;
    }
    br_aes_ct64_ortho(&mut q);
    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct64_bitslice_sbox(&mut q); shift_rows(&mut q); mix_columns(&mut q);
            add_round_key(&mut q, &ctx.tweaked512_rc64[2*i+j]);
        }
        for j in 0..8 {
            let t = q[j];
            q[j] = (t&0x0001000100010001)<<5|(t&0x0002000200020002)<<12|(t&0x0004000400040004)>>1
                |(t&0x0008000800080008)<<6|(t&0x0020002000200020)<<9|(t&0x0040004000400040)>>4
                |(t&0x0080008000800080)<<3|(t&0x2100210021002100)>>5|(t&0x0210021002100210)<<2
                |(t&0x0800080008000800)<<4|(t&0x1000100010001000)>>12|(t&0x4000400040004000)>>10
                |(t&0x8400840084008400)>>3;
        }
    }
    br_aes_ct64_ortho(&mut q);
    for i in 0..4 { br_aes_ct64_interleave_out(&mut w[i*4..], q[i], q[i+4]); }
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
    for i in 0..4 { q[2*i]=br_dec32le(&inp[4*i..]); q[2*i+1]=br_dec32le(&inp[4*i+16..]); }
    br_aes_ct_ortho(&mut q);
    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct_bitslice_sbox(&mut q); shift_rows32(&mut q); mix_columns32(&mut q);
            add_round_key32(&mut q, &ctx.tweaked256_rc32[2*i+j]);
        }
        for j in 0..8 {
            let t = q[j];
            q[j] = (t&0x81818181)|(t&0x02020202)<<1|(t&0x04040404)<<2|(t&0x08080808)<<3
                |(t&0x10101010)>>3|(t&0x20202020)>>2|(t&0x40404040)>>1;
        }
    }
    br_aes_ct_ortho(&mut q);
    for i in 0..4 { br_enc32le(&mut out[4*i..], q[2*i]); br_enc32le(&mut out[4*i+16..], q[2*i+1]); }
    for i in 0..32 { out[i] ^= inp[i]; }
}

pub fn tweak_constants(ctx: &mut SpxCtx) {
    // Copy standard constants
    ctx.tweaked512_rc64 = HARAKA512_RC64;
    // Generate tweaked constants from pub_seed
    let mut buf = [0u8; 40*16];
    haraka_s(&mut buf, &ctx.pub_seed[..SPX_N], ctx);
    for i in 0..10 {
        interleave_constant32(&mut ctx.tweaked256_rc32[i], &buf[32*i..]);
        interleave_constant(&mut ctx.tweaked512_rc64[i], &buf[64*i..]);
    }
}

fn haraka_s(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut s = [0u8; 64];
    haraka_s_absorb(&mut s, 32, inp, 0x1F, ctx);
    let outlen = out.len();
    let full_blocks = outlen / 32;
    haraka_s_squeezeblocks(&mut out[..full_blocks*32], full_blocks, &mut s, ctx);
    if outlen % 32 != 0 {
        let mut d = [0u8; 32];
        let tmp = s.clone();
        haraka512_perm(&mut s, &tmp, ctx);
        d.copy_from_slice(&s[..32]);
        out[full_blocks*32..].copy_from_slice(&d[..outlen%32]);
    }
}

fn haraka_s_absorb(s: &mut [u8; 64], r: usize, m: &[u8], p: u8, ctx: &SpxCtx) {
    let mut off = 0usize;
    let mlen = m.len();
    while off + r <= mlen {
        for i in 0..r { s[i] ^= m[off+i]; }
        let tmp = s.clone();
        haraka512_perm(s, &tmp, ctx);
        off += r;
    }
    let mut t = vec![0u8; r];
    let rem = mlen - off;
    t[..rem].copy_from_slice(&m[off..]);
    t[rem] = p;
    t[r-1] |= 128;
    for i in 0..r { s[i] ^= t[i]; }
}

fn haraka_s_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u8; 64], ctx: &SpxCtx) {
    for i in 0..nblocks {
        let tmp = s.clone();
        haraka512_perm(s, &tmp, ctx);
        h[i*HARAKAS_RATE..(i+1)*HARAKAS_RATE].copy_from_slice(&s[..HARAKAS_RATE]);
    }
}
