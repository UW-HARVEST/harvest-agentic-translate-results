use crate::params::*;
use crate::context::SpxCtx;
use crate::wotsx1::LeafInfoX1;

pub fn merkle_sign(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let mut info = LeafInfoX1::default();
    let mut steps = [0u32; SPX_WOTS_LEN];

    info.wots_sig = sig.as_mut_ptr();
    crate::wots::chain_lengths(&mut steps, root);
    info.wots_steps = steps.as_ptr();

    crate::address::set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    crate::address::set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    crate::address::copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    crate::address::copy_subtree_addr(&mut info.pk_addr, wots_addr);

    info.wots_sign_leaf = idx_leaf;

    // We need to pass both the wots_sig part and auth_path part of sig
    // but they don't overlap, so we use raw pointer arithmetic
    let auth_path_ptr = unsafe { sig.as_mut_ptr().add(SPX_WOTS_BYTES) };
    let auth_path_len = sig.len() - SPX_WOTS_BYTES;
    let auth_path = unsafe { std::slice::from_raw_parts_mut(auth_path_ptr, auth_path_len) };

    crate::utilsx1::wots_treehashx1(
        root,
        auth_path,
        ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT as u32,
        tree_addr,
        &mut info,
    );
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    crate::address::set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    crate::address::set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(
        &mut auth_path,
        root,
        ctx,
        &mut wots_addr,
        &mut top_tree_addr,
        !0u32,
    );
}
