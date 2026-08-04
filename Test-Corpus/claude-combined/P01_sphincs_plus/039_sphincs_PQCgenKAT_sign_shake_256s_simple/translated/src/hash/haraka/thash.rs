// Translation of c_src/lib/haraka/src/thash_haraka_{robust,simple}.c

use core::slice;

use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_N};

use super::haraka::{haraka256_inner, haraka512_inner, haraka_s_inner};

#[cfg(feature = "robust")]
pub fn thash_robust(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32]) {
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        for v in buf_tmp.iter_mut() { *v = 0; }
        let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf_tmp[..32].copy_from_slice(&addr_bytes[..32]);

        haraka256_inner(&mut outbuf, &buf_tmp, ctx);
        for i in 0..(inblocks * SPX_N) {
            buf_tmp[SPX_ADDR_BYTES + i] = input[i] ^ outbuf[i];
        }
        haraka512_inner(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[..32].copy_from_slice(&addr_bytes[..32]);
        haraka_s_inner(&mut bitmask, (inblocks * SPX_N) as u64, &buf, SPX_ADDR_BYTES as u64, ctx);

        for i in 0..(inblocks * SPX_N) {
            buf[SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }
        haraka_s_inner(out, SPX_N as u64, &buf, (SPX_ADDR_BYTES + inblocks * SPX_N) as u64, ctx);
    }
}

#[cfg(feature = "simple")]
pub fn thash_simple(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32]) {
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        for v in buf_tmp.iter_mut() { *v = 0; }
        let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf_tmp[..32].copy_from_slice(&addr_bytes[..32]);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&input[..SPX_N]);

        haraka512_inner(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
        let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[..32].copy_from_slice(&addr_bytes[..32]);
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + inblocks * SPX_N]
            .copy_from_slice(&input[..inblocks * SPX_N]);

        haraka_s_inner(out, SPX_N as u64, &buf, (SPX_ADDR_BYTES + inblocks * SPX_N) as u64, ctx);
    }
}
