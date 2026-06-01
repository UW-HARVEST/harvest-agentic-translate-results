// Translation of c_src/app/src/utils.c

use core::ffi::c_int;
use core::slice;

use crate::context::SpxCtx;
use crate::params::SPX_N;

/// Big-endian conversion of `in` into `outlen` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(
    out: *mut u8,
    outlen: core::ffi::c_uint,
    mut input: core::ffi::c_ulonglong,
) {
    let outlen = outlen as usize;
    let out = unsafe { slice::from_raw_parts_mut(out, outlen) };
    if outlen == 0 {
        return;
    }
    let mut i = outlen as isize - 1;
    while i >= 0 {
        out[i as usize] = (input & 0xff) as u8;
        input >>= 8;
        i -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, input: u32) {
    let out = unsafe { slice::from_raw_parts_mut(out, 4) };
    out[0] = (input >> 24) as u8;
    out[1] = (input >> 16) as u8;
    out[2] = (input >> 8) as u8;
    out[3] = input as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(
    input: *const u8,
    inlen: core::ffi::c_uint,
) -> core::ffi::c_ulonglong {
    let inlen = inlen as usize;
    let input = unsafe { slice::from_raw_parts(input, inlen) };
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// Pure Rust helpers
pub fn ull_to_bytes(out: &mut [u8], mut input: u64) {
    let outlen = out.len();
    if outlen == 0 {
        return;
    }
    let mut i = outlen as isize - 1;
    while i >= 0 {
        out[i as usize] = (input & 0xff) as u8;
        input >>= 8;
        i -= 1;
    }
}

pub fn u32_to_bytes(out: &mut [u8], input: u32) {
    out[0] = (input >> 24) as u8;
    out[1] = (input >> 16) as u8;
    out[2] = (input >> 8) as u8;
    out[3] = input as u8;
}

pub fn bytes_to_ull(input: &[u8]) -> u64 {
    let inlen = input.len();
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// Set tree height/index using crate::address
use crate::address::{set_tree_height_inner, set_tree_index_inner};
use crate::thash::thash_inner;

/// Compute root node given a leaf and an auth path.
#[unsafe(no_mangle)]
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
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    let leaf = unsafe { slice::from_raw_parts(leaf, SPX_N) };
    let auth_total_len = (tree_height as usize) * SPX_N;
    let auth_path_slice = unsafe { slice::from_raw_parts(auth_path, auth_total_len) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    compute_root_inner(
        root,
        leaf,
        leaf_idx,
        idx_offset,
        auth_path_slice,
        tree_height as usize,
        ctx,
        addr,
    );
}

pub fn compute_root_inner(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: usize,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    let mut buffer = vec![0u8; 2 * SPX_N];
    let mut ap_off = 0usize;

    if leaf_idx & 1 == 1 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(leaf);
        buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(leaf);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
    }
    ap_off += SPX_N;

    if tree_height == 0 {
        // edge: shouldn't happen in spec, but be defensive
        return;
    }

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height_inner(addr, (i as u32) + 1);
        set_tree_index_inner(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 == 1 {
            // thash works with overlapping pointers in C; we must mimic
            let mut out_tmp = vec![0u8; SPX_N];
            thash_inner(&mut out_tmp, &buffer, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&out_tmp);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            let mut out_tmp = vec![0u8; SPX_N];
            thash_inner(&mut out_tmp, &buffer, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&out_tmp);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        }
        ap_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height_inner(addr, tree_height as u32);
    set_tree_index_inner(addr, leaf_idx + idx_offset);
    thash_inner(root, &buffer, 2, ctx, addr);
}

/// Treehash: pointer to gen_leaf callback signature
pub type GenLeafFn =
    unsafe extern "C" fn(leaf: *mut u8, ctx: *const SpxCtx, addr_idx: u32, tree_addr: *const u32);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: *mut u32,
) {
    let root = unsafe { slice::from_raw_parts_mut(root, SPX_N) };
    let ah_len = (tree_height as usize) * SPX_N;
    let auth_path = unsafe { slice::from_raw_parts_mut(auth_path, ah_len) };
    let ctx_ref = unsafe { &*ctx };
    let tree_addr_slice = unsafe { slice::from_raw_parts_mut(tree_addr, 8) };

    let th = tree_height as usize;
    let mut stack = vec![0u8; (th + 1) * SPX_N];
    let mut heights = vec![0u32; th + 1];
    let mut offset: usize = 0;

    let max = 1u32 << th;
    for idx in 0..max {
        // gen_leaf via FFI callback
        unsafe {
            gen_leaf(
                stack.as_mut_ptr().add(offset * SPX_N),
                ctx,
                idx + idx_offset,
                tree_addr_slice.as_ptr(),
            );
        }
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N]
                .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            set_tree_height_inner(tree_addr_slice, heights[offset - 1] + 1);
            set_tree_index_inner(
                tree_addr_slice,
                tree_idx + (idx_offset >> (heights[offset - 1] + 1)),
            );

            // thash on stack[offset-2..offset]
            let start = (offset - 2) * SPX_N;
            // need a temporary copy since thash reads input then writes output
            let mut buf = vec![0u8; 2 * SPX_N];
            buf.copy_from_slice(&stack[start..start + 2 * SPX_N]);
            let mut out_tmp = vec![0u8; SPX_N];
            thash_inner(&mut out_tmp, &buf, 2, ctx_ref, tree_addr_slice);
            stack[start..start + SPX_N].copy_from_slice(&out_tmp);

            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                let src_start = (offset - 1) * SPX_N;
                let path_off = h * SPX_N;
                auth_path[path_off..path_off + SPX_N]
                    .copy_from_slice(&stack[src_start..src_start + SPX_N]);
            }
        }
    }

    root.copy_from_slice(&stack[..SPX_N]);
}

// Mark unused to silence
#[allow(dead_code)]
fn _unused(_: c_int) {}
