use crate::address::{copy_subtree_addr_rs, set_layer_addr_rs, set_type_rs};
use crate::context::spx_ctx;
use crate::params::*;
use crate::utilsx1::wots_treehashx1_rs;
use crate::wots::chain_lengths_rs;
use crate::wotsx1::leaf_info_x1;

pub(crate) fn merkle_sign_rs(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &spx_ctx,
    wots_addr: &mut [u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let (wots_sig, auth_path) = sig.split_at_mut(SPX_WOTS_BYTES);
    let mut info = leaf_info_x1 {
        wots_sig: wots_sig.as_mut_ptr(),
        wots_sign_leaf: idx_leaf,
        wots_steps: std::ptr::null_mut(),
        leaf_addr: [0; 8],
        pk_addr: [0; 8],
    };
    let mut steps = vec![0u32; SPX_WOTS_LEN];
    chain_lengths_rs(&mut steps, root);
    info.wots_steps = steps.as_mut_ptr();
    set_type_rs(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type_rs(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr_rs(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr_rs(&mut info.pk_addr, wots_addr);
    wots_treehashx1_rs(root, auth_path, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info);
}

pub(crate) fn merkle_gen_root_rs(root: &mut [u8], ctx: &spx_ctx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];
    set_layer_addr_rs(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr_rs(&mut wots_addr, (SPX_D - 1) as u32);
    merkle_sign_rs(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, u32::MAX);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const spx_ctx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    merkle_sign_rs(
        unsafe { std::slice::from_raw_parts_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N) },
        unsafe { std::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe { &*ctx },
        unsafe { &mut *(wots_addr as *mut [u32; 8]) },
        unsafe { &mut *(tree_addr as *mut [u32; 8]) },
        idx_leaf,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const spx_ctx) {
    merkle_gen_root_rs(
        unsafe { std::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe { &*ctx },
    );
}
