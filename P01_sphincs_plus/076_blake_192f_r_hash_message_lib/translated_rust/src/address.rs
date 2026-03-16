use crate::params::*;
use crate::utils::{u32_to_bytes, ull_to_bytes};

fn addr_bytes(addr: *mut u32) -> *mut u8 {
    addr as *mut u8
}

fn addr_bytes_const(addr: *const u32) -> *const u8 {
    addr as *const u8
}

pub fn set_layer_addr(addr: *mut u32, layer: u32) {
    unsafe {
        *addr_bytes(addr).add(SPX_OFFSET_LAYER) = layer as u8;
    }
}

pub fn set_tree_addr(addr: *mut u32, tree: u64) {
    unsafe {
        ull_to_bytes(
            std::slice::from_raw_parts_mut(addr_bytes(addr).add(SPX_OFFSET_TREE), 8),
            8,
            tree,
        );
    }
}

pub fn set_type(addr: *mut u32, type_val: u32) {
    unsafe {
        *addr_bytes(addr).add(SPX_OFFSET_TYPE) = type_val as u8;
    }
}

pub fn copy_subtree_addr(out: *mut u32, inp: *const u32) {
    unsafe {
        std::ptr::copy_nonoverlapping(inp as *const u8, out as *mut u8, SPX_OFFSET_TREE + 8);
    }
}

pub fn set_keypair_addr(addr: *mut u32, keypair: u32) {
    unsafe {
        u32_to_bytes(
            std::slice::from_raw_parts_mut(addr_bytes(addr).add(SPX_OFFSET_KP_ADDR), 4),
            keypair,
        );
    }
}

pub fn copy_keypair_addr(out: *mut u32, inp: *const u32) {
    unsafe {
        std::ptr::copy_nonoverlapping(inp as *const u8, out as *mut u8, SPX_OFFSET_TREE + 8);
        std::ptr::copy_nonoverlapping(
            addr_bytes_const(inp).add(SPX_OFFSET_KP_ADDR),
            addr_bytes(out).add(SPX_OFFSET_KP_ADDR),
            4,
        );
    }
}

pub fn set_chain_addr(addr: *mut u32, chain: u32) {
    unsafe {
        *addr_bytes(addr).add(SPX_OFFSET_CHAIN_ADDR) = chain as u8;
    }
}

pub fn set_hash_addr(addr: *mut u32, hash: u32) {
    unsafe {
        *addr_bytes(addr).add(SPX_OFFSET_HASH_ADDR) = hash as u8;
    }
}

pub fn set_tree_height(addr: *mut u32, tree_height: u32) {
    unsafe {
        *addr_bytes(addr).add(SPX_OFFSET_TREE_HGT) = tree_height as u8;
    }
}

pub fn set_tree_index(addr: *mut u32, tree_index: u32) {
    unsafe {
        u32_to_bytes(
            std::slice::from_raw_parts_mut(addr_bytes(addr).add(SPX_OFFSET_TREE_INDEX), 4),
            tree_index,
        );
    }
}

// --- Exported C functions ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    set_layer_addr(addr, layer);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    set_tree_addr(addr, tree);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    set_type(addr, type_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    copy_subtree_addr(out, inp);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    set_keypair_addr(addr, keypair);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    copy_keypair_addr(out, inp);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    set_chain_addr(addr, chain);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    set_hash_addr(addr, hash);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    set_tree_height(addr, tree_height);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    set_tree_index(addr, tree_index);
}
