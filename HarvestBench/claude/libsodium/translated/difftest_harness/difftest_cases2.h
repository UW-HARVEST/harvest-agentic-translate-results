/* Second batch of differential cases: the internal (`_sodium_*`) APIs, the
 * exported implementation structs, the sodium_* memory helpers, randombytes,
 * and the API corners the first batch did not name directly.
 * Included from difftest.c after difftest_cases.h. */

#define CASE(NAME, OUTLEN, ...)                                            \
    do {                                                                   \
        result ra, rb;                                                     \
        int    missing = 0;                                                \
        memset(&ra, 0, sizeof ra);                                         \
        memset(&rb, 0, sizeof rb);                                         \
        if (verbose) fprintf(stderr, "RUN  %s\n", NAME);                    \
        for (int _i = 0; _i < 2; _i++) {                                   \
            void   *h = _i ? hR : hC;                                      \
            result *R = _i ? &rb : &ra;                                    \
            (void) h; (void) R;                                            \
            det_reset();                                                   \
            __VA_ARGS__;                                                   \
        }                                                                  \
        if (missing) { n_skip++; fprintf(stderr, "SKIP %s\n", NAME); }      \
        else report(NAME, &ra, &rb, OUTLEN);                               \
    } while (0)

#define GETF(type, var, symname)                                           \
    type var = (type) dlsym(h, symname);                                   \
    if (!var) { missing = 1; break; }

#define GETD(type, var, symname)                                           \
    type var = (type) dlsym(h, symname);                                   \
    if (!var) { missing = 1; break; }

/* Compare the process exit status of a call that may abort() via sodium_misuse. */
#define MCASE(NAME, ...)                                                    \
    do {                                                                    \
        int _st[2];                                                         \
        if (verbose) fprintf(stderr, "RUNM %s\n", NAME);                    \
        for (int _i = 0; _i < 2; _i++) {                                    \
            void *h = _i ? hR : hC;                                         \
            (void) h;                                                       \
            fflush(NULL);                                                   \
            pid_t _p = fork();                                              \
            if (_p == 0) {                                                  \
                det_reset();                                                \
                do { __VA_ARGS__; } while (0);                              \
                _exit(0);                                                   \
            }                                                               \
            int _s = 0;                                                     \
            waitpid(_p, &_s, 0);                                            \
            _st[_i] = _s;                                                   \
        }                                                                   \
        if (_st[0] == _st[1]) { n_pass++; }                                 \
        else { n_fail++;                                                    \
            fprintf(stderr, "FAIL(status) %s C=%d R=%d\n", NAME, _st[0], _st[1]); \
            note_fail("%s ", NAME); }                                       \
    } while (0)

/* --- structs mirrored from the C headers --- */
typedef struct {
    uint64_t hh[8];
    uint64_t tt[2];
    uint64_t ff[2];
    uint8_t  bbuf[256];
    size_t   bbuflen;
    uint8_t  last_node;
} blake2b_state_c;

typedef struct {
    uint8_t digest_length, key_length, fanout, depth;
    uint8_t leaf_length[4];
    uint8_t node_offset[8];
    uint8_t node_depth, inner_length;
    uint8_t reserved[14];
    uint8_t salt[16];
    uint8_t personal[16];
} blake2b_param_c;

typedef struct {
    uint32_t w0, w1, w2, w3;
} SoftAesBlock_c;

typedef struct {
    void  *base, *aligned;
    size_t size;
} escrypt_region_c;

typedef struct { uint64_t v[128]; } a2_block_c;
typedef struct { a2_block_c *memory; size_t size; } a2_block_region_c;
typedef struct {
    a2_block_region_c *region;
    uint64_t          *pseudo_rands;
    uint32_t           passes;
    uint32_t           current_pass;
    uint32_t           memory_blocks;
    uint32_t           segment_length;
    uint32_t           lane_length;
    uint32_t           lanes;
    uint32_t           threads;
    int                type;
    int                print_internals;
} a2_instance_c;
typedef struct { uint32_t pass; uint32_t lane; uint8_t slice; uint32_t index; } a2_position_c;

typedef struct {
    uint8_t *out;    uint32_t outlen;
    uint8_t *pwd;    uint32_t pwdlen;
    uint8_t *salt;   uint32_t saltlen;
    uint8_t *secret; uint32_t secretlen;
    uint8_t *ad;     uint32_t adlen;
    uint32_t t_cost, m_cost, lanes, threads, flags;
} argon2_context_c;

/* --- function pointer typedefs --- */
typedef void (*fp61x)(void *, int);
typedef int (*aead_dd)(uc *, uc *, const uc *, ull, const uc *, const uc *, ull, const uc *, const uc *);
typedef int (*box_od)(uc *, const uc *, const uc *, ull, const uc *, const uc *, const uc *);
typedef int (*sb_od)(uc *, const uc *, const uc *, ull, const uc *, const uc *);
typedef int (*sb_det)(uc *, uc *, const uc *, ull, const uc *, const uc *);
typedef int  (*fp62x)(uc *, ull *, const uc *, ull, const uc *, int);
typedef int  (*fp63x)(const uc *, const uc *, ull, const uc *, int);
typedef int  (*b2_init)(void *, uint8_t);
typedef int  (*b2_init_sp)(void *, uint8_t, const void *, const void *);
typedef int  (*b2_init_key)(void *, uint8_t, const void *, uint8_t);
typedef int  (*b2_init_ksp)(void *, uint8_t, const void *, uint8_t, const void *, const void *);
typedef int  (*b2_init_param)(void *, const void *);
typedef int  (*b2_update)(void *, const uint8_t *, uint64_t);
typedef int  (*b2_final)(void *, uint8_t *, uint8_t);
typedef int  (*b2_simple)(uint8_t *, const void *, const void *, uint8_t, uint64_t, uint8_t);
typedef int  (*b2_sp)(uint8_t *, const void *, const void *, uint8_t, uint64_t, uint8_t, const void *, const void *);
typedef int  (*b2_compress)(void *, const uint8_t *);
typedef int  (*b2_pick)(void);
typedef int  (*b2_long)(void *, size_t, const void *, size_t);

typedef void (*kc_init)(void *);
typedef void (*kc_xor)(void *, const uc *, size_t, size_t);
typedef void (*kc_extract)(const void *, uc *, size_t, size_t);
typedef void (*kc_perm)(void *);

typedef int (*xf_oneshot)(uc *, size_t, const uc *, size_t);
typedef int (*xf_init)(void *);
typedef int (*xf_initdom)(void *, uc);
typedef int (*xf_update)(void *, const uc *, size_t);
typedef int (*xf_squeeze)(void *, uc *, size_t);

typedef void          (*sa_expand)(SoftAesBlock_c *, const uint8_t *);
typedef void          (*sa_invert)(SoftAesBlock_c *);
typedef SoftAesBlock_c (*sa_round)(SoftAesBlock_c, SoftAesBlock_c);
typedef SoftAesBlock_c (*sa_imc)(SoftAesBlock_c);

typedef int (*km_kp)(uc *, uc *);
typedef int (*km_skp)(uc *, uc *, const uc *);
typedef int (*km_enc)(uc *, uc *, const uc *);
typedef int (*km_encd)(uc *, uc *, const uc *, const uc *);
typedef int (*km_dec)(uc *, const uc *, const uc *);

typedef void (*ge_v2p)(uc *, const void *);
typedef int  (*ge_i2p)(void *, const uc *);
typedef void (*ge_sm)(void *, const uc *);
typedef void (*ge_sm3)(void *, const uc *, const void *);
typedef void (*ge_dsm)(void *, const uc *, const void *, const uc *);
typedef void (*ge_3p)(void *, const void *, const void *);
typedef void (*ge_2p)(void *, const void *);
typedef int  (*ge_pred)(const void *);
typedef int  (*ge_predb)(const uc *);
typedef void (*ge_1p)(void *);
typedef void (*ge_bytes2)(uc *, const uc *);

typedef int      (*es_init_local)(escrypt_region_c *);
typedef int      (*es_free_local)(escrypt_region_c *);
typedef void *   (*es_alloc)(escrypt_region_c *, size_t);
typedef int      (*es_free_region)(escrypt_region_c *);
typedef int      (*es_kdf)(escrypt_region_c *, const uint8_t *, size_t, const uint8_t *, size_t, uint64_t, uint32_t, uint32_t, uint8_t *, size_t);
typedef uint8_t *(*es_r)(escrypt_region_c *, const uint8_t *, size_t, const uint8_t *, uint8_t *, size_t);
typedef uint8_t *(*es_gensalt)(uint32_t, uint32_t, uint32_t, const uint8_t *, size_t, uint8_t *, size_t);
typedef const uint8_t *(*es_parse)(const uint8_t *, uint32_t *, uint32_t *, uint32_t *);
typedef void     (*es_pbkdf2)(const uint8_t *, size_t, const uint8_t *, size_t, uint64_t, uint8_t *, size_t);

typedef int  (*a2_ctx)(argon2_context_c *, int);
typedef int  (*a2_initialize)(a2_instance_c *, argon2_context_c *);
typedef void (*a2_finalize)(const argon2_context_c *, a2_instance_c *);
typedef void (*a2_fillmem)(a2_instance_c *, uint32_t);
typedef void (*a2_fillseg)(const a2_instance_c *, a2_position_c);
typedef void (*v_misuse)(void);
typedef int (*a2_validate)(const argon2_context_c *);
typedef int (*a2_hash)(uint32_t, uint32_t, uint32_t, const void *, size_t, const void *, size_t, void *, size_t, char *, size_t, int, uint32_t);
typedef int (*a2_hash_enc)(uint32_t, uint32_t, uint32_t, const void *, size_t, const void *, size_t, size_t, char *, size_t);
typedef int (*a2_hash_raw)(uint32_t, uint32_t, uint32_t, const void *, size_t, const void *, size_t, void *, size_t);
typedef int (*a2_verify)(const char *, const void *, size_t);
typedef int (*a2_verify_t)(const char *, const void *, size_t, int);
typedef int (*a2_encode)(char *, size_t, argon2_context_c *, int);
typedef int (*a2_decode)(argon2_context_c *, const char *, int);

