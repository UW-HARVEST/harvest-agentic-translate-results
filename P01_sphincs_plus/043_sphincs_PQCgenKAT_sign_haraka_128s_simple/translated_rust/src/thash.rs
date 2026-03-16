use crate::context::SpxCtx;
use crate::params::*;
use crate::haraka::*;

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u8; 32]) {
    if inblocks == 1 {
        let mut buf_tmp = [0u8; 64];
        buf_tmp[..32].copy_from_slice(addr);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&inp[..SPX_N]);
        let mut outbuf = [0u8; 32];
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let total = SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; total];
        buf[..32].copy_from_slice(addr);
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + inblocks * SPX_N]
            .copy_from_slice(&inp[..inblocks * SPX_N]);
        let mut tmp = vec![0u8; SPX_N];
        haraka_s(&mut tmp, &buf, ctx);
        out[..SPX_N].copy_from_slice(&tmp);
    }
}
