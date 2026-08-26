// Translation of c_src/app/src/utilsx1.c

use core::slice;

use crate::address::{set_tree_height_inner, set_tree_index_inner};
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::thash::thash_inner;
use crate::wotsx1::{wots_gen_leafx1_inner, LeafInfoX1};
use crate::fors::{fors_gen_leafx1_inner, ForsGenLeafInfo};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut LeafInfoX1,
) {
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path =
        unsafe { slice::from_raw_parts_mut(auth_path, (tree_height as usize) * SPX_N) };
    let ctx = unsafe { &*ctx };
    let tree_addr = unsafe { slice::from_raw_parts_mut(tree_addr, 8) };
    let info = unsafe { &mut *info };
    wots_treehashx1_inner(root, auth_path, ctx, leaf_idx, idx_offset, tree_height as usize, tree_addr, info);
}

pub fn wots_treehashx1_inner(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: usize,
    tree_addr: &mut [u32],
    info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        wots_gen_leafx1_inner(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h as usize == tree_height {
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
            set_tree_height_inner(tree_addr, h + 1);
            set_tree_index_inner(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]);
            // thash on current[0..2N], output to current[N..2N]
            let input_copy = current[..2 * SPX_N].to_vec();
            thash_inner(
                &mut current[SPX_N..2 * SPX_N],
                &input_copy,
                2,
                ctx,
                tree_addr,
            );

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut ForsGenLeafInfo,
) {
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path =
        unsafe { slice::from_raw_parts_mut(auth_path, (tree_height as usize) * SPX_N) };
    let ctx = unsafe { &*ctx };
    let tree_addr = unsafe { slice::from_raw_parts_mut(tree_addr, 8) };
    let info = unsafe { &mut *info };
    fors_treehashx1_inner(
        root,
        auth_path,
        ctx,
        leaf_idx,
        idx_offset,
        tree_height as usize,
        tree_addr,
        info,
    );
}

pub fn fors_treehashx1_inner(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: usize,
    tree_addr: &mut [u32],
    info: &mut ForsGenLeafInfo,
) {
    let mut stack = vec![0u8; tree_height * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        fors_gen_leafx1_inner(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h as usize == tree_height {
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
            set_tree_height_inner(tree_addr, h + 1);
            set_tree_index_inner(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]);
            let input_copy = current[..2 * SPX_N].to_vec();
            thash_inner(
                &mut current[SPX_N..2 * SPX_N],
                &input_copy,
                2,
                ctx,
                tree_addr,
            );

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        stack[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}
