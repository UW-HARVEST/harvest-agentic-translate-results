use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::wots::*;

pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8],
    wots_sig: &mut [u8], wots_sign_leaf: u32, wots_steps: &[u32],
    leaf_addr: &mut [u32; 8], pk_addr: &mut [u32; 8]) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset,
            wots_sig, wots_sign_leaf, wots_steps, leaf_addr, pk_addr);
        let mut iio = idx_offset;
        let mut ii = idx;
        let mut il = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
                return;
            }
            if (ii ^ il) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
            }
            if (ii & 1) == 0 && idx < max_idx { break; }
            iio >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, ii / 2 + iio);
            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);
            ii >>= 1; il >>= 1; h += 1;
        }
        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
    }
}

pub fn wots_treehashx1_inplace(root: &mut [u8], sig: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8],
    wots_sign_leaf: u32, wots_steps: &[u32],
    leaf_addr: &mut [u32; 8], pk_addr: &mut [u32; 8]) {
    // sig[..SPX_WOTS_BYTES] = wots_sig, sig[SPX_WOTS_BYTES..] = auth_path
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset,
            &mut sig[..SPX_WOTS_BYTES], wots_sign_leaf, wots_steps, leaf_addr, pk_addr);
        let mut iio = idx_offset;
        let mut ii = idx;
        let mut il = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
                return;
            }
            if (ii ^ il) == 0x01 {
                let ap_off = SPX_WOTS_BYTES + h as usize * SPX_N;
                sig[ap_off..ap_off + SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
            }
            if (ii & 1) == 0 && idx < max_idx { break; }
            iio >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, ii / 2 + iio);
            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);
            ii >>= 1; il >>= 1; h += 1;
        }
        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
    }
}

pub fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    let mut pk_addr = [0u32; 8];
    set_type(&mut pk_addr, SPX_ADDR_TYPE_WOTSPK);
    let mut leaf_addr = [0u32; 8];
    copy_subtree_addr(&mut leaf_addr, wots_addr);
    copy_subtree_addr(&mut pk_addr, wots_addr);

    // We need to pass both sig (for wots_sig) and sig[SPX_WOTS_BYTES..] (for auth_path)
    // but they overlap. The treehash writes to different parts, so we pass the whole sig.
    wots_treehashx1_inplace(root, sig, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32,
        tree_addr, idx_leaf, &steps, &mut leaf_addr, &mut pk_addr);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];
    set_layer_addr(&mut top_tree_addr, SPX_D as u32 - 1);
    set_layer_addr(&mut wots_addr, SPX_D as u32 - 1);
    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}
