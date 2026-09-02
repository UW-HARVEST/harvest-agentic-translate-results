#ifndef COMPAT_OPENSSL_EVP_H
#define COMPAT_OPENSSL_EVP_H
#include <stddef.h>
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
