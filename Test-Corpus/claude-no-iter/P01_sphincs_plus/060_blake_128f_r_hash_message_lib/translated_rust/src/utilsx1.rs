// Translation of c_src/app/src/utilsx1.c

use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};
use crate::params::SPX_N;
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
) {
    let stack_len = tree_height as usize * SPX_N;
    let mut stack = vec![0u8; stack_len];

    let mut idx: u32 = 0;
    let max_idx: u32 = (1u32 << tree_height) - 1;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

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
                auth_path[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            // current[..SPX_N] = stack[h*SPX_N..(h+1)*SPX_N]
            current[..SPX_N].copy_from_slice(&stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]);
            let in_block = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..2 * SPX_N], &in_block, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // we're at a left child; save current for when we get the right one.
        stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
        idx += 1;
    }
}

pub fn fors_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut ForsGenLeafInfo,
) {
    let stack_len = tree_height as usize * SPX_N;
    let mut stack = vec![0u8; stack_len];

    let mut idx: u32 = 0;
    let max_idx: u32 = (1u32 << tree_height) - 1;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

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
                auth_path[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]);
            let in_block = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..2 * SPX_N], &in_block, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
        idx += 1;
    }
}
