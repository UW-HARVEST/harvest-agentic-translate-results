//! Translation of libsodium argon2 (reference path).
//! Covers argon2.c, argon2-core.c, argon2-fill-block-ref.c, blake2b-long.c,
//! argon2-encoding.c, pwhash_argon2i.c, pwhash_argon2id.c.

use core::ffi::{c_char, c_int, c_void};
use crate::common::{load64_le, rotr64, store32_le, store64_le};

// ---------------------------------------------------------------------------
// Externs from other packages
// ---------------------------------------------------------------------------

#[repr(C, align(64))]
struct Blake2bState {
    opaque: [u8; 384],
}

extern "C" {
    fn crypto_generichash_blake2b_init(
        state: *mut Blake2bState,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_update(
        state: *mut Blake2bState,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_blake2b_final(
        state: *mut Blake2bState,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: u64,
        key: *const u8,
        keylen: usize,
    ) -> c_int;

    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> c_int;

    fn sodium_bin2base64(
        b64: *mut c_char,
        b64_maxlen: usize,
        bin: *const u8,
        bin_len: usize,
        variant: c_int,
    ) -> *mut c_char;
    fn sodium_base642bin(
        bin: *mut u8,
        bin_maxlen: usize,
        b64: *const c_char,
        b64_len: usize,
        ignore: *const c_char,
        bin_len: *mut usize,
        b64_end: *mut *const c_char,
        variant: c_int,
    ) -> c_int;
}

const SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING: c_int = 3;
const CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX: usize = 64;

#[inline]
unsafe fn set_errno(e: c_int) {
    *libc::__errno_location() = e;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ARGON2_VERSION_NUMBER: u32 = 0x13;
const ARGON2_BLOCK_SIZE: usize = 1024;
const ARGON2_QWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 8; // 128
const ARGON2_ADDRESSES_IN_BLOCK: usize = 128;
const ARGON2_PREHASH_DIGEST_LENGTH: usize = 64;
const ARGON2_PREHASH_SEED_LENGTH: usize = 72;
const ARGON2_SYNC_POINTS: u32 = 4;

const ARGON2_MIN_OUTLEN: u32 = 16;
const ARGON2_MAX_OUTLEN: u32 = 0xFFFFFFFF;
const ARGON2_MIN_PWD_LENGTH: u32 = 0;
const ARGON2_MAX_PWD_LENGTH: u32 = 0xFFFFFFFF;
const ARGON2_MIN_AD_LENGTH: u32 = 0;
const ARGON2_MAX_AD_LENGTH: u32 = 0xFFFFFFFF;
const ARGON2_MIN_SALT_LENGTH: u32 = 8;
const ARGON2_MAX_SALT_LENGTH: u32 = 0xFFFFFFFF;
const ARGON2_MIN_SECRET: u32 = 0;
const ARGON2_MAX_SECRET: u32 = 0xFFFFFFFF;
const ARGON2_MIN_LANES: u32 = 1;
const ARGON2_MAX_LANES: u32 = 0xFFFFFF;
const ARGON2_MIN_THREADS: u32 = 1;
const ARGON2_MAX_THREADS: u32 = 0xFFFFFF;
const ARGON2_MIN_MEMORY: u32 = 2 * ARGON2_SYNC_POINTS; // 8
// ARGON2_MAX_MEMORY on 64-bit: min(0xFFFFFFFF, 1<<32) = 0xFFFFFFFF
const ARGON2_MAX_MEMORY: u32 = 0xFFFFFFFF;
const ARGON2_MIN_TIME: u32 = 1;
const ARGON2_MAX_TIME: u32 = 0xFFFFFFFF;

const ARGON2_FLAG_CLEAR_PASSWORD: u32 = 1 << 0;
const ARGON2_FLAG_CLEAR_SECRET: u32 = 1 << 1;
const ARGON2_DEFAULT_FLAGS: u32 = 0;

// Error codes
const ARGON2_OK: c_int = 0;
const ARGON2_OUTPUT_PTR_NULL: c_int = -1;
const ARGON2_OUTPUT_TOO_SHORT: c_int = -2;
const ARGON2_OUTPUT_TOO_LONG: c_int = -3;
const ARGON2_PWD_TOO_SHORT: c_int = -4;
const ARGON2_PWD_TOO_LONG: c_int = -5;
const ARGON2_SALT_TOO_SHORT: c_int = -6;
const ARGON2_SALT_TOO_LONG: c_int = -7;
const ARGON2_AD_TOO_SHORT: c_int = -8;
const ARGON2_AD_TOO_LONG: c_int = -9;
const ARGON2_SECRET_TOO_SHORT: c_int = -10;
const ARGON2_SECRET_TOO_LONG: c_int = -11;
const ARGON2_TIME_TOO_SMALL: c_int = -12;
const ARGON2_TIME_TOO_LARGE: c_int = -13;
const ARGON2_MEMORY_TOO_LITTLE: c_int = -14;
const ARGON2_MEMORY_TOO_MUCH: c_int = -15;
const ARGON2_LANES_TOO_FEW: c_int = -16;
const ARGON2_LANES_TOO_MANY: c_int = -17;
const ARGON2_PWD_PTR_MISMATCH: c_int = -18;
const ARGON2_SALT_PTR_MISMATCH: c_int = -19;
const ARGON2_SECRET_PTR_MISMATCH: c_int = -20;
const ARGON2_AD_PTR_MISMATCH: c_int = -21;
const ARGON2_MEMORY_ALLOCATION_ERROR: c_int = -22;
const ARGON2_INCORRECT_PARAMETER: c_int = -25;
const ARGON2_INCORRECT_TYPE: c_int = -26;
const ARGON2_THREADS_TOO_FEW: c_int = -28;
const ARGON2_THREADS_TOO_MANY: c_int = -29;
const ARGON2_ENCODING_FAIL: c_int = -31;
const ARGON2_DECODING_FAIL: c_int = -32;
const ARGON2_DECODING_LENGTH_FAIL: c_int = -34;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

// argon2_type
const ARGON2_I: c_int = 1;
const ARGON2_ID: c_int = 2;

// ---------------------------------------------------------------------------
// Structures (repr(C) to match argon2 internal ABI)
// ---------------------------------------------------------------------------

type Block = [u64; ARGON2_QWORDS_IN_BLOCK];

#[repr(C)]
struct block_region {
    base: *mut c_void,
    memory: *mut Block,
    size: usize,
}

#[repr(C)]
pub struct argon2_context {
    out: *mut u8,
    outlen: u32,
    pwd: *mut u8,
    pwdlen: u32,
    salt: *mut u8,
    saltlen: u32,
    secret: *mut u8,
    secretlen: u32,
    ad: *mut u8,
    adlen: u32,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
}

#[repr(C)]
pub struct argon2_instance_t {
    region: *mut block_region,
    pseudo_rands: *mut u64,
    passes: u32,
    current_pass: u32,
    memory_blocks: u32,
    segment_length: u32,
    lane_length: u32,
    lanes: u32,
    threads: u32,
    type_: c_int,
    print_internals: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct argon2_position_t {
    pass: u32,
    lane: u32,
    slice: u8,
    index: u32,
}

// ---------------------------------------------------------------------------
// Block helpers
// ---------------------------------------------------------------------------

#[inline(always)]
unsafe fn init_block_value(b: *mut Block, val: u8) {
    core::ptr::write_bytes(b as *mut u8, val, ARGON2_BLOCK_SIZE);
}

#[inline(always)]
unsafe fn copy_block(dst: *mut Block, src: *const Block) {
    core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, ARGON2_BLOCK_SIZE);
}

#[inline(always)]
unsafe fn xor_block(dst: *mut Block, src: *const Block) {
    for i in 0..ARGON2_QWORDS_IN_BLOCK {
        (*dst)[i] ^= (*src)[i];
    }
}

unsafe fn load_block(dst: *mut Block, input: *const u8) {
    for i in 0..ARGON2_QWORDS_IN_BLOCK {
        let s = core::slice::from_raw_parts(input.add(i * 8), 8);
        (*dst)[i] = load64_le(s);
    }
}

unsafe fn store_block(output: *mut u8, src: *const Block) {
    for i in 0..ARGON2_QWORDS_IN_BLOCK {
        let d = core::slice::from_raw_parts_mut(output.add(i * 8), 8);
        store64_le(d, (*src)[i]);
    }
}

// ---------------------------------------------------------------------------
// BlaMka round (blamka-round-ref.h)
// ---------------------------------------------------------------------------

#[inline(always)]
fn fblamka(x: u64, y: u64) -> u64 {
    let m: u64 = 0xFFFFFFFF;
    let xy = (x & m).wrapping_mul(y & m);
    x.wrapping_add(y).wrapping_add(2u64.wrapping_mul(xy))
}

#[inline(always)]
fn g(v: &mut [u64; ARGON2_QWORDS_IN_BLOCK], a: usize, b: usize, c: usize, d: usize) {
    v[a] = fblamka(v[a], v[b]);
    v[d] = rotr64(v[d] ^ v[a], 32);
    v[c] = fblamka(v[c], v[d]);
    v[b] = rotr64(v[b] ^ v[c], 24);
    v[a] = fblamka(v[a], v[b]);
    v[d] = rotr64(v[d] ^ v[a], 16);
    v[c] = fblamka(v[c], v[d]);
    v[b] = rotr64(v[b] ^ v[c], 63);
}

#[inline(always)]
fn blake2_round_nomsg(v: &mut [u64; ARGON2_QWORDS_IN_BLOCK], idx: [usize; 16]) {
    g(v, idx[0], idx[4], idx[8], idx[12]);
    g(v, idx[1], idx[5], idx[9], idx[13]);
    g(v, idx[2], idx[6], idx[10], idx[14]);
    g(v, idx[3], idx[7], idx[11], idx[15]);
    g(v, idx[0], idx[5], idx[10], idx[15]);
    g(v, idx[1], idx[6], idx[11], idx[12]);
    g(v, idx[2], idx[7], idx[8], idx[13]);
    g(v, idx[3], idx[4], idx[9], idx[14]);
}

fn apply_rounds(block_r: &mut [u64; ARGON2_QWORDS_IN_BLOCK]) {
    // Columns
    for i in 0..8usize {
        let b = 16 * i;
        blake2_round_nomsg(
            block_r,
            [
                b, b + 1, b + 2, b + 3, b + 4, b + 5, b + 6, b + 7, b + 8, b + 9, b + 10, b + 11,
                b + 12, b + 13, b + 14, b + 15,
            ],
        );
    }
    // Rows
    for i in 0..8usize {
        let b = 2 * i;
        blake2_round_nomsg(
            block_r,
            [
                b,
                b + 1,
                b + 16,
                b + 17,
                b + 32,
                b + 33,
                b + 48,
                b + 49,
                b + 64,
                b + 65,
                b + 80,
                b + 81,
                b + 96,
                b + 97,
                b + 112,
                b + 113,
            ],
        );
    }
}

unsafe fn fill_block(prev_block: *const Block, ref_block: *const Block, next_block: *mut Block) {
    let mut block_r: Block = *ref_block;
    xor_block(&mut block_r, prev_block);
    let block_tmp: Block = block_r;

    apply_rounds(&mut block_r);

    *next_block = block_tmp;
    xor_block(next_block, &block_r);
}

unsafe fn fill_block_with_xor(
    prev_block: *const Block,
    ref_block: *const Block,
    next_block: *mut Block,
) {
    let mut block_r: Block = *ref_block;
    xor_block(&mut block_r, prev_block);
    let mut block_tmp: Block = block_r;
    xor_block(&mut block_tmp, next_block);

    apply_rounds(&mut block_r);

    *next_block = block_tmp;
    xor_block(next_block, &block_r);
}

// ---------------------------------------------------------------------------
// blake2b-long.c  ->  _sodium_blake2b_long
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_long(
    pout: *mut c_void,
    outlen: usize,
    in_: *const c_void,
    inlen: usize,
) -> c_int {
    let mut out = pout as *mut u8;
    let mut blake_state = Blake2bState { opaque: [0u8; 384] };
    let mut outlen_bytes = [0u8; 4];
    let mut ret: c_int = -1;

    'fail: {
        if outlen > u32::MAX as usize {
            break 'fail;
        }
        store32_le(&mut outlen_bytes, outlen as u32);

        macro_rules! trys {
            ($stmt:expr) => {{
                ret = $stmt;
                if ret < 0 {
                    break 'fail;
                }
            }};
        }

        if outlen <= CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX {
            trys!(crypto_generichash_blake2b_init(
                &mut blake_state,
                core::ptr::null(),
                0,
                outlen
            ));
            trys!(crypto_generichash_blake2b_update(
                &mut blake_state,
                outlen_bytes.as_ptr(),
                4
            ));
            trys!(crypto_generichash_blake2b_update(
                &mut blake_state,
                in_ as *const u8,
                inlen as u64
            ));
            trys!(crypto_generichash_blake2b_final(
                &mut blake_state,
                out,
                outlen
            ));
        } else {
            let mut toproduce: u32;
            let mut out_buffer = [0u8; CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX];
            let mut in_buffer = [0u8; CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX];
            trys!(crypto_generichash_blake2b_init(
                &mut blake_state,
                core::ptr::null(),
                0,
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX
            ));
            trys!(crypto_generichash_blake2b_update(
                &mut blake_state,
                outlen_bytes.as_ptr(),
                4
            ));
            trys!(crypto_generichash_blake2b_update(
                &mut blake_state,
                in_ as *const u8,
                inlen as u64
            ));
            trys!(crypto_generichash_blake2b_final(
                &mut blake_state,
                out_buffer.as_mut_ptr(),
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX
            ));
            let half = CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX / 2; // 32
            core::ptr::copy_nonoverlapping(out_buffer.as_ptr(), out, half);
            out = out.add(half);
            toproduce = (outlen as u32) - half as u32;

            while toproduce as usize > CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX {
                core::ptr::copy_nonoverlapping(
                    out_buffer.as_ptr(),
                    in_buffer.as_mut_ptr(),
                    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX,
                );
                trys!(crypto_generichash_blake2b(
                    out_buffer.as_mut_ptr(),
                    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX,
                    in_buffer.as_ptr(),
                    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX as u64,
                    core::ptr::null(),
                    0
                ));
                core::ptr::copy_nonoverlapping(out_buffer.as_ptr(), out, half);
                out = out.add(half);
                toproduce -= half as u32;
            }

            core::ptr::copy_nonoverlapping(
                out_buffer.as_ptr(),
                in_buffer.as_mut_ptr(),
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX,
            );
            trys!(crypto_generichash_blake2b(
                out_buffer.as_mut_ptr(),
                toproduce as usize,
                in_buffer.as_ptr(),
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX as u64,
                core::ptr::null(),
                0
            ));
            core::ptr::copy_nonoverlapping(out_buffer.as_ptr(), out, toproduce as usize);
        }
    }
    sodium_memzero(
        &mut blake_state as *mut Blake2bState as *mut c_void,
        core::mem::size_of::<Blake2bState>(),
    );
    ret
}

// ---------------------------------------------------------------------------
// argon2-core.c
// ---------------------------------------------------------------------------

fn index_alpha(
    instance: &argon2_instance_t,
    position: &argon2_position_t,
    pseudo_rand: u32,
    same_lane: bool,
) -> u32 {
    let reference_area_size: u32;

    if position.pass == 0 {
        if position.slice == 0 {
            reference_area_size = position.index.wrapping_sub(1);
        } else if same_lane {
            reference_area_size = (position.slice as u32)
                .wrapping_mul(instance.segment_length)
                .wrapping_add(position.index)
                .wrapping_sub(1);
        } else {
            let adj: u32 = if position.index == 0 { 0u32.wrapping_sub(1) } else { 0 };
            reference_area_size = (position.slice as u32)
                .wrapping_mul(instance.segment_length)
                .wrapping_add(adj);
        }
    } else if same_lane {
        reference_area_size = instance
            .lane_length
            .wrapping_sub(instance.segment_length)
            .wrapping_add(position.index)
            .wrapping_sub(1);
    } else {
        let adj: u32 = if position.index == 0 { 0u32.wrapping_sub(1) } else { 0 };
        reference_area_size = instance
            .lane_length
            .wrapping_sub(instance.segment_length)
            .wrapping_add(adj);
    }

    let mut relative_position: u64 = pseudo_rand as u64;
    relative_position = relative_position.wrapping_mul(relative_position) >> 32;
    relative_position = ((reference_area_size.wrapping_sub(1)) as u64)
        .wrapping_sub((reference_area_size as u64).wrapping_mul(relative_position) >> 32);

    let mut start_position: u32 = 0;
    if position.pass != 0 {
        start_position = if position.slice as u32 == ARGON2_SYNC_POINTS - 1 {
            0
        } else {
            (position.slice as u32 + 1).wrapping_mul(instance.segment_length)
        };
    }

    let mut absolute_position: u64 = (start_position as u64)
        .wrapping_add(relative_position)
        .wrapping_sub(instance.lane_length as u64);
    absolute_position = absolute_position
        .wrapping_add((instance.lane_length as u64) & (absolute_position >> 32));
    absolute_position as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_validate_inputs(
    context: *const argon2_context,
) -> c_int {
    if context.is_null() {
        return ARGON2_INCORRECT_PARAMETER;
    }
    let ctx = &*context;

    if ctx.out.is_null() {
        return ARGON2_OUTPUT_PTR_NULL;
    }
    if ARGON2_MIN_OUTLEN > ctx.outlen {
        return ARGON2_OUTPUT_TOO_SHORT;
    }
    if ARGON2_MAX_OUTLEN < ctx.outlen {
        return ARGON2_OUTPUT_TOO_LONG;
    }
    if ctx.pwd.is_null() {
        if 0 != ctx.pwdlen {
            return ARGON2_PWD_PTR_MISMATCH;
        }
    }
    if ARGON2_MIN_PWD_LENGTH > ctx.pwdlen {
        return ARGON2_PWD_TOO_SHORT;
    }
    if ARGON2_MAX_PWD_LENGTH < ctx.pwdlen {
        return ARGON2_PWD_TOO_LONG;
    }
    if ctx.salt.is_null() {
        if 0 != ctx.saltlen {
            return ARGON2_SALT_PTR_MISMATCH;
        }
    }
    if ARGON2_MIN_SALT_LENGTH > ctx.saltlen {
        return ARGON2_SALT_TOO_SHORT;
    }
    if ARGON2_MAX_SALT_LENGTH < ctx.saltlen {
        return ARGON2_SALT_TOO_LONG;
    }
    if ctx.secret.is_null() {
        if 0 != ctx.secretlen {
            return ARGON2_SECRET_PTR_MISMATCH;
        }
    } else {
        if ARGON2_MIN_SECRET > ctx.secretlen {
            return ARGON2_SECRET_TOO_SHORT;
        }
        if ARGON2_MAX_SECRET < ctx.secretlen {
            return ARGON2_SECRET_TOO_LONG;
        }
    }
    if ctx.ad.is_null() {
        if 0 != ctx.adlen {
            return ARGON2_AD_PTR_MISMATCH;
        }
    } else {
        if ARGON2_MIN_AD_LENGTH > ctx.adlen {
            return ARGON2_AD_TOO_SHORT;
        }
        if ARGON2_MAX_AD_LENGTH < ctx.adlen {
            return ARGON2_AD_TOO_LONG;
        }
    }
    if ARGON2_MIN_LANES > ctx.lanes {
        return ARGON2_LANES_TOO_FEW;
    }
    if ARGON2_MAX_LANES < ctx.lanes {
        return ARGON2_LANES_TOO_MANY;
    }
    if ARGON2_MIN_MEMORY > ctx.m_cost {
        return ARGON2_MEMORY_TOO_LITTLE;
    }
    if ARGON2_MAX_MEMORY < ctx.m_cost {
        return ARGON2_MEMORY_TOO_MUCH;
    }
    if ctx.m_cost < 8u32.wrapping_mul(ctx.lanes) {
        return ARGON2_MEMORY_TOO_LITTLE;
    }
    if ARGON2_MIN_TIME > ctx.t_cost {
        return ARGON2_TIME_TOO_SMALL;
    }
    if ARGON2_MAX_TIME < ctx.t_cost {
        return ARGON2_TIME_TOO_LARGE;
    }
    if ARGON2_MIN_THREADS > ctx.threads {
        return ARGON2_THREADS_TOO_FEW;
    }
    if ARGON2_MAX_THREADS < ctx.threads {
        return ARGON2_THREADS_TOO_MANY;
    }
    ARGON2_OK
}

unsafe fn allocate_memory(region: *mut *mut block_region, m_cost: u32) -> c_int {
    if region.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    let block_sz = core::mem::size_of::<Block>(); // 1024
    let memory_size = block_sz.wrapping_mul(m_cost as usize);
    if m_cost == 0 || memory_size / (m_cost as usize) != block_sz {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    *region = libc::malloc(core::mem::size_of::<block_region>()) as *mut block_region;
    if (*region).is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    (**region).base = core::ptr::null_mut();
    (**region).memory = core::ptr::null_mut();

    // Portable path: malloc(memory_size + 63) and align to 64.
    let mut base: *mut c_void = core::ptr::null_mut();
    let mut memory: *mut Block = core::ptr::null_mut();
    if memory_size + 63 < memory_size {
        base = core::ptr::null_mut();
        set_errno(libc::ENOMEM);
    } else {
        base = libc::malloc(memory_size + 63);
        if !base.is_null() {
            let mut aligned = (base as *mut u8).add(63);
            aligned = aligned.sub((aligned as usize) & 63);
            memory = aligned as *mut Block;
        }
    }
    if base.is_null() {
        libc::free(*region as *mut c_void);
        *region = core::ptr::null_mut();
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    (**region).base = base;
    (**region).memory = memory;
    (**region).size = memory_size;

    ARGON2_OK
}

unsafe fn free_memory(region: *mut block_region) {
    if !region.is_null() && !(*region).base.is_null() {
        libc::free((*region).base);
    }
    libc::free(region as *mut c_void);
}

unsafe fn argon2_free_instance(instance: *mut argon2_instance_t, _flags: u32) {
    libc::free((*instance).pseudo_rands as *mut c_void);
    (*instance).pseudo_rands = core::ptr::null_mut();
    free_memory((*instance).region);
    (*instance).region = core::ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_finalize(
    context: *const argon2_context,
    instance: *mut argon2_instance_t,
) {
    if !context.is_null() && !instance.is_null() {
        let ctx = &*context;
        let inst = &mut *instance;
        let memory = (*inst.region).memory;

        let mut blockhash: Block = [0u64; ARGON2_QWORDS_IN_BLOCK];
        copy_block(
            &mut blockhash,
            memory.add((inst.lane_length - 1) as usize),
        );

        let mut l: u32 = 1;
        while l < inst.lanes {
            let last_block_in_lane =
                l * inst.lane_length + (inst.lane_length - 1);
            xor_block(&mut blockhash, memory.add(last_block_in_lane as usize));
            l += 1;
        }

        {
            let mut blockhash_bytes = [0u8; ARGON2_BLOCK_SIZE];
            store_block(blockhash_bytes.as_mut_ptr(), &blockhash);
            _sodium_blake2b_long(
                ctx.out as *mut c_void,
                ctx.outlen as usize,
                blockhash_bytes.as_ptr() as *const c_void,
                ARGON2_BLOCK_SIZE,
            );
            sodium_memzero(blockhash.as_mut_ptr() as *mut c_void, ARGON2_BLOCK_SIZE);
            sodium_memzero(
                blockhash_bytes.as_mut_ptr() as *mut c_void,
                ARGON2_BLOCK_SIZE,
            );
        }

        argon2_free_instance(instance, ctx.flags);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_memory_blocks(
    instance: *mut argon2_instance_t,
    pass: u32,
) {
    if instance.is_null() || (*instance).lanes == 0 {
        return;
    }
    let mut position = argon2_position_t {
        pass,
        lane: 0,
        slice: 0,
        index: 0,
    };
    let lanes = (*instance).lanes;
    let mut s: u32 = 0;
    while s < ARGON2_SYNC_POINTS {
        position.slice = s as u8;
        let mut l: u32 = 0;
        while l < lanes {
            position.lane = l;
            position.index = 0;
            FILL_SEGMENT(instance, position);
            l += 1;
        }
        s += 1;
    }
}

unsafe fn generate_addresses(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rands: *mut u64,
) {
    let mut zero_block: Block = [0u64; ARGON2_QWORDS_IN_BLOCK];
    let mut input_block: Block = [0u64; ARGON2_QWORDS_IN_BLOCK];
    let mut address_block: Block = [0u64; ARGON2_QWORDS_IN_BLOCK];
    let mut tmp_block: Block = [0u64; ARGON2_QWORDS_IN_BLOCK];

    init_block_value(&mut zero_block, 0);
    init_block_value(&mut input_block, 0);

    if !instance.is_null() && !position.is_null() {
        let inst = &*instance;
        let pos = &*position;
        input_block[0] = pos.pass as u64;
        input_block[1] = pos.lane as u64;
        input_block[2] = pos.slice as u64;
        input_block[3] = inst.memory_blocks as u64;
        input_block[4] = inst.passes as u64;
        input_block[5] = inst.type_ as u64;

        let mut i: u32 = 0;
        while i < inst.segment_length {
            if (i as usize) % ARGON2_ADDRESSES_IN_BLOCK == 0 {
                input_block[6] = input_block[6].wrapping_add(1);
                init_block_value(&mut tmp_block, 0);
                init_block_value(&mut address_block, 0);
                fill_block_with_xor(&zero_block, &input_block, &mut tmp_block);
                fill_block_with_xor(&zero_block, &tmp_block, &mut address_block);
            }
            *pseudo_rands.add(i as usize) =
                address_block[(i as usize) % ARGON2_ADDRESSES_IN_BLOCK];
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_segment_ref(
    instance: *const argon2_instance_t,
    mut position: argon2_position_t,
) {
    if instance.is_null() {
        return;
    }
    let inst = &*instance;

    let mut data_independent_addressing: bool = true;
    if inst.type_ == ARGON2_ID
        && (position.pass != 0 || position.slice as u32 >= ARGON2_SYNC_POINTS / 2)
    {
        data_independent_addressing = false;
    }

    let pseudo_rands = inst.pseudo_rands;

    if data_independent_addressing {
        generate_addresses(instance, &position, pseudo_rands);
    }

    let mut starting_index: u32 = 0;
    if position.pass == 0 && position.slice == 0 {
        starting_index = 2;
    }

    let mut curr_offset: u32 = position.lane * inst.lane_length
        + (position.slice as u32) * inst.segment_length
        + starting_index;

    let mut prev_offset: u32;
    if 0 == curr_offset % inst.lane_length {
        prev_offset = curr_offset + inst.lane_length - 1;
    } else {
        prev_offset = curr_offset - 1;
    }

    let memory = (*inst.region).memory;

    let mut i: u32 = starting_index;
    while i < inst.segment_length {
        if curr_offset % inst.lane_length == 1 {
            prev_offset = curr_offset - 1;
        }

        let pseudo_rand: u64 = if data_independent_addressing {
            *pseudo_rands.add(i as usize)
        } else {
            (*memory.add(prev_offset as usize))[0]
        };

        let mut ref_lane: u64 = (pseudo_rand >> 32) % (inst.lanes as u64);

        if position.pass == 0 && position.slice == 0 {
            ref_lane = position.lane as u64;
        }

        position.index = i;
        let ref_index = index_alpha(
            inst,
            &position,
            (pseudo_rand & 0xFFFFFFFF) as u32,
            ref_lane == position.lane as u64,
        );

        let ref_block =
            memory.add((inst.lane_length as u64 * ref_lane + ref_index as u64) as usize);
        let curr_block = memory.add(curr_offset as usize);
        if position.pass != 0 {
            fill_block_with_xor(memory.add(prev_offset as usize), ref_block, curr_block);
        } else {
            fill_block(memory.add(prev_offset as usize), ref_block, curr_block);
        }

        i += 1;
        curr_offset += 1;
        prev_offset += 1;
    }
}

unsafe fn argon2_fill_first_blocks(blockhash: *mut u8, instance: *const argon2_instance_t) {
    let inst = &*instance;
    let memory = (*inst.region).memory;
    let mut blockhash_bytes = [0u8; ARGON2_BLOCK_SIZE];
    let mut l: u32 = 0;
    while l < inst.lanes {
        {
            let d = core::slice::from_raw_parts_mut(
                blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH),
                4,
            );
            store32_le(d, 0);
        }
        {
            let d = core::slice::from_raw_parts_mut(
                blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH + 4),
                4,
            );
            store32_le(d, l);
        }
        _sodium_blake2b_long(
            blockhash_bytes.as_mut_ptr() as *mut c_void,
            ARGON2_BLOCK_SIZE,
            blockhash as *const c_void,
            ARGON2_PREHASH_SEED_LENGTH,
        );
        load_block(
            memory.add((l * inst.lane_length + 0) as usize),
            blockhash_bytes.as_ptr(),
        );

        {
            let d = core::slice::from_raw_parts_mut(
                blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH),
                4,
            );
            store32_le(d, 1);
        }
        _sodium_blake2b_long(
            blockhash_bytes.as_mut_ptr() as *mut c_void,
            ARGON2_BLOCK_SIZE,
            blockhash as *const c_void,
            ARGON2_PREHASH_SEED_LENGTH,
        );
        load_block(
            memory.add((l * inst.lane_length + 1) as usize),
            blockhash_bytes.as_ptr(),
        );
        l += 1;
    }
    sodium_memzero(
        blockhash_bytes.as_mut_ptr() as *mut c_void,
        ARGON2_BLOCK_SIZE,
    );
}

unsafe fn argon2_initial_hash(blockhash: *mut u8, context: *mut argon2_context, type_: c_int) {
    if context.is_null() || blockhash.is_null() {
        return;
    }
    let ctx = &mut *context;
    let mut blake = Blake2bState { opaque: [0u8; 384] };
    let mut value = [0u8; 4];

    crypto_generichash_blake2b_init(
        &mut blake,
        core::ptr::null(),
        0,
        ARGON2_PREHASH_DIGEST_LENGTH,
    );

    store32_le(&mut value, ctx.lanes);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    store32_le(&mut value, ctx.outlen);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    store32_le(&mut value, ctx.m_cost);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    store32_le(&mut value, ctx.t_cost);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    store32_le(&mut value, ARGON2_VERSION_NUMBER);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    store32_le(&mut value, type_ as u32);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    store32_le(&mut value, ctx.pwdlen);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    if !ctx.pwd.is_null() {
        crypto_generichash_blake2b_update(&mut blake, ctx.pwd, ctx.pwdlen as u64);
        if ctx.flags & ARGON2_FLAG_CLEAR_PASSWORD != 0 {
            sodium_memzero(ctx.pwd as *mut c_void, ctx.pwdlen as usize);
            ctx.pwdlen = 0;
        }
    }

    store32_le(&mut value, ctx.saltlen);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    if !ctx.salt.is_null() {
        crypto_generichash_blake2b_update(&mut blake, ctx.salt, ctx.saltlen as u64);
    }

    store32_le(&mut value, ctx.secretlen);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    if !ctx.secret.is_null() {
        crypto_generichash_blake2b_update(&mut blake, ctx.secret, ctx.secretlen as u64);
        if ctx.flags & ARGON2_FLAG_CLEAR_SECRET != 0 {
            sodium_memzero(ctx.secret as *mut c_void, ctx.secretlen as usize);
            ctx.secretlen = 0;
        }
    }

    store32_le(&mut value, ctx.adlen);
    crypto_generichash_blake2b_update(&mut blake, value.as_ptr(), 4);

    if !ctx.ad.is_null() {
        crypto_generichash_blake2b_update(&mut blake, ctx.ad, ctx.adlen as u64);
    }

    crypto_generichash_blake2b_final(&mut blake, blockhash, ARGON2_PREHASH_DIGEST_LENGTH);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_initialize(
    instance: *mut argon2_instance_t,
    context: *mut argon2_context,
) -> c_int {
    let mut blockhash = [0u8; ARGON2_PREHASH_SEED_LENGTH];

    if instance.is_null() || context.is_null() {
        return ARGON2_INCORRECT_PARAMETER;
    }
    let inst = &mut *instance;

    inst.pseudo_rands =
        libc::malloc(8usize.wrapping_mul(inst.segment_length as usize)) as *mut u64;
    if inst.pseudo_rands.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    let result = allocate_memory(&mut inst.region, inst.memory_blocks);
    if ARGON2_OK != result {
        argon2_free_instance(instance, (*context).flags);
        return result;
    }

    argon2_initial_hash(blockhash.as_mut_ptr(), context, inst.type_);
    sodium_memzero(
        blockhash.as_mut_ptr().add(ARGON2_PREHASH_DIGEST_LENGTH) as *mut c_void,
        ARGON2_PREHASH_SEED_LENGTH - ARGON2_PREHASH_DIGEST_LENGTH,
    );

    argon2_fill_first_blocks(blockhash.as_mut_ptr(), instance);
    sodium_memzero(
        blockhash.as_mut_ptr() as *mut c_void,
        ARGON2_PREHASH_SEED_LENGTH,
    );

    ARGON2_OK
}

// fill_segment function pointer (only reference implementation exists).
type FillSegmentFn = unsafe extern "C" fn(*const argon2_instance_t, argon2_position_t);
static mut FILL_SEGMENT: FillSegmentFn = _sodium_argon2_fill_segment_ref;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_pwhash_argon2_pick_best_implementation() -> c_int {
    FILL_SEGMENT = _sodium_argon2_fill_segment_ref;
    0
}

// ---------------------------------------------------------------------------
// argon2.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_ctx(
    context: *mut argon2_context,
    type_: c_int,
) -> c_int {
    let mut result = _sodium_argon2_validate_inputs(context);
    if ARGON2_OK != result {
        return result;
    }
    if type_ != ARGON2_ID && type_ != ARGON2_I {
        return ARGON2_INCORRECT_TYPE;
    }

    let ctx = &*context;
    let mut memory_blocks: u32 = ctx.m_cost;
    if memory_blocks < 2 * ARGON2_SYNC_POINTS * ctx.lanes {
        memory_blocks = 2 * ARGON2_SYNC_POINTS * ctx.lanes;
    }
    let segment_length: u32 = memory_blocks / (ctx.lanes * ARGON2_SYNC_POINTS);
    memory_blocks = segment_length * (ctx.lanes * ARGON2_SYNC_POINTS);

    let mut instance = argon2_instance_t {
        region: core::ptr::null_mut(),
        pseudo_rands: core::ptr::null_mut(),
        passes: ctx.t_cost,
        current_pass: !0u32,
        memory_blocks,
        segment_length,
        lane_length: segment_length * ARGON2_SYNC_POINTS,
        lanes: ctx.lanes,
        threads: ctx.threads,
        type_,
        print_internals: 0,
    };

    result = _sodium_argon2_initialize(&mut instance, context);
    if ARGON2_OK != result {
        return result;
    }

    let mut pass: u32 = 0;
    while pass < instance.passes {
        _sodium_argon2_fill_memory_blocks(&mut instance, pass);
        pass += 1;
    }

    _sodium_argon2_finalize(context, &mut instance);

    ARGON2_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_hash(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
    type_: c_int,
) -> c_int {
    if !hash.is_null() {
        randombytes_buf(hash, hashlen);
    }
    if pwdlen > ARGON2_MAX_PWD_LENGTH as usize {
        return ARGON2_PWD_TOO_LONG;
    }
    if hashlen > ARGON2_MAX_OUTLEN as usize {
        return ARGON2_OUTPUT_TOO_LONG;
    }
    if saltlen > ARGON2_MAX_SALT_LENGTH as usize {
        return ARGON2_SALT_TOO_LONG;
    }

    let out = libc::malloc(hashlen) as *mut u8;
    if out.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    let mut context = argon2_context {
        out,
        outlen: hashlen as u32,
        pwd: pwd as *mut u8,
        pwdlen: pwdlen as u32,
        salt: salt as *mut u8,
        saltlen: saltlen as u32,
        secret: core::ptr::null_mut(),
        secretlen: 0,
        ad: core::ptr::null_mut(),
        adlen: 0,
        t_cost,
        m_cost,
        lanes: parallelism,
        threads: parallelism,
        flags: ARGON2_DEFAULT_FLAGS,
    };

    let result = _sodium_argon2_ctx(&mut context, type_);

    if result != ARGON2_OK {
        sodium_memzero(out as *mut c_void, hashlen);
        libc::free(out as *mut c_void);
        return result;
    }

    if !encoded.is_null() && encodedlen != 0 {
        if _sodium_argon2_encode_string(encoded, encodedlen, &mut context, type_) != ARGON2_OK {
            sodium_memzero(out as *mut c_void, hashlen);
            sodium_memzero(encoded as *mut c_void, encodedlen);
            libc::free(out as *mut c_void);
            return ARGON2_ENCODING_FAIL;
        }
    }

    if !hash.is_null() {
        core::ptr::copy_nonoverlapping(out, hash as *mut u8, hashlen);
    }

    sodium_memzero(out as *mut c_void, hashlen);
    libc::free(out as *mut c_void);

    ARGON2_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_hash_encoded(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        core::ptr::null_mut(),
        hashlen,
        encoded,
        encodedlen,
        ARGON2_I,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_hash_raw(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        hash,
        hashlen,
        core::ptr::null_mut(),
        0,
        ARGON2_I,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_hash_encoded(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        core::ptr::null_mut(),
        hashlen,
        encoded,
        encodedlen,
        ARGON2_ID,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_hash_raw(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        hash,
        hashlen,
        core::ptr::null_mut(),
        0,
        ARGON2_ID,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
    type_: c_int,
) -> c_int {
    let mut ctx: argon2_context = core::mem::zeroed();

    ctx.pwd = core::ptr::null_mut();
    ctx.pwdlen = 0;
    ctx.secret = core::ptr::null_mut();
    ctx.secretlen = 0;

    let encoded_len = c_strlen(encoded as *const u8);
    if encoded_len > u32::MAX as usize {
        return ARGON2_DECODING_LENGTH_FAIL;
    }
    ctx.adlen = encoded_len as u32;
    ctx.saltlen = encoded_len as u32;
    ctx.outlen = encoded_len as u32;

    ctx.ad = libc::malloc(ctx.adlen as usize) as *mut u8;
    ctx.salt = libc::malloc(ctx.saltlen as usize) as *mut u8;
    ctx.out = libc::malloc(ctx.outlen as usize) as *mut u8;
    if ctx.out.is_null() || ctx.salt.is_null() || ctx.ad.is_null() {
        libc::free(ctx.ad as *mut c_void);
        libc::free(ctx.salt as *mut c_void);
        libc::free(ctx.out as *mut c_void);
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    let out = libc::malloc(ctx.outlen as usize) as *mut u8;
    if out.is_null() {
        libc::free(ctx.ad as *mut c_void);
        libc::free(ctx.salt as *mut c_void);
        libc::free(ctx.out as *mut c_void);
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    let decode_result = _sodium_argon2_decode_string(&mut ctx, encoded, type_);
    if decode_result != ARGON2_OK {
        libc::free(ctx.ad as *mut c_void);
        libc::free(ctx.salt as *mut c_void);
        libc::free(ctx.out as *mut c_void);
        libc::free(out as *mut c_void);
        return decode_result;
    }

    let mut ret = _sodium_argon2_hash(
        ctx.t_cost,
        ctx.m_cost,
        ctx.threads,
        pwd,
        pwdlen,
        ctx.salt as *const c_void,
        ctx.saltlen as usize,
        out as *mut c_void,
        ctx.outlen as usize,
        core::ptr::null_mut(),
        0,
        type_,
    );

    libc::free(ctx.ad as *mut c_void);
    libc::free(ctx.salt as *mut c_void);

    if ret == ARGON2_OK
        && sodium_memcmp(out as *const c_void, ctx.out as *const c_void, ctx.outlen as usize) != 0
    {
        ret = ARGON2_VERIFY_MISMATCH;
    }
    libc::free(out as *mut c_void);
    libc::free(ctx.out as *mut c_void);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, ARGON2_I)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, ARGON2_ID)
}

// ---------------------------------------------------------------------------
// argon2-encoding.c
// ---------------------------------------------------------------------------

unsafe fn c_strlen(mut p: *const u8) -> usize {
    let mut n = 0usize;
    while *p != 0 {
        n += 1;
        p = p.add(1);
    }
    n
}

/// Returns true if `p` starts with `prefix`.
unsafe fn starts_with(p: *const u8, prefix: &[u8]) -> bool {
    for (i, &b) in prefix.iter().enumerate() {
        if *p.add(i) != b {
            return false;
        }
    }
    true
}

fn decode_decimal(str_: *const u8, v: &mut u64) -> *const u8 {
    unsafe {
        let mut acc: u64 = 0;
        let orig = str_;
        let mut s = str_;
        loop {
            let c = *s as i32;
            if c < b'0' as i32 || c > b'9' as i32 {
                break;
            }
            let c = c - b'0' as i32;
            if acc > (u64::MAX / 10) {
                return core::ptr::null();
            }
            acc *= 10;
            if (c as u64) > (u64::MAX - acc) {
                return core::ptr::null();
            }
            acc += c as u64;
            s = s.add(1);
        }
        if s == orig || (*orig == b'0' && s != orig.add(1)) {
            return core::ptr::null();
        }
        *v = acc;
        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_decode_string(
    ctx: *mut argon2_context,
    str_: *const c_char,
    type_: c_int,
) -> c_int {
    let ctxr = &mut *ctx;
    let mut str_ = str_ as *const u8;

    let maxsaltlen = ctxr.saltlen as usize;
    let maxoutlen = ctxr.outlen as usize;
    let mut version: u32 = 0;

    ctxr.saltlen = 0;
    ctxr.outlen = 0;

    // CC prefix helper (returns false -> DECODING_FAIL)
    macro_rules! cc {
        ($prefix:expr) => {{
            let prefix: &[u8] = $prefix;
            if !starts_with(str_, prefix) {
                return ARGON2_DECODING_FAIL;
            }
            str_ = str_.add(prefix.len());
        }};
    }

    macro_rules! decimal_u32 {
        ($x:expr) => {{
            let mut dec_x: u64 = 0;
            str_ = decode_decimal(str_, &mut dec_x);
            if str_.is_null() || dec_x > u32::MAX as u64 {
                return ARGON2_DECODING_FAIL;
            }
            $x = dec_x as u32;
        }};
    }

    if type_ == ARGON2_ID {
        cc!(b"$argon2id");
    } else if type_ == ARGON2_I {
        cc!(b"$argon2i");
    } else {
        return ARGON2_INCORRECT_TYPE;
    }
    cc!(b"$v=");
    decimal_u32!(version);
    if version != ARGON2_VERSION_NUMBER {
        return ARGON2_INCORRECT_TYPE;
    }
    cc!(b"$m=");
    decimal_u32!(ctxr.m_cost);
    cc!(b",t=");
    decimal_u32!(ctxr.t_cost);
    cc!(b",p=");
    decimal_u32!(ctxr.lanes);
    ctxr.threads = ctxr.lanes;

    // BIN(salt)
    cc!(b"$");
    {
        let mut bin_len: usize = maxsaltlen;
        let mut str_end: *const c_char = core::ptr::null();
        if sodium_base642bin(
            ctxr.salt,
            maxsaltlen,
            str_ as *const c_char,
            c_strlen(str_),
            core::ptr::null(),
            &mut bin_len,
            &mut str_end,
            SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING,
        ) != 0
            || bin_len > u32::MAX as usize
        {
            return ARGON2_DECODING_FAIL;
        }
        ctxr.saltlen = bin_len as u32;
        str_ = str_end as *const u8;
    }
    cc!(b"$");
    {
        let mut bin_len: usize = maxoutlen;
        let mut str_end: *const c_char = core::ptr::null();
        if sodium_base642bin(
            ctxr.out,
            maxoutlen,
            str_ as *const c_char,
            c_strlen(str_),
            core::ptr::null(),
            &mut bin_len,
            &mut str_end,
            SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING,
        ) != 0
            || bin_len > u32::MAX as usize
        {
            return ARGON2_DECODING_FAIL;
        }
        ctxr.outlen = bin_len as u32;
        str_ = str_end as *const u8;
    }
    let validation_result = _sodium_argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result;
    }
    if *str_ == 0 {
        return ARGON2_OK;
    }
    ARGON2_DECODING_FAIL
}

const U32_STR_MAXSIZE: usize = 11;

fn u32_to_string(str_: *mut u8, mut x: u32) {
    unsafe {
        let mut tmp = [0u8; U32_STR_MAXSIZE - 1]; // 10
        let mut i = tmp.len();
        loop {
            i -= 1;
            tmp[i] = ((x % 10) as u8) + b'0';
            x /= 10;
            if x == 0 || i == 0 {
                break;
            }
        }
        let len = tmp.len() - i;
        core::ptr::copy_nonoverlapping(tmp.as_ptr().add(i), str_, len);
        *str_.add(len) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_encode_string(
    dst: *mut c_char,
    dst_len: usize,
    ctx: *mut argon2_context,
    type_: c_int,
) -> c_int {
    let mut dst = dst as *mut u8;
    let mut dst_len = dst_len;

    // SS: copy a NUL-terminated byte string (bytes must include terminator handling).
    macro_rules! ss {
        ($bytes:expr) => {{
            let s: &[u8] = $bytes;
            let pp_len = s.len();
            if pp_len >= dst_len {
                return ARGON2_ENCODING_FAIL;
            }
            core::ptr::copy_nonoverlapping(s.as_ptr(), dst, pp_len);
            *dst.add(pp_len) = 0;
            dst = dst.add(pp_len);
            dst_len -= pp_len;
        }};
    }

    macro_rules! sx {
        ($x:expr) => {{
            let mut tmp = [0u8; U32_STR_MAXSIZE];
            u32_to_string(tmp.as_mut_ptr(), $x);
            let l = c_strlen(tmp.as_ptr());
            let s: &[u8] = core::slice::from_raw_parts(tmp.as_ptr(), l);
            // replicate SS on this slice
            let pp_len = s.len();
            if pp_len >= dst_len {
                return ARGON2_ENCODING_FAIL;
            }
            core::ptr::copy_nonoverlapping(s.as_ptr(), dst, pp_len);
            *dst.add(pp_len) = 0;
            dst = dst.add(pp_len);
            dst_len -= pp_len;
        }};
    }

    macro_rules! sb {
        ($buf:expr, $len:expr) => {{
            if sodium_bin2base64(
                dst as *mut c_char,
                dst_len,
                $buf,
                $len,
                SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING,
            )
            .is_null()
            {
                return ARGON2_ENCODING_FAIL;
            }
            let sb_len = c_strlen(dst);
            dst = dst.add(sb_len);
            dst_len -= sb_len;
        }};
    }

    match type_ {
        ARGON2_ID => ss!(b"$argon2id$v="),
        ARGON2_I => ss!(b"$argon2i$v="),
        _ => return ARGON2_ENCODING_FAIL,
    }

    let validation_result = _sodium_argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result;
    }
    let ctxr = &*ctx;
    sx!(ARGON2_VERSION_NUMBER);
    ss!(b"$m=");
    sx!(ctxr.m_cost);
    ss!(b",t=");
    sx!(ctxr.t_cost);
    ss!(b",p=");
    sx!(ctxr.lanes);

    ss!(b"$");
    sb!(ctxr.salt, ctxr.saltlen as usize);

    ss!(b"$");
    sb!(ctxr.out, ctxr.outlen as usize);

    ARGON2_OK
}

// ---------------------------------------------------------------------------
// Public constants for pwhash_argon2i / pwhash_argon2id (crypto_pwhash_argon2*.h)
// ---------------------------------------------------------------------------

pub const ARGON2I_ALG_ARGON2I13: c_int = 1;
pub const ARGON2ID_ALG_ARGON2ID13: c_int = 2;

pub const ARGON2_BYTES_MIN: usize = 16;
pub const ARGON2_BYTES_MAX: usize = 4294967295;
pub const ARGON2_PASSWD_MIN: usize = 0;
pub const ARGON2_PASSWD_MAX: usize = 4294967295;
pub const ARGON2_SALTBYTES: usize = 16;
pub const ARGON2_STRBYTES: usize = 128;
pub const ARGON2_OPSLIMIT_MAX: u64 = 4294967295;
pub const ARGON2_MEMLIMIT_MIN: usize = 8192;
pub const ARGON2_MEMLIMIT_MAX: usize = 4398046510080;

pub const ARGON2I_STRPREFIX: &[u8] = b"$argon2i$\0";
pub const ARGON2ID_STRPREFIX: &[u8] = b"$argon2id$\0";

// ---------------------------------------------------------------------------
// pwhash_argon2i.c
// ---------------------------------------------------------------------------

const STR_HASHBYTES: usize = 32;

const ARGON2I_OPSLIMIT_MIN: u64 = 3;
const ARGON2I_MEMLIMIT_MIN: usize = 8192;

const ARGON2ID_OPSLIMIT_MIN: u64 = 1;
const ARGON2ID_MEMLIMIT_MIN: usize = 8192;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_alg_argon2i13() -> c_int {
    ARGON2I_ALG_ARGON2I13
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_bytes_min() -> usize {
    ARGON2_BYTES_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_bytes_max() -> usize {
    ARGON2_BYTES_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_passwd_min() -> usize {
    ARGON2_PASSWD_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_passwd_max() -> usize {
    ARGON2_PASSWD_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_saltbytes() -> usize {
    ARGON2_SALTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_strbytes() -> usize {
    ARGON2_STRBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_strprefix() -> *const c_char {
    ARGON2I_STRPREFIX.as_ptr() as *const c_char
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_min() -> u64 {
    ARGON2I_OPSLIMIT_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_max() -> u64 {
    ARGON2_OPSLIMIT_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_min() -> usize {
    ARGON2_MEMLIMIT_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_max() -> usize {
    ARGON2_MEMLIMIT_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_interactive() -> u64 {
    4
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_interactive() -> usize {
    33554432
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_moderate() -> u64 {
    6
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_moderate() -> usize {
    134217728
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_sensitive() -> u64 {
    8
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_sensitive() -> usize {
    536870912
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    core::ptr::write_bytes(out, 0, outlen as usize);
    if outlen > ARGON2_BYTES_MAX as u64 {
        set_errno(libc::EFBIG);
        return -1;
    }
    if outlen < ARGON2_BYTES_MIN as u64 {
        set_errno(libc::EINVAL);
        return -1;
    }
    if passwdlen > ARGON2_PASSWD_MAX as u64
        || opslimit > ARGON2_OPSLIMIT_MAX
        || memlimit > ARGON2_MEMLIMIT_MAX
    {
        set_errno(libc::EFBIG);
        return -1;
    }
    if passwdlen < ARGON2_PASSWD_MIN as u64
        || opslimit < ARGON2I_OPSLIMIT_MIN
        || memlimit < ARGON2I_MEMLIMIT_MIN
    {
        set_errno(libc::EINVAL);
        return -1;
    }
    if out as *const c_void == passwd as *const c_void {
        set_errno(libc::EINVAL);
        return -1;
    }
    match alg {
        ARGON2I_ALG_ARGON2I13 => {
            if _sodium_argon2i_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                ARGON2_SALTBYTES,
                out as *mut c_void,
                outlen as usize,
            ) != ARGON2_OK
            {
                return -1;
            }
            0
        }
        _ => {
            set_errno(libc::EINVAL);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt = [0u8; ARGON2_SALTBYTES];

    core::ptr::write_bytes(out, 0, ARGON2_STRBYTES);
    if passwdlen > ARGON2_PASSWD_MAX as u64
        || opslimit > ARGON2_OPSLIMIT_MAX
        || memlimit > ARGON2_MEMLIMIT_MAX
    {
        set_errno(libc::EFBIG);
        return -1;
    }
    if passwdlen < ARGON2_PASSWD_MIN as u64
        || opslimit < ARGON2I_OPSLIMIT_MIN
        || memlimit < ARGON2I_MEMLIMIT_MIN
    {
        set_errno(libc::EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, ARGON2_SALTBYTES);
    if _sodium_argon2i_hash_encoded(
        opslimit as u32,
        (memlimit / 1024) as u32,
        1,
        passwd as *const c_void,
        passwdlen as usize,
        salt.as_ptr() as *const c_void,
        ARGON2_SALTBYTES,
        STR_HASHBYTES,
        out,
        ARGON2_STRBYTES,
    ) != ARGON2_OK
    {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    if passwdlen > ARGON2_PASSWD_MAX as u64 {
        set_errno(libc::EFBIG);
        return -1;
    }
    if passwdlen < ARGON2_PASSWD_MIN as u64 {
        set_errno(libc::EINVAL);
        return -1;
    }
    let verify_ret = _sodium_argon2i_verify(str_, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        set_errno(libc::EINVAL);
    }
    -1
}

const CRYPTO_PWHASH_STRBYTES: usize = 128;

unsafe fn needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    mut memlimit: usize,
    type_: c_int,
) -> c_int {
    let mut ctx: argon2_context = core::mem::zeroed();
    let mut ret: c_int;

    let fodder_len = c_strlen(str_ as *const u8);
    memlimit /= 1024;
    if opslimit > u32::MAX as u64
        || memlimit > u32::MAX as usize
        || fodder_len >= CRYPTO_PWHASH_STRBYTES
    {
        set_errno(libc::EINVAL);
        return -1;
    }
    // memset(&ctx, 0) already via zeroed().
    let fodder = libc::calloc(fodder_len, 1) as *mut u8;
    if fodder.is_null() {
        return -1;
    }
    ctx.out = fodder;
    ctx.pwd = fodder;
    ctx.salt = fodder;
    ctx.outlen = fodder_len as u32;
    ctx.pwdlen = fodder_len as u32;
    ctx.saltlen = fodder_len as u32;
    ctx.ad = core::ptr::null_mut();
    ctx.secret = core::ptr::null_mut();
    ctx.adlen = 0;
    ctx.secretlen = 0;
    if _sodium_argon2_decode_string(&mut ctx, str_, type_) != 0 {
        set_errno(libc::EINVAL);
        ret = -1;
    } else if ctx.t_cost != opslimit as u32 || ctx.m_cost != memlimit as u32 {
        ret = 1;
    } else {
        ret = 0;
    }
    libc::free(fodder as *mut c_void);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    needs_rehash(str_, opslimit, memlimit, ARGON2_I)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    needs_rehash(str_, opslimit, memlimit, ARGON2_ID)
}

// ---------------------------------------------------------------------------
// pwhash_argon2id.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_alg_argon2id13() -> c_int {
    ARGON2ID_ALG_ARGON2ID13
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_bytes_min() -> usize {
    ARGON2_BYTES_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_bytes_max() -> usize {
    ARGON2_BYTES_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_passwd_min() -> usize {
    ARGON2_PASSWD_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_passwd_max() -> usize {
    ARGON2_PASSWD_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_saltbytes() -> usize {
    ARGON2_SALTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_strbytes() -> usize {
    ARGON2_STRBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_strprefix() -> *const c_char {
    ARGON2ID_STRPREFIX.as_ptr() as *const c_char
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_min() -> u64 {
    ARGON2ID_OPSLIMIT_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_max() -> u64 {
    ARGON2_OPSLIMIT_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_min() -> usize {
    ARGON2_MEMLIMIT_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_max() -> usize {
    ARGON2_MEMLIMIT_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_interactive() -> u64 {
    2
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_interactive() -> usize {
    67108864
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_moderate() -> u64 {
    3
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_moderate() -> usize {
    268435456
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_sensitive() -> u64 {
    4
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_sensitive() -> usize {
    1073741824
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    core::ptr::write_bytes(out, 0, outlen as usize);
    if outlen > ARGON2_BYTES_MAX as u64 {
        set_errno(libc::EFBIG);
        return -1;
    }
    if outlen < ARGON2_BYTES_MIN as u64 {
        set_errno(libc::EINVAL);
        return -1;
    }
    if passwdlen > ARGON2_PASSWD_MAX as u64
        || opslimit > ARGON2_OPSLIMIT_MAX
        || memlimit > ARGON2_MEMLIMIT_MAX
    {
        set_errno(libc::EFBIG);
        return -1;
    }
    if passwdlen < ARGON2_PASSWD_MIN as u64
        || opslimit < ARGON2ID_OPSLIMIT_MIN
        || memlimit < ARGON2ID_MEMLIMIT_MIN
    {
        set_errno(libc::EINVAL);
        return -1;
    }
    if out as *const c_void == passwd as *const c_void {
        set_errno(libc::EINVAL);
        return -1;
    }
    match alg {
        ARGON2ID_ALG_ARGON2ID13 => {
            if _sodium_argon2id_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                ARGON2_SALTBYTES,
                out as *mut c_void,
                outlen as usize,
            ) != ARGON2_OK
            {
                return -1;
            }
            0
        }
        _ => {
            set_errno(libc::EINVAL);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt = [0u8; ARGON2_SALTBYTES];

    core::ptr::write_bytes(out, 0, ARGON2_STRBYTES);
    if passwdlen > ARGON2_PASSWD_MAX as u64
        || opslimit > ARGON2_OPSLIMIT_MAX
        || memlimit > ARGON2_MEMLIMIT_MAX
    {
        set_errno(libc::EFBIG);
        return -1;
    }
    if passwdlen < ARGON2_PASSWD_MIN as u64
        || opslimit < ARGON2ID_OPSLIMIT_MIN
        || memlimit < ARGON2ID_MEMLIMIT_MIN
    {
        set_errno(libc::EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, ARGON2_SALTBYTES);
    if _sodium_argon2id_hash_encoded(
        opslimit as u32,
        (memlimit / 1024) as u32,
        1,
        passwd as *const c_void,
        passwdlen as usize,
        salt.as_ptr() as *const c_void,
        ARGON2_SALTBYTES,
        STR_HASHBYTES,
        out,
        ARGON2_STRBYTES,
    ) != ARGON2_OK
    {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    if passwdlen > ARGON2_PASSWD_MAX as u64 {
        set_errno(libc::EFBIG);
        return -1;
    }
    if passwdlen < ARGON2_PASSWD_MIN as u64 {
        set_errno(libc::EINVAL);
        return -1;
    }
    let verify_ret = _sodium_argon2id_verify(str_, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        set_errno(libc::EINVAL);
    }
    -1
}
