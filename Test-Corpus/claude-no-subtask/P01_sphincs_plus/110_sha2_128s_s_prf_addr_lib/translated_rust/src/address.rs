// Address manipulation routines

use crate::params::offsets::*;
use crate::utils;

pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

#[inline]
fn addr_bytes(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    // Cast u32 array to u8 array safely (same memory representation)
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}

#[inline]
fn addr_bytes_const(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    let bytes = addr_bytes(addr);
    bytes[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_bytes(addr);
    utils::ull_to_bytes(&mut bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8], 8, tree);
}

pub fn set_type(addr: &mut [u32; 8], type_: u32) {
    let bytes = addr_bytes(addr);
    bytes[SPX_OFFSET_TYPE] = type_ as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], in_: &[u32; 8]) {
    // memcpy(out, in, SPX_OFFSET_TREE+8)
    let n = SPX_OFFSET_TREE + 8;
    let out_b = addr_bytes(out);
    let in_b = addr_bytes_const(in_);
    out_b[..n].copy_from_slice(&in_b[..n]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_bytes(addr);
    utils::u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4], keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], in_: &[u32; 8]) {
    let n = SPX_OFFSET_TREE + 8;
    {
        let out_b = addr_bytes(out);
        let in_b = addr_bytes_const(in_);
        out_b[..n].copy_from_slice(&in_b[..n]);
    }
    // copy 4 bytes at SPX_OFFSET_KP_ADDR
    let in_b = addr_bytes_const(in_);
    let kp_bytes = [
        in_b[SPX_OFFSET_KP_ADDR],
        in_b[SPX_OFFSET_KP_ADDR + 1],
        in_b[SPX_OFFSET_KP_ADDR + 2],
        in_b[SPX_OFFSET_KP_ADDR + 3],
    ];
    let out_b = addr_bytes(out);
    out_b[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4].copy_from_slice(&kp_bytes);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    let bytes = addr_bytes(addr);
    bytes[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    let bytes = addr_bytes(addr);
    bytes[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    let bytes = addr_bytes(addr);
    bytes[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes = addr_bytes(addr);
    utils::u32_to_bytes(
        &mut bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4],
        tree_index,
    );
}

// C ABI exports (with namespace prefix)
use std::ffi::c_uint;
use std::os::raw::c_void;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_layer_addr(a, layer);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_addr(a, tree);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, type_: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_type(a, type_);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, in_: *const u32) {
    let o = unsafe { &mut *(out as *mut [u32; 8]) };
    let i = unsafe { &*(in_ as *const [u32; 8]) };
    copy_subtree_addr(o, i);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_keypair_addr(a, keypair);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_chain_addr(a, chain);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_hash_addr(a, hash);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, in_: *const u32) {
    let o = unsafe { &mut *(out as *mut [u32; 8]) };
    let i = unsafe { &*(in_ as *const [u32; 8]) };
    copy_keypair_addr(o, i);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_height(a, tree_height);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_index(a, tree_index);
}

// silence unused warnings
#[allow(dead_code)]
fn _unused(_: c_uint, _: *mut c_void) {}
