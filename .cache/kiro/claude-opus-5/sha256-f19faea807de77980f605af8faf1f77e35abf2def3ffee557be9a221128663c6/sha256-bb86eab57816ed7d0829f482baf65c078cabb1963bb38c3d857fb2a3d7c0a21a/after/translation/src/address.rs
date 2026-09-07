//! Translation of `app/src/address.c` and `app/include/address.h`.
//!
//! The C code treats the `uint32_t addr[8]` hash address purely as a raw byte
//! array (every accessor casts to `unsigned char *`), so the Rust side models it
//! as `[u8; 32]` and the `extern "C"` shims simply reinterpret the incoming
//! `uint32_t *`.

use crate::params::*;
use crate::utils::{u32_to_bytes, ull_to_bytes};
use core::ffi::c_uint;

/// A hash address; `uint32_t addr[8]` viewed as the byte array the C accessors
/// operate on.
pub type Addr = [u8; SPX_ADDR_BYTES];

/// An all-zero address, matching `uint32_t addr[8] = {0}`.
pub const ZERO_ADDR: Addr = [0u8; SPX_ADDR_BYTES];

/* The hash types that are passed to set_type */
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

/// Specify which level of Merkle tree (the "layer") we're working on.
#[inline]
pub fn set_layer_addr(addr: &mut Addr, layer: u32) {
    addr[SPX_OFFSET_LAYER] = layer as u8;
}

/// Specify which Merkle tree within the level (the "tree address") we're
/// working on.
#[inline]
pub fn set_tree_addr(addr: &mut Addr, tree: u64) {
    ull_to_bytes(&mut addr[SPX_OFFSET_TREE..], 8, tree);
}

/// Specify the reason we'll use this address structure for, that is, what hash
/// will we compute with it.
#[inline]
pub fn set_type(addr: &mut Addr, ty: u32) {
    addr[SPX_OFFSET_TYPE] = ty as u8;
}

/// Copy the layer and tree fields of the address structure.
#[inline]
pub fn copy_subtree_addr(out: &mut Addr, inp: &Addr) {
    let n = SPX_OFFSET_TREE + 8;
    out[..n].copy_from_slice(&inp[..n]);
}

/* These functions are used for OTS addresses. */

/// Specify which Merkle leaf we're working on; that is, which OTS keypair
/// we're talking about.
#[inline]
pub fn set_keypair_addr(addr: &mut Addr, keypair: u32) {
    u32_to_bytes(&mut addr[SPX_OFFSET_KP_ADDR..], keypair);
}

/// Copy the layer, tree and keypair fields of the address structure.
#[inline]
pub fn copy_keypair_addr(out: &mut Addr, inp: &Addr) {
    let n = SPX_OFFSET_TREE + 8;
    out[..n].copy_from_slice(&inp[..n]);
    out[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&inp[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

/// Specify which Merkle chain within the OTS we're working with.
#[inline]
pub fn set_chain_addr(addr: &mut Addr, chain: u32) {
    addr[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

/// Specify where in the Merkle chain we are.
#[inline]
pub fn set_hash_addr(addr: &mut Addr, hash: u32) {
    addr[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

/* These functions are used for all hash tree addresses (including FORS). */

/// Specify the height of the node in the Merkle/FORS tree we are in.
#[inline]
pub fn set_tree_height(addr: &mut Addr, tree_height: u32) {
    addr[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

/// Specify the distance from the left edge of the node in the Merkle/FORS tree.
#[inline]
pub fn set_tree_index(addr: &mut Addr, tree_index: u32) {
    u32_to_bytes(&mut addr[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// ---------------------------------------------------------------------------
// C ABI.  `SPX_NAMESPACE(s)` expands to `SPX_##s`.
// ---------------------------------------------------------------------------

#[inline]
pub(crate) unsafe fn addr_mut<'a>(addr: *mut u32) -> &'a mut Addr {
    &mut *(addr as *mut Addr)
}

#[inline]
pub(crate) unsafe fn addr_ref<'a>(addr: *const u32) -> &'a Addr {
    &*(addr as *const Addr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: c_uint) {
    set_layer_addr(addr_mut(addr), layer as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    set_tree_addr(addr_mut(addr), tree);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, ty: c_uint) {
    set_type(addr_mut(addr), ty as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    let src = *addr_ref(inp);
    copy_subtree_addr(addr_mut(out), &src);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: c_uint) {
    set_keypair_addr(addr_mut(addr), keypair as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    let src = *addr_ref(inp);
    copy_keypair_addr(addr_mut(out), &src);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: c_uint) {
    set_chain_addr(addr_mut(addr), chain as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: c_uint) {
    set_hash_addr(addr_mut(addr), hash as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: c_uint) {
    set_tree_height(addr_mut(addr), tree_height as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: c_uint) {
    set_tree_index(addr_mut(addr), tree_index as u32);
}
