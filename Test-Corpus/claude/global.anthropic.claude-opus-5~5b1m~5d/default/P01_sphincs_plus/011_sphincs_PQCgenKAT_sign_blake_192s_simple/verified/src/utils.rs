//! Translation of `app/src/utils.c`.

use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::thash::thash;
use core::ffi::c_void;

/// Converts the value of `inv` to `outlen` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut inv: u64) {
    let mut i = outlen as isize - 1;
    while i >= 0 {
        out[i as usize] = (inv & 0xff) as u8;
        inv >>= 8;
        i -= 1;
    }
}

pub fn u32_to_bytes(out: &mut [u8], inv: u32) {
    out[0] = (inv >> 24) as u8;
    out[1] = (inv >> 16) as u8;
    out[2] = (inv >> 8) as u8;
    out[3] = inv as u8;
}

/// Converts `inlen` big-endian bytes in `inp` to an integer.
pub fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
/// Expects address to be complete other than the tree_height and tree_index.
pub fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut ap = 0usize; // offset into auth_path

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
    }
    ap += SPX_N;

    let mut i = 0u32;
    while i < tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp = buffer;
            thash(&mut buffer[SPX_N..2 * SPX_N], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
        } else {
            let tmp = buffer;
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap..ap + SPX_N]);
        }
        ap += SPX_N;
        i += 1;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    let tmp = buffer;
    thash(root, &tmp, 2, ctx, addr);
}

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, inv: u64) {
    let s = core::slice::from_raw_parts_mut(out, outlen as usize);
    ull_to_bytes(s, outlen as usize, inv);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, inv: u32) {
    let s = core::slice::from_raw_parts_mut(out, 4);
    u32_to_bytes(s, inv);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let s = core::slice::from_raw_parts(inp, inlen as usize);
    bytes_to_ull(s, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let root_s = core::slice::from_raw_parts_mut(root, SPX_N);
    let leaf_s = core::slice::from_raw_parts(leaf, SPX_N);
    let ap = core::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N);
    let addr_s = &mut *(addr as *mut [u32; 8]);
    compute_root(
        root_s,
        leaf_s,
        leaf_idx,
        idx_offset,
        ap,
        tree_height,
        &*ctx,
        addr_s,
    );
}

/// C function-pointer type for `gen_leaf` used by `treehash`.
pub type GenLeafFn =
    unsafe extern "C" fn(*mut u8, *const c_void, u32, *const u32);

/// Faithful translation of `treehash` (Merkle TreeHash). This generic routine
/// is part of the public library surface; the signer uses the specialised
/// `wots_treehashx1` / `fors_treehashx1` variants instead.
#[no_mangle]
pub unsafe extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const c_void,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: *mut u32,
) {
    let n = SPX_N;
    let mut stack = vec![0u8; (tree_height as usize + 1) * n];
    let mut heights = vec![0u32; tree_height as usize + 1];
    let mut offset: usize = 0;

    let addr_arr = tree_addr as *mut [u32; 8];

    for idx in 0..(1u32 << tree_height) {
        gen_leaf(
            stack.as_mut_ptr().add(offset * n),
            ctx,
            idx + idx_offset,
            tree_addr,
        );
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            core::ptr::copy_nonoverlapping(
                stack.as_ptr().add((offset - 1) * n),
                auth_path,
                n,
            );
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            set_tree_height(&mut *addr_arr, heights[offset - 1] + 1);
            set_tree_index(
                &mut *addr_arr,
                tree_idx + (idx_offset >> (heights[offset - 1] + 1)),
            );
            let base = (offset - 2) * n;
            // thash(stack+base, stack+base, 2, ...)
            let src: Vec<u8> = stack[base..base + 2 * n].to_vec();
            let out = core::slice::from_raw_parts_mut(stack.as_mut_ptr().add(base), n);
            thash(out, &src, 2, &*(ctx as *const SpxCtx), &mut *addr_arr);
            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                core::ptr::copy_nonoverlapping(
                    stack.as_ptr().add((offset - 1) * n),
                    auth_path.add(heights[offset - 1] as usize * n),
                    n,
                );
            }
        }
    }
    core::ptr::copy_nonoverlapping(stack.as_ptr(), root, n);
}
