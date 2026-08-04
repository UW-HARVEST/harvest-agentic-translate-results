/* Helper to dump test outputs for lzma_packet_encoder */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "src/lzma_state.h"
#include "src/lzma_packet.h"
#include "src/lzma_packet_encoder.h"
#include "src/perplexity_encoder.h"
#include "src/encoder_interface.h"
#include "src/probability.h"

int main(int argc, char** argv) {
    if (argc < 2) return 1;
    const char* what = argv[1];

    if (!strcmp(what, "encode_literal_simple")) {
        // Encode a single literal at position 0 with data[0] given as arg
        uint8_t value = (uint8_t)atoi(argv[2]);
        uint8_t data[8] = {value, 0, 0, 0, 0, 0, 0, 0};
        LZMAState s;
        LZMAProperties p = { .lc = 0, .lp = 0, .pb = 0 };
        lzma_state_init(&s, data, 8, p);

        uint64_t perp = 0;
        EncoderInterface enc;
        perplexity_encoder_new(&enc, &perp);
        LZMAPacket pkt = literal_packet();
        lzma_encode_packet(&s, &enc, pkt);
        printf("perp=%lu position=%zu ctx_state=%u\n", perp, s.position, s.ctx_state);
    } else if (!strcmp(what, "encode_match")) {
        // Encode match at position from arg, dist=arg, len=arg
        unsigned dist = atoi(argv[2]);
        unsigned len = atoi(argv[3]);
        unsigned pos = atoi(argv[4]);
        uint8_t data[100] = {0};
        // Make pattern where the match will validate (we don't actually verify match data; just to encode)
        for (int i = 0; i < 100; i++) data[i] = i;
        LZMAState s;
        LZMAProperties p = { .lc = 0, .lp = 0, .pb = 0 };
        lzma_state_init(&s, data, 100, p);
        s.position = pos;

        uint64_t perp = 0;
        EncoderInterface enc;
        perplexity_encoder_new(&enc, &perp);
        LZMAPacket pkt = match_packet(dist, len);
        lzma_encode_packet(&s, &enc, pkt);
        printf("perp=%lu position=%zu ctx_state=%u dists=%u,%u,%u,%u\n",
            perp, s.position, s.ctx_state,
            s.dists[0], s.dists[1], s.dists[2], s.dists[3]);
    } else if (!strcmp(what, "encode_short_rep")) {
        unsigned pos = atoi(argv[2]);
        uint8_t data[16] = {0};
        LZMAState s;
        LZMAProperties p = { .lc = 0, .lp = 0, .pb = 0 };
        lzma_state_init(&s, data, 16, p);
        s.position = pos;

        uint64_t perp = 0;
        EncoderInterface enc;
        perplexity_encoder_new(&enc, &perp);
        LZMAPacket pkt = short_rep_packet();
        lzma_encode_packet(&s, &enc, pkt);
        printf("perp=%lu position=%zu ctx_state=%u\n",
            perp, s.position, s.ctx_state);
    } else if (!strcmp(what, "encode_long_rep")) {
        unsigned dist_index = atoi(argv[2]);
        unsigned len = atoi(argv[3]);
        unsigned pos = atoi(argv[4]);
        uint8_t data[100] = {0};
        for (int i = 0; i < 100; i++) data[i] = i;
        LZMAState s;
        LZMAProperties p = { .lc = 0, .lp = 0, .pb = 0 };
        lzma_state_init(&s, data, 100, p);
        s.position = pos;
        s.dists[0] = 1; s.dists[1] = 5; s.dists[2] = 10; s.dists[3] = 20;

        uint64_t perp = 0;
        EncoderInterface enc;
        perplexity_encoder_new(&enc, &perp);
        LZMAPacket pkt = long_rep_packet(dist_index, len);
        lzma_encode_packet(&s, &enc, pkt);
        printf("perp=%lu position=%zu ctx_state=%u dists=%u,%u,%u,%u\n",
            perp, s.position, s.ctx_state,
            s.dists[0], s.dists[1], s.dists[2], s.dists[3]);
    }

    return 0;
}
