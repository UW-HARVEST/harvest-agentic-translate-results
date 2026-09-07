/* Shim: only ERR_print_errors_fp() is used by rng.c's handleErrors(). */
#ifndef SHIM_OPENSSL_ERR_H
#define SHIM_OPENSSL_ERR_H
#include <stdio.h>
#include <stdlib.h>
void ERR_print_errors_fp(FILE *fp);
#endif
