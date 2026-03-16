#![allow(non_snake_case, non_upper_case_globals, clippy::missing_safety_doc)]

mod address;
mod blake256;
mod blake512;
mod context;
mod hash_blake;
mod params;
mod thash;
mod utils;

use context::SpxCtx;

// --- hash_blake exports ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    hash_blake::initialize_hash_function(ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(
    out: *mut u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &*(addr as *const [u32; 8]) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, params::SPX_N) };
    hash_blake::prf_addr(out, ctx, addr);
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
    let ctx = unsafe { &*ctx };
    let sk_prf = unsafe { core::slice::from_raw_parts(sk_prf, params::SPX_N) };
    let optrand = unsafe { core::slice::from_raw_parts(optrand, params::SPX_N) };
    let m = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let r = unsafe { core::slice::from_raw_parts_mut(r, blake256::SPX_BLAKE256_OUTPUT_BYTES) };
    hash_blake::gen_message_random(r, sk_prf, optrand, m, mlen, ctx);
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
    let ctx = unsafe { &*ctx };
    let r_s = unsafe { core::slice::from_raw_parts(r, params::SPX_N) };
    let pk_s = unsafe { core::slice::from_raw_parts(pk, params::SPX_PK_BYTES) };
    let m_s = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let digest = unsafe { core::slice::from_raw_parts_mut(digest, params::SPX_FORS_MSG_BYTES) };
    let tree = unsafe { &mut *tree };
    let leaf_idx = unsafe { &mut *leaf_idx };
    hash_blake::hash_message(digest, tree, leaf_idx, r_s, pk_s, m_s, mlen, ctx);
}

// --- thash export ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &*(addr as *const [u32; 8]) };
    let inblocks = inblocks as usize;
    let inp = unsafe { core::slice::from_raw_parts(inp, inblocks * params::SPX_N) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, params::SPX_N) };
    thash::thash(out, inp, inblocks, ctx, addr);
}

// --- address exports ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_layer_addr(addr, layer);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_tree_addr(addr, tree);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_type(addr, type_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    let out = unsafe { &mut *(out as *mut [u32; 8]) };
    let inp = unsafe { &*(inp as *const [u32; 8]) };
    address::copy_subtree_addr(out, inp);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_keypair_addr(addr, keypair);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    let out = unsafe { &mut *(out as *mut [u32; 8]) };
    let inp = unsafe { &*(inp as *const [u32; 8]) };
    address::copy_keypair_addr(out, inp);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_chain_addr(addr, chain);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_hash_addr(addr, hash);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_tree_height(addr, tree_height);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    address::set_tree_index(addr, tree_index);
}

// --- utils exports ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    address::ull_to_bytes(out, outlen as usize, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    let out = unsafe { core::slice::from_raw_parts_mut(out, 4) };
    address::u32_to_bytes(out, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    address::bytes_to_ull(inp, inlen as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let leaf = unsafe { core::slice::from_raw_parts(leaf, params::SPX_N) };
    let auth_path = unsafe {
        core::slice::from_raw_parts(auth_path, tree_height as usize * params::SPX_N)
    };
    let root = unsafe { core::slice::from_raw_parts_mut(root, params::SPX_N) };
    utils::compute_root(root, leaf, leaf_idx, idx_offset, auth_path, tree_height, ctx, addr);
}

// --- blake256 exports ---

#[unsafe(no_mangle)]
pub extern "C" fn blake256(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, blake256::SPX_BLAKE256_OUTPUT_BYTES) };
    blake256::blake256(out, inp, inlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_init(s: *mut blake256::Blakestate256) {
    let s = unsafe { &mut *s };
    blake256::blake256_init(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_compress(s: *mut blake256::Blakestate256, block: *const u8) {
    let s = unsafe { &mut *s };
    let block = unsafe { core::slice::from_raw_parts(block, 64) };
    blake256::blake256_compress(s, block);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_update(
    s: *mut blake256::Blakestate256,
    data: *const u8,
    datalen: u64,
) {
    let s = unsafe { &mut *s };
    let byte_len = ((datalen + 7) / 8) as usize;
    let data = unsafe { core::slice::from_raw_parts(data, byte_len) };
    blake256::blake256_update(s, data, datalen);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_final(s: *mut blake256::Blakestate256, digest: *mut u8) {
    let s = unsafe { &mut *s };
    let digest = unsafe { core::slice::from_raw_parts_mut(digest, blake256::SPX_BLAKE256_OUTPUT_BYTES) };
    blake256::blake256_final(s, digest);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake256_mgf1(
    out: *mut u8,
    outlen: u64,
    inp: *const u8,
    inlen: u64,
) {
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    blake256::blake256_mgf1(out, outlen as usize, inp, inlen as usize);
}

// --- blake512 exports ---

#[unsafe(no_mangle)]
pub extern "C" fn blake512(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, blake512::SPX_BLAKE512_OUTPUT_BYTES) };
    blake512::blake512(out, inp, inlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_init(s: *mut blake512::Blakestate512) {
    let s = unsafe { &mut *s };
    blake512::blake512_init(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_compress(s: *mut blake512::Blakestate512, block: *const u8) {
    let s = unsafe { &mut *s };
    let block = unsafe { core::slice::from_raw_parts(block, 128) };
    blake512::blake512_compress(s, block);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_update(
    s: *mut blake512::Blakestate512,
    data: *const u8,
    datalen: u64,
) {
    let s = unsafe { &mut *s };
    let byte_len = ((datalen + 7) / 8) as usize;
    let data = unsafe { core::slice::from_raw_parts(data, byte_len) };
    blake512::blake512_update(s, data, datalen);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_final(s: *mut blake512::Blakestate512, digest: *mut u8) {
    let s = unsafe { &mut *s };
    let digest = unsafe { core::slice::from_raw_parts_mut(digest, blake512::SPX_BLAKE512_OUTPUT_BYTES) };
    blake512::blake512_final(s, digest);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake512_mgf1(
    out: *mut u8,
    outlen: u64,
    inp: *const u8,
    inlen: u64,
) {
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    blake512::blake512_mgf1(out, outlen as usize, inp, inlen as usize);
}
