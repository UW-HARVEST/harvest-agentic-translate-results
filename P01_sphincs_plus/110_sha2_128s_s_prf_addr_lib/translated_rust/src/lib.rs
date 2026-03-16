#![allow(non_snake_case, non_upper_case_globals, unused_imports)]

mod params;
mod context;
mod address;
mod sha2;
mod hash;
mod thash;
mod utils;
mod wots;
mod fors;
mod merkle;
mod sign;
mod randombytes;

use std::ffi::c_void;
use params::*;
use context::*;

// ============ API exports ============

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
        sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        sign::crypto_sign_keypair(pk_s, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    unsafe {
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sk_s = std::slice::from_raw_parts(sk, SPX_SK_BYTES);
        sign::crypto_sign_signature(sig_s, &mut *siglen, m_s, mlen, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    unsafe {
        let sig_s = std::slice::from_raw_parts(sig, if siglen > SPX_BYTES { siglen } else { SPX_BYTES });
        let m_s = std::slice::from_raw_parts(m, mlen);
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        sign::crypto_sign_verify(sig_s, siglen, m_s, mlen, pk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    unsafe {
        let sm_s = std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);
        let sk_s = std::slice::from_raw_parts(sk, SPX_SK_BYTES);

        let mut siglen: usize = 0;
        sign::crypto_sign_signature(sm_s, &mut siglen, m_s, mlen as usize, sk_s);

        // memmove sm + SPX_BYTES <- m
        std::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen as u64) + mlen;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    unsafe {
        let smlen_usize = smlen as usize;
        if smlen_usize < SPX_BYTES {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }

        let msg_len = smlen_usize - SPX_BYTES;
        *mlen = msg_len as u64;

        let sm_s = std::slice::from_raw_parts(sm, smlen_usize);
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);

        if sign::crypto_sign_verify(&sm_s[..SPX_BYTES], SPX_BYTES, &sm_s[SPX_BYTES..], msg_len, pk_s) != 0 {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }

        std::ptr::copy(sm.add(SPX_BYTES), m, msg_len);
        0
    }
}

// ============ Address function exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    unsafe {
        let a = &mut *(addr as *mut [u32; 8]);
        address::set_layer_addr(a, layer);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    unsafe {
        let a = &mut *(addr as *mut [u32; 8]);
        address::set_tree_addr(a, tree);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    unsafe {
        let a = &mut *(addr as *mut [u32; 8]);
        address::set_type(a, type_val);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    unsafe {
        let o = &mut *(out as *mut [u32; 8]);
        let i = &*(inp as *const [u32; 8]);
        address::copy_subtree_addr(o, i);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    unsafe {
        let a = &mut *(addr as *mut [u32; 8]);
        address::set_keypair_addr(a, keypair);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    unsafe {
        let a = &mut *(addr as *mut [u32; 8]);
        address::set_chain_addr(a, chain);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    unsafe {
        let a = &mut *(addr as *mut [u32; 8]);
        address::set_hash_addr(a, hash);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    unsafe {
        let o = &mut *(out as *mut [u32; 8]);
        let i = &*(inp as *const [u32; 8]);
        address::copy_keypair_addr(o, i);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    unsafe {
        let a = &mut *(addr as *mut [u32; 8]);
        address::set_tree_height(a, tree_height);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    unsafe {
        let a = &mut *(addr as *mut [u32; 8]);
        address::set_tree_index(a, tree_index);
    }
}

// ============ Utils exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    unsafe {
        let s = std::slice::from_raw_parts_mut(out, outlen as usize);
        address::ull_to_bytes(s, outlen as usize, val);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    unsafe {
        let s = std::slice::from_raw_parts_mut(out, 4);
        address::u32_to_bytes(s, val);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    unsafe {
        let s = std::slice::from_raw_parts(inp, inlen as usize);
        address::bytes_to_ull(s, inlen as usize)
    }
}

// ============ SHA2 exports ============

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_init(state: *mut u8) {
    unsafe {
        let s = std::slice::from_raw_parts_mut(state, 40);
        sha2::sha256_inc_init(s);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
    unsafe {
        let s = std::slice::from_raw_parts_mut(state, 40);
        let d = std::slice::from_raw_parts(inp, 64 * inblocks);
        sha2::sha256_inc_blocks(s, d, inblocks);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_finalize(out: *mut u8, state: *mut u8, inp: *const u8, inlen: usize) {
    unsafe {
        let o = std::slice::from_raw_parts_mut(out, 32);
        let s = std::slice::from_raw_parts_mut(state, 40);
        let d = std::slice::from_raw_parts(inp, inlen);
        sha2::sha256_inc_finalize(o, s, d, inlen);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256(out: *mut u8, inp: *const u8, inlen: usize) {
    unsafe {
        let o = std::slice::from_raw_parts_mut(out, 32);
        let d = std::slice::from_raw_parts(inp, inlen);
        sha2::sha256(o, d, inlen);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_init(state: *mut u8) {
    unsafe {
        let s = std::slice::from_raw_parts_mut(state, 72);
        sha2::sha512_inc_init(s);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
    unsafe {
        let s = std::slice::from_raw_parts_mut(state, 72);
        let d = std::slice::from_raw_parts(inp, 128 * inblocks);
        sha2::sha512_inc_blocks(s, d, inblocks);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_finalize(out: *mut u8, state: *mut u8, inp: *const u8, inlen: usize) {
    unsafe {
        let o = std::slice::from_raw_parts_mut(out, 64);
        let s = std::slice::from_raw_parts_mut(state, 72);
        let d = std::slice::from_raw_parts(inp, inlen);
        sha2::sha512_inc_finalize(o, s, d, inlen);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512(out: *mut u8, inp: *const u8, inlen: usize) {
    unsafe {
        let o = std::slice::from_raw_parts_mut(out, 64);
        let d = std::slice::from_raw_parts(inp, inlen);
        sha2::sha512(o, d, inlen);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_256(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    unsafe {
        let o = std::slice::from_raw_parts_mut(out, outlen as usize);
        let i = std::slice::from_raw_parts(inp, inlen as usize);
        sha2::mgf1_256(o, outlen as usize, i, inlen as usize);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_512(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    unsafe {
        let o = std::slice::from_raw_parts_mut(out, outlen as usize);
        let i = std::slice::from_raw_parts(inp, inlen as usize);
        sha2::mgf1_512(o, outlen as usize, i, inlen as usize);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_seed_state(ctx: *mut SpxCtx) {
    unsafe {
        sha2::seed_state(&mut *ctx);
    }
}

// ============ Hash exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe {
        hash::initialize_hash_function(&mut *ctx);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        let o = std::slice::from_raw_parts_mut(out, SPX_N);
        let a = &*(addr as *const [u32; 8]);
        hash::prf_addr(o, &*ctx, a);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    unsafe {
        let r_s = std::slice::from_raw_parts_mut(r, SPX_N);
        let sk_s = std::slice::from_raw_parts(sk_prf, SPX_N);
        let opt_s = std::slice::from_raw_parts(optrand, SPX_N);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);
        hash::gen_message_random(r_s, sk_s, opt_s, m_s, mlen as usize, &*ctx);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    unsafe {
        let d = std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES);
        let r_s = std::slice::from_raw_parts(r, SPX_N);
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);
        hash::hash_message(d, &mut *tree, &mut *leaf_idx, r_s, pk_s, m_s, mlen as usize, &*ctx);
    }
}

// ============ Thash export ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let o = std::slice::from_raw_parts_mut(out, SPX_N);
        let i = std::slice::from_raw_parts(inp, inblocks as usize * SPX_N);
        let a = &mut *(addr as *mut [u32; 8]);
        thash::thash(o, i, inblocks as usize, &*ctx, a);
    }
}

// ============ Randombytes export ============

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) {
    unsafe {
        let s = std::slice::from_raw_parts_mut(x, xlen as usize);
        randombytes::randombytes(s, xlen as usize);
    }
}

// ============ WOTS exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES);
        let sig_s = std::slice::from_raw_parts(sig, SPX_WOTS_BYTES);
        let msg_s = std::slice::from_raw_parts(msg, SPX_N);
        let a = &mut *(addr as *mut [u32; 8]);
        wots::wots_pk_from_sig(pk_s, sig_s, msg_s, &*ctx, a);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    unsafe {
        let l = std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN);
        let m = std::slice::from_raw_parts(msg, SPX_N);
        wots::chain_lengths(l, m);
    }
}

// ============ FORS exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    unsafe {
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES);
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_N);
        let m_s = std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES);
        let a = &*(fors_addr as *const [u32; 8]);
        fors::fors_sign(sig_s, pk_s, m_s, &*ctx, a);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_N);
        let sig_s = std::slice::from_raw_parts(sig, SPX_FORS_BYTES);
        let m_s = std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES);
        let a = &*(fors_addr as *const [u32; 8]);
        fors::fors_pk_from_sig(pk_s, sig_s, m_s, &*ctx, a);
    }
}

// ============ Merkle exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    unsafe {
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N);
        let root_s = std::slice::from_raw_parts_mut(root, SPX_N);
        let wa = &mut *(wots_addr as *mut [u32; 8]);
        let ta = &mut *(tree_addr as *mut [u32; 8]);
        merkle::merkle_sign(sig_s, root_s, &*ctx, wa, ta, idx_leaf);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    unsafe {
        let r = std::slice::from_raw_parts_mut(root, SPX_N);
        merkle::merkle_gen_root(r, &*ctx);
    }
}
