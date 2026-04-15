use crate::params::*;
use crate::context::SpxCtx;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};
use crate::thash::thash;
use crate::address::{set_tree_height, set_tree_index};

pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, idx_offset: u32, tree_height: u32, tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let mut stack = vec![0u8; (tree_height as usize) * SPX_N];
    let max_idx = (1 << tree_height) - 1;

    for idx in 0.. {
        let mut current = vec![0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0;

        loop {
            if h == tree_height {
                root.copy_from_slice(&current[SPX_N..]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 1 {
                auth_path[(h as usize) * SPX_N..(h as usize + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left = &stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N];
            current[..SPX_N].copy_from_slice(left);
            let mut temp = vec![0u8; SPX_N];
            thash(&mut temp, &current, 2, ctx, tree_addr);
            current[SPX_N..].copy_from_slice(&temp);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
    }
}

pub fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, idx_offset: u32, tree_height: u32, tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo) {
    let mut stack = vec![0u8; (tree_height as usize) * SPX_N];
    let max_idx = (1 << tree_height) - 1;

    for idx in 0.. {
        let mut current = vec![0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0;

        loop {
            if h == tree_height {
                root.copy_from_slice(&current[SPX_N..]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 1 {
                auth_path[(h as usize) * SPX_N..(h as usize + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left = &stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N];
            current[..SPX_N].copy_from_slice(left);
            let mut temp = vec![0u8; SPX_N];
            thash(&mut temp, &current, 2, ctx, tree_addr);
            current[SPX_N..].copy_from_slice(&temp);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
    }
}
