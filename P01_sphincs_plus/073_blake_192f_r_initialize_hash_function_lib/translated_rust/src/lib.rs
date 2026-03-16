#![allow(clippy::all, unused_imports, non_upper_case_globals)]

pub mod params;
pub mod context;
pub mod blake256;
pub mod blake512;
pub mod utils;
pub mod address;
pub mod hash;
pub mod thash;
pub mod wots;
pub mod fors;
pub mod utilsx1;
pub mod merkle;
pub mod sign;
pub mod randombytes;

use std::slice;
use params::*;
use context::SpxCtx;

// --- Exported C API ---

#[unsafe(no_mangle)]
pub extern "C" fn initialize_hash_function(ctx: *mut SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    hash::initialize_hash_function(ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &*(addr as *const [u32; 8]) };
    let out = unsafe { slice::from_raw_parts_mut(out, SPX_N) };
    hash::prf_addr(out, ctx, addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn gen_message_random(
    r: *mut u8, sk_prf: *const u8, optrand: *const u8,
    m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    let ctx = unsafe { &*ctx };
    let sk_prf = unsafe { slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand = unsafe { slice::from_raw_parts(optrand, SPX_N) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let r = unsafe { slice::from_raw_parts_mut(r, SPX_BLAKEX_OUTPUT_BYTES) };
    hash::gen_message_random(r, sk_prf, optrand, m, mlen, ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_message(
    digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
    r: *const u8, pk: *const u8, m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    let ctx = unsafe { &*ctx };
    let r = unsafe { slice::from_raw_parts(r, SPX_N) };
    let pk = unsafe { slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let digest = unsafe { slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    hash::hash_message(digest, unsafe { &mut *tree }, unsafe { &mut *leaf_idx }, r, pk, m, mlen, ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn thash(
    out: *mut u8, inp: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let inblocks = inblocks as usize;
    let inp = unsafe { slice::from_raw_parts(inp, inblocks * SPX_N) };
    let out = unsafe { slice::from_raw_parts_mut(out, SPX_N) };
    thash::thash(out, inp, inblocks, ctx, addr);
}

// Address functions
#[unsafe(no_mangle)]
pub extern "C" fn set_layer_addr(addr: *mut u32, layer: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_layer_addr(addr, layer);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_tree_addr(addr: *mut u32, tree: u64) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_tree_addr(addr, tree);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_type(addr: *mut u32, type_val: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_type(addr, type_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn copy_subtree_addr(out: *mut u32, inp: *const u32) {
    let out = unsafe { &mut *(out as *mut [u32; 8]) };
    let inp = unsafe { &*(inp as *const [u32; 8]) };
    address::copy_subtree_addr(out, inp);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_keypair_addr(addr: *mut u32, keypair: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_keypair_addr(addr, keypair);
}

#[unsafe(no_mangle)]
pub extern "C" fn copy_keypair_addr(out: *mut u32, inp: *const u32) {
    let out = unsafe { &mut *(out as *mut [u32; 8]) };
    let inp = unsafe { &*(inp as *const [u32; 8]) };
    address::copy_keypair_addr(out, inp);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_chain_addr(addr: *mut u32, chain: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_chain_addr(addr, chain);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_hash_addr(addr: *mut u32, hash: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_hash_addr(addr, hash);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_tree_height(addr: *mut u32, tree_height: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_tree_height(addr, tree_height);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_tree_index(addr: *mut u32, tree_index: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_tree_index(addr, tree_index);
}

// Utils
#[unsafe(no_mangle)]
pub extern "C" fn ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    let out = unsafe { slice::from_raw_parts_mut(out, outlen as usize) };
    utils::ull_to_bytes(out, outlen as usize, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn u32_to_bytes(out: *mut u8, val: u32) {
    let out = unsafe { slice::from_raw_parts_mut(out, 4) };
    utils::u32_to_bytes(out, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let inp = unsafe { slice::from_raw_parts(inp, inlen as usize) };
    utils::bytes_to_ull(inp, inlen as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_root(
    root: *mut u8, leaf: *const u8, leaf_idx: u32, idx_offset: u32,
    auth_path: *const u8, tree_height: u32, ctx: *const SpxCtx, addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let leaf = unsafe { slice::from_raw_parts(leaf, SPX_N) };
    let auth_path = unsafe { slice::from_raw_parts(auth_path, tree_height as usize * SPX_N) };
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    utils::compute_root(root, leaf, leaf_idx, idx_offset, auth_path, tree_height, ctx, addr);
}

// WOTS
#[unsafe(no_mangle)]
pub extern "C" fn wots_pk_from_sig(
    pk: *mut u8, sig: *const u8, msg: *const u8, ctx: *const SpxCtx, addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let sig = unsafe { slice::from_raw_parts(sig, SPX_WOTS_BYTES) };
    let msg = unsafe { slice::from_raw_parts(msg, SPX_N) };
    let pk = unsafe { slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES) };
    wots::wots_pk_from_sig(pk, sig, msg, ctx, addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn chain_lengths(lengths: *mut u32, msg: *const u8) {
    let lengths = unsafe { slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) };
    let msg = unsafe { slice::from_raw_parts(msg, SPX_N) };
    wots::chain_lengths(lengths, msg);
}

// FORS
#[unsafe(no_mangle)]
pub extern "C" fn fors_sign(
    sig: *mut u8, pk: *mut u8, m: *const u8, ctx: *const SpxCtx, fors_addr: *const u32,
) {
    let ctx = unsafe { &*ctx };
    let fors_addr = unsafe { &*(fors_addr as *const [u32; 8]) };
    let m = unsafe { slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let sig = unsafe { slice::from_raw_parts_mut(sig, SPX_FORS_BYTES) };
    let pk = unsafe { slice::from_raw_parts_mut(pk, SPX_N) };
    fors::fors_sign(sig, pk, m, ctx, fors_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn fors_pk_from_sig(
    pk: *mut u8, sig: *const u8, m: *const u8, ctx: *const SpxCtx, fors_addr: *const u32,
) {
    let ctx = unsafe { &*ctx };
    let fors_addr = unsafe { &*(fors_addr as *const [u32; 8]) };
    let m = unsafe { slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let sig = unsafe { slice::from_raw_parts(sig, SPX_FORS_BYTES) };
    let pk = unsafe { slice::from_raw_parts_mut(pk, SPX_N) };
    fors::fors_pk_from_sig(pk, sig, m, ctx, fors_addr);
}

// Merkle
#[unsafe(no_mangle)]
pub extern "C" fn merkle_sign(
    sig: *mut u8, root: *mut u8, ctx: *const SpxCtx,
    wots_addr: *mut u32, tree_addr: *mut u32, idx_leaf: u32,
) {
    let ctx = unsafe { &*ctx };
    let wots_addr = unsafe { &mut *(wots_addr as *mut [u32; 8]) };
    let tree_addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let sig = unsafe { slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N) };
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    merkle::merkle_sign(sig, root, ctx, wots_addr, tree_addr, idx_leaf);
}

#[unsafe(no_mangle)]
pub extern "C" fn merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let ctx = unsafe { &*ctx };
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    merkle::merkle_gen_root(root, ctx);
}

// Sign API
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 { sign::crypto_sign_secretkeybytes() }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 { sign::crypto_sign_publickeybytes() }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 { sign::crypto_sign_bytes() }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 { sign::crypto_sign_seedbytes() }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> i32 {
    let pk = unsafe { slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let pk = unsafe { slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    sign::crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize, m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    let sig = unsafe { slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen) };
    let sk = unsafe { slice::from_raw_parts(sk, SPX_SK_BYTES) };
    sign::crypto_sign_signature(sig, unsafe { &mut *siglen }, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    let sig = unsafe { slice::from_raw_parts(sig, siglen) };
    let m = unsafe { slice::from_raw_parts(m, mlen) };
    let pk = unsafe { slice::from_raw_parts(pk, SPX_PK_BYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64, m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    let sm = unsafe { slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { slice::from_raw_parts(sk, SPX_SK_BYTES) };
    sign::crypto_sign(sm, unsafe { &mut *smlen }, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64, sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    let sm = unsafe { slice::from_raw_parts(sm, smlen as usize) };
    let m = unsafe { slice::from_raw_parts_mut(m, smlen as usize) };
    let pk = unsafe { slice::from_raw_parts(pk, SPX_PK_BYTES) };
    sign::crypto_sign_open(m, unsafe { &mut *mlen }, sm, smlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) {
    let x = unsafe { slice::from_raw_parts_mut(x, xlen as usize) };
    randombytes::randombytes(x, xlen);
}
