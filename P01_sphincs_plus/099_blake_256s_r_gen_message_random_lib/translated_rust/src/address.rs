use crate::params::*;
use crate::context::addr_as_bytes_mut;
use crate::utils::{ull_to_bytes, u32_to_bytes};

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_as_bytes_mut(addr);
    let mut buf = [0u8; 8];
    ull_to_bytes(&mut buf, 8, tree);
    bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8].copy_from_slice(&buf);
}

pub fn set_type(addr: &mut [u32; 8], type_val: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = crate::context::addr_as_bytes(inp);
    let dst = addr_as_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_as_bytes_mut(addr);
    let mut buf = [0u8; 4];
    u32_to_bytes(&mut buf, keypair);
    bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4].copy_from_slice(&buf);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = crate::context::addr_as_bytes(inp);
    let dst = addr_as_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes = addr_as_bytes_mut(addr);
    let mut buf = [0u8; 4];
    u32_to_bytes(&mut buf, tree_index);
    bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4].copy_from_slice(&buf);
}
