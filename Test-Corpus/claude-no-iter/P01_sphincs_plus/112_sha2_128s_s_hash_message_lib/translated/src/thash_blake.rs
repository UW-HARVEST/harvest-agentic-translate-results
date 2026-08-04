// Translation of c_src/lib/blake/src/thash_blake_{robust,simple}.c

use crate::blake::{blake256, SPX_BLAKE256_OUTPUT_BYTES};
use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_N};

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
use crate::blake::{blake512, SPX_BLAKE512_OUTPUT_BYTES};

#[cfg(all(feature = "robust", any(feature = "128s", feature = "128f")))]
use crate::blake::blake256_mgf1;

#[cfg(all(feature = "robust", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
use crate::blake::blake512_mgf1;

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

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);

    #[cfg(any(feature = "128s", feature = "128f"))]
    blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        // For inblocks=1, the C code calls blake256_mgf1 unconditionally.
        crate::blake::blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);
    }

    for i in 0..(inblocks * SPX_N) {
        buf[SPX_N + SPX_ADDR_BYTES + i] = in_buf[i] ^ bitmask[i];
    }

    blake256(
        &mut outbuf,
        &buf[SPX_N..],
        (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(
    feature = "robust",
    any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")
))]
fn thash_512(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);

    blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..(inblocks * SPX_N) {
        buf[SPX_N + SPX_ADDR_BYTES + i] = in_buf[i] ^ bitmask[i];
    }

    blake512(
        &mut outbuf,
        &buf[SPX_N..],
        (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
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

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&in_buf[..inblocks * SPX_N]);

    blake256(
        &mut outbuf,
        &buf,
        (SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(
    feature = "simple",
    any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")
))]
fn thash_simple_512(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&in_buf[..inblocks * SPX_N]);

    blake512(
        &mut outbuf,
        &buf,
        (SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
