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

// Address functions

pub fn addr_as_bytes(addr: &[u32; 8]) -> &[u8; SPX_ADDR_BYTES] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; SPX_ADDR_BYTES]) }
}

pub fn addr_as_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; SPX_ADDR_BYTES] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; SPX_ADDR_BYTES]) }
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_as_bytes_mut(addr);
    ull_to_bytes(&mut bytes[SPX_OFFSET_TREE..], 8, tree);
}

pub fn set_type(addr: &mut [u32; 8], type_val: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let out_bytes = addr_as_bytes_mut(out);
    let in_bytes = addr_as_bytes(inp);
    out_bytes[..SPX_OFFSET_TREE + 8].copy_from_slice(&in_bytes[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..], keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let out_bytes = addr_as_bytes_mut(out);
    let in_bytes = addr_as_bytes(inp);
    out_bytes[..SPX_OFFSET_TREE + 8].copy_from_slice(&in_bytes[..SPX_OFFSET_TREE + 8]);
    out_bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&in_bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    let bytes = addr_as_bytes_mut(addr);
    bytes[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// Address type constants
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;
