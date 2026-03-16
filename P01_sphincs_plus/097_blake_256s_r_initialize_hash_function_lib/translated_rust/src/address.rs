use crate::params::*;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (val & 0xff) as u8;
        val >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

pub fn bytes_to_ull(input: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// Address manipulation functions
pub fn set_layer_addr(addr: &mut [u8; SPX_ADDR_BYTES], layer: u32) {
    addr[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u8; SPX_ADDR_BYTES], tree: u64) {
    ull_to_bytes(&mut addr[SPX_OFFSET_TREE..], 8, tree);
}

pub fn set_type(addr: &mut [u8; SPX_ADDR_BYTES], type_val: u32) {
    addr[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut [u8; SPX_ADDR_BYTES], input: &[u8; SPX_ADDR_BYTES]) {
    out[..SPX_OFFSET_TREE + 8].copy_from_slice(&input[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut [u8; SPX_ADDR_BYTES], keypair: u32) {
    u32_to_bytes(&mut addr[SPX_OFFSET_KP_ADDR..], keypair);
}

pub fn copy_keypair_addr(out: &mut [u8; SPX_ADDR_BYTES], input: &[u8; SPX_ADDR_BYTES]) {
    out[..SPX_OFFSET_TREE + 8].copy_from_slice(&input[..SPX_OFFSET_TREE + 8]);
    out[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&input[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut [u8; SPX_ADDR_BYTES], chain: u32) {
    addr[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u8; SPX_ADDR_BYTES], hash: u32) {
    addr[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u8; SPX_ADDR_BYTES], tree_height: u32) {
    addr[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u8; SPX_ADDR_BYTES], tree_index: u32) {
    u32_to_bytes(&mut addr[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// Address type constants
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;
