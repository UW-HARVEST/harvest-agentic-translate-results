// utilsx1 - tree hash routines

use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::fors::ForsGenLeafInfo;
use crate::params::*;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};

pub fn wots_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut LeafInfoX1,
    wots_sig: &mut [u8],
) {
    let mut stack = vec![0u8; (tree_height as usize) * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        wots_gen_leafx1(
            &mut current[SPX_N..],
            ctx,
            idx + idx_offset,
            info,
            Some(wots_sig),
        );

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let h_off = (h as usize) * SPX_N;
                auth_path[h_off..h_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let h_off = (h as usize) * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[h_off..h_off + SPX_N]);
            let in_data = current.clone();
            thash(&mut current[SPX_N..2 * SPX_N], &in_data, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
            h += 1;
        }
        let h_off = (h as usize) * SPX_N;
        stack[h_off..h_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
        idx += 1;
    }
}

pub fn fors_treehashx1<F>(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut ForsGenLeafInfo,
    mut gen_leaf: F,
) where
    F: FnMut(&mut [u8], &SpxCtx, u32, &mut ForsGenLeafInfo),
{
    let mut stack = vec![0u8; (tree_height as usize) * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        gen_leaf(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let h_off = (h as usize) * SPX_N;
                auth_path[h_off..h_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let h_off = (h as usize) * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[h_off..h_off + SPX_N]);
            let in_data = current.clone();
            thash(&mut current[SPX_N..2 * SPX_N], &in_data, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
            h += 1;
        }
        let h_off = (h as usize) * SPX_N;
        stack[h_off..h_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
        idx += 1;
    }
}