typedef void *(*v_malloc)(size_t);
typedef void *(*v_allocarray)(size_t, size_t);
typedef void  (*v_free)(void *);
typedef int   (*v_mprot)(void *);
typedef int   (*v_mlock)(void *, size_t);
typedef void  (*v_memzero)(void *, size_t);
typedef void  (*v_stackzero)(size_t);
typedef int   (*v_void_int)(void);
typedef void  (*v_void)(void);
typedef void  (*v_buf)(void *, size_t);
typedef uint32_t (*v_u32)(void);
typedef void  (*v_rb)(uc *, unsigned long long);
typedef int   (*v_setimpl)(const void *);
typedef int   (*v_setmis)(void (*)(void));

typedef int (*gh_init)(void *, const uc *, size_t, size_t);
typedef int (*gh_update)(void *, const uc *, ull);
typedef int (*gh_final)(void *, uc *, size_t);

/* implementation-struct shapes */
typedef struct {
    const char *(*implementation_name)(void);
    uint32_t (*random)(void);
    void (*stir)(void);
    uint32_t (*uniform)(uint32_t);
    void (*buf)(void *, size_t);
    int (*close)(void);
} rb_impl_c;

typedef struct {
    int (*stream)(unsigned char *, unsigned long long, const unsigned char *, const unsigned char *);
    int (*stream_xor_ic)(unsigned char *, const unsigned char *, unsigned long long,
                         const unsigned char *, uint64_t, const unsigned char *);
} salsa20_impl_c;

typedef struct {
    int (*stream)(unsigned char *, unsigned long long, const unsigned char *, const unsigned char *);
    int (*stream_ietf_ext)(unsigned char *, unsigned long long, const unsigned char *, const unsigned char *);
    int (*stream_xor_ic)(unsigned char *, const unsigned char *, unsigned long long,
                         const unsigned char *, uint64_t, const unsigned char *);
    int (*stream_ietf_ext_xor_ic)(unsigned char *, const unsigned char *, unsigned long long,
                                  const unsigned char *, uint32_t, const unsigned char *);
} chacha20_impl_c;

typedef struct {
    int (*onetimeauth)(unsigned char *, const unsigned char *, unsigned long long, const unsigned char *);
    int (*onetimeauth_verify)(const unsigned char *, const unsigned char *, unsigned long long, const unsigned char *);
    int (*onetimeauth_init)(void *, const unsigned char *);
    int (*onetimeauth_update)(void *, const unsigned char *, unsigned long long);
    int (*onetimeauth_final)(void *, unsigned char *);
} poly1305_impl_c;

typedef struct {
    int (*mult)(unsigned char *, const unsigned char *, const unsigned char *);
    int (*mult_base)(unsigned char *, const unsigned char *);
} x25519_impl_c;

typedef struct {
    int (*encrypt_detached)(uint8_t *c, uint8_t *mac, size_t maclen, const uint8_t *m, size_t mlen,
                            const uint8_t *ad, size_t adlen, const uint8_t *npub, const uint8_t *k);
    int (*decrypt_detached)(uint8_t *m, const uint8_t *c, size_t clen, const uint8_t *mac,
                            size_t maclen, const uint8_t *ad, size_t adlen, const uint8_t *npub,
                            const uint8_t *k);
} aegis_impl_c;

typedef struct {
    void (*encrypt)(unsigned char *, const unsigned char *, const unsigned char *);
    void (*decrypt)(unsigned char *, const unsigned char *, const unsigned char *);
    void (*nd_encrypt)(unsigned char *, const unsigned char *, const unsigned char *, const unsigned char *);
    void (*nd_decrypt)(unsigned char *, const unsigned char *, const unsigned char *);
    void (*ndx_encrypt)(unsigned char *, const unsigned char *, const unsigned char *, const unsigned char *);
    void (*ndx_decrypt)(unsigned char *, const unsigned char *, const unsigned char *);
    void (*pfx_encrypt)(unsigned char *, const unsigned char *, const unsigned char *);
    void (*pfx_decrypt)(unsigned char *, const unsigned char *, const unsigned char *);
} ipcrypt_impl_c;

/* ============ pick_best_implementation + runtime ============ */
{
    static const char *const pb[] = {
        "_crypto_aead_aegis128l_pick_best_implementation",
        "_crypto_aead_aegis256_pick_best_implementation",
        "_crypto_generichash_blake2b_pick_best_implementation",
        "_crypto_ipcrypt_pick_best_implementation",
        "_crypto_onetimeauth_poly1305_pick_best_implementation",
        "_crypto_pwhash_argon2_pick_best_implementation",
        "_crypto_scalarmult_curve25519_pick_best_implementation",
        "_crypto_stream_chacha20_pick_best_implementation",
        "_crypto_stream_salsa20_pick_best_implementation",
        "_sodium_blake2b_pick_best_implementation",
        "_sodium_runtime_get_cpu_features",
        "_sodium_alloc_init",
        "sodium_init",
        "sodium_crit_enter",
        "sodium_crit_leave",
        NULL
    };
    for (int k = 0; pb[k]; k++) {
        CASE(pb[k], 0, {
            GETF(v_void_int, f, pb[k]);
            R->ret = f();
        });
    }
    CASE("sodium_init twice", 0, {
        GETF(v_void_int, f, "sodium_init");
        R->ret = f() * 10 + f();
    });
}

/* ============ blake2b internal API ============ */
{
    uc msg[600], key[64], salt[16], pers[16];
    fillr(msg, sizeof msg); fillr(key, 64); fillr(salt, 16); fillr(pers, 16);
    for (int ol = 1; ol <= 64; ol += 21) {
        char nm[96];
        snprintf(nm, sizeof nm, "_sodium_blake2b_init/update/final ol%d", ol);
        CASE(nm, 64 + 400, {
            GETF(b2_init, fi, "_sodium_blake2b_init");
            GETF(b2_update, fu, "_sodium_blake2b_update");
            GETF(b2_final, ff, "_sodium_blake2b_final");
            blake2b_state_c S;
            memset(&S, 0xAB, sizeof S);
            R->ret = fi(&S, (uint8_t) ol);
            size_t off = 0, chunk = 1;
            while (off < sizeof msg) {
                size_t c = chunk;
                if (off + c > sizeof msg) c = sizeof msg - off;
                R->ret |= fu(&S, msg + off, c);
                off += c; chunk = chunk * 3 + 1;
            }
            R->ret |= ff(&S, R->out, (uint8_t) ol);
            memcpy(R->out + 64, &S, sizeof S > 400 ? 400 : sizeof S);
        });
        snprintf(nm, sizeof nm, "_sodium_blake2b_init_key ol%d", ol);
        CASE(nm, 64, {
            GETF(b2_init_key, fi, "_sodium_blake2b_init_key");
            GETF(b2_update, fu, "_sodium_blake2b_update");
            GETF(b2_final, ff, "_sodium_blake2b_final");
            blake2b_state_c S;
            memset(&S, 0, sizeof S);
            R->ret  = fi(&S, (uint8_t) ol, key, 32);
            R->ret |= fu(&S, msg, 137);
            R->ret |= ff(&S, R->out, (uint8_t) ol);
        });
        snprintf(nm, sizeof nm, "_sodium_blake2b_init_salt_personal ol%d", ol);
        CASE(nm, 64, {
            GETF(b2_init_sp, fi, "_sodium_blake2b_init_salt_personal");
            GETF(b2_update, fu, "_sodium_blake2b_update");
            GETF(b2_final, ff, "_sodium_blake2b_final");
            blake2b_state_c S;
            memset(&S, 0, sizeof S);
            R->ret  = fi(&S, (uint8_t) ol, salt, pers);
            R->ret |= fu(&S, msg, 137);
            R->ret |= ff(&S, R->out, (uint8_t) ol);
        });
        snprintf(nm, sizeof nm, "_sodium_blake2b_init_key_salt_personal ol%d", ol);
        CASE(nm, 64, {
            GETF(b2_init_ksp, fi, "_sodium_blake2b_init_key_salt_personal");
            GETF(b2_update, fu, "_sodium_blake2b_update");
            GETF(b2_final, ff, "_sodium_blake2b_final");
            blake2b_state_c S;
            memset(&S, 0, sizeof S);
            R->ret  = fi(&S, (uint8_t) ol, key, 17, salt, pers);
            R->ret |= fu(&S, msg, 137);
            R->ret |= ff(&S, R->out, (uint8_t) ol);
        });
        snprintf(nm, sizeof nm, "_sodium_blake2b ol%d", ol);
        CASE(nm, 64, {
            GETF(b2_simple, f, "_sodium_blake2b");
            R->ret = f(R->out, msg, key, (uint8_t) ol, 333, 32);
        });
        snprintf(nm, sizeof nm, "_sodium_blake2b nokey ol%d", ol);
        CASE(nm, 64, {
            GETF(b2_simple, f, "_sodium_blake2b");
            R->ret = f(R->out, msg, NULL, (uint8_t) ol, 333, 0);
        });
        snprintf(nm, sizeof nm, "_sodium_blake2b_salt_personal ol%d", ol);
        CASE(nm, 64, {
            GETF(b2_sp, f, "_sodium_blake2b_salt_personal");
            R->ret = f(R->out, msg, key, (uint8_t) ol, 333, 32, salt, pers);
        });
    }
    CASE("_sodium_blake2b_init_param", 64 + 400, {
        GETF(b2_init_param, fi, "_sodium_blake2b_init_param");
        GETF(b2_update, fu, "_sodium_blake2b_update");
        GETF(b2_final, ff, "_sodium_blake2b_final");
        blake2b_param_c P;
        blake2b_state_c S;
        memset(&P, 0, sizeof P);
        memset(&S, 0xCD, sizeof S);
        P.digest_length = 32; P.fanout = 1; P.depth = 1;
        memcpy(P.salt, salt, 16);
        memcpy(P.personal, pers, 16);
        R->ret  = fi(&S, &P);
        R->ret |= fu(&S, msg, 300);
        R->ret |= ff(&S, R->out, 32);
        memcpy(R->out + 64, &S, 361);
    });
    CASE("_sodium_blake2b_compress_ref", 400, {
        GETF(b2_init, fi, "_sodium_blake2b_init");
        GETF(b2_compress, fc, "_sodium_blake2b_compress_ref");
        blake2b_state_c S;
        memset(&S, 0, sizeof S);
        fi(&S, 64);
        S.tt[0] = 128;
        R->ret = fc(&S, msg);
        R->ret |= fc(&S, msg + 128);
        memcpy(R->out, &S, 361);
    });
    MCASE("_sodium_blake2b_init outlen0", {
        b2_init fi = (b2_init) dlsym(h, "_sodium_blake2b_init");
        blake2b_state_c S; memset(&S, 0, sizeof S);
        _exit(fi(&S, 0) == 0 ? 1 : 2);
    });
    MCASE("_sodium_blake2b_init outlen65", {
        b2_init fi = (b2_init) dlsym(h, "_sodium_blake2b_init");
        blake2b_state_c S; memset(&S, 0, sizeof S);
        _exit(fi(&S, 65) == 0 ? 1 : 2);
    });
    MCASE("_sodium_blake2b_init_key keylen0", {
        b2_init_key fk = (b2_init_key) dlsym(h, "_sodium_blake2b_init_key");
        blake2b_state_c S; memset(&S, 0, sizeof S);
        _exit(fk(&S, 32, key, 0) == 0 ? 1 : 2);
    });
    MCASE("_sodium_blake2b_init_key keylen65", {
        b2_init_key fk = (b2_init_key) dlsym(h, "_sodium_blake2b_init_key");
        blake2b_state_c S; memset(&S, 0, sizeof S);
        _exit(fk(&S, 32, key, 65) == 0 ? 1 : 2);
    });
    MCASE("_sodium_blake2b_init_key nullkey", {
        b2_init_key fk = (b2_init_key) dlsym(h, "_sodium_blake2b_init_key");
        blake2b_state_c S; memset(&S, 0, sizeof S);
        _exit(fk(&S, 32, NULL, 16) == 0 ? 1 : 2);
    });
    MCASE("_sodium_blake2b outlen0", {
        b2_simple f = (b2_simple) dlsym(h, "_sodium_blake2b");
        unsigned char o[64];
        _exit(f(o, msg, key, 0, 10, 32) == 0 ? 1 : 2);
    });
    MCASE("_sodium_blake2b outlen65", {
        b2_simple f = (b2_simple) dlsym(h, "_sodium_blake2b");
        unsigned char o[64];
        _exit(f(o, msg, key, 65, 10, 32) == 0 ? 1 : 2);
    });
    MCASE("_sodium_blake2b_final twice", {
        b2_init fi = (b2_init) dlsym(h, "_sodium_blake2b_init");
        b2_final ff = (b2_final) dlsym(h, "_sodium_blake2b_final");
        blake2b_state_c S; memset(&S, 0, sizeof S);
        unsigned char o[64];
        fi(&S, 32);
        int r1 = ff(&S, o, 32);
        int r2 = ff(&S, o, 32);
        _exit((r1 == 0 ? 1 : 2) * 10 + (r2 == 0 ? 1 : 2));
    });
    for (int inlen = 0; inlen <= 400; inlen = inlen ? inlen * 5 + 3 : 1) {
        for (int ol = 1; ol <= 200; ol = ol * 8 + 7) {
            char nm[96];
            snprintf(nm, sizeof nm, "_sodium_blake2b_long in%d out%d", inlen, ol);
            CASE(nm, (size_t) ol, {
                GETF(b2_long, f, "_sodium_blake2b_long");
                R->ret = f(R->out, (size_t) ol, msg, (size_t) inlen);
            });
        }
    }
}

