// Translation of c_src/app/src/address.c

use crate::params::{
    SPX_OFFSET_CHAIN_ADDR, SPX_OFFSET_HASH_ADDR, SPX_OFFSET_KP_ADDR, SPX_OFFSET_LAYER,
    SPX_OFFSET_TREE, SPX_OFFSET_TREE_HGT, SPX_OFFSET_TREE_INDEX, SPX_OFFSET_TYPE,
};
use crate::utils::{u32_to_bytes, ull_to_bytes};

/// Reinterpret the [u32; 8] address as a mutable byte slice (little/native order
/// — matches the way the C code accesses the bytes via a `(unsigned char *)` cast).
fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32) }
}

fn addr_bytes(addr: &[u32; 8]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) }
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    let bytes = addr_bytes_mut(addr);
    bytes[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_bytes_mut(addr);
    ull_to_bytes(&mut bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8], 8, tree);
}

pub fn set_type(addr: &mut [u32; 8], typ: u32) {
    let bytes = addr_bytes_mut(addr);
    bytes[SPX_OFFSET_TYPE] = typ as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    // Copies SPX_OFFSET_TREE + 8 bytes (matches `memcpy(out, in, SPX_OFFSET_TREE+8)`)
    let n = SPX_OFFSET_TREE + 8;
    let in_bytes = addr_bytes(inp);
    let out_bytes = addr_bytes_mut(out);
    out_bytes[..n].copy_from_slice(&in_bytes[..n]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4], keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let n = SPX_OFFSET_TREE + 8;
    let in_bytes_full = addr_bytes(inp);
    // First copy the layer/tree portion. This is a 0..n copy from `in` into `out`.
    let in_first = {
        let mut tmp = [0u8; 32];
        tmp[..n].copy_from_slice(&in_bytes_full[..n]);
        tmp
    };
    let in_kp = {
        let mut tmp = [0u8; 4];
        tmp.copy_from_slice(&in_bytes_full[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
        tmp
    };
    let out_bytes = addr_bytes_mut(out);
    out_bytes[..n].copy_from_slice(&in_first[..n]);
    out_bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4].copy_from_slice(&in_kp);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    let bytes = addr_bytes_mut(addr);
    bytes[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    let bytes = addr_bytes_mut(addr);
    bytes[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    let bytes = addr_bytes_mut(addr);
    bytes[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes = addr_bytes_mut(addr);
    u32_to_bytes(
        &mut bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4],
        tree_index,
    );
}
