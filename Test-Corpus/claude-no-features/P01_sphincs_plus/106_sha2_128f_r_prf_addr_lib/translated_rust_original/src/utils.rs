use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::thash::thash;
use std::ffi::c_uint;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: c_uint, mut input: u64) {
    unsafe {
        let mut i: i32 = (outlen as i32) - 1;
        while i >= 0 {
            *out.add(i as usize) = (input & 0xff) as u8;
            input >>= 8;
            i -= 1;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, input: u32) {
    unsafe {
        *out.add(0) = (input >> 24) as u8;
        *out.add(1) = (input >> 16) as u8;
        *out.add(2) = (input >> 8) as u8;
        *out.add(3) = input as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(input: *const u8, inlen: c_uint) -> u64 {
    let mut retval: u64 = 0;
    unsafe {
        for i in 0..(inlen as usize) {
            retval |= (*input.add(i) as u64) << (8 * (inlen as usize - 1 - i));
        }
    }
    retval
}

// Safe internal helpers
pub fn ull_to_bytes_slice(out: &mut [u8], mut input: u64) {
    let outlen = out.len();
    if outlen == 0 {
        return;
    }
    let mut i: isize = outlen as isize - 1;
    while i >= 0 {
        out[i as usize] = (input & 0xff) as u8;
        input >>= 8;
        i -= 1;
    }
}

pub fn bytes_to_ull_slice(input: &[u8]) -> u64 {
    let mut retval: u64 = 0;
    let inlen = input.len();
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
/// Expects address to be complete other than the tree_height and tree_index.
#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let leaf_slice = std::slice::from_raw_parts(leaf, SPX_N);
        let mut buffer = [0u8; 2 * SPX_N];
        let mut auth_path_offset: usize = 0;
        let auth_buf = std::slice::from_raw_parts(auth_path, SPX_N * (tree_height as usize));

        if leaf_idx & 1 != 0 {
            buffer[SPX_N..2 * SPX_N].copy_from_slice(leaf_slice);
            buffer[0..SPX_N].copy_from_slice(&auth_buf[auth_path_offset..auth_path_offset + SPX_N]);
        } else {
            buffer[0..SPX_N].copy_from_slice(leaf_slice);
            buffer[SPX_N..2 * SPX_N]
                .copy_from_slice(&auth_buf[auth_path_offset..auth_path_offset + SPX_N]);
        }
        auth_path_offset += SPX_N;

        for i in 0..(tree_height - 1) {
            leaf_idx >>= 1;
            idx_offset >>= 1;

            set_tree_height(addr, i + 1);
            set_tree_index(addr, leaf_idx + idx_offset);

            if leaf_idx & 1 != 0 {
                let mut tmp = [0u8; SPX_N];
                thash(tmp.as_mut_ptr(), buffer.as_ptr(), 2, ctx, addr);
                buffer[SPX_N..2 * SPX_N].copy_from_slice(&tmp);
                buffer[0..SPX_N]
                    .copy_from_slice(&auth_buf[auth_path_offset..auth_path_offset + SPX_N]);
            } else {
                let mut tmp = [0u8; SPX_N];
                thash(tmp.as_mut_ptr(), buffer.as_ptr(), 2, ctx, addr);
                buffer[0..SPX_N].copy_from_slice(&tmp);
                buffer[SPX_N..2 * SPX_N]
                    .copy_from_slice(&auth_buf[auth_path_offset..auth_path_offset + SPX_N]);
            }
            auth_path_offset += SPX_N;
        }

        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, tree_height);
        set_tree_index(addr, leaf_idx + idx_offset);
        thash(root, buffer.as_ptr(), 2, ctx, addr);
    }
}

pub type GenLeafFn = unsafe extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32);

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: *mut u32,
) {
    unsafe {
        let stack_size = (tree_height as usize + 1) * SPX_N;
        let mut stack: Vec<u8> = vec![0u8; stack_size];
        let mut heights: Vec<u32> = vec![0u32; tree_height as usize + 1];
        let mut offset: u32 = 0;

        let max_idx: u32 = 1u32 << tree_height;
        for idx in 0..max_idx {
            gen_leaf(
                stack.as_mut_ptr().add((offset as usize) * SPX_N),
                ctx,
                idx + idx_offset,
                tree_addr,
            );
            offset += 1;
            heights[(offset - 1) as usize] = 0;

            if (leaf_idx ^ 0x1) == idx {
                let src = stack.as_ptr().add((offset as usize - 1) * SPX_N);
                std::ptr::copy_nonoverlapping(src, auth_path, SPX_N);
            }

            while offset >= 2
                && heights[offset as usize - 1] == heights[offset as usize - 2]
            {
                let h = heights[offset as usize - 1];
                let tree_idx = idx >> (h + 1);

                set_tree_height(tree_addr, h + 1);
                set_tree_index(tree_addr, tree_idx + (idx_offset >> (h + 1)));

                let dst_off = (offset as usize - 2) * SPX_N;
                let src_ptr = stack.as_ptr().add(dst_off);
                let dst_ptr = stack.as_mut_ptr().add(dst_off);
                thash(dst_ptr, src_ptr, 2, ctx, tree_addr);
                offset -= 1;
                heights[offset as usize - 1] += 1;

                let new_h = heights[offset as usize - 1];
                if ((leaf_idx >> new_h) ^ 0x1) == tree_idx {
                    let src = stack.as_ptr().add((offset as usize - 1) * SPX_N);
                    std::ptr::copy_nonoverlapping(
                        src,
                        auth_path.add(new_h as usize * SPX_N),
                        SPX_N,
                    );
                }
            }
        }

        std::ptr::copy_nonoverlapping(stack.as_ptr(), root, SPX_N);
    }
}
