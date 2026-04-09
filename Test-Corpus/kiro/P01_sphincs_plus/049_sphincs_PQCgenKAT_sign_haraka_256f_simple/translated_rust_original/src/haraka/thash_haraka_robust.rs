use crate::params::*;
use crate::context::SpxCtx;
use crate::haraka::haraka_impl::{haraka256, haraka512, haraka_s};

pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };

    if inblocks == 1 {
        let mut outbuf = [0u8; 32];
        let mut buf_tmp = [0u8; 64];
        buf_tmp[..32].copy_from_slice(addr_bytes);

        // Generate bitmask via haraka256
        haraka256(&mut outbuf, &buf_tmp, ctx);
        // XOR input with bitmask
        for i in 0..inblocks * SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = input[i] ^ outbuf[i];
        }
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        buf[..32].copy_from_slice(addr_bytes);

        haraka_s(&mut bitmask, inblocks * SPX_N, &buf[..SPX_ADDR_BYTES], SPX_ADDR_BYTES, ctx);

        for i in 0..inblocks * SPX_N {
            buf[SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }
        haraka_s(out, SPX_N, &buf, SPX_ADDR_BYTES + inblocks * SPX_N, ctx);
    }
}
