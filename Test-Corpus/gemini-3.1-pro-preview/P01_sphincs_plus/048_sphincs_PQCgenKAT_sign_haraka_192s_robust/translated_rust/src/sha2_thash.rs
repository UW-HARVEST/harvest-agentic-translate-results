use crate::params::*;
use crate::context::SpxCtx;
use sha2::{Sha256, Sha512, Digest};

pub fn mgf1_256(out: &mut [u8], in_val: &[u8]) {
    let mut i = 0u32;
    let mut out_pos = 0;
    while out_pos < out.len() {
        let mut hasher = Sha256::new();
        hasher.update(in_val);
        hasher.update(&i.to_be_bytes());
        let res = hasher.finalize();
        let take = core::cmp::min(res.len(), out.len() - out_pos);
        out[out_pos..out_pos + take].copy_from_slice(&res[..take]);
        out_pos += take;
        i += 1;
    }
}

#[cfg(any(feature = "secpar-192f", feature = "secpar-192s", feature = "secpar-256f", feature = "secpar-256s"))]
pub fn mgf1_512(out: &mut [u8], in_val: &[u8]) {
    let mut i = 0u32;
    let mut out_pos = 0;
    while out_pos < out.len() {
        let mut hasher = Sha512::new();
        hasher.update(in_val);
        hasher.update(&i.to_be_bytes());
        let res = hasher.finalize();
        let take = core::cmp::min(res.len(), out.len() - out_pos);
        out[out_pos..out_pos + take].copy_from_slice(&res[..take]);
        out_pos += take;
        i += 1;
    }
}

#[cfg(feature = "thash-robust")]
pub fn thash(out: &mut [u8], in_val: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    #[cfg(any(feature = "secpar-192f", feature = "secpar-192s", feature = "secpar-256f", feature = "secpar-256s"))]
    if inblocks > 1 {
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let mut buf = vec![0u8; SPX_N + 22];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + 22].copy_from_slice(&addr_bytes[..22]);
        mgf1_512(&mut bitmask, &buf);

        let mut hasher = ctx.state_seeded_512.clone();
        hasher.update(&addr_bytes[..22]);
        for i in 0..inblocks * SPX_N {
            hasher.update(&[in_val[i] ^ bitmask[i]]);
        }
        let res = hasher.finalize();
        out[..SPX_N].copy_from_slice(&res[..SPX_N]);
        return;
    }

    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + 22];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[SPX_N..SPX_N + 22].copy_from_slice(&addr_bytes[..22]);
    mgf1_256(&mut bitmask, &buf);

    let mut hasher = ctx.state_seeded.clone();
    hasher.update(&addr_bytes[..22]);
    for i in 0..inblocks * SPX_N {
        hasher.update(&[in_val[i] ^ bitmask[i]]);
    }
    let res = hasher.finalize();
    out[..SPX_N].copy_from_slice(&res[..SPX_N]);
}

#[cfg(feature = "thash-simple")]
pub fn thash(out: &mut [u8], in_val: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    #[cfg(any(feature = "secpar-192f", feature = "secpar-192s", feature = "secpar-256f", feature = "secpar-256s"))]
    if inblocks > 1 {
        let mut hasher = ctx.state_seeded_512.clone();
        let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        hasher.update(&addr_bytes[..22]);
        hasher.update(&in_val[..inblocks * SPX_N]);
        let res = hasher.finalize();
        out[..SPX_N].copy_from_slice(&res[..SPX_N]);
        return;
    }

    let mut hasher = ctx.state_seeded.clone();
    let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    hasher.update(&addr_bytes[..22]);
    hasher.update(&in_val[..inblocks * SPX_N]);
    let res = hasher.finalize();
    out[..SPX_N].copy_from_slice(&res[..SPX_N]);
}
