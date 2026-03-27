use crate::params::*;
use crate::utils::{ull_to_bytes, u32_to_bytes};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let bytes = addr as *mut u8;
    *bytes.add(SPX_OFFSET_LAYER) = layer as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let bytes = addr as *mut u8;
    ull_to_bytes(bytes.add(SPX_OFFSET_TREE), 8, tree);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, type_: u32) {
    let bytes = addr as *mut u8;
    *bytes.add(SPX_OFFSET_TYPE) = type_ as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, in_: *const u32) {
    core::ptr::copy_nonoverlapping(in_ as *const u8, out as *mut u8, SPX_OFFSET_TREE + 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let bytes = addr as *mut u8;
    u32_to_bytes(bytes.add(SPX_OFFSET_KP_ADDR), keypair);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, in_: *const u32) {
    core::ptr::copy_nonoverlapping(in_ as *const u8, out as *mut u8, SPX_OFFSET_TREE + 8);
    core::ptr::copy_nonoverlapping(
        (in_ as *const u8).add(SPX_OFFSET_KP_ADDR),
        (out as *mut u8).add(SPX_OFFSET_KP_ADDR),
        4,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let bytes = addr as *mut u8;
    *bytes.add(SPX_OFFSET_CHAIN_ADDR) = chain as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let bytes = addr as *mut u8;
    *bytes.add(SPX_OFFSET_HASH_ADDR) = hash as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let bytes = addr as *mut u8;
    *bytes.add(SPX_OFFSET_TREE_HGT) = tree_height as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let bytes = addr as *mut u8;
    u32_to_bytes(bytes.add(SPX_OFFSET_TREE_INDEX), tree_index);
}

// Safe wrappers used internally
pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    unsafe { SPX_set_layer_addr(addr.as_mut_ptr(), layer) }
}
pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    unsafe { SPX_set_tree_addr(addr.as_mut_ptr(), tree) }
}
pub fn set_type(addr: &mut [u32; 8], type_: u32) {
    unsafe { SPX_set_type(addr.as_mut_ptr(), type_) }
}
pub fn copy_subtree_addr(out: &mut [u32; 8], in_: &[u32; 8]) {
    unsafe { SPX_copy_subtree_addr(out.as_mut_ptr(), in_.as_ptr()) }
}
pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    unsafe { SPX_set_keypair_addr(addr.as_mut_ptr(), keypair) }
}
pub fn copy_keypair_addr(out: &mut [u32; 8], in_: &[u32; 8]) {
    unsafe { SPX_copy_keypair_addr(out.as_mut_ptr(), in_.as_ptr()) }
}
pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    unsafe { SPX_set_chain_addr(addr.as_mut_ptr(), chain) }
}
pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    unsafe { SPX_set_hash_addr(addr.as_mut_ptr(), hash) }
}
pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    unsafe { SPX_set_tree_height(addr.as_mut_ptr(), tree_height) }
}
pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    unsafe { SPX_set_tree_index(addr.as_mut_ptr(), tree_index) }
}
