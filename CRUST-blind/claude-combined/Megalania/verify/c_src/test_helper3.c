/* Tests for range encoder: capture bytes written */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "src/range_encoder.h"
#include "src/output_interface.h"
#include "src/probability.h"

typedef struct {
    uint8_t buf[1024];
    size_t len;
} BufOut;

static bool buf_write(OutputInterface* o, const void* d, size_t sz) {
    BufOut* b = (BufOut*)o->private_data;
    memcpy(b->buf + b->len, d, sz);
    b->len += sz;
    return true;
}

int main(int argc, char** argv) {
    if (argc < 2) return 1;
    const char* what = argv[1];

    if (!strcmp(what, "encode_bits")) {
        // arg2: comma list of 0/1 bits, arg3: prob
        // e.g., encode_bits 0,0,0,0 1024
        BufOut b = {{0}, 0};
        OutputInterface oi = { .write = buf_write, .private_data = &b };
        EncoderInterface enc;
        range_encoder_new(&enc, &oi);
        char* bitlist = strdup(argv[2]);
        Prob prob = atoi(argv[3]);
        char* tok = strtok(bitlist, ",");
        while (tok) {
            int bit = atoi(tok);
            (*enc.encode_bit)(&enc, bit, prob);
            tok = strtok(NULL, ",");
        }
        range_encoder_free(&enc);
        free(bitlist);
        printf("len=%zu bytes=", b.len);
        for (size_t i = 0; i < b.len; i++) printf("%02x", b.buf[i]);
        printf("\n");
    } else if (!strcmp(what, "encode_direct")) {
        // arg2: bits, arg3: num_bits (just one direct bits call)
        BufOut b = {{0}, 0};
        OutputInterface oi = { .write = buf_write, .private_data = &b };
        EncoderInterface enc;
        range_encoder_new(&enc, &oi);
        unsigned bits = strtoul(argv[2], NULL, 0);
        unsigned num_bits = atoi(argv[3]);
        (*enc.encode_direct_bits)(&enc, bits, num_bits);
        range_encoder_free(&enc);
        printf("len=%zu bytes=", b.len);
        for (size_t i = 0; i < b.len; i++) printf("%02x", b.buf[i]);
        printf("\n");
    } else if (!strcmp(what, "encode_just_flush")) {
        // No bits encoded, just flush
        BufOut b = {{0}, 0};
        OutputInterface oi = { .write = buf_write, .private_data = &b };
        EncoderInterface enc;
        range_encoder_new(&enc, &oi);
        range_encoder_free(&enc);
        printf("len=%zu bytes=", b.len);
        for (size_t i = 0; i < b.len; i++) printf("%02x", b.buf[i]);
        printf("\n");
    }
    return 0;
}
