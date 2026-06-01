#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <math.h>
#include "m17.h"

int main(void) {
    // Print all bits of conv_encode_LSF for a known LSF
    lsf_t lsf = {0};
    for(int i=0;i<6;i++) lsf.dst[i] = i+1;
    for(int i=0;i<6;i++) lsf.src[i] = 0x10+i;
    lsf.type[0] = 0xAB; lsf.type[1] = 0xCD;
    for(int i=0;i<14;i++) lsf.meta[i] = i*3;
    lsf.crc[0] = 0x01; lsf.crc[1] = 0x74;

    uint8_t out_lsf[368];
    conv_encode_LSF(out_lsf, &lsf);
    printf("conv_encode_LSF (full):\n");
    for(int i=0;i<368;i++) printf("%d", out_lsf[i]);
    printf("\n");

    // conv_encode_stream_frame
    uint8_t stream_in[16];
    for(int i=0;i<16;i++) stream_in[i]=i*7;
    uint8_t stream_out[272];
    conv_encode_stream_frame(stream_out, stream_in, 0x1234);
    printf("conv_encode_stream_frame fn=0x1234:\n");
    for(int i=0;i<272;i++) printf("%d", stream_out[i]);
    printf("\n");

    // simpler: stream all zeros, fn=0
    uint8_t stream_in0[16] = {0};
    conv_encode_stream_frame(stream_out, stream_in0, 0);
    int sum = 0;
    for(int i=0;i<272;i++) sum += stream_out[i];
    printf("conv_encode_stream_frame zero,fn=0 sum=%d\n", sum);

    // conv_encode_packet_frame
    uint8_t pkt_in[26];
    for(int i=0;i<26;i++) pkt_in[i]=(i*13)&0xFF;
    uint8_t pkt_out[368];
    conv_encode_packet_frame(pkt_out, pkt_in);
    printf("conv_encode_packet_frame:\n");
    for(int i=0;i<368;i++) printf("%d", pkt_out[i]);
    printf("\n");

    // Viterbi tests
    // Encode all zeros LSF, then decode it
    {
        uint8_t in_zero[16] = {0};
        uint8_t enc[272];
        conv_encode_stream_frame(enc, in_zero, 0);
        // expand each bit to soft 0 or 0xFFFF
        uint16_t soft[272];
        for(int i=0;i<272;i++) soft[i] = enc[i] ? 0xFFFF : 0x0000;

        uint8_t dec[20] = {0};
        // viterbi_decode_punctured uses puncture_pattern_2, len=272
        uint32_t err = viterbi_decode_punctured(dec, soft, puncture_pattern_2, 272, sizeof(puncture_pattern_2));
        printf("viterbi_decode_punctured(zero,fn=0) err=%u, dec=", err);
        for(int i=0;i<20;i++) printf("%02X ", dec[i]);
        printf("\n");
    }

    // viterbi_decode plain on simple input
    {
        uint16_t in[10] = {0};
        uint8_t out[5] = {0xAA};
        uint32_t err = viterbi_decode(out, in, 10);
        printf("viterbi_decode(zeros) err=%u out=%02X %02X\n", err, out[0], out[1]);
    }

    // SYMBOL_MAP, SYMBOL_LIST, EOT_SYMBOLS
    printf("symbol_map: %d %d %d %d\n", symbol_map[0], symbol_map[1], symbol_map[2], symbol_map[3]);
    printf("symbol_list: %d %d %d %d\n", symbol_list[0], symbol_list[1], symbol_list[2], symbol_list[3]);

    // send_preamble
    float buf[192];
    uint32_t cnt = 0;
    send_preamble(buf, &cnt, PREAM_LSF);
    printf("send_preamble LSF cnt=%u, [0..3]: %g %g %g %g, [188..191]: %g %g %g %g\n",
        cnt, buf[0], buf[1], buf[2], buf[3], buf[188], buf[189], buf[190], buf[191]);
    cnt = 0;
    send_preamble(buf, &cnt, PREAM_BERT);
    printf("send_preamble BERT cnt=%u, [0..3]: %g %g %g %g\n", cnt, buf[0], buf[1], buf[2], buf[3]);

    // send_eot
    cnt = 0;
    send_eot(buf, &cnt);
    printf("send_eot cnt=%u, [0..7]: %g %g %g %g %g %g %g %g, [184..191]: %g %g %g %g %g %g %g %g\n",
        cnt, buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        buf[184], buf[185], buf[186], buf[187], buf[188], buf[189], buf[190], buf[191]);

    // send_syncword
    float swbuf[8];
    cnt = 0;
    send_syncword(swbuf, &cnt, SYNC_LSF);
    printf("send_syncword LSF cnt=%u: %g %g %g %g %g %g %g %g\n", cnt,
        swbuf[0], swbuf[1], swbuf[2], swbuf[3], swbuf[4], swbuf[5], swbuf[6], swbuf[7]);

    // send_data
    float dbuf[184];
    cnt = 0;
    uint8_t data_in[368] = {0};
    for(int i=0; i<368; i++) data_in[i] = i & 1;
    send_data(dbuf, &cnt, data_in);
    printf("send_data cnt=%u, [0..3]: %g %g %g %g\n", cnt, dbuf[0], dbuf[1], dbuf[2], dbuf[3]);

    // EOT_SYMBOLS
    printf("eot_symbols: ");
    for(int i=0;i<8;i++) printf("%g ", eot_symbols[i]);
    printf("\n");

    // send_frame for FRAME_LSF
    float fb[192];
    conv_encode_LSF(out_lsf, &lsf);  // ensure LSF.crc set
    send_frame(fb, NULL, FRAME_LSF, &lsf, 0, 0);
    printf("send_frame LSF [0..7]: %g %g %g %g %g %g %g %g\n",
        fb[0], fb[1], fb[2], fb[3], fb[4], fb[5], fb[6], fb[7]);
    printf("send_frame LSF [8..11]: %g %g %g %g\n", fb[8], fb[9], fb[10], fb[11]);

    return 0;
}
