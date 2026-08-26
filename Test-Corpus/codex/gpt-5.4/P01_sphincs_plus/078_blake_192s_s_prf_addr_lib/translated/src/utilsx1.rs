use crate::address::{set_tree_height_rs, set_tree_index_rs};
use crate::context::spx_ctx;
use crate::fors::{fors_gen_leafx1_rs, fors_gen_leaf_info};
use crate::params::*;
use crate::sha2_backend::SPX_thash_rs;
use crate::wotsx1::{leaf_info_x1, wots_gen_leafx1_rs};

pub(crate) fn wots_treehashx1_rs(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &spx_ctx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut leaf_info_x1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    let mut idx = 0u32;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        wots_gen_leafx1_rs(&mut current[SPX_N..], ctx, idx + idx_offset, info);
        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0usize;
        loop {
            if h == tree_height as usize {
                root.copy_from_slice(&current[SPX_N..]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height_rs(tree_addr, (h + 1) as u32);
            set_tree_index_rs(tree_addr, internal_idx / 2 + internal_idx_offset);
            current[..SPX_N].copy_from_slice(&stack[h * SPX_N..(h + 1) * SPX_N]);
            let input = current[..2 * SPX_N].to_vec();
            SPX_thash_rs(&mut current[SPX_N..], &input, 2, ctx, tree_addr);
            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        stack[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
        idx += 1;
    }
}

pub(crate) fn fors_treehashx1_rs(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &spx_ctx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut fors_gen_leaf_info,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    let mut idx = 0u32;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        fors_gen_leafx1_rs(&mut current[SPX_N..], ctx, idx + idx_offset, info);
        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0usize;
        loop {
            if h == tree_height as usize {
                root.copy_from_slice(&current[SPX_N..]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height_rs(tree_addr, (h + 1) as u32);
            set_tree_index_rs(tree_addr, internal_idx / 2 + internal_idx_offset);
            current[..SPX_N].copy_from_slice(&stack[h * SPX_N..(h + 1) * SPX_N]);
            let input = current[..2 * SPX_N].to_vec();
            SPX_thash_rs(&mut current[SPX_N..], &input, 2, ctx, tree_addr);
            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        stack[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
        idx += 1;
    }
}
