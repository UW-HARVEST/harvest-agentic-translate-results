use crate::params::*;
use crate::context::SpxCtx;
use sha3::{Shake256, digest::{Update, ExtendableOutput, XofReader}};

#[cfg(feature = "thash-robust")]
pub fn thash(out: &mut [u8], in_val: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut hasher = Shake256::default();
    hasher.update(&ctx.pub_seed);
    let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    hasher.update(addr_bytes);
    let mut reader = hasher.finalize_xof();
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    reader.read(&mut bitmask);

    let mut hasher2 = Shake256::default();
    hasher2.update(&ctx.pub_seed);
    hasher2.update(addr_bytes);
    for i in 0..inblocks * SPX_N {
        hasher2.update(&[in_val[i] ^ bitmask[i]]);
    }
    let mut reader2 = hasher2.finalize_xof();
    reader2.read(&mut out[..SPX_N]);
}

#[cfg(feature = "thash-simple")]
pub fn thash(out: &mut [u8], in_val: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut hasher = Shake256::default();
    hasher.update(&ctx.pub_seed);
    let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    hasher.update(addr_bytes);
    hasher.update(&in_val[..inblocks * SPX_N]);
    let mut reader = hasher.finalize_xof();
    reader.read(&mut out[..SPX_N]);
}
