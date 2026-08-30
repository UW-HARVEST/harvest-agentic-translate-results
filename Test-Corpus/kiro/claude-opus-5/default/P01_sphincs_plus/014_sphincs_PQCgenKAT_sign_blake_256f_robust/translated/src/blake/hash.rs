//! Translation of `lib/blake/src/hash_blake.c`.

use core::ffi::c_ulonglong;

use crate::address::addr_bytes;
use crate::blake::SPX_BLAKE512;
use crate::blake::blake256::{
    SPX_BLAKE256_OUTPUT_BYTES, BlakeState256, blake256, blake256_final, blake256_init,
    blake256_mgf1, blake256_update,
};
use crate::blake::blake512::{
    SPX_BLAKE512_OUTPUT_BYTES, BlakeState512, blake512_final, blake512_init, blake512_mgf1,
    blake512_update,
};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::bytes_to_ull;

/// `SPX_BLAKEX_OUTPUT_BYTES`
pub const SPX_BLAKEX_OUTPUT_BYTES: usize = if SPX_BLAKE512 {
    SPX_BLAKE512_OUTPUT_BYTES
} else {
    SPX_BLAKE256_OUTPUT_BYTES
};

/// `blakestateX` (either a `blakestate256` or a `blakestate512`).
struct BlakeStateX {
    s256: BlakeState256,
    s512: BlakeState512,
}

impl BlakeStateX {
    fn new() -> Self {
        BlakeStateX {
            s256: BlakeState256::new(),
            s512: BlakeState512::new(),
        }
    }

    fn init(&mut self) {
        if SPX_BLAKE512 {
            blake512_init(&mut self.s512);
        } else {
            blake256_init(&mut self.s256);
        }
    }

    /// `blakeX_update`.  `hash_blake.c` passes byte counts where the reference
    /// implementation expects a bit count; that is reproduced verbatim.
    fn update(&mut self, data: &[u8], datalen: u64) {
        if SPX_BLAKE512 {
            blake512_update(&mut self.s512, data, datalen);
        } else {
            blake256_update(&mut self.s256, data, datalen);
        }
    }

    fn finalize(&mut self, out: &mut [u8]) {
        if SPX_BLAKE512 {
            blake512_final(&mut self.s512, out);
        } else {
            blake256_final(&mut self.s256, out);
        }
    }
}

#[inline]
fn blakex_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    if SPX_BLAKE512 {
        blake512_mgf1(out, outlen as core::ffi::c_ulong, inp, inlen as core::ffi::c_ulong);
    } else {
        blake256_mgf1(out, outlen as core::ffi::c_ulong, inp, inlen as core::ffi::c_ulong);
    }
}

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

/// Computes `PRF(key, addr)`, given a secret key of `SPX_N` bytes and an
/// address.
///
/// Note that the C code only hashes `SPX_N + SPX_ADDR_BYTES` bytes even though
/// it fills `2 * SPX_N + SPX_ADDR_BYTES`, and always uses BLAKE-256; both are
/// reproduced as-is.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);

    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Computes the message-dependent randomness `R`, using a secret seed and an
/// optional randomization value as well as the message.
///
/// The C code hands `blakeX_final` the raw `R` pointer, so it writes
/// `SPX_BLAKEX_OUTPUT_BYTES` there even though only `SPX_N` bytes belong to
/// `R`.  In `crypto_sign_signature` the surplus lands in the part of the
/// signature buffer that `fors_sign` overwrites immediately afterwards, so
/// only `r_out.len()` bytes are handed back here.
pub fn gen_message_random(
    r_out: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut s = BlakeStateX::new();
    let mut outbuf = [0u8; SPX_BLAKEX_OUTPUT_BYTES];

    s.init();
    s.update(sk_prf, SPX_N as u64);
    s.update(optrand, SPX_N as u64);
    s.update(m, m.len() as u64);
    s.finalize(&mut outbuf);

    let n = r_out.len().min(SPX_BLAKEX_OUTPUT_BYTES);
    r_out[..n].copy_from_slice(&outbuf[..n]);
}

/// Computes the message hash using `R`, the public key, and the message.
pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r_in: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakeStateX::new();
    s.init();

    s.update(r_in, SPX_N as u64);
    s.update(pk, SPX_PK_BYTES as u64);
    s.update(m, m.len() as u64);

    s.finalize(&mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r_in[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blakex_mgf1(
        &mut buf,
        SPX_DGST_BYTES,
        &seed,
        2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES,
    );

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..bufp + SPX_TREE_BYTES]);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..bufp + SPX_LEAF_BYTES]) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ---------------------------------------------------------------------------
// C ABI.  `hash.h` renames everything through `SPX_NAMESPACE`.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe { initialize_hash_function(&mut *ctx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        prf_addr(
            core::slice::from_raw_parts_mut(out, SPX_N),
            &*ctx,
            &*(addr as *const [u32; 8]),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r_out: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    unsafe {
        gen_message_random(
            core::slice::from_raw_parts_mut(r_out, SPX_BLAKEX_OUTPUT_BYTES),
            core::slice::from_raw_parts(sk_prf, SPX_N),
            core::slice::from_raw_parts(optrand, SPX_N),
            core::slice::from_raw_parts(m, mlen as usize),
            &*ctx,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r_in: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    unsafe {
        hash_message(
            core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES),
            &mut *tree,
            &mut *leaf_idx,
            core::slice::from_raw_parts(r_in, SPX_N),
            core::slice::from_raw_parts(pk, SPX_PK_BYTES),
            core::slice::from_raw_parts(m, mlen as usize),
            &*ctx,
        )
    }
}
