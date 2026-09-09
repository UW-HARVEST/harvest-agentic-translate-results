//! Translation of `crypto_pwhash/argon2/argon2-fill-block-ref.c`
//! plus the `blamka-round-ref.h` round macros.

use crate::common::rotr64;

use super::argon2_core::*;

/* =====================================================================
 * blamka-round-ref.h
 * ===================================================================== */

/* designed by the Lyra PHC team */
#[inline(always)]
fn f_bla_mka(x: u64, y: u64) -> u64 {
    let m: u64 = 0xFFFFFFFF;
    let xy: u64 = (x & m).wrapping_mul(y & m);
    x.wrapping_add(y).wrapping_add(2u64.wrapping_mul(xy))
}

/* G(a, b, c, d) — operates on four &mut u64 */
#[inline(always)]
fn g(a: &mut u64, b: &mut u64, c: &mut u64, d: &mut u64) {
    *a = f_bla_mka(*a, *b);
    *d = rotr64(*d ^ *a, 32);
    *c = f_bla_mka(*c, *d);
    *b = rotr64(*b ^ *c, 24);
    *a = f_bla_mka(*a, *b);
    *d = rotr64(*d ^ *a, 16);
    *c = f_bla_mka(*c, *d);
    *b = rotr64(*b ^ *c, 63);
}

/* BLAKE2_ROUND_NOMSG on 16 elements of blockR.v starting at `base`
 * with stride `s` (so element k is v[base + k*s]). */
#[inline(always)]
unsafe fn blake2_round_nomsg(v: *mut u64, idx: &[usize; 16]) {
    macro_rules! call_g {
        ($i0:expr, $i1:expr, $i2:expr, $i3:expr) => {{
            let mut a = *v.add(idx[$i0]);
            let mut b = *v.add(idx[$i1]);
            let mut c = *v.add(idx[$i2]);
            let mut d = *v.add(idx[$i3]);
            g(&mut a, &mut b, &mut c, &mut d);
            *v.add(idx[$i0]) = a;
            *v.add(idx[$i1]) = b;
            *v.add(idx[$i2]) = c;
            *v.add(idx[$i3]) = d;
        }};
    }
    call_g!(0, 4, 8, 12);
    call_g!(1, 5, 9, 13);
    call_g!(2, 6, 10, 14);
    call_g!(3, 7, 11, 15);
    call_g!(0, 5, 10, 15);
    call_g!(1, 6, 11, 12);
    call_g!(2, 7, 8, 13);
    call_g!(3, 4, 9, 14);
}

/* =====================================================================
 * argon2-fill-block-ref.c
 * ===================================================================== */

unsafe fn fill_block(prev_block: *const block, ref_block: *const block, next_block: *mut block) {
    let mut block_r: block = core::mem::zeroed();
    let mut block_tmp: block = core::mem::zeroed();
    let mut i: usize;

    copy_block(&mut block_r, ref_block);
    xor_block(&mut block_r, prev_block);
    copy_block(&mut block_tmp, &block_r);
    /* Apply Blake2 on columns of 64-bit words: (0..15), (16..31)... (112..127) */
    let rv = block_r.v.as_mut_ptr();
    i = 0;
    while i < 8 {
        let mut idx = [0usize; 16];
        let mut k = 0;
        while k < 16 {
            idx[k] = 16 * i + k;
            k += 1;
        }
        blake2_round_nomsg(rv, &idx);
        i += 1;
    }

    /* Apply Blake2 on rows of 64-bit words */
    i = 0;
    while i < 8 {
        let idx: [usize; 16] = [
            2 * i,
            2 * i + 1,
            2 * i + 16,
            2 * i + 17,
            2 * i + 32,
            2 * i + 33,
            2 * i + 48,
            2 * i + 49,
            2 * i + 64,
            2 * i + 65,
            2 * i + 80,
            2 * i + 81,
            2 * i + 96,
            2 * i + 97,
            2 * i + 112,
            2 * i + 113,
        ];
        blake2_round_nomsg(rv, &idx);
        i += 1;
    }

    copy_block(next_block, &block_tmp);
    xor_block(next_block, &block_r);
}

unsafe fn fill_block_with_xor(
    prev_block: *const block,
    ref_block: *const block,
    next_block: *mut block,
) {
    let mut block_r: block = core::mem::zeroed();
    let mut block_tmp: block = core::mem::zeroed();
    let mut i: usize;

    copy_block(&mut block_r, ref_block);
    xor_block(&mut block_r, prev_block);
    copy_block(&mut block_tmp, &block_r);
    xor_block(&mut block_tmp, next_block); /* Saving the next block contents for XOR over */
    let rv = block_r.v.as_mut_ptr();
    i = 0;
    while i < 8 {
        let mut idx = [0usize; 16];
        let mut k = 0;
        while k < 16 {
            idx[k] = 16 * i + k;
            k += 1;
        }
        blake2_round_nomsg(rv, &idx);
        i += 1;
    }

    i = 0;
    while i < 8 {
        let idx: [usize; 16] = [
            2 * i,
            2 * i + 1,
            2 * i + 16,
            2 * i + 17,
            2 * i + 32,
            2 * i + 33,
            2 * i + 48,
            2 * i + 49,
            2 * i + 64,
            2 * i + 65,
            2 * i + 80,
            2 * i + 81,
            2 * i + 96,
            2 * i + 97,
            2 * i + 112,
            2 * i + 113,
        ];
        blake2_round_nomsg(rv, &idx);
        i += 1;
    }

    copy_block(next_block, &block_tmp);
    xor_block(next_block, &block_r);
}

