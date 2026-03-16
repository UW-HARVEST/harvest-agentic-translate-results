use crate::context::SpxCtx;
use crate::params::*;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 { SPX_SK_BYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 { SPX_PK_BYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 { SPX_BYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 { CRYPTO_SEEDBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> i32 {
    let p = unsafe { core::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let s = unsafe { core::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let sd = unsafe { core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    crate::sign::crypto_sign_seed_keypair_internal(p, s, sd)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let p = unsafe { core::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let s = unsafe { core::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    crate::sign::crypto_sign_keypair_internal(p, s)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    let sg = unsafe { core::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let sl = unsafe { &mut *siglen };
    let msg = unsafe { core::slice::from_raw_parts(m, mlen) };
    let s = unsafe { core::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    crate::sign::crypto_sign_signature_internal(sg, sl, msg, mlen, s)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    let sg = unsafe { core::slice::from_raw_parts(sig, siglen) };
    let msg = unsafe { core::slice::from_raw_parts(m, mlen) };
    let p = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    crate::sign::crypto_sign_verify_internal(sg, siglen, msg, mlen, p)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    let s = unsafe { core::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let sl = unsafe { &mut *smlen };
    let msg = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let sk_s = unsafe { core::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    crate::sign::crypto_sign_internal(s, sl, msg, mlen, sk_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    let m_s = unsafe { core::slice::from_raw_parts_mut(m, smlen as usize) };
    let ml = unsafe { &mut *mlen };
    let sm_s = unsafe { core::slice::from_raw_parts(sm, smlen as usize) };
    let p = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    crate::sign::crypto_sign_open_internal(m_s, ml, sm_s, smlen, p)
}

// Address functions
#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::set_layer_addr_internal(a, layer);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::set_tree_addr_internal(a, tree);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::set_type_internal(a, type_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, in_addr: *const u32) {
    let o = unsafe { &mut *(out as *mut [u32; 8]) };
    let i = unsafe { &*(in_addr as *const [u32; 8]) };
    crate::utils::copy_subtree_addr_internal(o, i);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::set_keypair_addr_internal(a, keypair);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::set_chain_addr_internal(a, chain);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::set_hash_addr_internal(a, hash);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, in_addr: *const u32) {
    let o = unsafe { &mut *(out as *mut [u32; 8]) };
    let i = unsafe { &*(in_addr as *const [u32; 8]) };
    crate::utils::copy_keypair_addr_internal(o, i);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::set_tree_height_internal(a, tree_height);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::set_tree_index_internal(a, tree_index);
}

// Utils
#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    crate::utils::ull_to_bytes_internal(o, outlen as usize, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, 4) };
    crate::utils::u32_to_bytes_internal(o, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(in_data: *const u8, inlen: u32) -> u64 {
    let d = unsafe { core::slice::from_raw_parts(in_data, inlen as usize) };
    crate::utils::bytes_to_ull_internal(d, inlen as usize)
}

// WOTS
#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8, sig: *const u8, msg: *const u8,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let p = unsafe { core::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES) };
    let s = unsafe { core::slice::from_raw_parts(sig, SPX_WOTS_BYTES) };
    let m = unsafe { core::slice::from_raw_parts(msg, SPX_N) };
    let c = unsafe { &*ctx };
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::wots::wots_pk_from_sig_internal(p, s, m, c, a);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    let l = unsafe { core::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) };
    let m = unsafe { core::slice::from_raw_parts(msg, SPX_N) };
    crate::wots::chain_lengths_internal(l, m);
}

// FORS
#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_sign(
    sig: *mut u8, pk: *mut u8, m: *const u8,
    ctx: *const SpxCtx, fors_addr: *const u32,
) {
    let s = unsafe { core::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES) };
    let p = unsafe { core::slice::from_raw_parts_mut(pk, SPX_N) };
    let msg = unsafe { core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let c = unsafe { &*ctx };
    let a = unsafe { &*(fors_addr as *const [u32; 8]) };
    crate::fors::fors_sign_internal(s, p, msg, c, a);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8, sig: *const u8, m: *const u8,
    ctx: *const SpxCtx, fors_addr: *const u32,
) {
    let p = unsafe { core::slice::from_raw_parts_mut(pk, SPX_N) };
    let s = unsafe { core::slice::from_raw_parts(sig, SPX_FORS_BYTES) };
    let msg = unsafe { core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let c = unsafe { &*ctx };
    let a = unsafe { &*(fors_addr as *const [u32; 8]) };
    crate::fors::fors_pk_from_sig_internal(p, s, msg, c, a);
}

// Merkle
#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_sign(
    sig: *mut u8, root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32, tree_addr: *mut u32,
    idx_leaf: u32,
) {
    let s = unsafe { core::slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N) };
    let r = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let c = unsafe { &*ctx };
    let wa = unsafe { &mut *(wots_addr as *mut [u32; 8]) };
    let ta = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    crate::merkle::merkle_sign_internal(s, r, c, wa, ta, idx_leaf);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let r = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let c = unsafe { &*ctx };
    crate::merkle::merkle_gen_root_internal(r, c);
}
