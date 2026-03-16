use crate::address::*;
use crate::context::*;
use crate::params::*;
use crate::utilsx1::wots_treehashx1;
use crate::wots::chain_lengths;

pub fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
                   wots_addr: &[u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let mut info = LeafInfoX1 {
        wots_sig: sig.as_mut_ptr(),
        wots_sign_leaf: idx_leaf,
        wots_steps: std::ptr::null_mut(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);
    info.wots_steps = steps.as_mut_ptr();

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    // Split sig into wots_sig part and auth_path part to avoid double borrow
    let sig_ptr = sig.as_mut_ptr();
    let auth_path = unsafe { std::slice::from_raw_parts_mut(sig_ptr.add(SPX_WOTS_BYTES), SPX_TREE_HEIGHT * SPX_N) };

    wots_treehashx1(root, auth_path, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32,
                    tree_addr, &mut info);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(&mut auth_path, root, ctx, &wots_addr, &mut top_tree_addr, !0u32);
}