unsafe fn generate_addresses(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rands: *mut u64,
) {
    let mut zero_block: block = core::mem::zeroed();
    let mut input_block: block = core::mem::zeroed();
    let mut address_block: block = core::mem::zeroed();
    let mut tmp_block: block = core::mem::zeroed();
    let mut i: u32;

    init_block_value(&mut zero_block, 0);
    init_block_value(&mut input_block, 0);

    if !instance.is_null() && !position.is_null() {
        input_block.v[0] = (*position).pass as u64;
        input_block.v[1] = (*position).lane as u64;
        input_block.v[2] = (*position).slice as u64;
        input_block.v[3] = (*instance).memory_blocks as u64;
        input_block.v[4] = (*instance).passes as u64;
        input_block.v[5] = (*instance).type_ as u64;

        i = 0;
        while i < (*instance).segment_length {
            if (i as usize) % ARGON2_ADDRESSES_IN_BLOCK == 0 {
                input_block.v[6] = input_block.v[6].wrapping_add(1);
                init_block_value(&mut tmp_block, 0);
                init_block_value(&mut address_block, 0);
                fill_block_with_xor(&zero_block, &input_block, &mut tmp_block);
                fill_block_with_xor(&zero_block, &tmp_block, &mut address_block);
            }

            *pseudo_rands.add(i as usize) =
                address_block.v[(i as usize) % ARGON2_ADDRESSES_IN_BLOCK];
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_segment_ref(
    instance: *const argon2_instance_t,
    mut position: argon2_position_t,
) {
    let mut ref_block: *mut block;
    let mut curr_block: *mut block;
    let pseudo_rands: *mut u64;
    let mut pseudo_rand: u64;
    let mut ref_index: u64;
    let mut ref_lane: u64;
    let mut prev_offset: u32;
    let mut curr_offset: u32;
    let starting_index: u32;
    let mut i: u32;
    let mut data_independent_addressing: c_int_local = 1;

    if instance.is_null() {
        return;
    }

    if (*instance).type_ == Argon2_id
        && (position.pass != 0 || position.slice as u32 >= ARGON2_SYNC_POINTS / 2)
    {
        data_independent_addressing = 0;
    }

    pseudo_rands = (*instance).pseudo_rands;

    if data_independent_addressing != 0 {
        generate_addresses(instance, &position, pseudo_rands);
    }

    starting_index = if (0 == position.pass) && (0 == position.slice) {
        2 /* we have already generated the first two blocks */
    } else {
        0
    };

    /* Offset of the current block */
    curr_offset = position.lane * (*instance).lane_length
        + (position.slice as u32) * (*instance).segment_length
        + starting_index;

    if 0 == curr_offset % (*instance).lane_length {
        /* Last block in this lane */
        prev_offset = curr_offset + (*instance).lane_length - 1;
    } else {
        /* Previous block */
        prev_offset = curr_offset - 1;
    }

    i = starting_index;
    while i < (*instance).segment_length {
        /*1.1 Rotating prev_offset if needed */
        if curr_offset % (*instance).lane_length == 1 {
            prev_offset = curr_offset - 1;
        }

        /* 1.2.1 Taking pseudo-random value from the previous block */
        if data_independent_addressing != 0 {
            pseudo_rand = *pseudo_rands.add(i as usize);
        } else {
            pseudo_rand = (*(*instance).region).memory.add(prev_offset as usize).as_ref().unwrap().v[0];
        }

        /* 1.2.2 Computing the lane of the reference block */
        ref_lane = (pseudo_rand >> 32) % ((*instance).lanes as u64);

        if (position.pass == 0) && (position.slice == 0) {
            /* Can not reference other lanes yet */
            ref_lane = position.lane as u64;
        }

        /* 1.2.3 Computing the number of possible reference block within the lane */
        position.index = i;
        ref_index = index_alpha(
            instance,
            &position,
            (pseudo_rand & 0xFFFFFFFF) as u32,
            (ref_lane == position.lane as u64) as c_int_local,
        ) as u64;

        /* 2 Creating a new block */
        ref_block = (*(*instance).region)
            .memory
            .add(((*instance).lane_length as u64 * ref_lane + ref_index) as usize);
        curr_block = (*(*instance).region).memory.add(curr_offset as usize);
        if position.pass != 0 {
            fill_block_with_xor(
                (*(*instance).region).memory.add(prev_offset as usize),
                ref_block,
                curr_block,
            );
        } else {
            fill_block(
                (*(*instance).region).memory.add(prev_offset as usize),
                ref_block,
                curr_block,
            );
        }

        i += 1;
        curr_offset += 1;
        prev_offset += 1;
    }
}

type c_int_local = core::ffi::c_int;
