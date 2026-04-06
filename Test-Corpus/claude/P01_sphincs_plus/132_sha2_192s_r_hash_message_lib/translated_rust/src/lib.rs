#![allow(non_snake_case)]
#![allow(clippy::needless_range_loop)]

pub mod params;
pub mod context;
pub mod address;
pub mod utils;
pub mod rng;
pub mod hash;
pub mod thash;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod utilsx1;
pub mod merkle;
pub mod sign;
pub mod fips202;
pub mod sha2_impl;
pub mod blake_impl;
#[cfg(feature = "haraka")]
pub mod haraka_impl;

use params::*;
use std::slice;

// ====== crypto_sign API ======

#[no_mangle]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[no_mangle]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[no_mangle]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[no_mangle]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> i32 {
    let pk_s = slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
    let sk_s = slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
    let seed_s = slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
    sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let pk_s = slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
    let sk_s = slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
    sign::crypto_sign_keypair(pk_s, sk_s)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_signature(sig: *mut u8, siglen: *mut usize, m: *const u8, mlen: usize, sk: *const u8) -> i32 {
    let sig_s = slice::from_raw_parts_mut(sig, CRYPTO_BYTES);
    let m_s = slice::from_raw_parts(m, mlen);
    let sk_s = slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
    sign::crypto_sign_signature(sig_s, &mut *siglen, m_s, mlen, sk_s)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_verify(sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8) -> i32 {
    let sig_s = slice::from_raw_parts(sig, siglen);
    let m_s = slice::from_raw_parts(m, mlen);
    let pk_s = slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
    sign::crypto_sign_verify(sig_s, siglen, m_s, mlen, pk_s)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign(sm: *mut u8, smlen: *mut u64, m: *const u8, mlen: u64, sk: *const u8) -> i32 {
    let sm_s = slice::from_raw_parts_mut(sm, CRYPTO_BYTES + mlen as usize);
    let m_s = slice::from_raw_parts(m, mlen as usize);
    let sk_s = slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
    sign::crypto_sign(sm_s, &mut *smlen, m_s, mlen, sk_s)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_open(m: *mut u8, mlen: *mut u64, sm: *const u8, smlen: u64, pk: *const u8) -> i32 {
    let m_s = slice::from_raw_parts_mut(m, smlen as usize);
    let sm_s = slice::from_raw_parts(sm, smlen as usize);
    let pk_s = slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
    sign::crypto_sign_open(m_s, &mut *mlen, sm_s, smlen, pk_s)
}

// ====== RNG functions ======

#[no_mangle]
pub unsafe extern "C" fn randombytes_init(entropy_input: *const u8, _security_strength: *const u8) {
    let ei = slice::from_raw_parts(entropy_input, 48);
    rng::randombytes_init(ei, None);
}

#[no_mangle]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) {
    let x_s = slice::from_raw_parts_mut(x, xlen as usize);
    rng::randombytes(x_s, xlen);
}

#[no_mangle]
pub unsafe extern "C" fn seedexpander_init(ctx: *mut rng::AesXofStruct, seed: *const u8, diversifier: *const u8, maxlen: u64) -> i32 {
    let seed_s = slice::from_raw_parts(seed, 32);
    let div_s = slice::from_raw_parts(diversifier, 8);
    rng::seedexpander_init(&mut *ctx, seed_s, div_s, maxlen)
}

#[no_mangle]
pub unsafe extern "C" fn seedexpander(ctx: *mut rng::AesXofStruct, x: *mut u8, xlen: u64) -> i32 {
    let x_s = slice::from_raw_parts_mut(x, xlen as usize);
    rng::seedexpander(&mut *ctx, x_s, xlen)
}

// ====== Address functions (SPX_NAMESPACE) ======

#[no_mangle]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    address::set_layer_addr(addr_s, layer);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    address::set_tree_addr(addr_s, tree);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    address::set_type(addr_s, type_val);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    let out_s: &mut [u32; 8] = &mut *(out as *mut [u32; 8]);
    let inp_s: &[u32; 8] = &*(inp as *const [u32; 8]);
    address::copy_subtree_addr(out_s, inp_s);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    address::set_keypair_addr(addr_s, keypair);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    let out_s: &mut [u32; 8] = &mut *(out as *mut [u32; 8]);
    let inp_s: &[u32; 8] = &*(inp as *const [u32; 8]);
    address::copy_keypair_addr(out_s, inp_s);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    address::set_chain_addr(addr_s, chain);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    address::set_hash_addr(addr_s, hash);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    address::set_tree_height(addr_s, tree_height);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    address::set_tree_index(addr_s, tree_index);
}

// ====== Utils (SPX_NAMESPACE) ======

#[no_mangle]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, inp: u64) {
    let out_s = slice::from_raw_parts_mut(out, outlen as usize);
    utils::ull_to_bytes(out_s, outlen as usize, inp);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, inp: u32) {
    let out_s = slice::from_raw_parts_mut(out, 4);
    utils::u32_to_bytes(out_s, inp);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let inp_s = slice::from_raw_parts(inp, inlen as usize);
    utils::bytes_to_ull(inp_s, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn SPX_compute_root(root: *mut u8, leaf: *const u8, leaf_idx: u32, idx_offset: u32, auth_path: *const u8, tree_height: u32, ctx: *const context::SpxCtx, addr: *mut u32) {
    let root_s = slice::from_raw_parts_mut(root, SPX_N);
    let leaf_s = slice::from_raw_parts(leaf, SPX_N);
    let auth_s = slice::from_raw_parts(auth_path, tree_height as usize * SPX_N);
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    utils::compute_root(root_s, leaf_s, leaf_idx, idx_offset, auth_s, tree_height, &*ctx, addr_s);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_treehash(root: *mut u8, auth_path: *mut u8, ctx: *const context::SpxCtx, leaf_idx: u32, idx_offset: u32, tree_height: u32, gen_leaf: unsafe extern "C" fn(*mut u8, *const context::SpxCtx, u32, *mut u8), tree_addr: *mut u32) {
    // This is a complex callback-based function. For symbol export purposes we provide the symbol.
    // The actual implementation would require careful callback wrapping.
    let _ = (root, auth_path, ctx, leaf_idx, idx_offset, tree_height, gen_leaf, tree_addr);
    unimplemented!("SPX_treehash C-compatible wrapper not implemented");
}

// ====== WOTS functions ======

#[no_mangle]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    let lengths_s = slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN);
    let msg_s = slice::from_raw_parts(msg, SPX_N);
    wots::chain_lengths(lengths_s, msg_s);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(pk: *mut u8, sig: *const u8, msg: *const u8, ctx: *const context::SpxCtx, addr: *mut u32) {
    let pk_s = slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES);
    let sig_s = slice::from_raw_parts(sig, SPX_WOTS_BYTES);
    let msg_s = slice::from_raw_parts(msg, SPX_N);
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    wots::wots_pk_from_sig(pk_s, sig_s, msg_s, &*ctx, addr_s);
}

// ====== FORS functions ======

#[no_mangle]
pub unsafe extern "C" fn SPX_fors_sign(sig: *mut u8, pk: *mut u8, m: *const u8, ctx: *const context::SpxCtx, fors_addr: *const u32) {
    let sig_s = slice::from_raw_parts_mut(sig, SPX_FORS_BYTES);
    let pk_s = slice::from_raw_parts_mut(pk, SPX_N);
    let m_s = slice::from_raw_parts(m, SPX_FORS_MSG_BYTES);
    let addr_s: &[u32; 8] = &*(fors_addr as *const [u32; 8]);
    fors::fors_sign(sig_s, pk_s, m_s, &*ctx, addr_s);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(pk: *mut u8, sig: *const u8, m: *const u8, ctx: *const context::SpxCtx, fors_addr: *const u32) {
    let pk_s = slice::from_raw_parts_mut(pk, SPX_N);
    let sig_s = slice::from_raw_parts(sig, SPX_FORS_BYTES);
    let m_s = slice::from_raw_parts(m, SPX_FORS_MSG_BYTES);
    let addr_s: &[u32; 8] = &*(fors_addr as *const [u32; 8]);
    fors::fors_pk_from_sig(pk_s, sig_s, m_s, &*ctx, addr_s);
}

// ====== Merkle functions ======

#[no_mangle]
pub unsafe extern "C" fn SPX_merkle_sign(sig: *mut u8, root: *mut u8, ctx: *const context::SpxCtx, wots_addr: *mut u32, tree_addr: *mut u32, idx_leaf: u32) {
    let sig_s = slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N);
    let root_s = slice::from_raw_parts_mut(root, SPX_N);
    let wots_s: &mut [u32; 8] = &mut *(wots_addr as *mut [u32; 8]);
    let tree_s: &mut [u32; 8] = &mut *(tree_addr as *mut [u32; 8]);
    merkle::merkle_sign(sig_s, root_s, &*ctx, wots_s, tree_s, idx_leaf);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const context::SpxCtx) {
    let root_s = slice::from_raw_parts_mut(root, SPX_N);
    merkle::merkle_gen_root(root_s, &*ctx);
}

// ====== Hash functions (SPX_NAMESPACE) ======

#[no_mangle]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut context::SpxCtx) {
    hash::initialize_hash_function(&mut *ctx);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const context::SpxCtx, addr: *const u32) {
    let out_s = slice::from_raw_parts_mut(out, SPX_N);
    let addr_s: &[u32; 8] = &*(addr as *const [u32; 8]);
    hash::prf_addr(out_s, &*ctx, addr_s);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_gen_message_random(r: *mut u8, sk_prf: *const u8, optrand: *const u8, m: *const u8, mlen: u64, ctx: *const context::SpxCtx) {
    let r_s = slice::from_raw_parts_mut(r, SPX_N);
    let sk_s = slice::from_raw_parts(sk_prf, SPX_N);
    let opt_s = slice::from_raw_parts(optrand, SPX_N);
    let m_s = slice::from_raw_parts(m, mlen as usize);
    hash::gen_message_random(r_s, sk_s, opt_s, m_s, mlen, &*ctx);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_hash_message(digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32, r: *const u8, pk: *const u8, m: *const u8, mlen: u64, ctx: *const context::SpxCtx) {
    let digest_s = slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES);
    let r_s = slice::from_raw_parts(r, SPX_N);
    let pk_s = slice::from_raw_parts(pk, SPX_PK_BYTES);
    let m_s = slice::from_raw_parts(m, mlen as usize);
    hash::hash_message(digest_s, &mut *tree, &mut *leaf_idx, r_s, pk_s, m_s, mlen, &*ctx);
}

// ====== Thash ======

#[no_mangle]
pub unsafe extern "C" fn SPX_thash(out: *mut u8, inp: *const u8, inblocks: u32, ctx: *const context::SpxCtx, addr: *mut u32) {
    let out_s = slice::from_raw_parts_mut(out, SPX_N);
    let inp_s = slice::from_raw_parts(inp, inblocks as usize * SPX_N);
    let addr_s: &mut [u32; 8] = &mut *(addr as *mut [u32; 8]);
    thash::thash(out_s, inp_s, inblocks, &*ctx, addr_s);
}

// ====== Utilsx1 ======

#[no_mangle]
pub unsafe extern "C" fn SPX_wots_treehashx1(root: *mut u8, auth_path: *mut u8, ctx: *const context::SpxCtx, leaf_idx: u32, idx_offset: u32, tree_height: u32, tree_addr: *mut u32, info: *mut wotsx1::LeafInfoX1) {
    let root_s = slice::from_raw_parts_mut(root, SPX_N);
    let auth_s = slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
    let tree_s: &mut [u32; 8] = &mut *(tree_addr as *mut [u32; 8]);
    utilsx1::wots_treehashx1(root_s, auth_s, &*ctx, leaf_idx, idx_offset, tree_height, tree_s, &mut *info);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_fors_treehashx1(root: *mut u8, auth_path: *mut u8, ctx: *const context::SpxCtx, leaf_idx: u32, idx_offset: u32, tree_height: u32, gen_leaf_info: *mut u8, tree_addr: *mut u32) {
    // fors_treehashx1 uses opaque info type - provide symbol
    let _ = (root, auth_path, ctx, leaf_idx, idx_offset, tree_height, gen_leaf_info, tree_addr);
    unimplemented!("SPX_fors_treehashx1 C-compatible wrapper");
}

// ====== WOTS x1 ======

#[no_mangle]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(dest: *mut u8, ctx: *const context::SpxCtx, leaf_idx: u32, info: *mut wotsx1::LeafInfoX1) {
    let dest_s = slice::from_raw_parts_mut(dest, SPX_N);
    wotsx1::wots_gen_leafx1(dest_s, &*ctx, leaf_idx, &mut *info);
}

// ====== FORS gen leaf ======

#[no_mangle]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(leaf: *mut u8, ctx: *const context::SpxCtx, addr_idx: u32, info: *mut fors::ForsGenLeafInfo) {
    let leaf_s = slice::from_raw_parts_mut(leaf, SPX_N);
    fors::fors_gen_leafx1(leaf_s, &*ctx, addr_idx, &mut *info);
}

// ====== AES functions from rng ======

#[no_mangle]
pub unsafe extern "C" fn AES256_ECB(key: *const u8, ctr: *const u8, buffer: *mut u8) {
    let key_s: &[u8; 32] = &*(key as *const [u8; 32]);
    let ctr_s: &[u8; 16] = &*(ctr as *const [u8; 16]);
    let buf_s: &mut [u8; 16] = &mut *(buffer as *mut [u8; 16]);
    rng::aes256_ecb(key_s, ctr_s, buf_s);
}

#[no_mangle]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(provided_data: *const u8, key: *mut u8, v: *mut u8) {
    let key_s: &mut [u8; 32] = &mut *(key as *mut [u8; 32]);
    let v_s: &mut [u8; 16] = &mut *(v as *mut [u8; 16]);
    if provided_data.is_null() {
        rng::aes256_ctr_drbg_update(None, key_s, v_s);
    } else {
        let data_s = slice::from_raw_parts(provided_data, 48);
        rng::aes256_ctr_drbg_update(Some(data_s), key_s, v_s);
    }
}

