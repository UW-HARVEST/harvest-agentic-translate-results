use crate::address::{copy_subtree_addr, set_layer_addr, set_type};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::wots_treehashx1;
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    unsafe {
        let auth_path = sig.add(SPX_WOTS_BYTES);
        let mut info: LeafInfoX1 = std::mem::zeroed();
        let mut steps = [0u32; SPX_WOTS_LEN];

        info.wots_sig = sig;
        chain_lengths(steps.as_mut_ptr(), root);
        info.wots_steps = steps.as_mut_ptr();

        set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
        set_type(info.pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTSPK);
        copy_subtree_addr(info.leaf_addr.as_mut_ptr(), wots_addr as *const u32);
        copy_subtree_addr(info.pk_addr.as_mut_ptr(), wots_addr as *const u32);

        info.wots_sign_leaf = idx_leaf;

        wots_treehashx1(
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
}

pub fn merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    SPX_merkle_sign(sig, root, ctx, wots_addr, tree_addr, idx_leaf);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(top_tree_addr.as_mut_ptr(), (SPX_D - 1) as u32);
    set_layer_addr(wots_addr.as_mut_ptr(), (SPX_D - 1) as u32);

    merkle_sign(
        auth_path.as_mut_ptr(),
        root,
        ctx,
        wots_addr.as_mut_ptr(),
        top_tree_addr.as_mut_ptr(),
        !0u32,
    );
}

pub fn merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    SPX_merkle_gen_root(root, ctx);
}
