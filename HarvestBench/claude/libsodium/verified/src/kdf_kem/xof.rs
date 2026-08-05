// Translation of crypto_xof/{shake128,shake256,turboshake128,turboshake256}.
// shake* use the 24-round Keccak permutation; turboshake* use the 12-round one.
// The *_ref state machine is identical across all four; only RATE and the
// permutation differ.

use core::ffi::c_int;

#[repr(C, align(16))]
struct KeccakState {
    opaque: [u8; 224],
}

#[repr(C)]
struct XofStateInternal {
    state: KeccakState,
    offset: usize,
    phase: u8,
    domain: u8,
}

const PHASE_ABSORBING: u8 = 0;
const PHASE_SQUEEZING: u8 = 1;

extern "C" {
    fn crypto_core_keccak1600_init(state: *mut KeccakState);
    fn crypto_core_keccak1600_xor_bytes(
        state: *mut KeccakState,
        bytes: *const u8,
        offset: usize,
        length: usize,
    );
    fn crypto_core_keccak1600_extract_bytes(
        state: *const KeccakState,
        bytes: *mut u8,
        offset: usize,
        length: usize,
    );
    fn crypto_core_keccak1600_permute_24(state: *mut KeccakState);
    fn crypto_core_keccak1600_permute_12(state: *mut KeccakState);
}

type PermuteFn = unsafe extern "C" fn(*mut KeccakState);

#[inline]
unsafe fn xof_init_with_domain(
    st: *mut XofStateInternal,
    domain: u8,
    phase_absorbing: u8,
) -> c_int {
    crypto_core_keccak1600_init(&mut (*st).state);
    (*st).offset = 0;
    (*st).phase = phase_absorbing;
    (*st).domain = domain;
    0
}

#[inline]
unsafe fn xof_update(
    st: *mut XofStateInternal,
    inp: *const u8,
    inlen: usize,
    rate: usize,
    permute: PermuteFn,
) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    if (*st).phase != PHASE_ABSORBING {
        permute(&mut (*st).state);
        (*st).phase = PHASE_ABSORBING;
        (*st).offset = 0;
        ret = -1;
    }

    if (*st).offset == rate && inlen > 0 {
        permute(&mut (*st).state);
        (*st).offset = 0;
    }
    if (*st).offset != 0 && inlen > 0 {
        chunk_size = rate - (*st).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(&mut (*st).state, inp, (*st).offset, chunk_size);
        (*st).offset += chunk_size;
        consumed = chunk_size;
        if (*st).offset == rate && consumed < inlen {
            permute(&mut (*st).state);
            (*st).offset = 0;
        }
    }
    while inlen - consumed >= rate {
        crypto_core_keccak1600_xor_bytes(&mut (*st).state, inp.add(consumed), 0, rate);
        consumed += rate;
        (*st).offset = rate;
        if consumed < inlen {
            permute(&mut (*st).state);
            (*st).offset = 0;
        }
    }
    if consumed < inlen {
        chunk_size = inlen - consumed;
        crypto_core_keccak1600_xor_bytes(&mut (*st).state, inp.add(consumed), 0, chunk_size);
        (*st).offset = chunk_size;
    }

    ret
}

#[inline]
unsafe fn xof_finalize(st: *mut XofStateInternal, rate: usize, permute: PermuteFn) {
    let pad: u8;

    if (*st).offset == rate {
        permute(&mut (*st).state);
        (*st).offset = 0;
    }

    if (*st).offset == rate - 1 {
        pad = (*st).domain ^ 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*st).state, &pad, (*st).offset, 1);
    } else {
        let domain = (*st).domain;
        crypto_core_keccak1600_xor_bytes(&mut (*st).state, &domain, (*st).offset, 1);
        let pad2: u8 = 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*st).state, &pad2, rate - 1, 1);
    }

    permute(&mut (*st).state);

    (*st).offset = 0;
    (*st).phase = PHASE_SQUEEZING;
}

