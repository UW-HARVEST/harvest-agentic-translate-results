// Translation of c_src/app/src/address.c

use core::slice;

use crate::params::{
    SPX_OFFSET_CHAIN_ADDR, SPX_OFFSET_HASH_ADDR, SPX_OFFSET_KP_ADDR, SPX_OFFSET_LAYER,
    SPX_OFFSET_TREE, SPX_OFFSET_TREE_HGT, SPX_OFFSET_TREE_INDEX, SPX_OFFSET_TYPE,
};
use crate::utils::{u32_to_bytes, ull_to_bytes};

// Reinterpret addr (8 u32 words) as 32-byte buffer.
fn addr_as_bytes_mut(addr: &mut [u32]) -> &mut [u8] {
    let p = addr.as_mut_ptr() as *mut u8;
    unsafe { slice::from_raw_parts_mut(p, 32) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_LAYER] = layer as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    let bytes = addr_as_bytes_mut(addr);
    let dst = &mut bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8];
    ull_to_bytes(dst, tree);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, type_: u32) {
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_TYPE] = type_ as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, input: *const u32) {
    let out = unsafe { slice::from_raw_parts_mut(out, 8) };
    let input = unsafe { slice::from_raw_parts(input, 8) };
    let n = SPX_OFFSET_TREE + 8;
    let out_b = addr_as_bytes_mut(out);
    let in_b = unsafe { slice::from_raw_parts(input.as_ptr() as *const u8, 32) };
    out_b[..n].copy_from_slice(&in_b[..n]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    let bytes = addr_as_bytes_mut(addr);
    let dst = &mut bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4];
    u32_to_bytes(dst, keypair);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, input: *const u32) {
    let out = unsafe { slice::from_raw_parts_mut(out, 8) };
    let input = unsafe { slice::from_raw_parts(input, 8) };
    let n = SPX_OFFSET_TREE + 8;
    let out_b = addr_as_bytes_mut(out);
    let in_b = unsafe { slice::from_raw_parts(input.as_ptr() as *const u8, 32) };
    out_b[..n].copy_from_slice(&in_b[..n]);
    out_b[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&in_b[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    let bytes = addr_as_bytes_mut(addr);
    let dst = &mut bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4];
    u32_to_bytes(dst, tree_index);
}

// Pure-Rust helpers usable internally without going through FFI ptrs.
pub fn set_layer_addr_inner(addr: &mut [u32], layer: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr_inner(addr: &mut [u32], tree: u64) {
    let bytes = addr_as_bytes_mut(addr);
    ull_to_bytes(&mut bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8], tree);
}

pub fn set_type_inner(addr: &mut [u32], type_: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_TYPE] = type_ as u8;
}

pub fn copy_subtree_addr_inner(out: &mut [u32], input: &[u32]) {
    let n = SPX_OFFSET_TREE + 8;
    let in_bytes_ptr = input.as_ptr() as *const u8;
    let in_b = unsafe { slice::from_raw_parts(in_bytes_ptr, 32) };
    let out_b = addr_as_bytes_mut(out);
    out_b[..n].copy_from_slice(&in_b[..n]);
}

pub fn set_keypair_addr_inner(addr: &mut [u32], keypair: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4], keypair);
}

pub fn copy_keypair_addr_inner(out: &mut [u32], input: &[u32]) {
    let n = SPX_OFFSET_TREE + 8;
    let in_bytes_ptr = input.as_ptr() as *const u8;
    let in_b = unsafe { slice::from_raw_parts(in_bytes_ptr, 32) };
    let out_b = addr_as_bytes_mut(out);
    out_b[..n].copy_from_slice(&in_b[..n]);
    out_b[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&in_b[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr_inner(addr: &mut [u32], chain: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr_inner(addr: &mut [u32], hash: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height_inner(addr: &mut [u32], tree_height: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index_inner(addr: &mut [u32], tree_index: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(
        &mut bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4],
        tree_index,
    );
}
