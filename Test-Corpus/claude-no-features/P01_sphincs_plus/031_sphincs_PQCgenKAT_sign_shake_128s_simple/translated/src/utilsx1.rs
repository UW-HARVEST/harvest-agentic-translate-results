use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::fors::{ForsGenLeafInfo, SPX_fors_gen_leafx1};
use crate::params::SPX_N;
use crate::thash::thash;
use crate::wots::LeafInfoX1;
use crate::wotsx1::SPX_wots_gen_leafx1;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut LeafInfoX1,
) {
    unsafe {
        let stack_size = (tree_height as usize) * SPX_N;
        let mut stack: Vec<u8> = vec![0u8; stack_size.max(1)];
        let max_idx: u32 = (1u32 << tree_height).wrapping_sub(1);
        let mut idx: u32 = 0;
        loop {
            let mut current = vec![0u8; 2 * SPX_N];
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
                    std::ptr::copy_nonoverlapping(current.as_ptr().add(SPX_N), root, SPX_N);
                    return;
                }

                if (internal_idx ^ internal_leaf) == 0x01 {
                    std::ptr::copy_nonoverlapping(
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

                // current[0..N] := stack[h*N..(h+1)*N]
                std::ptr::copy_nonoverlapping(
                    stack.as_ptr().add(h as usize * SPX_N),
                    current.as_mut_ptr(),
                    SPX_N,
                );
                let mut tmp = vec![0u8; SPX_N];
                thash(tmp.as_mut_ptr(), current.as_ptr(), 2, ctx, tree_addr);
                std::ptr::copy_nonoverlapping(tmp.as_ptr(), current.as_mut_ptr().add(SPX_N), SPX_N);

                h += 1;
                internal_idx >>= 1;
                internal_leaf >>= 1;
            }

            // Save current[N..] to stack[h*N..]
            std::ptr::copy_nonoverlapping(
                current.as_ptr().add(SPX_N),
                stack.as_mut_ptr().add(h as usize * SPX_N),
                SPX_N,
            );

            idx += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut ForsGenLeafInfo,
) {
    unsafe {
        let stack_size = (tree_height as usize) * SPX_N;
        let mut stack: Vec<u8> = vec![0u8; stack_size.max(1)];
        let max_idx: u32 = (1u32 << tree_height).wrapping_sub(1);
        let mut idx: u32 = 0;
        loop {
            let mut current = vec![0u8; 2 * SPX_N];
            SPX_fors_gen_leafx1(
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
                    std::ptr::copy_nonoverlapping(current.as_ptr().add(SPX_N), root, SPX_N);
                    return;
                }

                if (internal_idx ^ internal_leaf) == 0x01 {
                    std::ptr::copy_nonoverlapping(
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

                std::ptr::copy_nonoverlapping(
                    stack.as_ptr().add(h as usize * SPX_N),
                    current.as_mut_ptr(),
                    SPX_N,
                );
                let mut tmp = vec![0u8; SPX_N];
                thash(tmp.as_mut_ptr(), current.as_ptr(), 2, ctx, tree_addr);
                std::ptr::copy_nonoverlapping(tmp.as_ptr(), current.as_mut_ptr().add(SPX_N), SPX_N);

                h += 1;
                internal_idx >>= 1;
                internal_leaf >>= 1;
            }

            std::ptr::copy_nonoverlapping(
                current.as_ptr().add(SPX_N),
                stack.as_mut_ptr().add(h as usize * SPX_N),
                SPX_N,
            );

            idx += 1;
        }
    }
}
