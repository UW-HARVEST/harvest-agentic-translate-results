use crate::params::*;
use crate::address::*;
use crate::context::SpxCtx;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};
use crate::fors::ForsGenLeafInfo;

fn treehashx1_inner<F>(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8],
                       gen_leaf: F)
where F: Fn(&mut [u8], &SpxCtx, u32, &mut [u32; 8])
{
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        // Use a mutable copy of tree_addr for gen_leaf
        let mut ta_copy = *tree_addr;
        gen_leaf(&mut current[SPX_N..2 * SPX_N], ctx, idx + idx_offset, &mut ta_copy);
        *tree_addr = ta_copy;

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
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..2 * SPX_N], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let ho = h as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let info_ptr = info as *mut LeafInfoX1;
    treehashx1_inner(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        |dest, ctx, addr_idx, _ta| {
            unsafe { wots_gen_leafx1(dest, ctx, addr_idx, &mut *info_ptr); }
        });
}

pub fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let info_ptr = info as *mut LeafInfoX1;
    treehashx1_inner(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        |dest, ctx, addr_idx, _ta| {
            // Cast LeafInfoX1 pointer to ForsGenLeafInfo pointer
            // In C, fors_treehashx1 takes leaf_info_x1* but fors_gen_leafx1 takes fors_gen_leaf_info*
            // The fors_gen_leaf_info is just the leaf_addrx field at the start of leaf_info_x1
            unsafe {
                let fors_info = &mut *(info_ptr as *mut ForsGenLeafInfo);
                crate::fors::fors_gen_leafx1(dest, ctx, addr_idx, fors_info);
            }
        });
}
