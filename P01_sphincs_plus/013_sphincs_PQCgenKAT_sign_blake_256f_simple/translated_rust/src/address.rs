use crate::params::*;
use crate::utils::{ull_to_bytes, u32_to_bytes};

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    ull_to_bytes(&mut bytes[SPX_OFFSET_TREE..], 8, tree);
}

pub fn set_type(addr: &mut [u32; 8], type_val: u32) {
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let out_bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, 32)
    };
    let in_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(inp.as_ptr() as *const u8, 32)
    };
    out_bytes[..SPX_OFFSET_TREE + 8].copy_from_slice(&in_bytes[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..], keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let out_bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, 32)
    };
    let in_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(inp.as_ptr() as *const u8, 32)
    };
    out_bytes[..SPX_OFFSET_TREE + 8].copy_from_slice(&in_bytes[..SPX_OFFSET_TREE + 8]);
    out_bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&in_bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    u32_to_bytes(&mut bytes[SPX_OFFSET_TREE_INDEX..], tree_index);
}