/* ============ keccak1600 ref backend ============ */
{
    uc buf[400];
    fillr(buf, sizeof buf);
    CASE("_sodium_keccak1600_ref_*", 500, {
        GETF(kc_init, fi, "_sodium_keccak1600_ref_init");
        GETF(kc_xor, fx, "_sodium_keccak1600_ref_xor_bytes");
        GETF(kc_extract, fe, "_sodium_keccak1600_ref_extract_bytes");
        GETF(kc_perm, p24, "_sodium_keccak1600_ref_permute_24");
        GETF(kc_perm, p12, "_sodium_keccak1600_ref_permute_12");
        uc state[256];
        memset(state, 0xEE, sizeof state);
        fi(state);
        fx(state, buf, 0, 136);
        p24(state);
        fx(state, buf + 136, 7, 100);
        p12(state);
        fe(state, R->out, 0, 200);
        fe(state, R->out + 200, 13, 100);
        memcpy(R->out + 300, state, 200);
    });
}

/* ============ shake / turboshake ref backends ============ */
{
    uc msg[400];
    fillr(msg, sizeof msg);
    static const char *const rf[] = {
        "_sodium_shake128_ref", "_sodium_shake256_ref",
        "_sodium_turboshake128_ref", "_sodium_turboshake256_ref", NULL
    };
    for (int k = 0; rf[k]; k++) {
        char nm[128], s1[160], s2[160], s3[160], s4[160];
        snprintf(s1, sizeof s1, "%s_init", rf[k]);
        snprintf(s2, sizeof s2, "%s_update", rf[k]);
        snprintf(s3, sizeof s3, "%s_squeeze", rf[k]);
        snprintf(s4, sizeof s4, "%s_init_with_domain", rf[k]);
        for (int ol = 1; ol <= 400; ol = ol * 9 + 1) {
            snprintf(nm, sizeof nm, "%s out%d", rf[k], ol);
            CASE(nm, (size_t) ol, {
                GETF(xf_oneshot, f, rf[k]);
                R->ret = f(R->out, (size_t) ol, msg, sizeof msg);
            });
        }
        snprintf(nm, sizeof nm, "%s streaming", rf[k]);
        CASE(nm, 200 + 256, {
            GETF(xf_init, fi, s1);
            GETF(xf_update, fu, s2);
            GETF(xf_squeeze, fq, s3);
            uc st[400];
            memset(st, 0xA5, sizeof st);
            R->ret = fi(st);
            size_t off = 0, chunk = 1;
            while (off < sizeof msg) {
                size_t c = chunk;
                if (off + c > sizeof msg) c = sizeof msg - off;
                R->ret |= fu(st, msg + off, c);
                off += c; chunk = chunk * 4 + 1;
            }
            size_t so = 0; chunk = 1;
            while (so < 200) {
                size_t c = chunk;
                if (so + c > 200) c = 200 - so;
                R->ret |= fq(st, R->out + so, c);
                so += c; chunk = chunk * 6 + 1;
            }
            memcpy(R->out + 200, st, 240);
        });
        for (int dom = 1; dom <= 0xff; dom = dom * 5 + 6) {
            snprintf(nm, sizeof nm, "%s dom%02x", rf[k], dom);
            CASE(nm, 100, {
                GETF(xf_initdom, fi, s4);
                GETF(xf_update, fu, s2);
                GETF(xf_squeeze, fq, s3);
                uc st[400];
                memset(st, 0, sizeof st);
                R->ret  = fi(st, (uc) dom);
                R->ret |= fu(st, msg, sizeof msg);
                R->ret |= fq(st, R->out, 100);
            });
        }
        snprintf(nm, sizeof nm, "%s update-after-squeeze", rf[k]);
        CASE(nm, 64, {
            GETF(xf_init, fi, s1);
            GETF(xf_update, fu, s2);
            GETF(xf_squeeze, fq, s3);
            uc st[400];
            memset(st, 0, sizeof st);
            fi(st);
            fu(st, msg, 100);
            fq(st, R->out, 32);
            R->ret = fu(st, msg, 10);
            R->ret = R->ret * 100 + fq(st, R->out + 32, 32);
        });
    }
}

/* ============ softaes ============ */
{
    uc key[32];
    fillr(key, 32);
    CASE("_sodium_softaes_expand_key128 + rounds", 512, {
        GETF(sa_expand, e128, "_sodium_softaes_expand_key128");
        GETF(sa_round, enc, "_sodium_softaes_block_encrypt");
        GETF(sa_round, encl, "_sodium_softaes_block_encryptlast");
        GETF(sa_round, dec, "_sodium_softaes_block_decrypt");
        GETF(sa_round, decl, "_sodium_softaes_block_decryptlast");
        GETF(sa_imc, imc, "_sodium_softaes_inv_mix_columns");
        GETF(sa_invert, inv128, "_sodium_softaes_invert_key_schedule128");
        SoftAesBlock_c rk[11], blk;
        memset(rk, 0, sizeof rk);
        e128(rk, key);
        memcpy(R->out, rk, sizeof rk);
        blk.w0 = 0x03020100; blk.w1 = 0x07060504; blk.w2 = 0x0b0a0908; blk.w3 = 0x0f0e0d0c;
        for (int r = 0; r < 9; r++) blk = enc(blk, rk[r]);
        blk = encl(blk, rk[10]);
        memcpy(R->out + 176, &blk, 16);
        SoftAesBlock_c b2 = imc(blk);
        memcpy(R->out + 192, &b2, 16);
        inv128(rk);
        memcpy(R->out + 208, rk, sizeof rk);
        for (int r = 0; r < 9; r++) blk = dec(blk, rk[r]);
        blk = decl(blk, rk[10]);
        memcpy(R->out + 384, &blk, 16);
    });
    CASE("_sodium_softaes_expand_key256 + invert", 512, {
        GETF(sa_expand, e256, "_sodium_softaes_expand_key256");
        GETF(sa_invert, inv256, "_sodium_softaes_invert_key_schedule256");
        GETF(sa_round, enc, "_sodium_softaes_block_encrypt");
        GETF(sa_round, encl, "_sodium_softaes_block_encryptlast");
        SoftAesBlock_c rk[15], blk;
        memset(rk, 0, sizeof rk);
        e256(rk, key);
        memcpy(R->out, rk, sizeof rk);
        blk.w0 = 0x33221100; blk.w1 = 0x77665544; blk.w2 = 0xbbaa9988; blk.w3 = 0xffeeddcc;
        for (int r = 0; r < 13; r++) blk = enc(blk, rk[r]);
        blk = encl(blk, rk[14]);
        memcpy(R->out + 240, &blk, 16);
        inv256(rk);
        memcpy(R->out + 256, rk, sizeof rk);
    });
}

