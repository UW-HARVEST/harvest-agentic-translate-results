extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memmove(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memcmp(
        __s1: *const libc::c_void,
        __s2: *const libc::c_void,
        __n: size_t,
    ) -> libc::c_int;
    fn SPX_set_layer_addr(addr: *mut uint32_t, layer: uint32_t);
    fn SPX_set_tree_addr(addr: *mut uint32_t, tree: uint64_t);
    fn SPX_set_type(addr: *mut uint32_t, type_0: uint32_t);
    fn SPX_copy_subtree_addr(out: *mut uint32_t, in_0: *const uint32_t);
    fn SPX_set_keypair_addr(addr: *mut uint32_t, keypair: uint32_t);
    fn SPX_copy_keypair_addr(out: *mut uint32_t, in_0: *const uint32_t);
    fn SPX_fors_sign(
        sig: *mut libc::c_uchar,
        pk: *mut libc::c_uchar,
        m: *const libc::c_uchar,
        ctx: *const spx_ctx,
        fors_addr: *const uint32_t,
    );
    fn SPX_fors_pk_from_sig(
        pk: *mut libc::c_uchar,
        sig: *const libc::c_uchar,
        m: *const libc::c_uchar,
        ctx: *const spx_ctx,
        fors_addr: *const uint32_t,
    );
    fn SPX_initialize_hash_function(ctx: *mut spx_ctx);
    fn SPX_gen_message_random(
        R: *mut libc::c_uchar,
        sk_prf: *const libc::c_uchar,
        optrand: *const libc::c_uchar,
        m: *const libc::c_uchar,
        mlen: libc::c_ulonglong,
        ctx: *const spx_ctx,
    );
    fn SPX_hash_message(
        digest: *mut libc::c_uchar,
        tree: *mut uint64_t,
        leaf_idx: *mut uint32_t,
        R: *const libc::c_uchar,
        pk: *const libc::c_uchar,
        m: *const libc::c_uchar,
        mlen: libc::c_ulonglong,
        ctx: *const spx_ctx,
    );
    fn SPX_merkle_sign(
        sig: *mut uint8_t,
        root: *mut libc::c_uchar,
        ctx: *const spx_ctx,
        wots_addr: *mut uint32_t,
        tree_addr: *mut uint32_t,
        idx_leaf: uint32_t,
    );
    fn SPX_merkle_gen_root(root: *mut libc::c_uchar, ctx: *const spx_ctx);
    fn randombytes(x: *mut libc::c_uchar, xlen: libc::c_ulonglong);
    fn SPX_thash(
        out: *mut libc::c_uchar,
        in_0: *const libc::c_uchar,
        inblocks: libc::c_uint,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
    fn SPX_compute_root(
        root: *mut libc::c_uchar,
        leaf: *const libc::c_uchar,
        leaf_idx: uint32_t,
        idx_offset: uint32_t,
        auth_path: *const libc::c_uchar,
        tree_height: uint32_t,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
    fn SPX_wots_pk_from_sig(
        pk: *mut libc::c_uchar,
        sig: *const libc::c_uchar,
        msg: *const libc::c_uchar,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct spx_ctx {
    pub pub_seed: [uint8_t; 16],
    pub sk_seed: [uint8_t; 16],
    pub tweaked512_rc64: [[uint64_t; 8]; 10],
    pub tweaked256_rc32: [[uint32_t; 8]; 10],
}
pub const SPX_N: libc::c_int = 16 as libc::c_int;
pub const SPX_FULL_HEIGHT: libc::c_int = 63 as libc::c_int;
pub const SPX_D: libc::c_int = 7 as libc::c_int;
pub const SPX_FORS_HEIGHT: libc::c_int = 12 as libc::c_int;
pub const SPX_FORS_TREES: libc::c_int = 14 as libc::c_int;
pub const SPX_WOTS_LOGW: libc::c_int = 4 as libc::c_int;
pub const SPX_WOTS_LEN1: libc::c_int = 8 as libc::c_int * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: libc::c_int = 3 as libc::c_int;
pub const SPX_WOTS_LEN: libc::c_int = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: libc::c_int = SPX_WOTS_LEN * SPX_N;
pub const SPX_TREE_HEIGHT: libc::c_int = SPX_FULL_HEIGHT / SPX_D;
pub const SPX_FORS_BYTES: libc::c_int =
    (SPX_FORS_HEIGHT + 1 as libc::c_int) * SPX_FORS_TREES * SPX_N;
pub const SPX_BYTES: libc::c_int =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: libc::c_int = 2 as libc::c_int * SPX_N;
pub const SPX_SK_BYTES: libc::c_int = 2 as libc::c_int * SPX_N + SPX_PK_BYTES;
pub const SPX_ADDR_TYPE_WOTS: libc::c_int = 0 as libc::c_int;
pub const SPX_ADDR_TYPE_WOTSPK: libc::c_int = 1 as libc::c_int;
pub const SPX_ADDR_TYPE_HASHTREE: libc::c_int = 2 as libc::c_int;
pub const CRYPTO_SECRETKEYBYTES: libc::c_int = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: libc::c_int = SPX_PK_BYTES;
pub const CRYPTO_BYTES: libc::c_int = SPX_BYTES;
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> libc::c_ulonglong {
    return CRYPTO_SECRETKEYBYTES as libc::c_ulonglong;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> libc::c_ulonglong {
    return CRYPTO_PUBLICKEYBYTES as libc::c_ulonglong;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_bytes() -> libc::c_ulonglong {
    return CRYPTO_BYTES as libc::c_ulonglong;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> libc::c_ulonglong {
    return (3 as libc::c_int * SPX_N) as libc::c_ulonglong;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    mut pk: *mut libc::c_uchar,
    mut sk: *mut libc::c_uchar,
    mut seed: *const libc::c_uchar,
) -> libc::c_int {
    let mut ctx: spx_ctx = spx_ctx {
        pub_seed: [0; 16],
        sk_seed: [0; 16],
        tweaked512_rc64: [[0; 8]; 10],
        tweaked256_rc32: [[0; 8]; 10],
    };
    memcpy(
        sk as *mut libc::c_void,
        seed as *const libc::c_void,
        (3 as libc::c_int * SPX_N) as size_t,
    );
    memcpy(
        pk as *mut libc::c_void,
        sk.offset((2 as libc::c_int * SPX_N) as isize) as *const libc::c_void,
        SPX_N as size_t,
    );
    memcpy(
        &raw mut ctx.pub_seed as *mut uint8_t as *mut libc::c_void,
        pk as *const libc::c_void,
        SPX_N as size_t,
    );
    memcpy(
        &raw mut ctx.sk_seed as *mut uint8_t as *mut libc::c_void,
        sk as *const libc::c_void,
        SPX_N as size_t,
    );
    SPX_initialize_hash_function(&raw mut ctx);
    SPX_merkle_gen_root(
        sk.offset((3 as libc::c_int * SPX_N) as isize),
        &raw mut ctx,
    );
    memcpy(
        pk.offset(SPX_N as isize) as *mut libc::c_void,
        sk.offset((3 as libc::c_int * SPX_N) as isize) as *const libc::c_void,
        SPX_N as size_t,
    );
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_keypair(
    mut pk: *mut libc::c_uchar,
    mut sk: *mut libc::c_uchar,
) -> libc::c_int {
    let mut seed: [libc::c_uchar; 48] = [0; 48];
    randombytes(
        &raw mut seed as *mut libc::c_uchar,
        (3 as libc::c_int * SPX_N) as libc::c_ulonglong,
    );
    crypto_sign_seed_keypair(pk, sk, &raw mut seed as *mut libc::c_uchar);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_signature(
    mut sig: *mut uint8_t,
    mut siglen: *mut size_t,
    mut m: *const uint8_t,
    mut mlen: size_t,
    mut sk: *const uint8_t,
) -> libc::c_int {
    let mut ctx: spx_ctx = spx_ctx {
        pub_seed: [0; 16],
        sk_seed: [0; 16],
        tweaked512_rc64: [[0; 8]; 10],
        tweaked256_rc32: [[0; 8]; 10],
    };
    let mut sk_prf: *const libc::c_uchar = sk.offset(SPX_N as isize);
    let mut pk: *const libc::c_uchar = sk.offset((2 as libc::c_int * SPX_N) as isize);
    let mut optrand: [libc::c_uchar; 16] = [0; 16];
    let mut mhash: [libc::c_uchar; 21] = [0; 21];
    let mut root: [libc::c_uchar; 16] = [0; 16];
    let mut i: uint32_t = 0;
    let mut tree: uint64_t = 0;
    let mut idx_leaf: uint32_t = 0;
    let mut wots_addr: [uint32_t; 8] = [0 as libc::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    let mut tree_addr: [uint32_t; 8] = [0 as libc::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    memcpy(
        &raw mut ctx.sk_seed as *mut uint8_t as *mut libc::c_void,
        sk as *const libc::c_void,
        SPX_N as size_t,
    );
    memcpy(
        &raw mut ctx.pub_seed as *mut uint8_t as *mut libc::c_void,
        pk as *const libc::c_void,
        SPX_N as size_t,
    );
    SPX_initialize_hash_function(&raw mut ctx);
    SPX_set_type(
        &raw mut wots_addr as *mut uint32_t,
        SPX_ADDR_TYPE_WOTS as uint32_t,
    );
    SPX_set_type(
        &raw mut tree_addr as *mut uint32_t,
        SPX_ADDR_TYPE_HASHTREE as uint32_t,
    );
    randombytes(
        &raw mut optrand as *mut libc::c_uchar,
        SPX_N as libc::c_ulonglong,
    );
    SPX_gen_message_random(
        sig as *mut libc::c_uchar,
        sk_prf,
        &raw mut optrand as *mut libc::c_uchar,
        m as *const libc::c_uchar,
        mlen as libc::c_ulonglong,
        &raw mut ctx,
    );
    SPX_hash_message(
        &raw mut mhash as *mut libc::c_uchar,
        &raw mut tree,
        &raw mut idx_leaf,
        sig,
        pk,
        m as *const libc::c_uchar,
        mlen as libc::c_ulonglong,
        &raw mut ctx,
    );
    sig = sig.offset(SPX_N as isize);
    SPX_set_tree_addr(&raw mut wots_addr as *mut uint32_t, tree);
    SPX_set_keypair_addr(&raw mut wots_addr as *mut uint32_t, idx_leaf);
    SPX_fors_sign(
        sig as *mut libc::c_uchar,
        &raw mut root as *mut libc::c_uchar,
        &raw mut mhash as *mut libc::c_uchar,
        &raw mut ctx,
        &raw mut wots_addr as *mut uint32_t as *const uint32_t,
    );
    sig = sig.offset(SPX_FORS_BYTES as isize);
    i = 0 as uint32_t;
    while i < SPX_D as uint32_t {
        SPX_set_layer_addr(&raw mut tree_addr as *mut uint32_t, i);
        SPX_set_tree_addr(&raw mut tree_addr as *mut uint32_t, tree);
        SPX_copy_subtree_addr(
            &raw mut wots_addr as *mut uint32_t,
            &raw mut tree_addr as *mut uint32_t as *const uint32_t,
        );
        SPX_set_keypair_addr(&raw mut wots_addr as *mut uint32_t, idx_leaf);
        SPX_merkle_sign(
            sig,
            &raw mut root as *mut libc::c_uchar,
            &raw mut ctx,
            &raw mut wots_addr as *mut uint32_t,
            &raw mut tree_addr as *mut uint32_t,
            idx_leaf,
        );
        sig = sig.offset((SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N) as isize);
        idx_leaf = (tree
            & (((1 as libc::c_int) << SPX_TREE_HEIGHT) - 1 as libc::c_int)
                as uint64_t) as uint32_t;
        tree = tree >> SPX_TREE_HEIGHT;
        i = i.wrapping_add(1);
    }
    *siglen = SPX_BYTES as size_t;
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_verify(
    mut sig: *const uint8_t,
    mut siglen: size_t,
    mut m: *const uint8_t,
    mut mlen: size_t,
    mut pk: *const uint8_t,
) -> libc::c_int {
    let mut ctx: spx_ctx = spx_ctx {
        pub_seed: [0; 16],
        sk_seed: [0; 16],
        tweaked512_rc64: [[0; 8]; 10],
        tweaked256_rc32: [[0; 8]; 10],
    };
    let mut pub_root: *const libc::c_uchar = pk.offset(SPX_N as isize);
    let mut mhash: [libc::c_uchar; 21] = [0; 21];
    let mut wots_pk: [libc::c_uchar; 560] = [0; 560];
    let mut root: [libc::c_uchar; 16] = [0; 16];
    let mut leaf: [libc::c_uchar; 16] = [0; 16];
    let mut i: libc::c_uint = 0;
    let mut tree: uint64_t = 0;
    let mut idx_leaf: uint32_t = 0;
    let mut wots_addr: [uint32_t; 8] = [0 as libc::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    let mut tree_addr: [uint32_t; 8] = [0 as libc::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    let mut wots_pk_addr: [uint32_t; 8] =
        [0 as libc::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    if siglen != SPX_BYTES as size_t {
        return -(1 as libc::c_int);
    }
    memcpy(
        &raw mut ctx.pub_seed as *mut uint8_t as *mut libc::c_void,
        pk as *const libc::c_void,
        SPX_N as size_t,
    );
    SPX_initialize_hash_function(&raw mut ctx);
    SPX_set_type(
        &raw mut wots_addr as *mut uint32_t,
        SPX_ADDR_TYPE_WOTS as uint32_t,
    );
    SPX_set_type(
        &raw mut tree_addr as *mut uint32_t,
        SPX_ADDR_TYPE_HASHTREE as uint32_t,
    );
    SPX_set_type(
        &raw mut wots_pk_addr as *mut uint32_t,
        SPX_ADDR_TYPE_WOTSPK as uint32_t,
    );
    SPX_hash_message(
        &raw mut mhash as *mut libc::c_uchar,
        &raw mut tree,
        &raw mut idx_leaf,
        sig as *const libc::c_uchar,
        pk as *const libc::c_uchar,
        m as *const libc::c_uchar,
        mlen as libc::c_ulonglong,
        &raw mut ctx,
    );
    sig = sig.offset(SPX_N as isize);
    SPX_set_tree_addr(&raw mut wots_addr as *mut uint32_t, tree);
    SPX_set_keypair_addr(&raw mut wots_addr as *mut uint32_t, idx_leaf);
    SPX_fors_pk_from_sig(
        &raw mut root as *mut libc::c_uchar,
        sig as *const libc::c_uchar,
        &raw mut mhash as *mut libc::c_uchar,
        &raw mut ctx,
        &raw mut wots_addr as *mut uint32_t as *const uint32_t,
    );
    sig = sig.offset(SPX_FORS_BYTES as isize);
    i = 0 as libc::c_uint;
    while i < SPX_D as libc::c_uint {
        SPX_set_layer_addr(&raw mut tree_addr as *mut uint32_t, i as uint32_t);
        SPX_set_tree_addr(&raw mut tree_addr as *mut uint32_t, tree);
        SPX_copy_subtree_addr(
            &raw mut wots_addr as *mut uint32_t,
            &raw mut tree_addr as *mut uint32_t as *const uint32_t,
        );
        SPX_set_keypair_addr(&raw mut wots_addr as *mut uint32_t, idx_leaf);
        SPX_copy_keypair_addr(
            &raw mut wots_pk_addr as *mut uint32_t,
            &raw mut wots_addr as *mut uint32_t as *const uint32_t,
        );
        SPX_wots_pk_from_sig(
            &raw mut wots_pk as *mut libc::c_uchar,
            sig as *const libc::c_uchar,
            &raw mut root as *mut libc::c_uchar,
            &raw mut ctx,
            &raw mut wots_addr as *mut uint32_t,
        );
        sig = sig.offset(SPX_WOTS_BYTES as isize);
        SPX_thash(
            &raw mut leaf as *mut libc::c_uchar,
            &raw mut wots_pk as *mut libc::c_uchar,
            SPX_WOTS_LEN as libc::c_uint,
            &raw mut ctx,
            &raw mut wots_pk_addr as *mut uint32_t,
        );
        SPX_compute_root(
            &raw mut root as *mut libc::c_uchar,
            &raw mut leaf as *mut libc::c_uchar,
            idx_leaf,
            0 as uint32_t,
            sig as *const libc::c_uchar,
            SPX_TREE_HEIGHT as uint32_t,
            &raw mut ctx,
            &raw mut tree_addr as *mut uint32_t,
        );
        sig = sig.offset((SPX_TREE_HEIGHT * SPX_N) as isize);
        idx_leaf = (tree
            & (((1 as libc::c_int) << SPX_TREE_HEIGHT) - 1 as libc::c_int)
                as uint64_t) as uint32_t;
        tree = tree >> SPX_TREE_HEIGHT;
        i = i.wrapping_add(1);
    }
    if memcmp(
        &raw mut root as *mut libc::c_uchar as *const libc::c_void,
        pub_root as *const libc::c_void,
        SPX_N as size_t,
    ) != 0
    {
        return -(1 as libc::c_int);
    }
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign(
    mut sm: *mut libc::c_uchar,
    mut smlen: *mut libc::c_ulonglong,
    mut m: *const libc::c_uchar,
    mut mlen: libc::c_ulonglong,
    mut sk: *const libc::c_uchar,
) -> libc::c_int {
    let mut siglen: size_t = 0;
    crypto_sign_signature(
        sm as *mut uint8_t,
        &raw mut siglen,
        m as *const uint8_t,
        mlen as size_t,
        sk as *const uint8_t,
    );
    memmove(
        sm.offset(SPX_BYTES as isize) as *mut libc::c_void,
        m as *const libc::c_void,
        mlen as size_t,
    );
    *smlen = (siglen as libc::c_ulonglong).wrapping_add(mlen);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_open(
    mut m: *mut libc::c_uchar,
    mut mlen: *mut libc::c_ulonglong,
    mut sm: *const libc::c_uchar,
    mut smlen: libc::c_ulonglong,
    mut pk: *const libc::c_uchar,
) -> libc::c_int {
    if smlen < SPX_BYTES as libc::c_ulonglong {
        memset(
            m as *mut libc::c_void,
            0 as libc::c_int,
            smlen as size_t,
        );
        *mlen = 0 as libc::c_ulonglong;
        return -(1 as libc::c_int);
    }
    *mlen = smlen.wrapping_sub(SPX_BYTES as libc::c_ulonglong);
    if crypto_sign_verify(
        sm as *const uint8_t,
        SPX_BYTES as size_t,
        sm.offset(SPX_BYTES as isize),
        *mlen as size_t,
        pk as *const uint8_t,
    ) != 0
    {
        memset(
            m as *mut libc::c_void,
            0 as libc::c_int,
            smlen as size_t,
        );
        *mlen = 0 as libc::c_ulonglong;
        return -(1 as libc::c_int);
    }
    memmove(
        m as *mut libc::c_void,
        sm.offset(SPX_BYTES as isize) as *const libc::c_void,
        *mlen as size_t,
    );
    return 0 as libc::c_int;
}
