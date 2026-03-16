use crate::params::*;

pub fn ull_to_bytes_internal(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (val & 0xff) as u8;
        val >>= 8;
    }
}

pub fn u32_to_bytes_internal(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

pub fn bytes_to_ull_internal(in_data: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (in_data[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub fn set_layer_addr_internal(addr: &mut [u32; 8], layer: u32) {
    let bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr_internal(addr: &mut [u32; 8], tree: u64) {
    let bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    ull_to_bytes_internal(&mut bytes[SPX_OFFSET_TREE..], 8, tree);
}

pub fn set_type_internal(addr: &mut [u32; 8], type_val: u32) {
    let bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr_internal(out: &mut [u32; 8], in_addr: &[u32; 8]) {
    let out_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, 32)
    };
    let in_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(in_addr.as_ptr() as *const u8, 32)
    };
    out_bytes[..SPX_OFFSET_TREE + 8].copy_from_slice(&in_bytes[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr_internal(addr: &mut [u32; 8], keypair: u32) {
    let bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    u32_to_bytes_internal(&mut bytes[SPX_OFFSET_KP_ADDR..], keypair);
}

pub fn copy_keypair_addr_internal(out: &mut [u32; 8], in_addr: &[u32; 8]) {
    let out_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, 32)
    };
    let in_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(in_addr.as_ptr() as *const u8, 32)
    };
    out_bytes[..SPX_OFFSET_TREE + 8].copy_from_slice(&in_bytes[..SPX_OFFSET_TREE + 8]);
    out_bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&in_bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr_internal(addr: &mut [u32; 8], chain: u32) {
    let bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr_internal(addr: &mut [u32; 8], hash: u32) {
    let bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height_internal(addr: &mut [u32; 8], tree_height: u32) {
    let bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    bytes[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index_internal(addr: &mut [u32; 8], tree_index: u32) {
    let bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, 32)
    };
    u32_to_bytes_internal(&mut bytes[SPX_OFFSET_TREE_INDEX..], tree_index);
}

pub fn compute_root_internal(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &crate::context::SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut auth_pos = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[auth_pos..auth_pos + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_pos..auth_pos + SPX_N]);
    }
    auth_pos += SPX_N;

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height_internal(addr, i + 1);
        set_tree_index_internal(addr, leaf_idx.wrapping_add(idx_offset));

        if leaf_idx & 1 != 0 {
            let tmp = buffer.clone();
            crate::thash::thash_internal(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_pos..auth_pos + SPX_N]);
        } else {
            let tmp = buffer.clone();
            crate::thash::thash_internal(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_pos..auth_pos + SPX_N]);
        }
        auth_pos += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height_internal(addr, tree_height);
    set_tree_index_internal(addr, leaf_idx.wrapping_add(idx_offset));
    crate::thash::thash_internal(root, &buffer, 2, ctx, addr);
}

pub fn treehash_internal(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &crate::context::SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: fn(&mut [u8], &crate::context::SpxCtx, u32, &[u32; 8]),
    tree_addr: &mut [u32; 8],
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; (th + 1) * SPX_N];
    let mut heights = vec![0u32; th + 1];
    let mut offset: usize = 0;

    for idx in 0..(1u32 << tree_height) {
        gen_leaf(&mut stack[offset * SPX_N..], ctx, idx.wrapping_add(idx_offset), tree_addr);
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..(offset) * SPX_N]);
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            set_tree_height_internal(tree_addr, heights[offset - 1] + 1);
            set_tree_index_internal(
                tree_addr,
                tree_idx.wrapping_add(idx_offset >> (heights[offset - 1] + 1)),
            );
            let base = (offset - 2) * SPX_N;
            // thash in-place: input is stack[base..base+2*SPX_N], output is stack[base..base+SPX_N]
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&stack[base..base + 2 * SPX_N]);
            crate::thash::thash_internal(&mut stack[base..], &tmp, 2, ctx, tree_addr);
            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                auth_path[h * SPX_N..(h + 1) * SPX_N]
                    .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}
