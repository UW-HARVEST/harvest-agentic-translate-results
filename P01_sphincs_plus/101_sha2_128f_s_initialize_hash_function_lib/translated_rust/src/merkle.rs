use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::wots::{LeafInfoX1, chain_lengths, wots_gen_leafx1};

pub fn merkle_sign(
    sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32,
) {
    let mut info = LeafInfoX1::default();
    let mut steps = [0u32; SPX_WOTS_LEN];

    info.wots_sig = sig.as_mut_ptr();
    chain_lengths(&mut steps, root);
    info.wots_steps = steps.as_mut_ptr();

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    info.wots_sign_leaf = idx_leaf;

    // auth_path starts at sig + SPX_WOTS_BYTES, but we pass the whole sig
    // and let wots_treehashx1 write into it. We need to split the borrow.
    let sig_ptr = sig.as_mut_ptr();
    let auth_path = unsafe {
        std::slice::from_raw_parts_mut(sig_ptr.add(SPX_WOTS_BYTES), SPX_TREE_HEIGHT * SPX_N)
    };

    crate::utilsx1::wots_treehashx1(
        root, auth_path, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info,
    );
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = [0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, SPX_D as u32 - 1);
    set_layer_addr(&mut wots_addr, SPX_D as u32 - 1);

    merkle_sign(
        &mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32,
    );
}
