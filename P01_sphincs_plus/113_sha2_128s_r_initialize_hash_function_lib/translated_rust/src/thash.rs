use crate::params::*;
use crate::context::SpxCtx;
use crate::sha2::*;

/// Robust thash: XOR with MGF1 bitmask before hashing
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..],
                        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
