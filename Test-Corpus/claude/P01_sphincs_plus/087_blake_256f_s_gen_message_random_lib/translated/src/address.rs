//! Translation of `app/src/address.c`.
//!
//! All accesses mirror the C code, which casts the `uint32_t addr[8]` to a byte
//! pointer and reads/writes individual bytes (or big-endian multi-byte fields
//! via `u32_to_bytes`/`ull_to_bytes`). This is endianness-independent because
//! every access goes through byte offsets.

use crate::params::*;
use crate::utils::{SPX_u32_to_bytes, SPX_ull_to_bytes};

#[inline]
unsafe fn byte_ptr(addr: *mut u32) -> *mut u8 {
    addr as *mut u8
}

/// Specify which level of Merkle tree (the "layer") we're working on.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    *byte_ptr(addr).add(SPX_OFFSET_LAYER) = layer as u8;
}

/// Specify which Merkle tree within the level (the "tree address").
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    SPX_ull_to_bytes(byte_ptr(addr).add(SPX_OFFSET_TREE), 8, tree);
}

/// Specify the reason we'll use this address structure for.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, atype: u32) {
    *byte_ptr(addr).add(SPX_OFFSET_TYPE) = atype as u8;
}

/// Copy the layer and tree fields of the address structure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    core::ptr::copy_nonoverlapping(inp as *const u8, out as *mut u8, SPX_OFFSET_TREE + 8);
}

/// Specify which Merkle leaf we're working on (which OTS keypair).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    SPX_u32_to_bytes(byte_ptr(addr).add(SPX_OFFSET_KP_ADDR), keypair);
}

/// Copy the layer, tree and keypair fields of the address structure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    core::ptr::copy_nonoverlapping(inp as *const u8, out as *mut u8, SPX_OFFSET_TREE + 8);
    core::ptr::copy_nonoverlapping(
        (inp as *const u8).add(SPX_OFFSET_KP_ADDR),
        (out as *mut u8).add(SPX_OFFSET_KP_ADDR),
        4,
    );
}

/// Specify which Merkle chain within the OTS we're working with.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    *byte_ptr(addr).add(SPX_OFFSET_CHAIN_ADDR) = chain as u8;
}

/// Specify where in the Merkle chain we are (the hash address).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    *byte_ptr(addr).add(SPX_OFFSET_HASH_ADDR) = hash as u8;
}

/// Specify the height of the node in the Merkle/FORS tree we are in.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    *byte_ptr(addr).add(SPX_OFFSET_TREE_HGT) = tree_height as u8;
}

/// Specify the distance from the left edge of the node in the Merkle/FORS tree.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    SPX_u32_to_bytes(byte_ptr(addr).add(SPX_OFFSET_TREE_INDEX), tree_index);
}
