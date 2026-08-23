// Translation of crypto_ipcrypt/crypto_ipcrypt.c and crypto_ipcrypt/ipcrypt_soft.c
// (soft/portable implementation).

use core::ffi::{c_int, c_void};

use super::softaes::{
    block_load, block_store, block_xor, SoftAesBlock, _sodium_softaes_block_decrypt,
    _sodium_softaes_block_decryptlast, _sodium_softaes_block_encrypt,
    _sodium_softaes_block_encryptlast, _sodium_softaes_expand_key128,
    _sodium_softaes_inv_mix_columns, _sodium_softaes_invert_key_schedule128,
};

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

const CRYPTO_IPCRYPT_BYTES: usize = 16;
const CRYPTO_IPCRYPT_KEYBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_KEYBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_TWEAKBYTES: usize = 8;
const CRYPTO_IPCRYPT_ND_INPUTBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_OUTPUTBYTES: usize = 24;
const CRYPTO_IPCRYPT_NDX_KEYBYTES: usize = 32;
const CRYPTO_IPCRYPT_NDX_TWEAKBYTES: usize = 16;
const CRYPTO_IPCRYPT_NDX_INPUTBYTES: usize = 16;
const CRYPTO_IPCRYPT_NDX_OUTPUTBYTES: usize = 32;
const CRYPTO_IPCRYPT_PFX_KEYBYTES: usize = 32;
const CRYPTO_IPCRYPT_PFX_BYTES: usize = 16;

const ROUNDS: usize = 10;

type KeySchedule = [SoftAesBlock; 1 + ROUNDS];

const ZERO_BLOCK: SoftAesBlock = SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 };

#[inline]
fn expand_key(rkeys: &mut KeySchedule, key: &[u8]) {
    unsafe {
        _sodium_softaes_expand_key128(rkeys.as_mut_ptr(), key.as_ptr());
    }
}

fn aes_encrypt(out: &mut [u8], inp: &[u8], rkeys: &KeySchedule) {
    let mut t = block_xor(block_load(inp), rkeys[0]);
    for i in 1..ROUNDS {
        t = _sodium_softaes_block_encrypt(t, rkeys[i]);
    }
    t = _sodium_softaes_block_encryptlast(t, rkeys[ROUNDS]);
    block_store(out, t);
}

fn aes_decrypt(out: &mut [u8], inp: &[u8], rkeys: &KeySchedule) {
    let mut rkeys_inv: KeySchedule = *rkeys;
    unsafe {
        _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());
    }

    let mut t = block_xor(block_load(inp), rkeys_inv[ROUNDS]);
    let mut i = ROUNDS - 1;
    while i > 0 {
        t = _sodium_softaes_block_decrypt(t, rkeys_inv[i]);
        i -= 1;
    }
    t = _sodium_softaes_block_decryptlast(t, rkeys_inv[0]);
    block_store(out, t);
    unsafe {
        sodium_memzero(rkeys_inv.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    }
}

fn tweak_expand(tweak: &[u8]) -> SoftAesBlock {
    SoftAesBlock {
        w0: (tweak[0] as u32) | ((tweak[1] as u32) << 8),
        w1: (tweak[2] as u32) | ((tweak[3] as u32) << 8),
        w2: (tweak[4] as u32) | ((tweak[5] as u32) << 8),
        w3: (tweak[6] as u32) | ((tweak[7] as u32) << 8),
    }
}

fn aes_encrypt_with_tweak(out: &mut [u8], inp: &[u8], tweak: &[u8], rkeys: &KeySchedule) {
    let tweak_block = tweak_expand(tweak);
    let mut t = block_xor(block_xor(block_load(inp), tweak_block), rkeys[0]);
    for i in 1..ROUNDS {
        t = _sodium_softaes_block_encrypt(t, block_xor(tweak_block, rkeys[i]));
    }
    t = _sodium_softaes_block_encryptlast(t, block_xor(tweak_block, rkeys[ROUNDS]));
    block_store(out, t);
}

fn aes_decrypt_with_tweak(out: &mut [u8], inp: &[u8], tweak: &[u8], rkeys: &KeySchedule) {
    let mut rkeys_inv: KeySchedule = *rkeys;
    let tweak_block = tweak_expand(tweak);
    let tweak_block_inv = _sodium_softaes_inv_mix_columns(tweak_block);
    unsafe {
        _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());
    }

    let mut t = block_xor(block_xor(block_load(inp), tweak_block), rkeys_inv[ROUNDS]);
    let mut i = ROUNDS - 1;
    while i > 0 {
        t = _sodium_softaes_block_decrypt(t, block_xor(tweak_block_inv, rkeys_inv[i]));
        i -= 1;
    }
    t = _sodium_softaes_block_decryptlast(t, block_xor(tweak_block, rkeys_inv[0]));
    block_store(out, t);
    unsafe {
        sodium_memzero(rkeys_inv.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    }
}

