#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <math.h>
#include "m17.h"

int main(void) {
    // golay24_encode
    printf("golay24_encode(0)=%u\n", golay24_encode(0));
    printf("golay24_encode(1)=%u\n", golay24_encode(1));
    printf("golay24_encode(0xFFF)=%u\n", golay24_encode(0xFFF));
    printf("golay24_encode(0x123)=%u\n", golay24_encode(0x123));
    printf("golay24_encode(0xABC)=%u\n", golay24_encode(0xABC));

    // q_abs_diff
    printf("q_abs_diff(10,3)=%u\n", q_abs_diff(10,3));
    printf("q_abs_diff(3,10)=%u\n", q_abs_diff(3,10));

    // eucl_norm
    float a[3]={1.0f, 2.0f, 3.0f};
    int8_t b[3]={0, 0, 0};
    printf("eucl_norm=%f\n", eucl_norm(a, b, 3));

    // int_to_soft
    uint16_t soft[12];
    int_to_soft(soft, 0xA5C, 12);
    printf("int_to_soft(0xA5C):");
    for(int i=0;i<12;i++) printf(" %04X", soft[i]);
    printf("\n");

    // soft_to_int
    uint16_t s[16] = {0xFFFF, 0, 0xFFFF, 0, 0x8000, 0x7FFF, 0xFFFF, 0,
                     0, 0xFFFF, 0xFFFF, 0xFFFF, 0, 0, 0xFFFF, 0xFFFF};
    printf("soft_to_int(s,16)=%u\n", soft_to_int(s, 16));
    printf("soft_to_int(s,8)=%u\n", soft_to_int(s, 8));

    // div16, mul16, soft_bit_XOR, soft_bit_NOT
    printf("div16(0xFFFF, 2)=%u\n", div16(0xFFFF, 2));
    printf("div16(0x1000, 0x10)=%u\n", div16(0x1000, 0x10));
    printf("mul16(0xFFFF, 0xFFFF)=%u\n", mul16(0xFFFF, 0xFFFF));
    printf("mul16(0x8000, 0x8000)=%u\n", mul16(0x8000, 0x8000));
    printf("soft_bit_XOR(0,0)=%u\n", soft_bit_XOR(0, 0));
    printf("soft_bit_XOR(0xFFFF,0)=%u\n", soft_bit_XOR(0xFFFF, 0));
    printf("soft_bit_XOR(0xFFFF,0xFFFF)=%u\n", soft_bit_XOR(0xFFFF, 0xFFFF));
    printf("soft_bit_XOR(0x7FFF,0x7FFF)=%u\n", soft_bit_XOR(0x7FFF, 0x7FFF));
    printf("soft_bit_XOR(0xFFFF,0x7FFF)=%u\n", soft_bit_XOR(0xFFFF, 0x7FFF));
    printf("soft_bit_NOT(0xFFFF)=%u\n", soft_bit_NOT(0xFFFF));
    printf("soft_bit_NOT(0)=%u\n", soft_bit_NOT(0));
    printf("soft_bit_NOT(0x1234)=%u\n", soft_bit_NOT(0x1234));

    // CRC
    uint8_t s_in[] = "123456789";
    printf("CRC_M17(\"123456789\")=%04X\n", CRC_M17(s_in, 9));
    uint8_t empty[1]={0};
    printf("CRC_M17(\"\")=%04X\n", CRC_M17(empty, 0));
    uint8_t hello[] = {0x48,0x65,0x6C,0x6C,0x6F};
    printf("CRC_M17(\"Hello\")=%04X\n", CRC_M17(hello, 5));

    // LSF_CRC
    lsf_t lsf = {0};
    memset(&lsf, 0, sizeof(lsf));
    printf("LSF_CRC(zero)=%04X\n", LSF_CRC(&lsf));
    for(int i=0;i<6;i++) lsf.dst[i] = i+1;
    for(int i=0;i<6;i++) lsf.src[i] = 0x10+i;
    lsf.type[0] = 0xAB; lsf.type[1] = 0xCD;
    for(int i=0;i<14;i++) lsf.meta[i] = i*3;
    printf("LSF_CRC(populated)=%04X\n", LSF_CRC(&lsf));

    // encode/decode callsign
    uint64_t out_val=0;
    encode_callsign_value(&out_val, (uint8_t*)"AB1CD");
    printf("encode_callsign_value(AB1CD)=%lu\n", (unsigned long)out_val);
    encode_callsign_value(&out_val, (uint8_t*)"@ALL");
    printf("encode_callsign_value(@ALL)=%lu\n", (unsigned long)out_val);
    encode_callsign_value(&out_val, (uint8_t*)"#TEST");
    printf("encode_callsign_value(#TEST)=%lu\n", (unsigned long)out_val);
    encode_callsign_value(&out_val, (uint8_t*)"");
    printf("encode_callsign_value(empty)=%lu\n", (unsigned long)out_val);
    encode_callsign_value(&out_val, (uint8_t*)"SP5WWP");
    printf("encode_callsign_value(SP5WWP)=%lu\n", (unsigned long)out_val);

    uint8_t out_bytes[6];
    int8_t r = encode_callsign_bytes(out_bytes, (uint8_t*)"SP5WWP");
    printf("encode_callsign_bytes(SP5WWP)=%d, ", r);
    for(int i=0;i<6;i++) printf("%02X ", out_bytes[i]);
    printf("\n");

    // long string returns -1
    r = encode_callsign_value(&out_val, (uint8_t*)"TOOLONGCALLSIGN");
    printf("encode_callsign_value(TOOLONGCALLSIGN) ret=%d\n", r);

    uint8_t decoded[20] = {0};
    encode_callsign_value(&out_val, (uint8_t*)"SP5WWP");
    decode_callsign_value(decoded, out_val);
    printf("decode_callsign_value(SP5WWP_val)=\"%s\"\n", decoded);

    memset(decoded, 0, 20);
    decode_callsign_value(decoded, 0xFFFFFFFFFFFFULL);
    printf("decode_callsign_value(BCAST)=\"%s\"\n", decoded);

    encode_callsign_value(&out_val, (uint8_t*)"#TEST");
    memset(decoded, 0, 20);
    decode_callsign_value(decoded, out_val);
    printf("decode_callsign_value(#TEST_val)=\"%s\"\n", decoded);

    // decode_callsign_bytes
    encode_callsign_bytes(out_bytes, (uint8_t*)"SP5WWP");
    memset(decoded, 0, 20);
    decode_callsign_bytes(decoded, out_bytes);
    printf("decode_callsign_bytes(SP5WWP)=\"%s\"\n", decoded);

    // unpack_LICH
    uint8_t lich_packed[12] = {0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x00};
    uint8_t lich_unpacked[96];
    unpack_LICH(lich_unpacked, lich_packed);
    printf("unpack_LICH:");
    for(int i=0;i<96;i++) printf("%d", lich_unpacked[i]);
    printf("\n");

    // extract_LICH
    uint8_t lich_out[6];
    for(int c=0; c<6; c++) {
        memset(lich_out, 0xAA, 6);
        extract_LICH(lich_out, c, &lsf);
        printf("extract_LICH(c=%d):", c);
        for(int i=0;i<6;i++) printf(" %02X", lich_out[i]);
        printf("\n");
    }

    // golay24_sdecode
    uint16_t cw[24];
    uint32_t encoded = golay24_encode(0xABC);
    // pack into soft codeword reverse
    for(int i=0;i<24;i++) {
        cw[i] = ((encoded >> (23-i)) & 1) ? 0xFFFF : 0x0000;
    }
    printf("golay24_sdecode(0xABC encoded)=%u\n", golay24_sdecode(cw));
    encoded = golay24_encode(0x123);
    for(int i=0;i<24;i++) {
        cw[i] = ((encoded >> (23-i)) & 1) ? 0xFFFF : 0x0000;
    }
    printf("golay24_sdecode(0x123 encoded)=%u\n", golay24_sdecode(cw));

    // encode_LICH
    uint8_t lich_in[6] = {0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC};
    uint8_t lich_enc[12];
    encode_LICH(lich_enc, lich_in);
    printf("encode_LICH:");
    for(int i=0;i<12;i++) printf(" %02X", lich_enc[i]);
    printf("\n");

    // reorder_bits
    uint8_t inb[368], outb[368];
    for(int i=0;i<368;i++) inb[i] = (uint8_t)(i & 0xFF);
    reorder_bits(outb, inb);
    printf("reorder_bits: outb[0]=%d outb[5]=%d outb[100]=%d outb[367]=%d\n",
           outb[0], outb[5], outb[100], outb[367]);

    // randomize_bits
    uint8_t rb[368];
    memset(rb, 0, 368);
    randomize_bits(rb);
    printf("randomize_bits: rb[0]=%d rb[1]=%d rb[7]=%d rb[8]=%d rb[100]=%d rb[367]=%d\n",
           rb[0], rb[1], rb[7], rb[8], rb[100], rb[367]);

    // conv_encode_LSF
    uint8_t lsf_out[368];
    conv_encode_LSF(lsf_out, &lsf);
    int sum = 0;
    for(int i=0;i<368;i++) sum += lsf_out[i];
    printf("conv_encode_LSF sum=%d, first10=", sum);
    for(int i=0;i<10;i++) printf("%d", lsf_out[i]);
    printf("\n");

    // conv_encode_stream_frame
    uint8_t stream_in[16];
    for(int i=0;i<16;i++) stream_in[i]=i*7;
    uint8_t stream_out[272];
    conv_encode_stream_frame(stream_out, stream_in, 0x1234);
    sum = 0;
    for(int i=0;i<272;i++) sum += stream_out[i];
    printf("conv_encode_stream_frame sum=%d, first10=", sum);
    for(int i=0;i<10;i++) printf("%d", stream_out[i]);
    printf("\n");

    // conv_encode_packet_frame
    uint8_t pkt_in[26];
    for(int i=0;i<26;i++) pkt_in[i]=(i*13)&0xFF;
    uint8_t pkt_out[368];
    conv_encode_packet_frame(pkt_out, pkt_in);
    sum = 0;
    for(int i=0;i<368;i++) sum += pkt_out[i];
    printf("conv_encode_packet_frame sum=%d, first10=", sum);
    for(int i=0;i<10;i++) printf("%d", pkt_out[i]);
    printf("\n");

    // SYNC values
    printf("SYNC_LSF=%04X SYNC_STR=%04X SYNC_PKT=%04X SYNC_BER=%04X EOT_MRKR=%04X\n",
        SYNC_LSF, SYNC_STR, SYNC_PKT, SYNC_BER, EOT_MRKR);

    return 0;
}
