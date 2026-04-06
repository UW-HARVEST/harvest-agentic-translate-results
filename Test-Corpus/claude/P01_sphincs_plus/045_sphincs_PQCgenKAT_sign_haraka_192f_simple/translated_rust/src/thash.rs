use crate::context::SpxCtx;
use crate::params::*;

// ===== SHAKE simple =====
#[cfg(all(feature = "shake", feature = "simple"))]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);
    crate::fips202::shake256(out, SPX_N, &buf, buf_len);
}

// ===== SHAKE robust =====
#[cfg(all(feature = "shake", feature = "robust"))]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    crate::fips202::shake256(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES], SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    crate::fips202::shake256(out, SPX_N, &buf, buf_len);
}

// ===== SHA2 simple =====
#[cfg(all(feature = "sha2", feature = "simple"))]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    if inblocks > 1 {
        thash_sha2_512_simple(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_SHA256_ADDR_BYTES) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    crate::sha2_impl::sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "sha2", feature = "simple", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
fn thash_sha2_512_simple(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_SHA256_ADDR_BYTES) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    crate::sha2_impl::sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ===== SHA2 robust =====
#[cfg(all(feature = "sha2", feature = "robust"))]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    if inblocks > 1 {
        thash_sha2_512_robust(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_SHA256_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(addr_bytes);
    crate::sha2_impl::mgf1_256(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES], SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    crate::sha2_impl::sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "sha2", feature = "robust", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
fn thash_sha2_512_robust(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 72];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_SHA256_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(addr_bytes);
    crate::sha2_impl::mgf1_512(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES], SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    crate::sha2_impl::sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ===== BLAKE simple =====
#[cfg(all(feature = "blake", feature = "simple"))]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    if inblocks > 1 {
        thash_blake512_simple(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    crate::blake_impl::blake256(&mut outbuf, &buf, buf_len as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "blake", feature = "simple", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
fn thash_blake512_simple(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    crate::blake_impl::blake512(&mut outbuf, &buf, buf_len as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ===== BLAKE robust =====
#[cfg(all(feature = "blake", feature = "robust"))]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    if inblocks > 1 {
        thash_blake512_robust(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    crate::blake_impl::blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES], SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    crate::blake_impl::blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "blake", feature = "robust", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
fn thash_blake512_robust(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    crate::blake_impl::blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES], SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    crate::blake_impl::blake512(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ===== HARAKA simple =====
#[cfg(all(feature = "haraka", feature = "simple"))]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };

    if inblocks == 1 {
        let mut buf_tmp = [0u8; 64];
        let mut outbuf = [0u8; 32];
        buf_tmp[..32].copy_from_slice(addr_bytes);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&inp[..SPX_N]);
        crate::haraka_impl::haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let buf_len = SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; buf_len];
        buf[..32].copy_from_slice(addr_bytes);
        buf[SPX_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);
        crate::haraka_impl::haraka_S(out, SPX_N as u64, &buf, buf_len as u64, ctx);
    }
}

// ===== HARAKA robust =====
#[cfg(all(feature = "haraka", feature = "robust"))]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };

    if inblocks == 1 {
        let mut buf_tmp = [0u8; 64];
        let mut outbuf = [0u8; 32];
        buf_tmp[..32].copy_from_slice(addr_bytes);

        let mut mask_out = [0u8; 32];
        crate::haraka_impl::haraka256(&mut mask_out, &buf_tmp[..32], ctx);
        for i in 0..inblocks * SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = inp[i] ^ mask_out[i];
        }
        crate::haraka_impl::haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let buf_len = SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; buf_len];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        buf[..32].copy_from_slice(addr_bytes);
        crate::haraka_impl::haraka_S(&mut bitmask, (inblocks * SPX_N) as u64, &buf[..SPX_ADDR_BYTES], SPX_ADDR_BYTES as u64, ctx);

        for i in 0..inblocks * SPX_N {
            buf[SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
        }

        crate::haraka_impl::haraka_S(out, SPX_N as u64, &buf, buf_len as u64, ctx);
    }
}
