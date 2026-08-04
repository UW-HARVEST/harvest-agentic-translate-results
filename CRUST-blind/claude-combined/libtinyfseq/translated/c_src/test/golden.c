#undef NDEBUG
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#define TINYFSEQ_IMPLEMENTATION
#include "../tinyfseq.h"

static void dump_header(const TFHeader *h, TFError err, const uint8_t *ep, const uint8_t *bd) {
    printf("err=%d\n", err);
    if (err != TF_OK) return;
    printf("channelDataOffset=%u\n", h->channelDataOffset);
    printf("minorVersion=%u\n", h->minorVersion);
    printf("majorVersion=%u\n", h->majorVersion);
    printf("variableDataOffset=%u\n", h->variableDataOffset);
    printf("channelCount=%u\n", h->channelCount);
    printf("frameCount=%u\n", h->frameCount);
    printf("frameStepTimeMillis=%u\n", h->frameStepTimeMillis);
    printf("compressionType=%d\n", h->compressionType);
    printf("compressionBlockCount=%u\n", h->compressionBlockCount);
    printf("channelRangeCount=%u\n", h->channelRangeCount);
    printf("sequenceUid=%llu\n", (unsigned long long)h->sequenceUid);
    printf("ep_offset=%ld\n", (long)(ep - bd));
}

static void dump_block(const TFCompressionBlock *b, TFError err, const uint8_t *ep, const uint8_t *bd) {
    printf("err=%d\n", err);
    if (err != TF_OK) return;
    printf("firstFrameId=%u\n", b->firstFrameId);
    printf("size=%u\n", b->size);
    printf("ep_offset=%ld\n", (long)(ep - bd));
}

static void dump_var(const TFVarHeader *v, TFError err, const uint8_t *ep, const uint8_t *bd, const uint8_t *vd, int vs) {
    printf("err=%d\n", err);
    if (err != TF_OK) return;
    printf("size=%u\n", v->size);
    printf("id=%u,%u\n", v->id[0], v->id[1]);
    if (ep) printf("ep_offset=%ld\n", (long)(ep - bd));
    if (vd) {
        printf("vd=");
        for (int i = 0; i < vs; i++) printf("%u,", vd[i]);
        printf("\n");
    }
}

static void dump_range(const TFChannelRange *r, TFError err, const uint8_t *ep, const uint8_t *bd) {
    printf("err=%d\n", err);
    if (err != TF_OK) return;
    printf("firstChannelNumber=%u\n", r->firstChannelNumber);
    printf("channelCount=%u\n", r->channelCount);
    printf("ep_offset=%ld\n", (long)(ep - bd));
}

