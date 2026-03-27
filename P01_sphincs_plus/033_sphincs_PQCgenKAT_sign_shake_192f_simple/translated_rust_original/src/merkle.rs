use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utilsx1::wots_treehashx1;
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    let auth_path = sig.add(SPX_WOTS_BYTES);
    let mut info: LeafInfoX1 = core::mem::zeroed();
    let mut steps = [0u32; SPX_WOTS_LEN];

    info.wots_sig = sig;
    let root_slice = core::slice::from_raw_parts(root, SPX_N);
    chain_lengths(&mut steps, root_slice);
    info.wots_steps = steps.as_mut_ptr();

    let tree_addr_ref = &mut *(tree_addr as *mut [u32; 8]);
    set_type(tree_addr_ref, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let wots_addr_ref = &*(wots_addr as *const [u32; 8]);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr_ref);
    copy_subtree_addr(&mut info.pk_addr, wots_addr_ref);

    info.wots_sign_leaf = idx_leaf;

    wots_treehashx1(
        root,
        auth_path,
        &*ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT as u32,
        tree_addr_ref,
        &mut info,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    SPX_merkle_sign(
        auth_path.as_mut_ptr(),
        root,
        ctx,
        wots_addr.as_mut_ptr(),
        top_tree_addr.as_mut_ptr(),
        !0u32,
    );
}
