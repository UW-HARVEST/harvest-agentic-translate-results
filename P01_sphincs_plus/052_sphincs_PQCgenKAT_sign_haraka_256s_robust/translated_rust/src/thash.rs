use crate::context::SpxCtx;
use crate::haraka::*;
use crate::params::*;

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
    if inblocks == 1 {
        let mut buf_tmp = [0u8; 64];
        buf_tmp[..32].copy_from_slice(addr_bytes);
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
        buf[..32].copy_from_slice(addr_bytes);
        haraka_s(&mut bitmask, inblocks * SPX_N, &buf[..SPX_ADDR_BYTES], SPX_ADDR_BYTES, ctx);
        for i in 0..inblocks * SPX_N {
            buf[SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
        }
        haraka_s(out, SPX_N, &buf, total, ctx);
    }
}
