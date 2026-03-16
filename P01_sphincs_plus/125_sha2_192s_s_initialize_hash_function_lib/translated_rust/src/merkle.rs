use crate::context::{SpxCtx, LeafInfoX1};
use crate::params::*;
use crate::utils::*;

pub fn merkle_sign_internal(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let sig_ptr = sig.as_mut_ptr();
    let mut info = LeafInfoX1 {
        wots_sig: sig_ptr,
        wots_sign_leaf: idx_leaf,
        wots_steps: core::ptr::null_mut(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    let mut steps = [0u32; SPX_WOTS_LEN];
    crate::wots::chain_lengths_internal(&mut steps, root);
    info.wots_steps = steps.as_mut_ptr();

    set_type_internal(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type_internal(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr_internal(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr_internal(&mut info.pk_addr, wots_addr);

    let auth_path_ptr = unsafe { sig_ptr.add(SPX_WOTS_BYTES) };
    let auth_path = unsafe { core::slice::from_raw_parts_mut(auth_path_ptr, SPX_TREE_HEIGHT * SPX_N) };

    crate::utilsx1::wots_treehashx1_internal(
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

pub fn merkle_gen_root_internal(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr_internal(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr_internal(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign_internal(
        &mut auth_path,
        root,
        ctx,
        &mut wots_addr,
        &mut top_tree_addr,
        !0u32,
    );
}
