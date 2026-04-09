use crate::params::*;
use crate::utils::{u32_to_bytes, ull_to_bytes};

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(OFFSET_LAYER) = layer as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    unsafe {
        let bytes = addr as *mut u8;
        ull_to_bytes(bytes.add(OFFSET_TREE), 8, tree);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(OFFSET_TYPE) = type_val as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    unsafe {
        std::ptr::copy_nonoverlapping(inp as *const u8, out as *mut u8, OFFSET_TREE + 8);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        u32_to_bytes(bytes.add(OFFSET_KP_ADDR), keypair);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    unsafe {
        std::ptr::copy_nonoverlapping(inp as *const u8, out as *mut u8, OFFSET_TREE + 8);
        std::ptr::copy_nonoverlapping(
            (inp as *const u8).add(OFFSET_KP_ADDR),
            (out as *mut u8).add(OFFSET_KP_ADDR),
            4,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(OFFSET_CHAIN_ADDR) = chain as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(OFFSET_HASH_ADDR) = hash as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(OFFSET_TREE_HGT) = tree_height as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        u32_to_bytes(bytes.add(OFFSET_TREE_INDEX), tree_index);
    }
}

// Internal Rust helpers that call the no_mangle versions
pub fn set_layer_addr(addr: *mut u32, layer: u32) {
    SPX_set_layer_addr(addr, layer);
}
pub fn set_tree_addr(addr: *mut u32, tree: u64) {
    SPX_set_tree_addr(addr, tree);
}
pub fn set_type(addr: *mut u32, type_val: u32) {
    SPX_set_type(addr, type_val);
}
pub fn copy_subtree_addr(out: *mut u32, inp: *const u32) {
    SPX_copy_subtree_addr(out, inp);
}
pub fn set_keypair_addr(addr: *mut u32, keypair: u32) {
    SPX_set_keypair_addr(addr, keypair);
}
pub fn copy_keypair_addr(out: *mut u32, inp: *const u32) {
    SPX_copy_keypair_addr(out, inp);
}
pub fn set_chain_addr(addr: *mut u32, chain: u32) {
    SPX_set_chain_addr(addr, chain);
}
pub fn set_hash_addr(addr: *mut u32, hash: u32) {
    SPX_set_hash_addr(addr, hash);
}
pub fn set_tree_height(addr: *mut u32, tree_height: u32) {
    SPX_set_tree_height(addr, tree_height);
}
pub fn set_tree_index(addr: *mut u32, tree_index: u32) {
    SPX_set_tree_index(addr, tree_index);
}