int main(void) {
    // Test header read with a valid PSEQ header
    {
        uint8_t buf[40] = {
            'P','S','E','Q',          // magic
            0x10,0x00,                // channelDataOffset = 16
            0x02,                     // minorVersion = 2
            0x02,                     // majorVersion = 2
            0x20,0x00,                // variableDataOffset = 32
            0x10,0x00,0x00,0x00,      // channelCount = 16
            0x05,0x00,0x00,0x00,      // frameCount = 5
            0x32,                     // frameStepTimeMillis = 50
            0x00,                     // reserved
            0x21,                     // compression byte: lower 4 bits = 1 (ZSTD), upper = 2
            0x03,                     // compressionBlockCount = 3
            0x02,                     // channelRangeCount = 2
            0x00,                     // reserved
            0x78,0x56,0x34,0x12,0xAB,0xCD,0xEF,0x01, // sequenceUid
            0xDE,0xAD,0xBE,0xEF,      // padding (after 32 bytes)
            0x00,0x00,0x00,0x00
        };
        TFHeader h = {0};
        uint8_t *ep = NULL;
        TFError err = TFHeader_read(buf, sizeof(buf), &h, &ep);
        printf("---HEADER_VALID---\n");
        dump_header(&h, err, ep, buf);
    }

    // Test header invalid magic
    {
        uint8_t buf[32] = {0};
        buf[0]='X'; buf[1]='S'; buf[2]='E'; buf[3]='Q';
        TFHeader h = {0};
        TFError err = TFHeader_read(buf, sizeof(buf), &h, NULL);
        printf("---HEADER_INVALID_MAGIC---\n");
        dump_header(&h, err, NULL, buf);
    }

    // Test header buffer too small
    {
        uint8_t buf[10] = {0};
        TFHeader h = {0};
        TFError err = TFHeader_read(buf, sizeof(buf), &h, NULL);
        printf("---HEADER_SMALL---\n");
        dump_header(&h, err, NULL, buf);
    }

    // Test header invalid compression
    {
        uint8_t buf[32] = {
            'P','S','E','Q',
            0,0,2,2,
            0,0,
            0,0,0,0,
            0,0,0,0,
            0, 0,
            0x05, // compression = 5 in lower 4 bits => invalid
            0,0,0,
            0,0,0,0,0,0,0,0
        };
        TFHeader h = {0};
        TFError err = TFHeader_read(buf, sizeof(buf), &h, NULL);
        printf("---HEADER_INVALID_COMPRESSION---\n");
        dump_header(&h, err, NULL, buf);
    }

    // Compression block valid
    {
        uint8_t buf[10] = {0x01,0x02,0x03,0x04, 0xAA,0xBB,0xCC,0xDD, 0,0};
        TFCompressionBlock b = {0};
        uint8_t *ep = NULL;
        TFError err = TFCompressionBlock_read(buf, sizeof(buf), &b, &ep);
        printf("---BLOCK_VALID---\n");
        dump_block(&b, err, ep, buf);
    }

    // Compression block buffer too small
    {
        uint8_t buf[4] = {0,0,0,0};
        TFCompressionBlock b = {0};
        TFError err = TFCompressionBlock_read(buf, sizeof(buf), &b, NULL);
        printf("---BLOCK_SMALL---\n");
        dump_block(&b, err, NULL, buf);
    }

    // Var header valid (size=8, value: "abcd")
    {
        uint8_t buf[8] = {0x08,0x00, 'A','B', 'a','b','c','d'};
        TFVarHeader v = {0};
        uint8_t *ep = NULL;
        uint8_t vd[4] = {0};
        TFError err = TFVarHeader_read(buf, sizeof(buf), &v, vd, sizeof(vd), &ep);
        printf("---VAR_VALID---\n");
        dump_var(&v, err, ep, buf, vd, sizeof(vd));
    }

    // Var header NULL vd (only header read, no value)
    {
        uint8_t buf[8] = {0x08,0x00, 'A','B', 'a','b','c','d'};
        TFVarHeader v = {0};
        uint8_t *ep = NULL;
        TFError err = TFVarHeader_read(buf, sizeof(buf), &v, NULL, 0, &ep);
        printf("---VAR_NO_VD---\n");
        dump_var(&v, err, ep, buf, NULL, 0);
    }

    // Var header buffer too small
    {
        uint8_t buf[4] = {0x08,0x00, 'A','B'};
        TFVarHeader v = {0};
        TFError err = TFVarHeader_read(buf, sizeof(buf), &v, NULL, 0, NULL);
        printf("---VAR_SMALL---\n");
        dump_var(&v, err, NULL, buf, NULL, 0);
    }

    // Var header size <= 4 (TF_EINVALID_VAR_SIZE)
    {
        uint8_t buf[8] = {0x04,0x00,'A','B',0,0,0,0};
        TFVarHeader v = {0};
        TFError err = TFVarHeader_read(buf, sizeof(buf), &v, NULL, 0, NULL);
        printf("---VAR_INVALID_SIZE---\n");
        dump_var(&v, err, NULL, buf, NULL, 0);
    }

    // Var header bs < size (TF_EINVALID_VAR_SIZE) when vd is provided
    {
        // size says 10 but only 8 bytes
        uint8_t buf[8] = {0x0A,0x00,'A','B',1,2,3,4};
        TFVarHeader v = {0};
        uint8_t vd[6] = {0};
        TFError err = TFVarHeader_read(buf, sizeof(buf), &v, vd, sizeof(vd), NULL);
        printf("---VAR_BS_LT_SIZE---\n");
        dump_var(&v, err, NULL, buf, vd, sizeof(vd));
    }

    // Var header vd too small (TF_EINVALID_BUFFER_SIZE)
    {
        uint8_t buf[8] = {0x08,0x00,'A','B',1,2,3,4};
        TFVarHeader v = {0};
        uint8_t vd[2] = {0};
        TFError err = TFVarHeader_read(buf, sizeof(buf), &v, vd, sizeof(vd), NULL);
        printf("---VAR_VD_SMALL---\n");
        dump_var(&v, err, NULL, buf, vd, sizeof(vd));
    }

    // Channel range valid
    {
        uint8_t buf[8] = {0x01,0x02,0x03, 0x04,0x05,0x06, 0,0};
        TFChannelRange r = {0};
        uint8_t *ep = NULL;
        TFError err = TFChannelRange_read(buf, sizeof(buf), &r, &ep);
        printf("---RANGE_VALID---\n");
        dump_range(&r, err, ep, buf);
    }

    // Channel range buffer too small
    {
        uint8_t buf[3] = {0,0,0};
        TFChannelRange r = {0};
        TFError err = TFChannelRange_read(buf, sizeof(buf), &r, NULL);
        printf("---RANGE_SMALL---\n");
        dump_range(&r, err, NULL, buf);
    }

    // TFError_string for each value
    printf("---ERR_STRINGS---\n");
    printf("0=%s\n", TFError_string(TF_OK));
    printf("1=%s\n", TFError_string(TF_EINVALID_MAGIC));
    printf("2=%s\n", TFError_string(TF_EINVALID_COMPRESSION_TYPE));
    printf("3=%s\n", TFError_string(TF_EINVALID_BUFFER_SIZE));
    printf("4=%s\n", TFError_string(TF_EINVALID_VAR_SIZE));

    return 0;
}
