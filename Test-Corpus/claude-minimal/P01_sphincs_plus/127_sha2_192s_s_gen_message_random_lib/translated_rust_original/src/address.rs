use crate::params::{
    SPX_OFFSET_CHAIN_ADDR, SPX_OFFSET_HASH_ADDR, SPX_OFFSET_KP_ADDR, SPX_OFFSET_LAYER,
    SPX_OFFSET_TREE, SPX_OFFSET_TREE_HGT, SPX_OFFSET_TREE_INDEX, SPX_OFFSET_TYPE,
};
use crate::utils::{u32_to_bytes, ull_to_bytes};

// addr is &mut [u32; 8] => view as 32 bytes (little-endian native, matching C code's pointer cast)
fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}

fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_bytes_mut(addr);
    let slice = &mut bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8];
    ull_to_bytes(slice, 8, tree);
}

pub fn set_type(addr: &mut [u32; 8], type_val: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], in_addr: &[u32; 8]) {
    let in_bytes = addr_bytes(in_addr);
    let out_bytes = addr_bytes_mut(out);
    let n = SPX_OFFSET_TREE + 8;
    out_bytes[..n].copy_from_slice(&in_bytes[..n]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_bytes_mut(addr);
    let slice = &mut bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4];
    u32_to_bytes(slice, keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], in_addr: &[u32; 8]) {
    let in_bytes_arr = *addr_bytes(in_addr);
    let out_bytes = addr_bytes_mut(out);
    let n = SPX_OFFSET_TREE + 8;
    out_bytes[..n].copy_from_slice(&in_bytes_arr[..n]);
    out_bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&in_bytes_arr[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
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
