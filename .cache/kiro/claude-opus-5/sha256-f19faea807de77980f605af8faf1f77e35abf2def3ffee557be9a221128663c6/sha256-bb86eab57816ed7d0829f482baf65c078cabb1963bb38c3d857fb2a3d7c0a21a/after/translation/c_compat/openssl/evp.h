/* Minimal shim so that c_src/app/src/rng.c can be compiled on hosts without
 * openssl-devel installed.  Only the handful of EVP entry points that rng.c
 * actually calls are declared; the real implementations come from the system
 * libcrypto.so.3 at link time.  Nothing in c_src/ is modified. */
#ifndef SHIM_OPENSSL_EVP_H
#define SHIM_OPENSSL_EVP_H

typedef struct evp_cipher_ctx_st EVP_CIPHER_CTX;
typedef struct evp_cipher_st EVP_CIPHER;
typedef struct engine_st ENGINE;

EVP_CIPHER_CTX *EVP_CIPHER_CTX_new(void);
void EVP_CIPHER_CTX_free(EVP_CIPHER_CTX *c);
const EVP_CIPHER *EVP_aes_256_ecb(void);
int EVP_EncryptInit_ex(EVP_CIPHER_CTX *ctx, const EVP_CIPHER *cipher,
                       ENGINE *impl, const unsigned char *key,
                       const unsigned char *iv);
int EVP_EncryptUpdate(EVP_CIPHER_CTX *ctx, unsigned char *out, int *outl,
                      const unsigned char *in, int inl);

#endif
