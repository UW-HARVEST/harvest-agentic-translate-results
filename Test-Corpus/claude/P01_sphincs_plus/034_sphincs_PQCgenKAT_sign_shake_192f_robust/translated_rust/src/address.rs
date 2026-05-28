use crate::params::offsets::*;
use crate::utils::{u32_to_bytes, ull_to_bytes};

pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

#[inline]
fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    // Safety: 8 * u32 is exactly 32 bytes, and `addr` is suitably aligned.
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
}

#[inline]
fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) }
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_bytes_mut(addr);
    let slice = &mut bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8];
    ull_to_bytes(slice, 8, tree);
}

pub fn set_type(addr: &mut [u32; 8], type_: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_ as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], input: &[u32; 8]) {
    // Copy bytes [0 .. SPX_OFFSET_TREE+8]
    let len = SPX_OFFSET_TREE + 8;
    let dst = addr_bytes_mut(out);
    let src = addr_bytes(input);
    dst[..len].copy_from_slice(&src[..len]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_bytes_mut(addr);
    let slice = &mut bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4];
    u32_to_bytes(slice, keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], input: &[u32; 8]) {
    let len_subtree = SPX_OFFSET_TREE + 8;
    let dst = addr_bytes_mut(out);
    // Copy from input (we keep a snapshot to avoid aliasing concerns)
    let src_full = *addr_bytes(input);
    dst[..len_subtree].copy_from_slice(&src_full[..len_subtree]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src_full[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
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
    let bytes = addr_bytes_mut(addr);
    let slice = &mut bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4];
    u32_to_bytes(slice, tree_index);
}

// ----- C-ABI exports (with SPX_ namespace prefix) -----

unsafe fn ptr_to_arr_mut<'a>(p: *mut u32) -> &'a mut [u32; 8] {
    unsafe { &mut *(p as *mut [u32; 8]) }
}
unsafe fn ptr_to_arr<'a>(p: *const u32) -> &'a [u32; 8] {
    unsafe { &*(p as *const [u32; 8]) }
}

#[unsafe(export_name = "SPX_set_layer_addr")]
pub unsafe extern "C" fn c_set_layer_addr(addr: *mut u32, layer: u32) {
    set_layer_addr(unsafe { ptr_to_arr_mut(addr) }, layer);
}

#[unsafe(export_name = "SPX_set_tree_addr")]
pub unsafe extern "C" fn c_set_tree_addr(addr: *mut u32, tree: u64) {
    set_tree_addr(unsafe { ptr_to_arr_mut(addr) }, tree);
}

#[unsafe(export_name = "SPX_set_type")]
pub unsafe extern "C" fn c_set_type(addr: *mut u32, type_: u32) {
    set_type(unsafe { ptr_to_arr_mut(addr) }, type_);
}

#[unsafe(export_name = "SPX_copy_subtree_addr")]
pub unsafe extern "C" fn c_copy_subtree_addr(out: *mut u32, input: *const u32) {
    copy_subtree_addr(unsafe { ptr_to_arr_mut(out) }, unsafe { ptr_to_arr(input) });
}

#[unsafe(export_name = "SPX_set_keypair_addr")]
pub unsafe extern "C" fn c_set_keypair_addr(addr: *mut u32, kp: u32) {
    set_keypair_addr(unsafe { ptr_to_arr_mut(addr) }, kp);
}

#[unsafe(export_name = "SPX_set_chain_addr")]
pub unsafe extern "C" fn c_set_chain_addr(addr: *mut u32, chain: u32) {
    set_chain_addr(unsafe { ptr_to_arr_mut(addr) }, chain);
}

#[unsafe(export_name = "SPX_set_hash_addr")]
pub unsafe extern "C" fn c_set_hash_addr(addr: *mut u32, hash: u32) {
    set_hash_addr(unsafe { ptr_to_arr_mut(addr) }, hash);
}

#[unsafe(export_name = "SPX_copy_keypair_addr")]
pub unsafe extern "C" fn c_copy_keypair_addr(out: *mut u32, input: *const u32) {
    copy_keypair_addr(unsafe { ptr_to_arr_mut(out) }, unsafe { ptr_to_arr(input) });
}

#[unsafe(export_name = "SPX_set_tree_height")]
pub unsafe extern "C" fn c_set_tree_height(addr: *mut u32, h: u32) {
    set_tree_height(unsafe { ptr_to_arr_mut(addr) }, h);
}

#[unsafe(export_name = "SPX_set_tree_index")]
pub unsafe extern "C" fn c_set_tree_index(addr: *mut u32, idx: u32) {
    set_tree_index(unsafe { ptr_to_arr_mut(addr) }, idx);
}