/* ============ ml-kem 768 ref ============ */
{
    uc seed[64];
    fillr(seed, 64);
    CASE("_sodium_mlkem768_ref_seed_keypair", 64, {
        GETF(km_skp, f, "_sodium_mlkem768_ref_seed_keypair");
        uc pk[1184], sk[2400];
        R->ret = f(pk, sk, seed);
        unsigned long long acc = 1469598103934665603ULL;
        for (int i = 0; i < 1184; i++) { acc ^= pk[i]; acc *= 1099511628211ULL; }
        memcpy(R->out, &acc, 8);
        acc = 1469598103934665603ULL;
        for (int i = 0; i < 2400; i++) { acc ^= sk[i]; acc *= 1099511628211ULL; }
        memcpy(R->out + 8, &acc, 8);
    });
    CASE("_sodium_mlkem768_ref_enc_deterministic + dec", 96, {
        GETF(km_skp, kp, "_sodium_mlkem768_ref_seed_keypair");
        GETF(km_encd, e, "_sodium_mlkem768_ref_enc_deterministic");
        GETF(km_dec, d, "_sodium_mlkem768_ref_dec");
        uc pk[1184], sk[2400], ct[1088];
        kp(pk, sk, seed);
        R->ret = e(ct, R->out, pk, seed + 32);
        R->ret |= d(R->out + 32, ct, sk);
        ct[5] ^= 0x40;
        R->ret |= d(R->out + 64, ct, sk);
    });
    CASE("_sodium_mlkem768_ref_keypair/enc (random)", 0, {
        GETF(km_kp, kp, "_sodium_mlkem768_ref_keypair");
        GETF(km_enc, e, "_sodium_mlkem768_ref_enc");
        GETF(km_dec, d, "_sodium_mlkem768_ref_dec");
        uc pk[1184], sk[2400], ct[1088], ss1[32], ss2[32];
        R->ret = kp(pk, sk);
        R->ret |= e(ct, ss1, pk);
        R->ret |= d(ss2, ct, sk);
        /* deterministic RNG => the whole flow is reproducible */
        R->extra = memcmp(ss1, ss2, 32) == 0 ? 1 : 0;
        unsigned long long acc = 1469598103934665603ULL;
        for (int i = 0; i < 1088; i++) { acc ^= ct[i]; acc *= 1099511628211ULL; }
        R->extra = R->extra * 1000003 + (acc & 0xffffff);
    });
}

/* ============ internal ed25519 group ops ============ */
{
    uc a[32], b[64], pt[32];
    fillr(a, 32); fillr(b, 64);
    /* obtain a valid point encoding */
    {
        void *h = hC;
        ge_sm sm = (ge_sm) dlsym(h, "_sodium_ge25519_scalarmult_base");
        ge_v2p tb = (ge_v2p) dlsym(h, "_sodium_ge25519_p3_tobytes");
        if (sm && tb) { uc p3[512]; memset(p3, 0, sizeof p3); sm(p3, a); tb(pt, p3); }
    }
    CASE("_sodium_ge25519_frombytes", 512, {
        GETF(ge_i2p, f, "_sodium_ge25519_frombytes");
        uc p3[512];
        memset(p3, 0x11, sizeof p3);
        R->ret = f(p3, pt);
        memcpy(R->out, p3, 160);
        memset(p3, 0x22, sizeof p3);
        R->extra = (unsigned long long) (f(p3, a) + 100);
        memcpy(R->out + 160, p3, 160);
    });
    CASE("_sodium_ge25519_frombytes_negate_vartime", 512, {
        GETF(ge_i2p, f, "_sodium_ge25519_frombytes_negate_vartime");
        uc p3[512];
        memset(p3, 0x11, sizeof p3);
        R->ret = f(p3, pt);
        memcpy(R->out, p3, 160);
        memset(p3, 0x22, sizeof p3);
        R->extra = (unsigned long long) (f(p3, a) + 100);
        memcpy(R->out + 160, p3, 160);
    });
    CASE("_sodium_ge25519_p3_add/sub/scalarmult", 256, {
        GETF(ge_sm, sm, "_sodium_ge25519_scalarmult_base");
        GETF(ge_3p, ad, "_sodium_ge25519_p3_add");
        GETF(ge_3p, sb, "_sodium_ge25519_p3_sub");
        GETF(ge_sm3, s3, "_sodium_ge25519_scalarmult");
        GETF(ge_v2p, tb, "_sodium_ge25519_p3_tobytes");
        uc P[512], Q[512], Rr[512];
        memset(P, 0, sizeof P); memset(Q, 0, sizeof Q); memset(Rr, 0, sizeof Rr);
        sm(P, a);
        sm(Q, b);
        ad(Rr, P, Q);
        tb(R->out, Rr);
        sb(Rr, P, Q);
        tb(R->out + 32, Rr);
        s3(Rr, a, P);
        tb(R->out + 64, Rr);
        memcpy(R->out + 96, Rr, 160);
    });
    CASE("_sodium_ge25519_double_scalarmult_vartime + tobytes", 96, {
        GETF(ge_sm, sm, "_sodium_ge25519_scalarmult_base");
        GETF(ge_dsm, ds, "_sodium_ge25519_double_scalarmult_vartime");
        GETF(ge_v2p, tb, "_sodium_ge25519_tobytes");
        uc A[512], p2[512];
        memset(A, 0, sizeof A); memset(p2, 0, sizeof p2);
        sm(A, a);
        ds(p2, a, A, b);
        tb(R->out, p2);
        uc zero[32] = { 0 };
        ds(p2, zero, A, zero);
        tb(R->out + 32, p2);
        memcpy(R->out + 64, p2, 32);
    });
    CASE("_sodium_ge25519_p1p1_to_p2/p3 + p2_to_p3", 512, {
        GETF(ge_2p, f1, "_sodium_ge25519_p1p1_to_p2");
        GETF(ge_2p, f2, "_sodium_ge25519_p1p1_to_p3");
        GETF(ge_2p, f3, "_sodium_ge25519_p2_to_p3");
        uc p1p1[160], p2[160], p3[160];
        memset(p1p1, 0, sizeof p1p1);
        for (int i = 0; i < 160; i++) p1p1[i] = (uc) (i * 13 + 7);
        /* keep limbs in range: mask the top byte of each 4-byte limb */
        for (int i = 3; i < 160; i += 4) p1p1[i] = 0;
        memset(p2, 0xAA, sizeof p2); memset(p3, 0xBB, sizeof p3);
        f1(p2, p1p1);
        memcpy(R->out, p2, 120);
        f2(p3, p1p1);
        memcpy(R->out + 120, p3, 160);
        uc p3b[160];
        memset(p3b, 0, sizeof p3b);
        f3(p3b, p2);
        memcpy(R->out + 280, p3b, 160);
    });
    CASE("_sodium_ge25519 predicates", 0, {
        GETF(ge_sm, sm, "_sodium_ge25519_scalarmult_base");
        GETF(ge_pred, oc, "_sodium_ge25519_is_on_curve");
        GETF(ge_pred, ms, "_sodium_ge25519_is_on_main_subgroup");
        GETF(ge_pred, so, "_sodium_ge25519_has_small_order");
        GETF(ge_predb, ic, "_sodium_ge25519_is_canonical");
        GETF(ge_1p, cc, "_sodium_ge25519_clear_cofactor");
        GETF(ge_v2p, tb, "_sodium_ge25519_p3_tobytes");
        uc P[512];
        memset(P, 0, sizeof P);
        sm(P, a);
        long long acc = 0;
        acc = acc * 7 + oc(P);
        acc = acc * 7 + ms(P);
        acc = acc * 7 + so(P);
        acc = acc * 7 + ic(pt);
        acc = acc * 7 + ic(a);
        cc(P);
        uc enc[32];
        tb(enc, P);
        acc = acc * 7 + ic(enc);
        R->ret = acc;
        R->extra = enc[0] + 256ULL * enc[31];
    });
    CASE("_sodium_ristretto255_frombytes/p3_tobytes", 256, {
        GETF(ge_bytes2, fh, "_sodium_ristretto255_from_hash");
        GETF(ge_i2p, fb, "_sodium_ristretto255_frombytes");
        GETF(ge_v2p, tb, "_sodium_ristretto255_p3_tobytes");
        uc s[32], p3[512];
        fh(s, b);
        memcpy(R->out, s, 32);
        memset(p3, 0x33, sizeof p3);
        R->ret = fb(p3, s);
        memcpy(R->out + 32, p3, 160);
        tb(R->out + 192, p3);
        memset(p3, 0x44, sizeof p3);
        R->extra = (unsigned long long) (fb(p3, a) + 100);
    });
}

