use crate::params::*;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut v: u64) {
    for i in (0..outlen).rev() {
        out[i] = (v & 0xff) as u8;
        v >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], v: u32) {
    out[0] = (v >> 24) as u8;
    out[1] = (v >> 16) as u8;
    out[2] = (v >> 8) as u8;
    out[3] = v as u8;
}

pub fn bytes_to_ull(input: &[u8], inlen: usize) -> u64 {
    let mut retval = 0u64;
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// addr is stored as [u32; 8] but accessed as bytes
pub fn addr_as_bytes(addr: &[u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..8 {
        let b = addr[i].to_le_bytes();
        out[4*i..4*i+4].copy_from_slice(&b);
    }
    out
}

pub fn addr_as_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    // Safety: [u32; 8] and [u8; 32] have same size and alignment requirements are met
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
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

pub fn copy_subtree_addr(out: &mut [u32; 8], input: &[u32; 8]) {
    let ob = addr_as_bytes_mut(out);
    let ib = addr_as_bytes(input);
    ob[..SPX_OFFSET_TREE+8].copy_from_slice(&ib[..SPX_OFFSET_TREE+8]);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..], keypair);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], input: &[u32; 8]) {
    let ob = addr_as_bytes_mut(out);
    let ib = addr_as_bytes(input);
    ob[..SPX_OFFSET_TREE+8].copy_from_slice(&ib[..SPX_OFFSET_TREE+8]);
    ob[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR+4].copy_from_slice(&ib[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR+4]);
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
