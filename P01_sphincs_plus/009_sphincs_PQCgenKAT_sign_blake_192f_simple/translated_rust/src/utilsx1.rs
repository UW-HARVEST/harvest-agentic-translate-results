use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::hash::prf_addr;
use crate::thash::thash;
use crate::fors::ForsGenLeafInfo;
use crate::wotsx1::{LeafInfoX1, wots_gen_leafx1};

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    prf_addr(leaf, ctx, &info.leaf_addrx);

    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&leaf[..SPX_N]);
    thash(leaf, &tmp, 1, ctx, &info.leaf_addrx);
}

pub fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx: u32 = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = vec![0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
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
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save current to stack
        // h is the height where we broke out of the inner loop
        // We need to figure out h: it's the number of trailing 1-bits of idx (when idx < max_idx)
        // Actually, let's recompute h
        let mut h = 0u32;
        {
            let mut ti = idx;
            let mut tl = leaf_idx;
            loop {
                if h == tree_height { break; }
                if (ti ^ tl) == 0x01 { }
                if (ti & 1) == 0 && idx < max_idx { break; }
                ti >>= 1;
                tl >>= 1;
                h += 1;
            }
        }
        let off = h as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

pub fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx: u32 = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = vec![0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
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
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let mut h = 0u32;
        {
            let mut ti = idx;
            let mut tl = leaf_idx;
            loop {
                if h == tree_height { break; }
                if (ti & 1) == 0 && idx < max_idx { break; }
                ti >>= 1;
                tl >>= 1;
                h += 1;
            }
        }
        let off = h as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}