fn aes_xex_tweak(tweak: &[u8], tkeys: &KeySchedule) -> SoftAesBlock {
    let mut tt = block_xor(block_load(tweak), tkeys[0]);
    for i in 1..ROUNDS {
        tt = _sodium_softaes_block_encrypt(tt, tkeys[i]);
    }
    tt = _sodium_softaes_block_encryptlast(tt, tkeys[ROUNDS]);
    tt
}

fn aes_xex_encrypt(out: &mut [u8], inp: &[u8], tweak: &[u8], tkeys: &KeySchedule, rkeys: &KeySchedule) {
    let tt = aes_xex_tweak(tweak, tkeys);
    let mut t = block_xor(block_xor(block_load(inp), tt), rkeys[0]);
    for i in 1..ROUNDS {
        t = _sodium_softaes_block_encrypt(t, rkeys[i]);
    }
    t = _sodium_softaes_block_encryptlast(t, block_xor(rkeys[ROUNDS], tt));
    block_store(out, t);
}

fn aes_xex_decrypt(out: &mut [u8], inp: &[u8], tweak: &[u8], tkeys: &KeySchedule, rkeys: &KeySchedule) {
    let mut rkeys_inv: KeySchedule = *rkeys;
    let tt = aes_xex_tweak(tweak, tkeys);
    unsafe {
        _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());
    }

    let mut t = block_xor(block_xor(block_load(inp), tt), rkeys_inv[ROUNDS]);
    let mut i = ROUNDS - 1;
    while i > 0 {
        t = _sodium_softaes_block_decrypt(t, rkeys_inv[i]);
        i -= 1;
    }
    t = _sodium_softaes_block_decryptlast(t, block_xor(rkeys_inv[0], tt));
    block_store(out, t);
    unsafe {
        sodium_memzero(rkeys_inv.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    }
}

// --- implementation function-pointer targets (C linkage, unexported) ---

unsafe extern "C" fn imp_encrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    expand_key(&mut rkeys, core::slice::from_raw_parts(k, 16));
    aes_encrypt(
        core::slice::from_raw_parts_mut(out, 16),
        core::slice::from_raw_parts(inp, 16),
        &rkeys,
    );
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn imp_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    expand_key(&mut rkeys, core::slice::from_raw_parts(k, 16));
    aes_decrypt(
        core::slice::from_raw_parts_mut(out, 16),
        core::slice::from_raw_parts(inp, 16),
        &rkeys,
    );
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn imp_nd_encrypt(out: *mut u8, inp: *const u8, t: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    expand_key(&mut rkeys, core::slice::from_raw_parts(k, 16));
    core::ptr::copy_nonoverlapping(t, out, 8);
    aes_encrypt_with_tweak(
        core::slice::from_raw_parts_mut(out.add(8), 16),
        core::slice::from_raw_parts(inp, 16),
        core::slice::from_raw_parts(t, 8),
        &rkeys,
    );
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn imp_nd_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    expand_key(&mut rkeys, core::slice::from_raw_parts(k, 16));
    aes_decrypt_with_tweak(
        core::slice::from_raw_parts_mut(out, 16),
        core::slice::from_raw_parts(inp.add(8), 16),
        core::slice::from_raw_parts(inp, 8),
        &rkeys,
    );
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe fn ndx_xex_setup(tkeys: &mut KeySchedule, rkeys: &mut KeySchedule, k: *const u8) {
    expand_key(tkeys, core::slice::from_raw_parts(k.add(16), 16));
    expand_key(rkeys, core::slice::from_raw_parts(k, 16));

    let mut diff = [0u8; 16];
    block_store(&mut diff, block_xor(tkeys[ROUNDS / 2], rkeys[ROUNDS / 2]));
    let mut d: u8 = 0;
    for i in 0..16 {
        d |= diff[i];
    }
    if d == 0 {
        let ks = core::slice::from_raw_parts(k, 16);
        for i in 0..16 {
            diff[i] = ks[i] ^  0x5a;
        }
        expand_key(rkeys, &diff);
    }
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, diff.len());
}

