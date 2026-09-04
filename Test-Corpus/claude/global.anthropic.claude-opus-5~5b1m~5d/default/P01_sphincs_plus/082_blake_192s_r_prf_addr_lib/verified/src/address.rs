//! Translation of `app/src/address.c`.
//!
//! Addresses are `uint32_t addr[8]` in C, but every access is performed
//! through a byte view (`(unsigned char *)addr`). We keep the `[u32; 8]`
//! representation and manipulate its native-endian bytes, which reproduces the
//! exact in-memory layout the C code hashes on a little-endian target.

use crate::params::*;
use crate::utils::{u32_to_bytes, ull_to_bytes};

/// View a `[u32; 8]` address as its 32 raw bytes (native endianness).
#[inline]
pub fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    // Safe: `[u32; 8]` and `[u8; 32]` have identical size; alignment of the
    // source (4) exceeds that of the destination (1).
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

#[inline]
fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let b = addr_bytes_mut(addr);
    ull_to_bytes(&mut b[SPX_OFFSET_TREE..], 8, tree);
}

pub fn set_type(addr: &mut [u32; 8], t: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = t as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let b = addr_bytes_mut(addr);
    u32_to_bytes(&mut b[SPX_OFFSET_KP_ADDR..], keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = *addr_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let b = addr_bytes_mut(addr);
    u32_to_bytes(&mut b[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// ------------------------------------------------------------------
// Exported C ABI wrappers (namespaced `SPX_*` linker symbols).
// ------------------------------------------------------------------

macro_rules! as_arr8_mut {
    ($p:expr) => {
        &mut *($p as *mut [u32; 8])
    };
}
macro_rules! as_arr8 {
    ($p:expr) => {
        &*($p as *const [u32; 8])
    };
}

#[no_mangle]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    set_layer_addr(as_arr8_mut!(addr), layer);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    set_tree_addr(as_arr8_mut!(addr), tree);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, t: u32) {
    set_type(as_arr8_mut!(addr), t);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    copy_subtree_addr(as_arr8_mut!(out), as_arr8!(inp));
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    set_keypair_addr(as_arr8_mut!(addr), keypair);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    set_chain_addr(as_arr8_mut!(addr), chain);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    set_hash_addr(as_arr8_mut!(addr), hash);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    copy_keypair_addr(as_arr8_mut!(out), as_arr8!(inp));
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    set_tree_height(as_arr8_mut!(addr), tree_height);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    set_tree_index(as_arr8_mut!(addr), tree_index);
}
