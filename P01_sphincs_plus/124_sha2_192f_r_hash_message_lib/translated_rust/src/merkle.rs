use crate::params::*;
use crate::context::SpxCtx;

pub fn merkle_sign(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let auth_path = &mut sig[SPX_WOTS_BYTES..];
    let mut steps = [0u32; SPX_WOTS_LEN];
    crate::wots::chain_lengths(&mut steps, root);

    let mut info = crate::wotsx1::LeafInfoX1 {
        wots_sig: sig[..SPX_WOTS_BYTES].to_vec(),
        wots_sign_leaf: idx_leaf,
        wots_steps: steps.to_vec(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    crate::address::set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    crate::address::set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    crate::address::copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    crate::address::copy_subtree_addr(&mut info.pk_addr, wots_addr);

    let mut local_root = [0u8; SPX_N];
    let mut local_auth = vec![0u8; SPX_TREE_HEIGHT * SPX_N];

    crate::utilsx1::wots_treehashx1(
        &mut local_root,
        &mut local_auth,
        ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT as u32,
        tree_addr,
        &mut info,
    );

    root[..SPX_N].copy_from_slice(&local_root);
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
    sig[SPX_WOTS_BYTES..SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N]
        .copy_from_slice(&local_auth);
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
