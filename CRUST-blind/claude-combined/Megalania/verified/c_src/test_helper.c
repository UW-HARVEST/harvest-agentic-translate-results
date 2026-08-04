/* Helper to dump test outputs */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "src/lzma_state.h"
#include "src/lzma_packet.h"
#include "src/probability_model.h"
#include "src/encoder_interface.h"
#include "src/perplexity_encoder.h"
#include "src/range_encoder.h"
#include "src/output_interface.h"
#include "src/packet_slab.h"
#include "src/packet_slab_undo_stack.h"
#include "src/lzma_packet_encoder.h"
#include "src/lzma_header_encoder.h"
#include "src/substring_enumerator.h"
#include "src/probability.h"

typedef struct {
    uint8_t* buf;
    size_t cap;
    size_t len;
} BufOut;

static bool buf_write(OutputInterface* o, const void* data, size_t sz) {
    BufOut* b = (BufOut*)o->private_data;
    if (b->len + sz > b->cap) {
        b->cap = (b->len + sz) * 2;
        b->buf = realloc(b->buf, b->cap);
    }
    memcpy(b->buf + b->len, data, sz);
    b->len += sz;
    return true;
}

int main(int argc, char** argv) {
    if (argc < 2) return 1;
    const char* what = argv[1];

    if (!strcmp(what, "header")) {
        // arg2: lc, arg3: lp, arg4: pb, arg5: data_size
        unsigned lc = atoi(argv[2]);
        unsigned lp = atoi(argv[3]);
        unsigned pb = atoi(argv[4]);
        unsigned data_size = atoi(argv[5]);
        uint8_t fake_data[1024] = {0};
        LZMAState s;
        LZMAProperties props = { .lc = lc, .lp = lp, .pb = pb };
        lzma_state_init(&s, fake_data, data_size, props);

        BufOut b = { .buf = malloc(64), .cap = 64, .len = 0 };
        OutputInterface oi = { .write = buf_write, .private_data = &b };
        lzma_encode_header(&s, &oi);
        printf("len=%zu bytes=", b.len);
        for (size_t i = 0; i < b.len; i++) printf("%02x", b.buf[i]);
        printf("\n");
        free(b.buf);
    } else if (!strcmp(what, "encode_bit_perp")) {
        // simulates: encode a sequence of bits with given probabilities
        // arg2: list of bits as 0/1, arg3: list of probs
        // for simplicity: test single bit
        bool bit = atoi(argv[2]);
        Prob prob = atoi(argv[3]);
        uint64_t perp = 0;
        EncoderInterface enc;
        perplexity_encoder_new(&enc, &perp);
        Prob p = prob;
        encode_bit(bit, &p, &enc);
        printf("perp=%lu prob_after=%u\n", perp, p);
    } else if (!strcmp(what, "encode_bit_tree_perp")) {
        unsigned bits = atoi(argv[2]);
        unsigned num_bits = atoi(argv[3]);
        uint64_t perp = 0;
        EncoderInterface enc;
        perplexity_encoder_new(&enc, &perp);
        size_t arr_sz = 1 << num_bits;
        Prob* probs = malloc(sizeof(Prob)*arr_sz);
        for (size_t i = 0; i < arr_sz; i++) probs[i] = PROB_INIT_VAL;
        encode_bit_tree(bits, probs, num_bits, &enc);
        printf("perp=%lu probs=", perp);
        for (size_t i = 0; i < arr_sz; i++) printf("%u,", probs[i]);
        printf("\n");
        free(probs);
    } else if (!strcmp(what, "encode_bit_tree_rev_perp")) {
        unsigned bits = atoi(argv[2]);
        unsigned num_bits = atoi(argv[3]);
        uint64_t perp = 0;
        EncoderInterface enc;
        perplexity_encoder_new(&enc, &perp);
        size_t arr_sz = 1 << num_bits;
        Prob* probs = malloc(sizeof(Prob)*arr_sz);
        for (size_t i = 0; i < arr_sz; i++) probs[i] = PROB_INIT_VAL;
        encode_bit_tree_reverse(bits, probs, num_bits, &enc);
        printf("perp=%lu probs=", perp);
        for (size_t i = 0; i < arr_sz; i++) printf("%u,", probs[i]);
        printf("\n");
        free(probs);
    } else if (!strcmp(what, "ctx_state_update")) {
        // arg2: initial ctx_state, arg3: packet_type
        uint8_t initial = atoi(argv[2]);
        unsigned ptype = atoi(argv[3]);
        uint8_t fake_data[16] = {0};
        LZMAState s;
        LZMAProperties props = { .lc = 0, .lp = 0, .pb = 0 };
        lzma_state_init(&s, fake_data, 16, props);
        s.ctx_state = initial;
        lzma_state_update_ctx_state(&s, ptype);
        printf("ctx_state=%u\n", s.ctx_state);
    } else if (!strcmp(what, "push_dist")) {
        uint8_t fake_data[16] = {0};
        LZMAState s;
        LZMAProperties props = { .lc = 0, .lp = 0, .pb = 0 };
        lzma_state_init(&s, fake_data, 16, props);
        for (int i = 2; i < argc; i++) {
            uint32_t d = (uint32_t)atoi(argv[i]);
            lzma_state_push_distance(&s, d);
        }
        printf("dists=%u,%u,%u,%u\n", s.dists[0], s.dists[1], s.dists[2], s.dists[3]);
    } else if (!strcmp(what, "promote_dist")) {
        uint8_t fake_data[16] = {0};
        LZMAState s;
        LZMAProperties props = { .lc = 0, .lp = 0, .pb = 0 };
        lzma_state_init(&s, fake_data, 16, props);
        s.dists[0] = atoi(argv[2]);
        s.dists[1] = atoi(argv[3]);
        s.dists[2] = atoi(argv[4]);
        s.dists[3] = atoi(argv[5]);
        unsigned dist_index = atoi(argv[6]);
        lzma_state_promote_distance_at(&s, dist_index);
        printf("dists=%u,%u,%u,%u\n", s.dists[0], s.dists[1], s.dists[2], s.dists[3]);
    }
    return 0;
}
