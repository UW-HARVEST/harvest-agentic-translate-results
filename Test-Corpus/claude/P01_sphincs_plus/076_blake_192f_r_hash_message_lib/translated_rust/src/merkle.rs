use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::{wots_treehashx1_rs};
use crate::wots::chain_lengths_rs;
use crate::wotsx1::LeafInfoX1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut [u32; 8],
    tree_addr: *mut [u32; 8],
    idx_leaf: u32,
) {
    let root_slice = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path = unsafe {
        core::slice::from_raw_parts_mut(sig.add(SPX_WOTS_BYTES), SPX_TREE_HEIGHT * SPX_N)
    };
    let ctx_ref = unsafe { &*ctx };
    let wots_addr_ref = unsafe { &mut *wots_addr };
    let tree_addr_ref = unsafe { &mut *tree_addr };

    let mut steps = vec![0u32; SPX_WOTS_LEN];
    chain_lengths_rs(&mut steps, root_slice);

    let mut info = LeafInfoX1 {
        wots_sig: sig,
        wots_sign_leaf: idx_leaf,
        wots_steps: steps.as_mut_ptr(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    set_type(tree_addr_ref, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr_ref);
    copy_subtree_addr(&mut info.pk_addr, wots_addr_ref);

    wots_treehashx1_rs(
        root_slice,
        auth_path,
        ctx_ref,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT as u32,
        tree_addr_ref,
        &mut info,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let auth_path_len = SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES;
    let mut auth_path = vec![0u8; auth_path_len];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr_arr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr_arr, (SPX_D - 1) as u32);

    SPX_merkle_sign(
        auth_path.as_mut_ptr(),
        root,
        ctx,
        &mut wots_addr_arr as *mut _,
        &mut top_tree_addr as *mut _,
        !0u32,
    );
}
