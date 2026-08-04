use crate::params::*;
use crate::context::SpxCtx;
use crate::address::{set_type, copy_subtree_addr, set_layer_addr};
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;
use crate::utilsx1::wots_treehashx1;

pub fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx, wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let mut steps = vec![0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);

    let (wots_sig, auth_path) = sig.split_at_mut(SPX_WOTS_BYTES);
    let mut info = LeafInfoX1 {
        wots_sig: Some(wots_sig),
        wots_sign_leaf: idx_leaf,
        wots_steps: &steps,
        leaf_addr: [0; 8],
        pk_addr: [0; 8],
    };

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    wots_treehashx1(root, auth_path, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}