#[inline]
unsafe fn xof_squeeze(
    st: *mut XofStateInternal,
    out: *mut u8,
    outlen: usize,
    rate: usize,
    permute: PermuteFn,
) -> c_int {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    if (*st).phase == PHASE_ABSORBING {
        xof_finalize(st, rate, permute);
    }

    if (*st).offset == rate && outlen > 0 {
        permute(&mut (*st).state);
        (*st).offset = 0;
    }
    if (*st).offset != 0 && outlen > 0 {
        chunk_size = rate - (*st).offset;
        if chunk_size > outlen {
            chunk_size = outlen;
        }
        crypto_core_keccak1600_extract_bytes(&(*st).state, out, (*st).offset, chunk_size);
        (*st).offset += chunk_size;
        extracted = chunk_size;
        if (*st).offset == rate && extracted < outlen {
            permute(&mut (*st).state);
            (*st).offset = 0;
        }
    }
    while outlen - extracted >= rate {
        crypto_core_keccak1600_extract_bytes(&(*st).state, out.add(extracted), 0, rate);
        extracted += rate;
        (*st).offset = rate;
        if extracted < outlen {
            permute(&mut (*st).state);
            (*st).offset = 0;
        }
    }
    if extracted < outlen {
        chunk_size = outlen - extracted;
        crypto_core_keccak1600_extract_bytes(&(*st).state, out.add(extracted), 0, chunk_size);
        (*st).offset = chunk_size;
    }

    0
}