// ====== Blake functions (when blake feature enabled) ======

// ====== Blake raw functions (always exported when blake feature) ======

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake256_init(s: *mut blake_impl::BlakeState256) {
    blake_impl::blake256_init(&mut *s);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake256_compress(s: *mut blake_impl::BlakeState256, block: *const u8) {
    let block_s = slice::from_raw_parts(block, 64);
    blake_impl::blake256_compress(&mut *s, block_s);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake256_update(s: *mut blake_impl::BlakeState256, data: *const u8, datalen: u64) {
    let data_s = slice::from_raw_parts(data, (datalen >> 3) as usize + 1);
    blake_impl::blake256_update(&mut *s, data_s, datalen);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake256_final(s: *mut blake_impl::BlakeState256, digest: *mut u8) {
    let digest_s = slice::from_raw_parts_mut(digest, 32);
    blake_impl::blake256_final(&mut *s, digest_s);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake256(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let out_s = slice::from_raw_parts_mut(out, 32);
    let inp_s = slice::from_raw_parts(inp, inlen as usize);
    blake_impl::blake256(out_s, inp_s, inlen)
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake512_init(s: *mut blake_impl::BlakeState512) {
    blake_impl::blake512_init(&mut *s);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake512_compress(s: *mut blake_impl::BlakeState512, block: *const u8) {
    let block_s = slice::from_raw_parts(block, 128);
    blake_impl::blake512_compress(&mut *s, block_s);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake512_update(s: *mut blake_impl::BlakeState512, data: *const u8, datalen: u64) {
    let data_s = slice::from_raw_parts(data, (datalen >> 3) as usize + 1);
    blake_impl::blake512_update(&mut *s, data_s, datalen);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake512_final(s: *mut blake_impl::BlakeState512, digest: *mut u8) {
    let digest_s = slice::from_raw_parts_mut(digest, 64);
    blake_impl::blake512_final(&mut *s, digest_s);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn blake512(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let out_s = slice::from_raw_parts_mut(out, 64);
    let inp_s = slice::from_raw_parts(inp, inlen as usize);
    blake_impl::blake512(out_s, inp_s, inlen)
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn SPX_blake256_mgf1(out: *mut u8, outlen: usize, inp: *const u8, inlen: usize) {
    let out_s = slice::from_raw_parts_mut(out, outlen);
    let inp_s = slice::from_raw_parts(inp, inlen);
    blake_impl::blake256_mgf1(out_s, outlen, inp_s, inlen);
}

#[cfg(feature = "blake")]
#[no_mangle]
pub unsafe extern "C" fn SPX_blake512_mgf1(out: *mut u8, outlen: usize, inp: *const u8, inlen: usize) {
    let out_s = slice::from_raw_parts_mut(out, outlen);
    let inp_s = slice::from_raw_parts(inp, inlen);
    blake_impl::blake512_mgf1(out_s, outlen, inp_s, inlen);
}

// ====== SHA2 raw functions (when sha2 feature enabled) ======

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn sha256_inc_init(state: *mut u8) {
    let state_s: &mut [u8; 40] = &mut *(state as *mut [u8; 40]);
    sha2_impl::sha256_inc_init(state_s);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn sha256_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
    let state_s: &mut [u8; 40] = &mut *(state as *mut [u8; 40]);
    let inp_s = slice::from_raw_parts(inp, 64 * inblocks);
    sha2_impl::sha256_inc_blocks(state_s, inp_s, inblocks);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn sha256_inc_finalize(out: *mut u8, state: *mut u8, inp: *const u8, inlen: usize) {
    let state_s: &mut [u8; 40] = &mut *(state as *mut [u8; 40]);
    let out_s = slice::from_raw_parts_mut(out, 32);
    let inp_s = slice::from_raw_parts(inp, inlen);
    sha2_impl::sha256_inc_finalize(out_s, state_s, inp_s, inlen);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn sha256(out: *mut u8, inp: *const u8, inlen: usize) {
    let out_s = slice::from_raw_parts_mut(out, 32);
    let inp_s = slice::from_raw_parts(inp, inlen);
    sha2_impl::sha256(out_s, inp_s, inlen);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn sha512_inc_init(state: *mut u8) {
    let state_s: &mut [u8; 72] = &mut *(state as *mut [u8; 72]);
    sha2_impl::sha512_inc_init(state_s);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn sha512_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
    let state_s: &mut [u8; 72] = &mut *(state as *mut [u8; 72]);
    let inp_s = slice::from_raw_parts(inp, 128 * inblocks);
    sha2_impl::sha512_inc_blocks(state_s, inp_s, inblocks);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn sha512_inc_finalize(out: *mut u8, state: *mut u8, inp: *const u8, inlen: usize) {
    let state_s: &mut [u8; 72] = &mut *(state as *mut [u8; 72]);
    let out_s = slice::from_raw_parts_mut(out, 64);
    let inp_s = slice::from_raw_parts(inp, inlen);
    sha2_impl::sha512_inc_finalize(out_s, state_s, inp_s, inlen);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn sha512(out: *mut u8, inp: *const u8, inlen: usize) {
    let out_s = slice::from_raw_parts_mut(out, 64);
    let inp_s = slice::from_raw_parts(inp, inlen);
    sha2_impl::sha512(out_s, inp_s, inlen);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn SPX_mgf1_256(out: *mut u8, outlen: usize, inp: *const u8, inlen: usize) {
    let out_s = slice::from_raw_parts_mut(out, outlen);
    let inp_s = slice::from_raw_parts(inp, inlen);
    sha2_impl::mgf1_256(out_s, outlen, inp_s, inlen);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn SPX_mgf1_512(out: *mut u8, outlen: usize, inp: *const u8, inlen: usize) {
    let out_s = slice::from_raw_parts_mut(out, outlen);
    let inp_s = slice::from_raw_parts(inp, inlen);
    sha2_impl::mgf1_512(out_s, outlen, inp_s, inlen);
}

#[cfg(feature = "sha2")]
#[no_mangle]
pub unsafe extern "C" fn SPX_seed_state(ctx: *mut context::SpxCtx) {
    sha2_impl::seed_state(&mut *ctx);
}

// ====== SHAKE raw functions (when shake feature enabled) ======

#[cfg(feature = "shake")]
#[no_mangle]
pub unsafe extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    let s_s: &mut [u64; 25] = &mut *(s as *mut [u64; 25]);
    let inp_s = slice::from_raw_parts(input, inlen);
    fips202::shake256_absorb(s_s, inp_s, inlen);
}

#[cfg(feature = "shake")]
#[no_mangle]
pub unsafe extern "C" fn shake256_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    let s_s: &mut [u64; 25] = &mut *(s as *mut [u64; 25]);
    let out_s = slice::from_raw_parts_mut(output, nblocks * 136);
    fips202::shake256_squeezeblocks(out_s, nblocks, s_s);
}

#[cfg(feature = "shake")]
#[no_mangle]
pub unsafe extern "C" fn shake256_inc_init(s_inc: *mut u64) {
    let s_s: &mut [u64; 26] = &mut *(s_inc as *mut [u64; 26]);
    fips202::shake256_inc_init(s_s);
}

#[cfg(feature = "shake")]
#[no_mangle]
pub unsafe extern "C" fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    let s_s: &mut [u64; 26] = &mut *(s_inc as *mut [u64; 26]);
    let inp_s = slice::from_raw_parts(input, inlen);
    fips202::shake256_inc_absorb(s_s, inp_s, inlen);
}

#[cfg(feature = "shake")]
#[no_mangle]
pub unsafe extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
    let s_s: &mut [u64; 26] = &mut *(s_inc as *mut [u64; 26]);
    fips202::shake256_inc_finalize(s_s);
}

#[cfg(feature = "shake")]
#[no_mangle]
pub unsafe extern "C" fn shake256_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    let s_s: &mut [u64; 26] = &mut *(s_inc as *mut [u64; 26]);
    let out_s = slice::from_raw_parts_mut(output, outlen);
    fips202::shake256_inc_squeeze(out_s, outlen, s_s);
}

#[cfg(feature = "shake")]
#[no_mangle]
pub unsafe extern "C" fn shake256(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    let out_s = slice::from_raw_parts_mut(output, outlen);
    let inp_s = slice::from_raw_parts(input, inlen);
    fips202::shake256(out_s, outlen, inp_s, inlen);
}

// ====== Haraka functions (when haraka feature enabled) ======

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_tweak_constants(ctx: *mut context::SpxCtx) {
    haraka_impl::tweak_constants(&mut *ctx);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn tweak_constants(ctx: *mut context::SpxCtx) {
    haraka_impl::tweak_constants(&mut *ctx);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka512_perm(out: *mut u8, inp: *const u8, ctx: *const context::SpxCtx) {
    let out_s = slice::from_raw_parts_mut(out, 64);
    let inp_s = slice::from_raw_parts(inp, 64);
    haraka_impl::haraka512_perm(out_s, inp_s, &*ctx);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka512(out: *mut u8, inp: *const u8, ctx: *const context::SpxCtx) {
    let out_s = slice::from_raw_parts_mut(out, 32);
    let inp_s = slice::from_raw_parts(inp, 64);
    haraka_impl::haraka512(out_s, inp_s, &*ctx);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka256(out: *mut u8, inp: *const u8, ctx: *const context::SpxCtx) {
    let out_s = slice::from_raw_parts_mut(out, 32);
    let inp_s = slice::from_raw_parts(inp, 32);
    haraka_impl::haraka256(out_s, inp_s, &*ctx);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64, ctx: *const context::SpxCtx) {
    let out_s = slice::from_raw_parts_mut(out, outlen as usize);
    let inp_s = slice::from_raw_parts(inp, inlen as usize);
    haraka_impl::haraka_S(out_s, outlen, inp_s, inlen, &*ctx);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S_inc_init(s_inc: *mut u8) {
    let s_s: &mut [u8; 65] = &mut *(s_inc as *mut [u8; 65]);
    haraka_impl::haraka_S_inc_init(s_s);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S_inc_absorb(s_inc: *mut u8, m: *const u8, mlen: usize, ctx: *const context::SpxCtx) {
    let s_s: &mut [u8; 65] = &mut *(s_inc as *mut [u8; 65]);
    let m_s = slice::from_raw_parts(m, mlen);
    haraka_impl::haraka_S_inc_absorb(s_s, m_s, mlen, &*ctx);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S_inc_finalize(s_inc: *mut u8) {
    let s_s: &mut [u8; 65] = &mut *(s_inc as *mut [u8; 65]);
    haraka_impl::haraka_S_inc_finalize(s_s);
}

#[cfg(feature = "haraka")]
#[no_mangle]
pub unsafe extern "C" fn SPX_haraka_S_inc_squeeze(out: *mut u8, outlen: usize, s_inc: *mut u8, ctx: *const context::SpxCtx) {
    let s_s: &mut [u8; 65] = &mut *(s_inc as *mut [u8; 65]);
    let out_s = slice::from_raw_parts_mut(out, outlen);
    haraka_impl::haraka_S_inc_squeeze(out_s, outlen, s_s, &*ctx);
}
