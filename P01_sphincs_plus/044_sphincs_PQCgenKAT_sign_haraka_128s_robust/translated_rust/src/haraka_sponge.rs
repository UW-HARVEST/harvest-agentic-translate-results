pub fn haraka_s_inc_init(s_inc: &mut [u8; 65]) {
    for i in 0..65 { s_inc[i] = 0; }
}

pub fn haraka_s_inc_absorb(s_inc: &mut [u8; 65], m: &[u8], ctx: &SpxCtx) {
    let mut off = 0usize;
    let mlen = m.len();
    while mlen - off + s_inc[64] as usize >= HARAKAS_RATE {
        let start = s_inc[64] as usize;
        for i in 0..(HARAKAS_RATE - start) {
            s_inc[start + i] ^= m[off + i];
        }
        off += HARAKAS_RATE - start;
        s_inc[64] = 0;
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        haraka512_perm(&mut s_inc[..64], &tmp, ctx);
    }
    let start = s_inc[64] as usize;
    let rem = mlen - off;
    for i in 0..rem {
        s_inc[start + i] ^= m[off + i];
    }
    s_inc[64] = (start + rem) as u8;
}

pub fn haraka_s_inc_finalize(s_inc: &mut [u8; 65]) {
    s_inc[s_inc[64] as usize] ^= 0x1F;
    s_inc[HARAKAS_RATE - 1] ^= 128;
    s_inc[64] = 0;
}

pub fn haraka_s_inc_squeeze(out: &mut [u8], outlen: usize, s_inc: &mut [u8; 65], ctx: &SpxCtx) {
    let mut out_off = 0usize;
    let mut remaining = outlen;
    // First consume leftover bytes
    let avail = s_inc[64] as usize;
    let take = core::cmp::min(remaining, avail);
    for i in 0..take {
        out[out_off + i] = s_inc[HARAKAS_RATE - avail + i];
    }
    out_off += take;
    remaining -= take;
    s_inc[64] -= take as u8;

    while remaining > 0 {
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        haraka512_perm(&mut s_inc[..64], &tmp, ctx);
        let take = core::cmp::min(remaining, HARAKAS_RATE);
        out[out_off..out_off + take].copy_from_slice(&s_inc[..take]);
        out_off += take;
        remaining -= take;
        s_inc[64] = (HARAKAS_RATE - take) as u8;
    }
}
