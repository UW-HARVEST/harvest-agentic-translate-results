/* Tests for packet_enumerator. Counts each type of packet returned. */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "src/lzma_state.h"
#include "src/lzma_packet.h"
#include "src/packet_enumerator.h"

typedef struct {
    int literal;
    int match_;
    int short_rep;
    int long_rep;
} Counts;

static void cb(void* user_data, const LZMAState* state, LZMAPacket packet) {
    (void)state;
    Counts* c = (Counts*) user_data;
    switch (packet.type) {
        case LITERAL: c->literal++; break;
        case MATCH: c->match_++; break;
        case SHORT_REP: c->short_rep++; break;
        case LONG_REP: c->long_rep++; break;
        default: break;
    }
}

int main(int argc, char** argv) {
    if (argc < 4) return 1;
    // arg1: data string, arg2: position, arg3: dist0, dist1, dist2, dist3
    const char* str = argv[1];
    size_t pos = atoi(argv[2]);
    uint32_t d0 = (uint32_t)atoi(argv[3]);
    uint32_t d1 = (uint32_t)atoi(argv[4]);
    uint32_t d2 = (uint32_t)atoi(argv[5]);
    uint32_t d3 = (uint32_t)atoi(argv[6]);

    size_t len = strlen(str);
    const uint8_t* data = (const uint8_t*) str;

    LZMAState s;
    LZMAProperties p = { .lc = 0, .lp = 0, .pb = 0 };
    lzma_state_init(&s, data, len, p);
    s.position = pos;
    s.dists[0] = d0;
    s.dists[1] = d1;
    s.dists[2] = d2;
    s.dists[3] = d3;

    PacketEnumerator* pe = packet_enumerator_new(data, len);

    Counts c = {0, 0, 0, 0};
    packet_enumerator_for_each(pe, &s, cb, &c);
    printf("literal=%d match=%d short_rep=%d long_rep=%d\n",
        c.literal, c.match_, c.short_rep, c.long_rep);

    packet_enumerator_free(pe);
    return 0;
}
