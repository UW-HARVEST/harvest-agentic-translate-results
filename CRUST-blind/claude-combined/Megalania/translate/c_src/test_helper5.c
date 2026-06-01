/* Tests for top_k_packet_finder */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "src/lzma_state.h"
#include "src/lzma_packet.h"
#include "src/packet_enumerator.h"
#include "src/top_k_packet_finder.h"
#include "src/packet_slab.h"

int main(int argc, char** argv) {
    if (argc < 4) return 1;
    const char* str = argv[1];
    size_t pos = atoi(argv[2]);
    size_t k = atoi(argv[3]);
    size_t len = strlen(str);
    const uint8_t* data = (const uint8_t*) str;

    LZMAState s;
    LZMAProperties p = { .lc = 0, .lp = 0, .pb = 0 };
    lzma_state_init(&s, data, len, p);
    s.position = pos;

    PacketEnumerator* pe = packet_enumerator_new(data, len);
    TopKPacketFinder* finder = top_k_packet_finder_new(k, pe);

    PacketSlab* slab = packet_slab_new(len);
    LZMAPacket* packets = packet_slab_packets(slab);

    top_k_packet_finder_find(finder, &s, packets);
    size_t count = top_k_packet_finder_count(finder);
    printf("count=%zu\n", count);
    LZMAPacket pkt;
    while (top_k_packet_finder_pop(finder, &pkt)) {
        printf("pop: type=%u dist=%u len=%u\n", pkt.type, pkt.dist, pkt.len);
    }

    packet_slab_free(slab);
    top_k_packet_finder_free(finder);
    packet_enumerator_free(pe);
    return 0;
}
