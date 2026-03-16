use crate::context::SpxCtx;
use crate::haraka::*;
use crate::params::*;

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let ab = crate::address::addr_bytes(addr);
    if inblocks == 1 {
        let mut buf_tmp = [0u8; 64];
        buf_tmp[..32].copy_from_slice(ab);
        let mut outbuf = [0u8; 32];
        haraka256(&mut outbuf, &buf_tmp[..32], ctx);
        for i in 0..SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = inp[i] ^ outbuf[i];
        }
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let total = SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; total];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        buf[..32].copy_from_slice(ab);

        // Generate bitmask
        let mut s_inc = [0u8; 65];
        haraka_s_inc_init(&mut s_inc);
        haraka_s_inc_absorb(&mut s_inc, &buf[..SPX_ADDR_BYTES], ctx);
        haraka_s_inc_finalize(&mut s_inc);
        haraka_s_inc_squeeze(&mut bitmask, inblocks * SPX_N, &mut s_inc, ctx);

        for i in 0..inblocks * SPX_N {
            buf[SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
        }

        let mut s_inc2 = [0u8; 65];
        haraka_s_inc_init(&mut s_inc2);
        haraka_s_inc_absorb(&mut s_inc2, &buf[..total], ctx);
        haraka_s_inc_finalize(&mut s_inc2);
        haraka_s_inc_squeeze(out, SPX_N, &mut s_inc2, ctx);
    }
}