/* ============ escrypt internals ============ */
{
    const uint8_t *pw = (const uint8_t *) "password";
    const uint8_t *sa = (const uint8_t *) "NaCl";
    CASE("_sodium_escrypt_PBKDF2_SHA256", 128, {
        GETF(es_pbkdf2, f, "_sodium_escrypt_PBKDF2_SHA256");
        f(pw, 8, sa, 4, 3, R->out, 100);
    });
    CASE("_sodium_escrypt_PBKDF2_SHA256 long key", 128, {
        GETF(es_pbkdf2, f, "_sodium_escrypt_PBKDF2_SHA256");
        uint8_t longkey[200];
        for (int i = 0; i < 200; i++) longkey[i] = (uint8_t) (i * 3);
        f(longkey, 200, sa, 4, 7, R->out, 77);
    });
    CASE("_sodium_escrypt_alloc_region/free_region", 64, {
        GETF(es_alloc, al, "_sodium_escrypt_alloc_region");
        GETF(es_free_region, fr, "_sodium_escrypt_free_region");
        escrypt_region_c reg;
        memset(&reg, 0, sizeof reg);
        void *p = al(&reg, 4096);
        R->ret = (p != NULL) * 10 + (reg.size == 4096);
        R->extra = (unsigned long long) ((((uintptr_t) reg.aligned) & 63) == 0);
        R->extra = R->extra * 1000 + (unsigned long long) ((uintptr_t) reg.aligned - (uintptr_t) reg.base);
        R->ret = R->ret * 100 + fr(&reg);
        R->ret = R->ret * 100 + fr(&reg);
    });
    CASE("_sodium_escrypt_init_local/kdf_nosse/free_local", 128, {
        GETF(es_init_local, il, "_sodium_escrypt_init_local");
        GETF(es_kdf, kdf, "_sodium_escrypt_kdf_nosse");
        GETF(es_free_local, fl, "_sodium_escrypt_free_local");
        escrypt_region_c loc;
        R->ret = il(&loc);
        R->ret = R->ret * 100 + kdf(&loc, pw, 8, sa, 4, 1024, 8, 1, R->out, 64);
        R->ret = R->ret * 100 + kdf(&loc, pw, 8, sa, 4, 16, 1, 2, R->out + 64, 32);
        R->ret = R->ret * 100 + kdf(&loc, pw, 8, sa, 4, 0, 8, 1, R->out + 96, 8);
        R->ret = R->ret * 100 + fl(&loc);
    });
    CASE("_sodium_escrypt_gensalt_r", 128, {
        GETF(es_gensalt, f, "_sodium_escrypt_gensalt_r");
        uint8_t src[32];
        for (int i = 0; i < 32; i++) src[i] = (uint8_t) (i * 11 + 3);
        uint8_t *p = f(10, 8, 1, src, 32, R->out, 128);
        R->ret = p ? 1 : 0;
        uint8_t buf2[16];
        R->extra = f(10, 8, 1, src, 32, buf2, 16) ? 1 : 0;
        R->extra = R->extra * 10 + (f(64, 8, 1, src, 32, buf2, 16) ? 1 : 0);
    });
    CASE("_sodium_escrypt_parse_setting", 32, {
        GETF(es_parse, f, "_sodium_escrypt_parse_setting");
        uint32_t N = 0, r = 0, p = 0;
        const uint8_t *e = f((const uint8_t *) "$7$C6..../....abcd", &N, &r, &p);
        R->ret = e ? 1 : 0;
        R->out[0] = (uc) N; R->out[1] = (uc) r; R->out[2] = (uc) p;
        N = r = p = 0;
        R->out[3] = f((const uint8_t *) "$8$C6..../....", &N, &r, &p) ? 1 : 0;
        R->out[4] = f((const uint8_t *) "$7$", &N, &r, &p) ? 1 : 0;
    });
    CASE("_sodium_escrypt_r", 128, {
        GETF(es_init_local, il, "_sodium_escrypt_init_local");
        GETF(es_r, f, "_sodium_escrypt_r");
        GETF(es_free_local, fl, "_sodium_escrypt_free_local");
        escrypt_region_c loc;
        il(&loc);
        uint8_t *p = f(&loc, pw, 8, (const uint8_t *) "$7$C6..../....SodiumChloride", R->out, 128);
        R->ret = p ? 1 : 0;
        uint8_t small[8];
        R->extra = f(&loc, pw, 8, (const uint8_t *) "$7$C6..../....SodiumChloride", small, 8) ? 1 : 0;
        fl(&loc);
    });
}

/* ============ argon2 internals ============ */
{
    uint8_t pwd[32], salt[16], secret[8], ad[12];
    for (int i = 0; i < 32; i++) pwd[i] = (uint8_t) (i * 7 + 1);
    for (int i = 0; i < 16; i++) salt[i] = (uint8_t) (i * 5 + 2);
    for (int i = 0; i < 8; i++) secret[i] = (uint8_t) (i * 3 + 4);
    for (int i = 0; i < 12; i++) ad[i] = (uint8_t) (i * 9 + 5);
    for (int type = 1; type <= 2; type++) {
        char nm[80];
        snprintf(nm, sizeof nm, "_sodium_argon2_ctx type%d", type);
        CASE(nm, 64, {
            GETF(a2_ctx, f, "_sodium_argon2_ctx");
            argon2_context_c c;
            memset(&c, 0, sizeof c);
            c.out = R->out; c.outlen = 32;
            c.pwd = pwd; c.pwdlen = 32;
            c.salt = salt; c.saltlen = 16;
            c.secret = secret; c.secretlen = 8;
            c.ad = ad; c.adlen = 12;
            c.t_cost = 3; c.m_cost = 64; c.lanes = 1; c.threads = 1;
            c.flags = 0;
            R->ret = f(&c, type);
        });
        snprintf(nm, sizeof nm, "_sodium_argon2_ctx lanes4 type%d", type);
        CASE(nm, 64, {
            GETF(a2_ctx, f, "_sodium_argon2_ctx");
            argon2_context_c c;
            memset(&c, 0, sizeof c);
            c.out = R->out; c.outlen = 48;
            c.pwd = pwd; c.pwdlen = 32;
            c.salt = salt; c.saltlen = 16;
            c.t_cost = 2; c.m_cost = 256; c.lanes = 4; c.threads = 4;
            R->ret = f(&c, type);
        });
        snprintf(nm, sizeof nm, "_sodium_argon2_validate_inputs type%d", type);
        CASE(nm, 0, {
            GETF(a2_validate, f, "_sodium_argon2_validate_inputs");
            argon2_context_c c;
            long long acc = 0;
            uc out[64];
            memset(&c, 0, sizeof c);
            c.out = out; c.outlen = 32;
            c.pwd = pwd; c.pwdlen = 32;
            c.salt = salt; c.saltlen = 16;
            c.t_cost = 3; c.m_cost = 64; c.lanes = 1; c.threads = 1;
            acc = acc * 1009 + f(&c);
            c.outlen = 1;  acc = acc * 1009 + f(&c); c.outlen = 32;
            c.saltlen = 1; acc = acc * 1009 + f(&c); c.saltlen = 16;
            c.t_cost = 0;  acc = acc * 1009 + f(&c); c.t_cost = 3;
            c.m_cost = 1;  acc = acc * 1009 + f(&c); c.m_cost = 64;
            c.lanes = 0;   acc = acc * 1009 + f(&c); c.lanes = 1;
            c.out = NULL;  acc = acc * 1009 + f(&c); c.out = out;
            c.pwd = NULL;  acc = acc * 1009 + f(&c); c.pwd = pwd;
            c.pwdlen = 0;  acc = acc * 1009 + f(&c); c.pwdlen = 32;
            R->ret = acc;
        });
        snprintf(nm, sizeof nm, "_sodium_argon2_encode/decode_string type%d", type);
        CASE(nm, 256, {
            GETF(a2_encode, en, "_sodium_argon2_encode_string");
            GETF(a2_decode, de, "_sodium_argon2_decode_string");
            argon2_context_c c;
            uint8_t out[32];
            memset(&c, 0, sizeof c);
            for (int i = 0; i < 32; i++) out[i] = (uint8_t) (i * 17 + type);
            c.out = out; c.outlen = 32;
            c.salt = salt; c.saltlen = 16;
            c.t_cost = 3; c.m_cost = 64; c.lanes = 1; c.threads = 1;
            R->ret = en((char *) R->out, 200, &c, type);
            /* decode into a fresh context */
            argon2_context_c d;
            uint8_t dout[64], dsalt[64], dpwd[64];
            memset(&d, 0, sizeof d);
            d.out = dout; d.outlen = 64;
            d.salt = dsalt; d.saltlen = 64;
            d.pwd = dpwd; d.pwdlen = 64;
            R->ret = R->ret * 1000 + de(&d, (const char *) R->out, type);
            memcpy(R->out + 200, dout, 32);
            R->extra = (unsigned long long) d.t_cost * 1000000ULL
                     + (unsigned long long) d.m_cost * 1000ULL
                     + d.lanes;
            R->out[232] = (uc) d.outlen;
            R->out[233] = (uc) d.saltlen;
            memcpy(R->out + 234, dsalt, 16);
            /* malformed inputs */
            R->out[250] = (uc) (de(&d, "$argon2i$v=19$m=64,t=3,p=1$", type) + 100);
            R->out[251] = (uc) (de(&d, "garbage", type) + 100);
            R->out[252] = (uc) (en((char *) NULL, 0, &c, type) + 100);
            R->out[253] = (uc) (en((char *) R->out + 300, 4, &c, type) + 100);
        });
    }
    CASE("_sodium_argon2_hash raw+encoded", 256, {
        GETF(a2_hash, f, "_sodium_argon2_hash");
        char enc[200];
        memset(enc, 0, sizeof enc);
        R->ret = f(3, 64, 1, pwd, 32, salt, 16, R->out, 32, enc, sizeof enc, 1, 0x13);
        memcpy(R->out + 32, enc, 180);
        R->extra = strlen(enc);
    });
    CASE("_sodium_argon2i_hash_raw/encoded/verify", 256, {
        GETF(a2_hash_raw, hr, "_sodium_argon2i_hash_raw");
        GETF(a2_hash_enc, he, "_sodium_argon2i_hash_encoded");
        GETF(a2_verify, v, "_sodium_argon2i_verify");
        GETF(a2_verify_t, vt, "_sodium_argon2_verify");
        char enc[200];
        memset(enc, 0, sizeof enc);
        R->ret = hr(3, 64, 1, pwd, 32, salt, 16, R->out, 32);
        R->ret = R->ret * 1000 + he(3, 64, 1, pwd, 32, salt, 16, 32, enc, sizeof enc);
        memcpy(R->out + 32, enc, 180);
        R->extra = (unsigned long long) (v(enc, pwd, 32) + 100);
        R->extra = R->extra * 1000 + (unsigned long long) (v(enc, "wrong", 5) + 100);
        R->extra = R->extra * 1000 + (unsigned long long) (vt(enc, pwd, 32, 1) + 100);
    });
    CASE("_sodium_argon2id_hash_raw/encoded/verify", 256, {
        GETF(a2_hash_raw, hr, "_sodium_argon2id_hash_raw");
        GETF(a2_hash_enc, he, "_sodium_argon2id_hash_encoded");
        GETF(a2_verify, v, "_sodium_argon2id_verify");
        GETF(a2_verify_t, vt, "_sodium_argon2_verify");
        char enc[200];
        memset(enc, 0, sizeof enc);
        R->ret = hr(3, 64, 1, pwd, 32, salt, 16, R->out, 32);
        R->ret = R->ret * 1000 + he(3, 64, 1, pwd, 32, salt, 16, 32, enc, sizeof enc);
        memcpy(R->out + 32, enc, 180);
        R->extra = (unsigned long long) (v(enc, pwd, 32) + 100);
        R->extra = R->extra * 1000 + (unsigned long long) (v(enc, "wrong", 5) + 100);
        R->extra = R->extra * 1000 + (unsigned long long) (vt(enc, pwd, 32, 2) + 100);
    });
    CASE("crypto_pwhash_argon2i/id_str_needs_rehash", 0, {
        GETF(fp67, s1, "crypto_pwhash_argon2i_str");
        GETF(fp69, n1, "crypto_pwhash_argon2i_str_needs_rehash");
        GETF(fp69, n2, "crypto_pwhash_argon2id_str_needs_rehash");
        char hash[200];
        memset(hash, 0, sizeof hash);
        s1(hash, "pw", 2, 3, 16384);
        long long acc = 0;
        acc = acc * 1009 + n1(hash, 3, 16384);
        acc = acc * 1009 + n1(hash, 4, 16384);
        acc = acc * 1009 + n1(hash, 3, 32768);
        acc = acc * 1009 + n2(hash, 3, 16384);
        acc = acc * 1009 + n1("garbage", 3, 16384);
        R->ret = acc;
    });
}