unsafe extern "C" fn imp_ndx_encrypt(out: *mut u8, inp: *const u8, t: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    ndx_xex_setup(&mut tkeys, &mut rkeys, k);

    core::ptr::copy_nonoverlapping(t, out, 16);
    aes_xex_encrypt(
        core::slice::from_raw_parts_mut(out.add(16), 16),
        core::slice::from_raw_parts(inp, 16),
        core::slice::from_raw_parts(t, 16),
        &tkeys,
        &rkeys,
    );
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    sodium_memzero(tkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn imp_ndx_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    ndx_xex_setup(&mut tkeys, &mut rkeys, k);

    aes_xex_decrypt(
        core::slice::from_raw_parts_mut(out, 16),
        core::slice::from_raw_parts(inp.add(16), 16),
        core::slice::from_raw_parts(inp, 16),
        &tkeys,
        &rkeys,
    );
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    sodium_memzero(tkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

// --- prefix-preserving (pfx) ---

fn is_ipv4_mapped(ip16: &[u8]) -> bool {
    const PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];
    ip16[..12] == PREFIX
}

fn pfx_get_bit(ip16: &[u8], bit_index: u32) -> u8 {
    (ip16[15 - (bit_index / 8) as usize] >> (bit_index % 8)) & 1
}

fn pfx_set_bit(ip16: &mut [u8], bit_index: u32, bit_value: u8) {
    let byte_index = 15 - (bit_index / 8) as usize;
    let bit_mask: u8 = 1u8 << (bit_index % 8);
    let mask: u8 = (0u8).wrapping_sub(bit_value & 1);
    ip16[byte_index] = (ip16[byte_index] & !bit_mask) | (bit_mask & mask);
}

fn pfx_shift_left(ip16: &mut [u8]) {
    for i in 0..15 {
        ip16[i] = (ip16[i] << 1) | (ip16[i + 1] >> 7);
    }
    ip16[15] <<= 1;
}

fn pfx_pad_prefix(padded_prefix: &mut [u8; 16], prefix_len_bits: u32) {
    *padded_prefix = [0u8; 16];
    if prefix_len_bits == 0 {
        padded_prefix[15] = 0x01;
    } else {
        padded_prefix[3] = 0x01;
        padded_prefix[14] = 0xff;
        padded_prefix[15] = 0xff;
    }
}

unsafe fn pfx_setup(k1keys: &mut KeySchedule, k2keys: &mut KeySchedule, k: *const u8) {
    expand_key(k1keys, core::slice::from_raw_parts(k, 16));
    expand_key(k2keys, core::slice::from_raw_parts(k.add(16), 16));

    let mut diff = [0u8; 16];
    block_store(&mut diff, block_xor(k1keys[ROUNDS / 2], k2keys[ROUNDS / 2]));
    let mut d: u8 = 0;
    for i in 0..16 {
        d |= diff[i];
    }
    if d == 0 {
        let ks = core::slice::from_raw_parts(k, 16);
        for i in 0..16 {
            diff[i] = ks[i] ^ 0x5a;
        }
        expand_key(k2keys, &diff);
    }
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, diff.len());
}

unsafe extern "C" fn imp_pfx_encrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    pfx_setup(&mut k1keys, &mut k2keys, k);

    let in_slice = core::slice::from_raw_parts(inp, 16);

    let mut prefix_start: u32 = 0;
    if is_ipv4_mapped(in_slice) {
        prefix_start = 96;
    }

    let mut padded_prefix = [0u8; 16];
    pfx_pad_prefix(&mut padded_prefix, prefix_start);

    let mut encrypted = [0u8; 16];
    if prefix_start == 96 {
        encrypted[10] = 0xff;
        encrypted[11] = 0xff;
    }

    let mut t = [0u8; 16];
    let mut prefix_len_bits = prefix_start;
    while prefix_len_bits < 128 {
        let mut e1 = block_xor(block_load(&padded_prefix), k1keys[0]);
        let mut e2 = block_xor(block_load(&padded_prefix), k2keys[0]);
        for i in 1..ROUNDS {
            e1 = _sodium_softaes_block_encrypt(e1, k1keys[i]);
            e2 = _sodium_softaes_block_encrypt(e2, k2keys[i]);
        }
        e1 = _sodium_softaes_block_encryptlast(e1, k1keys[ROUNDS]);
        e2 = _sodium_softaes_block_encryptlast(e2, k2keys[ROUNDS]);

        let e = block_xor(e1, e2);
        block_store(&mut t, e);

        let cipher_bit = t[15] & 1;
        let bit_pos = 127 - prefix_len_bits;
        let original_bit = pfx_get_bit(in_slice, bit_pos);
        pfx_set_bit(&mut encrypted, bit_pos, original_bit ^ cipher_bit);

        pfx_shift_left(&mut padded_prefix);
        pfx_set_bit(&mut padded_prefix, 0, original_bit);

        prefix_len_bits += 1;
    }

    core::ptr::copy_nonoverlapping(encrypted.as_ptr(), out, 16);
    sodium_memzero(k2keys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    sodium_memzero(k1keys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn imp_pfx_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    pfx_setup(&mut k1keys, &mut k2keys, k);

    let in_slice = core::slice::from_raw_parts(inp, 16);

    let mut prefix_start: u32 = 0;
    if is_ipv4_mapped(in_slice) {
        prefix_start = 96;
    }

    let mut padded_prefix = [0u8; 16];
    pfx_pad_prefix(&mut padded_prefix, prefix_start);

    let mut decrypted = [0u8; 16];
    if prefix_start == 96 {
        decrypted[10] = 0xff;
        decrypted[11] = 0xff;
    }

    let mut t = [0u8; 16];
    let mut prefix_len_bits = prefix_start;
    while prefix_len_bits < 128 {
        let mut e1 = block_xor(block_load(&padded_prefix), k1keys[0]);
        let mut e2 = block_xor(block_load(&padded_prefix), k2keys[0]);
        for i in 1..ROUNDS {
            e1 = _sodium_softaes_block_encrypt(e1, k1keys[i]);
            e2 = _sodium_softaes_block_encrypt(e2, k2keys[i]);
        }
        e1 = _sodium_softaes_block_encryptlast(e1, k1keys[ROUNDS]);
        e2 = _sodium_softaes_block_encryptlast(e2, k2keys[ROUNDS]);

        let e = block_xor(e1, e2);
        block_store(&mut t, e);

        let cipher_bit = t[15] & 1;
        let bit_pos = 127 - prefix_len_bits;
        let encrypted_bit = pfx_get_bit(in_slice, bit_pos);
        let original_bit = encrypted_bit ^ cipher_bit;
        pfx_set_bit(&mut decrypted, bit_pos, original_bit);

        pfx_shift_left(&mut padded_prefix);
        pfx_set_bit(&mut padded_prefix, 0, original_bit);

        prefix_len_bits += 1;
    }

    core::ptr::copy_nonoverlapping(decrypted.as_ptr(), out, 16);
    sodium_memzero(k2keys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    sodium_memzero(k1keys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

// --- implementation struct ---

#[repr(C)]
pub struct IpcryptImplementation {
    pub encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pub decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pub nd_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8),
    pub nd_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pub ndx_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8),
    pub ndx_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pub pfx_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pub pfx_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
}

unsafe impl Sync for IpcryptImplementation {}

#[unsafe(no_mangle)]
pub static ipcrypt_soft_implementation: IpcryptImplementation = IpcryptImplementation {
    encrypt: imp_encrypt,
    decrypt: imp_decrypt,
    nd_encrypt: imp_nd_encrypt,
    nd_decrypt: imp_nd_decrypt,
    ndx_encrypt: imp_ndx_encrypt,
    ndx_decrypt: imp_ndx_decrypt,
    pfx_encrypt: imp_pfx_encrypt,
    pfx_decrypt: imp_pfx_decrypt,
};

// current implementation pointer (defaults to soft)
static mut IMPLEMENTATION: *const IpcryptImplementation = &ipcrypt_soft_implementation;

#[inline]
unsafe fn imp() -> &'static IpcryptImplementation {
    &*IMPLEMENTATION
}

// --- public crypto_ipcrypt API ---

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_bytes() -> usize {
    CRYPTO_IPCRYPT_BYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_keybytes() -> usize {
    CRYPTO_IPCRYPT_KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_nd_keybytes() -> usize {
    CRYPTO_IPCRYPT_ND_KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_nd_tweakbytes() -> usize {
    CRYPTO_IPCRYPT_ND_TWEAKBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_nd_inputbytes() -> usize {
    CRYPTO_IPCRYPT_ND_INPUTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_nd_outputbytes() -> usize {
    CRYPTO_IPCRYPT_ND_OUTPUTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_ndx_keybytes() -> usize {
    CRYPTO_IPCRYPT_NDX_KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_ndx_tweakbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_TWEAKBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_ndx_inputbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_INPUTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_ndx_outputbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_OUTPUTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_pfx_keybytes() -> usize {
    CRYPTO_IPCRYPT_PFX_KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_pfx_bytes() -> usize {
    CRYPTO_IPCRYPT_PFX_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_IPCRYPT_KEYBYTES);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_IPCRYPT_ND_KEYBYTES);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_IPCRYPT_NDX_KEYBYTES);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_IPCRYPT_PFX_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_encrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    (imp().encrypt)(out, inp, k);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    (imp().decrypt)(out, inp, k);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_encrypt(out: *mut u8, inp: *const u8, t: *const u8, k: *const u8) {
    (imp().nd_encrypt)(out, inp, t, k);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    (imp().nd_decrypt)(out, inp, k);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_encrypt(out: *mut u8, inp: *const u8, t: *const u8, k: *const u8) {
    (imp().ndx_encrypt)(out, inp, t, k);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    (imp().ndx_decrypt)(out, inp, k);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_encrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    (imp().pfx_encrypt)(out, inp, k);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    (imp().pfx_decrypt)(out, inp, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_ipcrypt_pick_best_implementation() -> c_int {
    IMPLEMENTATION = &ipcrypt_soft_implementation;
    0
}
