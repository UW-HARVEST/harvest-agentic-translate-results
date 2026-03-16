use crate::params::*;
use crate::context::*;
use crate::thash::thash;
use crate::wotsx1::*;
use crate::fors::ForsGenLeafInfo;

/// Generic treehash used by both WOTS and FORS
fn treehashx1_inner(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8],
    gen_leaf: &mut dyn FnMut(&mut [u8], &SpxCtx, u32),
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    for idx in 0u32.. {
        let mut current = vec![0u8; 2 * SPX_N];
        gen_leaf(&mut current[SPX_N..], ctx, idx + idx_offset);
        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let off = h as usize * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);
            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);
            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        let save_off = h as usize * SPX_N;
        stack[save_off..save_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    // We need a mutable closure that captures info
    let info_ptr = info as *mut LeafInfoX1;
    treehashx1_inner(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest: &mut [u8], ctx: &SpxCtx, leaf_idx_inner: u32| {
            let info_ref = unsafe { &mut *info_ptr };
            wots_gen_leafx1(dest, ctx, leaf_idx_inner, info_ref);
        });
}

pub fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo) {
    let info_ptr = info as *mut ForsGenLeafInfo;
    treehashx1_inner(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest: &mut [u8], ctx: &SpxCtx, addr_idx: u32| {
            let info_ref = unsafe { &mut *info_ptr };
            crate::fors::fors_gen_leafx1(dest, ctx, addr_idx, info_ref);
        });
}