/* ============ argon2 low-level pipeline (initialize/fill/finalize) ============ */
{
    uint8_t pwd2[24], salt2[16];
    for (int i = 0; i < 24; i++) pwd2[i] = (uint8_t) (i * 11 + 3);
    for (int i = 0; i < 16; i++) salt2[i] = (uint8_t) (i * 13 + 5);
    for (int type = 1; type <= 2; type++) {
        for (int lanes = 1; lanes <= 2; lanes++) {
            char nm[120];
            snprintf(nm, sizeof nm, "_sodium_argon2_initialize/fill_memory_blocks/finalize t%d l%d", type, lanes);
            CASE(nm, 64, {
                GETF(a2_initialize, init, "_sodium_argon2_initialize");
                GETF(a2_fillmem, fill, "_sodium_argon2_fill_memory_blocks");
                GETF(a2_finalize, fin, "_sodium_argon2_finalize");
                argon2_context_c ctx;
                a2_instance_c    inst;
                memset(&ctx, 0, sizeof ctx);
                memset(&inst, 0, sizeof inst);
                ctx.out = R->out; ctx.outlen = 32;
                ctx.pwd = pwd2; ctx.pwdlen = 24;
                ctx.salt = salt2; ctx.saltlen = 16;
                ctx.t_cost = 3; ctx.m_cost = 64 * lanes; ctx.lanes = (uint32_t) lanes;
                ctx.threads = (uint32_t) lanes;
                inst.passes = ctx.t_cost;
                inst.memory_blocks = ctx.m_cost;
                inst.segment_length = ctx.m_cost / (ctx.lanes * 4);
                inst.lane_length = inst.segment_length * 4;
                inst.lanes = ctx.lanes;
                inst.threads = ctx.threads;
                inst.type = type;
                R->ret = init(&inst, &ctx);
                if (R->ret == 0) {
                    for (uint32_t p = 0; p < inst.passes; p++) {
                        inst.current_pass = p;
                        fill(&inst, p);
                    }
                    fin(&ctx, &inst);
                }
                R->extra = (unsigned long long) inst.memory_blocks * 1000
                         + inst.segment_length;
            });
            snprintf(nm, sizeof nm, "_sodium_argon2_fill_segment_ref t%d l%d", type, lanes);
            CASE(nm, 64 + 1024, {
                GETF(a2_initialize, init, "_sodium_argon2_initialize");
                GETF(a2_fillseg, fs, "_sodium_argon2_fill_segment_ref");
                GETF(a2_finalize, fin, "_sodium_argon2_finalize");
                argon2_context_c ctx;
                a2_instance_c    inst;
                memset(&ctx, 0, sizeof ctx);
                memset(&inst, 0, sizeof inst);
                ctx.out = R->out; ctx.outlen = 32;
                ctx.pwd = pwd2; ctx.pwdlen = 24;
                ctx.salt = salt2; ctx.saltlen = 16;
                ctx.t_cost = 1; ctx.m_cost = 64 * lanes; ctx.lanes = (uint32_t) lanes;
                ctx.threads = (uint32_t) lanes;
                inst.passes = ctx.t_cost;
                inst.memory_blocks = ctx.m_cost;
                inst.segment_length = ctx.m_cost / (ctx.lanes * 4);
                inst.lane_length = inst.segment_length * 4;
                inst.lanes = ctx.lanes;
                inst.threads = ctx.threads;
                inst.type = type;
                R->ret = init(&inst, &ctx);
                if (R->ret == 0) {
                    inst.current_pass = 0;
                    for (uint8_t sl = 0; sl < 4; sl++) {
                        for (uint32_t ln = 0; ln < inst.lanes; ln++) {
                            a2_position_c pos;
                            pos.pass = 0; pos.lane = ln; pos.slice = sl; pos.index = 0;
                            fs(&inst, pos);
                        }
                    }
                    /* snapshot the last block before finalize frees the region */
                    memcpy(R->out + 64,
                           &inst.region->memory[inst.memory_blocks - 1], 1024);
                    fin(&ctx, &inst);
                }
            });
        }
    }
}

/* ============ sodium_misuse ============ */
{
    MCASE("sodium_misuse", {
        v_misuse f = (v_misuse) dlsym(h, "sodium_misuse");
        f();
        _exit(3);
    });
    MCASE("sodium_misuse with handler", {
        v_setmis sm = (v_setmis) dlsym(h, "sodium_set_misuse_handler");
        v_misuse f = (v_misuse) dlsym(h, "sodium_misuse");
        sm(NULL);
        f();
        _exit(3);
    });
}

/* ============ exported implementation structs ============ */
{
    uc key[32], n8[8], n12[12], msg[200], npub[32], ad[16], ip[16], tw[16];
    fillr(key, 32); fillr(n8, 8); fillr(n12, 12); fillr(msg, sizeof msg);
    fillr(npub, 32); fillr(ad, 16); fillr(ip, 16); fillr(tw, 16);
    CASE("crypto_stream_salsa20_ref_implementation", 400, {
        GETD(salsa20_impl_c *, im, "crypto_stream_salsa20_ref_implementation");
        R->ret  = im->stream(R->out, 200, n8, key);
        R->ret |= im->stream_xor_ic(R->out + 200, msg, 200, n8, 3, key);
    });
    CASE("crypto_stream_chacha20_ref_implementation", 800, {
        GETD(chacha20_impl_c *, im, "crypto_stream_chacha20_ref_implementation");
        R->ret  = im->stream(R->out, 200, n8, key);
        R->ret |= im->stream_ietf_ext(R->out + 200, 200, n12, key);
        R->ret |= im->stream_xor_ic(R->out + 400, msg, 200, n8, 5, key);
        R->ret |= im->stream_ietf_ext_xor_ic(R->out + 600, msg, 200, n12, 7, key);
    });
    CASE("crypto_onetimeauth_poly1305_donna_implementation", 64, {
        GETD(poly1305_impl_c *, im, "crypto_onetimeauth_poly1305_donna_implementation");
        R->ret = im->onetimeauth(R->out, msg, 137, key);
        R->ret = R->ret * 10 + im->onetimeauth_verify(R->out, msg, 137, key);
        uc st[256] __attribute__((aligned(16)));
        memset(st, 0, sizeof st);
        R->ret = R->ret * 10 + im->onetimeauth_init(st, key);
        R->ret = R->ret * 10 + im->onetimeauth_update(st, msg, 100);
        R->ret = R->ret * 10 + im->onetimeauth_update(st, msg + 100, 37);
        R->ret = R->ret * 10 + im->onetimeauth_final(st, R->out + 16);
    });
    CASE("crypto_scalarmult_curve25519_ref10_implementation", 64, {
        GETD(x25519_impl_c *, im, "crypto_scalarmult_curve25519_ref10_implementation");
        uc n[32];
        memcpy(n, key, 32);
        n[0] &= 248; n[31] &= 127; n[31] |= 64;
        R->ret = im->mult_base(R->out, n);
        R->ret |= im->mult(R->out + 32, n, R->out);
    });
    CASE("aegis128l_soft_implementation", 512, {
        GETD(aegis_impl_c *, im, "aegis128l_soft_implementation");
        R->ret = im->encrypt_detached(R->out, R->out + 200, 32, msg, 200, ad, 16, npub, key);
        R->ret |= im->decrypt_detached(R->out + 256, R->out, 200, R->out + 200, 32, ad, 16, npub, key);
        R->out[240] = (uc) (im->decrypt_detached(R->out + 400, R->out, 200, ad, 32, ad, 16, npub, key) + 100);
    });
    CASE("aegis256_soft_implementation", 512, {
        GETD(aegis_impl_c *, im, "aegis256_soft_implementation");
        R->ret = im->encrypt_detached(R->out, R->out + 200, 32, msg, 200, ad, 16, npub, key);
        R->ret |= im->decrypt_detached(R->out + 256, R->out, 200, R->out + 200, 32, ad, 16, npub, key);
        R->out[240] = (uc) (im->decrypt_detached(R->out + 400, R->out, 200, ad, 32, ad, 16, npub, key) + 100);
    });
    CASE("ipcrypt_soft_implementation", 256, {
        GETD(ipcrypt_impl_c *, im, "ipcrypt_soft_implementation");
        uc k64[64];
        fillr(k64, 64);
        memcpy(k64, key, 32);
        memcpy(k64 + 32, key, 32);
        im->encrypt(R->out, ip, k64);
        im->decrypt(R->out + 16, R->out, k64);
        im->nd_encrypt(R->out + 32, ip, tw, k64);
        im->nd_decrypt(R->out + 56, R->out + 32, k64);
        im->ndx_encrypt(R->out + 72, ip, tw, k64);
        im->ndx_decrypt(R->out + 104, R->out + 72, k64);
        im->pfx_encrypt(R->out + 120, ip, k64);
        im->pfx_decrypt(R->out + 136, R->out + 120, k64);
    });
    static const char *const rbi[] = {
        "randombytes_sysrandom_implementation", "randombytes_internal_implementation", NULL
    };
    for (int k = 0; rbi[k]; k++) {
        CASE(rbi[k], 64, {
            GETD(rb_impl_c *, im, rbi[k]);
            const char *n = im->implementation_name();
            size_t l = strlen(n);
            if (l > 40) l = 40;
            memcpy(R->out, n, l + 1);
            R->ret = (im->random != NULL) * 8 + (im->stir != NULL) * 4
                   + (im->uniform != NULL) * 2 + (im->buf != NULL);
            R->extra = (im->close != NULL);
        });
    }
}

