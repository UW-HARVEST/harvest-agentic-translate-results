use crate::params::*;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(SPX_OFFSET_LAYER) = layer as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    unsafe {
        let bytes = addr as *mut u8;
        let mut input = tree;
        let outlen = 8usize;
        let mut i: isize = outlen as isize - 1;
        while i >= 0 {
            *bytes.add(SPX_OFFSET_TREE + i as usize) = (input & 0xff) as u8;
            input >>= 8;
            i -= 1;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, ty: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(SPX_OFFSET_TYPE) = ty as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, input: *const u32) {
    unsafe {
        let dst = out as *mut u8;
        let src = input as *const u8;
        std::ptr::copy_nonoverlapping(src, dst, SPX_OFFSET_TREE + 8);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(SPX_OFFSET_KP_ADDR) = (keypair >> 24) as u8;
        *bytes.add(SPX_OFFSET_KP_ADDR + 1) = (keypair >> 16) as u8;
        *bytes.add(SPX_OFFSET_KP_ADDR + 2) = (keypair >> 8) as u8;
        *bytes.add(SPX_OFFSET_KP_ADDR + 3) = keypair as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, input: *const u32) {
    unsafe {
        let dst = out as *mut u8;
        let src = input as *const u8;
        std::ptr::copy_nonoverlapping(src, dst, SPX_OFFSET_TREE + 8);
        std::ptr::copy_nonoverlapping(src.add(SPX_OFFSET_KP_ADDR), dst.add(SPX_OFFSET_KP_ADDR), 4);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(SPX_OFFSET_CHAIN_ADDR) = chain as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(SPX_OFFSET_HASH_ADDR) = hash as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(SPX_OFFSET_TREE_HGT) = tree_height as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    unsafe {
        let bytes = addr as *mut u8;
        *bytes.add(SPX_OFFSET_TREE_INDEX) = (tree_index >> 24) as u8;
        *bytes.add(SPX_OFFSET_TREE_INDEX + 1) = (tree_index >> 16) as u8;
        *bytes.add(SPX_OFFSET_TREE_INDEX + 2) = (tree_index >> 8) as u8;
        *bytes.add(SPX_OFFSET_TREE_INDEX + 3) = tree_index as u8;
    }
}

// Internal callable wrappers
pub fn set_layer_addr(addr: *mut u32, layer: u32) {
    SPX_set_layer_addr(addr, layer);
}
pub fn set_tree_addr(addr: *mut u32, tree: u64) {
    SPX_set_tree_addr(addr, tree);
}
pub fn set_type(addr: *mut u32, ty: u32) {
    SPX_set_type(addr, ty);
}
pub fn copy_subtree_addr(out: *mut u32, input: *const u32) {
    SPX_copy_subtree_addr(out, input);
}
pub fn set_keypair_addr(addr: *mut u32, keypair: u32) {
    SPX_set_keypair_addr(addr, keypair);
}
pub fn copy_keypair_addr(out: *mut u32, input: *const u32) {
    SPX_copy_keypair_addr(out, input);
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
