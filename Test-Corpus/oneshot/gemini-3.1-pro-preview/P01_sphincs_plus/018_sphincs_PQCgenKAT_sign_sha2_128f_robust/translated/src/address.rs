use crate::params::*;

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    unsafe { *(addr.as_mut_ptr() as *mut u8).add(SPX_OFFSET_LAYER) = layer as u8; }
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let mut bytes = [0u8; 8];
    crate::utils::ull_to_bytes(&mut bytes, 8, tree);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), (addr.as_mut_ptr() as *mut u8).add(SPX_OFFSET_TREE), 8);
    }
}

pub fn set_type(addr: &mut [u32; 8], type_: u32) {
    unsafe { *(addr.as_mut_ptr() as *mut u8).add(SPX_OFFSET_TYPE) = type_ as u8; }
}

pub fn copy_subtree_addr(out: &mut [u32; 8], in_: &[u32; 8]) {
    unsafe {
        std::ptr::copy_nonoverlapping(in_.as_ptr() as *const u8, out.as_mut_ptr() as *mut u8, SPX_OFFSET_TREE + 8);
    }
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let mut bytes = [0u8; 4];
    crate::utils::u32_to_bytes(&mut bytes, keypair);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), (addr.as_mut_ptr() as *mut u8).add(SPX_OFFSET_KP_ADDR), 4);
    }
}

pub fn copy_keypair_addr(out: &mut [u32; 8], in_: &[u32; 8]) {
    unsafe {
        std::ptr::copy_nonoverlapping(in_.as_ptr() as *const u8, out.as_mut_ptr() as *mut u8, SPX_OFFSET_TREE + 8);
        std::ptr::copy_nonoverlapping((in_.as_ptr() as *const u8).add(SPX_OFFSET_KP_ADDR), (out.as_mut_ptr() as *mut u8).add(SPX_OFFSET_KP_ADDR), 4);
    }
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    unsafe { *(addr.as_mut_ptr() as *mut u8).add(SPX_OFFSET_CHAIN_ADDR) = chain as u8; }
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    unsafe { *(addr.as_mut_ptr() as *mut u8).add(SPX_OFFSET_HASH_ADDR) = hash as u8; }
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    unsafe { *(addr.as_mut_ptr() as *mut u8).add(SPX_OFFSET_TREE_HGT) = tree_height as u8; }
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let mut bytes = [0u8; 4];
    crate::utils::u32_to_bytes(&mut bytes, tree_index);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), (addr.as_mut_ptr() as *mut u8).add(SPX_OFFSET_TREE_INDEX), 4);
    }
}
