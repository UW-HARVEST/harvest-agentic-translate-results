use crate::params::*;
use crate::blake256;
use crate::address;
use crate::hash_blake::SpxCtx;

/// Robust thash: uses blake256_mgf1 bitmask XOR.
/// SPX_BLAKE512=0 so we always use blake256 path.
pub fn thash(
    out: &mut [u8], inp: &[u8], inblocks: u32,
    ctx: &SpxCtx, addr: &mut [u32; 8],
) {
    let inblocks_usize = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks_usize * SPX_N];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks_usize * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = address::addr_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);

    blake256::blake256_mgf1(&mut bitmask, inblocks_usize * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks_usize * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256::blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks_usize * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
