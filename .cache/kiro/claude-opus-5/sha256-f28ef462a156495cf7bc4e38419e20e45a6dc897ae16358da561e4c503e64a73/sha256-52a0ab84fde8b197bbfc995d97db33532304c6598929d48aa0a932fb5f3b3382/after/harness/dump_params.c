/*
 * Ground-truth parameter dumper.
 *
 * Compiled once per (HASH_BACKEND, THASH, SECPAR) combination with exactly the
 * same -DPARAMS=... that c_src/app/CMakeLists.txt uses, so every number it
 * prints comes from the C preprocessor rather than from an assumption.  The
 * Rust differential tests read the resulting key=value file at runtime, which
 * keeps the Rust .so a pure black box (no Rust constants are trusted).
 *
 * This file lives OUTSIDE c_src/ and does not modify it; it only #includes it.
 */
#include <stdio.h>
#include <stddef.h>
#include <stdint.h>

#include "../c_src/app/include/params.h"
#include "../c_src/app/include/context.h"
#include "../c_src/app/include/api.h"
#include "../c_src/app/include/fors.h"
#include "../c_src/app/include/wotsx1.h"
#ifdef SPX_BLAKE
#include "../c_src/lib/blake/include/blake.h"
#endif

int main(void)
{
    printf("SPX_N=%d\n", SPX_N);
    printf("SPX_FULL_HEIGHT=%d\n", SPX_FULL_HEIGHT);
    printf("SPX_D=%d\n", SPX_D);
    printf("SPX_TREE_HEIGHT=%d\n", SPX_TREE_HEIGHT);
    printf("SPX_FORS_HEIGHT=%d\n", SPX_FORS_HEIGHT);
    printf("SPX_FORS_TREES=%d\n", SPX_FORS_TREES);
    printf("SPX_FORS_MSG_BYTES=%d\n", SPX_FORS_MSG_BYTES);
    printf("SPX_FORS_BYTES=%d\n", SPX_FORS_BYTES);
    printf("SPX_FORS_PK_BYTES=%d\n", SPX_FORS_PK_BYTES);
    printf("SPX_WOTS_W=%d\n", SPX_WOTS_W);
    printf("SPX_WOTS_LOGW=%d\n", SPX_WOTS_LOGW);
    printf("SPX_WOTS_LEN1=%d\n", SPX_WOTS_LEN1);
    printf("SPX_WOTS_LEN2=%d\n", SPX_WOTS_LEN2);
    printf("SPX_WOTS_LEN=%d\n", SPX_WOTS_LEN);
    printf("SPX_WOTS_BYTES=%d\n", SPX_WOTS_BYTES);
    printf("SPX_ADDR_BYTES=%d\n", SPX_ADDR_BYTES);
    printf("SPX_BYTES=%d\n", SPX_BYTES);
    printf("SPX_PK_BYTES=%d\n", SPX_PK_BYTES);
    printf("SPX_SK_BYTES=%d\n", SPX_SK_BYTES);
    printf("CRYPTO_SEEDBYTES=%d\n", CRYPTO_SEEDBYTES);

    /* address-field offsets (backend dependent) */
    printf("SPX_OFFSET_LAYER=%d\n", SPX_OFFSET_LAYER);
    printf("SPX_OFFSET_TREE=%d\n", SPX_OFFSET_TREE);
    printf("SPX_OFFSET_TYPE=%d\n", SPX_OFFSET_TYPE);
    printf("SPX_OFFSET_KP_ADDR=%d\n", SPX_OFFSET_KP_ADDR);
    printf("SPX_OFFSET_CHAIN_ADDR=%d\n", SPX_OFFSET_CHAIN_ADDR);
    printf("SPX_OFFSET_HASH_ADDR=%d\n", SPX_OFFSET_HASH_ADDR);
    printf("SPX_OFFSET_TREE_HGT=%d\n", SPX_OFFSET_TREE_HGT);
    printf("SPX_OFFSET_TREE_INDEX=%d\n", SPX_OFFSET_TREE_INDEX);

    /* spx_ctx layout: the differential tests pass a raw buffer of this size */
    printf("sizeof_spx_ctx=%zu\n", sizeof(spx_ctx));
    printf("offsetof_pub_seed=%zu\n", offsetof(spx_ctx, pub_seed));
    printf("offsetof_sk_seed=%zu\n", offsetof(spx_ctx, sk_seed));
#ifdef SPX_SHA2
    printf("offsetof_state_seeded=%zu\n", offsetof(spx_ctx, state_seeded));
#endif

    /* which 512-bit variant is compiled in (drives the `inblocks > 1` branch
       in thash and the blakeX/shaX aliasing in hash_<backend>.c) */
#if defined(SPX_SHA512) && SPX_SHA512
    printf("X512=1\n");
#elif defined(SPX_BLAKE512) && SPX_BLAKE512
    printf("X512=1\n");
#else
    printf("X512=0\n");
#endif

#ifdef SPX_SHA2
    printf("BACKEND=sha2\n");
    printf("SPX_SHA256_ADDR_BYTES=22\n");
#elif defined(SPX_SHAKE)
    printf("BACKEND=shake\n");
#elif defined(SPX_BLAKE)
    printf("BACKEND=blake\n");
#elif defined(SPX_HARAKA)
    printf("BACKEND=haraka\n");
#endif

    /* leaf_info_x1 / fors_gen_leaf_info layouts, needed to drive the
       lowest-level treehash entry points across the FFI boundary */
    printf("sizeof_leaf_info_x1=%zu\n", sizeof(leaf_info_x1));
    printf("offsetof_li_wots_sig=%zu\n", offsetof(leaf_info_x1, wots_sig));
    printf("offsetof_li_wots_sign_leaf=%zu\n", offsetof(leaf_info_x1, wots_sign_leaf));
    printf("offsetof_li_wots_steps=%zu\n", offsetof(leaf_info_x1, wots_steps));
    printf("offsetof_li_leaf_addr=%zu\n", offsetof(leaf_info_x1, leaf_addr));
    printf("offsetof_li_pk_addr=%zu\n", offsetof(leaf_info_x1, pk_addr));
    printf("sizeof_fors_gen_leaf_info=%zu\n", sizeof(fors_gen_leaf_info));
    printf("sizeof_uint=%zu\n", sizeof(unsigned int));
#ifdef SPX_BLAKE
    printf("sizeof_blakestate256=%zu\n", sizeof(blakestate256));
    printf("sizeof_blakestate512=%zu\n", sizeof(blakestate512));
    printf("SPX_BLAKE256_OUTPUT_BYTES=%d\n", SPX_BLAKE256_OUTPUT_BYTES);
    printf("SPX_BLAKE512_OUTPUT_BYTES=%d\n", SPX_BLAKE512_OUTPUT_BYTES);
#endif
    return 0;
}
