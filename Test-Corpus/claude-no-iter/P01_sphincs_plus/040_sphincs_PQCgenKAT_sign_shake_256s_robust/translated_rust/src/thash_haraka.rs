// Translation of c_src/lib/haraka/src/thash_haraka_{robust,simple}.c

use crate::context::SpxCtx;
use crate::haraka::{haraka_s, haraka256, haraka512};
use crate::params::{SPX_ADDR_BYTES, SPX_N};

#[cfg(feature = "robust")]
pub fn thash(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let buf_len = SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };

    if inblocks == 1 {
        // F function
        for b in buf_tmp.iter_mut() {
            *b = 0;
        }
        buf_tmp[..32].copy_from_slice(&addr_bytes[..32]);

        haraka256(&mut outbuf, &buf_tmp, ctx);
        for i in 0..(inblocks * SPX_N) {
            buf_tmp[SPX_ADDR_BYTES + i] = in_buf[i] ^ outbuf[i];
        }
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        // All other tweakable hashes
        buf[..32].copy_from_slice(&addr_bytes[..32]);
        haraka_s(
            &mut bitmask,
            (inblocks * SPX_N) as u64,
            &buf,
            SPX_ADDR_BYTES as u64,
            ctx,
        );

        for i in 0..(inblocks * SPX_N) {
            buf[SPX_ADDR_BYTES + i] = in_buf[i] ^ bitmask[i];
        }

        haraka_s(out, SPX_N as u64, &buf, (SPX_ADDR_BYTES + inblocks * SPX_N) as u64, ctx);
    }
}

#[cfg(feature = "simple")]
pub fn thash(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let buf_len = SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };

    if inblocks == 1 {
        for b in buf_tmp.iter_mut() {
            *b = 0;
        }
        buf_tmp[..32].copy_from_slice(&addr_bytes[..32]);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&in_buf[..SPX_N]);

        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        buf[..32].copy_from_slice(&addr_bytes[..32]);
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + inblocks * SPX_N]
            .copy_from_slice(&in_buf[..inblocks * SPX_N]);

        haraka_s(out, SPX_N as u64, &buf, (SPX_ADDR_BYTES + inblocks * SPX_N) as u64, ctx);
    }
}
