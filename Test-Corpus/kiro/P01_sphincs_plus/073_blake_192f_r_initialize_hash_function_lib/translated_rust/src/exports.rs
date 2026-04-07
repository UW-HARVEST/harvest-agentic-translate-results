#![allow(non_snake_case, unused)]
//! FFI export wrappers matching C's SPX_NAMESPACE(name) -> SPX_name symbols.

use crate::params::*;
use crate::context::SpxCtx;

// ============ address.rs exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::address::set_layer_addr(addr, layer);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::address::set_tree_addr(addr, tree);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::address::set_type(addr, type_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    let out = unsafe { &mut *(out as *mut [u32; 8]) };
    let inp = unsafe { &*(inp as *const [u32; 8]) };
    crate::address::copy_subtree_addr(out, inp);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::address::set_keypair_addr(addr, keypair);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    let out = unsafe { &mut *(out as *mut [u32; 8]) };
    let inp = unsafe { &*(inp as *const [u32; 8]) };
    crate::address::copy_keypair_addr(out, inp);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::address::set_chain_addr(addr, chain);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::address::set_hash_addr(addr, hash);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::address::set_tree_height(addr, tree_height);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::address::set_tree_index(addr, tree_index);
}

// ============ utils.rs exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    let out = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    crate::utils::ull_to_bytes(out, outlen as usize, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    let out = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    crate::utils::u32_to_bytes(out, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(input: *const u8, inlen: u32) -> u64 {
    let input = unsafe { std::slice::from_raw_parts(input, inlen as usize) };
    crate::utils::bytes_to_ull(input, inlen as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8, leaf: *const u8,
    leaf_idx: u32, idx_offset: u32,
    auth_path: *const u8, tree_height: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    unsafe {
        let root = std::slice::from_raw_parts_mut(root, SPX_N);
        let leaf = std::slice::from_raw_parts(leaf, SPX_N);
        let auth_path = std::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N);
        let addr = &mut *(addr as *mut [u32; 8]);
        crate::utils::compute_root(root, leaf, leaf_idx, idx_offset, auth_path, tree_height, &*ctx, addr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8, auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    gen_leaf: fn(&mut [u8], &SpxCtx, u32, &[u32; 8]),
    tree_addr: *mut u32,
) {
    unsafe {
        let root = std::slice::from_raw_parts_mut(root, SPX_N);
        let auth_path = std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
        let tree_addr = &mut *(tree_addr as *mut [u32; 8]);
        crate::utils::treehash(root, auth_path, &*ctx, leaf_idx, idx_offset, tree_height, gen_leaf, tree_addr);
    }
}

// ============ wots.rs exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    unsafe {
        let lengths = std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN);
        let msg = std::slice::from_raw_parts(msg, SPX_N);
        crate::wots::chain_lengths(lengths, msg);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8, sig: *const u8, msg: *const u8,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    unsafe {
        let pk = std::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES);
        let sig = std::slice::from_raw_parts(sig, SPX_WOTS_BYTES);
        let msg = std::slice::from_raw_parts(msg, SPX_N);
        let addr = &mut *(addr as *mut [u32; 8]);
        crate::wots::wots_pk_from_sig(pk, sig, msg, &*ctx, addr);
    }
}

// ============ fors.rs exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_sign(
    sig: *mut u8, pk: *mut u8, m: *const u8,
    ctx: *const SpxCtx, fors_addr: *const u32,
) {
    unsafe {
        let sig = std::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES);
        let pk = std::slice::from_raw_parts_mut(pk, SPX_N);
        let m = std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES);
        let fors_addr = &*(fors_addr as *const [u32; 8]);
        crate::fors::fors_sign(sig, pk, m, &*ctx, fors_addr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8, sig: *const u8, m: *const u8,
    ctx: *const SpxCtx, fors_addr: *const u32,
) {
    unsafe {
        let pk = std::slice::from_raw_parts_mut(pk, SPX_N);
        let sig = std::slice::from_raw_parts(sig, SPX_FORS_BYTES);
        let m = std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES);
        let fors_addr = &*(fors_addr as *const [u32; 8]);
        crate::fors::fors_pk_from_sig(pk, sig, m, &*ctx, fors_addr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8, ctx: *const SpxCtx,
    addr_idx: u32, info: *mut crate::fors::ForsGenLeafInfo,
) {
    unsafe {
        let leaf = std::slice::from_raw_parts_mut(leaf, SPX_N);
        crate::fors::fors_gen_leafx1(leaf, &*ctx, addr_idx, &mut *info);
    }
}

// ============ merkle.rs exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_sign(
    sig: *mut u8, root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32, tree_addr: *mut u32,
    idx_leaf: u32,
) {
    unsafe {
        let sig = std::slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N);
        let root = std::slice::from_raw_parts_mut(root, SPX_N);
        let wots_addr = &mut *(wots_addr as *mut [u32; 8]);
        let tree_addr = &mut *(tree_addr as *mut [u32; 8]);
        crate::merkle::merkle_sign(sig, root, &*ctx, wots_addr, tree_addr, idx_leaf);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    unsafe {
        let root = std::slice::from_raw_parts_mut(root, SPX_N);
        crate::merkle::merkle_gen_root(root, &*ctx);
    }
}

