use crate::params::*;

pub fn addr_bytes(addr: &[u32; 8]) -> [u8; 32] {
    let mut b = [0u8; 32];
    for i in 0..8 {
        let v = addr[i];
        b[4*i] = (v & 0xff) as u8;
        b[4*i+1] = ((v >> 8) & 0xff) as u8;
        b[4*i+2] = ((v >> 16) & 0xff) as u8;
        b[4*i+3] = ((v >> 24) & 0xff) as u8;
    }
    b
}

pub fn addr_from_bytes(b: &[u8; 32]) -> [u32; 8] {
    let mut addr = [0u32; 8];
    for i in 0..8 {
        addr[i] = b[4*i] as u32
            | ((b[4*i+1] as u32) << 8)
            | ((b[4*i+2] as u32) << 16)
            | ((b[4*i+3] as u32) << 24);
    }
    addr
}

pub fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    let b = addr_bytes(addr);
    let mut b2 = b;
    b2[SPX_OFFSET_LAYER] = layer as u8;
    *addr = addr_from_bytes(&b2);
}

pub fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let mut b = addr_bytes(addr);
    crate::utils::ull_to_bytes(&mut b[SPX_OFFSET_TREE..], 8, tree);
    *addr = addr_from_bytes(&b);
}

pub fn set_type(addr: &mut [u32; 8], type_val: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_TYPE] = type_val as u8;
    *addr = addr_from_bytes(&b);
}

pub fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let b_in = addr_bytes(inp);
    let mut b_out = addr_bytes(out);
    b_out[..SPX_OFFSET_TREE + 8].copy_from_slice(&b_in[..SPX_OFFSET_TREE + 8]);
    *out = addr_from_bytes(&b_out);
}

pub fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let mut b = addr_bytes(addr);
    crate::sha2::u32_to_bytes(&mut b[SPX_OFFSET_KP_ADDR..], keypair);
    *addr = addr_from_bytes(&b);
}

pub fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let b_in = addr_bytes(inp);
    let mut b_out = addr_bytes(out);
    b_out[..SPX_OFFSET_TREE + 8].copy_from_slice(&b_in[..SPX_OFFSET_TREE + 8]);
    b_out[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4].copy_from_slice(&b_in[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
    *out = addr_from_bytes(&b_out);
}

pub fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
    *addr = addr_from_bytes(&b);
}

pub fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_HASH_ADDR] = hash as u8;
    *addr = addr_from_bytes(&b);
}

pub fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_TREE_HGT] = tree_height as u8;
    *addr = addr_from_bytes(&b);
}

pub fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let mut b = addr_bytes(addr);
    crate::sha2::u32_to_bytes(&mut b[SPX_OFFSET_TREE_INDEX..], tree_index);
    *addr = addr_from_bytes(&b);
}
