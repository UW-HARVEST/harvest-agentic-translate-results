// Translation of c_src/lib/sha2/src/thash_sha2_{robust,simple}.c

use crate::context::SpxCtx;
use crate::params::{SPX_N};
use crate::sha2::{
    mgf1_256, sha256_inc_finalize, SPX_SHA256_ADDR_BYTES, SPX_SHA256_OUTPUT_BYTES,
};

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
use crate::sha2::{mgf1_512, sha512_inc_finalize, SPX_SHA512_OUTPUT_BYTES};

#[cfg(feature = "robust")]
pub fn thash(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if inblocks > 1 {
            thash_512(out, in_buf, inblocks as u32, ctx, addr);
            return;
        }
    }
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(
        &mut bitmask,
        inblocks * SPX_N,
        &buf,
        SPX_N + SPX_SHA256_ADDR_BYTES,
    );

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..(inblocks * SPX_N) {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = in_buf[i] ^ bitmask[i];
    }

    let in_data = buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N].to_vec();
    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &in_data,
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(
    feature = "robust",
    any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")
))]
fn thash_512(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 72];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_512(
        &mut bitmask,
        inblocks * SPX_N,
        &buf,
        SPX_N + SPX_SHA256_ADDR_BYTES,
    );

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..(inblocks * SPX_N) {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = in_buf[i] ^ bitmask[i];
    }

    let in_data = buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N].to_vec();
    sha512_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &in_data,
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "simple")]
pub fn thash(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if inblocks > 1 {
            thash_simple_512(out, in_buf, inblocks as u32, ctx, addr);
            return;
        }
    }
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    sha2_state.copy_from_slice(&ctx.state_seeded);

    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&in_buf[..inblocks * SPX_N]);

    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf,
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(
    feature = "simple",
    any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")
))]
fn thash_simple_512(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&in_buf[..inblocks * SPX_N]);

    sha512_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf,
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