// Generates the *_ref functions plus the public crypto_xof_* wrappers for one variant.
macro_rules! xof_variant {
    (
        rate = $rate:expr,
        permute = $permute:path,
        blockbytes = $blockbytes:expr,
        statebytes = $statebytes:expr,
        domain = $domain:expr,
        ref_init = $ref_init:ident,
        ref_init_dom = $ref_init_dom:ident,
        ref_update = $ref_update:ident,
        ref_squeeze = $ref_squeeze:ident,
        ref_all = $ref_all:ident,
        pub_blockbytes = $pub_blockbytes:ident,
        pub_statebytes = $pub_statebytes:ident,
        pub_domain = $pub_domain:ident,
        pub_all = $pub_all:ident,
        pub_init = $pub_init:ident,
        pub_init_dom = $pub_init_dom:ident,
        pub_update = $pub_update:ident,
        pub_squeeze = $pub_squeeze:ident,
    ) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $ref_init_dom(st: *mut XofStateInternal, domain: u8) -> c_int {
            xof_init_with_domain(st, domain, PHASE_ABSORBING)
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $ref_init(st: *mut XofStateInternal) -> c_int {
            $ref_init_dom(st, $domain)
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $ref_update(
            st: *mut XofStateInternal,
            inp: *const u8,
            inlen: usize,
        ) -> c_int {
            xof_update(st, inp, inlen, $rate, $permute)
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $ref_squeeze(
            st: *mut XofStateInternal,
            out: *mut u8,
            outlen: usize,
        ) -> c_int {
            xof_squeeze(st, out, outlen, $rate, $permute)
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $ref_all(
            out: *mut u8,
            outlen: usize,
            inp: *const u8,
            inlen: usize,
        ) -> c_int {
            let mut state = core::mem::MaybeUninit::<XofStateInternal>::uninit();
            let st = state.as_mut_ptr();
            $ref_init(st);
            $ref_update(st, inp, inlen);
            $ref_squeeze(st, out, outlen);
            0
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn $pub_blockbytes() -> usize {
            $blockbytes
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn $pub_statebytes() -> usize {
            $statebytes
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn $pub_domain() -> u8 {
            $domain
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $pub_all(
            out: *mut u8,
            outlen: usize,
            inp: *const u8,
            inlen: u64,
        ) -> c_int {
            $ref_all(out, outlen, inp, inlen as usize)
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $pub_init(state: *mut core::ffi::c_void) -> c_int {
            $ref_init(state as *mut XofStateInternal)
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $pub_init_dom(
            state: *mut core::ffi::c_void,
            domain: u8,
        ) -> c_int {
            $ref_init_dom(state as *mut XofStateInternal, domain)
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $pub_update(
            state: *mut core::ffi::c_void,
            inp: *const u8,
            inlen: u64,
        ) -> c_int {
            $ref_update(state as *mut XofStateInternal, inp, inlen as usize)
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $pub_squeeze(
            state: *mut core::ffi::c_void,
            out: *mut u8,
            outlen: usize,
        ) -> c_int {
            $ref_squeeze(state as *mut XofStateInternal, out, outlen)
        }
    };
}

xof_variant! {
    rate = 168,
    permute = crypto_core_keccak1600_permute_24,
    blockbytes = 168,
    statebytes = 256,
    domain = 0x1f,
    ref_init = _sodium_shake128_ref_init,
    ref_init_dom = _sodium_shake128_ref_init_with_domain,
    ref_update = _sodium_shake128_ref_update,
    ref_squeeze = _sodium_shake128_ref_squeeze,
    ref_all = _sodium_shake128_ref,
    pub_blockbytes = crypto_xof_shake128_blockbytes,
    pub_statebytes = crypto_xof_shake128_statebytes,
    pub_domain = crypto_xof_shake128_domain_standard,
    pub_all = crypto_xof_shake128,
    pub_init = crypto_xof_shake128_init,
    pub_init_dom = crypto_xof_shake128_init_with_domain,
    pub_update = crypto_xof_shake128_update,
    pub_squeeze = crypto_xof_shake128_squeeze,
}

xof_variant! {
    rate = 136,
    permute = crypto_core_keccak1600_permute_24,
    blockbytes = 136,
    statebytes = 256,
    domain = 0x1f,
    ref_init = _sodium_shake256_ref_init,
    ref_init_dom = _sodium_shake256_ref_init_with_domain,
    ref_update = _sodium_shake256_ref_update,
    ref_squeeze = _sodium_shake256_ref_squeeze,
    ref_all = _sodium_shake256_ref,
    pub_blockbytes = crypto_xof_shake256_blockbytes,
    pub_statebytes = crypto_xof_shake256_statebytes,
    pub_domain = crypto_xof_shake256_domain_standard,
    pub_all = crypto_xof_shake256,
    pub_init = crypto_xof_shake256_init,
    pub_init_dom = crypto_xof_shake256_init_with_domain,
    pub_update = crypto_xof_shake256_update,
    pub_squeeze = crypto_xof_shake256_squeeze,
}

xof_variant! {
    rate = 168,
    permute = crypto_core_keccak1600_permute_12,
    blockbytes = 168,
    statebytes = 256,
    domain = 0x1f,
    ref_init = _sodium_turboshake128_ref_init,
    ref_init_dom = _sodium_turboshake128_ref_init_with_domain,
    ref_update = _sodium_turboshake128_ref_update,
    ref_squeeze = _sodium_turboshake128_ref_squeeze,
    ref_all = _sodium_turboshake128_ref,
    pub_blockbytes = crypto_xof_turboshake128_blockbytes,
    pub_statebytes = crypto_xof_turboshake128_statebytes,
    pub_domain = crypto_xof_turboshake128_domain_standard,
    pub_all = crypto_xof_turboshake128,
    pub_init = crypto_xof_turboshake128_init,
    pub_init_dom = crypto_xof_turboshake128_init_with_domain,
    pub_update = crypto_xof_turboshake128_update,
    pub_squeeze = crypto_xof_turboshake128_squeeze,
}

xof_variant! {
    rate = 136,
    permute = crypto_core_keccak1600_permute_12,
    blockbytes = 136,
    statebytes = 256,
    domain = 0x1f,
    ref_init = _sodium_turboshake256_ref_init,
    ref_init_dom = _sodium_turboshake256_ref_init_with_domain,
    ref_update = _sodium_turboshake256_ref_update,
    ref_squeeze = _sodium_turboshake256_ref_squeeze,
    ref_all = _sodium_turboshake256_ref,
    pub_blockbytes = crypto_xof_turboshake256_blockbytes,
    pub_statebytes = crypto_xof_turboshake256_statebytes,
    pub_domain = crypto_xof_turboshake256_domain_standard,
    pub_all = crypto_xof_turboshake256,
    pub_init = crypto_xof_turboshake256_init,
    pub_init_dom = crypto_xof_turboshake256_init_with_domain,
    pub_update = crypto_xof_turboshake256_update,
    pub_squeeze = crypto_xof_turboshake256_squeeze,
}
