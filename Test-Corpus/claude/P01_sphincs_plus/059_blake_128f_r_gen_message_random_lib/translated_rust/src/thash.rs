use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_N};

#[cfg(feature = "sha2")]
mod backend {
    use super::*;
    use crate::sha2_impl::*;

    const SPX_SHA256_ADDR_BYTES: usize = 22;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn thash_512_robust(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
        let mut sha2_state = [0u8; 72];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
        let buf_prefix = buf[..SPX_N + SPX_SHA256_ADDR_BYTES].to_vec();
        mgf1_512(&mut bitmask, inblocks * SPX_N, &buf_prefix, SPX_N + SPX_SHA256_ADDR_BYTES);

        sha2_state.copy_from_slice(&ctx.state_seeded_512);

        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        let buf_for_finalize = buf[SPX_N..].to_vec();
        sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf_for_finalize, SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn thash_512_simple(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
        let mut sha2_state = [0u8; 72];
        let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

        sha2_state.copy_from_slice(&ctx.state_seeded_512);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
        buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);
        sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    #[cfg(feature = "robust")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        if inblocks > 1 {
            thash_512_robust(out, input, inblocks, ctx, addr);
            return;
        }
        let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
        let mut sha2_state = [0u8; 40];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
        let buf_prefix = buf[..SPX_N + SPX_SHA256_ADDR_BYTES].to_vec();
        mgf1_256(&mut bitmask, inblocks * SPX_N, &buf_prefix, SPX_N + SPX_SHA256_ADDR_BYTES);

        sha2_state.copy_from_slice(&ctx.state_seeded);

        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        let buf_for_finalize = buf[SPX_N..].to_vec();
        sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf_for_finalize, SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    #[cfg(feature = "simple")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        if inblocks > 1 {
            thash_512_simple(out, input, inblocks, ctx, addr);
            return;
        }

        let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
        let mut sha2_state = [0u8; 40];
        let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

        sha2_state.copy_from_slice(&ctx.state_seeded);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
        buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);
        sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

#[cfg(feature = "shake")]
mod backend {
    use super::*;
    use crate::fips202::shake256;

    #[cfg(feature = "robust")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
        let mut bitmask = vec![0u8; inblocks * SPX_N];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);

        let prefix = buf[..SPX_N + SPX_ADDR_BYTES].to_vec();
        shake256(&mut bitmask, inblocks * SPX_N, &prefix, SPX_N + SPX_ADDR_BYTES);

        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
        let buf_copy = buf.clone();
        shake256(out, SPX_N, &buf_copy, total);
    }

    #[cfg(feature = "simple")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);
        let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
        let buf_copy = buf.clone();
        shake256(out, SPX_N, &buf_copy, total);
    }
}

#[cfg(feature = "blake")]
mod backend {
    use super::*;
    use crate::blake::*;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn thash_512_simple(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
        let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);
        let total = (SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N) as u64;
        blake512(&mut outbuf, &buf, total);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn thash_512_robust(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);

        blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES], SPX_N + SPX_ADDR_BYTES);

        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }
        // blake512(buf + SPX_N, ...)
        let inlen = (SPX_ADDR_BYTES + inblocks * SPX_N) as u64;
        let buf_offset = buf[SPX_N..].to_vec();
        blake512(&mut outbuf, &buf_offset, inlen);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    #[cfg(feature = "robust")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        if inblocks > 1 {
            thash_512_robust(out, input, inblocks, ctx, addr);
            return;
        }
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);

        blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES], SPX_N + SPX_ADDR_BYTES);

        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }
        let inlen = (SPX_ADDR_BYTES + inblocks * SPX_N) as u64;
        let buf_offset = buf[SPX_N..].to_vec();
        blake256(&mut outbuf, &buf_offset, inlen);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    #[cfg(feature = "simple")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        if inblocks > 1 {
            thash_512_simple(out, input, inblocks, ctx, addr);
            return;
        }
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);
        let total = (SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N) as u64;
        blake256(&mut outbuf, &buf, total);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

#[cfg(feature = "haraka")]
mod backend {
    use super::*;
    use crate::haraka::*;

    #[cfg(feature = "simple")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut outbuf = [0u8; 32];
        if inblocks == 1 {
            let mut buf_tmp = [0u8; 64];
            let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
            buf_tmp[..32].copy_from_slice(addr_bytes);
            buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&input[..SPX_N]);
            haraka512(&mut outbuf, &buf_tmp, ctx);
            out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
        } else {
            let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
            let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
            buf[..32].copy_from_slice(addr_bytes);
            buf[SPX_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);
            haraka_s(out, SPX_N as u64, &buf, (SPX_ADDR_BYTES + inblocks * SPX_N) as u64, ctx);
        }
    }

    #[cfg(feature = "robust")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut outbuf = [0u8; 32];
        if inblocks == 1 {
            let mut buf_tmp = [0u8; 64];
            let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
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
            let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
            buf[..32].copy_from_slice(addr_bytes);
            haraka_s(&mut bitmask, (inblocks * SPX_N) as u64, &buf, SPX_ADDR_BYTES as u64, ctx);
            for i in 0..inblocks * SPX_N {
                buf[SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
            }
            haraka_s(out, SPX_N as u64, &buf, (SPX_ADDR_BYTES + inblocks * SPX_N) as u64, ctx);
        }
    }
}

pub use backend::thash;

// C-ABI export
#[unsafe(export_name = "SPX_thash")]
pub unsafe extern "C" fn spx_thash(
    out: *mut u8,
    input: *const u8,
    inblocks: core::ffi::c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let inblocks = inblocks as usize;
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    let in_slice = unsafe { core::slice::from_raw_parts(input, inblocks * SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let addr_ref = unsafe { &mut *(addr as *mut [u32; 8]) };
    thash(out_slice, in_slice, inblocks, ctx_ref, addr_ref);
}
