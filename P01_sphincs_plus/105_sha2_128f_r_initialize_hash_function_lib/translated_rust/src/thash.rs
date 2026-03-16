use crate::params::*;
use crate::hash::SpxCtx;
use crate::sha2::*;
use crate::address::addr_as_bytes;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8, inp: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let inp = unsafe { std::slice::from_raw_parts(inp, inblocks as usize * SPX_N) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, SPX_N) };
    thash_rs(out, inp, inblocks as usize, ctx, addr);
}

pub fn thash_rs(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    // SHA2-128f robust: always uses SHA-256 (SPX_SHA512=0)
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let ab = addr_as_bytes(addr);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256_rs(&mut bitmask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES]);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha256_inc_finalize(
        outbuf.as_mut_ptr(), sha2_state.as_mut_ptr(),
        buf[SPX_N..].as_ptr(), SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
