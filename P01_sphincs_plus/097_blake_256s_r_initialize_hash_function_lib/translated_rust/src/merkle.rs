use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::wots_treehashx1;
use crate::params::*;
use crate::wots::chain_lengths;
use crate::wotsx1::LeafInfoX1;

pub fn merkle_sign(
    sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &[u8; SPX_ADDR_BYTES], tree_addr: &mut [u8; SPX_ADDR_BYTES],
    idx_leaf: u32,
) {
    let auth_path = SPX_WOTS_BYTES;
    let mut steps = [0u32; SPX_WOTS_LEN];

    let mut info = LeafInfoX1 {
        wots_sig: sig[..SPX_WOTS_BYTES].to_vec(),
        wots_sign_leaf: idx_leaf,
        wots_steps: vec![0u32; SPX_WOTS_LEN],
        leaf_addr: [0u8; SPX_ADDR_BYTES],
        pk_addr: [0u8; SPX_ADDR_BYTES],
    };

    chain_lengths(&mut steps, root);
    info.wots_steps = steps.to_vec();

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    wots_treehashx1(
        root,
        &mut sig[auth_path..],
        ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT as u32,
        tree_addr,
        &mut info,
    );

    // Copy back the wots_sig into sig
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u8; SPX_ADDR_BYTES];
    let mut wots_addr = [0u8; SPX_ADDR_BYTES];

    set_layer_addr(&mut top_tree_addr, SPX_D as u32 - 1);
    set_layer_addr(&mut wots_addr, SPX_D as u32 - 1);

    merkle_sign(
        &mut auth_path, root, ctx,
        &wots_addr, &mut top_tree_addr,
        !0u32,
    );
}
