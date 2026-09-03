//! Translation of `app/src/address.c` / `app/include/address.h`.
//!
//! The C code treats the `uint32_t addr[8]` array as a raw byte buffer and
//! writes individual bytes at the `SPX_OFFSET_*` positions.  We reproduce that
//! exactly by re-interpreting the `[u32; 8]` as a `[u8; 32]` in native byte
//! order (little-endian on x86_64), which is precisely what
//! `(unsigned char *)addr` does in C.

use crate::params::{
    SPX_OFFSET_CHAIN_ADDR, SPX_OFFSET_HASH_ADDR, SPX_OFFSET_KP_ADDR, SPX_OFFSET_LAYER,
    SPX_OFFSET_TREE, SPX_OFFSET_TREE_HGT, SPX_OFFSET_TREE_INDEX, SPX_OFFSET_TYPE,
};
use crate::utils::{u32_to_bytes, ull_to_bytes};

/* The hash types that are passed to set_type */
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

/// Native-endian byte view of the address, exactly matching
/// `(const unsigned char *)addr` in C.
#[inline]
pub fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

/// Native-endian mutable byte view of the address, exactly matching
/// `(unsigned char *)addr` in C.
#[inline]
pub fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}

/*
 * Specify which level of Merkle tree (the "layer") we're working on
 */
pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

/*
 * Specify which Merkle tree within the level (the "tree address") we're
 * working on
 */
pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    // #if (SPX_TREE_HEIGHT * (SPX_D - 1)) > 64 -> #error
    // (statically satisfied for all supported parameter sets)
    let bytes = addr_bytes_mut(addr);
    ull_to_bytes(&mut bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8], 8, tree);
}

/*
 * Specify the reason we'll use this address structure for, that is, what
 * hash will we compute with it.
 */
pub fn set_type(addr: &mut [u32; 8], type_: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_ as u8;
}

/*
 * Copy the layer and tree fields of the address structure.
 */
pub fn copy_subtree_addr(out: &mut [u32; 8], input: &[u32; 8]) {
    const LEN: usize = SPX_OFFSET_TREE + 8;
    let src = addr_bytes(input);
    let dst = addr_bytes_mut(out);
    dst[..LEN].copy_from_slice(&src[..LEN]);
}

/* These functions are used for OTS addresses. */

/*
 * Specify which Merkle leaf we're working on; that is, which OTS keypair
 * we're talking about.
 */
pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4], keypair);
}

/*
 * Copy the layer, tree and keypair fields of the address structure.
 */
pub fn copy_keypair_addr(out: &mut [u32; 8], input: &[u32; 8]) {
    const LEN: usize = SPX_OFFSET_TREE + 8;
    let src = *addr_bytes(input);
    let dst = addr_bytes_mut(out);
    dst[..LEN].copy_from_slice(&src[..LEN]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

/*
 * Specify which Merkle chain within the OTS we're working with
 * (the chain address)
 */
pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

/*
 * Specify where in the Merkle chain we are (the hash address)
 */
pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

/* These functions are used for all hash tree addresses (including FORS). */

/*
 * Specify the height of the node in the Merkle/FORS tree we are in
 * (the tree height)
 */
pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

/*
 * Specify the distance from the left edge of the node in the Merkle/FORS tree
 * (the tree index)
 */
pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes = addr_bytes_mut(addr);
    u32_to_bytes(
        &mut bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4],
        tree_index,
    );
}

// ---------------------------------------------------------------------------
// C ABI wrappers (exported linker symbols carry the `SPX_` namespace prefix)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn as_addr_mut(addr: *mut u32) -> &'static mut [u32; 8] {
    unsafe { &mut *(addr as *mut [u32; 8]) }
}

#[inline]
unsafe fn as_addr(addr: *const u32) -> &'static [u32; 8] {
    unsafe { &*(addr as *const [u32; 8]) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    unsafe { set_layer_addr(as_addr_mut(addr), layer) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    unsafe { set_tree_addr(as_addr_mut(addr), tree) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_: u32) {
    unsafe { set_type(as_addr_mut(addr), type_) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, input: *const u32) {
    unsafe {
        let src = *as_addr(input);
        copy_subtree_addr(as_addr_mut(out), &src)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    unsafe { set_keypair_addr(as_addr_mut(addr), keypair) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    unsafe { set_chain_addr(as_addr_mut(addr), chain) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    unsafe { set_hash_addr(as_addr_mut(addr), hash) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, input: *const u32) {
    unsafe {
        let src = *as_addr(input);
        copy_keypair_addr(as_addr_mut(out), &src)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    unsafe { set_tree_height(as_addr_mut(addr), tree_height) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    unsafe { set_tree_index(as_addr_mut(addr), tree_index) }
}