/* ============ sodium memory helpers ============ */
{
    CASE("sodium_malloc/free", 64, {
        GETF(v_malloc, m, "sodium_malloc");
        GETF(v_free, fr, "sodium_free");
        unsigned char *p = m(37);
        R->ret = p != NULL;
        if (p) { memcpy(R->out, p, 37); fr(p); }
        p = m(0);
        R->ret = R->ret * 10 + (p != NULL);
        fr(p);
        fr(NULL);
    });
    CASE("sodium_allocarray", 64, {
        GETF(v_allocarray, m, "sodium_allocarray");
        GETF(v_free, fr, "sodium_free");
        unsigned char *p = m(7, 5);
        R->ret = p != NULL;
        if (p) { memcpy(R->out, p, 35); fr(p); }
        errno = 0;
        p = m((size_t) -1, 4);
        R->ret = R->ret * 10 + (p == NULL);
        R->extra = (unsigned long long) errno;
    });
    CASE("sodium_mlock/munlock/mprotect", 64, {
        GETF(v_mlock, ml, "sodium_mlock");
        GETF(v_mlock, mu, "sodium_munlock");
        GETF(v_mprot, na, "sodium_mprotect_noaccess");
        GETF(v_mprot, ro, "sodium_mprotect_readonly");
        GETF(v_mprot, rw, "sodium_mprotect_readwrite");
        unsigned char buf[64];
        memset(buf, 0x5a, sizeof buf);
        errno = 0;
        R->ret = ml(buf, 64);
        R->extra = (unsigned long long) errno;
        errno = 0;
        R->ret = R->ret * 10 + mu(buf, 64);
        R->extra = R->extra * 1000 + (unsigned long long) errno;
        memcpy(R->out, buf, 64);
        R->ret = R->ret * 10 + na(buf);
        R->ret = R->ret * 10 + ro(buf);
        R->ret = R->ret * 10 + rw(buf);
    });
    CASE("sodium_memzero/stackzero", 64, {
        GETF(v_memzero, mz, "sodium_memzero");
        GETF(v_stackzero, sz, "sodium_stackzero");
        memset(R->out, 0x77, 64);
        mz(R->out + 8, 40);
        mz(R->out, 0);
        sz(128);
        R->ret = 0;
    });
    CASE("sodium_set_misuse_handler", 0, {
        GETF(v_setmis, f, "sodium_set_misuse_handler");
        R->ret = f(NULL);
    });
}

/* ============ randombytes plumbing ============ */
{
    CASE("randombytes_buf/random/stir/close", 64, {
        GETF(v_buf, bf, "randombytes_buf");
        GETF(v_u32, rnd, "randombytes_random");
        GETF(v_void, st, "randombytes_stir");
        GETF(v_void_int, cl, "randombytes_close");
        GETF(v_rb, rb, "randombytes");
        GETF(fp1, nm, "randombytes_implementation_name");
        st();
        bf(R->out, 32);
        rb(R->out + 32, 24);
        unsigned long long acc = 0;
        for (int i = 0; i < 8; i++) acc = acc * 131 + rnd();
        R->extra = acc;
        R->ret = cl();
        const char *n = nm();
        size_t l = strlen(n);
        if (l > 20) l = 20;
        memcpy(R->out + 56, n, l);
    });
    CASE("randombytes_set_implementation(internal)", 128, {
        GETF(v_setimpl, si, "randombytes_set_implementation");
        GETF(v_buf, bf, "randombytes_buf");
        GETF(v_u32, rnd, "randombytes_random");
        GETF(v_void, st, "randombytes_stir");
        GETF(v_void_int, cl, "randombytes_close");
        GETD(void *, im, "randombytes_internal_implementation");
        R->ret = si(im);
        st();
        /* internal RNG is seeded from the OS -> only structural checks */
        bf(R->out, 64);
        (void) rnd();
        R->ret = R->ret * 10 + (cl() == 0 || 1);
        memset(R->out, 0, 64);
        /* restore the deterministic implementation for later cases */
        si(&det_impl);
    });
}

