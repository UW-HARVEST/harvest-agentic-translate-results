use crate::params::*;

/*
 * Specify which level of Merkle tree (the "layer") we're working on
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let a = addr as *mut u8;
    *a.add(SPX_OFFSET_LAYER) = layer as u8;
}

/*
 * Specify which Merkle tree within the level (the "tree address") we're working on
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let a = addr as *mut u8;
    crate::utils::SPX_ull_to_bytes(a.add(SPX_OFFSET_TREE), 8, tree);
}

/*
 * Specify the reason we'll use this address structure for, that is, what
 * hash will we compute with it.  This is used so that unrelated types of
 * hashes don't accidentally get the same address structure.  The type will be
 * one of the SPX_ADDR_TYPE constants
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_type(addr: *mut u32, type_: u32) {
    let a = addr as *mut u8;
    *a.add(SPX_OFFSET_TYPE) = type_ as u8;
}

/*
 * Copy the layer and tree fields of the address structure.  This is used
 * when we're doing multiple types of hashes within the same Merkle tree
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, in_: *const u32) {
    core::ptr::copy_nonoverlapping(in_ as *const u8, out as *mut u8, SPX_OFFSET_TREE + 8);
}

/* These functions are used for OTS addresses. */

/*
 * Specify which Merkle leaf we're working on; that is, which OTS keypair
 * we're talking about.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let a = addr as *mut u8;
    crate::utils::SPX_u32_to_bytes(a.add(SPX_OFFSET_KP_ADDR), keypair);
}

/*
 * Copy the layer, tree and keypair fields of the address structure.  This is
 * used when we're doing multiple things within the same OTS keypair
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, in_: *const u32) {
    core::ptr::copy_nonoverlapping(in_ as *const u8, out as *mut u8, SPX_OFFSET_TREE + 8);
    core::ptr::copy_nonoverlapping(
        (in_ as *const u8).add(SPX_OFFSET_KP_ADDR),
        (out as *mut u8).add(SPX_OFFSET_KP_ADDR),
        4,
    );
}

/*
 * Specify which Merkle chain within the OTS we're working with
 * (the chain address)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let a = addr as *mut u8;
    *a.add(SPX_OFFSET_CHAIN_ADDR) = chain as u8;
}

/*
 * Specify where in the Merkle chain we are
 * (the hash address)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let a = addr as *mut u8;
    *a.add(SPX_OFFSET_HASH_ADDR) = hash as u8;
}

/* These functions are used for all hash tree addresses (including FORS). */

/*
 * Specify the height of the node in the Merkle/FORS tree we are in
 * (the tree height)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let a = addr as *mut u8;
    *a.add(SPX_OFFSET_TREE_HGT) = tree_height as u8;
}

/*
 * Specify the distance from the left edge of the node in the Merkle/FORS tree
 * (the tree index)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let a = addr as *mut u8;
    crate::utils::SPX_u32_to_bytes(a.add(SPX_OFFSET_TREE_INDEX), tree_index);
}
