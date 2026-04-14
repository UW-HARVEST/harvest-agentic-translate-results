use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_N};
use sha2::{Digest, Sha256};

pub fn thash(input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) -> [u8; SPX_N] {
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
    let mut h = Sha256::new();
    h.update(ctx.pub_seed);
    h.update(&addr_bytes[..SPX_ADDR_BYTES]);
    h.update(&input[..inblocks * SPX_N]);
    let digest = h.finalize();
    let mut out = [0u8; SPX_N];
    out.copy_from_slice(&digest[..SPX_N]);
    out
}
