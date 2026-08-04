// Haraka thash implementation
#![cfg(feature = "haraka")]
#![allow(dead_code)]

use crate::context::SpxCtx;
use crate::haraka::*;
use crate::params::*;

#[cfg(feature = "robust")]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        for i in 0..64 {
            buf_tmp[i] = 0;
        }
        let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
        buf_tmp[..32].copy_from_slice(addr_bytes);

        haraka256(&mut outbuf, &buf_tmp, ctx);
        for i in 0..inblocks * SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = input[i] ^ outbuf[i];
        }
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
        buf[..32].copy_from_slice(addr_bytes);
        haraka_S(
            &mut bitmask,
            (inblocks * SPX_N) as u64,
            &buf,
            SPX_ADDR_BYTES as u64,
            ctx,
        );

        for i in 0..inblocks * SPX_N {
            buf[SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }
        haraka_S(
            out,
            SPX_N as u64,
            &buf,
            (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
            ctx,
        );
    }
}

#[cfg(feature = "simple")]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        for i in 0..64 {
            buf_tmp[i] = 0;
        }
        let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
        buf_tmp[..32].copy_from_slice(addr_bytes);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&input[..SPX_N]);

        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
        let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
        buf[..32].copy_from_slice(addr_bytes);
        buf[SPX_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);
        haraka_S(
            out,
            SPX_N as u64,
            &buf,
            (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
            ctx,
        );
    }
}
