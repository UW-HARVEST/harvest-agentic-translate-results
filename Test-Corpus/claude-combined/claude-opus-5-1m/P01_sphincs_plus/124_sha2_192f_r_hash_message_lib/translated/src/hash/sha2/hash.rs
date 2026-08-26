// Translation of c_src/lib/sha2/src/hash_sha2.c

use core::slice;

use crate::context::SpxCtx;
use crate::params::{
    SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_SHA256_ADDR_BYTES,
    SPX_SHA256_BLOCK_BYTES, SPX_SHA256_OUTPUT_BYTES, SPX_SHA512, SPX_SHA512_BLOCK_BYTES,
    SPX_SHA512_OUTPUT_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

use super::sha2::{
    mgf1_256_inner, mgf1_512_inner, seed_state_inner, sha256_inc_blocks_inner,
    sha256_inc_finalize_inner, sha256_inc_init_inner, sha256_one, sha512_inc_blocks_inner,
    sha512_inc_finalize_inner, sha512_inc_init_inner, sha512_one,
};

const fn shax_block_bytes() -> usize {
    if SPX_N >= 24 { SPX_SHA512_BLOCK_BYTES } else { SPX_SHA256_BLOCK_BYTES }
}
const fn shax_output_bytes() -> usize {
    if SPX_N >= 24 { SPX_SHA512_OUTPUT_BYTES } else { SPX_SHA256_OUTPUT_BYTES }
}
const fn shax_state_len() -> usize {
    if SPX_N >= 24 { 72 } else { 40 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    seed_state_inner(ctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let out = unsafe { slice::from_raw_parts_mut(out, SPX_N) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { slice::from_raw_parts(addr, 8) };
    prf_addr_inner(out, ctx, addr);
}

pub fn prf_addr_inner(out: &mut [u8], ctx: &SpxCtx, addr: &[u32]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize_inner(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    let r = unsafe { slice::from_raw_parts_mut(r, SPX_N) };
    let sk_prf = unsafe { slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand = unsafe { slice::from_raw_parts(optrand, SPX_N) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };

    let bb = shax_block_bytes();
    let ob = shax_output_bytes();
    let sl = shax_state_len();

    let mut buf = vec![0u8; bb + ob];
    let mut state = vec![0u8; sl];

    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..bb {
        buf[i] = 0x36;
    }

    if bb == SPX_SHA512_BLOCK_BYTES {
        sha512_inc_init_inner(&mut state);
        sha512_inc_blocks_inner(&mut state, &buf, 1);
    } else {
        sha256_inc_init_inner(&mut state);
        sha256_inc_blocks_inner(&mut state, &buf, 1);
    }

    buf[..SPX_N].copy_from_slice(optrand);

    if SPX_N + (mlen as usize) < bb {
        buf[SPX_N..SPX_N + mlen as usize].copy_from_slice(m);
        let (head, tail) = buf.split_at_mut(bb);
        if bb == SPX_SHA512_BLOCK_BYTES {
            sha512_inc_finalize_inner(tail, &mut state, head, mlen as usize + SPX_N);
        } else {
            sha256_inc_finalize_inner(tail, &mut state, head, mlen as usize + SPX_N);
        }
    } else {
        let take = bb - SPX_N;
        buf[SPX_N..SPX_N + take].copy_from_slice(&m[..take]);
        if bb == SPX_SHA512_BLOCK_BYTES {
            sha512_inc_blocks_inner(&mut state, &buf[..bb], 1);
        } else {
            sha256_inc_blocks_inner(&mut state, &buf[..bb], 1);
        }
        let m_rest = &m[take..];
        let mlen_rest = m_rest.len();

        let (head, tail) = buf.split_at_mut(bb);
        let _ = head;
        if bb == SPX_SHA512_BLOCK_BYTES {
            sha512_inc_finalize_inner(tail, &mut state, m_rest, mlen_rest);
        } else {
            sha256_inc_finalize_inner(tail, &mut state, m_rest, mlen_rest);
        }
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..bb {
        buf[i] = 0x5c;
    }

    let total = bb + ob;
    let mut tmp = vec![0u8; total];
    if bb == SPX_SHA512_BLOCK_BYTES {
        sha512_one(&mut tmp[..ob], &buf[..total]);
    } else {
        sha256_one(&mut tmp[..ob], &buf[..total]);
    }
    r.copy_from_slice(&tmp[..SPX_N]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let digest = unsafe { slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let r_slice = unsafe { slice::from_raw_parts(r, SPX_N) };
    let pk_slice = unsafe { slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };

    let bb = shax_block_bytes();
    let ob = shax_output_bytes();
    let sl = shax_state_len();

    let inblocks = (SPX_N + SPX_PK_BYTES + bb - 1) / bb;

    let mut seed = vec![0u8; 2 * SPX_N + ob];
    let mut inbuf = vec![0u8; inblocks * bb];
    let mut state = vec![0u8; sl];

    if bb == SPX_SHA512_BLOCK_BYTES {
        sha512_inc_init_inner(&mut state);
    } else {
        sha256_inc_init_inner(&mut state);
    }

    inbuf[..SPX_N].copy_from_slice(r_slice);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(pk_slice);

    let total_inbuf = inblocks * bb;
    if SPX_N + SPX_PK_BYTES + (mlen as usize) < total_inbuf {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen as usize].copy_from_slice(m);
        if bb == SPX_SHA512_BLOCK_BYTES {
            sha512_inc_finalize_inner(
                &mut seed[2 * SPX_N..],
                &mut state,
                &inbuf,
                SPX_N + SPX_PK_BYTES + mlen as usize,
            );
        } else {
            sha256_inc_finalize_inner(
                &mut seed[2 * SPX_N..],
                &mut state,
                &inbuf,
                SPX_N + SPX_PK_BYTES + mlen as usize,
            );
        }
    } else {
        let take = total_inbuf - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + take].copy_from_slice(&m[..take]);
        if bb == SPX_SHA512_BLOCK_BYTES {
            sha512_inc_blocks_inner(&mut state, &inbuf, inblocks);
        } else {
            sha256_inc_blocks_inner(&mut state, &inbuf, inblocks);
        }
        let m_rest = &m[take..];
        if bb == SPX_SHA512_BLOCK_BYTES {
            sha512_inc_finalize_inner(&mut seed[2 * SPX_N..], &mut state, m_rest, m_rest.len());
        } else {
            sha256_inc_finalize_inner(&mut seed[2 * SPX_N..], &mut state, m_rest, m_rest.len());
        }
    }

    seed[..SPX_N].copy_from_slice(r_slice);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk_slice[..SPX_N]);

    let mut buf = vec![0u8; SPX_DGST_BYTES];
    if bb == SPX_SHA512_BLOCK_BYTES {
        mgf1_512_inner(&mut buf, &seed);
    } else {
        mgf1_256_inner(&mut buf, &seed);
    }

    digest.copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    let tree_val: u64 = if SPX_D == 1 {
        0
    } else {
        let v = bytes_to_ull(&buf[bufp..bufp + SPX_TREE_BYTES]);
        v & ((!0u64) >> (64 - SPX_TREE_BITS))
    };
    unsafe { *tree = tree_val; }
    bufp += SPX_TREE_BYTES;

    let leaf_idx_val =
        bytes_to_ull(&buf[bufp..bufp + SPX_LEAF_BYTES]) as u32 & ((!0u32) >> (32 - SPX_LEAF_BITS));
    unsafe { *leaf_idx = leaf_idx_val; }

    let _ = SPX_SHA512;
}