// ============ wotsx1.rs exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8, ctx: *const SpxCtx,
    leaf_idx: u32, info: *mut crate::wotsx1::LeafInfoX1,
) {
    unsafe {
        let dest = std::slice::from_raw_parts_mut(dest, SPX_N);
        crate::wotsx1::wots_gen_leafx1(dest, &*ctx, leaf_idx, &mut *info);
    }
}

// ============ utilsx1.rs exports ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_treehashx1(
    root: *mut u8, auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: *mut u32, info: *mut crate::wotsx1::LeafInfoX1,
) {
    unsafe {
        let root = std::slice::from_raw_parts_mut(root, SPX_N);
        let auth_path = std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
        let tree_addr = &mut *(tree_addr as *mut [u32; 8]);
        crate::utilsx1::wots_treehashx1(root, auth_path, &*ctx, leaf_idx, idx_offset, tree_height, tree_addr, &mut *info);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_treehashx1(
    root: *mut u8, auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: *mut u32, info: *mut crate::fors::ForsGenLeafInfo,
) {
    unsafe {
        let root = std::slice::from_raw_parts_mut(root, SPX_N);
        let auth_path = std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
        let tree_addr = &mut *(tree_addr as *mut [u32; 8]);
        crate::utilsx1::fors_treehashx1(root, auth_path, &*ctx, leaf_idx, idx_offset, tree_height, tree_addr, &mut *info);
    }
}

// ============ rng.rs AES256_ECB export ============

#[unsafe(no_mangle)]
pub extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    unsafe {
        let key_arr = &*(key as *const [u8; 32]);
        let ctr_arr = &*(ctr as *const [u8; 16]);
        let buf_arr = &mut *(buffer as *mut [u8; 16]);
        crate::rng::aes256_ecb_export(key_arr, ctr_arr, buf_arr);
    }
}

// ============ hash.rs exports (backend-dispatched) ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe { crate::hash::initialize_hash_function(&mut *ctx); }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        let out = std::slice::from_raw_parts_mut(out, SPX_N);
        let addr = &*(addr as *const [u32; 8]);
        crate::hash::prf_addr(out, &*ctx, addr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8, sk_prf: *const u8, optrand: *const u8,
    m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    unsafe {
        let r = std::slice::from_raw_parts_mut(r, SPX_N);
        let sk_prf = std::slice::from_raw_parts(sk_prf, SPX_N);
        let optrand = std::slice::from_raw_parts(optrand, SPX_N);
        let m = std::slice::from_raw_parts(m, mlen as usize);
        crate::hash::gen_message_random(r, sk_prf, optrand, m, mlen, &*ctx);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
    r: *const u8, pk: *const u8, m: *const u8, mlen: u64,
    ctx: *const SpxCtx,
) {
    unsafe {
        let digest = std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES);
        let r = std::slice::from_raw_parts(r, SPX_N);
        let pk = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m = std::slice::from_raw_parts(m, mlen as usize);
        crate::hash::hash_message(digest, &mut *tree, &mut *leaf_idx, r, pk, m, mlen, &*ctx);
    }
}

// ============ thash.rs export ============

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8, input: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    unsafe {
        let out = std::slice::from_raw_parts_mut(out, SPX_N);
        let input = std::slice::from_raw_parts(input, inblocks as usize * SPX_N);
        let addr = &mut *(addr as *mut [u32; 8]);
        crate::thash::thash(out, input, inblocks as usize, &*ctx, addr);
    }
}

