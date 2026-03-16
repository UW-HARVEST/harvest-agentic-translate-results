use crate::params::*;

pub type Addr = [u8; 32];

pub fn addr_zero() -> Addr { [0u8; 32] }

pub fn set_layer_addr(addr: &mut Addr, layer: u32) {
    addr[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut Addr, tree: u64) {
    crate::utils::ull_to_bytes(&mut addr[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8], 8, tree);
}

pub fn set_type(addr: &mut Addr, type_val: u32) {
    addr[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut Addr, inp: &Addr) {
    out[..SPX_OFFSET_TREE + 8].copy_from_slice(&inp[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut Addr, keypair: u32) {
    crate::utils::u32_to_bytes(&mut addr[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4], keypair);
}

pub fn copy_keypair_addr(out: &mut Addr, inp: &Addr) {
    out[..SPX_OFFSET_TREE + 8].copy_from_slice(&inp[..SPX_OFFSET_TREE + 8]);
    out[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&inp[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut Addr, chain: u32) {
    addr[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut Addr, hash: u32) {
    addr[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut Addr, tree_height: u32) {
    addr[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut Addr, tree_index: u32) {
    crate::utils::u32_to_bytes(&mut addr[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4], tree_index);
}
