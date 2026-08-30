//! Translation of `lib/sha2/src/hash_sha2.c`.

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::{
    sha256_inc_finalize, SPX_SHA256_OUTPUT_BYTES,
};
use crate::utils::bytes_to_ull;

const SPX_SHA256_ADDR_BYTES: usize = 22;

// Select the "SHA-X" primitive family (SHA-512 for N >= 24, else SHA-256).
#[cfg(spx_sha512)]
mod shax {
    pub use crate::sha2::{
        mgf1_512 as mgf1_x, sha512 as shax, sha512_inc_blocks as shax_inc_blocks,
        sha512_inc_finalize as shax_inc_finalize, sha512_inc_init as shax_inc_init,
    };
    pub const OUT: usize = 64;
    pub const BLOCK: usize = 128;
    pub const STATE: usize = 72;
}
#[cfg(not(spx_sha512))]
mod shax {
    pub use crate::sha2::{
        mgf1_256 as mgf1_x, sha256 as shax, sha256_inc_blocks as shax_inc_blocks,
        sha256_inc_finalize as shax_inc_finalize, sha256_inc_init as shax_inc_init,
    };
    pub const OUT: usize = 32;
    pub const BLOCK: usize = 64;
    pub const STATE: usize = 40;
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    crate::sha2::seed_state(ctx);
}

/// Computes PRF(pk_seed, sk_seed, addr). Always uses SHA-256.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let ab = addr_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Computes the message-dependent randomness R (HMAC-SHA-X construction).
pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    use shax::*;
    let mut buf = [0u8; BLOCK + OUT];
    let mut state = [0u8; STATE];
    let mlen = m.len();

    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..BLOCK {
        buf[i] = 0x36;
    }

    shax_inc_init(&mut state);
    shax_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    if SPX_N + mlen < BLOCK {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let (head, tail) = buf.split_at_mut(BLOCK);
        shax_inc_finalize(&mut tail[..OUT], &mut state, &head[..mlen + SPX_N], mlen + SPX_N);
    } else {
        buf[SPX_N..BLOCK].copy_from_slice(&m[..BLOCK - SPX_N]);
        shax_inc_blocks(&mut state, &buf, 1);
        let m2 = &m[BLOCK - SPX_N..];
        let mlen2 = mlen - (BLOCK - SPX_N);
        let (_head, tail) = buf.split_at_mut(BLOCK);
        shax_inc_finalize(&mut tail[..OUT], &mut state, m2, mlen2);
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..BLOCK {
        buf[i] = 0x5c;
    }

    let tmp = buf;
    shax(&mut buf, &tmp, BLOCK + OUT);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

const INBLOCKS: usize =
    ((SPX_N + SPX_PK_BYTES + shax::BLOCK - 1) & !(shax::BLOCK - 1)) / shax::BLOCK;

/// Computes the message hash and splits it into tree/leaf indices.
pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    use shax::*;
    let mut seed = [0u8; 2 * SPX_N + OUT];
    let mut inbuf = [0u8; INBLOCKS * BLOCK];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; STATE];
    let mlen = m.len();

    shax_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < INBLOCKS * BLOCK {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        shax_inc_finalize(
            &mut seed[2 * SPX_N..2 * SPX_N + OUT],
            &mut state,
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        let first = INBLOCKS * BLOCK - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..INBLOCKS * BLOCK].copy_from_slice(&m[..first]);
        shax_inc_blocks(&mut state, &inbuf, INBLOCKS);
        let m2 = &m[first..];
        let mlen2 = mlen - first;
        shax_inc_finalize(&mut seed[2 * SPX_N..2 * SPX_N + OUT], &mut state, m2, mlen2);
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_x(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + OUT);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    initialize_hash_function(&mut *ctx);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let o = core::slice::from_raw_parts_mut(out, SPX_N);
    prf_addr(o, &*ctx, &*(addr as *const [u32; 8]));
}

#[no_mangle]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ctx: *const SpxCtx,
) {
    let r_s = core::slice::from_raw_parts_mut(r, SPX_N);
    let sk = core::slice::from_raw_parts(sk_prf, SPX_N);
    let opt = core::slice::from_raw_parts(optrand, SPX_N);
    let m_s = core::slice::from_raw_parts(m, mlen as usize);
    gen_message_random(r_s, sk, opt, m_s, &*ctx);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ctx: *const SpxCtx,
) {
    let d = core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES);
    let r_s = core::slice::from_raw_parts(r, SPX_N);
    let pk_s = core::slice::from_raw_parts(pk, SPX_PK_BYTES);
    let m_s = core::slice::from_raw_parts(m, mlen as usize);
    hash_message(d, &mut *tree, &mut *leaf_idx, r_s, pk_s, m_s, &*ctx);
}
