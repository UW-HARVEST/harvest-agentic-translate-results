// SPHINCS+ address manipulation. Symbols are linker-renamed via SPX_NAMESPACE
// in C, so we expose them here as SPX_<funcname>.

use crate::params::*;
use crate::utils::{SPX_u32_to_bytes, SPX_ull_to_bytes};

#[inline]
fn addr_byte_mut(addr: &mut [u32; 8], offset: usize) -> &mut u8 {
    let bytes = unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) };
    &mut bytes[offset]
}

#[inline]
fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}

#[inline]
fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

/* Specify which level of Merkle tree (the "layer") we're working on */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut [u32; 8], layer: u32) {
    let a = unsafe { &mut *addr };
    *addr_byte_mut(a, SPX_OFFSET_LAYER) = layer as u8;
}

/* Specify which Merkle tree within the level we're working on */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut [u32; 8], tree: u64) {
    let a = unsafe { &mut *addr };
    let bytes = addr_bytes_mut(a);
    unsafe {
        SPX_ull_to_bytes(
            bytes.as_mut_ptr().add(SPX_OFFSET_TREE),
            8,
            tree,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_type(addr: *mut [u32; 8], typ: u32) {
    let a = unsafe { &mut *addr };
    *addr_byte_mut(a, SPX_OFFSET_TYPE) = typ as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut [u32; 8], inp: *const [u32; 8]) {
    let inp_ref = unsafe { &*inp };
    let out_ref = unsafe { &mut *out };
    let copy_len = SPX_OFFSET_TREE + 8;
    let in_bytes = addr_bytes(inp_ref);
    let out_bytes = addr_bytes_mut(out_ref);
    out_bytes[..copy_len].copy_from_slice(&in_bytes[..copy_len]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut [u32; 8], keypair: u32) {
    let a = unsafe { &mut *addr };
    let bytes = addr_bytes_mut(a);
    unsafe {
        SPX_u32_to_bytes(bytes.as_mut_ptr().add(SPX_OFFSET_KP_ADDR), keypair);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut [u32; 8], inp: *const [u32; 8]) {
    let inp_ref = unsafe { &*inp };
    let out_ref = unsafe { &mut *out };
    let copy_len = SPX_OFFSET_TREE + 8;
    let in_bytes = addr_bytes(inp_ref);
    let out_bytes = addr_bytes_mut(out_ref);
    out_bytes[..copy_len].copy_from_slice(&in_bytes[..copy_len]);
    // copy 4 bytes at SPX_OFFSET_KP_ADDR
    let kp = SPX_OFFSET_KP_ADDR;
    out_bytes[kp..kp + 4].copy_from_slice(&in_bytes[kp..kp + 4]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut [u32; 8], chain: u32) {
    let a = unsafe { &mut *addr };
    *addr_byte_mut(a, SPX_OFFSET_CHAIN_ADDR) = chain as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut [u32; 8], hash: u32) {
    let a = unsafe { &mut *addr };
    *addr_byte_mut(a, SPX_OFFSET_HASH_ADDR) = hash as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut [u32; 8], tree_height: u32) {
    let a = unsafe { &mut *addr };
    *addr_byte_mut(a, SPX_OFFSET_TREE_HGT) = tree_height as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut [u32; 8], tree_index: u32) {
    let a = unsafe { &mut *addr };
    let bytes = addr_bytes_mut(a);
    unsafe {
        SPX_u32_to_bytes(bytes.as_mut_ptr().add(SPX_OFFSET_TREE_INDEX), tree_index);
    }
}

// Safe Rust wrappers for internal use.
pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    unsafe { SPX_set_layer_addr(addr as *mut _, layer) }
}
pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    unsafe { SPX_set_tree_addr(addr as *mut _, tree) }
}
pub fn set_type(addr: &mut [u32; 8], typ: u32) {
    unsafe { SPX_set_type(addr as *mut _, typ) }
}
pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    unsafe { SPX_copy_subtree_addr(out as *mut _, inp as *const _) }
}
pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    unsafe { SPX_set_keypair_addr(addr as *mut _, keypair) }
}
pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    unsafe { SPX_copy_keypair_addr(out as *mut _, inp as *const _) }
}
pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    unsafe { SPX_set_chain_addr(addr as *mut _, chain) }
}
pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    unsafe { SPX_set_hash_addr(addr as *mut _, hash) }
}
pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    unsafe { SPX_set_tree_height(addr as *mut _, tree_height) }
}
pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    unsafe { SPX_set_tree_index(addr as *mut _, tree_index) }
}

// Helper: write address bytes to a buffer (used for hash inputs).
pub fn addr_to_bytes(addr: &[u32; 8]) -> [u8; 32] {
    *addr_bytes(addr)
}
