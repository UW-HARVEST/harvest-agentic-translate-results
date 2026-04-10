use crate::params::*;
use crate::utils::{ull_to_bytes, u32_to_bytes};

/// Helper to get a mutable byte-level view of a `[u32; 8]` address.
#[inline]
fn addr_as_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
}

/// Helper to get an immutable byte-level view of a `[u32; 8]` address.
#[inline]
fn addr_as_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) }
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_as_bytes_mut(addr);
    ull_to_bytes(&mut bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8], 8, tree);
}

pub fn set_type(addr: &mut [u32; 8], type_val: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], in_addr: &[u32; 8]) {
    let src = addr_as_bytes(in_addr);
    let dst = addr_as_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4], keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], in_addr: &[u32; 8]) {
    let src = addr_as_bytes(in_addr);
    let dst = addr_as_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4], tree_index);
}

// --- extern "C" wrappers with SPX_ prefix ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_layer_addr(addr, layer);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_addr(addr, tree);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_type(addr, type_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, in_addr: *const u32) {
    let out = unsafe { &mut *(out as *mut [u32; 8]) };
    let in_addr = unsafe { &*(in_addr as *const [u32; 8]) };
    copy_subtree_addr(out, in_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_keypair_addr(addr, keypair);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, in_addr: *const u32) {
    let out = unsafe { &mut *(out as *mut [u32; 8]) };
    let in_addr = unsafe { &*(in_addr as *const [u32; 8]) };
    copy_keypair_addr(out, in_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_chain_addr(addr, chain);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_hash_addr(addr, hash);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_height(addr, tree_height);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_index(addr, tree_index);
}
