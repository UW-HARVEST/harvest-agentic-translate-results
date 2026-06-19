use crate::address::{set_tree_height_rs, set_tree_index_rs};
use crate::context::spx_ctx;
use crate::params::*;
use crate::sha2_backend::SPX_thash_rs;

pub(crate) fn ull_to_bytes_into(out: &mut [u8], mut value: u64) {
    for byte in out.iter_mut().rev() {
        *byte = (value & 0xff) as u8;
        value >>= 8;
    }
}

pub(crate) fn u32_to_bytes_into(out: &mut [u8], value: u32) {
    out[0] = (value >> 24) as u8;
    out[1] = (value >> 16) as u8;
    out[2] = (value >> 8) as u8;
    out[3] = value as u8;
}

pub(crate) fn bytes_to_ull_impl(input: &[u8]) -> u64 {
    let mut value = 0u64;
    for (i, b) in input.iter().enumerate() {
        value |= (*b as u64) << (8 * (input.len() - 1 - i));
    }
    value
}

pub(crate) fn compute_root_rs(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &spx_ctx,
    addr: &mut [u32; 8],
) {
    let mut buffer = vec![0u8; 2 * SPX_N];
    if (leaf_idx & 1) != 0 {
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
        buffer[SPX_N..].copy_from_slice(leaf);
    } else {
        buffer[..SPX_N].copy_from_slice(leaf);
        buffer[SPX_N..].copy_from_slice(&auth_path[..SPX_N]);
    }
    let mut auth_off = SPX_N;
    for i in 0..(tree_height as usize - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height_rs(addr, (i + 1) as u32);
        set_tree_index_rs(addr, leaf_idx + idx_offset);
        if (leaf_idx & 1) != 0 {
            let mut out = [0u8; SPX_N];
            SPX_thash_rs(&mut out, &buffer, 2, ctx, addr);
            buffer[SPX_N..].copy_from_slice(&out);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            let mut out = [0u8; SPX_N];
            SPX_thash_rs(&mut out, &buffer, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&out);
            buffer[SPX_N..].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }
    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height_rs(addr, tree_height);
    set_tree_index_rs(addr, leaf_idx + idx_offset);
    SPX_thash_rs(root, &buffer, 2, ctx, addr);
}

pub(crate) fn treehash_rs(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &spx_ctx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: fn(&mut [u8], &spx_ctx, u32, &[u32; 8]),
    tree_addr: &mut [u32; 8],
) {
    let mut stack = vec![0u8; (tree_height as usize + 1) * SPX_N];
    let mut heights = vec![0u32; tree_height as usize + 1];
    let mut offset = 0usize;
    for idx in 0..(1u32 << tree_height) {
        gen_leaf(
            &mut stack[offset * SPX_N..(offset + 1) * SPX_N],
            ctx,
            idx + idx_offset,
            tree_addr,
        );
        offset += 1;
        heights[offset - 1] = 0;
        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }
        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let h = heights[offset - 1];
            let tree_idx = idx >> (h + 1);
            set_tree_height_rs(tree_addr, h + 1);
            set_tree_index_rs(tree_addr, tree_idx + (idx_offset >> (h + 1)));
            let start = (offset - 2) * SPX_N;
            let input = stack[start..start + 2 * SPX_N].to_vec();
            SPX_thash_rs(&mut stack[start..start + SPX_N], &input, 2, ctx, tree_addr);
            offset -= 1;
            heights[offset - 1] += 1;
            let hh = heights[offset - 1] as usize;
            if ((leaf_idx >> hh) ^ 0x1) == tree_idx {
                auth_path[hh * SPX_N..(hh + 1) * SPX_N]
                    .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root.copy_from_slice(&stack[..SPX_N]);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, input: u64) {
    ull_to_bytes_into(unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) }, input);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, input: u32) {
    u32_to_bytes_into(unsafe { std::slice::from_raw_parts_mut(out, 4) }, input);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(input: *const u8, inlen: u32) -> u64 {
    bytes_to_ull_impl(unsafe { std::slice::from_raw_parts(input, inlen as usize) })
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const spx_ctx,
    addr: *mut u32,
) {
    compute_root_rs(
        unsafe { std::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe { std::slice::from_raw_parts(leaf, SPX_N) },
        leaf_idx,
        idx_offset,
        unsafe { std::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N) },
        tree_height,
        unsafe { &*ctx },
        unsafe { &mut *(addr as *mut [u32; 8]) },
    );
}
