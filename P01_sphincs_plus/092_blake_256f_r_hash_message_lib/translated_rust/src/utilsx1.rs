use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::ForsGenLeafInfo;
use crate::forsx1::fors_gen_leafx1;
use crate::params::*;
use crate::wotsx1::{LeafInfoX1, SPX_wots_gen_leafx1};

extern "C" {
    fn SPX_thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32);
}

unsafe fn thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    SPX_thash(out, in_, inblocks, ctx as *const SpxCtx, addr.as_mut_ptr());
}

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
    wots_treehashx1(
        root,
        auth_path,
        &*ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        &mut *(tree_addr as *mut [u32; 8]),
        &mut *info,
    );
}

pub unsafe fn wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];

        SPX_wots_gen_leafx1(
            current.as_mut_ptr().add(SPX_N),
            ctx,
            idx + idx_offset,
            info,
        );

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                core::ptr::copy_nonoverlapping(current.as_ptr().add(SPX_N), root, SPX_N);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                core::ptr::copy_nonoverlapping(
                    current.as_ptr().add(SPX_N),
                    auth_path.add(h as usize * SPX_N),
                    SPX_N,
                );
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left = stack.as_ptr().add(h as usize * SPX_N);
            core::ptr::copy_nonoverlapping(left, current.as_mut_ptr(), SPX_N);
            thash(
                current.as_mut_ptr().add(SPX_N),
                current.as_ptr(),
                2,
                ctx,
                tree_addr,
            );

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        core::ptr::copy_nonoverlapping(
            current.as_ptr().add(SPX_N),
            stack.as_mut_ptr().add(h as usize * SPX_N),
            SPX_N,
        );

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
    info: *mut LeafInfoX1,
) {
    fors_treehashx1(
        root,
        auth_path,
        &*ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        &mut *(tree_addr as *mut [u32; 8]),
        info,
    );
}

pub unsafe fn fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: *mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];

        fors_gen_leafx1(
            current.as_mut_ptr().add(SPX_N),
            ctx,
            idx + idx_offset,
            info as *mut ForsGenLeafInfo,
        );

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                core::ptr::copy_nonoverlapping(current.as_ptr().add(SPX_N), root, SPX_N);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                core::ptr::copy_nonoverlapping(
                    current.as_ptr().add(SPX_N),
                    auth_path.add(h as usize * SPX_N),
                    SPX_N,
                );
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left = stack.as_ptr().add(h as usize * SPX_N);
            core::ptr::copy_nonoverlapping(left, current.as_mut_ptr(), SPX_N);
            thash(
                current.as_mut_ptr().add(SPX_N),
                current.as_ptr(),
                2,
                ctx,
                tree_addr,
            );

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        core::ptr::copy_nonoverlapping(
            current.as_ptr().add(SPX_N),
            stack.as_mut_ptr().add(h as usize * SPX_N),
            SPX_N,
        );

        idx += 1;
    }
}
