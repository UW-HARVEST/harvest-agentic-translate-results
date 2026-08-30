//! Translation of `app/src/address.c` and `app/include/address.h`.
//!
//! The C code declares addresses as `uint32_t addr[8]` but manipulates them
//! through an `unsigned char *` alias, so the byte layout is the platform's
//! native word order.  The helpers below expose the same byte view.

use crate::params::*;
use crate::utils::{u32_to_bytes, ull_to_bytes};

/* The hash types that are passed to set_type */
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

/// `(const unsigned char *)addr`
#[inline(always)]
pub fn addr_bytes(addr: &[u32; 8]) -> &[u8; SPX_ADDR_BYTES] {
    // SAFETY: `[u32; 8]` and `[u8; 32]` have the same size, and `[u8; 32]` has
    // weaker alignment requirements.
    unsafe { &*(addr.as_ptr() as *const [u8; SPX_ADDR_BYTES]) }
}

/// `(unsigned char *)addr`
#[inline(always)]
pub fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; SPX_ADDR_BYTES] {
    // SAFETY: see `addr_bytes`.
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; SPX_ADDR_BYTES]) }
}

/// Specify which level of Merkle tree (the "layer") we're working on.
#[inline]
pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

/// Specify which Merkle tree within the level (the "tree address") we're
/// working on.
#[inline]
pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    const _: () = assert!(
        SPX_TREE_HEIGHT * (SPX_D - 1) <= 64,
        "Subtree addressing is currently limited to at most 2^64 trees"
    );
    ull_to_bytes(
        &mut addr_bytes_mut(addr)[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8],
        tree,
    );
}

/// Specify the reason we'll use this address structure for, that is, what hash
/// will we compute with it.
#[inline]
pub fn set_type(addr: &mut [u32; 8], ty: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = ty as u8;
}

/// Copy the layer and tree fields of the address structure.
#[inline]
pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let n = SPX_OFFSET_TREE + 8;
    let src = *addr_bytes(inp);
    addr_bytes_mut(out)[..n].copy_from_slice(&src[..n]);
}

/* These functions are used for OTS addresses. */

/// Specify which Merkle leaf we're working on; that is, which OTS keypair.
#[inline]
pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let b = addr_bytes_mut(addr);
    u32_to_bytes(
        (&mut b[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4])
            .try_into()
            .unwrap(),
        keypair,
    );
}

/// Copy the layer, tree and keypair fields of the address structure.
#[inline]
pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let n = SPX_OFFSET_TREE + 8;
    let src = *addr_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..n].copy_from_slice(&src[..n]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

/// Specify which Merkle chain within the OTS we're working with.
#[inline]
pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

/// Specify where in the Merkle chain we are.
#[inline]
pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

/* These functions are used for all hash tree addresses (including FORS). */

/// Specify the height of the node in the Merkle/FORS tree we are in.
#[inline]
pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

/// Specify the distance from the left edge of the node in the Merkle/FORS tree.
#[inline]
pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let b = addr_bytes_mut(addr);
    u32_to_bytes(
        (&mut b[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4])
            .try_into()
            .unwrap(),
        tree_index,
    );
}

// ---------------------------------------------------------------------------
// C ABI.  `address.h` renames every function through `SPX_NAMESPACE`.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    set_layer_addr(unsafe { &mut *(addr as *mut [u32; 8]) }, layer);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    set_tree_addr(unsafe { &mut *(addr as *mut [u32; 8]) }, tree);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, ty: u32) {
    set_type(unsafe { &mut *(addr as *mut [u32; 8]) }, ty);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    copy_subtree_addr(unsafe { &mut *(out as *mut [u32; 8]) }, unsafe {
        &*(inp as *const [u32; 8])
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    set_keypair_addr(unsafe { &mut *(addr as *mut [u32; 8]) }, keypair);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    copy_keypair_addr(unsafe { &mut *(out as *mut [u32; 8]) }, unsafe {
        &*(inp as *const [u32; 8])
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    set_chain_addr(unsafe { &mut *(addr as *mut [u32; 8]) }, chain);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    set_hash_addr(unsafe { &mut *(addr as *mut [u32; 8]) }, hash);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    set_tree_height(unsafe { &mut *(addr as *mut [u32; 8]) }, tree_height);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    set_tree_index(unsafe { &mut *(addr as *mut [u32; 8]) }, tree_index);
}
