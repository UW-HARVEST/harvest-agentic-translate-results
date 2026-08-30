//! Translation of `app/src/utilsx1.c`.

use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};
use crate::params::*;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};

/// Builds a WOTS Merkle tree, computing the auth path for `leaf_idx` and the
/// resulting root.
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
    let n = SPX_N;
    let mut stack = vec![0u8; tree_height as usize * n];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx = 0u32;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[n..2 * n], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..n].copy_from_slice(&current[n..2 * n]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let ho = h as usize * n;
                auth_path[ho..ho + n].copy_from_slice(&current[n..2 * n]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize * n;
            current[0..n].copy_from_slice(&stack[ho..ho + n]);
            // thash(&current[N], &current[0], 2, ...) with overlap -> copy input.
            let tmp = current;
            thash(&mut current[n..2 * n], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let ho = h as usize * n;
        stack[ho..ho + n].copy_from_slice(&current[n..2 * n]);
        idx += 1;
    }
}

/// Builds a FORS Merkle tree, computing the auth path for `leaf_idx` and the
/// resulting root.
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
    let n = SPX_N;
    let mut stack = vec![0u8; tree_height as usize * n];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx = 0u32;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[n..2 * n], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..n].copy_from_slice(&current[n..2 * n]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let ho = h as usize * n;
                auth_path[ho..ho + n].copy_from_slice(&current[n..2 * n]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize * n;
            current[0..n].copy_from_slice(&stack[ho..ho + n]);
            let tmp = current;
            thash(&mut current[n..2 * n], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let ho = h as usize * n;
        stack[ho..ho + n].copy_from_slice(&current[n..2 * n]);
        idx += 1;
    }
}

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[no_mangle]
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
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    let ap = core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
    let addr = &mut *(tree_addr as *mut [u32; 8]);
    wots_treehashx1(root_s, ap, &*ctx, leaf_idx, idx_offset, tree_height, addr, &mut *info);
}

#[no_mangle]
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
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    let ap = core::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N);
    let addr = &mut *(tree_addr as *mut [u32; 8]);
    fors_treehashx1(root_s, ap, &*ctx, leaf_idx, idx_offset, tree_height, addr, &mut *info);
}