// ============ SHAKE backend exports ============

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub extern "C" fn shake256(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    unsafe {
        let output = std::slice::from_raw_parts_mut(output, outlen);
        let input = std::slice::from_raw_parts(input, inlen);
        crate::shake_backend::shake256_ffi(output, outlen, input, inlen);
    }
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    unsafe {
        let s = &mut *(s as *mut [u64; 25]);
        let input = std::slice::from_raw_parts(input, inlen);
        crate::shake_backend::shake256_absorb_ffi(s, input, inlen);
    }
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub extern "C" fn shake256_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    unsafe {
        let s = &mut *(s as *mut [u64; 25]);
        let output = std::slice::from_raw_parts_mut(output, nblocks * 136);
        crate::shake_backend::shake256_squeezeblocks_ffi(output, nblocks, s);
    }
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub extern "C" fn shake256_inc_init(s_inc: *mut u64) {
    unsafe {
        let s_inc = &mut *(s_inc as *mut [u64; 26]);
        crate::shake_backend::shake256_inc_init_ffi(s_inc);
    }
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub extern "C" fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    unsafe {
        let s_inc = &mut *(s_inc as *mut [u64; 26]);
        let input = std::slice::from_raw_parts(input, inlen);
        crate::shake_backend::shake256_inc_absorb_ffi(s_inc, input, inlen);
    }
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
    unsafe {
        let s_inc = &mut *(s_inc as *mut [u64; 26]);
        crate::shake_backend::shake256_inc_finalize_ffi(s_inc);
    }
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub extern "C" fn shake256_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    unsafe {
        let s_inc = &mut *(s_inc as *mut [u64; 26]);
        let output = std::slice::from_raw_parts_mut(output, outlen);
        crate::shake_backend::shake256_inc_squeeze_ffi(output, outlen, s_inc);
    }
}

// ============ BLAKE backend exports ============

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake256_mgf1(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    unsafe {
        let out = std::slice::from_raw_parts_mut(out, outlen as usize);
        let inp = std::slice::from_raw_parts(inp, inlen as usize);
        crate::blake_backend::blake256_mgf1(out, outlen as usize, inp, inlen as usize);
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake512_mgf1(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    unsafe {
        let out = std::slice::from_raw_parts_mut(out, outlen as usize);
        let inp = std::slice::from_raw_parts(inp, inlen as usize);
        crate::blake_backend::blake512_mgf1(out, outlen as usize, inp, inlen as usize);
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake256(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    unsafe {
        let out = std::slice::from_raw_parts_mut(out, 32);
        let inp = std::slice::from_raw_parts(inp, inlen as usize);
        crate::blake_backend::blake256(out, inp, inlen)
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake512(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    unsafe {
        let out = std::slice::from_raw_parts_mut(out, 64);
        let inp = std::slice::from_raw_parts(inp, inlen as usize);
        crate::blake_backend::blake512(out, inp, inlen)
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake256_init(s: *mut crate::blake_backend::BlakeState256) {
    unsafe { crate::blake_backend::blake256_init(&mut *s); }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake256_compress(s: *mut crate::blake_backend::BlakeState256, block: *const u8) {
    unsafe {
        let block = std::slice::from_raw_parts(block, 64);
        crate::blake_backend::blake256_compress_ffi(&mut *s, block);
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake256_update(s: *mut crate::blake_backend::BlakeState256, data: *const u8, datalen: u64) {
    unsafe {
        let data = std::slice::from_raw_parts(data, (datalen / 8) as usize);
        crate::blake_backend::blake256_update(&mut *s, data, datalen);
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake256_final(s: *mut crate::blake_backend::BlakeState256, digest: *mut u8) {
    unsafe {
        let digest = std::slice::from_raw_parts_mut(digest, 32);
        crate::blake_backend::blake256_final(&mut *s, digest);
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake512_init(s: *mut crate::blake_backend::BlakeState512) {
    unsafe { crate::blake_backend::blake512_init(&mut *s); }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake512_compress(s: *mut crate::blake_backend::BlakeState512, block: *const u8) {
    unsafe {
        let block = std::slice::from_raw_parts(block, 128);
        crate::blake_backend::blake512_compress_ffi(&mut *s, block);
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake512_update(s: *mut crate::blake_backend::BlakeState512, data: *const u8, datalen: u64) {
    unsafe {
        let data = std::slice::from_raw_parts(data, (datalen / 8) as usize);
        crate::blake_backend::blake512_update(&mut *s, data, datalen);
    }
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub extern "C" fn blake512_final(s: *mut crate::blake_backend::BlakeState512, digest: *mut u8) {
    unsafe {
        let digest = std::slice::from_raw_parts_mut(digest, 64);
        crate::blake_backend::blake512_final(&mut *s, digest);
    }
}

// ============ SHA2 backend exports ============

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_init(state: *mut u8) {
    unsafe { crate::sha2_backend::sha256_inc_init_ffi(std::slice::from_raw_parts_mut(state, 40)); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_blocks(state: *mut u8, data: *const u8, inblocks: usize) {
    unsafe { crate::sha2_backend::sha256_inc_blocks_ffi(std::slice::from_raw_parts_mut(state, 40), std::slice::from_raw_parts(data, inblocks * 64), inblocks); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_finalize(out: *mut u8, state: *mut u8, data: *const u8, inlen: usize) {
    unsafe { crate::sha2_backend::sha256_inc_finalize_ffi(std::slice::from_raw_parts_mut(out, 32), std::slice::from_raw_parts_mut(state, 40), std::slice::from_raw_parts(data, inlen), inlen); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn sha256(out: *mut u8, data: *const u8, inlen: usize) {
    unsafe { crate::sha2_backend::sha256_ffi(std::slice::from_raw_parts_mut(out, 32), std::slice::from_raw_parts(data, inlen), inlen); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_init(state: *mut u8) {
    unsafe { crate::sha2_backend::sha512_inc_init_ffi(std::slice::from_raw_parts_mut(state, 72)); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_blocks(state: *mut u8, data: *const u8, inblocks: usize) {
    unsafe { crate::sha2_backend::sha512_inc_blocks_ffi(std::slice::from_raw_parts_mut(state, 72), std::slice::from_raw_parts(data, inblocks * 128), inblocks); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_finalize(out: *mut u8, state: *mut u8, data: *const u8, inlen: usize) {
    unsafe { crate::sha2_backend::sha512_inc_finalize_ffi(std::slice::from_raw_parts_mut(out, 64), std::slice::from_raw_parts_mut(state, 72), std::slice::from_raw_parts(data, inlen), inlen); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn sha512(out: *mut u8, data: *const u8, inlen: usize) {
    unsafe { crate::sha2_backend::sha512_ffi(std::slice::from_raw_parts_mut(out, 64), std::slice::from_raw_parts(data, inlen), inlen); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_256(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    unsafe { crate::sha2_backend::mgf1_256_ffi(std::slice::from_raw_parts_mut(out, outlen as usize), outlen as usize, std::slice::from_raw_parts(inp, inlen as usize), inlen as usize); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_512(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    unsafe { crate::sha2_backend::mgf1_512_ffi(std::slice::from_raw_parts_mut(out, outlen as usize), outlen as usize, std::slice::from_raw_parts(inp, inlen as usize), inlen as usize); }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_seed_state(ctx: *mut SpxCtx) {
    unsafe { crate::sha2_backend::seed_state_ffi(&mut *ctx); }
}

// ============ Haraka backend exports ============

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_tweak_constants(ctx: *mut SpxCtx) {
    unsafe { crate::haraka_backend::tweak_constants(&mut *ctx); }
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_haraka_S_inc_init(s_inc: *mut u8) {
    unsafe { crate::haraka_backend::haraka_S_inc_init(&mut *(s_inc as *mut [u8; 65])); }
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_haraka_S_inc_absorb(s_inc: *mut u8, m: *const u8, mlen: usize, ctx: *const SpxCtx) {
    unsafe { crate::haraka_backend::haraka_S_inc_absorb(&mut *(s_inc as *mut [u8; 65]), std::slice::from_raw_parts(m, mlen), mlen, &*ctx); }
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_haraka_S_inc_finalize(s_inc: *mut u8) {
    unsafe { crate::haraka_backend::haraka_S_inc_finalize(&mut *(s_inc as *mut [u8; 65])); }
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_haraka_S_inc_squeeze(out: *mut u8, outlen: usize, s_inc: *mut u8, ctx: *const SpxCtx) {
    unsafe { crate::haraka_backend::haraka_S_inc_squeeze(std::slice::from_raw_parts_mut(out, outlen), outlen, &mut *(s_inc as *mut [u8; 65]), &*ctx); }
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_haraka_S(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64, ctx: *const SpxCtx) {
    unsafe { crate::haraka_backend::haraka_S(std::slice::from_raw_parts_mut(out, outlen as usize), outlen as usize, std::slice::from_raw_parts(inp, inlen as usize), inlen as usize, &*ctx); }
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_haraka512_perm(out: *mut u8, inp: *const u8, ctx: *const SpxCtx) {
    unsafe { crate::haraka_backend::haraka512_perm(std::slice::from_raw_parts_mut(out, 64), std::slice::from_raw_parts(inp, 64), &*ctx); }
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_haraka512(out: *mut u8, inp: *const u8, ctx: *const SpxCtx) {
    unsafe { crate::haraka_backend::haraka512(std::slice::from_raw_parts_mut(out, 32), std::slice::from_raw_parts(inp, 64), &*ctx); }
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub extern "C" fn SPX_haraka256(out: *mut u8, inp: *const u8, ctx: *const SpxCtx) {
    unsafe { crate::haraka_backend::haraka256(std::slice::from_raw_parts_mut(out, 32), std::slice::from_raw_parts(inp, 32), &*ctx); }
}
