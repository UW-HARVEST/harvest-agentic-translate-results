use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::wots_treehashx1;
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;

pub fn merkle_sign(
    sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &[u8; 32], tree_addr: &mut [u8; 32], idx_leaf: u32,
) {
    let auth_path_offset = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1::new();
    let mut steps = [0u32; SPX_WOTS_LEN];

    info.wots_sig = sig[..SPX_WOTS_BYTES].to_vec();
    // We need wots_sig to be a mutable reference into sig. We'll copy back after.
    // Actually, let's use a separate buffer and copy back.
    info.wots_sig = vec![0u8; SPX_WOTS_BYTES];
    chain_lengths(&mut steps, root);
    info.wots_steps = steps;

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    info.wots_sign_leaf = idx_leaf;

    wots_treehashx1(
        root,
        &mut sig[auth_path_offset..],
        ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32,
        tree_addr, &mut info,
    );

    // Copy wots_sig back into sig
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u8; 32];
    let mut wots_addr = [0u8; 32];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(
        &mut auth_path, root, ctx,
        &wots_addr, &mut top_tree_addr, !0u32,
    );
}