/* ============ API corners not named by the first batch ============ */
{
    uc key[32], npub[32], msg[300], ad[19], n24[24], seed[32];
    fillr(key, 32); fillr(npub, 32); fillr(msg, sizeof msg); fillr(ad, sizeof ad);
    fillr(n24, 24); fillr(seed, 32);

    /* AEAD *_decrypt_detached */
    struct { const char *pfx; int abytes; } de[] = {
        { "crypto_aead_chacha20poly1305", 16 },
        { "crypto_aead_chacha20poly1305_ietf", 16 },
        { "crypto_aead_xchacha20poly1305_ietf", 16 },
        { "crypto_aead_aegis128l", 32 },
        { "crypto_aead_aegis256", 32 },
    };
    for (unsigned j = 0; j < sizeof de / sizeof de[0]; j++) {
        for (int len = 0; len <= 300; len = len ? len * 9 + 7 : 1) {
            char nm[160], se[160], sd[160];
            snprintf(nm, sizeof nm, "%s_decrypt_detached len%d", de[j].pfx, len);
            snprintf(se, sizeof se, "%s_encrypt_detached", de[j].pfx);
            snprintf(sd, sizeof sd, "%s_decrypt_detached", de[j].pfx);
            CASE(nm, (size_t) len, {
                GETF(fp41, e, se);
                GETF(aead_dd, f, sd);
                uc ct[400], mac[64];
                ull maclen = 0;
                e(ct, mac, &maclen, msg, (ull) len, ad, sizeof ad, NULL, npub, key);
                R->ret = f(R->out, NULL, ct, (ull) len, mac, ad, sizeof ad, npub, key);
                mac[0] ^= 0x20;
                R->extra = (unsigned long long) (f(R->out, NULL, ct, (ull) len, mac, ad, sizeof ad, npub, key) + 100);
            });
        }
    }
    /* aes256gcm: everything must fail identically */
    CASE("crypto_aead_aes256gcm all ops", 256, {
        GETF(fp42, av, "crypto_aead_aes256gcm_is_available");
        GETF(fp28, bn, "crypto_aead_aes256gcm_beforenm");
        GETF(fp39, e, "crypto_aead_aes256gcm_encrypt");
        GETF(fp40, d, "crypto_aead_aes256gcm_decrypt");
        GETF(fp41, ed, "crypto_aead_aes256gcm_encrypt_detached");
        GETF(aead_dd, dd, "crypto_aead_aes256gcm_decrypt_detached");
        long long acc = av();
        uc st[560] __attribute__((aligned(16)));
        memset(st, 0, sizeof st);
        errno = 0;
        acc = acc * 1009 + bn((uc *) st, 512, key, 32);
        R->extra = (unsigned long long) errno;
        ull clen = 0, mlen = 0, maclen = 0;
        acc = acc * 1009 + e(R->out, &clen, msg, 100, ad, sizeof ad, NULL, npub, key);
        acc = acc * 1009 + d(R->out, &mlen, NULL, msg, 116, ad, sizeof ad, npub, key);
        acc = acc * 1009 + ed(R->out, R->out + 100, &maclen, msg, 100, ad, sizeof ad, NULL, npub, key);
        acc = acc * 1009 + dd(R->out, NULL, msg, 100, msg, ad, sizeof ad, npub, key);
        R->ret = acc;
        R->extra = R->extra * 1000 + (unsigned long long) (clen + mlen + maclen);
    });
    CASE("crypto_aead_aes256gcm afternm ops", 256, {
        GETF(fp42, av, "crypto_aead_aes256gcm_is_available");
        long long acc = av();
        uc st[560] __attribute__((aligned(16)));
        memset(st, 0, sizeof st);
        ull clen = 0, mlen = 0, maclen = 0;
        {
            GETF(fp39, e, "crypto_aead_aes256gcm_encrypt_afternm");
            GETF(fp40, d, "crypto_aead_aes256gcm_decrypt_afternm");
            GETF(fp41, ed, "crypto_aead_aes256gcm_encrypt_detached_afternm");
            GETF(aead_dd, dd, "crypto_aead_aes256gcm_decrypt_detached_afternm");
            errno = 0;
            acc = acc * 1009 + e(R->out, &clen, msg, 100, ad, sizeof ad, NULL, npub, (const uc *) st);
            acc = acc * 1009 + d(R->out, &mlen, NULL, msg, 116, ad, sizeof ad, npub, (const uc *) st);
            acc = acc * 1009 + ed(R->out, R->out + 100, &maclen, msg, 100, ad, sizeof ad, NULL, npub, (const uc *) st);
            acc = acc * 1009 + dd(R->out, NULL, msg, 100, msg, ad, sizeof ad, npub, (const uc *) st);
            R->extra = (unsigned long long) errno;
        }
        R->ret = acc;
    });

    /* secretbox open variants that were not named */
    for (int len = 0; len <= 300; len = len ? len * 9 + 7 : 1) {
        char nm[128];
        snprintf(nm, sizeof nm, "crypto_secretbox_xchacha20poly1305_open_easy len%d", len);
        CASE(nm, (size_t) len, {
            GETF(fp19, e, "crypto_secretbox_xchacha20poly1305_easy");
            GETF(fp19, f, "crypto_secretbox_xchacha20poly1305_open_easy");
            uc ct[400];
            e(ct, msg, (ull) len, n24, key);
            R->ret = f(R->out, ct, (ull) len + 16, n24, key);
            ct[0] ^= 1;
            R->extra = (unsigned long long) (f(R->out, ct, (ull) len + 16, n24, key) + 100);
        });
        snprintf(nm, sizeof nm, "crypto_secretbox_xchacha20poly1305_open_detached len%d", len);
        CASE(nm, (size_t) len, {
            GETF(sb_det, e, "crypto_secretbox_xchacha20poly1305_detached");
            GETF(sb_od, f, "crypto_secretbox_xchacha20poly1305_open_detached");
            uc ct[400], mac[16];
            e(ct, mac, msg, (ull) len, n24, key);
            R->ret = f(R->out, ct, mac, (ull) len, n24, key);
            mac[3] ^= 8;
            R->extra = (unsigned long long) (f(R->out, ct, mac, (ull) len, n24, key) + 100);
        });
        snprintf(nm, sizeof nm, "crypto_secretbox_xsalsa20poly1305_open len%d", len);
        CASE(nm, (size_t) len + 32, {
            GETF(fp19, e, "crypto_secretbox_xsalsa20poly1305");
            GETF(fp19, f, "crypto_secretbox_xsalsa20poly1305_open");
            uc padded[400], ct[400];
            memset(padded, 0, 32);
            memcpy(padded + 32, msg, (size_t) len);
            e(ct, padded, (ull) len + 32, n24, key);
            R->ret = f(R->out, ct, (ull) len + 32, n24, key);
            ct[33] ^= 1;
            R->extra = (unsigned long long) (f(R->out, ct, (ull) len + 32, n24, key) + 100);
        });
    }

    /* box *_afternm variants */
    CASE("crypto_box afternm family", 512, {
        GETF(fp55, kp, "crypto_box_seed_keypair");
        GETF(fp50, bn, "crypto_box_beforenm");
        GETF(fp19, ea, "crypto_box_easy_afternm");
        GETF(fp19, oa, "crypto_box_open_easy_afternm");
        GETF(sb_det, da, "crypto_box_detached_afternm");
        GETF(sb_od, oda, "crypto_box_open_detached_afternm");
        GETF(fp19, na, "crypto_box_afternm");
        GETF(fp19, noa, "crypto_box_open_afternm");
        uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32], k[32], ct[400], mac[16], padded[400];
        memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
        kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
        R->ret = bn(k, pk2, sk1);
        R->ret |= ea(ct, msg, 100, n24, k);
        memcpy(R->out, ct, 116);
        R->ret |= oa(R->out + 116, ct, 116, n24, k);
        R->ret |= da(ct, mac, msg, 100, n24, k);
        memcpy(R->out + 216, ct, 100);
        memcpy(R->out + 316, mac, 16);
        R->ret |= oda(R->out + 332, ct, mac, 100, n24, k);
        memset(padded, 0, 32);
        memcpy(padded + 32, msg, 100);
        R->ret |= na(ct, padded, 132, n24, k);
        memcpy(R->out + 432, ct + 16, 60);
        R->ret |= noa(padded, ct, 132, n24, k);
    });
    CASE("crypto_box_curve25519xchacha20poly1305 afternm family", 512, {
        GETF(fp55, kp, "crypto_box_curve25519xchacha20poly1305_seed_keypair");
        GETF(fp50, bn, "crypto_box_curve25519xchacha20poly1305_beforenm");
        GETF(fp19, ea, "crypto_box_curve25519xchacha20poly1305_easy_afternm");
        GETF(fp19, oa, "crypto_box_curve25519xchacha20poly1305_open_easy_afternm");
        GETF(sb_det, da, "crypto_box_curve25519xchacha20poly1305_detached_afternm");
        GETF(sb_od, oda, "crypto_box_curve25519xchacha20poly1305_open_detached_afternm");
        GETF(fp58, det, "crypto_box_curve25519xchacha20poly1305_detached");
        GETF(box_od, od, "crypto_box_curve25519xchacha20poly1305_open_detached");
        uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32], k[32], ct[400], mac[16];
        memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
        kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
        R->ret = bn(k, pk2, sk1);
        R->ret |= ea(ct, msg, 100, n24, k);
        memcpy(R->out, ct, 116);
        R->ret |= oa(R->out + 116, ct, 116, n24, k);
        R->ret |= da(ct, mac, msg, 100, n24, k);
        memcpy(R->out + 216, ct, 100);
        memcpy(R->out + 316, mac, 16);
        R->ret |= oda(R->out + 332, ct, mac, 100, n24, k);
        R->ret |= det(ct, mac, msg, 100, n24, pk2, sk1);
        memcpy(R->out + 432, mac, 16);
        R->ret |= od(R->out + 448, ct, mac, 100, n24, pk1, sk2);
    });
    CASE("crypto_box_curve25519xsalsa20poly1305 open/afternm", 512, {
        GETF(fp55, kp, "crypto_box_curve25519xsalsa20poly1305_seed_keypair");
        GETF(fp50, bn, "crypto_box_curve25519xsalsa20poly1305_beforenm");
        GETF(fp19, a, "crypto_box_curve25519xsalsa20poly1305_afternm");
        GETF(fp19, oa, "crypto_box_curve25519xsalsa20poly1305_open_afternm");
        GETF(fp58, o2, "crypto_box_curve25519xsalsa20poly1305_open");
        GETF(fp58, e, "crypto_box_curve25519xsalsa20poly1305");
        uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32], k[32], ct[400], padded[400];
        memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
        kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
        bn(k, pk2, sk1);
        memset(padded, 0, 32);
        memcpy(padded + 32, msg, 100);
        R->ret  = a(ct, padded, 132, n24, k);
        memcpy(R->out, ct + 16, 116);
        R->ret |= oa(R->out + 132, ct, 132, n24, k);
        R->ret |= ((int (*)(uc *, const uc *, ull, const uc *, const uc *, const uc *)) e)(ct, padded, 132, n24, pk2, sk1);
        R->ret |= ((int (*)(uc *, const uc *, ull, const uc *, const uc *, const uc *)) o2)(R->out + 264, ct, 132, n24, pk1, sk2);
    });
    CASE("crypto_box_primitive", 64, {
        GETF(fp1, f, "crypto_box_primitive");
        const char *s = f();
        size_t l = strlen(s);
        if (l > 60) l = 60;
        memcpy(R->out, s, l + 1);
    });

    /* crypto_sign_ed25519 explicit spellings + ph API */
    CASE("crypto_sign_ed25519 family", 800, {
        GETF(fp55, kp, "crypto_sign_ed25519_seed_keypair");
        GETF(fp56, kp2, "crypto_sign_ed25519_keypair");
        GETF(fp59, sg, "crypto_sign_ed25519");
        GETF(fp59, op, "crypto_sign_ed25519_open");
        GETF(fp59, sd, "crypto_sign_ed25519_detached");
        GETF(fp37, vd, "crypto_sign_ed25519_verify_detached");
        uc pk[32], sk[64], sm[400];
        R->ret = kp(pk, sk, seed);
        ull smlen = 0;
        R->ret |= sg(sm, &smlen, msg, 137, sk);
        memcpy(R->out, sm, 201);
        ull mlen = 0;
        R->ret |= op(R->out + 201, &mlen, sm, smlen, pk);
        R->extra = smlen * 1000 + mlen;
        ull siglen = 0;
        R->ret |= sd(R->out + 400, &siglen, msg, 137, sk);
        R->ret = R->ret * 10 + vd(R->out + 400, msg, 137, pk);
        uc pk2[32], sk2[64];
        R->ret = R->ret * 10 + kp2(pk2, sk2);
        memcpy(R->out + 470, pk2, 32);
        memcpy(R->out + 502, sk2, 64);
    });
    CASE("crypto_sign_ed25519ph family", 300, {
        GETF(fp55, kp, "crypto_sign_ed25519_seed_keypair");
        GETF(fp24, sb, "crypto_sign_ed25519ph_statebytes");
        GETF(fp25, fi, "crypto_sign_ed25519ph_init");
        GETF(fp26, fu, "crypto_sign_ed25519ph_update");
        GETF(fp60, fc, "crypto_sign_ed25519ph_final_create");
        GETF(fp46, fv, "crypto_sign_ed25519ph_final_verify");
        uc pk[32], sk[64];
        kp(pk, sk, seed);
        void *st = calloc(1, sb() + 64);
        R->ret = fi(st);
        R->ret |= fu(st, msg, 100);
        R->ret |= fu(st, msg + 100, 200);
        ull sl = 0;
        R->ret |= fc(st, R->out, &sl, sk);
        memcpy(R->out + 64, st, sb());
        void *st2 = calloc(1, sb() + 64);
        fi(st2);
        fu(st2, msg, 300);
        R->extra = (unsigned long long) (fv(st2, R->out, pk) + 100) * 1000 + sb();
        free(st); free(st2);
    });
    CASE("_crypto_sign_ed25519_detached/verify + hinit", 500, {
        GETF(fp55, kp, "crypto_sign_ed25519_seed_keypair");
        GETF(fp61x, hi, "_crypto_sign_ed25519_ref10_hinit");
        uc pk[32], sk[64];
        kp(pk, sk, seed);
        uc hs[208];
        for (int ph = -1; ph <= 2; ph++) {
            memset(hs, 0, sizeof hs);
            hi(hs, ph);
            memcpy(R->out + (ph + 1) * 104, hs, 104);
        }
        GETF(fp62x, sd, "_crypto_sign_ed25519_detached");
        GETF(fp63x, vd, "_crypto_sign_ed25519_verify_detached");
        ull sl = 0;
        R->ret = sd(R->out + 416, &sl, msg, 137, sk, 0);
        R->ret = R->ret * 10 + vd(R->out + 416, msg, 137, pk, 0);
        R->ret = R->ret * 10 + vd(R->out + 416, msg, 137, pk, 1);
    });

    /* secretstream tag accessors */
    static const char *const tg[] = {
        "crypto_secretstream_xchacha20poly1305_tag_message",
        "crypto_secretstream_xchacha20poly1305_tag_push",
        "crypto_secretstream_xchacha20poly1305_tag_rekey",
        "crypto_secretstream_xchacha20poly1305_tag_final",
        NULL
    };
    for (int k = 0; tg[k]; k++) {
        CASE(tg[k], 0, {
            GETF(fpuc_void, f, tg[k]);
            R->ret = (long long) f();
        });
    }
}

#undef CASE
#undef GETF
#undef GETD
