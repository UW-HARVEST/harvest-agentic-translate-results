use crate::context::SpxCtx;
use crate::params::*;
use crate::address::*;
use crate::wots::chain_lengths;
use crate::utilsx1::wots_treehashx1;

pub fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
                   wots_addr: &mut [u8; 32], tree_addr: &mut [u8; 32], idx_leaf: u32) {
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    let mut pk_addr = [0u8; 32];
    set_type(&mut pk_addr, SPX_ADDR_TYPE_WOTSPK);
    let mut leaf_addr = [0u8; 32];
    copy_subtree_addr(&mut leaf_addr, wots_addr);
    copy_subtree_addr(&mut pk_addr, wots_addr);

    // Split sig into wots_sig part and auth_path part to avoid double borrow
    let (wots_sig_part, auth_path_part) = sig.split_at_mut(SPX_WOTS_BYTES);

    wots_treehashx1(root, auth_path_part, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32,
                    tree_addr, wots_sig_part, idx_leaf, &steps, &mut leaf_addr, &mut pk_addr);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u8; 32];
    let mut wots_addr = [0u8; 32];
    set_layer_addr(&mut top_tree_addr, SPX_D as u32 - 1);
    set_layer_addr(&mut wots_addr, SPX_D as u32 - 1);
    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}
