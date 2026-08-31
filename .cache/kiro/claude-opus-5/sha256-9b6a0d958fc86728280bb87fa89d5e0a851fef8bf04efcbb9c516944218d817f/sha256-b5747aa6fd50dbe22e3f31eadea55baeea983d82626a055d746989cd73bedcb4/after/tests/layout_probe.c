/* Prints sizeof/offsetof for decoder-internal structs so the Rust layouts can
 * be checked against the C ones. Compiled against the c_src headers. */
#include <stdio.h>
#include <stddef.h>
#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#include "decompress/zstd_decompress_internal.h"

struct ZSTD_DDict_s {
    void* dictBuffer;
    const void* dictContent;
    size_t dictSize;
    ZSTD_entropyDTables_t entropy;
    U32 dictID;
    U32 entropyPresent;
    ZSTD_customMem cMem;
};

int main(void)
{
    printf("ZSTD_TRACE=%d\n", ZSTD_TRACE);
    printf("DYNAMIC_BMI2=%d\n", DYNAMIC_BMI2);
    printf("sizeof(ZSTD_seqSymbol)=%zu\n", sizeof(ZSTD_seqSymbol));
    printf("sizeof(ZSTD_entropyDTables_t)=%zu\n", sizeof(ZSTD_entropyDTables_t));
    printf("sizeof(ZSTD_DCtx)=%zu\n", sizeof(ZSTD_DCtx));
    printf("sizeof(ZSTD_DDict)=%zu\n", sizeof(struct ZSTD_DDict_s));
    printf("sizeof(ZSTD_FrameHeader)=%zu\n", sizeof(ZSTD_FrameHeader));
    printf("sizeof(XXH64_state_t)=%zu\n", sizeof(XXH64_state_t));
    printf("ZSTD_LITBUFFEREXTRASIZE=%d\n", (int)ZSTD_LITBUFFEREXTRASIZE);
    printf("ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32=%d\n", (int)ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32);
    printf("HUF_DECOMPRESS_WORKSPACE_SIZE_U32=%d\n", (int)HUF_DECOMPRESS_WORKSPACE_SIZE_U32);
#define O(f) printf("offsetof(%s)=%zu\n", #f, offsetof(ZSTD_DCtx, f))
    O(LLTptr); O(entropy); O(workspace); O(previousDstEnd); O(expected);
    O(fParams); O(processedCSize); O(bType); O(stage); O(xxhState);
    O(headerSize); O(format); O(litPtr); O(customMem); O(litSize);
    O(staticSize); O(isFrameDecompression); O(ddictLocal); O(dictID);
    O(dictUses); O(ddictSet); O(refMultipleDDicts); O(disableHufAsm);
    O(maxBlockSizeParam); O(streamStage); O(inBuff); O(maxWindowSize);
    O(outBuff); O(lhSize); O(legacyContext); O(previousLegacyVersion);
    O(hostageByte); O(noForwardProgress); O(outBufferMode);
    O(expectedOutBuffer); O(litBuffer); O(litBufferEnd);
    O(litBufferLocation); O(litExtraBuffer); O(headerBuffer);
    O(oversizedDuration); O(traceCtx);
#undef O
    printf("offsetof(DDict.entropy)=%zu\n", offsetof(struct ZSTD_DDict_s, entropy));
    printf("offsetof(DDict.dictID)=%zu\n", offsetof(struct ZSTD_DDict_s, dictID));
    printf("offsetof(DDict.cMem)=%zu\n", offsetof(struct ZSTD_DDict_s, cMem));
    return 0;
}
