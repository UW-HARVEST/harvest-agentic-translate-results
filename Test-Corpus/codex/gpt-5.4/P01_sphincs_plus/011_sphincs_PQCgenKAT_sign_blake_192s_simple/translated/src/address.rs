use crate::params::*;

pub(crate) fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

pub(crate) fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}

pub(crate) fn set_layer_addr_rs(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

pub(crate) fn set_tree_addr_rs(addr: &mut [u32; 8], tree: u64) {
    crate::utils::ull_to_bytes_into(&mut addr_bytes_mut(addr)[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8], tree);
}

pub(crate) fn set_type_rs(addr: &mut [u32; 8], ty: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = ty as u8;
}

pub(crate) fn copy_subtree_addr_rs(out: &mut [u32; 8], input: &[u32; 8]) {
    addr_bytes_mut(out)[..SPX_OFFSET_TREE + 8].copy_from_slice(&addr_bytes(input)[..SPX_OFFSET_TREE + 8]);
}

pub(crate) fn set_keypair_addr_rs(addr: &mut [u32; 8], keypair: u32) {
    crate::utils::u32_to_bytes_into(&mut addr_bytes_mut(addr)[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4], keypair);
}

pub(crate) fn copy_keypair_addr_rs(out: &mut [u32; 8], input: &[u32; 8]) {
    copy_subtree_addr_rs(out, input);
    addr_bytes_mut(out)[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&addr_bytes(input)[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub(crate) fn set_chain_addr_rs(addr: &mut [u32; 8], chain: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub(crate) fn set_hash_addr_rs(addr: &mut [u32; 8], hash: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub(crate) fn set_tree_height_rs(addr: &mut [u32; 8], tree_height: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub(crate) fn set_tree_index_rs(addr: &mut [u32; 8], tree_index: u32) {
    crate::utils::u32_to_bytes_into(
        &mut addr_bytes_mut(addr)[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4],
        tree_index,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    set_layer_addr_rs(unsafe { &mut *(addr as *mut [u32; 8]) }, layer);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    set_tree_addr_rs(unsafe { &mut *(addr as *mut [u32; 8]) }, tree);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, ty: u32) {
    set_type_rs(unsafe { &mut *(addr as *mut [u32; 8]) }, ty);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, input: *const u32) {
    copy_subtree_addr_rs(
        unsafe { &mut *(out as *mut [u32; 8]) },
        unsafe { &*(input as *const [u32; 8]) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    set_keypair_addr_rs(unsafe { &mut *(addr as *mut [u32; 8]) }, keypair);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    set_chain_addr_rs(unsafe { &mut *(addr as *mut [u32; 8]) }, chain);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    set_hash_addr_rs(unsafe { &mut *(addr as *mut [u32; 8]) }, hash);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, input: *const u32) {
    copy_keypair_addr_rs(
        unsafe { &mut *(out as *mut [u32; 8]) },
        unsafe { &*(input as *const [u32; 8]) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    set_tree_height_rs(unsafe { &mut *(addr as *mut [u32; 8]) }, tree_height);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    set_tree_index_rs(unsafe { &mut *(addr as *mut [u32; 8]) }, tree_index);
}
