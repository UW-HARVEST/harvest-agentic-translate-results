use crate::params::*;
use crate::context::SpxCtx;
use crate::blake::blake256::{blake256_fn, blake256_mgf1};
use crate::blake::blake512::{blake512_fn, blake512_mgf1};

pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; SPX_ADDR_BYTES]) };

    if SPX_BLAKE512 && inblocks > 1 {
        let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

        blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES], SPX_N + SPX_ADDR_BYTES);

        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        blake512_fn(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

        blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES], SPX_N + SPX_ADDR_BYTES);

        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        blake256_fn(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}
