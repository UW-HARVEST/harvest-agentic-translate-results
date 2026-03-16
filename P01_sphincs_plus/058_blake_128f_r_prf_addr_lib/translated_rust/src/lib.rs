#![allow(non_snake_case, non_camel_case_types, clippy::missing_safety_doc)]

mod params;
mod blake256;
mod blake512;
mod utils;
mod address;
mod thash;
mod hash_blake;

use params::*;
use thash::SpxCtx;

// ============================================================
// Exported C-ABI functions
// ============================================================

// --- blake256 ---

#[unsafe(no_mangle)]
pub extern "C" fn blake256_init(s: *mut blake256::Blakestate256) {
    blake256::blake256_init(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_compress(s: *mut blake256::Blakestate256, block: *const u8) {
    let s = unsafe { &mut *s };
    let block = unsafe { core::slice::from_raw_parts(block, 64) };
    blake256::blake256_compress(s, block);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_update(s: *mut blake256::Blakestate256, data: *const u8, datalen: u64) {
    let s = unsafe { &mut *s };
    // datalen is in bits; max bytes needed is datalen/8 + some buffer
    let byte_len = if datalen > 0 { ((datalen + 7) / 8) as usize } else { 0 };
    let data = unsafe { core::slice::from_raw_parts(data, byte_len) };
    blake256::blake256_update(s, data, datalen);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_final(s: *mut blake256::Blakestate256, digest: *mut u8) {
    let s = unsafe { &mut *s };
    let digest = unsafe { core::slice::from_raw_parts_mut(digest, SPX_BLAKE256_OUTPUT_BYTES) };
    blake256::blake256_final(s, digest);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let out = unsafe { core::slice::from_raw_parts_mut(out, SPX_BLAKE256_OUTPUT_BYTES) };
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake256::blake256(out, inp, inlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake256_mgf1(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake256::blake256_mgf1(out, outlen as usize, inp, inlen as usize);
}

// --- blake512 ---

#[unsafe(no_mangle)]
pub extern "C" fn blake512_init(s: *mut blake512::Blakestate512) {
    blake512::blake512_init(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_compress(s: *mut blake512::Blakestate512, block: *const u8) {
    let s = unsafe { &mut *s };
    let block = unsafe { core::slice::from_raw_parts(block, 128) };
    blake512::blake512_compress(s, block);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_update(s: *mut blake512::Blakestate512, data: *const u8, datalen: u64) {
    let s = unsafe { &mut *s };
    let byte_len = if datalen > 0 { ((datalen + 7) / 8) as usize } else { 0 };
    let data = unsafe { core::slice::from_raw_parts(data, byte_len) };
    blake512::blake512_update(s, data, datalen);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_final(s: *mut blake512::Blakestate512, digest: *mut u8) {
    let s = unsafe { &mut *s };
    let digest = unsafe { core::slice::from_raw_parts_mut(digest, SPX_BLAKE512_OUTPUT_BYTES) };
    blake512::blake512_final(s, digest);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let out = unsafe { core::slice::from_raw_parts_mut(out, SPX_BLAKE512_OUTPUT_BYTES) };
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake512::blake512(out, inp, inlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake512_mgf1(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake512::blake512_mgf1(out, outlen as usize, inp, inlen as usize);
}

// --- utils ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    utils::ull_to_bytes(out, outlen as usize, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    let out = unsafe { core::slice::from_raw_parts_mut(out, 4) };
    utils::u32_to_bytes(out, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    utils::bytes_to_ull(inp, inlen as usize)
}

// --- address ---

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

// --- hash_blake ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    hash_blake::initialize_hash_function(unsafe { &mut *ctx });
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &*(addr as *const [u32; 8]) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    hash_blake::prf_addr(out, ctx, addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8, sk_prf: *const u8, optrand: *const u8,
    m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    let ctx = unsafe { &*ctx };
    let r = unsafe { core::slice::from_raw_parts_mut(r, SPX_BLAKEX_OUTPUT_BYTES) };
    let sk_prf = unsafe { core::slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand = unsafe { core::slice::from_raw_parts(optrand, SPX_N) };
    let m = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    hash_blake::gen_message_random(r, sk_prf, optrand, m, mlen, ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
    r: *const u8, pk: *const u8, m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    let ctx = unsafe { &*ctx };
    let tree = unsafe { &mut *tree };
    let leaf_idx = unsafe { &mut *leaf_idx };
    let r_slice = unsafe { core::slice::from_raw_parts(r, SPX_N) };
    let pk_slice = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen as usize) };

    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES
        + ((SPX_TREE_HEIGHT * (SPX_D - 1) + 7) / 8)
        + ((SPX_TREE_HEIGHT + 7) / 8);
    let digest = unsafe { core::slice::from_raw_parts_mut(digest, SPX_DGST_BYTES) };
    hash_blake::hash_message(digest, tree, leaf_idx, r_slice, pk_slice, m_slice, mlen, ctx);
}

// --- thash ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8, inp: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let inblocks = inblocks as usize;
    let out = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    let inp = unsafe { core::slice::from_raw_parts(inp, inblocks * SPX_N) };
    thash::thash(out, inp, inblocks, ctx, addr);
}

// --- compute_root ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8, leaf: *const u8,
    mut leaf_idx: u32, mut idx_offset: u32,
    auth_path: *const u8, tree_height: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let root = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let leaf = unsafe { core::slice::from_raw_parts(leaf, SPX_N) };
    let auth_path = unsafe { core::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N) };

    let mut buffer = [0u8; 2 * SPX_N];
    let mut auth_off = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(leaf);
        buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(leaf);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    }
    auth_off += SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        address::set_tree_height(addr, i + 1);
        address::set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp = buffer;
            thash::thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            let tmp = buffer;
            thash::thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    address::set_tree_height(addr, tree_height);
    address::set_tree_index(addr, leaf_idx + idx_offset);
    thash::thash(root, &buffer, 2, ctx, addr);
}

// --- treehash ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8, auth_path: *mut u8, ctx: *const SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    gen_leaf: unsafe extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32),
    tree_addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let tree_addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let root = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path = unsafe { core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N) };

    let th = tree_height as usize;
    let mut stack = vec![0u8; (th + 1) * SPX_N];
    let mut heights = vec![0u32; th + 1];
    let mut offset: usize = 0;

    for idx in 0..(1u32 << tree_height) {
        unsafe {
            gen_leaf(
                stack.as_mut_ptr().add(offset * SPX_N),
                ctx as *const SpxCtx,
                idx + idx_offset,
                tree_addr.as_ptr(),
            );
        }
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            address::set_tree_height(tree_addr, heights[offset - 1] + 1);
            address::set_tree_index(tree_addr, tree_idx + (idx_offset >> (heights[offset - 1] + 1)));

            let base = (offset - 2) * SPX_N;
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&stack[base..base + 2 * SPX_N]);
            thash::thash(&mut stack[base..base + SPX_N], &tmp, 2, ctx, tree_addr);

            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                auth_path[h * SPX_N..(h + 1) * SPX_N]
                    .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root.copy_from_slice(&stack[..SPX_N]);
}
